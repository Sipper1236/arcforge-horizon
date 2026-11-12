//! Arcforge heat cells: tiered HU storage, heat loss, and bounded transfer.

use horizon_client::{
    containers::ModBlockEntity,
    mod_api::{Host, ItemStack, Tag},
};
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
    time::Instant,
};

use crate::heat::HeatStore;

pub(crate) const WROUGHT: &str = "arcforge:wrought_heat_cell";
pub(crate) const TEMPERED: &str = "arcforge:tempered_heat_cell";
pub(crate) const HARDENED: &str = "arcforge:hardened_heat_cell";
pub(crate) const ARCFORGED: &str = "arcforge:arcforged_heat_cell";

const DIRECTIONS: [[i32; 3]; 6] = [
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, -1],
    [0, 0, 1],
    [-1, 0, 0],
    [1, 0, 0],
];
const CHARGE_LIFETIME_SECS: u64 = 10;

type ChargeKey = (String, [i32; 3]);

#[derive(Default)]
struct PendingCharges {
    entries: HashMap<ChargeKey, (String, i32, Instant)>,
}

static PENDING_CHARGES: LazyLock<Mutex<PendingCharges>> =
    LazyLock::new(|| Mutex::new(PendingCharges::default()));

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Spec {
    pub capacity: u64,
    pub rate: u64,
    pub max_temperature: u32,
    pub leak_percent_per_minute: f64,
}

pub(crate) fn spec(id: &str) -> Option<Spec> {
    let spec = match id {
        WROUGHT => Spec {
            capacity: 50_000,
            rate: 50,
            max_temperature: 1_100,
            leak_percent_per_minute: 10.0,
        },
        TEMPERED => Spec {
            capacity: 200_000,
            rate: 200,
            max_temperature: 1_100,
            leak_percent_per_minute: 4.0,
        },
        HARDENED => Spec {
            capacity: 800_000,
            rate: 800,
            max_temperature: 1_300,
            leak_percent_per_minute: 1.5,
        },
        ARCFORGED => Spec {
            capacity: 3_200_000,
            rate: 3_200,
            max_temperature: 1_400,
            leak_percent_per_minute: 0.5,
        },
        _ => return None,
    };
    Some(spec)
}

pub(crate) fn ids() -> [&'static str; 4] {
    [WROUGHT, TEMPERED, HARDENED, ARCFORGED]
}

pub(crate) fn item_heat_amount(stack: &ItemStack) -> u64 {
    let Some(spec) = spec(&stack.item) else {
        return 0;
    };
    let Some(Tag::Compound(data)) = stack
        .extra
        .as_deref()
        .and_then(|extra| extra.other("minecraft:custom_data"))
    else {
        return 0;
    };
    match data.get(&"horizon_heat".into()) {
        Some(Tag::Int(amount)) => ((*amount).max(0) as u64).min(spec.capacity),
        _ => 0,
    }
}

pub(crate) fn set_item_heat(stack: &mut ItemStack, amount: i32) {
    if amount <= 0 {
        return;
    }
    let mut custom_data = match stack
        .extra
        .as_deref()
        .and_then(|extra| extra.other("minecraft:custom_data"))
    {
        Some(Tag::Compound(data)) => data.clone(),
        _ => horizon_client::mod_api::Compound::default(),
    };
    custom_data.insert("horizon_heat".into(), Tag::Int(amount));
    stack
        .extra
        .get_or_insert_with(Box::default)
        .set_other("minecraft:custom_data", Some(Tag::Compound(custom_data)));
}

pub(crate) fn restore_from_item(host: &mut dyn Host, pos: [i32; 3], stack: &ItemStack) {
    let stored = item_heat_amount(stack);
    if stored == 0 {
        return;
    }
    let item = stack.item.to_string();
    host.change_mod_block_entity(pos, &mut |entity| {
        if entity.id == item
            && let Some(spec) = spec(&entity.id)
        {
            let mut heat = heat_store(entity, spec);
            heat.add_hu(stored);
            write_heat(entity, heat, spec.rate);
        }
    });
    host.block_entity_sync(pos);
}

pub(crate) fn cache_charge(dimension: &str, pos: [i32; 3], entity: &ModBlockEntity) {
    let Some(spec) = spec(&entity.id) else {
        return;
    };
    let stored = int(&entity.nbt, "HeatHU").max(0).min(spec.capacity as i32);
    let mut pending = PENDING_CHARGES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    pending
        .entries
        .retain(|_, (_, _, created)| created.elapsed().as_secs() < CHARGE_LIFETIME_SECS);
    pending.entries.insert(
        (dimension.to_owned(), pos),
        (entity.id.clone(), stored, Instant::now()),
    );
}

pub(crate) fn take_charge(dimension: &str, pos: [i32; 3]) -> Option<(String, i32)> {
    let mut pending = PENDING_CHARGES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (item, heat, created) = pending.entries.remove(&(dimension.to_owned(), pos))?;
    (created.elapsed().as_secs() < CHARGE_LIFETIME_SECS).then_some((item, heat))
}

