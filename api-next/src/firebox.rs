//! Current-API Firebox: solid fuel, stored heat, and bounded adjacent output.

use mod_api::{
    BlockEntityTickEvent, BlockEntityTickReply, Compound, Host, Identifier, ItemStack, ServerHost,
    Tag,
};

const CAPACITY: u64 = 40_000;
const MAX_TEMPERATURE_C: u32 = 1_100;
const HEAT_PER_TICK: u64 = 80;
const OUTPUT_PER_TARGET: u64 = 100;
const OUTPUT_PER_TICK: u64 = 600;
const WOOD_ASH: &str = "arcforge:wood_ash";

const DIRECTIONS: [[i32; 3]; 6] = [
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, -1],
    [0, 0, 1],
    [-1, 0, 0],
    [1, 0, 0],
];

pub(crate) fn accepts_fuel(item: &str) -> bool {
    matches!(
        item,
        "minecraft:coal"
            | "minecraft:charcoal"
            | "arcforge:bio_coal"
            | "minecraft:coal_block"
            | "arcforge:bio_coal_block"
    )
}

fn fuel_ticks(item: &str) -> Option<u32> {
    Some(match item {
        "minecraft:coal" | "minecraft:charcoal" | "arcforge:bio_coal" => 800,
        "minecraft:coal_block" | "arcforge:bio_coal_block" => 8_000,
        _ => return None,
    })
}

pub(crate) fn tick(host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
    *event.reply = BlockEntityTickReply::Unchanged;
    event.entity.items.resize(2, None);

    let mut changed = false;
    let mut heat = super::heat_cell::HeatStore::new(
        CAPACITY,
        MAX_TEMPERATURE_C,
        get_i32(&event.entity.nbt, "HeatHU").max(0) as u64,
    );
    let mut burn = get_i32(&event.entity.nbt, "BurnTime").max(0) as u32;

    if burn == 0 && heat.stored_hu() < CAPACITY {
        let fuel = event.entity.items[0].as_ref().and_then(|stack| {
            let item = stack.item().id(host);
            fuel_ticks(&item).map(|ticks| (item, ticks))
        });
        if let Some((fuel_item, ticks)) = fuel
            && consume_one(&mut event.entity.items[0])
        {
            burn = ticks;
            changed |= set_i32(&mut event.entity.nbt, "BurnDuration", burn as i32);
            changed |= set_string(&mut event.entity.nbt, "BurningItem", &fuel_item);
        }
    }

    let produced = if burn > 0 {
        heat.add_hu(HEAT_PER_TICK)
    } else {
        0
    };
    heat.set_source_temperature(if produced > 0 { MAX_TEMPERATURE_C } else { 20 });

    if produced > 0 {
        burn = burn.saturating_sub(1);
        if burn == 0 {
            if let Some(item) = get_string(&event.entity.nbt, "BurningItem") {
                changed |= try_leave_wood_ash(host, &item, &mut event.entity.items[1]);
            }
            changed |= remove_tag(&mut event.entity.nbt, "BurningItem");
        }
    }

    changed |= set_i32(
        &mut event.entity.nbt,
        "BurnTime",
        burn.min(i32::MAX as u32) as i32,
    );
    let lit = burn > 0;
    changed |= host.set_block_property(event.pos, "lit", if lit { "true" } else { "false" });

    let level = host.level(host.dimension());
    if let Some(level) = level {
        transfer_heat(host, level, event.pos, &mut heat);
    }

    changed |= set_i32(
        &mut event.entity.nbt,
        "HeatHU",
        heat.stored_hu().min(i32::MAX as u64) as i32,
    );
    changed |= set_i32(
        &mut event.entity.nbt,
        "TemperatureC",
        heat.temperature_c().min(i32::MAX as u32) as i32,
    );

    let burn_duration = get_i32(&event.entity.nbt, "BurnDuration");
    let data = [
        heat.stored_hu().min(i32::MAX as u64) as i32,
        CAPACITY as i32,
        heat.temperature_c().min(i32::MAX as u32) as i32,
        burn.min(i32::MAX as u32) as i32,
        burn_duration,
    ];
    if event.entity.data.as_slice() != data.as_slice() {
        event.entity.data = data.into();
        changed = true;
    }

    if changed {
        *event.reply = BlockEntityTickReply::Changed;
    }
}

fn transfer_heat(
    host: &mut dyn ServerHost,
    level: mod_api::LevelRef,
    pos: [i32; 3],
    heat: &mut super::heat_cell::HeatStore,
) {
    let mut output_left = OUTPUT_PER_TICK;
    for direction in DIRECTIONS {
        if heat.temperature_c() <= 20 || output_left == 0 {
            break;
        }
        let neighbor = [
            pos[0] + direction[0],
            pos[1] + direction[1],
            pos[2] + direction[2],
        ];
        let rate = OUTPUT_PER_TARGET.min(output_left);
        let moved = super::heat_cell::accept_heat(host, level, neighbor, heat, rate)
            + super::cracking_unit::accept_heat(host, level, neighbor, heat, rate);
        output_left = output_left.saturating_sub(moved);
    }
}

fn try_leave_wood_ash(host: &mut dyn ServerHost, item: &str, slot: &mut Option<ItemStack>) -> bool {
    if item != "minecraft:charcoal" {
        return false;
    }
    let dimension = host.dimension();
    let Some(level) = host.level(dimension) else {
        return false;
    };
    if level
        .with_random(host, |random| random.next_float() < 0.5)
        .unwrap_or(false)
    {
        insert_wood_ash(host, slot)
    } else {
        false
    }
}

fn insert_wood_ash(host: &dyn Host, slot: &mut Option<ItemStack>) -> bool {
    match slot {
        Some(stack) if stack.item().id(host) == WOOD_ASH && stack.count() < 64 => {
            stack.set_count(stack.count() + 1).is_ok()
        }
        Some(_) => false,
        None => {
            let Ok(id) = Identifier::parse(WOOD_ASH) else {
                return false;
            };
            let Some(item) = host.item_key(&id) else {
                return false;
            };
            *slot = Some(ItemStack::of(item));
            true
        }
    }
}

fn consume_one(slot: &mut Option<ItemStack>) -> bool {
    let Some(stack) = slot.as_mut() else {
        return false;
    };
    if stack.count() <= 1 {
        *slot = None;
    } else if stack.set_count(stack.count() - 1).is_err() {
        return false;
    }
    true
}

fn get_i32(nbt: &Compound, name: &str) -> i32 {
    match nbt.get(&name.into()) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn set_i32(nbt: &mut Compound, name: &str, value: i32) -> bool {
    if get_i32(nbt, name) == value {
        return false;
    }
    nbt.insert(name.into(), Tag::Int(value));
    true
}

fn get_string(nbt: &Compound, name: &str) -> Option<String> {
    match nbt.get(&name.into()) {
        Some(Tag::String(value)) => Some(value.to_string_lossy()),
        _ => None,
    }
}

fn set_string(nbt: &mut Compound, name: &str, value: &str) -> bool {
    if get_string(nbt, name).as_deref() == Some(value) {
        return false;
    }
    nbt.insert(name.into(), Tag::String(value.into()));
    true
}

fn remove_tag(nbt: &mut Compound, name: &str) -> bool {
    nbt.remove(&name.into()).is_some()
}
