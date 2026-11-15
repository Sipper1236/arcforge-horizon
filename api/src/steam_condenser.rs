//! Condenser Array: a formed casing box that turns Exhaust Steam back into
//! water. Cooling and tank sizing follow the pinned Arcforge source defaults.
use horizon_client::{
    containers::ModBlockEntity,
    mod_api::{Compound, Host, Tag},
};

pub(crate) const CONTROLLER: &str = "arcforge:condenser_array_controller";
pub(crate) const CASING: &str = "arcforge:condenser_array_casing";
pub(crate) const EXHAUST_STEAM: &str = "arcforge:exhaust_steam";
pub(crate) const WATER: &str = "minecraft:water";

const MIN_SIZE: u32 = 3;
const MAX_SIZE: u32 = 7;
const BASE_RATE_PER_CUBE: u32 = 400;
const MAX_RATE_PER_CUBE: u32 = 800;
const WATER_BONUS: u32 = 20;
const ICE_BONUS: u32 = 30;
const PACKED_ICE_BONUS: u32 = 40;
const BLUE_ICE_BONUS: u32 = 60;
const TANK_CAPACITY: u32 = 203_259;
const SCAN_INTERVAL: i32 = 40;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Shell {
    width: u32,
    height: u32,
    length: u32,
}

impl Shell {
    fn volume(self) -> u32 {
        self.width * self.height * self.length
    }

    fn contains(self, center: [i32; 3], facing: &str, pos: [i32; 3]) -> bool {
        let Some((out, axis)) = orientation(facing) else {
            return false;
        };
        let inward = [-out[0], -out[1], -out[2]];
        let along = [pos[0] - center[0], pos[1] - center[1], pos[2] - center[2]];
        let length_index = along[0] * axis[0] + along[2] * axis[2] + ((self.length - 1) / 2) as i32;
        let width_index = along[0] * inward[0] + along[2] * inward[2];
        let height_index = along[1] + ((self.height - 1) / 2) as i32;
        (0..self.length as i32).contains(&length_index)
            && (0..self.width as i32).contains(&width_index)
            && (0..self.height as i32).contains(&height_index)
    }

