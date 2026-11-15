//! First playable Steam Boiler Array section. The current slice implements the
//! source 3×3×3 baseline; larger shells, pressure selection, and venting remain
//! follow-up work rather than silently changing the source's progression.
use horizon_client::{containers::ModBlockEntity, mod_api::Host};

pub(crate) const CONTROLLER: &str = "arcforge:steam_boiler_array_controller";
pub(crate) const CASING: &str = "arcforge:steam_boiler_array_casing";
pub(crate) const WATER: &str = "minecraft:water";
pub(crate) const STEAM: &str = "arcforge:steam";
pub(crate) const HIGH_PRESSURE_STEAM: &str = "arcforge:high_pressure_steam";
pub(crate) const SUPERHEATED_STEAM: &str = "arcforge:superheated_steam";
pub(crate) const TANK_CAPACITY: u32 = 48_000;
pub(crate) const HEAT_CAPACITY_HU: u64 = 300_000;
pub(crate) const MAX_HEAT_PER_TICK_HU: u64 = 1_736;
pub(crate) const MAX_TEMPERATURE_C: u32 = 1_390;


pub(crate) fn tick(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    entity.items.resize(2, None);
    let facing = host
        .block_property(pos, "facing")
        .unwrap_or_else(|| "north".to_owned());
    let formed = structure_valid(host, pos, &facing);
    let state = if formed { "true" } else { "false" };
    crate::set_i32(&mut entity.nbt, "BoilerFormed", if formed { 1 } else { 0 });
    if host.block_property(pos, "formed").as_deref() != Some(state) {
        host.set_block_property(pos, "formed", state);
    }

    if let Some(bucket) = entity.items[0].as_ref()
        && bucket.item.as_ref() == "minecraft:water_bucket"
        && crate::fluid::can_insert(&entity.nbt, 0, WATER, 1_000, TANK_CAPACITY)
        && crate::can_add(entity.items[1].as_ref(), "minecraft:bucket", 1)
    {
        let amount = crate::fluid::insert(&mut entity.nbt, 0, WATER, 1_000, TANK_CAPACITY);
        if amount == 1_000 && crate::consume_one(&mut entity.items[0]) {
            crate::add_items(&mut entity.items[1], "minecraft:bucket", 1);
        }
    }

    let mut heat = crate::heat::HeatStore::new(
        HEAT_CAPACITY_HU,
        MAX_TEMPERATURE_C,
        crate::get_i32(&entity.nbt, "HeatHU").max(0) as u64,
    );
    let mut produced = 0_u32;
    if formed {
        let temperature = heat.temperature_c();
        if temperature >= 100 {
            let (gas, hu_per_mb) = if temperature >= 900 {
                (SUPERHEATED_STEAM, 16_u64)
            } else if temperature >= 500 {
                (HIGH_PRESSURE_STEAM, 12_u64)
            } else {
                (STEAM, 8_u64)
            };
            let old_gas = crate::fluid::name(&entity.nbt, 1);
            if let Some(old_gas) = old_gas.as_deref()
                && old_gas != gas
            {
                let held = crate::fluid::amount(&entity.nbt, 1);
                let moved = crate::fluid::extract(&mut entity.nbt, 1, old_gas, held);
                crate::fluid::insert(&mut entity.nbt, 1, gas, moved, TANK_CAPACITY);
            }
            let water = crate::fluid::amount(&entity.nbt, 0);
            let steam_space = TANK_CAPACITY.saturating_sub(crate::fluid::amount(&entity.nbt, 1));
            let heat_budget = heat.stored_hu().min(MAX_HEAT_PER_TICK_HU);
            let boiled = (heat_budget / hu_per_mb)
                .min(u64::from(water))
                .min(u64::from(steam_space)) as u32;
            if boiled > 0 {
                let spent = boiled as u64 * hu_per_mb;
                let removed = heat.remove_hu(spent);
                let consumed = crate::fluid::extract(&mut entity.nbt, 0, WATER, boiled);
                produced = crate::fluid::insert(&mut entity.nbt, 1, gas, consumed, TANK_CAPACITY);
                debug_assert_eq!(removed, spent);
                debug_assert_eq!(consumed, boiled);
                debug_assert_eq!(produced, boiled);
            }
        }
    }
    crate::set_i32(
        &mut entity.nbt,
        "HeatHU",
        heat.stored_hu().min(i32::MAX as u64) as i32,
    );
    crate::set_i32(&mut entity.nbt, "TemperatureC", heat.temperature_c() as i32);
    crate::set_i32(&mut entity.nbt, "BoiledLastTick", produced as i32);
    let lit = formed && produced > 0;
    if host.block_property(pos, "lit").as_deref() != Some(if lit { "true" } else { "false" }) {
        host.set_block_property(pos, "lit", if lit { "true" } else { "false" });
    }
}

