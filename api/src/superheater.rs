use horizon_client::{
    containers::ModBlockEntity,
    mod_api::{Compound, Host, Tag},
};
pub(crate) const CONTROLLER: &str = "arcforge:superheater_array_controller";
pub(crate) const CASING: &str = "arcforge:superheater_array_casing";
const MIN: i32 = 3;
const MAX: i32 = 7;
const TANK_PER_CUBE: u64 = 16_000;
const FLOW_PER_CUBE: u64 = 1_000;
const HEAT_PER_CUBE: u64 = 200_000;
const HEAT_RATE_PER_CUBE: u64 = 2_000;
const MAX_TANK: u32 = 203_259;
const MAX_TEMP: u32 = 1_400;
const COST_MULTIPLIER: f64 = 0.6;
const SCAN: i32 = 40;
const STEAM: &str = "arcforge:steam";
const HIGH: &str = "arcforge:high_pressure_steam";
const SUPER: &str = "arcforge:superheated_steam";
#[derive(Clone, Copy, Default)]
struct Shell {
    size: i32,
}
impl Shell {
    fn volume(self) -> u32 {
        (self.size.max(0) as u32).pow(3)
    }
    fn positions(self, center: [i32; 3], facing: &str) -> Vec<[i32; 3]> {
        let Some((out, axis)) = orientation(facing) else {
            return Vec::new();
        };
        let inward = [-out[0], -out[1], -out[2]];
        let mut out_positions = Vec::with_capacity(self.volume() as usize);
        for l in 0..self.size {
            for y in 0..self.size {
                for d in 0..self.size {
                    let l = l - (self.size - 1) / 2;
                    out_positions.push([
                        center[0] + axis[0] * l + inward[0] * d,
                        center[1] + y - (self.size - 1) / 2,
                        center[2] + axis[2] * l + inward[2] * d,
                    ]);
                }
            }
        }
        out_positions
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
pub(crate) fn tick(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let facing = host
        .block_property(pos, "facing")
        .unwrap_or_else(|| "north".into());
    let previous = get_string(&entity.nbt, "SuperheaterFacing").unwrap_or_else(|| "north".into());
    let old = saved_shell(&entity.nbt);
    let elapsed = get_i32(&entity.nbt, "SuperheaterScan") + 1;
    let scan =
        elapsed >= SCAN || previous != facing || get_i32(&entity.nbt, "SuperheaterInit") == 0;
    let shell = if scan {
        set_i32(&mut entity.nbt, "SuperheaterScan", 0);
        set_i32(&mut entity.nbt, "SuperheaterInit", 1);
        find_shell(host, pos, &facing)
    } else {
        set_i32(&mut entity.nbt, "SuperheaterScan", elapsed);
        old
    };
    if shell.map(|s| s.size) != old.map(|s| s.size) || previous != facing {
        if let Some(s) = old {
            set_formed(host, pos, &previous, s, false);
        }
        write_shell(&mut entity.nbt, shell);
        if let Some(s) = shell {
            set_formed(host, pos, &facing, s, true);
        }
        set_string(&mut entity.nbt, "SuperheaterFacing", &facing);
    }
    let Some(shell) = shell else {
        set_i32(&mut entity.nbt, "SuperheaterFlow", 0);
        set_i32(&mut entity.nbt, "SuperheaterUsed", 0);
        set_prop(host, pos, "lit", "false");
        return;
    };
    let capacity = scale(HEAT_PER_CUBE, shell).min(i32::MAX as u64);
    let stored = (get_i32(&entity.nbt, "HeatHU").max(0) as u64).min(capacity);
    if get_i32(&entity.nbt, "HeatHU").max(0) as u64 > stored {
        set_i32(&mut entity.nbt, "HeatHU", stored as i32);
    }
    let temperature = temperature(stored, capacity);
    let mode = get_i32(&entity.nbt, "SuperheaterPressure").clamp(0, 2);
    let input = crate::fluid::name(&entity.nbt, 0);
    let input_amount = crate::fluid::amount(&entity.nbt, 0);
    let out_name = crate::fluid::name(&entity.nbt, 1);
    let out_amount = crate::fluid::amount(&entity.nbt, 1);
    let (in_grade, in_hu) = match input.as_deref() {
        Some(STEAM) => (Some(0), 10.0),
        Some(HIGH) => (Some(1), 15.0),
        Some(SUPER) => (Some(2), 20.0),
        _ => (None, 0.0),
    };
    let target = if let Some(g) = in_grade {
        target_grade(g, mode, temperature)
    } else {
        None
    };
    let out_grade = target.unwrap_or_else(|| in_grade.unwrap_or(0));
    let output = grade_name(out_grade);
    let compatible = out_name.as_deref().is_none_or(|name| name == output);
    let active_tank_capacity = scale(TANK_PER_CUBE, shell).min(u32::MAX as u64) as u32;
    let space = if compatible {
        active_tank_capacity.saturating_sub(out_amount.min(active_tank_capacity))
    } else {
        0
    };
    let mut flow = input_amount
        .min(active_tank_capacity)
        .min(space)
        .min(scale(FLOW_PER_CUBE, shell).min(u32::MAX as u64) as u32);
    let mut used = 0u64;
    if let (Some(g), Some(t)) = (in_grade, target) {
        let target_c = if t == 1 { 500 } else { 900 };
        let min_stored = capacity
            .saturating_mul((target_c - 20) as u64)
            .div_ceil(u64::from(MAX_TEMP - 20));
        let available = stored
            .saturating_sub(min_stored)
            .min(scale(HEAT_RATE_PER_CUBE, shell));
        let cost = ((grade_hu(t) - grade_hu(g)) * COST_MULTIPLIER).max(0.0);
        if cost > 0.0 {
            flow = flow.min((available as f64 / cost).floor() as u32);
            used = ((flow as f64 * cost).ceil() as u64).min(available);
        }
    }
    if flow > 0 {
        let name = input.unwrap_or_default();
        let moved = crate::fluid::extract(&mut entity.nbt, 0, &name, flow);
        let written = crate::fluid::insert(
            &mut entity.nbt,
            1,
            output,
            moved,
            scale(TANK_PER_CUBE, shell).min(u32::MAX as u64) as u32,
        );
        if written < moved {
            crate::fluid::insert(
                &mut entity.nbt,
                0,
                &name,
                moved - written,
                scale(TANK_PER_CUBE, shell).min(u32::MAX as u64) as u32,
            );
        }
        let actual = moved.min(written);
        used = if target.is_some() {
            used.min(
                (actual as f64 * (grade_hu(out_grade) - in_hu) * COST_MULTIPLIER).ceil() as u64,
            )
        } else {
            0
        };
        set_i32(
            &mut entity.nbt,
            "HeatHU",
            stored.saturating_sub(used).min(i32::MAX as u64) as i32,
        );
        set_i32(&mut entity.nbt, "SuperheaterFlow", actual as i32);
        set_i32(
            &mut entity.nbt,
            "SuperheaterUsed",
            used.min(i32::MAX as u64) as i32,
        );
        set_i32(
            &mut entity.nbt,
            "SuperheaterTarget",
            if target.is_some() {
                out_grade as i32
            } else {
                -1
            },
        );
    } else {
        set_i32(&mut entity.nbt, "SuperheaterFlow", 0);
        set_i32(&mut entity.nbt, "SuperheaterUsed", 0);
        set_i32(
            &mut entity.nbt,
            "SuperheaterTarget",
            if target.is_some() {
                out_grade as i32
            } else {
                -1
            },
        );
    }
    set_prop(
        host,
        pos,
        "lit",
        if flow > 0 && target.is_some() {
            "true"
        } else {
            "false"
        },
    );
}
fn target_grade(input: i32, mode: i32, temp: u32) -> Option<i32> {
    match mode {
        1 if input == 0 && temp >= 500 => Some(1),
        2 if input < 2 && temp >= 900 => Some(2),
        0 => {
            if input < 2 && temp >= 900 {
                Some(2)
            } else if input < 1 && temp >= 500 {
                Some(1)
            } else {
                None
            }
        }
        _ => None,
    }
}
fn grade_name(g: i32) -> &'static str {
    match g {
        1 => HIGH,
        2 => SUPER,
        _ => STEAM,
    }
}
fn grade_hu(g: i32) -> f64 {
    match g {
        1 => 15.0,
        2 => 20.0,
        _ => 10.0,
    }
}
fn scale(per_cube: u64, shell: Shell) -> u64 {
    per_cube
        .saturating_mul(u64::from(shell.volume()))
        .saturating_add(13)
        / 27
}
fn temperature(stored: u64, capacity: u64) -> u32 {
    20 + (stored as u128 * (MAX_TEMP - 20) as u128 / capacity.max(1) as u128) as u32
}
fn find_shell(host: &dyn Host, controller: [i32; 3], facing: &str) -> Option<Shell> {
    let (out, axis) = orientation(facing)?;
    let inward = [-out[0], -out[1], -out[2]];
    for size in MIN..=MAX {
        let s = Shell { size };
        let c = (size - 1) / 2;
        let mut valid = true;
        'cells: for l in 0..size {
            for y in 0..size {
                for d in 0..size {
                    let along = l - c;
                    let p = [
                        controller[0] + axis[0] * along + inward[0] * d,
                        controller[1] + y - c,
                        controller[2] + axis[2] * along + inward[2] * d,
                    ];
                    let boundary = l == 0
                        || l == size - 1
                        || y == 0
                        || y == size - 1
                        || d == 0
                        || d == size - 1;
                    let control = l == c && y == c && d == 0;
                    let block = host.block(p);
                    if control {
                        if p != controller || block != CONTROLLER {
                            valid = false;
                            break 'cells;
                        }
                    } else if boundary {
                        if block != CASING {
                            valid = false;
                            break 'cells;
                        }
                    } else if !matches!(
                        block.as_str(),
                        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
                    ) {
                        valid = false;
                        break 'cells;
                    }
                }
            }
        }
        if valid {
            return Some(s);
        }
    }
    None
}
fn set_formed(host: &mut dyn Host, center: [i32; 3], facing: &str, s: Shell, formed: bool) {
    for p in s.positions(center, facing) {
        match host.block(p).as_str() {
            CONTROLLER => set_prop(host, p, "formed", if formed { "true" } else { "false" }),
            CASING => {
                set_prop(host, p, "part", if formed { "other" } else { "none" });
                set_prop(host, p, "facing", facing);
                set_prop(host, p, "box", if s.size == 3 { "false" } else { "true" });
            }
            _ => {}
        }
    }
}
pub(crate) fn unform(host: &mut dyn Host, pos: [i32; 3], nbt: &Compound) {
    if let Some(s) = saved_shell(nbt) {
        set_formed(host, pos, &saved_facing(nbt), s, false)
    }
}
pub(crate) fn face(host: &mut dyn Host, pos: [i32; 3], facing: &str) {
    let Some(e) = host.mod_block_entity(pos) else {
        return;
    };
    let current = host
        .block_property(pos, "facing")
        .unwrap_or_else(|| saved_facing(&e.nbt));
    if e.id != CONTROLLER || current == facing {
        return;
    }
    unform(host, pos, &e.nbt);
    host.set_block_property(pos, "facing", facing);
    host.change_mod_block_entity(pos, &mut |e| {
        if e.id == CONTROLLER {
            write_shell(&mut e.nbt, None);
            set_string(&mut e.nbt, "SuperheaterFacing", facing);
            set_i32(&mut e.nbt, "SuperheaterInit", 0);
            set_i32(&mut e.nbt, "SuperheaterScan", 0)
        }
    });
    host.block_entity_sync(pos);
}
pub(crate) fn cycle_pressure(host: &mut dyn Host, pos: [i32; 3]) {
    host.change_mod_block_entity(pos, &mut |e| {
        if e.id == CONTROLLER {
            let p = (get_i32(&e.nbt, "SuperheaterPressure") + 1) % 3;
            set_i32(&mut e.nbt, "SuperheaterPressure", p)
        }
    });
    host.block_entity_sync(pos)
}
pub(crate) fn notify_placed(host: &mut dyn Host, pos: [i32; 3]) {
    for x in -7..=7 {
        for y in -7..=7 {
            for z in -7..=7 {
                let p = [pos[0] + x, pos[1] + y, pos[2] + z];
                host.change_mod_block_entity(p, &mut |e| {
                    if e.id == CONTROLLER {
                        set_i32(&mut e.nbt, "SuperheaterScan", SCAN)
                    }
                });
            }
        }
    }
}
pub(crate) fn unform_near(host: &mut dyn Host, pos: [i32; 3]) {
    for x in -7..=7 {
        for y in -7..=7 {
            for z in -7..=7 {
                let p = [pos[0] + x, pos[1] + y, pos[2] + z];
                if let Some(e) = host.mod_block_entity(p) {
                    if e.id == CONTROLLER {
                        unform(host, p, &e.nbt);
                        host.change_mod_block_entity(p, &mut |e| write_shell(&mut e.nbt, None));
                    }
                }
            }
        }
    }
}
pub(crate) fn accepts(fluid: &str) -> bool {
    matches!(fluid, STEAM | HIGH | SUPER)
}
pub(crate) fn fluid_tanks() -> horizon_client::mod_api::ModFluidTanks {
    horizon_client::mod_api::ModFluidTanks {
        tanks: vec![MAX_TANK as i32, MAX_TANK as i32],
        faces: [vec![0, 1]; 6],
        inputs: vec![0],
        outputs: vec![1],
    }
}
pub(crate) fn status(entity: &ModBlockEntity) -> String {
    let s = saved_shell(&entity.nbt);
    let heat = get_i32(&entity.nbt, "HeatHU").max(0);
    let capacity = s.map_or(HEAT_PER_CUBE, |s| scale(HEAT_PER_CUBE, s));
    let mode = ["Auto", "High-Pressure", "Superheated"]
        [get_i32(&entity.nbt, "SuperheaterPressure").clamp(0, 2) as usize];
    format!(
        "Superheater {} {}x{}x{} | {mode} | {}C | Heat {heat}/{capacity} HU | In {} mB | Out {} mB | Flow {} mB/t | Used {} HU/t",
        if s.is_some() { "formed" } else { "unformed" },
        s.map_or(0, |s| s.size),
        s.map_or(0, |s| s.size),
        s.map_or(0, |s| s.size),
        temperature(heat as u64, capacity),
        crate::fluid::amount(&entity.nbt, 0),
        crate::fluid::amount(&entity.nbt, 1),
        get_i32(&entity.nbt, "SuperheaterFlow").max(0),
        get_i32(&entity.nbt, "SuperheaterUsed").max(0)
    )
}
pub(crate) fn accept_heat(
    host: &mut dyn Host,
    pos: [i32; 3],
    source: &mut crate::heat::HeatStore,
    rate: u64,
) -> u64 {
    let mut moved = 0;
    host.change_mod_block_entity(pos, &mut |e| {
        if e.id != CONTROLLER {
            return;
        }
        let Some(s) = saved_shell(&e.nbt) else { return };
        let cap = scale(HEAT_PER_CUBE, s);
        let mut sink =
            crate::heat::HeatStore::new(cap, MAX_TEMP, get_i32(&e.nbt, "HeatHU").max(0) as u64);
        moved = source.transfer_to(&mut sink, rate.min(scale(HEAT_RATE_PER_CUBE, s)));
        if moved > 0 {
            set_i32(
                &mut e.nbt,
                "HeatHU",
                sink.stored_hu().min(i32::MAX as u64) as i32,
            )
        }
    });
    if moved > 0 {
        host.block_entity_sync(pos)
    }
    moved
}
fn saved_shell(n: &Compound) -> Option<Shell> {
    let size = get_i32(n, "SuperheaterSize");
    if get_i32(n, "SuperheaterFormed") == 0 || !(MIN..=MAX).contains(&size) {
        None
    } else {
        Some(Shell { size })
    }
}
fn saved_facing(n: &Compound) -> String {
    get_string(n, "SuperheaterFacing").unwrap_or_else(|| "north".into())
}
fn write_shell(n: &mut Compound, s: Option<Shell>) {
    set_i32(n, "SuperheaterFormed", i32::from(s.is_some()));
    set_i32(n, "SuperheaterSize", s.map_or(0, |s| s.size))
}
fn set_prop(h: &mut dyn Host, p: [i32; 3], k: &str, v: &str) {
    if h.block_property(p, k).as_deref() != Some(v) {
        h.set_block_property(p, k, v)
    }
}
fn get_i32(n: &Compound, k: &str) -> i32 {
    match n.get(&k.into()) {
        Some(Tag::Int(v)) => *v,
        _ => 0,
    }
}
fn set_i32(n: &mut Compound, k: &str, v: i32) {
    n.insert(k.into(), Tag::Int(v));
}
fn get_string(n: &Compound, k: &str) -> Option<String> {
    match n.get(&k.into()) {
        Some(Tag::String(v)) => Some(v.clone()),
        _ => None,
    }
}
fn set_string(n: &mut Compound, k: &str, v: &str) {
    n.insert(k.into(), Tag::String(v.to_owned()));
}
