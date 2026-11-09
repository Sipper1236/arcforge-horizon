//! Mod-side pooled FE conduit networks and bounded adjacent transfer.

use horizon_client::{
    containers::ModBlockEntity,
    mod_api::{Compound, Host, Tag},
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
    sync::{LazyLock, Mutex},
};

use crate::energy::EnergyStore;

pub(crate) const WROUGHT: &str = "arcforge:wrought_energy_conduit";
pub(crate) const TEMPERED: &str = "arcforge:tempered_energy_conduit";
pub(crate) const HARDENED: &str = "arcforge:hardened_energy_conduit";
pub(crate) const ARCFORGED: &str = "arcforge:arcforged_energy_conduit";

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
    offset: Pos,
    opposite: Pos,
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
    bidirectional: bool,
}

#[derive(Clone, Debug)]
struct Network {
    leader: Pos,
    members: Vec<Pos>,
    rate: u64,
    base_capacity: u64,
    capacity: u64,
    stored: u64,
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

pub(crate) fn spec(id: &str) -> Option<u64> {
    match id {
        WROUGHT => Some(256),
        TEMPERED => Some(1_024),
        HARDENED => Some(8_192),
        ARCFORGED => Some(262_144),
        _ => None,
    }
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

fn setting_key(face: &str) -> String {
    format!("setting_{face}")
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
                .find(|other| other.offset == side.opposite)?;
            let other_mode = get_string(&neighbor.nbt, &setting_key(opposite.name))
                .unwrap_or_else(|| "auto".into());
            Some(NeighborState {
                pos: neighbor_pos,
                id: neighbor.id.clone(),
                bidirectional: modes_connect_both_ways(&own_mode, &other_mode),
                own_mode,
                other_mode,
            })
        })
        .collect()
}

fn network_for(host: &dyn Host, pos: Pos, current: CurrentEntity<'_>) -> Option<Network> {
    let entity = entity_at(host, pos, current)?;
    spec(&entity.id)?;
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
    let mut rate = spec(&first.id)?;
    let mut queue = VecDeque::from([start]);
    let mut seen = HashSet::from([start]);
    let mut neighbors = HashMap::new();
    while let Some(pos) = queue.pop_front() {
        let Some(entity) = entity_at(host, pos, current) else {
            continue;
        };
        let Some(node_rate) = spec(&entity.id) else {
            continue;
        };
        rate = rate.min(node_rate);
        let edges = neighbor_states(host, pos, &entity, current);
        for edge in edges.iter().filter(|edge| edge.bidirectional) {
            if seen.insert(edge.pos) {
                queue.push_back(edge.pos);
            }
        }
        neighbors.insert(pos, edges);
    }
    let mut members: Vec<_> = seen.into_iter().collect();
    members.sort_unstable();
    let leader = *members.first()?;
    let network_capacity = rate
        .saturating_mul(members.len() as u64)
        .min(i32::MAX as u64);
    let mut stored = 0_u64;
    for member in &members {
        if let Some(entity) = entity_at(host, *member, current) {
            stored = stored.saturating_add(get_i32(&entity.nbt, "energy").max(0) as u64);
        }
    }
    // Older Arcforge-for-Horizon saves used a 50,000 FE buffer on every pipe.
    // Preserve that already-saved charge; the temporary larger capacity drains
    // away as the pool is used and never permits the network to lose FE on load.
    let base_capacity = network_capacity;
    let capacity = base_capacity.max(stored.min(i32::MAX as u64));
    Some(Network {
        leader,
        members,
        rate,
        base_capacity,
        capacity,
        stored: stored.min(capacity),
        last_moved_tick: None,
        neighbors,
        input_tick: None,
        input_used: 0,
        presence_check_tick: None,
        next_presence_check: 0,
    })
}

