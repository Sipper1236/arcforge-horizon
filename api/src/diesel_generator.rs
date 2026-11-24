//! Pinned Arcforge Diesel Generator fuel-to-FE behaviour.
//!
//! Horizon's block-entity tank and energy declarations are block-ID-wide. The
//! fuel tank therefore keeps the V16 maximum for every engine, while generated
//! FE and the internal energy buffer follow the formed V8/V12/V16 length.

use horizon_client::{
    containers::{ModBlockEntity, ModContainer},
    mod_api::{Compound, Host, ModEnergyBuffer, ModFluidTanks, Tag},
};

pub(crate) const CONTROLLER: &str = "arcforge:diesel_generator_controller";
pub(crate) const CASING: &str = "arcforge:diesel_generator_casing";

const WIDTH: i32 = 3;
const HEIGHT: i32 = 3;
const LENGTHS: [i32; 3] = [9, 7, 5];
const V8_OUTPUT: u64 = 3_000;
const V12_OUTPUT: u64 = 8_000;
const V16_OUTPUT: u64 = 15_000;
const V8_ENERGY_CAPACITY: u64 = 400_000;
const MAX_FUEL_CAPACITY: u32 = 28_800;
const MAX_ENERGY_CAPACITY: u64 = 2_000_000;
const MAX_OUTPUT_RATE: u64 = 16_384;
const SPIN_UP_TICKS: u32 = 60;
const STRUCTURE_SCAN_TICKS: i32 = 20;
const REDSTONE_SCAN_TICKS: i32 = 10;
const FUEL_TANK: usize = 0;
const DATA_SLOTS: usize = 14;

const DIESEL: &str = "arcforge:diesel";
const GASOLINE: &str = "arcforge:gasoline";
const KEROSENE: &str = "arcforge:kerosene";
const BIODIESEL: &str = "arcforge:biodiesel";
const REFINERY_GAS: &str = "arcforge:refinery_gas";

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
    FACINGS.into_iter().find(|value| value.name == name)
}

fn facing_from_hit(face: [i32; 3]) -> Option<Facing> {
    let name = match face {
        [0, 0, -1] => "north",
        [1, 0, 0] => "east",
        [0, 0, 1] => "south",
        [-1, 0, 0] => "west",
        _ => return None,
    };
    facing(name)
}

