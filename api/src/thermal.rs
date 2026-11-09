//! Mod-side thermal conduit pools and bounded adjacent transfer.

use horizon_client::{
    containers::ModBlockEntity,
    mod_api::{Compound, Host, Tag},
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
    sync::{LazyLock, Mutex},
};

use crate::heat::HeatStore;

pub(crate) const WROUGHT: &str = "arcforge:wrought_thermal_conduit";
pub(crate) const TEMPERED: &str = "arcforge:tempered_thermal_conduit";
pub(crate) const HARDENED: &str = "arcforge:hardened_thermal_conduit";
pub(crate) const ARCFORGED: &str = "arcforge:arcforged_thermal_conduit";

const MAX_TEMPERATURE_C: u32 = 2_000;
const NEIGHBORS: [Side; 6] = [
    Side {
        name: "up",
        offset: [0, 1, 0],
        opposite: [0, -1, 0],
    },
    Side {
        name: "down",
        offset: [0, -1, 0],
        opposite: [0, 1, 0],
    },
    Side {
        name: "north",
        offset: [0, 0, -1],
        opposite: [0, 0, 1],
    },
    Side {
        name: "south",
        offset: [0, 0, 1],
        opposite: [0, 0, -1],
    },
    Side {
        name: "west",
        offset: [-1, 0, 0],
        opposite: [1, 0, 0],
    },
    Side {
        name: "east",
        offset: [1, 0, 0],
        opposite: [-1, 0, 0],
    },
];

#[derive(Clone, Copy)]
struct Side {
    name: &'static str,
    offset: [i32; 3],
    opposite: [i32; 3],
}

type Pos = [i32; 3];
type CurrentEntity<'a> = Option<(Pos, &'a ModBlockEntity)>;

