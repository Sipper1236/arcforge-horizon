//! Mod-side fluid conduit routing through Horizon's transactional fluid move helper.

use horizon_client::{
    containers::ModBlockEntity,
    mod_api::{Direction, Host},
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
    sync::{LazyLock, Mutex},
};

use crate::{get_string, set_string};

pub(crate) const WROUGHT: &str = "arcforge:wrought_fluid_conduit";
pub(crate) const TEMPERED: &str = "arcforge:tempered_fluid_conduit";
pub(crate) const HARDENED: &str = "arcforge:hardened_fluid_conduit";
pub(crate) const ARCFORGED: &str = "arcforge:arcforged_fluid_conduit";
pub(crate) const WROUGHT_PRESSURIZED: &str = "arcforge:wrought_pressurized_conduit";
pub(crate) const TEMPERED_PRESSURIZED: &str = "arcforge:tempered_pressurized_conduit";
pub(crate) const HARDENED_PRESSURIZED: &str = "arcforge:hardened_pressurized_conduit";
pub(crate) const ARCFORGED_PRESSURIZED: &str = "arcforge:arcforged_pressurized_conduit";

const ARCFORGE_GASES: &[&str] = &[
    "arcforge:natural_gas",
    "arcforge:propylene",
    "arcforge:hydrogen_sulfide",
    "arcforge:refinery_gas",
    "arcforge:exhaust_steam",
    "arcforge:hydrogen",
    "arcforge:oxygen",
    "arcforge:carbon_dioxide",
    "arcforge:nitrogen",
    "arcforge:ammonia",
    "arcforge:biogas",
    "arcforge:chlorine",
    "arcforge:ethylene",
    "arcforge:syngas",
    "arcforge:steam",
    "arcforge:high_pressure_steam",
    "arcforge:superheated_steam",
    "arcforge:fluorine",
    "arcforge:uranium_hexafluoride",
    "arcforge:depleted_uranium_hexafluoride",
    "arcforge:contaminated_steam",
    "arcforge:nitrogen_dioxide",
];

type Pos = [i32; 3];

#[derive(Clone, Copy)]
struct Side {
    name: &'static str,
    offset: Pos,
    direction: Direction,
    opposite: Direction,
}

const SIDES: [Side; 6] = [
    Side {
        name: "down",
        offset: [0, -1, 0],
        direction: Direction::Down,
        opposite: Direction::Up,
    },
    Side {
        name: "up",
        offset: [0, 1, 0],
        direction: Direction::Up,
        opposite: Direction::Down,
    },
    Side {
        name: "north",
        offset: [0, 0, -1],
        direction: Direction::North,
        opposite: Direction::South,
    },
    Side {
        name: "south",
        offset: [0, 0, 1],
        direction: Direction::South,
        opposite: Direction::North,
    },
    Side {
        name: "west",
        offset: [-1, 0, 0],
        direction: Direction::West,
        opposite: Direction::East,
    },
    Side {
        name: "east",
        offset: [1, 0, 0],
        direction: Direction::East,
        opposite: Direction::West,
    },
];

