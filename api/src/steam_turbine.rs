//! Steam Turbine Array alpha. Shell dimensions and generation follow the pinned
//! Arcforge defaults. Horizon declares energy/fluid capabilities by block ID,
//! so this uses a separate controller and fixed capability capacities.
use horizon_client::{
    containers::ModBlockEntity,
    mod_api::{Compound, Host, Tag},
};

pub(crate) const CONTROLLER: &str = "arcforge:steam_turbine_array_controller";
pub(crate) const CASING: &str = "arcforge:steam_turbine_array_casing";
pub(crate) const PRESSURE_GLASS: &str = "arcforge:pressure_glass";
pub(crate) const EXHAUST_STEAM: &str = "arcforge:exhaust_steam";
pub(crate) const STEAM_TANK_CAPACITY: u32 = 96_000;
pub(crate) const LUBRICANT_TANK_CAPACITY: u32 = 4_000;
pub(crate) const EXHAUST_TANK_CAPACITY: u32 = 96_000;

const MIN_WIDTH: u32 = 3;
const MAX_WIDTH: u32 = 7;
const MIN_HEIGHT: u32 = 3;
const MAX_HEIGHT: u32 = 7;
const MIN_LENGTH: u32 = 3;
const MAX_LENGTH: u32 = 27;
const FLOW_PER_SECTION: f64 = 60.0;
const MAX_SECTIONS: f64 = 7.0 * 7.0 / 9.0 * 27.0;
const MAX_FLOW: u32 = (FLOW_PER_SECTION * MAX_SECTIONS) as u32;
const MAX_OUTPUT_FE: u32 = 582_826;
const ENERGY_BUFFER_CAPACITY: u32 = 4_762_560;
const MAX_RPM: u32 = 3_600;
const SPIN_UP: f64 = 0.01;
const SPIN_DOWN: f64 = 0.005;
const LUBRICANT_INTERVAL: f64 = 20.0;
const LUBRICANT_BONUS: f64 = 0.08;
const PREMIUM_LUBRICANT_BONUS: f64 = 0.12;
const VACUUM_BONUS: f64 = 0.10;
const SCAN_INTERVAL: i32 = 20;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Shell {
    width: u32,
    height: u32,
    length: u32,
}

impl Shell {
    fn sections(self) -> f64 {
        f64::from(self.width) * f64::from(self.height) / 9.0 * f64::from(self.length)
    }

    fn max_flow(self) -> u32 {
        (FLOW_PER_SECTION * self.sections())
            .round()
            .clamp(1.0, f64::from(MAX_FLOW)) as u32
    }

    fn contains(self, center: [i32; 3], facing: &str, pos: [i32; 3]) -> bool {
        let Some((out, axis)) = orientation(facing) else {
            return false;
        };
        let inward = [-out[0], -out[1], -out[2]];
        let along = [pos[0] - center[0], pos[1] - center[1], pos[2] - center[2]];
        let length_offset = along[0] * axis[0] + along[2] * axis[2];
        let width_offset = along[0] * inward[0] + along[2] * inward[2];
        let height_offset = along[1];
        let center_length = ((self.length - 1) / 2) as i32;
        let center_height = ((self.height - 1) / 2) as i32;
        let length_index = length_offset + center_length;
        let height_index = height_offset + center_height;
        length_index >= 0
            && length_index < self.length as i32
            && (0..self.width as i32).contains(&width_offset)
            && height_index >= 0
            && height_index < self.height as i32
    }

    fn positions(self, center: [i32; 3], facing: &str) -> Vec<[i32; 3]> {
        let Some((out, axis)) = orientation(facing) else {
            return Vec::new();
        };
        let inward = [-out[0], -out[1], -out[2]];
        let center_length = ((self.length - 1) / 2) as i32;
        let center_height = ((self.height - 1) / 2) as i32;
        let mut positions = Vec::with_capacity((self.width * self.height * self.length) as usize);
        for length in 0..self.length {
            for height in 0..self.height {
                for width in 0..self.width {
                    let l = length as i32 - center_length;
                    let y = height as i32 - center_height;
                    let d = width as i32;
                    positions.push([
                        center[0] + axis[0] * l + inward[0] * d,
                        center[1] + y,
                        center[2] + axis[2] * l + inward[2] * d,
                    ]);
                }
            }
        }
        positions
    }
}

