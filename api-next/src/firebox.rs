//! Current-API Firebox: solid fuel, stored heat, and bounded adjacent output.

use mod_api::{
    BlockEntityTickEvent, BlockEntityTickReply, Compound, Host, Identifier, ItemStack,
    RandomSource, ServerHost, Tag,
};

const CAPACITY: u64 = 40_000;
const MAX_TEMPERATURE_C: u32 = 1_100;
const HEAT_PER_TICK: u64 = 80;
const OUTPUT_PER_TARGET: u64 = 100;
const OUTPUT_PER_TICK: u64 = 600;
const MAX_UPGRADES: u32 = 8;
const SPEED_SLOT: usize = 2;
const HEAT_SLOT: usize = 3;
const SPEED_UPGRADE: &str = "arcforge:speed_upgrade";
const HEAT_UPGRADE: &str = "arcforge:heat_upgrade";
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
    let mut changed = event.entity.items.len() < 4;
    if changed {
        event.entity.items.resize(4, None);
    }
    let mut heat = super::heat_cell::HeatStore::new(
        CAPACITY,
        MAX_TEMPERATURE_C,
        get_i32(&event.entity.nbt, "HeatHU").max(0) as u64,
    );
    let mut burn = get_i32(&event.entity.nbt, "BurnTime").max(0) as u32;
    let speed_upgrades = upgrade_count(
        host,
        event.entity.items.get(SPEED_SLOT).and_then(Option::as_ref),
        SPEED_UPGRADE,
    );
    let heat_upgrades = upgrade_count(
        host,
        event.entity.items.get(HEAT_SLOT).and_then(Option::as_ref),
        HEAT_UPGRADE,
    );
    let speed = speed_upgrade_multiplier(speed_upgrades);

    if burn == 0 && heat.stored_hu() < CAPACITY {
        let fuel = event.entity.items[0].as_ref().and_then(|stack| {
            let item = stack.item().id(host);
            fuel_ticks(&item)
                .map(|ticks| scaled_burn_duration(ticks, speed))
                .map(|duration| (item, duration))
        });
        if let Some((fuel_item, ticks)) = fuel
            && consume_one(&mut event.entity.items[0])
        {
            burn = ticks;
            changed |= set_i32(&mut event.entity.nbt, "BurnDuration", burn as i32);
            changed |= set_string(&mut event.entity.nbt, "BurningItem", &fuel_item);
        }
    }

    let output_per_tick =
        (HEAT_PER_TICK as f64 * speed * heat_upgrade_multiplier(heat_upgrades)).round() as u64;
    let produced = if burn > 0 {
        heat.add_hu(output_per_tick)
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
        (heat.stored_hu() / 10).min(i16::MAX as u64) as i32,
        (CAPACITY / 10).min(i16::MAX as u64) as i32,
        heat.temperature_c().min(i16::MAX as u32) as i32,
        burn.min(i16::MAX as u32) as i32,
        burn_duration.clamp(0, i16::MAX as i32),
        produced.min(i16::MAX as u64) as i32,
        speed_upgrades as i32,
        heat_upgrades as i32,
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

fn upgrade_count(host: &dyn Host, stack: Option<&ItemStack>, item_id: &str) -> u32 {
    stack
        .filter(|stack| stack.item().id(host) == item_id)
        .map_or(0, |stack| stack.count().min(MAX_UPGRADES))
}

fn speed_upgrade_multiplier(upgrades: u32) -> f64 {
    2.0_f64.powf(f64::from(upgrades.min(MAX_UPGRADES)) / 2.0)
}

fn heat_upgrade_multiplier(upgrades: u32) -> f64 {
    1.0 + 0.125 * f64::from(upgrades.min(MAX_UPGRADES))
}

fn scaled_burn_duration(base_ticks: u32, speed: f64) -> u32 {
    (f64::from(base_ticks) / speed)
        .round()
        .clamp(1.0, f64::from(u32::MAX)) as u32
}

pub(crate) fn screen() -> mod_api::Screen {
    mod_api::Screen {
        width: 176.0,
        height: 254.0,
        slots: vec![
            Some([16.0, 108.0]),
            Some([40.0, 108.0]),
            Some([104.0, 108.0]),
            Some([128.0, 108.0]),
        ],
        inventory_y: 168.0,
        inventory_x: 8.0,
        own_labels: true,
        own_background: false,
        ..mod_api::Screen::default()
    }
}

pub(crate) fn draw_screen(event: &mut mod_api::ScreenDrawEvent<'_>) {
    let [ox, oy] = event.origin;
    let data = event.data;
    let stored_hu = data.first().copied().unwrap_or_default().max(0) * 10;
    let capacity_hu = data.get(1).copied().unwrap_or_default().max(0) * 10;
    let temperature = data.get(2).copied().unwrap_or_default().max(0);
    let burn = data.get(3).copied().unwrap_or_default().max(0);
    let duration = data.get(4).copied().unwrap_or_default().max(0);
    let output = data.get(5).copied().unwrap_or_default().max(0);
    let speed_upgrades = data
        .get(6)
        .copied()
        .unwrap_or_default()
        .clamp(0, MAX_UPGRADES as i32);
    let heat_upgrades = data
        .get(7)
        .copied()
        .unwrap_or_default()
        .clamp(0, MAX_UPGRADES as i32);
    let draw = &mut *event.draw;
    draw.text("Firebox", [ox + 8.0, oy + 6.0], 0xFFFFFFFF, true);
    draw.text(
        &format!("Temperature: {temperature} C"),
        [ox + 8.0, oy + 19.0],
        0xFFBBBBBB,
        true,
    );

    draw.fill([ox + 8.0, oy + 33.0, 160.0, 8.0], 0xFF343434);
    let heat_width = if capacity_hu > 0 {
        (160.0 * stored_hu.min(capacity_hu) as f32 / capacity_hu as f32).round()
    } else {
        0.0
    };
    if heat_width > 0.0 {
        draw.fill([ox + 8.0, oy + 33.0, heat_width, 8.0], 0xFFE37B32);
    }
    draw.text(
        &format!("Heat: {stored_hu} / {capacity_hu} HU"),
        [ox + 8.0, oy + 45.0],
        0xFFFFFFFF,
        true,
    );

    draw.fill([ox + 8.0, oy + 61.0, 160.0, 6.0], 0xFF343434);
    let burn_width = if duration > 0 {
        (160.0 * burn.min(duration) as f32 / duration as f32).round()
    } else {
        0.0
    };
    if burn_width > 0.0 {
        draw.fill([ox + 8.0, oy + 61.0, burn_width, 6.0], 0xFFB8492D);
    }
    draw.text(
        &format!("Burn: {burn} / {duration} ticks"),
        [ox + 8.0, oy + 72.0],
        0xFFFFFFFF,
        true,
    );
    draw.text(
        &format!("Heat output: {output} HU/t"),
        [ox + 8.0, oy + 84.0],
        0xFFFFFFFF,
        true,
    );
    draw.text("Fuel", [ox + 16.0, oy + 94.0], 0xFFBBBBBB, true);
    draw.text("Ash", [ox + 40.0, oy + 94.0], 0xFFBBBBBB, true);
    draw.text(
        &format!("S:{speed_upgrades}/{MAX_UPGRADES}"),
        [ox + 92.0, oy + 94.0],
        0xFFBBBBBB,
        true,
    );
    draw.text(
        &format!("H:{heat_upgrades}/{MAX_UPGRADES}"),
        [ox + 132.0, oy + 94.0],
        0xFFBBBBBB,
        true,
    );
}

pub(crate) fn screen_tooltip(at: [f32; 2], lines: &mut Vec<String>) {
    if (33.0..41.0).contains(&at[1]) && (8.0..168.0).contains(&at[0]) {
        lines.push("Stored heat HU / capacity".to_owned());
    } else if (61.0..67.0).contains(&at[1]) && (8.0..168.0).contains(&at[0]) {
        lines.push("Current fuel burn progress".to_owned());
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