#[derive(Clone, Copy)]
struct Tier {
    millibuckets_per_tick: i32,
    kind: ConduitKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ConduitKind {
    Fluid,
    Gas,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct WorldKey {
    directory: Option<PathBuf>,
    seed: Option<i64>,
    dimension: String,
}

#[derive(Clone)]
struct Network {
    members: Vec<Pos>,
    tier: Tier,
    edges: HashMap<Pos, Vec<(Pos, String, bool)>>,
    last_operation_tick: Option<i64>,
    selected_fluid: Option<String>,
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

fn tier(id: &str) -> Option<Tier> {
    Some(match id {
        WROUGHT => Tier {
            millibuckets_per_tick: 200,
            kind: ConduitKind::Fluid,
        },
        TEMPERED => Tier {
            millibuckets_per_tick: 800,
            kind: ConduitKind::Fluid,
        },
        HARDENED => Tier {
            millibuckets_per_tick: 3_200,
            kind: ConduitKind::Fluid,
        },
        ARCFORGED => Tier {
            millibuckets_per_tick: 12_800,
            kind: ConduitKind::Fluid,
        },
        WROUGHT_PRESSURIZED => Tier {
            millibuckets_per_tick: 400,
            kind: ConduitKind::Gas,
        },
        TEMPERED_PRESSURIZED => Tier {
            millibuckets_per_tick: 1_600,
            kind: ConduitKind::Gas,
        },
        HARDENED_PRESSURIZED => Tier {
            millibuckets_per_tick: 6_400,
            kind: ConduitKind::Gas,
        },
        ARCFORGED_PRESSURIZED => Tier {
            millibuckets_per_tick: 25_600,
            kind: ConduitKind::Gas,
        },
        _ => return None,
    })
}

fn carries(kind: ConduitKind, fluid: &str) -> bool {
    match kind {
        ConduitKind::Fluid => !ARCFORGE_GASES.contains(&fluid),
        ConduitKind::Gas => ARCFORGE_GASES.contains(&fluid),
    }
}

pub(crate) fn is_arcforge_gas(fluid: &str) -> bool {
    ARCFORGE_GASES.contains(&fluid)
}

fn same_network_kind(left: &str, right: &str) -> bool {
    tier(left)
        .zip(tier(right))
        .is_some_and(|(left, right)| left.kind == right.kind)
}

pub(crate) fn spec(id: &str) -> Option<i32> {
    tier(id).map(|tier| tier.millibuckets_per_tick)
}

fn add(pos: Pos, offset: Pos) -> Pos {
    [pos[0] + offset[0], pos[1] + offset[1], pos[2] + offset[2]]
}

fn entity_at(host: &dyn Host, pos: Pos, current: (Pos, &ModBlockEntity)) -> Option<ModBlockEntity> {
    if current.0 == pos {
        Some(current.1.clone())
    } else {
        host.mod_block_entity(pos)
    }
}

fn setting(entity: &ModBlockEntity, face: &str) -> String {
    get_string(&entity.nbt, &format!("setting_{face}")).unwrap_or_else(|| "auto".to_owned())
}

fn disabled(setting: &str) -> bool {
    setting == "disabled"
}

fn side_for_direction(direction: Direction) -> Side {
    SIDES
        .into_iter()
        .find(|side| side.direction == direction)
        .expect("all six directions are represented")
}

fn edge_signature(
    host: &dyn Host,
    pos: Pos,
    entity: &ModBlockEntity,
    current: (Pos, &ModBlockEntity),
) -> Vec<(Pos, String, bool)> {
    SIDES
        .into_iter()
        .filter_map(|side| {
            let neighbor_pos = add(pos, side.offset);
            let neighbor = entity_at(host, neighbor_pos, current)?;
            if !same_network_kind(&entity.id, &neighbor.id) {
                return None;
            }
            let connected = !disabled(&setting(entity, side.name))
                && !disabled(&setting(&neighbor, side_for_direction(side.opposite).name));
            Some((neighbor_pos, neighbor.id.clone(), connected))
        })
        .collect()
}

fn world_key(host: &dyn Host) -> WorldKey {
    WorldKey {
        directory: host.world_dir(),
        seed: host.world_seed(),
        dimension: host.dimension().to_owned(),
    }
}

fn discover(host: &dyn Host, start: Pos, entity: &ModBlockEntity) -> Option<Network> {
    let current = (start, entity);
    let mut seen = HashSet::from([start]);
    let mut queue = VecDeque::from([start]);
    let mut slowest = tier(&entity.id)?;
    let mut edges = HashMap::new();
    while let Some(pos) = queue.pop_front() {
        let conduit = entity_at(host, pos, current)?;
        let member_tier = tier(&conduit.id)?;
        slowest.millibuckets_per_tick = slowest
            .millibuckets_per_tick
            .min(member_tier.millibuckets_per_tick);
        let signature = edge_signature(host, pos, &conduit, current);
        for (next, _, connected) in &signature {
            if *connected && seen.insert(*next) {
                queue.push_back(*next);
            }
        }
        edges.insert(pos, signature);
    }
    let mut members: Vec<_> = seen.into_iter().collect();
    members.sort_unstable();
    Some(Network {
        members,
        tier: slowest,
        edges,
        last_operation_tick: None,
        selected_fluid: None,
    })
}

fn network_for(
    host: &dyn Host,
    start: Pos,
    entity: &ModBlockEntity,
) -> Option<(WorldKey, Pos, Network)> {
    let key = world_key(host);
    let current_edges = edge_signature(host, start, entity, (start, entity));
    {
        let cache = NETWORK_CACHE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(world) = cache.worlds.get(&key)
            && let Some(leader) = world.by_pos.get(&start).copied()
            && let Some(network) = world.networks.get(&leader)
            && network.edges.get(&start) == Some(&current_edges)
        {
            return Some((key, leader, network.clone()));
        }
    }

    NETWORK_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .worlds
        .remove(&key);
    let network = discover(host, start, entity)?;
    let leader = *network.members.first()?;
    let mut cache = NETWORK_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let world = cache.worlds.entry(key.clone()).or_default();
    for member in &network.members {
        world.by_pos.insert(*member, leader);
    }
    world.networks.insert(leader, network.clone());
    Some((key, leader, network))
}

fn claim_operation(key: &WorldKey, leader: Pos, tick: i64) -> bool {
    let mut cache = NETWORK_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(network) = cache
        .worlds
        .get_mut(key)
        .and_then(|world| world.networks.get_mut(&leader))
    else {
        return false;
    };
    if network.last_operation_tick == Some(tick) {
        return false;
    }
    network.last_operation_tick = Some(tick);
    true
}

fn remember_fluid(key: &WorldKey, leader: Pos, fluid: Option<String>) {
    let mut cache = NETWORK_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(network) = cache
        .worlds
        .get_mut(key)
        .and_then(|world| world.networks.get_mut(&leader))
    {
        network.selected_fluid = fluid;
    }
}

pub(crate) fn invalidate_world(host: &dyn Host) {
    NETWORK_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .worlds
        .remove(&world_key(host));
}

/// Route one fluid type through one connected network per tick, bounded by its slowest tier.
/// The host helper makes each endpoint transfer transactional; conduits do not buffer fluids.
pub(crate) fn tick(host: &mut dyn Host, pos: Pos, entity: &ModBlockEntity) {
    let Some(kind) = tier(&entity.id).map(|tier| tier.kind) else {
        return;
    };
    let Some((key, leader, network)) = network_for(host, pos, entity) else {
        return;
    };
    let dimension = host.dimension().to_owned();
    let Some(time) = host
        .level(&dimension)
        .and_then(|level| level.game_time(host))
    else {
        return;
    };
    if !claim_operation(&key, leader, time) {
        return;
    }

    let current = (pos, entity);
    let mut sources = Vec::new();
    let mut sinks = Vec::new();
    for conduit_pos in network.members {
        let Some(conduit) = entity_at(host, conduit_pos, current) else {
            continue;
        };
        for side in SIDES {
            let endpoint = add(conduit_pos, side.offset);
            let mode = setting(&conduit, side.name);
            if mode == "output"
                || (mode == "auto"
                    && auto_can_drain(host, &dimension, endpoint, side.opposite, kind))
            {
                sources.push((endpoint, side.opposite));
            }
            if mode == "input"
                || (mode == "auto" && auto_can_fill(host, &dimension, endpoint, side.opposite))
            {
                sinks.push((endpoint, side.opposite));
            }
        }
    }
    if sources.is_empty() || sinks.is_empty() {
        return;
    }

    let offset = time.unsigned_abs() as usize;
    let source_count = sources.len();
    let sink_count = sinks.len();
    sources.rotate_left(offset % source_count);
    sinks.rotate_left(offset % sink_count);

    // Hold one fluid type until its accessible sources run dry, matching the upstream's
    // single-fluid network even though this simplified route has no conduit-held buffer.
    let active_fluid_is_available = network.selected_fluid.as_ref().is_some_and(|active| {
        sources.iter().any(|(source, face)| {
            first_drainable_fluid(host, &dimension, *source, *face, kind).as_deref()
                == Some(active.as_str())
        })
    });
    let fluid = if active_fluid_is_available {
        network.selected_fluid.clone()
    } else {
        sources.iter().find_map(|(source, face)| {
            first_drainable_fluid(host, &dimension, *source, *face, kind)
        })
    };
    remember_fluid(&key, leader, fluid.clone());
    let Some(fluid) = fluid else {
        return;
    };

    let mut remaining = network.tier.millibuckets_per_tick;
    'sources: for (source, source_face) in sources {
        let source_has_fluid = first_drainable_fluid(host, &dimension, source, source_face, kind)
            .is_some_and(|available| available == fluid);
        if !source_has_fluid {
            continue;
        }
        for &(sink, sink_face) in &sinks {
            if remaining <= 0 {
                break 'sources;
            }
            if source == sink {
                continue;
            }
            if let Some((moved_fluid, moved)) =
                host.move_fluids(&dimension, source, source_face, sink, sink_face, remaining)
            {
                if moved_fluid == fluid && moved > 0 {
                    remaining -= moved;
                }
            }
        }
    }
}

fn first_drainable_fluid(
    host: &dyn Host,
    dimension: &str,
    pos: Pos,
    face: Direction,
    kind: ConduitKind,
) -> Option<String> {
    host.fluid_tanks(dimension, pos, face)
        .and_then(|tanks| {
            tanks
                .into_iter()
                .find(|tank| tank.can_drain && tank.amount > 0)
        })
        .and_then(|tank| tank.fluid)
        .filter(|fluid| carries(kind, fluid))
}

fn auto_can_drain(
    host: &dyn Host,
    dimension: &str,
    pos: Pos,
    face: Direction,
    kind: ConduitKind,
) -> bool {
    first_drainable_fluid(host, dimension, pos, face, kind).is_some()
}

fn auto_can_fill(host: &dyn Host, dimension: &str, pos: Pos, face: Direction) -> bool {
    host.fluid_tanks(dimension, pos, face).is_some_and(|tanks| {
        tanks
            .iter()
            .any(|tank| tank.can_fill && tank.amount < tank.capacity)
    })
}

/// Wrench face cycle; Auto uses Horizon's native fluid-tank sided capabilities.
pub(crate) fn cycle_face(host: &mut dyn Host, pos: Pos, face: Pos) -> bool {
    let Some(side) = SIDES.iter().copied().find(|side| side.offset == face) else {
        return false;
    };
    let Some(entity) = host.mod_block_entity(pos) else {
        return false;
    };
    if tier(&entity.id).is_none() {
        return false;
    }
    let current = setting(&entity, side.name);
    let next = match current.as_str() {
        "auto" => "input",
        "input" => "output",
        "output" => "disabled",
        _ => "auto",
    };
    host.change_mod_block_entity(pos, &mut |updated| {
        if tier(&updated.id).is_some() {
            set_string(&mut updated.nbt, &format!("setting_{}", side.name), next);
        }
    });

    let neighbor_pos = add(pos, side.offset);
    let neighbor = host.mod_block_entity(neighbor_pos);
    let Some(updated) = host.mod_block_entity(pos) else {
        return true;
    };
    refresh_face(host, pos, &updated, side);
    if let Some(neighbor) = neighbor.filter(|neighbor| same_network_kind(&entity.id, &neighbor.id))
    {
        refresh_face(
            host,
            neighbor_pos,
            &neighbor,
            side_for_direction(side.opposite),
        );
    }
    invalidate_world(host);
    true
}

pub(crate) fn connect_placed(host: &mut dyn Host, pos: Pos, entity: &ModBlockEntity) {
    if tier(&entity.id).is_none() {
        return;
    }
    for side in SIDES {
        refresh_face(host, pos, entity, side);
        let neighbor_pos = add(pos, side.offset);
        if let Some(neighbor) = host.mod_block_entity(neighbor_pos)
            && same_network_kind(&entity.id, &neighbor.id)
        {
            refresh_face(
                host,
                neighbor_pos,
                &neighbor,
                side_for_direction(side.opposite),
            );
        }
    }
}

pub(crate) fn neighbor_placed(host: &mut dyn Host, placed_pos: Pos) {
    for side in SIDES {
        let conduit_pos = add(placed_pos, side.offset);
        if let Some(entity) = host.mod_block_entity(conduit_pos)
            && tier(&entity.id).is_some()
        {
            refresh_face(
                host,
                conduit_pos,
                &entity,
                side_for_direction(side.opposite),
            );
            invalidate_world(host);
        }
    }
}

fn refresh_face(host: &mut dyn Host, pos: Pos, entity: &ModBlockEntity, side: Side) {
    let Some(kind) = tier(&entity.id).map(|tier| tier.kind) else {
        return;
    };
    let neighbor_pos = add(pos, side.offset);
    let neighbor = host.mod_block_entity(neighbor_pos);
    let value = if let Some(neighbor) =
        neighbor.filter(|neighbor| same_network_kind(&entity.id, &neighbor.id))
    {
        let other = setting(&neighbor, side_for_direction(side.opposite).name);
        if disabled(&setting(entity, side.name)) || disabled(&other) {
            "none"
        } else {
            "pipe"
        }
    } else {
        match setting(entity, side.name).as_str() {
            "auto" => display_auto_mode(host, pos, side, kind),
            "input" => "input",
            "output" => "output",
            _ => "none",
        }
    };
    host.set_block_property(pos, side.name, value);
}

fn display_auto_mode(host: &dyn Host, pos: Pos, side: Side, kind: ConduitKind) -> &'static str {
    let endpoint = add(pos, side.offset);
    if first_drainable_fluid(host, host.dimension(), endpoint, side.opposite, kind).is_some() {
        "output"
    } else if auto_can_fill(host, host.dimension(), endpoint, side.opposite) {
        "input"
    } else {
        "none"
    }
}

pub(crate) fn disconnect_destroyed(host: &mut dyn Host, pos: Pos, entity: &ModBlockEntity) {
    if tier(&entity.id).is_none() {
        return;
    }
    for side in SIDES {
        let neighbor_pos = add(pos, side.offset);
        let Some(neighbor) = host.mod_block_entity(neighbor_pos) else {
            continue;
        };
        if !same_network_kind(&entity.id, &neighbor.id) {
            continue;
        }
        let opposite = side_for_direction(side.opposite);
        host.set_block_property(neighbor_pos, opposite.name, "none");
    }
}
