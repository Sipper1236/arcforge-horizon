const REGION_SIZE: i32 = 8;
const CHANCE: f64 = 0.30;
const SOUR_CHANCE: f64 = 0.30;
const GAS_CAP_CHANCE: f64 = 0.35;
const MIN_Y: i32 = -48;
const MAX_Y: i32 = 8;
const SALT: u64 = 0x6F696C5F64657073;
const TRAIT_SALT: u64 = 0x737570686F726761;
const DIM_SALT: u64 = 0x9E3779B97F4A7C15;
const X_SALT: u64 = 0xC2B2AE3D27D4EB4F;
const Z_SALT: u64 = 0x165667B19E3779F9;
#[derive(Clone, Copy)]
struct Kind {
    name: &'static str,
    min_buckets: u32,
    max_buckets: u32,
    min_radius: i32,
    max_radius: i32,
    weight: u32,
}
const KINDS: [Kind; 4] = [
    Kind {
        name: "Seep",
        min_buckets: 1_000,
        max_buckets: 3_000,
        min_radius: 1,
        max_radius: 2,
        weight: 45,
    },
    Kind {
        name: "Pocket",
        min_buckets: 4_000,
        max_buckets: 10_000,
        min_radius: 2,
        max_radius: 3,
        weight: 30,
    },
    Kind {
        name: "Field",
        min_buckets: 12_000,
        max_buckets: 30_000,
        min_radius: 2,
        max_radius: 3,
        weight: 18,
    },
    Kind {
        name: "Giant Oil Field",
        min_buckets: 40_000,
        max_buckets: 100_000,
        min_radius: 3,
        max_radius: 4,
        weight: 7,
    },
];
pub(crate) fn survey(host: &dyn horizon_client::mod_api::Host, pos: [i32; 3]) -> String {
    if host.dimension() != "minecraft:overworld" {
        return "No oil deposits are configured in this dimension.".to_owned();
    }
    let Some(seed) = host.world_seed() else {
        return "Seismic Survey unavailable: world seed not available.".to_owned();
    };
    let chunk_x = pos[0].div_euclid(16);
    let chunk_z = pos[2].div_euclid(16);
    let region_x = chunk_x.div_euclid(REGION_SIZE);
    let region_z = chunk_z.div_euclid(REGION_SIZE);
    let dimension_hash = java_hash(host.dimension()) as u64;
    let mut state = mix(seed as u64 ^ SALT ^ dimension_hash.wrapping_mul(DIM_SALT));
    state = mix(state ^ (region_x as i64 as u64).wrapping_mul(X_SALT));
    state = mix(state ^ (region_z as i64 as u64).wrapping_mul(Z_SALT));
    if next_unit(&mut state) >= CHANCE {
        return "Seismic survey: no oil deposit in this region.".to_owned();
    }
    let kind = pick_kind(next_unit(&mut state));
    let share = next_unit(&mut state);
    let buckets =
        kind.min_buckets as f64 + (share * (kind.max_buckets - kind.min_buckets) as f64).round();
    let radius = (kind.min_radius
        + (next_unit(&mut state) * (kind.max_radius - kind.min_radius) as f64) as i32)
        .min(REGION_SIZE / 2 - 1)
        .max(1);
    let span = (REGION_SIZE - 2 * radius).max(1);
    let center_x = region_x * REGION_SIZE + radius + (next_unit(&mut state) * span as f64) as i32;
    let center_z = region_z * REGION_SIZE + radius + (next_unit(&mut state) * span as f64) as i32;
    if (chunk_x - center_x).pow(2) + (chunk_z - center_z).pow(2) > radius * radius + radius {
        return "Seismic survey: no oil deposit beneath this chunk.".to_owned();
    }
    let y = MIN_Y + (unit(mix(state)) * (MAX_Y - MIN_Y + 1) as f64) as i32;
    let mut traits = mix(seed as u64 ^ TRAIT_SALT ^ dimension_hash.wrapping_mul(DIM_SALT));
    traits = mix(traits ^ (region_x as i64 as u64).wrapping_mul(X_SALT));
    traits = mix(traits ^ (region_z as i64 as u64).wrapping_mul(Z_SALT));
    let sour = next_unit(&mut traits) < SOUR_CHANCE;
    let gas_cap = unit(mix(traits)) < GAS_CAP_CHANCE;
    format!(
        "Seismic survey: {} | {} | about {buckets:.0} buckets | depth {y} | {}",
        kind.name,
        if sour { "sour crude" } else { "sweet crude" },
        if gas_cap { "gas cap" } else { "no gas cap" }
    )
}
fn pick_kind(roll: f64) -> Kind {
    let total: u32 = KINDS.iter().map(|kind| kind.weight).sum();
    let mut at = roll * f64::from(total);
    for kind in KINDS {
        at -= f64::from(kind.weight);
        if at < 0.0 {
            return kind;
        }
    }
    KINDS[3]
}
fn java_hash(value: &str) -> i32 {
    value.encode_utf16().fold(0_i32, |hash, unit| {
        hash.wrapping_mul(31).wrapping_add(i32::from(unit))
    })
}
fn mix(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D049BB133111EB);
    value ^ (value >> 31)
}
fn next_unit(state: &mut u64) -> f64 {
    *state = mix(*state);
    unit(*state)
}
fn unit(value: u64) -> f64 {
    (value >> 11) as f64 * (1.0 / 9_007_199_254_740_992.0)
}