fn entity_at(host: &dyn Host, pos: Pos, current: CurrentEntity<'_>) -> Option<ModBlockEntity> {
    current
        .filter(|(current_pos, _)| *current_pos == pos)
        .map(|(_, entity)| entity.clone())
        .or_else(|| host.mod_block_entity(pos))
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct WorldKey {
    directory: Option<PathBuf>,
    seed: Option<i64>,
    dimension: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NeighborState {
    pos: Pos,
    id: String,
    own_mode: String,
    other_mode: String,
    connected: bool,
}

#[derive(Clone, Debug)]
struct Network {
    leader: Pos,
    members: Vec<Pos>,
    rate: u64,
    capacity: u64,
    stored: u64,
    temperature: u32,
    last_moved_tick: Option<i64>,
    neighbors: HashMap<Pos, Vec<NeighborState>>,
    input_tick: Option<i64>,
    input_used: u64,
    presence_check_tick: Option<i64>,
    next_presence_check: usize,
}

#[derive(Default)]
struct WorldNetworks {
    by_pos: HashMap<Pos, Pos>,
    networks: HashMap<Pos, Network>,
}

#[derive(Default)]
struct NetworkCache {
    worlds: HashMap<WorldKey, WorldNetworks>,
}

static NETWORK_CACHE: LazyLock<Mutex<NetworkCache>> =
    LazyLock::new(|| Mutex::new(NetworkCache::default()));

pub(crate) fn spec(id: &str) -> Option<(u64, u64)> {
    let rate = match id {
        WROUGHT => 50,
        TEMPERED => 200,
        HARDENED => 800,
        ARCFORGED => 51_200,
        _ => return None,
    };
    Some((rate, rate))
}

pub(crate) fn ids() -> [&'static str; 4] {
    [WROUGHT, TEMPERED, HARDENED, ARCFORGED]
}

fn world_key(host: &dyn Host) -> WorldKey {
    WorldKey {
        directory: host.world_dir(),
        seed: host.world_seed(),
        dimension: host.dimension().to_owned(),
    }
}

fn game_time(host: &dyn Host) -> Option<i64> {
    host.level(host.dimension())
        .and_then(|level| level.game_time(host))
}

fn neighbor_states(
    host: &dyn Host,
    pos: Pos,
    entity: &ModBlockEntity,
    current: CurrentEntity<'_>,
) -> Vec<NeighborState> {
    NEIGHBORS
        .iter()
        .filter_map(|side| {
            let neighbor_pos = add(pos, side.offset);
            let neighbor = entity_at(host, neighbor_pos, current)?;
            spec(&neighbor.id)?;
            let own_mode =
                get_string(&entity.nbt, &setting_key(side.name)).unwrap_or_else(|| "auto".into());
            let opposite = NEIGHBORS
                .iter()
                .find(|other| other.offset == side.opposite)
                .expect("thermal conduit has six opposite faces");
            let other_mode = get_string(&neighbor.nbt, &setting_key(opposite.name))
                .unwrap_or_else(|| "auto".into());
            // A shared pool can only span links that carry heat in both directions.
            // One-way links stay visible and transfer through the normal input path.
            let connected = modes_connect_both_ways(&own_mode, &other_mode);
            Some(NeighborState {
                pos: neighbor_pos,
                id: neighbor.id.clone(),
                own_mode,
                other_mode,
                connected,
            })
        })
        .collect()
}

fn network_for(host: &dyn Host, pos: Pos, current: CurrentEntity<'_>) -> Option<Network> {
    let entity = entity_at(host, pos, current)?;
    let Some((_, _)) = spec(&entity.id) else {
        return None;
    };
    let key = world_key(host);
    let current_neighbors = neighbor_states(host, pos, &entity, current);
    let current_tick = game_time(host);
    let mut cached = None;
    let mut presence_check = None;
    {
        let mut cache = NETWORK_CACHE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(world) = cache.worlds.get_mut(&key) {
            if let Some(leader) = world.by_pos.get(&pos).copied() {
                if let Some(network) = world.networks.get(&leader) {
                    if network.neighbors.get(&pos) == Some(&current_neighbors) {
                        cached = Some(network.clone());
                    }
                }
                if cached.is_none() {
                    cache.worlds.remove(&key);
                } else if let Some(tick) = current_tick {
                    if let Some(network) = cache
                        .worlds
                        .get_mut(&key)
                        .and_then(|world| world.networks.get_mut(&leader))
                        && network.presence_check_tick != Some(tick)
                        && !network.members.is_empty()
                    {
                        let index = network.next_presence_check % network.members.len();
                        presence_check = Some(network.members[index]);
                        network.next_presence_check = (index + 1) % network.members.len();
                        network.presence_check_tick = Some(tick);
                    }
                }
            }
        }
    }

    if let Some(network) = cached {
        if presence_check.is_none_or(|member| entity_at(host, member, current).is_some()) {
            return Some(network);
        }
        NETWORK_CACHE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .worlds
            .remove(&key);
    }

    let network = discover_network(host, pos, current)?;
    let mut cache = NETWORK_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let world = cache.worlds.entry(key).or_default();
    for member in &network.members {
        world.by_pos.insert(*member, network.leader);
    }
    world.networks.insert(network.leader, network.clone());
    Some(network)
}

fn discover_network(host: &dyn Host, start: Pos, current: CurrentEntity<'_>) -> Option<Network> {
    let first = entity_at(host, start, current)?;
    let (first_rate, _) = spec(&first.id)?;
    let mut queue = VecDeque::from([start]);
    let mut seen = HashSet::from([start]);
    let mut rate = first_rate;
    let mut neighbors = HashMap::new();

    while let Some(pos) = queue.pop_front() {
        let Some(entity) = entity_at(host, pos, current) else {
            continue;
        };
        let Some((node_rate, _)) = spec(&entity.id) else {
            continue;
        };
        rate = rate.min(node_rate);
        let edges = neighbor_states(host, pos, &entity, current);
        for edge in edges.iter().filter(|edge| edge.connected) {
            if seen.insert(edge.pos) {
                queue.push_back(edge.pos);
            }
        }
        neighbors.insert(pos, edges);
    }

    let mut members: Vec<_> = seen.into_iter().collect();
    members.sort_unstable();
    let leader = *members.first()?;
    let capacity = rate
        .saturating_mul(members.len() as u64)
        .min(i32::MAX as u64);
    let mut stored = 0_u64;
    let mut temperature = 20_u32;
    for member in &members {
        if let Some(entity) = entity_at(host, *member, current) {
            let node_stored = get_i32(&entity.nbt, "HeatHU").max(0) as u64;
            stored = stored.saturating_add(node_stored);
            if node_stored > 0 {
                temperature = temperature.max(get_i32(&entity.nbt, "TemperatureC").max(20) as u32);
            }
        }
    }
    Some(Network {
        leader,
        members,
        rate,
        capacity,
        stored: stored.min(capacity),
        temperature: if stored == 0 { 20 } else { temperature },
        last_moved_tick: None,
        neighbors,
        input_tick: None,
        input_used: 0,
        presence_check_tick: None,
        next_presence_check: 0,
    })
}

fn update_cached_pool(host: &dyn Host, network: &Network, stored: u64, temperature: u32) {
    let key = world_key(host);
    let mut cache = NETWORK_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(entry) = cache
        .worlds
        .get_mut(&key)
        .and_then(|world| world.networks.get_mut(&network.leader))
    {
        entry.stored = stored.min(entry.capacity);
        entry.temperature = if entry.stored == 0 { 20 } else { temperature };
        entry.last_moved_tick = network.last_moved_tick;
    }
}

fn set_network_active(host: &mut dyn Host, network: &Network, active: bool) {
    let state = if active { "true" } else { "false" };
    for pos in &network.members {
        if host.block_property(*pos, "active").as_deref() != Some(state) {
            host.set_block_property(*pos, "active", state);
        }
    }
}

fn reserve_input(host: &dyn Host, network: &Network, requested: u64) -> u64 {
    let Some(tick) = game_time(host) else {
        return requested.min(network.rate);
    };
    let key = world_key(host);
    let mut cache = NETWORK_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(entry) = cache
        .worlds
        .get_mut(&key)
        .and_then(|world| world.networks.get_mut(&network.leader))
    else {
        return 0;
    };
    if entry.input_tick != Some(tick) {
        entry.input_tick = Some(tick);
        entry.input_used = 0;
    }
    let reserved = requested.min(entry.rate.saturating_sub(entry.input_used));
    entry.input_used = entry.input_used.saturating_add(reserved);
    reserved
}

fn release_input(host: &dyn Host, network: &Network, unused: u64) {
    if unused == 0 {
        return;
    }
    let key = world_key(host);
    let Some(tick) = game_time(host) else {
        return;
    };
    let mut cache = NETWORK_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(entry) = cache
        .worlds
        .get_mut(&key)
        .and_then(|world| world.networks.get_mut(&network.leader))
        && entry.input_tick == Some(tick)
    {
        entry.input_used = entry.input_used.saturating_sub(unused);
    }
}

pub(crate) fn invalidate_world(host: &dyn Host) {
    let key = world_key(host);
    NETWORK_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .worlds
        .remove(&key);
}

pub(crate) fn cycle_face(host: &mut dyn Host, pos: [i32; 3], face: [i32; 3]) -> bool {
    let Some(side) = NEIGHBORS.iter().find(|side| side.offset == face).copied() else {
        return false;
    };
    let Some(entity) = host.mod_block_entity(pos) else {
        return false;
    };
    if spec(&entity.id).is_none() {
        return false;
    }
    let key = setting_key(side.name);
    let next = match get_string(&entity.nbt, &key).as_deref().unwrap_or("auto") {
        "auto" => "input",
        "input" => "output",
        "output" => "disabled",
        _ => "auto",
    };
    host.change_mod_block_entity(pos, &mut |target| {
        if spec(&target.id).is_some() {
            set_string(&mut target.nbt, &key, next);
        }
    });
    invalidate_world(host);
    let neighbor = add(pos, side.offset);
    let neighbor_entity = host.mod_block_entity(neighbor);
    let connection = connection_state(
        next,
        neighbor_entity.as_ref().map(|entity| entity.id.as_str()),
    );
    host.set_block_property(pos, side.name, connection);
    if neighbor_entity
        .as_ref()
        .is_some_and(|other| spec(&other.id).is_some())
    {
        let other_side = NEIGHBORS
            .iter()
            .find(|other| other.offset == side.opposite)
            .unwrap();
        let other_setting = neighbor_entity
            .as_ref()
            .and_then(|other| get_string(&other.nbt, &setting_key(other_side.name)));
        let own_mode = next.to_owned();
        let other_mode = other_setting.unwrap_or_else(|| "auto".into());
        let state = if modes_connect(&own_mode, &other_mode) {
            "pipe"
        } else {
            "none"
        };
        host.set_block_property(pos, side.name, state);
        host.set_block_property(neighbor, other_side.name, state);
    }
    true
}

pub(crate) fn accept_heat(
    host: &mut dyn Host,
    pos: [i32; 3],
    source_pos: [i32; 3],
    source: &mut HeatStore,
    rate: u64,
) -> u64 {
    let Some(entity) = host.mod_block_entity(pos) else {
        return 0;
    };
    let Some((_, max_insert)) = spec(&entity.id) else {
        return 0;
    };
    let incoming_face = [
        source_pos[0] - pos[0],
        source_pos[1] - pos[1],
        source_pos[2] - pos[2],
    ];
    let Some(side) = NEIGHBORS.iter().find(|side| side.offset == incoming_face) else {
        return 0;
    };
    if !can_input(get_string(&entity.nbt, &setting_key(side.name)).as_deref()) {
        return 0;
    }
    let Some(network) = network_for(host, pos, None) else {
        return 0;
    };
    let stored = network.stored.min(network.capacity);
    let carried_temperature = network.temperature;
    let source_temperature = source.temperature_c();
    if stored > 0 && source_temperature < carried_temperature {
        return 0;
    }
    let capacity_left = network.capacity.saturating_sub(stored);
    let requested = source
        .stored_hu()
        .min(rate.min(max_insert))
        .min(capacity_left);
    let reserved = reserve_input(host, &network, requested);
    if reserved == 0 {
        return 0;
    }
    let moved = source.remove_hu(reserved);
    if moved > 0 {
        let temperature = if stored == 0 {
            source_temperature
        } else {
            source_temperature.max(carried_temperature)
        };
        let mut updated = network.clone();
        updated.last_moved_tick = game_time(host);
        distribute_network_heat(
            host,
            &updated,
            stored.saturating_add(moved),
            temperature,
            None,
        );
        set_network_active(host, &updated, true);
        update_cached_pool(host, &updated, stored.saturating_add(moved), temperature);
    }
    release_input(host, &network, reserved.saturating_sub(moved));
    moved
}

fn write_network_heat_node(entity: &mut ModBlockEntity, stored: u64, rate: u64, temperature: u32) {
    set_i32(
        &mut entity.nbt,
        "HeatHU",
        stored.min(i32::MAX as u64) as i32,
    );
    set_i32(
        &mut entity.nbt,
        "TemperatureC",
        if stored == 0 { 20 } else { temperature as i32 },
    );
    entity.data.resize(2, 0);
    entity.data[0] = stored.min(i32::MAX as u64) as i32;
    entity.data[1] = rate.min(i32::MAX as u64) as i32;
}

fn network_heat_node_matches(
    entity: &ModBlockEntity,
    stored: u64,
    rate: u64,
    temperature: u32,
) -> bool {
    let expected_temperature = if stored == 0 { 20 } else { temperature as i32 };
    get_i32(&entity.nbt, "HeatHU").max(0) as u64 == stored
        && get_i32(&entity.nbt, "TemperatureC") == expected_temperature
        && entity.data.get(0).copied().unwrap_or_default() == stored as i32
        && entity.data.get(1).copied().unwrap_or_default() == rate as i32
}

fn distribute_network_heat(
    host: &mut dyn Host,
    network: &Network,
    total: u64,
    temperature: u32,
    mut current: Option<(Pos, &mut ModBlockEntity)>,
) {
    let total = total.min(network.capacity);
    let count = network.members.len() as u64;
    let each = if count == 0 { 0 } else { total / count };
    let remainder = if count == 0 { 0 } else { total % count };
    for (index, pos) in network.members.iter().enumerate() {
        let stored = each + u64::from((index as u64) < remainder);
        if current
            .as_ref()
            .is_some_and(|(current_pos, _)| current_pos == pos)
        {
            let (_, entity) = current.as_mut().expect("current node was checked");
            if !network_heat_node_matches(entity, stored, network.rate, temperature) {
                write_network_heat_node(entity, stored, network.rate, temperature);
            }
        } else {
            let mut changed = false;
            host.change_mod_block_entity(*pos, &mut |entity| {
                if spec(&entity.id).is_some() {
                    if !network_heat_node_matches(entity, stored, network.rate, temperature) {
                        write_network_heat_node(entity, stored, network.rate, temperature);
                        changed = true;
                    }
                }
            });
            if changed {
                host.block_entity_sync(*pos);
            }
        }
    }
}

pub(crate) fn tick(
    host: &mut dyn Host,
    pos: [i32; 3],
    entity: &mut ModBlockEntity,
    mut transfer_to_neighbor: impl FnMut(&mut dyn Host, [i32; 3], [i32; 3], &mut HeatStore, u64) -> u64,
) {
    let Some(network) = network_for(host, pos, Some((pos, entity))) else {
        return;
    };
    update_connections(host, pos, &entity.nbt);
    if pos != network.leader {
        return;
    }

    let stored = network.stored.min(network.capacity);
    let temperature = network.temperature;
    let mut heat = HeatStore::new(network.capacity.max(1), MAX_TEMPERATURE_C, stored);
    heat.set_source_temperature(temperature);
    let endpoints = network_output_endpoints(host, &network, Some((pos, entity)));
    let stored_before_output = heat.stored_hu();
    distribute_heat(
        host,
        &endpoints,
        &mut heat,
        network.rate,
        &mut transfer_to_neighbor,
    );
    let remaining = heat.stored_hu();
    let transferred = stored_before_output.saturating_sub(remaining);
    let tick = game_time(host);
    let mut updated = network.clone();
    updated.stored = remaining;
    if transferred > 0 {
        updated.last_moved_tick = tick;
    }
    distribute_network_heat(host, &updated, remaining, temperature, Some((pos, entity)));
    let recently_moved = tick
        .zip(updated.last_moved_tick)
        .is_some_and(|(now, last)| now >= last && now - last < 10);
    set_network_active(host, &updated, remaining > 0 || recently_moved);
    update_cached_pool(host, &updated, remaining, temperature);
}

fn network_output_endpoints(
    host: &dyn Host,
    network: &Network,
    current: CurrentEntity<'_>,
) -> Vec<(Pos, Pos)> {
    let mut endpoints = Vec::new();
    for pos in &network.members {
        let Some(entity) = entity_at(host, *pos, current) else {
            continue;
        };
        for side in NEIGHBORS {
            if !can_output(get_string(&entity.nbt, &setting_key(side.name)).as_deref()) {
                continue;
            }
            let neighbor = add(*pos, side.offset);
            let Some(target) = host.mod_block_entity(neighbor) else {
                continue;
            };
            if spec(&target.id).is_some() {
                // Bidirectional conduit links are already in this pool. A one-way
                // link joins separate pools and must pass through the target's
                // configured input face so the direction and both budgets apply.
                if network.members.binary_search(&neighbor).is_ok() {
                    continue;
                }
                let Some(opposite) = NEIGHBORS.iter().find(|other| other.offset == side.opposite)
                else {
                    continue;
                };
                let target_mode = get_string(&target.nbt, &setting_key(opposite.name));
                if can_input(target_mode.as_deref()) {
                    endpoints.push((*pos, neighbor));
                }
            } else if is_heat_target(&target.id) {
                endpoints.push((*pos, neighbor));
            }
        }
    }
    endpoints
}

fn distribute_heat(
    host: &mut dyn Host,
    endpoints: &[(Pos, Pos)],
    heat: &mut HeatStore,
    rate: u64,
    transfer_to_neighbor: &mut impl FnMut(&mut dyn Host, Pos, Pos, &mut HeatStore, u64) -> u64,
) {
    let mut open: Vec<_> = endpoints.iter().copied().collect();
    if !open.is_empty() {
        let tick = game_time(host).unwrap_or_default();
        let start = tick.rem_euclid(open.len() as i64) as usize;
        open.rotate_left(start);
    }
    let mut remaining = rate.min(heat.stored_hu());
    while remaining > 0 && !open.is_empty() {
        let count = open.len() as u64;
        let each = remaining / count;
        let remainder = remaining % count;
        let mut next = Vec::with_capacity(open.len());
        for (index, (source, sink)) in open.iter().copied().enumerate() {
            let share = each + u64::from((index as u64) < remainder);
            if share == 0 {
                next.push((source, sink));
                continue;
            }
            let moved = transfer_to_neighbor(host, source, sink, heat, share.min(remaining));
            remaining = remaining.saturating_sub(moved);
            if moved >= share {
                next.push((source, sink));
            }
        }
        if next.len() == open.len() {
            break;
        }
        open = next;
    }
}

pub(crate) fn update_connections(host: &mut dyn Host, pos: [i32; 3], nbt: &Compound) {
    for side in NEIGHBORS {
        let neighbor = add(pos, side.offset);
        let neighbor_entity = host.mod_block_entity(neighbor);
        let setting = get_string(nbt, &setting_key(side.name));
        let mut state = connection_state(
            setting.as_deref().unwrap_or("auto"),
            neighbor_entity.as_ref().map(|entity| entity.id.as_str()),
        );
        if let Some(neighbor_entity) = neighbor_entity.as_ref() {
            if spec(&neighbor_entity.id).is_some() {
                let opposite = NEIGHBORS
                    .iter()
                    .find(|other| other.offset == side.opposite)
                    .unwrap();
                let neighbor_setting =
                    get_string(&neighbor_entity.nbt, &setting_key(opposite.name));
                let own_setting = setting.as_deref().unwrap_or("auto");
                let other_setting = neighbor_setting.as_deref().unwrap_or("auto");
                state = if modes_connect(own_setting, other_setting) {
                    "pipe"
                } else {
                    "none"
                };
            }
        }
        host.set_block_property(pos, side.name, state);
    }
}

fn connection_state(setting: &str, neighbor_id: Option<&str>) -> &'static str {
    if setting == "disabled" {
        return "none";
    }
    if neighbor_id.is_some_and(|id| spec(id).is_some()) {
        return "pipe";
    }
    match setting {
        "input" => "input",
        "output" => "output",
        "auto" if neighbor_id.is_some_and(is_heat_source) => "output",
        "auto" if neighbor_id.is_some_and(is_heat_target) => "input",
        _ => "none",
    }
}

fn is_heat_source(id: &str) -> bool {
    matches!(
        id,
        "arcforge:firebox"
            | "arcforge:firebox_array_controller"
            | "arcforge:geothermal_plant"
            | "arcforge:solar_thermal_array_controller"
            | "arcforge:fuel_burner"
    )
}

fn is_heat_target(id: &str) -> bool {
    matches!(
        id,
        "arcforge:fiberizer"
            | "arcforge:vulcanizer"
            | "arcforge:grain_dryer"
            | "arcforge:thermoelectric_plant"
    )
}

fn can_output(setting: Option<&str>) -> bool {
    matches!(setting, None | Some("auto") | Some("output"))
}

fn can_input(setting: Option<&str>) -> bool {
    matches!(setting, None | Some("auto") | Some("input"))
}

fn modes_connect(first: &str, second: &str) -> bool {
    (can_output(Some(first)) && can_input(Some(second)))
        || (can_output(Some(second)) && can_input(Some(first)))
}

fn modes_connect_both_ways(first: &str, second: &str) -> bool {
    can_output(Some(first))
        && can_input(Some(first))
        && can_output(Some(second))
        && can_input(Some(second))
}

fn setting_key(side: &str) -> String {
    format!("setting_{side}")
}

fn add(pos: [i32; 3], offset: [i32; 3]) -> [i32; 3] {
    [pos[0] + offset[0], pos[1] + offset[1], pos[2] + offset[2]]
}

fn get_i32(nbt: &Compound, key: &str) -> i32 {
    match nbt.get(&key.into()) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn get_string(nbt: &Compound, key: &str) -> Option<String> {
    match nbt.get(&key.into()) {
        Some(Tag::String(value)) => Some(value.to_string_lossy()),
        _ => None,
    }
}

fn set_i32(nbt: &mut Compound, key: &str, value: i32) {
    nbt.insert(key.into(), Tag::Int(value));
}

fn set_string(nbt: &mut Compound, key: &str, value: &str) {
    nbt.insert(key.into(), Tag::String(value.into()));
}