    fn positions(self, center: [i32; 3], facing: &str) -> Vec<[i32; 3]> {
        let Some((out, axis)) = orientation(facing) else {
            return Vec::new();
        };
        let inward = [-out[0], -out[1], -out[2]];
        let center_length = ((self.length - 1) / 2) as i32;
        let center_height = ((self.height - 1) / 2) as i32;
        let mut positions = Vec::with_capacity(self.volume() as usize);
        for length in 0..self.length {
            for height in 0..self.height {
                for width in 0..self.width {
                    let l = length as i32 - center_length;
                    positions.push([
                        center[0] + axis[0] * l + inward[0] * width as i32,
                        center[1] + height as i32 - center_height,
                        center[2] + axis[2] * l + inward[2] * width as i32,
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
    let elapsed = get_i32(&entity.nbt, "CondenserScan").saturating_add(1);
    let old_shell = saved_shell(&entity.nbt);
    let previous_facing = saved_facing(&entity.nbt);
    let facing_changed = previous_facing != facing;
    let should_scan = elapsed >= SCAN_INTERVAL
        || facing_changed
        || get_i32(&entity.nbt, "CondenserInitialized") == 0;
    let shell = if should_scan {
        set_i32(&mut entity.nbt, "CondenserScan", 0);
        set_i32(&mut entity.nbt, "CondenserInitialized", 1);
        find_shell(host, pos, &facing)
    } else {
        set_i32(&mut entity.nbt, "CondenserScan", elapsed);
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
        set_string(&mut entity.nbt, "CondenserFacing", &facing);
    }

    let formed = shell.is_some();
    let shell = shell.unwrap_or_default();
    let rate = if formed {
        cooling_rate(host, pos, &facing, shell)
    } else {
        0
    };
    let exhaust = crate::fluid::amount(&entity.nbt, 0);
    let input_name = crate::fluid::name(&entity.nbt, 0);
    let water_name = crate::fluid::name(&entity.nbt, 1);
    let water_amount = crate::fluid::amount(&entity.nbt, 1);
    let compatible_water = water_name.as_deref().is_none_or(|fluid| fluid == WATER);
    let possible = if formed && compatible_water && input_name.as_deref() == Some(EXHAUST_STEAM) {
        rate.min(exhaust)
            .min(TANK_CAPACITY.saturating_sub(water_amount))
    } else {
        0
    };
    let condensed = if possible > 0 {
        let extracted = crate::fluid::extract(&mut entity.nbt, 0, EXHAUST_STEAM, possible);
        crate::fluid::insert(&mut entity.nbt, 1, WATER, extracted, TANK_CAPACITY)
    } else {
        0
    };

    set_i32(&mut entity.nbt, "CondenserRate", rate as i32);
    set_i32(&mut entity.nbt, "CondenserFlow", condensed as i32);
    set_block_bool(host, pos, "formed", formed);
    set_block_bool(host, pos, "lit", condensed > 0);
}

fn find_shell(host: &dyn Host, controller: [i32; 3], facing: &str) -> Option<Shell> {
    let (out, axis) = orientation(facing)?;
    let inward = [-out[0], -out[1], -out[2]];
    let mut candidates = Vec::with_capacity(125);
    for length in MIN_SIZE..=MAX_SIZE {
        for width in MIN_SIZE..=MAX_SIZE {
            for height in MIN_SIZE..=MAX_SIZE {
                candidates.push((length * width * height, width, height, length));
            }
        }
    }
    candidates.sort_unstable();
    'candidate: for (_, width, height, length) in candidates {
        let shell = Shell {
            width,
            height,
            length,
        };
        let center_length = ((length - 1) / 2) as i32;
        let center_height = ((height - 1) / 2) as i32;
        for l in 0..length {
            for y in 0..height {
                for d in 0..width {
                    let pos = [
                        controller[0] + axis[0] * (l as i32 - center_length) + inward[0] * d as i32,
                        controller[1] + y as i32 - center_height,
                        controller[2] + axis[2] * (l as i32 - center_length) + inward[2] * d as i32,
                    ];
                    let boundary = l == 0
                        || l + 1 == length
                        || y == 0
                        || y + 1 == height
                        || d == 0
                        || d + 1 == width;
                    let block = host.block(pos);
                    let is_controller =
                        l == center_length as u32 && y == center_height as u32 && d == 0;
                    if is_controller {
                        if pos != controller || block != CONTROLLER {
                            continue 'candidate;
                        }
                    } else if boundary {
                        if block != CASING {
                            continue 'candidate;
                        }
                    } else if !matches!(
                        block.as_str(),
                        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
                    ) {
                        continue 'candidate;
                    }
                }
            }
        }
        return Some(shell);
    }
    None
}

fn cooling_rate(host: &dyn Host, center: [i32; 3], facing: &str, shell: Shell) -> u32 {
    let positions = shell.positions(center, facing);
    let (mut min, mut max) = ([i32::MAX; 3], [i32::MIN; 3]);
    for pos in positions {
        for axis in 0..3 {
            min[axis] = min[axis].min(pos[axis]);
            max[axis] = max[axis].max(pos[axis]);
        }
    }
    let mut water = 0_u32;
    let mut ice = 0_u32;
    for x in min[0] - 1..=max[0] + 1 {
        for y in min[1] - 1..=max[1] + 1 {
            for z in min[2] - 1..=max[2] + 1 {
                let outside = usize::from(x < min[0] || x > max[0])
                    + usize::from(y < min[1] || y > max[1])
                    + usize::from(z < min[2] || z > max[2]);
                if outside != 1 {
                    continue;
                }
                let block = host.block([x, y, z]);
                ice = ice.saturating_add(match block.as_str() {
                    "minecraft:blue_ice" => BLUE_ICE_BONUS,
                    "minecraft:packed_ice" => PACKED_ICE_BONUS,
                    "minecraft:ice" => ICE_BONUS,
                    _ => 0,
                });
                if block == WATER {
                    let source = host
                        .block_property([x, y, z], "level")
                        .is_none_or(|level| level == "0");
                    if source {
                        water = water.saturating_add(WATER_BONUS);
                    }
                }
            }
        }
    }
    let volume = shell.volume();
    let cubes_x27 = f64::from(volume) / 27.0;
    let base = (f64::from(BASE_RATE_PER_CUBE) * cubes_x27).round() as u32;
    let cap = (f64::from(MAX_RATE_PER_CUBE) * cubes_x27).round() as u32;
    base.saturating_add(water).saturating_add(ice).min(cap)
}

fn set_shell_formed(
    host: &mut dyn Host,
    center: [i32; 3],
    facing: &str,
    shell: Shell,
    formed: bool,
) {
    for pos in shell.positions(center, facing) {
        match host.block(pos).as_str() {
            CONTROLLER => set_block_bool(host, pos, "formed", formed),
            CASING => {
                set_block_property(host, pos, "part", if formed { "other" } else { "none" });
                set_block_property(host, pos, "facing", facing);
                set_block_property(
                    host,
                    pos,
                    "box",
                    if shell.volume() == 27 {
                        "false"
                    } else {
                        "true"
                    },
                );
            }
            _ => {}
        }
    }
}

pub(crate) fn unform(host: &mut dyn Host, pos: [i32; 3], nbt: &Compound) {
    if let Some(shell) = saved_shell(nbt) {
        set_shell_formed(host, pos, &saved_facing(nbt), shell, false);
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
            set_string(&mut current.nbt, "CondenserFacing", facing);
            set_i32(&mut current.nbt, "CondenserInitialized", 0);
            set_i32(&mut current.nbt, "CondenserScan", 0);
        }
    });
    host.block_entity_sync(pos);
}

pub(crate) fn notify_placed(host: &mut dyn Host, pos: [i32; 3]) {
    for dx in -6..=6 {
        for dy in -6..=6 {
            for dz in -6..=6 {
                let controller = [pos[0] + dx, pos[1] + dy, pos[2] + dz];
                if host
                    .mod_block_entity(controller)
                    .is_some_and(|entity| entity.id == CONTROLLER)
                {
                    host.change_mod_block_entity(controller, &mut |entity| {
                        if entity.id == CONTROLLER {
                            set_i32(&mut entity.nbt, "CondenserScan", SCAN_INTERVAL);
                        }
                    });
                }
            }
        }
    }
}

pub(crate) fn unform_near(host: &mut dyn Host, broken: [i32; 3]) {
    for dx in -6..=6 {
        for dy in -6..=6 {
            for dz in -6..=6 {
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
                            set_i32(&mut current.nbt, "CondenserInitialized", 0);
                        }
                    });
                    host.block_entity_sync(controller);
                }
            }
        }
    }
}