pub(crate) fn tick(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let facing = host
        .block_property(pos, "facing")
        .unwrap_or_else(|| "north".to_owned());
    let elapsed = get_i32(&entity.nbt, "TurbineScan").saturating_add(1);
    let old_shell = saved_shell(&entity.nbt);
    let previous_facing = saved_facing(&entity.nbt);
    let facing_changed = previous_facing != facing;
    let should_scan = elapsed >= SCAN_INTERVAL
        || facing_changed
        || get_i32(&entity.nbt, "TurbineInitialized") == 0;
    let shell = if should_scan {
        set_i32(&mut entity.nbt, "TurbineScan", 0);
        set_i32(&mut entity.nbt, "TurbineInitialized", 1);
        find_shell(host, pos, &facing)
    } else {
        set_i32(&mut entity.nbt, "TurbineScan", elapsed);
        old_shell
    };

    if shell != old_shell || facing_changed {
        if let Some(old) = old_shell {
            set_shell_formed(host, pos, &previous_facing, old, false);
        }
        write_shell(&mut entity.nbt, shell);
        if let Some(found) = shell {
            set_shell_formed(host, pos, &facing, found, true);
        }
        set_string(&mut entity.nbt, "TurbineFacing", &facing);
    }

    let formed = shell.is_some();
    set_i32(&mut entity.nbt, "TurbineFormed", i32::from(formed));
    set_block_bool(host, pos, "formed", formed);
    let shell = shell.unwrap_or_default();
    let sections = if formed { shell.sections() } else { 3.0 };
    let max_flow = if formed { shell.max_flow() } else { 0 };
    let mut rpm = get_f64(&entity.nbt, "TurbineRpm").clamp(0.0, f64::from(MAX_RPM));
    let mut energy = get_i32(&entity.nbt, "energy").max(0) as u32;
    energy = energy.min(ENERGY_BUFFER_CAPACITY);
    let steam = crate::fluid::name(&entity.nbt, 0);
    let grade = steam.as_deref().and_then(steam_grade);
    let steam_amount = crate::fluid::amount(&entity.nbt, 0);
    let lubricant = crate::fluid::name(&entity.nbt, 1);
    let lubricant_amount = crate::fluid::amount(&entity.nbt, 1);
    let exhaust_before = crate::fluid::amount(&entity.nbt, 2);
    let exhaust_compatible =
        crate::fluid::name(&entity.nbt, 2).is_none_or(|fluid| fluid == EXHAUST_STEAM);
    let vacuum = exhaust_before < EXHAUST_TANK_CAPACITY;
    let can_run = formed && energy < ENERGY_BUFFER_CAPACITY;
    let flow = if can_run && grade.is_some() {
        if exhaust_compatible {
            crate::fluid::extract(
                &mut entity.nbt,
                0,
                steam.as_deref().unwrap_or(""),
                steam_amount.min(max_flow),
            )
        } else {
            0
        }
    } else {
        0
    };

    if flow > 0 {
        crate::fluid::insert(
            &mut entity.nbt,
            2,
            EXHAUST_STEAM,
            flow,
            EXHAUST_TANK_CAPACITY,
        );
    }

    let grade_fe = grade.map_or(0.0, |(_, fe)| fe);
    let target = if flow > 0 && max_flow > 0 {
        f64::from(MAX_RPM) * f64::from(flow) * grade_fe / (f64::from(max_flow) * 56.0)
    } else {
        0.0
    };
    let lubricated =
        lubricant_amount > 0 && lubricant.as_deref().and_then(lubricant_bonus).is_some();
    let spin_up = SPIN_UP * if lubricated && flow > 0 { 2.0 } else { 1.0 };
    rpm += (target - rpm) * if target > rpm { spin_up } else { SPIN_DOWN };
    if rpm < 0.5 && target == 0.0 {
        rpm = 0.0;
    }

    let lube_share = if lubricated {
        lubricant
            .as_deref()
            .and_then(lubricant_bonus)
            .unwrap_or(0.0)
    } else {
        0.0
    };
    let mut generated = 0;
    if flow > 0 && grade.is_some() {
        let spun = (rpm / target.max(1.0)).clamp(0.0, 1.0);
        let bonus = 1.0 + lube_share + if vacuum { VACUUM_BONUS } else { 0.0 };
        let max_output =
            (f64::from(max_flow) * 56.0 * (1.0 + PREMIUM_LUBRICANT_BONUS + VACUUM_BONUS))
                .round()
                .min(f64::from(MAX_OUTPUT_FE)) as u32;
        let wanted = (f64::from(flow) * grade_fe * spun * bonus).round() as u32;
        generated = wanted
            .min(max_output)
            .min(ENERGY_BUFFER_CAPACITY.saturating_sub(energy));
        energy += generated;
        if lubricated {
            consume_lubricant(&mut entity.nbt, sections);
        }
    }

    set_i32(&mut entity.nbt, "energy", energy as i32);
    set_i32(&mut entity.nbt, "TurbineFlow", flow as i32);
    set_i32(&mut entity.nbt, "TurbineFEPerTick", generated as i32);
    set_f64(&mut entity.nbt, "TurbineRpm", rpm);
    set_block_bool(host, pos, "lit", flow > 0);
}

