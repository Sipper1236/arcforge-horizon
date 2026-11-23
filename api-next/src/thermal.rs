//! Current-API thermal conduit pools with bounded, directed face transfer.

use mod_api::{
    BlockEntityTickEvent, BlockEntityTickReply, Compound, Host, LevelRef, ServerHost, Tag, Text,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{LazyLock, Mutex},
};

pub(crate) const WROUGHT: &str = "arcforge:wrought_thermal_conduit";
pub(crate) const TEMPERED: &str = "arcforge:tempered_thermal_conduit";
pub(crate) const HARDENED: &str = "arcforge:hardened_thermal_conduit";
pub(crate) const ARCFORGED: &str = "arcforge:arcforged_thermal_conduit";

const MAX_TEMPERATURE_C: u32 = 2_000;
const SIDES: [Side; 6] = [
    Side {
        name: "up",
        offset: [0, 1, 0],
        opposite: 1,
    },
    Side {
        name: "down",
        offset: [0, -1, 0],
        opposite: 0,
    },
    Side {
        name: "north",
        offset: [0, 0, -1],
        opposite: 3,
    },
    Side {
        name: "south",
        offset: [0, 0, 1],
        opposite: 2,
    },
    Side {
        name: "west",
        offset: [-1, 0, 0],
        opposite: 5,
    },
    Side {
        name: "east",
        offset: [1, 0, 0],
        opposite: 4,
    },
];

#[derive(Clone, Copy)]
struct Side {
    name: &'static str,
    offset: [i32; 3],
    opposite: usize,
}

#[derive(Clone)]
struct Node {
    id: String,
    nbt: Compound,
}