// The controller is at the centre of the bottom row on the front end.
fn box_positions(controller: [i32; 3], direction: Facing, length: i32) -> Vec<[i32; 3]> {
    let inward = [-direction.vector[0], 0, -direction.vector[2]];
    let right = [-inward[2], 0, inward[0]];
    let mut positions = Vec::with_capacity((WIDTH * HEIGHT * length) as usize);
    for depth in 0..length {
        for y in 0..HEIGHT {
            for x in -1..=1 {
                positions.push([
                    controller[0] + inward[0] * depth + right[0] * x,
                    controller[1] + y,
                    controller[2] + inward[2] * depth + right[2] * x,
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

fn set_i32(nbt: &mut Compound, name: &str, value: i32) {
    let Tag::String(key) = Tag::String(name.into()) else {
        unreachable!()
    };
    nbt.insert(key, Tag::Int(value));
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

fn set_bool(host: &mut dyn Host, pos: [i32; 3], property: &str, value: bool) {
    let value = if value { "true" } else { "false" };
    if host.block_property(pos, property).as_deref() != Some(value) {
        host.set_block_property(pos, property, value);
    }
}

fn is_formed(entity: &ModBlockEntity) -> bool {
    get_i32(&entity.nbt, "DieselGeneratorFormed") != 0
}

fn engine_length(entity: &ModBlockEntity) -> i32 {
    match get_i32(&entity.nbt, "DieselGeneratorLength") {
        7 => 7,
        9 => 9,
        _ => 5,
    }
}

fn max_output(length: i32) -> u64 {
    match length {
        7 => V12_OUTPUT,
        9 => V16_OUTPUT,
        _ => V8_OUTPUT,
    }
}

fn energy_capacity(length: i32) -> u64 {
    V8_ENERGY_CAPACITY
        .saturating_mul(max_output(length))
        .checked_div(V8_OUTPUT)
        .unwrap_or(V8_ENERGY_CAPACITY)
        .min(MAX_ENERGY_CAPACITY)
}

fn fuel_fe_per_mb(fluid: &str) -> Option<u64> {
    // Defaults from the pinned README. Its engine_fuels data map omits
    // Biodiesel even though that same pinned README lists it as a fuel.
    match fluid {
        DIESEL => Some(420),
        GASOLINE => Some(400),
        KEROSENE => Some(360),
        BIODIESEL => Some(340),
        REFINERY_GAS => Some(240),
        _ => None,
    }
}

fn fuel_code(fluid: &str) -> i32 {
    match fluid {
        DIESEL => 1,
        GASOLINE => 2,
        KEROSENE => 3,
        BIODIESEL => 4,
        REFINERY_GAS => 5,
        _ => 0,
    }
}

fn engine_efficiency_percent(length: i32) -> u64 {
    match length {
        7 => 105,
        9 => 110,
        _ => 100,
    }
}

fn scaled_fe_per_mb(fluid: &str, length: i32) -> Option<u64> {
    fuel_fe_per_mb(fluid).map(|value| {
        value
            .saturating_mul(engine_efficiency_percent(length))
            .saturating_add(50)
            / 100
    })
}

fn valid_box(
    host: &dyn Host,
    positions: &[[i32; 3]],
    controller: [i32; 3],
    allowed_formed: &[[i32; 3]],
) -> bool {
    positions.iter().all(|part| {
        if *part == controller {
            host.block(*part) == CONTROLLER
        } else {
            host.block(*part) == CASING
                && (host.block_property(*part, "formed").as_deref() != Some("true")
                    || allowed_formed.contains(part))
        }
    })
}

fn save_facing_length(nbt: &Compound) -> Option<(Facing, i32)> {
    Some((
        facing(get_string(nbt, "DieselGeneratorFacing")?.as_str())?,
        match get_i32(nbt, "DieselGeneratorLength") {
            5 => 5,
            7 => 7,
            9 => 9,
            _ => return None,
        },
    ))
}

fn form(
    host: &mut dyn Host,
    controller: [i32; 3],
    direction: Facing,
    length: i32,
    entity: &mut ModBlockEntity,
) -> bool {
    let positions = box_positions(controller, direction, length);
    let previous = save_facing_length(&entity.nbt)
        .map(|(face, len)| box_positions(controller, face, len))
        .unwrap_or_default();
    if positions.len() != (WIDTH * HEIGHT * length) as usize
        || !valid_box(host, &positions, controller, &previous)
    {
        return false;
    }
    for part in positions {
        if part != controller {
            set_bool(host, part, "formed", true);
        }
    }
    set_bool(host, controller, "formed", true);
    set_bool(host, controller, "lit", false);
    set_i32(&mut entity.nbt, "DieselGeneratorFormed", 1);
    set_i32(&mut entity.nbt, "DieselGeneratorLength", length);
    set_i32(
        &mut entity.nbt,
        "DieselGeneratorStructureScan",
        STRUCTURE_SCAN_TICKS,
    );
    set_string(&mut entity.nbt, "DieselGeneratorFacing", direction.name);
    host.block_entity_sync(controller);
    true
}

fn ordered_facings(host: &dyn Host, controller: [i32; 3]) -> Vec<Facing> {
    let current = host
        .block_property(controller, "facing")
        .and_then(|name| facing(&name))
        .unwrap_or(FACINGS[0]);
    std::iter::once(current)
        .chain(
            FACINGS
                .into_iter()
                .filter(|other| other.name != current.name),
        )
        .collect()
}

fn scan(host: &mut dyn Host, controller: [i32; 3], entity: &mut ModBlockEntity) -> bool {
    let previous = save_facing_length(&entity.nbt)
        .map(|(face, length)| box_positions(controller, face, length))
        .unwrap_or_default();
    for direction in ordered_facings(host, controller) {
        for length in LENGTHS {
            if form(host, controller, direction, length, entity) {
                return true;
            }
        }
    }
    // A stale saved formation must not make casing blocks appear formed.
    if !previous.is_empty() && !is_formed(entity) {
        for part in previous {
            if part != controller && host.block(part) == CASING {
                set_bool(host, part, "formed", false);
            }
        }
    }
    false
}

fn clear_box(host: &mut dyn Host, controller: [i32; 3], direction: Facing, length: i32) {
    for part in box_positions(controller, direction, length) {
        if part != controller && host.block(part) == CASING {
            set_bool(host, part, "formed", false);
        }
    }
    set_bool(host, controller, "formed", false);
    set_bool(host, controller, "lit", false);
    host.change_mod_block_entity(controller, &mut |entity| {
        if entity.id == CONTROLLER {
            set_i32(&mut entity.nbt, "DieselGeneratorFormed", 0);
            set_i32(&mut entity.nbt, "DieselGeneratorStructureScan", 0);
            set_i32(&mut entity.nbt, "DieselGeneratorSpeed", 0);
        }
    });
    host.block_entity_sync(controller);
}

pub(crate) fn notify_placed(host: &mut dyn Host, placed: [i32; 3]) {
    for dy in 0..HEIGHT {
        for dz in -9..=9 {
            for dx in -9..=9 {
                let controller = [placed[0] + dx, placed[1] - dy, placed[2] + dz];
                if host
                    .mod_block_entity(controller)
                    .is_some_and(|entity| entity.id == CONTROLLER)
                {
                    host.change_mod_block_entity(controller, &mut |entity| {
                        if entity.id == CONTROLLER {
                            set_i32(&mut entity.nbt, "DieselGeneratorStructureScan", 0);
                        }
                    });
                }
            }
        }
    }
}

pub(crate) fn unform(host: &mut dyn Host, controller: [i32; 3], nbt: &Compound) {
    if let Some((direction, length)) = save_facing_length(nbt) {
        clear_box(host, controller, direction, length);
    }
}

pub(crate) fn unform_near(host: &mut dyn Host, broken: [i32; 3]) {
    for dy in 0..HEIGHT {
        for dz in -9..=9 {
            for dx in -9..=9 {
                let controller = [broken[0] + dx, broken[1] - dy, broken[2] + dz];
                let Some(entity) = host.mod_block_entity(controller) else {
                    continue;
                };
                if entity.id != CONTROLLER || !is_formed(&entity) {
                    continue;
                }
                let Some((direction, length)) = save_facing_length(&entity.nbt) else {
                    continue;
                };
                if box_positions(controller, direction, length).contains(&broken) {
                    clear_box(host, controller, direction, length);
                }
            }
        }
    }
}

fn status_code(entity: &ModBlockEntity) -> i32 {
    get_i32(&entity.nbt, "DieselGeneratorStatus").clamp(0, 5)
}

fn sync_screen(entity: &mut ModBlockEntity, made: u64, status: i32, powered: bool) {
    entity.data.resize(DATA_SLOTS, 0);
    let length = engine_length(entity);
    entity.data[0] = get_i32(&entity.nbt, "energy").max(0);
    entity.data[1] = energy_capacity(length).min(i32::MAX as u64) as i32;
    entity.data[2] = made.min(i32::MAX as u64) as i32;
    entity.data[3] = max_output(length).min(i32::MAX as u64) as i32;
    entity.data[4] = get_i32(&entity.nbt, "DieselGeneratorSpeed").clamp(0, 120) * 1000 / 120;
    entity.data[5] = crate::fluid::amount(&entity.nbt, FUEL_TANK).min(i32::MAX as u32) as i32;
    entity.data[6] = MAX_FUEL_CAPACITY.min(i32::MAX as u32) as i32;
    let credit = get_i32(&entity.nbt, "DieselGeneratorFuelCredit").max(0);
    let fuel_name = if credit > 0 {
        get_string(&entity.nbt, "DieselGeneratorCreditFuel")
    } else {
        crate::fluid::name(&entity.nbt, FUEL_TANK)
    };
    entity.data[7] = fuel_name.as_deref().map(fuel_code).unwrap_or_default();
    entity.data[8] = status;
    entity.data[9] = i32::from(is_formed(entity));
    entity.data[10] = length;
    entity.data[11] = get_i32(&entity.nbt, "redstone_mode").clamp(0, 2);
    entity.data[12] = fuel_name
        .as_deref()
        .and_then(|fluid| scaled_fe_per_mb(fluid, length))
        .unwrap_or_default() as i32;
    entity.data[13] = i32::from(powered);
}

fn redstone_allowed(mode: i32, powered: bool) -> bool {
    match mode {
        1 => powered,  // high
        2 => !powered, // low
        0 => true,     // ignore
        _ => false,    // pulse and throttle are not implemented for this generator
    }
}

pub(crate) fn tick(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    entity.items.clear();

    let mut scan_count = get_i32(&entity.nbt, "DieselGeneratorStructureScan").max(0);
    if is_formed(entity) && scan_count == 0 {
        let Some((direction, length)) = save_facing_length(&entity.nbt) else {
            set_i32(&mut entity.nbt, "DieselGeneratorFormed", 0);
            set_i32(&mut entity.nbt, "DieselGeneratorSpeed", 0);
            set_i32(&mut entity.nbt, "DieselGeneratorStatus", 1);
            set_i32(&mut entity.nbt, "DieselGeneratorOutput", 0);
            set_bool(host, pos, "formed", false);
            set_bool(host, pos, "lit", false);
            sync_screen(entity, 0, 1, false);
            return;
        };
        let positions = box_positions(pos, direction, length);
        let previous = positions.clone();
        if !valid_box(host, &positions, pos, &previous) {
            clear_box(host, pos, direction, length);
            set_i32(&mut entity.nbt, "DieselGeneratorFormed", 0);
            set_i32(&mut entity.nbt, "DieselGeneratorSpeed", 0);
            set_i32(&mut entity.nbt, "DieselGeneratorStatus", 1);
            set_i32(&mut entity.nbt, "DieselGeneratorOutput", 0);
            sync_screen(entity, 0, 1, false);
            return;
        } else {
            scan_count = STRUCTURE_SCAN_TICKS;
            set_i32(&mut entity.nbt, "DieselGeneratorStructureScan", scan_count);
        }
    } else if !is_formed(entity) && scan_count == 0 {
        let _ = scan(host, pos, entity);
        if !is_formed(entity) {
            scan_count = STRUCTURE_SCAN_TICKS;
            set_i32(&mut entity.nbt, "DieselGeneratorStructureScan", scan_count);
        }
    } else {
        scan_count -= 1;
        set_i32(&mut entity.nbt, "DieselGeneratorStructureScan", scan_count);
    }

    if !is_formed(entity) {
        set_i32(&mut entity.nbt, "DieselGeneratorSpeed", 0);
        set_i32(&mut entity.nbt, "DieselGeneratorStatus", 1);
        set_i32(&mut entity.nbt, "DieselGeneratorOutput", 0);
        set_bool(host, pos, "lit", false);
        sync_screen(entity, 0, 1, false);
        return;
    }

    let length = engine_length(entity);
    let energy_cap = energy_capacity(length);
    let mut stored = (get_i32(&entity.nbt, "energy").max(0) as u64).min(energy_cap);
    let fuel_amount = crate::fluid::amount(&entity.nbt, FUEL_TANK);
    let tank_fuel = crate::fluid::name(&entity.nbt, FUEL_TANK);
    let mut credit = get_i32(&entity.nbt, "DieselGeneratorFuelCredit").max(0) as u64;
    let mut credit_fuel = get_string(&entity.nbt, "DieselGeneratorCreditFuel");

    let mut powered = get_i32(&entity.nbt, "DieselGeneratorPowered") != 0;
    let mut redstone_scan = get_i32(&entity.nbt, "DieselGeneratorRedstoneScan").max(0);
    if redstone_scan == 0 {
        let Some((direction, len)) = save_facing_length(&entity.nbt) else {
            return;
        };
        powered = box_positions(pos, direction, len)
            .iter()
            .any(|part| host.neighbor_signal(host.dimension(), *part) > 0);
        set_i32(
            &mut entity.nbt,
            "DieselGeneratorPowered",
            i32::from(powered),
        );
        redstone_scan = REDSTONE_SCAN_TICKS;
    } else {
        redstone_scan -= 1;
    }
    set_i32(
        &mut entity.nbt,
        "DieselGeneratorRedstoneScan",
        redstone_scan,
    );

    let mode = get_i32(&entity.nbt, "redstone_mode");
    let allowed = redstone_allowed(mode, powered);
    let current_fuel = if credit > 0 {
        credit_fuel.clone()
    } else {
        tank_fuel.clone()
    };
    let current_profile = current_fuel
        .as_deref()
        .and_then(|fluid| scaled_fe_per_mb(fluid, length));
    let fuelled = current_profile.is_some() && (fuel_amount > 0 || credit > 0);

    let mut speed = get_i32(&entity.nbt, "DieselGeneratorSpeed").clamp(0, 120);
    if allowed && fuelled {
        speed = (speed + 2).min(120);
    } else {
        speed = (speed - 1).max(0);
    }
    set_i32(&mut entity.nbt, "DieselGeneratorSpeed", speed);

    let full_output = max_output(length);
    let wanted = (full_output * speed as u64 + 60) / 120;
    let room = energy_cap.saturating_sub(stored);
    let requested = wanted.min(room);
    let mut made = 0_u64;
    if allowed && fuelled && speed > 0 && requested > 0 {
        if let (Some(fluid), Some(per_mb)) = (current_fuel.as_deref(), current_profile) {
            if credit < requested && tank_fuel.as_deref() == Some(fluid) {
                let missing = requested - credit;
                let need_mb = missing.div_ceil(per_mb).min(u64::from(fuel_amount)) as u32;
                if need_mb > 0 {
                    let taken = crate::fluid::extract(&mut entity.nbt, FUEL_TANK, fluid, need_mb);
                    credit = credit.saturating_add(u64::from(taken).saturating_mul(per_mb));
                    credit_fuel = Some(fluid.to_owned());
                }
            }
            made = requested.min(credit);
            credit -= made;
            stored = stored.saturating_add(made).min(energy_cap);
        }
    }

    if credit == 0 {
        credit_fuel = None;
    }
    set_i32(
        &mut entity.nbt,
        "DieselGeneratorFuelCredit",
        credit.min(i32::MAX as u64) as i32,
    );
    if let Some(fluid) = credit_fuel.as_deref() {
        set_string(&mut entity.nbt, "DieselGeneratorCreditFuel", fluid);
    } else {
        set_string(&mut entity.nbt, "DieselGeneratorCreditFuel", "");
    }

    let status = if !allowed {
        5
    } else if !fuelled {
        if speed > 0 { 4 } else { 2 }
    } else if made == 0 || made < wanted {
        3
    } else if speed < 118 {
        4
    } else {
        0
    };
    set_i32(&mut entity.nbt, "DieselGeneratorStatus", status);
    set_i32(&mut entity.nbt, "DieselGeneratorOutput", made as i32);

    let mut buffer = crate::energy::EnergyStore::new(energy_cap, stored);
    super::push_adjacent_arcforge_energy_with_context(
        host,
        pos,
        &mut buffer,
        MAX_OUTPUT_RATE,
        false,
        Some(&entity.nbt),
        false,
    );
    stored = buffer.stored_fe();
    set_i32(
        &mut entity.nbt,
        "energy",
        stored.min(i32::MAX as u64) as i32,
    );
    set_bool(host, pos, "lit", made > 0);
    sync_screen(entity, made, status, powered);
}

pub(crate) fn accepts(tank: usize, fluid: &str) -> bool {
    tank == FUEL_TANK && fuel_fe_per_mb(fluid).is_some()
}

pub(crate) fn fluid_tanks() -> ModFluidTanks {
    ModFluidTanks {
        tanks: vec![MAX_FUEL_CAPACITY as i32],
        faces: [vec![0], vec![0], vec![0], vec![0], vec![0], vec![0]],
        inputs: vec![0],
        outputs: vec![],
    }
}

pub(crate) fn energy_buffer() -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: MAX_ENERGY_CAPACITY as i32,
        max_insert: 0,
        max_extract: MAX_OUTPUT_RATE as i32,
        insert_faces: [false; 6],
        extract_faces: [true; 6],
    }
}

pub(crate) fn energy_source_spec() -> (u64, u64) {
    (MAX_ENERGY_CAPACITY, MAX_OUTPUT_RATE)
}

pub(crate) fn container() -> ModContainer {
    ModContainer {
        // Horizon menu types require at least one slot. This output-only
        // sentinel keeps the fluid-and-energy GUI itemless.
        slots: 1,
        faces: [vec![], vec![], vec![], vec![], vec![], vec![]],
        inputs: vec![],
        outputs: vec![0],
        menu: Some("arcforge:diesel_generator".into()),
    }
}

pub(crate) fn wrench(host: &mut dyn Host, pos: [i32; 3], face_hit: [i32; 3]) {
    let Some(entity) = host.mod_block_entity(pos) else {
        return;
    };
    if entity.id != CONTROLLER {
        return;
    }
    if is_formed(&entity) {
        if face_hit == [0, 1, 0] {
            let next = (get_i32(&entity.nbt, "redstone_mode").clamp(0, 2) + 1) % 3;
            host.change_mod_block_entity(pos, &mut |current| {
                if current.id == CONTROLLER {
                    set_i32(&mut current.nbt, "redstone_mode", next);
                }
            });
        } else if let Some(side) = side_name(face_hit) {
            let key = format!("setting_{side}");
            let current = get_string(&entity.nbt, &key).unwrap_or_else(|| "auto".to_owned());
            let next = match current.as_str() {
                "auto" => "input",
                "input" => "output",
                "output" => "disabled",
                _ => "auto",
            };
            host.change_mod_block_entity(pos, &mut |current| {
                if current.id == CONTROLLER {
                    set_string(&mut current.nbt, &key, next);
                }
            });
        }
    } else if let Some(direction) = facing_from_hit(face_hit) {
        host.set_block_property(pos, "facing", direction.name);
        host.change_mod_block_entity(pos, &mut |current| {
            if current.id == CONTROLLER {
                set_string(&mut current.nbt, "DieselGeneratorFacing", direction.name);
                set_i32(&mut current.nbt, "DieselGeneratorStructureScan", 0);
            }
        });
    }
    host.block_entity_sync(pos);
    if let Some(player) = host.acting_player() {
        let current = host.mod_block_entity(pos).unwrap_or(entity);
        host.action_bar(
            player.uuid,
            horizon_client::mod_api::Text::literal(status(&current)),
        );
    }
}

fn side_name(face: [i32; 3]) -> Option<&'static str> {
    match face {
        [0, 1, 0] => Some("up"),
        [0, -1, 0] => Some("down"),
        [0, 0, -1] => Some("north"),
        [0, 0, 1] => Some("south"),
        [1, 0, 0] => Some("east"),
        [-1, 0, 0] => Some("west"),
        _ => None,
    }
}

pub(crate) fn status(entity: &ModBlockEntity) -> String {
    let status = match status_code(entity) {
        1 => "Needs a 3×3×5, 7, or 9 casing structure",
        2 => "No fuel",
        3 => "Output throttled by FE storage or fuel",
        4 => "Spinning up or coasting",
        5 => "Disabled by redstone mode",
        _ => "Generating FE",
    };
    let length = engine_length(entity);
    let fuel = crate::fluid::name(&entity.nbt, FUEL_TANK).unwrap_or_else(|| "empty".to_owned());
    let redstone_mode = redstone_mode_name(get_i32(&entity.nbt, "redstone_mode"));
    let energy = get_i32(&entity.nbt, "energy").max(0) as u64;
    let speed = get_i32(&entity.nbt, "DieselGeneratorSpeed").clamp(0, 120) * 100 / 120;
    format!(
        "Diesel Generator V{} | {status} | redstone {redstone_mode} | {fuel} {}/{} mB | {energy}/{} FE | {speed}%",
        match length {
            5 => 8,
            7 => 12,
            _ => 16,
        },
        crate::fluid::amount(&entity.nbt, FUEL_TANK),
        MAX_FUEL_CAPACITY,
        energy_capacity(length),
    )
}

fn redstone_mode_name(mode: i32) -> &'static str {
    match mode {
        1 => "high",
        2 => "low",
        _ => "ignore",
    }
}

pub(crate) fn draw_screen(
    host: &dyn Host,
    event: &mut horizon_client::mod_api::ScreenDrawEvent<'_>,
) {
    let [ox, oy] = event.origin;
    let data = event.data;
    let batch = &mut *event.batch;
    macro_rules! text {
        ($label:expr, $x:expr, $y:expr, $color:expr $(,)?) => {
            host.text(batch, $label, [ox + $x, oy + $y], $color, true)
        };
    }

    let engine = match data.get(10).copied().unwrap_or(5) {
        7 => "V12",
        9 => "V16",
        _ => "V8",
    };
    let status = match data.get(8).copied().unwrap_or_default() {
        1 => "Needs formed structure",
        2 => "No engine fuel",
        3 => "Throttled by fuel or full FE buffer",
        4 => "Spinning up / coasting",
        5 => "Disabled by redstone mode",
        _ => "Generating",
    };
    let fuel_name = match data.get(7).copied().unwrap_or_default() {
        1 => "Diesel",
        2 => "Gasoline",
        3 => "Kerosene",
        4 => "Biodiesel",
        5 => "Refinery Gas",
        _ => "No accepted fuel",
    };
    text!("Diesel Generator", 8.0, 6.0, 0xFFFFFFFF);
    text!(engine, 8.0, 20.0, 0xFFE0E0E0);
    text!(status, 8.0, 36.0, 0xFFBBBBBB);
    text!(
        &format!(
            "{fuel_name} | {} FE/mB",
            data.get(12).copied().unwrap_or_default()
        ),
        8.0,
        56.0,
        0xFFBBBBBB,
    );
    text!(
        &format!(
            "Fuel {}/{} mB",
            data.get(5).copied().unwrap_or_default(),
            data.get(6).copied().unwrap_or_default()
        ),
        8.0,
        70.0,
        0xFFE0E0E0,
    );
    text!(
        &format!(
            "Rate {}/{} FE/t",
            data.get(2).copied().unwrap_or_default(),
            data.get(3).copied().unwrap_or_default()
        ),
        8.0,
        86.0,
        0xFFE0E0E0,
    );
    text!(
        &format!(
            "Buffer {}/{} FE",
            data.first().copied().unwrap_or_default(),
            data.get(1).copied().unwrap_or_default()
        ),
        8.0,
        102.0,
        0xFFE0E0E0,
    );
    let redstone_mode = match data.get(11).copied().unwrap_or_default() {
        1 => "high",
        2 => "low",
        _ => "ignore",
    };
    let powered = if data.get(13).copied().unwrap_or_default() != 0 {
        "on"
    } else {
        "off"
    };
    text!(
        &format!(
            "Speed {}% | redstone {redstone_mode} ({powered})",
            data.get(4).copied().unwrap_or_default() / 10
        ),
        8.0,
        118.0,
        0xFFE0E0E0,
    );
    text!("Inventory", 8.0, 136.0, 0xFFFFFFFF);
}
