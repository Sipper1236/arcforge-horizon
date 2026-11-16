use horizon_client::mod_api::Host;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex},
};
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
const SAVE_NAME: &str = "arcforge-oil-deposits.tsv";
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
#[derive(Clone, Copy, Debug)]
pub(crate) struct Deposit {
    pub(crate) region_x: i32,
    pub(crate) region_z: i32,
    pub(crate) kind: usize,
    pub(crate) amount: u64,
    pub(crate) depth: i32,
    pub(crate) sour: bool,
    pub(crate) gas_cap: bool,
    center_x: i32,
    center_z: i32,
    radius: i32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct DepositKey {
    seed: i64,
    region_x: i32,
    region_z: i32,
}
#[derive(Default)]
struct WorldStore {
    amounts: HashMap<DepositKey, u64>,
}
static STORES: LazyLock<Mutex<HashMap<PathBuf, WorldStore>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
fn key(seed: i64, deposit: Deposit) -> DepositKey {
    DepositKey {
        seed,
        region_x: deposit.region_x,
        region_z: deposit.region_z,
    }
}
fn store_file(world_dir: &Path) -> PathBuf {
    world_dir.join("data").join(SAVE_NAME)
}
fn load_store(path: &Path) -> Result<WorldStore, &'static str> {
    let file = store_file(path);
    let text = match fs::read_to_string(file) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(WorldStore::default());
        }
        Err(_) => return Err("Unable to read saved oil deposits"),
    };
    let mut store = WorldStore::default();
    for line in text.lines() {
        let mut fields = line.split('\t');
        let (Some(seed), Some(x), Some(z), Some(amount)) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        let (Ok(seed), Ok(region_x), Ok(region_z), Ok(amount)) =
            (seed.parse(), x.parse(), z.parse(), amount.parse())
        else {
            continue;
        };
        store.amounts.insert(
            DepositKey {
                seed,
                region_x,
                region_z,
            },
            amount,
        );
    }
    Ok(store)
}
fn save_store(path: &Path, store: &WorldStore) -> Result<(), &'static str> {
    let file = store_file(path);
    let parent = file.parent().ok_or("Invalid oil deposit save path")?;
    fs::create_dir_all(parent).map_err(|_| "Unable to create oil deposit save directory")?;
    let mut rows: Vec<_> = store.amounts.iter().collect();
    rows.sort_by_key(|(key, _)| (key.seed, key.region_x, key.region_z));
    let mut text = String::new();
    for (key, amount) in rows {
        use std::fmt::Write as _;
        writeln!(
            &mut text,
            "{}\t{}\t{}\t{}",
            key.seed, key.region_x, key.region_z, amount
        )
        .map_err(|_| "Unable to serialize saved oil deposits")?;
    }
    let temporary = file.with_extension("tsv.tmp");
    fs::write(&temporary, text).map_err(|_| "Unable to write saved oil deposits")?;
    fs::rename(&temporary, &file).map_err(|_| "Unable to replace saved oil deposits")
}
fn remaining_for(host: &dyn Host, deposit: Deposit) -> Result<u64, &'static str> {
    let Some(path) = host.world_dir() else {
        return Err("World save path unavailable");
    };
    let Some(seed) = host.world_seed() else {
        return Err("World seed unavailable");
    };
    let mut stores = STORES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !stores.contains_key(&path) {
        stores.insert(path.clone(), load_store(&path)?);
    }
    Ok(stores
        .get(&path)
        .and_then(|store| store.amounts.get(&key(seed, deposit)).copied())
        .unwrap_or(deposit.amount))
}
pub(crate) fn remaining(
    host: &dyn Host,
    pos: [i32; 3],
) -> Result<Option<(Deposit, u64)>, &'static str> {
    let Some(deposit) = at(host, pos)? else {
        return Ok(None);
    };
    Ok(Some((deposit, remaining_for(host, deposit)?)))
}
pub(crate) fn take(
    host: &dyn Host,
    pos: [i32; 3],
    requested: u64,
) -> Result<Option<(Deposit, u64, u64)>, &'static str> {
    let Some(deposit) = at(host, pos)? else {
        return Ok(None);
    };
    let Some(path) = host.world_dir() else {
        return Err("World save path unavailable");
    };
    let Some(seed) = host.world_seed() else {
        return Err("World seed unavailable");
    };
    let mut stores = STORES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !stores.contains_key(&path) {
        stores.insert(path.clone(), load_store(&path)?);
    }
    let store = stores.get_mut(&path).expect("oil store was loaded");
    let key = key(seed, deposit);
    let left = store.amounts.get(&key).copied().unwrap_or(deposit.amount);
    let taken = left.min(requested);
    let next = left - taken;
    store.amounts.insert(key, next);
    if let Err(error) = save_store(&path, store) {
        store.amounts.insert(key, left);
        return Err(error);
    }
    Ok(Some((deposit, next, taken)))
}
fn at(host: &dyn Host, pos: [i32; 3]) -> Result<Option<Deposit>, &'static str> {
    if host.dimension() != "minecraft:overworld" {
        return Ok(None);
    }
    let Some(seed) = host.world_seed() else {
        return Err("World seed unavailable");
    };
    let chunk_x = pos[0].div_euclid(16);
    let chunk_z = pos[2].div_euclid(16);
    let region_x = chunk_x.div_euclid(REGION_SIZE);
    let region_z = chunk_z.div_euclid(REGION_SIZE);
    let deposit = generate(seed, host.dimension(), region_x, region_z);
    Ok(deposit.filter(|deposit| covers(*deposit, chunk_x, chunk_z)))
}
fn generate(seed: i64, dimension: &str, region_x: i32, region_z: i32) -> Option<Deposit> {
    let dimension_hash = java_hash(dimension) as u64;
    let mut state = mix(seed as u64 ^ SALT ^ dimension_hash.wrapping_mul(DIM_SALT));
    state = mix(state ^ (region_x as i64 as u64).wrapping_mul(X_SALT));
    state = mix(state ^ (region_z as i64 as u64).wrapping_mul(Z_SALT));
    if next_unit(&mut state) >= CHANCE {
        return None;
    }
    let (kind_index, kind) = pick_kind(next_unit(&mut state));
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
    let depth = MIN_Y + (unit(mix(state)) * (MAX_Y - MIN_Y + 1) as f64) as i32;
    let mut traits = mix(seed as u64 ^ TRAIT_SALT ^ dimension_hash.wrapping_mul(DIM_SALT));
    traits = mix(traits ^ (region_x as i64 as u64).wrapping_mul(X_SALT));
    traits = mix(traits ^ (region_z as i64 as u64).wrapping_mul(Z_SALT));
    let sour = next_unit(&mut traits) < SOUR_CHANCE;
    let gas_cap = unit(mix(traits)) < GAS_CAP_CHANCE;
    Some(Deposit {
        region_x,
        region_z,
        kind: kind_index,
        amount: buckets as u64 * 1_000,
        depth,
        sour,
        gas_cap,
        center_x,
        center_z,
        radius,
    })
}
fn covers(deposit: Deposit, chunk_x: i32, chunk_z: i32) -> bool {
    let dx = chunk_x - deposit.center_x;
    let dz = chunk_z - deposit.center_z;
    dx * dx + dz * dz <= deposit.radius * deposit.radius + deposit.radius
}
pub(crate) fn survey(host: &dyn Host, pos: [i32; 3]) -> String {
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
    let Some(deposit) = generate(seed, host.dimension(), region_x, region_z) else {
        return "Seismic survey: no oil deposit in this region.".to_owned();
    };
    if !covers(deposit, chunk_x, chunk_z) {
        return "Seismic survey: no oil deposit beneath this chunk.".to_owned();
    }
    let remaining = remaining_for(host, deposit).unwrap_or(deposit.amount);
    format!(
        "Seismic survey: {} | {} | {} of {} buckets remaining | depth {} | {}",
        KINDS[deposit.kind].name,
        if deposit.sour {
            "sour crude"
        } else {
            "sweet crude"
        },
        remaining.div_ceil(1_000),
        deposit.amount.div_ceil(1_000),
        deposit.depth,
        if deposit.gas_cap {
            "gas cap"
        } else {
            "no gas cap"
        }
    )
}
fn pick_kind(roll: f64) -> (usize, Kind) {
    let total: u32 = KINDS.iter().map(|kind| kind.weight).sum();
    let mut at = roll * f64::from(total);
    for (index, kind) in KINDS.into_iter().enumerate() {
        at -= f64::from(kind.weight);
        if at < 0.0 {
            return (index, kind);
        }
    }
    (KINDS.len() - 1, KINDS[KINDS.len() - 1])
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
