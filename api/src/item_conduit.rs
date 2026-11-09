//! Mod-side item conduit routing through Horizon's transactional inventory move helper.

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

pub(crate) const WROUGHT: &str = "arcforge:wrought_item_conduit";
pub(crate) const TEMPERED: &str = "arcforge:tempered_item_conduit";
pub(crate) const HARDENED: &str = "arcforge:hardened_item_conduit";
pub(crate) const ARCFORGED: &str = "arcforge:arcforged_item_conduit";

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
    items_per_operation: u32,
    ticks_per_operation: i64,
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
            items_per_operation: 8,
            ticks_per_operation: 20,
        },
        TEMPERED => Tier {
            items_per_operation: 16,
            ticks_per_operation: 10,
        },
        HARDENED => Tier {
            items_per_operation: 32,
            ticks_per_operation: 5,
        },
        ARCFORGED => Tier {
            items_per_operation: 64,
            ticks_per_operation: 2,
        },
        _ => return None,
    })
}

pub(crate) fn spec(id: &str) -> Option<(u32, i64)> {
    tier(id).map(|tier| (tier.items_per_operation, tier.ticks_per_operation))
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
            tier(&neighbor.id)?;
            let connected = !disabled(&setting(entity, side.name))
                && !disabled(&setting(&neighbor, side_for_direction(side.opposite).name));
            Some((neighbor_pos, neighbor.id.clone(), connected))
        })
        .collect()
}

fn side_for_direction(direction: Direction) -> Side {
    SIDES
        .into_iter()
        .find(|side| side.direction == direction)
        .expect("all six directions are represented")
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
        slowest.items_per_operation = slowest
            .items_per_operation
            .min(member_tier.items_per_operation);
        slowest.ticks_per_operation = slowest
            .ticks_per_operation
            .max(member_tier.ticks_per_operation);
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
    })
}

fn network_for(
    host: &dyn Host,
    start: Pos,
    entity: &ModBlockEntity,
) -> Option<(WorldKey, Pos, Network)> {
    let key = world_key(host);
    let current_edges = edge_signature(host, start, entity, (start, entity));
    let mut cached = None;
    {
        let cache = NETWORK_CACHE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(world) = cache.worlds.get(&key)
            && let Some(leader) = world.by_pos.get(&start).copied()
            && let Some(network) = world.networks.get(&leader)
            && network.edges.get(&start) == Some(&current_edges)
        {
            cached = Some((leader, network.clone()));
        }
    }
    if let Some((leader, network)) = cached {
        return Some((key, leader, network));
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

pub(crate) fn invalidate_world(host: &dyn Host) {
    NETWORK_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .worlds
        .remove(&world_key(host));
}

/// Try one bounded transfer operation for a network, once per tier interval.
/// Horizon's move helper commits the source extraction and destination insertion together.
pub(crate) fn tick(host: &mut dyn Host, pos: Pos, entity: &ModBlockEntity) {
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
    let tier = network.tier;
    if time.rem_euclid(tier.ticks_per_operation) != 0 || !claim_operation(&key, leader, time) {
        return;
    }

    let current = (pos, entity);
    let mut sources = Vec::new();
    let mut sinks = Vec::new();
    for conduit_pos in network.members {
        if entity_at(host, conduit_pos, current).is_none() {
            continue;
        }
        for side in SIDES {
            let face_mode = host
                .block_property(conduit_pos, side.name)
                .unwrap_or_else(|| "none".to_owned());
            let inventory_pos = add(conduit_pos, side.offset);
            match face_mode.as_str() {
                "output" => sources.push((inventory_pos, side.opposite)),
                "input" => sinks.push((inventory_pos, side.opposite)),
                _ => {}
            }
        }
    }
    if sources.is_empty() || sinks.is_empty() {
        return;
    }

    // Rotate endpoint order deterministically so one chest cannot monopolize the network.
    let offset = ((time / tier.ticks_per_operation).unsigned_abs() as usize) % sources.len();
    sources.rotate_left(offset);
    let sink_offset = ((time / tier.ticks_per_operation).unsigned_abs() as usize) % sinks.len();
    sinks.rotate_left(sink_offset);
    for (source, source_face) in sources {
        for &(sink, sink_face) in &sinks {
            if source == sink {
                continue;
            }
            let moved = host.move_items(
                &dimension,
                source,
                source_face,
                sink,
                sink_face,
                None,
                tier.items_per_operation,
            );
            if moved > 0 {
                return;
            }
        }
    }
}

/// Wrench cycle for a conduit face. Auto leaves a non-pipe face disconnected
/// because the pinned API cannot inspect generic inventory insert/extract roles.
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
    let connection = if neighbor
        .as_ref()
        .is_some_and(|neighbor| tier(&neighbor.id).is_some())
    {
        let other_setting = neighbor
            .as_ref()
            .map(|neighbor| setting(neighbor, side_for_direction(side.opposite).name));
        if disabled(next) || other_setting.as_deref().is_some_and(disabled) {
            "none"
        } else {
            "pipe"
        }
    } else {
        match next {
            "input" | "output" => next,
            "auto" | "disabled" => "none",
            _ => unreachable!(),
        }
    };
    host.set_block_property(pos, side.name, connection);
    if neighbor
        .as_ref()
        .is_some_and(|neighbor| tier(&neighbor.id).is_some())
    {
        let opposite = side_for_direction(side.opposite);
        host.set_block_property(neighbor_pos, opposite.name, connection);
    }
    invalidate_world(host);
    true
}

pub(crate) fn connect_placed(host: &mut dyn Host, pos: Pos, entity: &ModBlockEntity) {
    if tier(&entity.id).is_none() {
        return;
    }
    for side in SIDES {
        let neighbor_pos = add(pos, side.offset);
        let Some(neighbor) = host.mod_block_entity(neighbor_pos) else {
            continue;
        };
        if tier(&neighbor.id).is_none() {
            continue;
        }
        let opposite = side_for_direction(side.opposite);
        let connected =
            !disabled(&setting(entity, side.name)) && !disabled(&setting(&neighbor, opposite.name));
        let value = if connected { "pipe" } else { "none" };
        host.set_block_property(pos, side.name, value);
        host.set_block_property(neighbor_pos, opposite.name, value);
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
        if tier(&neighbor.id).is_none() {
            continue;
        }
        let opposite = side_for_direction(side.opposite);
        host.set_block_property(neighbor_pos, opposite.name, "none");
    }
}
