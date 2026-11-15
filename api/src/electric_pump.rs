//! Electric Pump alpha for renewable source water. Finite block extraction requires
//! a world-block mutation hook, which the pinned Horizon Host does not expose.
use horizon_client::{
    containers::ModBlockEntity,
    mod_api::{Compound, Host, Tag},
};
pub(crate) const BLOCK: &str = "arcforge:electric_pump";
const WATER: &str = "minecraft:water";
const TANK_CAPACITY: u32 = 16_000;
const ENERGY_CAPACITY: i32 = 20_000;
const ENERGY_PER_TICK: i32 = 10;
const CYCLE_TICKS: i32 = 20;
const BUCKET: u32 = 1_000;

fn get_i32(nbt: &Compound, name: &str) -> i32 {
    let Tag::String(key) = Tag::String(name.into()) else {
        unreachable!()
    };
    match nbt.get(&key) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}
fn set_i32(nbt: &mut Compound, name: &str, value: i32) {
    let Tag::String(key) = Tag::String(name.into()) else {
        unreachable!()
    };
    nbt.insert(key, Tag::Int(value));
}
fn source_water(host: &dyn Host, pos: [i32; 3]) -> bool {
    host.block(pos) == WATER
        && host
            .block_property(pos, "level")
            .is_none_or(|level| level == "0")
}
fn infinite_water(host: &dyn Host, pos: [i32; 3]) -> bool {
    if !source_water(host, pos) {
        return false;
    }
    let adjacent = [
        [pos[0] + 1, pos[1], pos[2]],
        [pos[0] - 1, pos[1], pos[2]],
        [pos[0], pos[1], pos[2] + 1],
        [pos[0], pos[1], pos[2] - 1],
    ];
    adjacent
        .into_iter()
        .filter(|next| source_water(host, *next))
        .count()
        >= 2
}
pub(crate) fn tick(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let below = [pos[0], pos[1] - 1, pos[2]];
    let water_source = source_water(host, below);
    let renewable = water_source && infinite_water(host, below);
    let tank = crate::fluid::amount(&entity.nbt, 0);
    let energy = get_i32(&entity.nbt, "energy").max(0);
    let progress = get_i32(&entity.nbt, "PumpProgress").clamp(0, CYCLE_TICKS);
    let can_pump = renewable && tank.saturating_add(BUCKET) <= TANK_CAPACITY;
    let next_progress = if !water_source { 0 } else { progress };
    let mut pumped = 0;
    let status = if !water_source {
        "No source"
    } else if !renewable {
        "Needs infinite water"
    } else if tank.saturating_add(BUCKET) > TANK_CAPACITY {
        "Tank full"
    } else if energy < ENERGY_PER_TICK {
        "No power"
    } else {
        set_i32(&mut entity.nbt, "energy", energy - ENERGY_PER_TICK);
        let progress = next_progress + 1;
        if progress >= CYCLE_TICKS {
            pumped = crate::fluid::insert(&mut entity.nbt, 0, WATER, BUCKET, TANK_CAPACITY);
            set_i32(&mut entity.nbt, "PumpProgress", 0);
        } else {
            set_i32(&mut entity.nbt, "PumpProgress", progress);
        }
        "Pumping"
    };
    if !can_pump || energy < ENERGY_PER_TICK {
        set_i32(&mut entity.nbt, "PumpPumped", 0);
        if !water_source {
            set_i32(&mut entity.nbt, "PumpProgress", 0);
        }
    } else {
        set_i32(&mut entity.nbt, "PumpPumped", pumped as i32);
    }
    set_i32(
        &mut entity.nbt,
        "PumpStatus",
        match status {
            "No source" => 1,
            "Needs infinite water" => 2,
            "Tank full" => 3,
            "No power" => 4,
            _ => 0,
        },
    );
    host.set_block_property(
        pos,
        "lit",
        if status == "Pumping" { "true" } else { "false" },
    );
}
pub(crate) fn accepts(_fluid: &str) -> bool {
    false
}
pub(crate) fn energy_buffer() -> horizon_client::mod_api::ModEnergyBuffer {
    horizon_client::mod_api::ModEnergyBuffer {
        capacity: ENERGY_CAPACITY,
        max_insert: 200,
        max_extract: 0,
        insert_faces: [true; 6],
        extract_faces: [false; 6],
    }
}
pub(crate) fn fluid_tanks() -> horizon_client::mod_api::ModFluidTanks {
    horizon_client::mod_api::ModFluidTanks {
        tanks: vec![TANK_CAPACITY as i32],
        faces: [vec![0], vec![0], vec![0], vec![0], vec![0], vec![0]],
        inputs: vec![],
        outputs: vec![0],
    }
}
pub(crate) fn status(entity: &ModBlockEntity) -> String {
    let source = get_i32(&entity.nbt, "PumpStatus");
    let progress = get_i32(&entity.nbt, "PumpProgress").max(0);
    let tank = crate::fluid::amount(&entity.nbt, 0);
    let energy = get_i32(&entity.nbt, "energy").max(0);
    let state = match source {
        1 => "No source below",
        2 => "Needs infinite source water",
        3 => "Tank full",
        4 => "No power",
        _ => "Pumping",
    };
    format!(
        "Electric Pump: {state} | {progress}/{CYCLE_TICKS} ticks | Water {tank}/{TANK_CAPACITY} mB | Energy {energy}/{ENERGY_CAPACITY} FE"
    )
}
