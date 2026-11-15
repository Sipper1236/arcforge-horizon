//! Arcforge Industrial Water Intake alpha: a fixed-size pump house over a large water body.
use horizon_client::{
    containers::ModBlockEntity,
    mod_api::{Compound, Host, Tag},
};
use std::collections::{HashSet, VecDeque};
pub(crate) const CONTROLLER: &str = "arcforge:water_intake_controller";
pub(crate) const CASING: &str = "arcforge:water_intake_casing";
const WATER: &str = "minecraft:water";
const REQUIRED: usize = 400;
const RATE: u32 = 8_000;
const FE_PER_MB: f64 = 0.25;
const ENERGY_CAP: i32 = 200_000;
const TANK_CAP: u32 = 128_000;
const SCAN_TICKS: i32 = 100;
fn axes(f: &str) -> Option<([i32; 3], [i32; 3])> {
    match f {
        "north" => Some(([0, 0, -1], [1, 0, 0])),
        "east" => Some(([1, 0, 0], [0, 0, 1])),
        "south" => Some(([0, 0, 1], [-1, 0, 0])),
        "west" => Some(([-1, 0, 0], [0, 0, -1])),
        _ => None,
    }
}
fn positions(p: [i32; 3], f: &str) -> Option<Vec<[i32; 3]>> {
    let (out, side) = axes(f)?;
    let inward = [-out[0], 0, -out[2]];
    let mut v = Vec::with_capacity(27);
    for y in -1..=1 {
        for d in 0..=2 {
            for w in -1..=1 {
                v.push([
                    p[0] + inward[0] * d + side[0] * w,
                    p[1] + y,
                    p[2] + inward[2] * d + side[2] * w,
                ]);
            }
        }
    }
    Some(v)
}
fn intake(p: [i32; 3], f: &str) -> Option<[i32; 3]> {
    let (o, _) = axes(f)?;
    Some([p[0] - o[0], p[1] - 2, p[2] - o[2]])
}
fn get_i32(n: &Compound, k: &str) -> i32 {
    let Tag::String(k) = Tag::String(k.into()) else {
        unreachable!()
    };
    match n.get(&k) {
        Some(Tag::Int(v)) => *v,
        _ => 0,
    }
}
fn set_i32(n: &mut Compound, k: &str, v: i32) {
    let Tag::String(k) = Tag::String(k.into()) else {
        unreachable!()
    };
    n.insert(k, Tag::Int(v));
}
fn get_str(n: &Compound, k: &str) -> String {
    let Tag::String(k) = Tag::String(k.into()) else {
        unreachable!()
    };
    match n.get(&k) {
        Some(Tag::String(v)) => v.to_string_lossy(),
        _ => String::new(),
    }
}
fn set_str(n: &mut Compound, k: &str, v: &str) {
    let Tag::String(k) = Tag::String(k.into()) else {
        unreachable!()
    };
    let Tag::String(v) = Tag::String(v.into()) else {
        unreachable!()
    };
    n.insert(k, Tag::String(v));
}
fn mark(host: &mut dyn Host, ps: &[[i32; 3]], formed: bool, f: &str) {
    for p in ps {
        match host.block(*p).as_str() {
            CONTROLLER => {
                host.set_block_property(*p, "formed", if formed { "true" } else { "false" });
            }
            CASING => {
                host.set_block_property(*p, "facing", f);
                host.set_block_property(*p, "part", if formed { "other" } else { "none" });
            }
            _ => {}
        }
    }
}
fn find(host: &dyn Host, p: [i32; 3], f: &str) -> Option<Vec<[i32; 3]>> {
    let ps = positions(p, f)?;
    for q in &ps {
        let d = [q[0] - p[0], q[1] - p[1], q[2] - p[2]];
        let (out, side) = axes(f)?;
        let inward = [-out[0], 0, -out[2]];
        let depth = d[0] * inward[0] + d[2] * inward[2];
        let width = d[0] * side[0] + d[2] * side[2];
        let boundary = depth == 0 || depth == 2 || d[1].abs() == 1 || width.abs() == 1;
        let b = host.block(*q);
        if *q == p {
            if b != CONTROLLER {
                return None;
            }
        } else if boundary {
            if b != CASING {
                return None;
            }
        } else if !matches!(
            b.as_str(),
            "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
        ) {
            return None;
        }
    }
    Some(ps)
}
fn source(host: &dyn Host, p: [i32; 3]) -> bool {
    host.block(p) == WATER && host.block_property(p, "level").is_none_or(|v| v == "0")
}
fn water_count(host: &dyn Host, start: [i32; 3]) -> usize {
    if !source(host, start) {
        return 0;
    }
    let mut seen = HashSet::from([start]);
    let mut q = VecDeque::from([start]);
    while let Some(p) = q.pop_front() {
        for n in [
            [p[0] + 1, p[1], p[2]],
            [p[0] - 1, p[1], p[2]],
            [p[0], p[1] + 1, p[2]],
            [p[0], p[1] - 1, p[2]],
            [p[0], p[1], p[2] + 1],
            [p[0], p[1], p[2] - 1],
        ] {
            if seen.len() >= REQUIRED {
                return seen.len();
            }
            if !seen.contains(&n) && source(host, n) {
                seen.insert(n);
                q.push_back(n)
            }
        }
    }
    seen.len()
}
pub(crate) fn tick(host: &mut dyn Host, p: [i32; 3], e: &mut ModBlockEntity) {
    let f = host
        .block_property(p, "facing")
        .unwrap_or_else(|| "north".into());
    let prev = get_str(&e.nbt, "WaterIntakeFacing");
    let formed_before = get_i32(&e.nbt, "WaterIntakeFormed") != 0;
    let elapsed = get_i32(&e.nbt, "WaterIntakeScan").saturating_add(1);
    let scan = elapsed >= SCAN_TICKS || prev != f;
    let found = if scan { find(host, p, &f) } else { None };
    let formed = if scan { found.is_some() } else { formed_before };
    if scan {
        if formed_before {
            if let Some(old) = positions(p, &prev) {
                mark(host, &old, false, &prev)
            }
        }
        if let Some(ps) = &found {
            mark(host, ps, true, &f)
        }
        set_i32(&mut e.nbt, "WaterIntakeScan", 0);
        set_i32(&mut e.nbt, "WaterIntakeFormed", i32::from(formed));
        set_str(&mut e.nbt, "WaterIntakeFacing", &f);
    } else {
        set_i32(&mut e.nbt, "WaterIntakeScan", elapsed)
    }
    let count = if scan && formed {
        intake(p, &f).map_or(0, |w| water_count(host, w))
    } else if formed {
        get_i32(&e.nbt, "WaterIntakeWaterBlocks").max(0) as usize
    } else {
        0
    };
    set_i32(&mut e.nbt, "WaterIntakeWaterBlocks", count as i32);
    let tank = crate::fluid::amount(&e.nbt, 0);
    let energy = get_i32(&e.nbt, "energy").max(0);
    let amount = if formed && count >= REQUIRED && tank < TANK_CAP {
        RATE.min(TANK_CAP - tank)
            .min((f64::from(energy) / FE_PER_MB).floor() as u32)
    } else {
        0
    };
    if amount > 0 {
        crate::fluid::insert(&mut e.nbt, 0, WATER, amount, TANK_CAP);
        set_i32(
            &mut e.nbt,
            "energy",
            energy.saturating_sub((amount as f64 * FE_PER_MB).ceil() as i32),
        );
    }
    set_i32(&mut e.nbt, "WaterIntakeFlow", amount as i32);
    host.set_block_property(p, "lit", if amount > 0 { "true" } else { "false" });
}
pub(crate) fn unform(host: &mut dyn Host, p: [i32; 3], entity: &ModBlockEntity) {
    let facing = get_str(&entity.nbt, "WaterIntakeFacing");
    if get_i32(&entity.nbt, "WaterIntakeFormed") != 0 {
        if let Some(ps) = positions(p, &facing) {
            mark(host, &ps, false, &facing);
        }
    }
}
pub(crate) fn status(e: &ModBlockEntity) -> String {
    let formed = get_i32(&e.nbt, "WaterIntakeFormed") != 0;
    let water = get_i32(&e.nbt, "WaterIntakeWaterBlocks").max(0);
    let flow = get_i32(&e.nbt, "WaterIntakeFlow").max(0);
    let tank = crate::fluid::amount(&e.nbt, 0);
    let energy = get_i32(&e.nbt, "energy").max(0);
    format!(
        "Water Intake {} | source water {water}/{REQUIRED} | pumping {flow}/{RATE} mB/t | Tank {tank}/{TANK_CAP} mB | Energy {energy}/{ENERGY_CAP} FE",
        if formed { "formed" } else { "unformed" }
    )
}
pub(crate) fn accepts(_fluid: &str) -> bool {
    false
}
pub(crate) fn energy_buffer() -> horizon_client::mod_api::ModEnergyBuffer {
    horizon_client::mod_api::ModEnergyBuffer {
        capacity: ENERGY_CAP,
        max_insert: 1_000,
        max_extract: 0,
        insert_faces: [true; 6],
        extract_faces: [false; 6],
    }
}
pub(crate) fn fluid_tanks() -> horizon_client::mod_api::ModFluidTanks {
    horizon_client::mod_api::ModFluidTanks {
        tanks: vec![TANK_CAP as i32],
        faces: [vec![0], vec![0], vec![0], vec![0], vec![0], vec![0]],
        inputs: vec![],
        outputs: vec![0],
    }
}