#[derive(Clone)]
struct Network {
    leader: [i32; 3],
    members: Vec<[i32; 3]>,
    rate: u64,
    capacity: u64,
    stored: u64,
    temperature: u32,
    member_heat: HashMap<[i32; 3], (u64, i32)>,
    last_moved_tick: Option<i64>,
    neighbors: HashMap<[i32; 3], Vec<NeighborState>>,
    input_tick: Option<i64>,
    input_used: u64,
    presence_check_tick: Option<i64>,
    next_presence_check: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NeighborState {
    pos: [i32; 3],
    id: String,
    own_mode: String,
    other_mode: String,
    connected: bool,
}

#[derive(Default)]
struct LevelNetworks {
    by_pos: HashMap<[i32; 3], [i32; 3]>,
    networks: HashMap<[i32; 3], Network>,
}

#[derive(Default)]
struct NetworkCache {
    levels: HashMap<LevelRef, LevelNetworks>,
}

static NETWORK_CACHE: LazyLock<Mutex<NetworkCache>> =
    LazyLock::new(|| Mutex::new(NetworkCache::default()));

fn neighbor_states(
    host: &dyn Host,
    level: LevelRef,
    pos: [i32; 3],
    current: Option<([i32; 3], &str, &Compound)>,
) -> Vec<NeighborState> {
    let Some(node) = node_at(host, level, pos, current) else {
        return Vec::new();
    };
    SIDES
        .iter()
        .enumerate()
        .filter_map(|(index, side)| {
            let neighbor_pos = add(pos, side.offset);
            let neighbor = node_at(host, level, neighbor_pos, current)?;
            let own_mode = mode(&node.nbt, index);
            let other_mode = mode(&neighbor.nbt, side.opposite);
            let connected = connects_both_ways(&own_mode, &other_mode);
            Some(NeighborState {
                pos: neighbor_pos,
                id: neighbor.id,
                own_mode,
                other_mode,
                connected,
            })
        })
        .collect()
}

pub(crate) fn ids() -> [&'static str; 4] {
    [WROUGHT, TEMPERED, HARDENED, ARCFORGED]
}

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

fn node_at(
    host: &dyn Host,
    level: LevelRef,
    pos: [i32; 3],
    current: Option<([i32; 3], &str, &Compound)>,
) -> Option<Node> {
    if let Some((current_pos, id, nbt)) = current
        && current_pos == pos
    {
        return spec(id).map(|_| Node {
            id: id.to_owned(),
            nbt: nbt.clone(),
        });
    }
    let id = level.block(host, pos)?.id(host).to_owned();
    if spec(&id).is_none() {
        return None;
    }
    let entity = host.block_entity(level, pos)?;
    Some(Node {
        id,
        nbt: entity.data(host)?,
    })
}

fn network_for(
    host: &dyn Host,
    level: LevelRef,
    start: [i32; 3],
    current: Option<([i32; 3], &str, &Compound)>,
) -> Option<Network> {
    let first = node_at(host, level, start, current)?;
    let (first_rate, _) = spec(&first.id)?;
    let current_neighbors = neighbor_states(host, level, start, current);
    let tick = level.game_time(host);
    let mut cached = None;
    let mut presence_check = None;
    {
        let mut cache = NETWORK_CACHE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(world) = cache.levels.get_mut(&level) {
            if let Some(leader) = world.by_pos.get(&start).copied() {
                if let Some(network) = world.networks.get(&leader) {
                    if network.neighbors.get(&start) == Some(&current_neighbors)
                        && network.member_heat.get(&start)
                            == Some(&(
                                get_i32(&first.nbt, "HeatHU").max(0) as u64,
                                get_i32(&first.nbt, "TemperatureC"),
                            ))
                    {
                        cached = Some(network.clone());
                    }
                }
                if cached.is_none() {
                    cache.levels.remove(&level);
                } else if let Some(tick) = tick
                    && let Some(network) = cache
                        .levels
                        .get_mut(&level)
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
    if let Some(network) = cached.as_ref() {
        if presence_check.is_none_or(|pos| {
            node_at(host, level, pos, current).is_some_and(|node| {
                network.member_heat.get(&pos)
                    == Some(&(
                        get_i32(&node.nbt, "HeatHU").max(0) as u64,
                        get_i32(&node.nbt, "TemperatureC"),
                    ))
            })
        }) {
            return Some(network.clone());
        }
        NETWORK_CACHE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .levels
            .remove(&level);
    }

    let network = discover_network(host, level, start, current, first_rate)?;
    let mut cache = NETWORK_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let world = cache.levels.entry(level).or_default();
    for member in &network.members {
        world.by_pos.insert(*member, network.leader);
    }
    world.networks.insert(network.leader, network.clone());
    Some(network)
}

fn discover_network(
    host: &dyn Host,
    level: LevelRef,
    start: [i32; 3],
    current: Option<([i32; 3], &str, &Compound)>,
    first_rate: u64,
) -> Option<Network> {
    let mut queue = VecDeque::from([start]);
    let mut seen = HashSet::from([start]);
    let mut rate = first_rate;
    let mut neighbors = HashMap::new();

    while let Some(pos) = queue.pop_front() {
        let node = node_at(host, level, pos, current)?;
        let (node_rate, _) = spec(&node.id)?;
        rate = rate.min(node_rate);
        let edges = neighbor_states(host, level, pos, current);
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
    let mut member_heat = HashMap::new();
    for pos in &members {
        let Some(node) = node_at(host, level, *pos, current) else {
            continue;
        };
        let node_stored = get_i32(&node.nbt, "HeatHU").max(0) as u64;
        let node_temperature = get_i32(&node.nbt, "TemperatureC");
        member_heat.insert(*pos, (node_stored, node_temperature));
        stored = stored.saturating_add(node_stored);
        if node_stored > 0 {
            temperature = temperature.max(node_temperature.max(20) as u32);
        }
    }
    Some(Network {
        leader,
        members,
        rate,
        capacity,
        stored: stored.min(capacity),
        temperature: if stored == 0 { 20 } else { temperature },
        member_heat,
        last_moved_tick: None,
        neighbors,
        input_tick: None,
        input_used: 0,
        presence_check_tick: None,
        next_presence_check: 0,
    })
}

pub(crate) fn accept_heat(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    source_pos: [i32; 3],
    source: &mut super::heat_cell::HeatStore,
    rate: u64,
) -> u64 {
    let Some(node) = node_at(host, level, pos, None) else {
        return 0;
    };
    let incoming = [
        source_pos[0] - pos[0],
        source_pos[1] - pos[1],
        source_pos[2] - pos[2],
    ];
    let Some(index) = SIDES.iter().position(|side| side.offset == incoming) else {
        return 0;
    };
    if !can_input(&mode(&node.nbt, index)) {
        return 0;
    }
    let Some(network) = network_for(host, level, pos, None) else {
        return 0;
    };
    let source_temperature = source.temperature_c();
    if network.stored > 0 && source_temperature < network.temperature {
        return 0;
    }
    let Some((_, max_insert)) = spec(&node.id) else {
        return 0;
    };
    let available = network.capacity.saturating_sub(network.stored);
    let requested = source.stored_hu().min(rate.min(max_insert)).min(available);
    let reserved = reserve_input(host, level, &network, requested);
    if reserved == 0 {
        return 0;
    }
    let moved = source.remove_hu(reserved);
    if moved > 0 {
        let temperature = source_temperature.max(network.temperature);
        let last_moved_tick = level.game_time(host).or(network.last_moved_tick);
        distribute_network_heat(
            host,
            level,
            &network,
            network.stored.saturating_add(moved),
            temperature,
            None,
        );
        set_network_active(host, level, &network, true);
        update_cached_pool(
            level,
            &network,
            network.stored.saturating_add(moved),
            temperature,
            last_moved_tick,
        );
    }
    release_input(host, level, &network, reserved.saturating_sub(moved));
    moved
}

fn reserve_input(
    host: &mut dyn ServerHost,
    level: LevelRef,
    network: &Network,
    requested: u64,
) -> u64 {
    let Some(tick) = level.game_time(host) else {
        return requested.min(network.rate);
    };
    let mut cache = NETWORK_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(entry) = cache
        .levels
        .get_mut(&level)
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

fn release_input(host: &mut dyn ServerHost, level: LevelRef, network: &Network, unused: u64) {
    if unused == 0 {
        return;
    }
    let Some(tick) = level.game_time(host) else {
        return;
    };
    let mut cache = NETWORK_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(entry) = cache
        .levels
        .get_mut(&level)
        .and_then(|world| world.networks.get_mut(&network.leader))
        && entry.input_tick == Some(tick)
    {
        entry.input_used = entry.input_used.saturating_sub(unused);
    }
}

fn update_cached_pool(
    level: LevelRef,
    network: &Network,
    stored: u64,
    temperature: u32,
    last_moved_tick: Option<i64>,
) {
    let mut cache = NETWORK_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(entry) = cache
        .levels
        .get_mut(&level)
        .and_then(|world| world.networks.get_mut(&network.leader))
    {
        entry.stored = stored.min(entry.capacity);
        entry.temperature = if entry.stored == 0 { 20 } else { temperature };
        entry.last_moved_tick = last_moved_tick;
        let count = entry.members.len() as u64;
        let each = if count == 0 { 0 } else { entry.stored / count };
        let remainder = if count == 0 { 0 } else { entry.stored % count };
        let member_positions = entry.members.clone();
        for (index, pos) in member_positions.into_iter().enumerate() {
            let member_stored = each + u64::from((index as u64) < remainder);
            let member_temperature = if member_stored == 0 {
                20
            } else {
                entry.temperature as i32
            };
            entry
                .member_heat
                .insert(pos, (member_stored, member_temperature));
        }
    }
}

pub(crate) fn invalidate_level(level: LevelRef) {
    NETWORK_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .levels
        .remove(&level);
}

fn distribute_network_heat(
    host: &mut dyn ServerHost,
    level: LevelRef,
    network: &Network,
    stored: u64,
    temperature: u32,
    current_pos: Option<[i32; 3]>,
) {
    let total = stored.min(network.capacity);
    let count = network.members.len() as u64;
    let each = if count == 0 { 0 } else { total / count };
    let remainder = if count == 0 { 0 } else { total % count };
    for (index, pos) in network.members.iter().enumerate() {
        if current_pos == Some(*pos) {
            continue;
        }
        let node_stored = each + u64::from((index as u64) < remainder);
        let Some(entity) = host.block_entity(level, *pos) else {
            continue;
        };
        let Some(nbt) = entity.data(&*host) else {
            continue;
        };
        let node_temperature = if node_stored == 0 { 20 } else { temperature };
        if get_i32(&nbt, "HeatHU").max(0) as u64 == node_stored
            && get_i32(&nbt, "TemperatureC") == node_temperature as i32
        {
            continue;
        }
        let mut updates = Compound::new();
        set_i32(
            &mut updates,
            "HeatHU",
            node_stored.min(i32::MAX as u64) as i32,
        );
        set_i32(&mut updates, "TemperatureC", node_temperature as i32);
        let _ = entity.merge_data(host, updates);
    }
}

fn assigned_heat(network: &Network, pos: [i32; 3], total: u64) -> u64 {
    let count = network.members.len() as u64;
    if count == 0 {
        return 0;
    }
    let index = network.members.binary_search(&pos).unwrap_or(usize::MAX);
    if index == usize::MAX {
        return 0;
    }
    total / count + u64::from((index as u64) < total % count)
}

fn set_network_active(host: &mut dyn ServerHost, level: LevelRef, network: &Network, active: bool) {
    let state = if active { "true" } else { "false" };
    for pos in &network.members {
        if level.block_property(host, *pos, "active").as_deref() != Some(state) {
            let _ = host.set_block_property(*pos, "active", state);
        }
    }
}

pub(crate) fn tick(host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>, id: &str) {
    *event.reply = BlockEntityTickReply::Unchanged;
    let Some(level) = host.level(host.dimension()) else {
        return;
    };
    let Some(network) = network_for(
        host,
        level,
        event.pos,
        Some((event.pos, id, &event.entity.nbt)),
    ) else {
        return;
    };
    let mut changed = update_connections(host, level, event.pos, &event.entity.nbt);
    if event.pos != network.leader {
        if changed {
            *event.reply = BlockEntityTickReply::Changed;
        }
        return;
    }

    let mut heat = super::heat_cell::HeatStore::new(
        network.capacity.max(1),
        MAX_TEMPERATURE_C,
        network.stored,
    );
    heat.set_source_temperature(network.temperature);
    let endpoints = output_endpoints(
        host,
        level,
        &network,
        Some((event.pos, id, &event.entity.nbt)),
    );
    let initial = heat.stored_hu();
    distribute_heat(host, level, &network, &endpoints, &mut heat);
    let remaining = heat.stored_hu();
    let temperature = if remaining == 0 {
        20
    } else {
        network.temperature
    };
    let tick = level.game_time(host);
    let transferred = initial.saturating_sub(remaining);
    let last_moved_tick = if transferred > 0 {
        tick.or(network.last_moved_tick)
    } else {
        network.last_moved_tick
    };
    distribute_network_heat(
        host,
        level,
        &network,
        remaining,
        temperature,
        Some(event.pos),
    );
    let own_stored = assigned_heat(&network, event.pos, remaining);
    changed |= set_i32(
        &mut event.entity.nbt,
        "HeatHU",
        own_stored.min(i32::MAX as u64) as i32,
    );
    changed |= set_i32(
        &mut event.entity.nbt,
        "TemperatureC",
        if own_stored == 0 {
            20
        } else {
            temperature as i32
        },
    );
    let recently_moved = tick
        .zip(last_moved_tick)
        .is_some_and(|(now, last)| now >= last && now - last < 10);
    set_network_active(host, level, &network, remaining > 0 || recently_moved);
    update_cached_pool(level, &network, remaining, temperature, last_moved_tick);
    changed |= initial != remaining;
    if changed {
        *event.reply = BlockEntityTickReply::Changed;
    }
}

fn output_endpoints(
    host: &dyn Host,
    level: LevelRef,
    network: &Network,
    current: Option<([i32; 3], &str, &Compound)>,
) -> Vec<([i32; 3], [i32; 3])> {
    let mut endpoints = Vec::new();
    for pos in &network.members {
        let Some(node) = node_at(host, level, *pos, current) else {
            continue;
        };
        for (index, side) in SIDES.iter().enumerate() {
            if !can_output(&mode(&node.nbt, index)) {
                continue;
            }
            let target_pos = add(*pos, side.offset);
            let Some(target) = node_at(host, level, target_pos, None) else {
                let Some(target_id) = level
                    .block(host, target_pos)
                    .map(|block| block.id(host).to_owned())
                else {
                    continue;
                };
                if is_heat_target(&target_id) {
                    endpoints.push((*pos, target_pos));
                }
                continue;
            };
            if network.members.binary_search(&target_pos).is_ok() {
                continue;
            }
            if can_input(&mode(&target.nbt, side.opposite)) {
                endpoints.push((*pos, target_pos));
            }
        }
    }
    endpoints
}

fn distribute_heat(
    host: &mut dyn ServerHost,
    level: LevelRef,
    network: &Network,
    endpoints: &[([i32; 3], [i32; 3])],
    heat: &mut super::heat_cell::HeatStore,
) {
    let mut open = endpoints.to_vec();
    if let Some(tick) = level.game_time(host)
        && !open.is_empty()
    {
        let start = tick.rem_euclid(open.len() as i64) as usize;
        open.rotate_left(start);
    }
    let mut remaining = network.rate.min(heat.stored_hu());
    while remaining > 0 && !open.is_empty() {
        let count = open.len() as u64;
        let each = remaining / count;
        let remainder = remaining % count;
        let mut next = Vec::with_capacity(open.len());
        for (index, (source_pos, target_pos)) in open.iter().copied().enumerate() {
            let share = each + u64::from((index as u64) < remainder);
            if share == 0 {
                next.push((source_pos, target_pos));
                continue;
            }
            let moved = transfer_to_neighbor(
                host,
                level,
                source_pos,
                target_pos,
                heat,
                share.min(remaining),
            );
            remaining = remaining.saturating_sub(moved);
            if moved >= share {
                next.push((source_pos, target_pos));
            }
        }
        if next.len() == open.len() {
            break;
        }
        open = next;
    }
}

fn transfer_to_neighbor(
    host: &mut dyn ServerHost,
    level: LevelRef,
    source_pos: [i32; 3],
    target_pos: [i32; 3],
    heat: &mut super::heat_cell::HeatStore,
    rate: u64,
) -> u64 {
    let incoming_face = [
        source_pos[0] - target_pos[0],
        source_pos[1] - target_pos[1],
        source_pos[2] - target_pos[2],
    ];
    let target_id = level
        .block(host, target_pos)
        .map(|block| block.id(host).to_owned());
    let Some(target_id) = target_id else {
        return 0;
    };
    if spec(&target_id).is_some() {
        return accept_heat(host, level, target_pos, source_pos, heat, rate);
    }
    if super::heat_cell::ids().contains(&target_id.as_str()) {
        return super::heat_cell::accept_heat(host, level, target_pos, heat, rate);
    }
    if target_id == super::THERMOELECTRIC_PLANT {
        let Some(face) = SIDES.iter().position(|side| side.offset == incoming_face) else {
            return 0;
        };
        return super::thermoelectric::accept_heat(host, level, target_pos, face, heat, rate);
    }
    if target_id == super::CONTROLLER {
        return super::cracking_unit::accept_heat(host, level, target_pos, heat, rate);
    }
    0
}

pub(crate) fn wrench(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    face: [i32; 3],
    player_uuid: u128,
) {
    let Some(index) = SIDES.iter().position(|side| side.offset == face) else {
        return;
    };
    let Some(entity) = host.block_entity(level, pos) else {
        return;
    };
    let Some(nbt) = entity.data(&*host) else {
        return;
    };
    let next = match get_string(&nbt, &setting_key(SIDES[index].name)).as_deref() {
        None | Some("auto") => "input",
        Some("input") => "output",
        Some("output") => "disabled",
        _ => "auto",
    };
    let mut updates = Compound::new();
    set_string(&mut updates, &setting_key(SIDES[index].name), next);
    let _ = entity.merge_data(host, updates);
    invalidate_level(level);
    if let Some(player) = host.player(player_uuid) {
        let _ = player.action_bar(
            host,
            Text::literal(format!("Thermal conduit {}: {next}", SIDES[index].name)),
        );
    }
}

fn update_connections(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    nbt: &Compound,
) -> bool {
    let id = level
        .block(host, pos)
        .map(|block| block.id(host).to_owned());
    let Some(id) = id else {
        return false;
    };
    let Some(node) = node_at(host, level, pos, Some((pos, &id, nbt))) else {
        return false;
    };
    let mut changed = false;
    for (index, side) in SIDES.iter().enumerate() {
        let neighbor_pos = add(pos, side.offset);
        let neighbor_id = level
            .block(host, neighbor_pos)
            .map(|block| block.id(host).to_owned());
        let own_mode = mode(&node.nbt, index);
        let state = if let Some(neighbor) = node_at(host, level, neighbor_pos, None) {
            if connects(&own_mode, &mode(&neighbor.nbt, side.opposite)) {
                "pipe"
            } else {
                "none"
            }
        } else {
            match own_mode.as_str() {
                "disabled" => "none",
                "input" => "input",
                "output" => "output",
                _ if neighbor_id.as_deref().is_some_and(is_heat_source) => "output",
                _ if neighbor_id.as_deref().is_some_and(is_heat_target) => "input",
                _ => "none",
            }
        };
        if level.block_property(host, pos, side.name).as_deref() != Some(state) {
            changed |= host.set_block_property(pos, side.name, state);
        }
    }
    changed
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
    super::heat_cell::ids().contains(&id)
        || matches!(id, super::THERMOELECTRIC_PLANT | super::CONTROLLER)
}

fn mode(nbt: &Compound, index: usize) -> String {
    get_string(nbt, &setting_key(SIDES[index].name)).unwrap_or_else(|| "auto".to_owned())
}

fn can_output(mode: &str) -> bool {
    matches!(mode, "auto" | "output")
}

fn can_input(mode: &str) -> bool {
    matches!(mode, "auto" | "input")
}

fn connects(first: &str, second: &str) -> bool {
    (can_output(first) && can_input(second)) || (can_output(second) && can_input(first))
}

fn connects_both_ways(first: &str, second: &str) -> bool {
    can_output(first) && can_input(first) && can_output(second) && can_input(second)
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

fn set_i32(nbt: &mut Compound, key: &str, value: i32) -> bool {
    let key_tag = key.into();
    if matches!(nbt.get(&key_tag), Some(Tag::Int(current)) if *current == value) {
        return false;
    }
    nbt.insert(key_tag, Tag::Int(value));
    true
}

fn set_string(nbt: &mut Compound, key: &str, value: &str) {
    nbt.insert(key.into(), Tag::String(value.into()));
}