fn find_shell(host: &dyn Host, controller: [i32; 3], facing: &str) -> Option<Shell> {
    let (out, axis) = orientation(facing)?;
    let inward = [-out[0], -out[1], -out[2]];
    let mut candidates = Vec::with_capacity(625);
    for length in MIN_LENGTH..=MAX_LENGTH {
        for width in MIN_WIDTH..=MAX_WIDTH {
            for height in MIN_HEIGHT..=MAX_HEIGHT {
                candidates.push((length * width * height, width, height, length));
            }
        }
    }
    candidates.sort_unstable();
    for (_, width, height, length) in candidates {
        let shell = Shell {
            width,
            height,
            length,
        };
        let center_length = (length - 1) / 2;
        let center_height = (height - 1) / 2;
        let wide = width != 3 || height != 3;
        let mut valid = true;
        'shell: for l in 0..length {
            for y in 0..height {
                for d in 0..width {
                    let length_offset = l as i32 - center_length as i32;
                    let height_offset = y as i32 - center_height as i32;
                    let width_offset = d as i32;
                    let pos = [
                        controller[0] + axis[0] * length_offset + inward[0] * width_offset,
                        controller[1] + height_offset,
                        controller[2] + axis[2] * length_offset + inward[2] * width_offset,
                    ];
                    let boundaries = usize::from(l == 0 || l + 1 == length)
                        + usize::from(y == 0 || y + 1 == height)
                        + usize::from(d == 0 || d + 1 == width);
                    let block = host.block(pos);
                    let is_controller = l == center_length && y == center_height && d == 0;
                    if is_controller {
                        if pos != controller || block != CONTROLLER {
                            valid = false;
                            break 'shell;
                        }
                    } else if boundaries == 3 || (wide && boundaries >= 2) {
                        if block != CASING {
                            valid = false;
                            break 'shell;
                        }
                    } else if boundaries > 0 {
                        if block != CASING && block != PRESSURE_GLASS {
                            valid = false;
                            break 'shell;
                        }
                    } else if !matches!(
                        block.as_str(),
                        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
                    ) {
                        valid = false;
                        break 'shell;
                    }
                }
            }
        }
        if valid {
            return Some(shell);
        }
    }
    None
}

fn set_shell_formed(
    host: &mut dyn Host,
    controller: [i32; 3],
    facing: &str,
    shell: Shell,
    formed: bool,
) {
    for pos in shell.positions(controller, facing) {
        match host.block(pos).as_str() {
            CONTROLLER | CASING | PRESSURE_GLASS => {
                host.set_block_property(pos, "formed", if formed { "true" } else { "false" });
            }
            _ => {}
        }
    }
}

pub(crate) fn unform(host: &mut dyn Host, pos: [i32; 3], nbt: &Compound) {
    if let Some(shell) = saved_shell(nbt) {
        let facing = saved_facing(nbt);
        set_shell_formed(host, pos, &facing, shell, false);
    }
}

pub(crate) fn face(host: &mut dyn Host, pos: [i32; 3], facing: &str) {
    let Some(entity) = host.mod_block_entity(pos) else {
        return;
    };
    let current = host
        .block_property(pos, "facing")
        .unwrap_or_else(|| saved_facing(&entity.nbt));
    if entity.id != CONTROLLER || current == facing {
        return;
    }
    unform(host, pos, &entity.nbt);
    host.set_block_property(pos, "facing", facing);
    host.change_mod_block_entity(pos, &mut |current| {
        if current.id == CONTROLLER {
            write_shell(&mut current.nbt, None);
            set_string(&mut current.nbt, "TurbineFacing", facing);
            set_i32(&mut current.nbt, "TurbineInitialized", 0);
            set_i32(&mut current.nbt, "TurbineScan", 0);
        }
    });
    host.block_entity_sync(pos);
}