pub(crate) fn status(entity: &ModBlockEntity) -> String {
    let formed = crate::get_i32(&entity.nbt, "BoilerFormed") != 0;
    let water = crate::fluid::amount(&entity.nbt, 0);
    let steam = crate::fluid::amount(&entity.nbt, 1);
    let heat = crate::get_i32(&entity.nbt, "TemperatureC").max(20);
    let gas = crate::fluid::name(&entity.nbt, 1).unwrap_or_else(|| "none".to_owned());
    format!("Boiler {} | {heat}°C | Water {water}/{TANK_CAPACITY} mB | {gas} {steam}/{TANK_CAPACITY} mB", if formed { "formed" } else { "unformed" })
}

pub(crate) fn accept_heat(
    host: &mut dyn Host,
    pos: [i32; 3],
    source: &mut crate::heat::HeatStore,
    rate: u64,
) -> u64 {
    let mut moved = 0;
    host.change_mod_block_entity(pos, &mut |entity| {
        if entity.id != CONTROLLER {
            return;
        }
        let mut sink = crate::heat::HeatStore::new(
            HEAT_CAPACITY_HU,
            MAX_TEMPERATURE_C,
            crate::get_i32(&entity.nbt, "HeatHU").max(0) as u64,
        );
        moved = source.transfer_to(&mut sink, rate);
        if moved > 0 {
            crate::set_i32(&mut entity.nbt, "HeatHU", sink.stored_hu() as i32);
            crate::set_i32(&mut entity.nbt, "TemperatureC", sink.temperature_c() as i32);
        }
    });
    if moved > 0 {
        host.block_entity_sync(pos);
    }
    moved
}

fn structure_valid(host: &dyn Host, controller: [i32; 3], facing: &str) -> bool {
    let (out, right) = match facing {
        "north" => ([0, 0, -1], [1, 0, 0]),
        "east" => ([1, 0, 0], [0, 0, 1]),
        "south" => ([0, 0, 1], [-1, 0, 0]),
        "west" => ([-1, 0, 0], [0, 0, -1]),
        _ => return false,
    };
    let inward = [-out[0], -out[1], -out[2]];
    for x in -1..=1 {
        for y in -1..=1 {
            for depth in 0..=2 {
                let pos = [
                    controller[0] + right[0] * x + inward[0] * depth,
                    controller[1] + y,
                    controller[2] + right[2] * x + inward[2] * depth,
                ];
                let boundaries = usize::from(x.abs() == 1)
                    + usize::from(y.abs() == 1)
                    + usize::from(depth == 0 || depth == 2);
                let block = host.block(pos);
                if pos == controller {
                    if block != CONTROLLER {
                        return false;
                    }
                } else if boundaries == 3 {
                    if block != CASING {
                        return false;
                    }
                } else if boundaries > 0 {
                    if block != CASING && block != "arcforge:pressure_glass" {
                        return false;
                    }
                } else if block != "minecraft:air"
                    && block != "minecraft:cave_air"
                    && block != "minecraft:void_air"
                {
                    return false;
                }
            }
        }
    }
    true
}
