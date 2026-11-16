use horizon_client::{
    containers::ModBlockEntity,
    mod_api::{Compound, Host, Tag},
};
pub(crate) const CONTROLLER: &str = "arcforge:pumpjack_controller";
pub(crate) const CASING: &str = "arcforge:pumpjack_casing";
pub(crate) const WELL_HEAD: &str = "arcforge:well_head";
const WIDTH: i32 = 3;
const LENGTH: i32 = 5;
const HEIGHT: i32 = 4;
const POST_U: i32 = 2;
const TANK_CAPACITY: u32 = 16_000;
const ENERGY_CAPACITY: i32 = 100_000;
const ENERGY_PER_TICK: i32 = 60;
const MAX_ENERGY_INPUT: i32 = 1_000;
const FULL_RATE_MILLI: u64 = 30_000;
const MIN_SHARE_MILLI: u64 = 250;
const TRICKLE_MILLI: u64 = 2_000;
const RATE_SCALE: u64 = 1_000;
const SCAN_INTERVAL: i32 = 20;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Facing {
    name: &'static str,
    vector: [i32; 3],
    clockwise: [i32; 3],
}
const FACINGS: [Facing; 4] = [
    Facing {
        name: "north",
        vector: [0, 0, -1],
        clockwise: [1, 0, 0],
    },
    Facing {
        name: "east",
        vector: [1, 0, 0],
        clockwise: [0, 0, 1],
    },
    Facing {
        name: "south",
        vector: [0, 0, 1],
        clockwise: [-1, 0, 0],
    },
    Facing {
        name: "west",
        vector: [-1, 0, 0],
        clockwise: [0, 0, -1],
    },
];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Unit {
    facing: Facing,
    well: [i32; 3],
}
impl Unit {
    fn controller(self) -> [i32; 3] {
        at(self, 0, 0, LENGTH - 1)
    }
    fn parts(self) -> Vec<[i32; 3]> {
        let mut parts = Vec::with_capacity(17);
        for u in 0..LENGTH {
            for x in -(WIDTH / 2)..=(WIDTH / 2) {
                if u == 0 && x == 0 {
                    continue;
                }
                parts.push(at(self, x, 0, u));
            }
        }
        for y in 1..HEIGHT {
            parts.push(at(self, 0, y, POST_U));
        }
        parts
    }
    fn contains(self, pos: [i32; 3]) -> bool {
        pos == self.well || self.parts().contains(&pos)
    }
}
fn at(unit: Unit, x: i32, y: i32, u: i32) -> [i32; 3] {
    [
        unit.well[0] + unit.facing.vector[0] * u + unit.facing.clockwise[0] * x,
        unit.well[1] + y,
        unit.well[2] + unit.facing.vector[2] * u + unit.facing.clockwise[2] * x,
    ]
}
fn facing(name: &str) -> Option<Facing> {
    FACINGS.into_iter().find(|facing| facing.name == name)
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
fn set_i32(nbt: &mut Compound, name: &str, value: i32) {
    let Tag::String(key) = Tag::String(name.into()) else {
        unreachable!()
    };
    nbt.insert(key, Tag::Int(value));
}
fn set_block_bool(host: &mut dyn Host, pos: [i32; 3], property: &str, value: bool) {
    let text = if value { "true" } else { "false" };
    if host.block_property(pos, property).as_deref() != Some(text) {
        host.set_block_property(pos, property, text);
    }
}
fn saved_unit(nbt: &Compound) -> Option<Unit> {
    if get_i32(nbt, "PumpjackFormed") == 0 {
        return None;
    }
    Some(Unit {
        facing: facing(&get_string(nbt, "PumpjackFacing")?)?,
        well: [
            get_i32(nbt, "PumpjackWellX"),
            get_i32(nbt, "PumpjackWellY"),
            get_i32(nbt, "PumpjackWellZ"),
        ],
    })
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
fn set_string(nbt: &mut Compound, name: &str, value: &str) {
    let Tag::String(key) = Tag::String(name.into()) else {
        unreachable!()
    };
    nbt.insert(key, Tag::String(value.into()));
}
fn write_unit(nbt: &mut Compound, unit: Option<Unit>) {
    set_i32(nbt, "PumpjackFormed", i32::from(unit.is_some()));
    if let Some(unit) = unit {
        set_string(nbt, "PumpjackFacing", unit.facing.name);
        set_i32(nbt, "PumpjackWellX", unit.well[0]);
        set_i32(nbt, "PumpjackWellY", unit.well[1]);
        set_i32(nbt, "PumpjackWellZ", unit.well[2]);
    }
}
fn unform_parts(host: &mut dyn Host, unit: Unit, skip: Option<[i32; 3]>) {
    for pos in unit.parts() {
        if Some(pos) == skip {
            continue;
        }
        if matches!(host.block(pos).as_str(), CONTROLLER | CASING) {
            set_block_bool(host, pos, "formed", false);
            if host.block(pos) == CONTROLLER {
                set_block_bool(host, pos, "lit", false);
            }
        }
    }
}
fn set_formed(host: &mut dyn Host, unit: Unit, formed: bool) {
    for pos in unit.parts() {
        set_block_bool(host, pos, "formed", formed);
        if host.block(pos) == CONTROLLER {
            if host.block_property(pos, "facing").as_deref() != Some(unit.facing.name) {
                host.set_block_property(pos, "facing", unit.facing.name);
            }
            set_block_bool(host, pos, "lit", false);
        }
    }
}
fn valid_unit(host: &dyn Host, unit: Unit, previous: Option<Unit>) -> bool {
    if host.block(unit.well) != WELL_HEAD
        || host.block_property(unit.well, "gas").as_deref() == Some("true")
    {
        return false;
    }
    for pos in unit.parts() {
        let expected = if pos == unit.controller() {
            CONTROLLER
        } else {
            CASING
        };
        if host.block(pos) != expected {
            return false;
        }
        if host.block_property(pos, "formed").as_deref() == Some("true")
            && !previous.is_some_and(|old| old.contains(pos))
        {
            return false;
        }
    }
    true
}
fn find_unit(
    host: &dyn Host,
    controller: [i32; 3],
    preferred: &str,
    previous: Option<Unit>,
) -> Option<Unit> {
    let first = facing(preferred).unwrap_or(FACINGS[0]);
    let start = FACINGS
        .iter()
        .position(|candidate| *candidate == first)
        .unwrap_or(0);
    for offset in [0, 1, 2, 3] {
        let direction = FACINGS[(start + offset) % FACINGS.len()];
        let well = [
            controller[0] - direction.vector[0] * (LENGTH - 1),
            controller[1],
            controller[2] - direction.vector[2] * (LENGTH - 1),
        ];
        let unit = Unit {
            facing: direction,
            well,
        };
        if unit.controller() == controller && valid_unit(host, unit, previous) {
            return Some(unit);
        }
    }
    None
}
pub(crate) fn tick(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let current_facing = host
        .block_property(pos, "facing")
        .unwrap_or_else(|| "north".to_owned());
    let old = saved_unit(&entity.nbt);
    let elapsed = get_i32(&entity.nbt, "PumpjackScan").saturating_add(1);
    let facing_changed = old.is_some_and(|unit| unit.facing.name != current_facing);
    let scan = elapsed >= SCAN_INTERVAL
        || facing_changed
        || get_i32(&entity.nbt, "PumpjackInitialized") == 0;
    let unit = if scan {
        set_i32(&mut entity.nbt, "PumpjackScan", 0);
        set_i32(&mut entity.nbt, "PumpjackInitialized", 1);
        find_unit(host, pos, &current_facing, old)
    } else {
        set_i32(&mut entity.nbt, "PumpjackScan", elapsed);
        old
    };
    if unit != old || facing_changed {
        if let Some(previous) = old {
            unform_parts(host, previous, None);
        }
        write_unit(&mut entity.nbt, unit);
        if let Some(unit) = unit {
            set_formed(host, unit, true);
        }
    }
    let Some(unit) = unit else {
        set_i32(&mut entity.nbt, "OilPumpStatus", 1);
        set_i32(&mut entity.nbt, "OilPumpRate", 0);
        set_block_bool(host, pos, "formed", false);
        set_block_bool(host, pos, "lit", false);
        return;
    };
    let deposit = match crate::oil_deposit::remaining(host, unit.well) {
        Ok(Some((deposit, remaining))) => (deposit, remaining),
        Ok(None) => {
            set_i32(&mut entity.nbt, "OilPumpStatus", 2);
            set_i32(&mut entity.nbt, "OilPumpRate", 0);
            set_block_bool(host, pos, "lit", false);
            return;
        }
        Err(_) => {
            set_i32(&mut entity.nbt, "OilPumpStatus", 3);
            set_i32(&mut entity.nbt, "OilPumpRate", 0);
            set_block_bool(host, pos, "lit", false);
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
        set_i32(&mut entity.nbt, "OilPumpStatus", 4);
        set_i32(&mut entity.nbt, "OilPumpRate", 0);
        set_block_bool(host, pos, "lit", false);
        return;
    }
    if current.as_deref().is_some_and(|current| current != fluid) {
        set_i32(&mut entity.nbt, "OilPumpStatus", 5);
        set_i32(&mut entity.nbt, "OilPumpRate", 0);
        set_block_bool(host, pos, "lit", false);
        return;
    }
    if energy < ENERGY_PER_TICK {
        set_i32(&mut entity.nbt, "OilPumpStatus", 6);
        set_i32(&mut entity.nbt, "OilPumpRate", 0);
        set_block_bool(host, pos, "lit", false);
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
            match crate::oil_deposit::take(host, unit.well, requested as u64) {
                Ok(Some((_, _, taken))) => taken,
                _ => {
                    set_i32(&mut entity.nbt, "OilPumpStatus", 3);
                    set_i32(&mut entity.nbt, "OilPumpRate", 0);
                    set_block_bool(host, pos, "lit", false);
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
    set_block_bool(host, pos, "lit", true);
}
pub(crate) fn notify_placed(host: &mut dyn Host, placed: [i32; 3]) {
    for dx in -5..=5 {
        for dy in -3..=3 {
            for dz in -5..=5 {
                let controller = [placed[0] + dx, placed[1] + dy, placed[2] + dz];
                if host
                    .mod_block_entity(controller)
                    .is_some_and(|entity| entity.id == CONTROLLER)
                {
                    host.change_mod_block_entity(controller, &mut |entity| {
                        if entity.id == CONTROLLER {
                            set_i32(&mut entity.nbt, "PumpjackScan", SCAN_INTERVAL);
                        }
                    });
                }
            }
        }
    }
}
pub(crate) fn unform(host: &mut dyn Host, pos: [i32; 3], nbt: &Compound) {
    if let Some(unit) = saved_unit(nbt) {
        unform_parts(host, unit, Some(pos));
    }
}
pub(crate) fn unform_near(host: &mut dyn Host, broken: [i32; 3]) {
    for dx in -5..=5 {
        for dy in -3..=3 {
            for dz in -5..=5 {
                let controller = [broken[0] + dx, broken[1] + dy, broken[2] + dz];
                let Some(entity) = host.mod_block_entity(controller) else {
                    continue;
                };
                if entity.id != CONTROLLER {
                    continue;
                }
                let Some(unit) = saved_unit(&entity.nbt) else {
                    continue;
                };
                if unit.contains(broken) {
                    unform_parts(host, unit, None);
                    host.change_mod_block_entity(controller, &mut |current| {
                        if current.id == CONTROLLER {
                            write_unit(&mut current.nbt, None);
                            set_i32(&mut current.nbt, "PumpjackInitialized", 0);
                        }
                    });
                    host.block_entity_sync(controller);
                }
            }
        }
    }
}
pub(crate) fn face(host: &mut dyn Host, pos: [i32; 3], name: &str) {
    if facing(name).is_none() {
        return;
    }
    let Some(entity) = host.mod_block_entity(pos) else {
        return;
    };
    if entity.id != CONTROLLER {
        return;
    }
    if let Some(unit) = saved_unit(&entity.nbt) {
        unform_parts(host, unit, None);
    }
    host.set_block_property(pos, "facing", name);
    host.change_mod_block_entity(pos, &mut |current| {
        if current.id == CONTROLLER {
            write_unit(&mut current.nbt, None);
            set_i32(&mut current.nbt, "PumpjackScan", 0);
            set_i32(&mut current.nbt, "PumpjackInitialized", 0);
        }
    });
    host.block_entity_sync(pos);
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
        1 => "Needs 3x5 structure and Well Head",
        2 => "No oil deposit under Well Head",
        3 => "World save unavailable",
        4 => "Oil tank full",
        5 => "Oil tank contains another fluid",
        6 => "No power",
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
