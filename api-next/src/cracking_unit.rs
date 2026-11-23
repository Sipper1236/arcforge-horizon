//! Current-Horizon structure detection for the Cracking Unit.
use mod_api::{
    BlockEntityTickEvent, BlockEntityTickReply, BlockKey, Compound, Host, LevelRef, ServerHost, Tag,
};

const WIDTH: i32 = 3;
const HEIGHT: i32 = 5;
const LENGTH: i32 = 3;

#[derive(Clone, Copy)]
struct Facing {
    name: &'static str,
    vector: [i32; 3],
}

const FACINGS: [Facing; 4] = [
    Facing {
        name: "north",
        vector: [0, 0, -1],
    },
    Facing {
        name: "east",
        vector: [1, 0, 0],
    },
    Facing {
        name: "south",
        vector: [0, 0, 1],
    },
    Facing {
        name: "west",
        vector: [-1, 0, 0],
    },
];

fn facing(name: &str) -> Option<Facing> {
    FACINGS.iter().copied().find(|value| value.name == name)
}

fn positions(controller: [i32; 3], facing: Facing) -> Vec<[i32; 3]> {
    let back = [-facing.vector[0], 0, -facing.vector[2]];
    let right = [-back[2], 0, back[0]];
    let mut positions = Vec::with_capacity((WIDTH * HEIGHT * LENGTH) as usize);
    for depth in 0..LENGTH {
        for y in 0..HEIGHT {
            for side in -1..=1 {
                positions.push([
                    controller[0] + back[0] * depth + right[0] * side,
                    controller[1] + y,
                    controller[2] + back[2] * depth + right[2] * side,
                ]);
            }
        }
    }
    positions
}

fn get_i32(nbt: &Compound, name: &str) -> i32 {
    let Tag::String(key) = Tag::String(name.into()) else {
        unreachable!()
    };
    match nbt.get(&key) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn set_i32(nbt: &mut Compound, name: &str, value: i32) -> bool {
    if get_i32(nbt, name) == value {
        return false;
    }
    let Tag::String(key) = Tag::String(name.into()) else {
        unreachable!()
    };
    nbt.insert(key, Tag::Int(value));
    true
}

fn get_string(nbt: &Compound, name: &str) -> Option<String> {
    let Tag::String(key) = Tag::String(name.into()) else {
        unreachable!()
    };
    match nbt.get(&key) {
        Some(Tag::String(value)) => Some(value.to_string_lossy()),
        _ => None,
    }
}

fn set_string(nbt: &mut Compound, name: &str, value: &str) -> bool {
    if get_string(nbt, name).as_deref() == Some(value) {
        return false;
    }
    let Tag::String(key) = Tag::String(name.into()) else {
        unreachable!()
    };
    nbt.insert(key, Tag::String(value.into()));
    true
}

fn set_property(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    key: &str,
    value: &str,
) -> bool {
    if host.level_block_property(level, pos, key).as_deref() == Some(value) {
        return false;
    }
    host.set_block_property(pos, key, value)
}

fn has_structure(
    host: &dyn Host,
    level: LevelRef,
    controller: [i32; 3],
    facing: Facing,
    controller_block: BlockKey,
    casing_block: BlockKey,
    reject_formed_casing: bool,
) -> bool {
    positions(controller, facing).into_iter().all(|pos| {
        if pos == controller {
            level.block(host, pos) == Some(controller_block)
        } else if level.block(host, pos) != Some(casing_block) {
            false
        } else {
            !reject_formed_casing
                || host.level_block_property(level, pos, "formed").as_deref() != Some("true")
        }
    })
}

fn mark_formed(
    host: &mut dyn ServerHost,
    level: LevelRef,
    controller: [i32; 3],
    facing: Facing,
    event: &mut BlockEntityTickEvent<'_>,
) -> bool {
    let mut changed = false;
    for pos in positions(controller, facing) {
        if pos != controller {
            set_property(host, level, pos, "formed", "true");
        }
    }
    set_property(host, level, controller, "formed", "true");
    changed |= set_i32(&mut event.entity.nbt, "CrackingFormed", 1);
    changed |= set_i32(&mut event.entity.nbt, "CrackingProgress", 0);
    changed |= set_i32(&mut event.entity.nbt, "CrackingScan", 0);
    changed |= set_string(&mut event.entity.nbt, "CrackingFacing", facing.name);
    changed
}

fn clear_formed(
    host: &mut dyn ServerHost,
    level: LevelRef,
    controller: [i32; 3],
    facing: Facing,
    casing_block: BlockKey,
    event: &mut BlockEntityTickEvent<'_>,
) -> bool {
    for pos in positions(controller, facing) {
        if pos != controller && level.block(host, pos) == Some(casing_block) {
            set_property(host, level, pos, "formed", "false");
        }
    }
    set_property(host, level, controller, "formed", "false");
    set_property(host, level, controller, "lit", "false");
    let mut changed = false;
    changed |= set_i32(&mut event.entity.nbt, "CrackingFormed", 0);
    changed |= set_i32(&mut event.entity.nbt, "CrackingProgress", 0);
    changed |= set_i32(&mut event.entity.nbt, "CrackingScan", 0);
    changed
}

pub(crate) fn tick(
    host: &mut dyn ServerHost,
    event: &mut BlockEntityTickEvent<'_>,
    controller_block: BlockKey,
    casing_block: BlockKey,
) {
    event.reply = BlockEntityTickReply::Unchanged;
    let Some(level) = host.level(host.dimension()) else {
        return;
    };
    let controller = event.pos;
    let facing_name = host
        .level_block_property(level, controller, "facing")
        .unwrap_or_else(|| "north".to_owned());
    let current_facing = facing(&facing_name).unwrap_or(FACINGS[0]);
    let mut changed = false;

    if get_i32(&event.entity.nbt, "CrackingFormed") != 0 {
        let saved_facing = get_string(&event.entity.nbt, "CrackingFacing")
            .and_then(|name| facing(&name))
            .unwrap_or(current_facing);
        if saved_facing.name != current_facing.name
            || !has_structure(
                host,
                level,
                controller,
                saved_facing,
                controller_block,
                casing_block,
                false,
            )
        {
            changed |= clear_formed(host, level, controller, saved_facing, casing_block, event);
        } else {
            for pos in positions(controller, saved_facing) {
                if pos != controller {
                    set_property(host, level, pos, "formed", "true");
                }
            }
            set_property(host, level, controller, "formed", "true");
            set_property(host, level, controller, "lit", "false");
        }
    } else {
        set_property(host, level, controller, "formed", "false");
        set_property(host, level, controller, "lit", "false");
        let scan = get_i32(&event.entity.nbt, "CrackingScan").max(0);
        if scan == 0 {
            if has_structure(
                host,
                level,
                controller,
                current_facing,
                controller_block,
                casing_block,
                true,
            ) {
                changed |= mark_formed(host, level, controller, current_facing, event);
            } else {
                changed |= set_i32(&mut event.entity.nbt, "CrackingScan", 19);
            }
        } else {
            changed |= set_i32(&mut event.entity.nbt, "CrackingScan", scan - 1);
        }
    }

    if changed {
        event.reply = BlockEntityTickReply::Changed;
    }
}