fn update_cached_pool(host: &dyn Host, network: &Network, stored: u64) {
    let key = world_key(host);
    let mut cache = NETWORK_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(entry) = cache
        .worlds
        .get_mut(&key)
        .and_then(|world| world.networks.get_mut(&network.leader))
    {
        entry.capacity = network.capacity;
        entry.stored = stored.min(entry.capacity);
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

fn reconcile_pool(host: &dyn Host, network: &Network, current: CurrentEntity<'_>) -> Network {
    let mut refreshed = network.clone();
    let mut stored = 0_u64;
    for member in &network.members {
        if let Some(entity) = entity_at(host, *member, current) {
            stored = stored.saturating_add(get_i32(&entity.nbt, "energy").max(0) as u64);
        }
    }
    // Horizon exposes per-block capability buffers without a transfer hook.
    // Preserve any energy inserted through those buffers, even if it temporarily
    // exceeds the network's normal source-sized capacity.
    let stored = stored.min(i32::MAX as u64);
    refreshed.capacity = refreshed.base_capacity.max(stored).min(i32::MAX as u64);
    refreshed.stored = stored.min(refreshed.capacity);
    update_cached_pool(host, &refreshed, refreshed.stored);
    refreshed
}

fn reserve_input(host: &dyn Host, network: &Network, amount: u64) -> u64 {
    let Some(tick) = game_time(host) else {
        return amount.min(network.rate);
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
    let reserved = amount.min(entry.rate.saturating_sub(entry.input_used));
    entry.input_used = entry.input_used.saturating_add(reserved);
    reserved
}

fn release_input(host: &dyn Host, network: &Network, unused: u64) {
    if unused == 0 {
        return;
    }
    let Some(tick) = game_time(host) else {
        return;
    };
    let key = world_key(host);
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

pub(crate) fn accept_energy(
    host: &mut dyn Host,
    pos: Pos,
    source_pos: Pos,
    source: &mut EnergyStore,
    rate: u64,
) -> u64 {
    let Some(entity) = host.mod_block_entity(pos) else {
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
    let network = reconcile_pool(host, &network, None);
    let requested = source
        .stored_fe()
        .min(rate)
        .min(network.capacity.saturating_sub(network.stored));
    let reserved = reserve_input(host, &network, requested);
    let moved = source.extract(reserved, reserved);
    if moved > 0 {
        let mut updated = network.clone();
        updated.stored = network.stored.saturating_add(moved).min(network.capacity);
        updated.capacity = updated.base_capacity.max(updated.stored);
        updated.last_moved_tick = game_time(host);
        distribute_network_energy(host, &updated, updated.stored, None);
        set_network_active(host, &updated, true);
        update_cached_pool(host, &updated, updated.stored);
    }
    release_input(host, &network, reserved.saturating_sub(moved));
    moved
}

fn write_network_energy(entity: &mut ModBlockEntity, stored: u64, rate: u64) {
    set_i32(
        &mut entity.nbt,
        "energy",
        stored.min(i32::MAX as u64) as i32,
    );
    entity.data.resize(2, 0);
    entity.data[0] = stored.min(i32::MAX as u64) as i32;
    entity.data[1] = rate.min(i32::MAX as u64) as i32;
}

fn distribute_network_energy(
    host: &mut dyn Host,
    network: &Network,
    total: u64,
    mut current: Option<(Pos, &mut ModBlockEntity)>,
) {
    let total = total.min(network.capacity);
    let count = network.members.len() as u64;
    let each = if count == 0 { 0 } else { total / count };
    let remainder = if count == 0 { 0 } else { total % count };
    for (index, pos) in network.members.iter().enumerate() {
        let amount = each + u64::from((index as u64) < remainder);
        if current
            .as_ref()
            .is_some_and(|(current_pos, _)| current_pos == pos)
        {
            let (_, entity) = current.as_mut().expect("current node was checked");
            write_network_energy(entity, amount, network.rate);
        } else {
            let mut changed = false;
            host.change_mod_block_entity(*pos, &mut |entity| {
                if spec(&entity.id).is_some() {
                    if get_i32(&entity.nbt, "energy").max(0) as u64 != amount
                        || entity.data.first().copied() != Some(amount as i32)
                        || entity.data.get(1).copied() != Some(network.rate as i32)
                    {
                        write_network_energy(entity, amount, network.rate);
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
    pos: Pos,
    entity: &mut ModBlockEntity,
    mut transfer_to_neighbor: impl FnMut(&mut dyn Host, Pos, Pos, &mut EnergyStore, u64) -> u64,
) {
    let Some(network) = network_for(host, pos, Some((pos, entity))) else {
        return;
    };
    update_connections(host, pos, &entity.nbt);
    if pos != network.leader {
        return;
    }
    let network = reconcile_pool(host, &network, Some((pos, entity)));
    let mut energy = EnergyStore::new(network.capacity.max(1), network.stored);
    let endpoints = network_output_endpoints(host, &network, Some((pos, entity)));
    let stored_before_output = energy.stored_fe();
    distribute_energy(
        host,
        &endpoints,
        &mut energy,
        network.rate,
        &mut transfer_to_neighbor,
    );
    let mut updated = network.clone();
    updated.stored = energy.stored_fe();
    updated.capacity = updated.base_capacity.max(updated.stored);
    let transferred = stored_before_output.saturating_sub(updated.stored);
    let tick = game_time(host);
    if transferred > 0 {
        updated.last_moved_tick = tick;
    }
    distribute_network_energy(host, &updated, updated.stored, Some((pos, entity)));
    let recently_moved = tick
        .zip(updated.last_moved_tick)
        .is_some_and(|(now, last)| now >= last && now - last < 10);
    set_network_active(host, &updated, updated.stored > 0 || recently_moved);
    update_cached_pool(host, &updated, updated.stored);
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
                if network.members.binary_search(&neighbor).is_ok() {
                    continue;
                }
                let Some(opposite) = NEIGHBORS.iter().find(|other| other.offset == side.opposite)
                else {
                    continue;
                };
                if can_input(get_string(&target.nbt, &setting_key(opposite.name)).as_deref()) {
                    endpoints.push((*pos, neighbor));
                }
            } else if is_energy_target(&target.id) {
                endpoints.push((*pos, neighbor));
            }
        }
    }
    endpoints
}

fn distribute_energy(
    host: &mut dyn Host,
    endpoints: &[(Pos, Pos)],
    energy: &mut EnergyStore,
    rate: u64,
    transfer: &mut impl FnMut(&mut dyn Host, Pos, Pos, &mut EnergyStore, u64) -> u64,
) {
    let mut open: Vec<_> = endpoints.to_vec();
    if !open.is_empty() {
        let tick = game_time(host).unwrap_or_default();
        let start = tick.rem_euclid(open.len() as i64) as usize;
        open.rotate_left(start);
    }
    let mut remaining = rate.min(energy.stored_fe());
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
            let moved = transfer(host, source, sink, energy, share.min(remaining));
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

pub(crate) fn update_connections(host: &mut dyn Host, pos: Pos, nbt: &Compound) {
    for side in NEIGHBORS {
        let neighbor = add(pos, side.offset);
        let neighbor_entity = host.mod_block_entity(neighbor);
        let setting = get_string(nbt, &setting_key(side.name));
        let state = if let Some(neighbor_entity) = neighbor_entity.as_ref() {
            if spec(&neighbor_entity.id).is_some() {
                let opposite = NEIGHBORS.iter().find(|other| other.offset == side.opposite);
                let other_mode = opposite
                    .and_then(|other| get_string(&neighbor_entity.nbt, &setting_key(other.name)))
                    .unwrap_or_else(|| "auto".into());
                if modes_connect(&setting.as_deref().unwrap_or("auto"), &other_mode) {
                    "pipe"
                } else {
                    "none"
                }
            } else {
                connection_state(
                    setting.as_deref().unwrap_or("auto"),
                    Some(neighbor_entity.id.as_str()),
                )
            }
        } else {
            "none"
        };
        if host.block_property(pos, side.name).as_deref() != Some(state) {
            host.set_block_property(pos, side.name, state);
        }
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
        "auto" if neighbor_id.is_some_and(is_energy_source) => "output",
        "auto" if neighbor_id.is_some_and(is_energy_target) => "input",
        _ => "none",
    }
}

fn is_energy_source(id: &str) -> bool {
    matches!(
        id,
        "arcforge:combustion_plant" | "arcforge:thermoelectric_plant"
    )
}

fn is_energy_target(id: &str) -> bool {
    matches!(
        id,
        "arcforge:arc_crusher"
            | "arcforge:arc_crushing_array"
            | "arcforge:arc_crushing_array_casing"
            | "arcforge:metal_press"
            | "arcforge:induction_furnace"
            | "arcforge:infuser"
            | "arcforge:sifter"
            | "arcforge:fiberizer"
    )
}

fn can_output(setting: Option<&str>) -> bool {
    matches!(setting, None | Some("auto") | Some("output"))
}

fn can_input(setting: Option<&str>) -> bool {
    matches!(setting, None | Some("auto") | Some("input"))
}

pub(crate) fn modes_connect(first: &str, second: &str) -> bool {
    (can_output(Some(first)) && can_input(Some(second)))
        || (can_output(Some(second)) && can_input(Some(first)))
}

fn modes_connect_both_ways(first: &str, second: &str) -> bool {
    can_output(Some(first))
        && can_input(Some(first))
        && can_output(Some(second))
        && can_input(Some(second))
}

fn add(pos: Pos, offset: Pos) -> Pos {
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