pub(crate) fn notify_placed(host: &mut dyn Host, pos: [i32; 3]) {
    for dx in -13..=13 {
        for dy in -3..=3 {
            for dz in -13..=13 {
                let controller = [pos[0] + dx, pos[1] + dy, pos[2] + dz];
                let Some(entity) = host.mod_block_entity(controller) else {
                    continue;
                };
                if entity.id == CONTROLLER {
                    host.change_mod_block_entity(controller, &mut |current| {
                        if current.id == CONTROLLER {
                            set_i32(&mut current.nbt, "TurbineScan", SCAN_INTERVAL);
                        }
                    });
                }
            }
        }
    }
}

pub(crate) fn unform_near(host: &mut dyn Host, broken: [i32; 3]) {
    for dx in -13..=13 {
        for dy in -3..=3 {
            for dz in -13..=13 {
                let controller = [broken[0] + dx, broken[1] + dy, broken[2] + dz];
                let Some(entity) = host.mod_block_entity(controller) else {
                    continue;
                };
                if entity.id != CONTROLLER {
                    continue;
                }
                let Some(shell) = saved_shell(&entity.nbt) else {
                    continue;
                };
                let facing = saved_facing(&entity.nbt);
                if shell.contains(controller, &facing, broken) {
                    set_shell_formed(host, controller, &facing, shell, false);
                    host.change_mod_block_entity(controller, &mut |current| {
                        if current.id == CONTROLLER {
                            write_shell(&mut current.nbt, None);
                            set_i32(&mut current.nbt, "TurbineInitialized", 0);
                        }
                    });
                    host.block_entity_sync(controller);
                }
            }
        }
    }
}

pub(crate) fn status(entity: &ModBlockEntity) -> String {
    let formed = get_i32(&entity.nbt, "TurbineFormed") != 0;
    let shell = saved_shell(&entity.nbt).unwrap_or_default();
    let rpm = get_f64(&entity.nbt, "TurbineRpm").round() as u32;
    let steam_name = crate::fluid::name(&entity.nbt, 0).unwrap_or_else(|| "none".to_owned());
    let lubricant = crate::fluid::amount(&entity.nbt, 1);
    let exhaust = crate::fluid::amount(&entity.nbt, 2);
    let energy = get_i32(&entity.nbt, "energy").max(0);
    let flow = get_i32(&entity.nbt, "TurbineFlow").max(0);
    let generated = get_i32(&entity.nbt, "TurbineFEPerTick").max(0);
    let shape = format!("{}x{}x{}", shell.width, shell.height, shell.length);
    format!(
        "Turbine {} {shape} | {rpm}/{MAX_RPM} RPM | {flow} mB/t {steam_name} | {generated} FE/t | Energy {energy}/{ENERGY_BUFFER_CAPACITY} | Lube {lubricant} | Exhaust {exhaust}/{EXHAUST_TANK_CAPACITY}",
        if formed { "formed" } else { "unformed" }
    )
}

pub(crate) fn accepts(fluid: &str) -> bool {
    steam_grade(fluid).is_some() || lubricant_bonus(fluid).is_some()
}

pub(crate) fn fluid_tanks() -> horizon_client::mod_api::ModFluidTanks {
    horizon_client::mod_api::ModFluidTanks {
        tanks: vec![
            STEAM_TANK_CAPACITY as i32,
            LUBRICANT_TANK_CAPACITY as i32,
            EXHAUST_TANK_CAPACITY as i32,
        ],
        faces: [
            vec![0, 1, 2],
            vec![0, 1, 2],
            vec![0, 1, 2],
            vec![0, 1, 2],
            vec![0, 1, 2],
            vec![0, 1, 2],
        ],
        inputs: vec![0, 1],
        outputs: vec![2],
    }
}

pub(crate) fn energy_buffer() -> horizon_client::mod_api::ModEnergyBuffer {
    horizon_client::mod_api::ModEnergyBuffer {
        capacity: ENERGY_BUFFER_CAPACITY as i32,
        max_insert: 0,
        max_extract: MAX_OUTPUT_FE as i32,
        insert_faces: [false; 6],
        extract_faces: [true; 6],
    }
}