fn int(nbt: &horizon_client::mod_api::Compound, key: &str) -> i32 {
    match nbt.get(&key.into()) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn set_int(nbt: &mut horizon_client::mod_api::Compound, key: &str, value: i32) {
    nbt.insert(key.into(), Tag::Int(value));
}

fn double(nbt: &horizon_client::mod_api::Compound, key: &str) -> f64 {
    match nbt.get(&key.into()) {
        Some(Tag::Double(bits)) => f64::from_bits(*bits),
        _ => 0.0,
    }
}

fn set_double(nbt: &mut horizon_client::mod_api::Compound, key: &str, value: f64) {
    nbt.insert(key.into(), Tag::Double(value.to_bits()));
}

fn game_tick(host: &dyn Host) -> i32 {
    host.level(host.dimension())
        .and_then(|level| level.game_time(host))
        .unwrap_or_default() as i32
}

fn heat_store(entity: &ModBlockEntity, spec: Spec) -> HeatStore {
    HeatStore::new(
        spec.capacity,
        spec.max_temperature,
        int(&entity.nbt, "HeatHU").max(0) as u64,
    )
}

fn write_heat(entity: &mut ModBlockEntity, heat: HeatStore, rate: u64) {
    set_int(
        &mut entity.nbt,
        "HeatHU",
        heat.stored_hu().min(i32::MAX as u64) as i32,
    );
    set_int(
        &mut entity.nbt,
        "TemperatureC",
        heat.temperature_c().min(i32::MAX as u32) as i32,
    );
    entity.data.resize(2, 0);
    entity.data[0] = heat.stored_hu().min(i32::MAX as u64) as i32;
    entity.data[1] = rate.min(i32::MAX as u64) as i32;
}

/// Receives heat from a machine or conduit. A block can accept at most its tier's rate per tick,
/// across all neighbors. Temperature equality and capacity limits are enforced by `HeatStore`.
pub(crate) fn accept_heat(
    host: &mut dyn Host,
    pos: [i32; 3],
    source: &mut HeatStore,
    requested: u64,
) -> u64 {
    let Some(target) = host.mod_block_entity(pos) else {
        return 0;
    };
    let Some(spec) = spec(&target.id) else {
        return 0;
    };
    let tick = game_tick(host);
    let mut moved = 0;
    host.change_mod_block_entity(pos, &mut |entity| {
        if entity.id != target.id {
            return;
        }
        let stored_tick = int(&entity.nbt, "HeatCellInputTick");
        let used = if stored_tick == tick {
            int(&entity.nbt, "HeatCellInputUsed").max(0) as u64
        } else {
            0
        };
        let budget = requested.min(spec.rate.saturating_sub(used));
        if budget == 0 {
            return;
        }
        let mut sink = heat_store(entity, spec);
        moved = source.transfer_to(&mut sink, budget);
        if moved > 0 {
            write_heat(entity, sink, spec.rate);
            set_int(&mut entity.nbt, "HeatCellInputTick", tick);
            set_int(
                &mut entity.nbt,
                "HeatCellInputUsed",
                used.saturating_add(moved).min(i32::MAX as u64) as i32,
            );
        }
    });
    if moved > 0 {
        host.block_entity_sync(pos);
    }
    moved
}

/// Leaks heat every tick and pushes it to adjacent colder consumers, up to the tier's total rate.
/// The UI, capsule slots, and insulation upgrade slot are deferred; the cell remains usable as an
/// automatically connected heat buffer in the alpha.
pub(crate) fn tick(
    host: &mut dyn Host,
    pos: [i32; 3],
    entity: &mut ModBlockEntity,
    mut transfer_to_neighbor: impl FnMut(&mut dyn Host, [i32; 3], [i32; 3], &mut HeatStore, u64) -> u64,
) {
    let Some(spec) = spec(&entity.id) else {
        return;
    };
    let tick = game_tick(host);
    let mut heat = heat_store(entity, spec);
    if tick.rem_euclid(20) == 0 && heat.stored_hu() > 0 {
        let per_second = heat.stored_hu() as f64 * spec.leak_percent_per_minute / 6_000.0;
        let total = per_second + double(&entity.nbt, "HeatCellLeakRemainder");
        let lost = total.floor().min(i32::MAX as f64) as u64;
        set_double(
            &mut entity.nbt,
            "HeatCellLeakRemainder",
            if heat.stored_hu() <= lost {
                0.0
            } else {
                total - lost as f64
            },
        );
        heat.remove_hu(lost);
    }

    let mut output_used = 0_u64;
    for offset in DIRECTIONS {
        let budget = spec.rate.saturating_sub(output_used);
        if budget == 0 || heat.stored_hu() == 0 {
            break;
        }
        let neighbor = [pos[0] + offset[0], pos[1] + offset[1], pos[2] + offset[2]];
        let before = heat.stored_hu();
        transfer_to_neighbor(host, pos, neighbor, &mut heat, budget);
        output_used = output_used.saturating_add(before - heat.stored_hu());
    }

    write_heat(entity, heat, spec.rate);
    let level = match heat_store(entity, spec).temperature_c() {
        1_000.. => 4,
        850.. => 3,
        650.. => 2,
        300.. => 1,
        _ => 0,
    };
    let level = level.to_string();
    if host.block_property(pos, "heat").as_deref() != Some(level.as_str()) {
        host.set_block_property(pos, "heat", &level);
    }
}
