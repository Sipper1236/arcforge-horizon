use horizon_client::{
    containers::ModBlockEntity,
    mod_api::{Compound, Host, Tag},
};
pub(crate) const BLOCK: &str = "arcforge:oil_pumpjack";
const TANK_CAPACITY: u32 = 16_000;
const ENERGY_CAPACITY: i32 = 100_000;
const ENERGY_PER_TICK: i32 = 60;
const MAX_ENERGY_INPUT: i32 = 1_000;
const FULL_RATE_MILLI: u64 = 30_000;
const MIN_SHARE_MILLI: u64 = 250;
const TRICKLE_MILLI: u64 = 2_000;
const RATE_SCALE: u64 = 1_000;
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
pub(crate) fn tick(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let deposit = match crate::oil_deposit::remaining(host, pos) {
        Ok(Some((deposit, remaining))) => (deposit, remaining),
        Ok(None) => {
            set_i32(&mut entity.nbt, "OilPumpStatus", 1);
            set_i32(&mut entity.nbt, "OilPumpRate", 0);
            host.set_block_property(pos, "lit", "false");
            return;
        }
        Err(_) => {
            set_i32(&mut entity.nbt, "OilPumpStatus", 2);
            set_i32(&mut entity.nbt, "OilPumpRate", 0);
            host.set_block_property(pos, "lit", "false");
            return;
        }
    };
    let (field, remaining) = deposit;
    let fluid = if field.sour {
        "arcforge:sour_crude_oil"
    } else {
        "arcforge:crude_oil"
    };
    let stored = crate::fluid::amount(&entity.nbt, 0);
    let current = crate::fluid::name(&entity.nbt, 0);
    let energy = get_i32(&entity.nbt, "energy").clamp(0, ENERGY_CAPACITY);
    if stored >= TANK_CAPACITY {
        set_i32(&mut entity.nbt, "OilPumpStatus", 3);
        set_i32(&mut entity.nbt, "OilPumpRate", 0);
        host.set_block_property(pos, "lit", "false");
        return;
    }
    if current.as_deref().is_some_and(|current| current != fluid) {
        set_i32(&mut entity.nbt, "OilPumpStatus", 4);
        set_i32(&mut entity.nbt, "OilPumpRate", 0);
        host.set_block_property(pos, "lit", "false");
        return;
    }
    if energy < ENERGY_PER_TICK {
        set_i32(&mut entity.nbt, "OilPumpStatus", 5);
        set_i32(&mut entity.nbt, "OilPumpRate", 0);
        host.set_block_property(pos, "lit", "false");
        return;
    }
    let rate_milli = if remaining == 0 {
        TRICKLE_MILLI
    } else {
        let share =
            MIN_SHARE_MILLI + (RATE_SCALE - MIN_SHARE_MILLI) * remaining / field.amount.max(1);
        (FULL_RATE_MILLI * share / RATE_SCALE).max(TRICKLE_MILLI)
    };
    let pending = get_i32(&entity.nbt, "OilPumpFraction").clamp(0, 999) as u64;
    let accumulated = pending + rate_milli;
    let requested = (accumulated / RATE_SCALE).min((TANK_CAPACITY - stored) as u64) as u32;
    let mut output = 0;
    if requested > 0 {
        let taken = if remaining > 0 {
            match crate::oil_deposit::take(host, pos, requested as u64) {
                Ok(Some((_, _, taken))) => taken,
                _ => {
                    set_i32(&mut entity.nbt, "OilPumpStatus", 2);
                    set_i32(&mut entity.nbt, "OilPumpRate", 0);
                    host.set_block_property(pos, "lit", "false");
                    return;
                }
            }
        } else {
            0
        };
        let trickle = if remaining == 0 || taken < requested as u64 {
            requested.min(2) as u64
        } else {
            0
        };
        let produced = taken.max(trickle).min((TANK_CAPACITY - stored) as u64) as u32;
        output = crate::fluid::insert(&mut entity.nbt, 0, fluid, produced, TANK_CAPACITY);
        set_i32(
            &mut entity.nbt,
            "OilPumpFraction",
            (accumulated - requested as u64 * RATE_SCALE) as i32,
        );
    } else {
        set_i32(&mut entity.nbt, "OilPumpFraction", accumulated as i32);
    }
    set_i32(&mut entity.nbt, "energy", energy - ENERGY_PER_TICK);
    set_i32(&mut entity.nbt, "OilPumpOutput", output as i32);
    set_i32(
        &mut entity.nbt,
        "OilPumpRemaining",
        remaining.saturating_sub(output as u64) as i32,
    );
    set_i32(
        &mut entity.nbt,
        "OilPumpTotal",
        field.amount.min(i32::MAX as u64) as i32,
    );
    set_i32(
        &mut entity.nbt,
        "OilPumpRate",
        rate_milli.min(i32::MAX as u64) as i32,
    );
    set_i32(&mut entity.nbt, "OilPumpKind", field.kind as i32);
    set_i32(&mut entity.nbt, "OilPumpSour", i32::from(field.sour));
    set_i32(&mut entity.nbt, "OilPumpStatus", 0);
    host.set_block_property(pos, "lit", "true");
}
pub(crate) fn accepts(_fluid: &str) -> bool {
    false
}
pub(crate) fn energy_buffer() -> horizon_client::mod_api::ModEnergyBuffer {
    horizon_client::mod_api::ModEnergyBuffer {
        capacity: ENERGY_CAPACITY,
        max_insert: MAX_ENERGY_INPUT,
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
    let state = match get_i32(&entity.nbt, "OilPumpStatus") {
        1 => "No deposit here",
        2 => "World save unavailable",
        3 => "Oil tank full",
        4 => "Oil tank contains another fluid",
        5 => "No power",
        _ => "Pumping",
    };
    let fluid = crate::fluid::name(&entity.nbt, 0).unwrap_or_else(|| "empty".to_owned());
    let rate = get_i32(&entity.nbt, "OilPumpRate").max(0) as u64;
    let left = get_i32(&entity.nbt, "OilPumpRemaining").max(0) as u64;
    let total = get_i32(&entity.nbt, "OilPumpTotal").max(0) as u64;
    let energy = get_i32(&entity.nbt, "energy").clamp(0, ENERGY_CAPACITY);
    format!(
        "Pumpjack: {state} | Field {} / {} buckets | {fluid} {}/{} mB | {:.1} mB/t | {energy}/{ENERGY_CAPACITY} FE",
        left.div_ceil(1_000),
        total.div_ceil(1_000),
        crate::fluid::amount(&entity.nbt, 0),
        TANK_CAPACITY,
        rate as f64 / RATE_SCALE as f64
    )
}