fn consume_lubricant(nbt: &mut Compound, sections: f64) {
    let mut used = get_f64(nbt, "TurbineLubeUsed").max(0.0);
    used += sections / 3.0 / LUBRICANT_INTERVAL;
    let available = crate::fluid::amount(nbt, 1);
    let whole = (used.floor() as u32).min(available);
    if whole > 0 {
        let fluid = crate::fluid::name(nbt, 1).unwrap_or_default();
        crate::fluid::extract(nbt, 1, &fluid, whole);
        used -= f64::from(whole);
    }
    set_f64(nbt, "TurbineLubeUsed", used);
}

fn steam_grade(fluid: &str) -> Option<(u32, f64)> {
    match fluid {
        "arcforge:steam" => Some((1, 20.0)),
        "arcforge:high_pressure_steam" => Some((2, 36.0)),
        "arcforge:superheated_steam" => Some((3, 56.0)),
        _ => None,
    }
}

fn lubricant_bonus(fluid: &str) -> Option<f64> {
    match fluid {
        "arcforge:heavy_oil" | "arcforge:seed_oil" => Some(LUBRICANT_BONUS),
        "arcforge:lubricant" => Some(PREMIUM_LUBRICANT_BONUS),
        _ => None,
    }
}

fn orientation(facing: &str) -> Option<([i32; 3], [i32; 3])> {
    match facing {
        "north" => Some(([0, 0, -1], [-1, 0, 0])),
        "east" => Some(([1, 0, 0], [0, 0, -1])),
        "south" => Some(([0, 0, 1], [1, 0, 0])),
        "west" => Some(([-1, 0, 0], [0, 0, 1])),
        _ => None,
    }
}

fn saved_shell(nbt: &Compound) -> Option<Shell> {
    if get_i32(nbt, "TurbineFormed") == 0 {
        return None;
    }
    let shell = Shell {
        width: get_i32(nbt, "TurbineWidth").clamp(MIN_WIDTH as i32, MAX_WIDTH as i32) as u32,
        height: get_i32(nbt, "TurbineHeight").clamp(MIN_HEIGHT as i32, MAX_HEIGHT as i32) as u32,
        length: get_i32(nbt, "TurbineLength").clamp(MIN_LENGTH as i32, MAX_LENGTH as i32) as u32,
    };
    Some(shell)
}

fn saved_facing(nbt: &Compound) -> String {
    get_string(nbt, "TurbineFacing").unwrap_or_else(|| "north".to_owned())
}

fn write_shell(nbt: &mut Compound, shell: Option<Shell>) {
    set_i32(nbt, "TurbineFormed", i32::from(shell.is_some()));
    if let Some(shell) = shell {
        set_i32(nbt, "TurbineWidth", shell.width as i32);
        set_i32(nbt, "TurbineHeight", shell.height as i32);
        set_i32(nbt, "TurbineLength", shell.length as i32);
    } else {
        set_i32(nbt, "TurbineWidth", 3);
        set_i32(nbt, "TurbineHeight", 3);
        set_i32(nbt, "TurbineLength", 3);
    }
}

fn set_block_bool(host: &mut dyn Host, pos: [i32; 3], property: &str, value: bool) {
    let state = if value { "true" } else { "false" };
    if host.block_property(pos, property).as_deref() != Some(state) {
        host.set_block_property(pos, property, state);
    }
}

fn get_i32(nbt: &Compound, key: &str) -> i32 {
    match nbt.get(&key.into()) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn set_i32(nbt: &mut Compound, key: &str, value: i32) {
    nbt.insert(key.into(), Tag::Int(value));
}

fn get_f64(nbt: &Compound, key: &str) -> f64 {
    match nbt.get(&key.into()) {
        Some(Tag::Double(bits)) => {
            let value = f64::from_bits(*bits);
            if value.is_finite() { value } else { 0.0 }
        }
        _ => 0.0,
    }
}

fn set_f64(nbt: &mut Compound, key: &str, value: f64) {
    nbt.insert(
        key.into(),
        Tag::Double(if value.is_finite() {
            value.to_bits()
        } else {
            0.0_f64.to_bits()
        }),
    );
}

fn get_string(nbt: &Compound, key: &str) -> Option<String> {
    match nbt.get(&key.into()) {
        Some(Tag::String(value)) => Some(value.to_string_lossy()),
        _ => None,
    }
}

fn set_string(nbt: &mut Compound, key: &str, value: &str) {
    nbt.insert(key.into(), Tag::String(value.into()));
}