pub(crate) fn accepts(fluid: &str) -> bool {
    fluid == EXHAUST_STEAM
}

pub(crate) fn fluid_tanks() -> horizon_client::mod_api::ModFluidTanks {
    horizon_client::mod_api::ModFluidTanks {
        tanks: vec![TANK_CAPACITY as i32, TANK_CAPACITY as i32],
        faces: [
            vec![0, 1],
            vec![0, 1],
            vec![0, 1],
            vec![0, 1],
            vec![0, 1],
            vec![0, 1],
        ],
        inputs: vec![0],
        outputs: vec![1],
    }
}

pub(crate) fn status(entity: &ModBlockEntity) -> String {
    let formed = get_i32(&entity.nbt, "CondenserFormed") != 0;
    let shell = saved_shell(&entity.nbt).unwrap_or_default();
    let exhaust = crate::fluid::amount(&entity.nbt, 0);
    let water = crate::fluid::amount(&entity.nbt, 1);
    let rate = get_i32(&entity.nbt, "CondenserRate").max(0);
    let flow = get_i32(&entity.nbt, "CondenserFlow").max(0);
    format!(
        "Condenser {} {}x{}x{} | {flow}/{rate} mB/t | Exhaust {exhaust}/{TANK_CAPACITY} | Water {water}/{TANK_CAPACITY}",
        if formed { "formed" } else { "unformed" },
        shell.width,
        shell.height,
        shell.length
    )
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
    if get_i32(nbt, "CondenserFormed") == 0 {
        return None;
    }
    Some(Shell {
        width: get_i32(nbt, "CondenserWidth").clamp(MIN_SIZE as i32, MAX_SIZE as i32) as u32,
        height: get_i32(nbt, "CondenserHeight").clamp(MIN_SIZE as i32, MAX_SIZE as i32) as u32,
        length: get_i32(nbt, "CondenserLength").clamp(MIN_SIZE as i32, MAX_SIZE as i32) as u32,
    })
}

fn saved_facing(nbt: &Compound) -> String {
    get_string(nbt, "CondenserFacing").unwrap_or_else(|| "north".to_owned())
}

fn write_shell(nbt: &mut Compound, shell: Option<Shell>) {
    set_i32(nbt, "CondenserFormed", i32::from(shell.is_some()));
    let shell = shell.unwrap_or_default();
    set_i32(nbt, "CondenserWidth", shell.width as i32);
    set_i32(nbt, "CondenserHeight", shell.height as i32);
    set_i32(nbt, "CondenserLength", shell.length as i32);
}

fn set_block_bool(host: &mut dyn Host, pos: [i32; 3], property: &str, value: bool) {
    let state = if value { "true" } else { "false" };
    if host.block_property(pos, property).as_deref() != Some(state) {
        host.set_block_property(pos, property, state);
    }
}

fn set_block_property(host: &mut dyn Host, pos: [i32; 3], property: &str, value: &str) {
    if host.block_property(pos, property).as_deref() != Some(value) {
        host.set_block_property(pos, property, value);
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
fn get_string(nbt: &Compound, key: &str) -> Option<String> {
    match nbt.get(&key.into()) {
        Some(Tag::String(value)) => Some(value.to_string_lossy()),
        _ => None,
    }
}
fn set_string(nbt: &mut Compound, key: &str, value: &str) {
    nbt.insert(key.into(), Tag::String(value.into()));
}
