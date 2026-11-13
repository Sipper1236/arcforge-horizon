//! Gas-only pressurized cylinders and portable gas cartridges.

use horizon_client::{
    containers::ModBlockEntity,
    mod_api::{ItemStack, Tag},
};
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
    time::Instant,
};

pub(crate) const WROUGHT: &str = "arcforge:wrought_pressurized_cylinder";
pub(crate) const TEMPERED: &str = "arcforge:tempered_pressurized_cylinder";
pub(crate) const HARDENED: &str = "arcforge:hardened_pressurized_cylinder";
pub(crate) const ARCFORGED: &str = "arcforge:arcforged_pressurized_cylinder";

pub(crate) const WROUGHT_CARTRIDGE: &str = "arcforge:wrought_gas_cartridge";
pub(crate) const TEMPERED_CARTRIDGE: &str = "arcforge:tempered_gas_cartridge";
pub(crate) const HARDENED_CARTRIDGE: &str = "arcforge:hardened_gas_cartridge";
pub(crate) const ARCFORGED_CARTRIDGE: &str = "arcforge:arcforged_gas_cartridge";

const CHARGE_LIFETIME_SECS: u64 = 10;
type ChargeKey = (String, [i32; 3]);

#[derive(Default)]
struct PendingCharges {
    entries: HashMap<ChargeKey, (String, Option<String>, u32, Instant)>,
}

static PENDING_CHARGES: LazyLock<Mutex<PendingCharges>> =
    LazyLock::new(|| Mutex::new(PendingCharges::default()));

pub(crate) fn cylinder_capacity(id: &str) -> Option<u32> {
    match id {
        WROUGHT => Some(64_000),
        TEMPERED => Some(256_000),
        HARDENED => Some(1_024_000),
        ARCFORGED => Some(4_096_000),
        _ => None,
    }
}

pub(crate) fn cartridge_spec(item: &str) -> Option<(u32, u32)> {
    match item {
        WROUGHT_CARTRIDGE => Some((32_000, 2_000)),
        TEMPERED_CARTRIDGE => Some((128_000, 8_000)),
        HARDENED_CARTRIDGE => Some((512_000, 32_000)),
        ARCFORGED_CARTRIDGE => Some((2_048_000, 128_000)),
        _ => None,
    }
}

pub(crate) fn cylinder_ids() -> [&'static str; 4] {
    [WROUGHT, TEMPERED, HARDENED, ARCFORGED]
}

pub(crate) fn cartridge_ids() -> [&'static str; 4] {
    [
        WROUGHT_CARTRIDGE,
        TEMPERED_CARTRIDGE,
        HARDENED_CARTRIDGE,
        ARCFORGED_CARTRIDGE,
    ]
}

pub(crate) fn cartridge_accepts_stack(stack: &ItemStack) -> bool {
    cartridge_spec(&stack.item).is_some()
        && crate::item_fluid(stack)
            .is_none_or(|(fluid, _)| crate::fluid_conduit::is_arcforge_gas(&fluid))
}

pub(crate) fn item_fill_level(stack: &ItemStack) -> Option<String> {
    let capacity = cylinder_capacity(&stack.item)?;
    let amount = item_fluid(stack).map_or(0, |(_, amount)| amount.min(capacity));
    Some(((amount as u64 * 5) / capacity as u64).min(4).to_string())
}

pub(crate) fn accepts_gas(block: &str, fluid: &str) -> bool {
    cylinder_capacity(block).is_some() && crate::fluid_conduit::is_arcforge_gas(fluid)
}

pub(crate) fn tick(dimension: &str, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let Some(capacity) = cylinder_capacity(&entity.id) else {
        return;
    };
    entity.items.resize(2, None);

    let mut contents = crate::fluid::name(&entity.nbt, 0);
    let mut amount = crate::fluid::amount(&entity.nbt, 0).min(capacity);

    if let Some(mut cartridge) = entity.items[0].clone()
        && let Some((cartridge_capacity, rate)) = cartridge_spec(&cartridge.item)
        && let Some((gas, held)) = crate::item_fluid(&cartridge)
        && crate::fluid_conduit::is_arcforge_gas(&gas)
        && contents.as_deref().is_none_or(|stored| stored == gas)
    {
        let held = held.min(cartridge_capacity);
        let moved = held.min(rate).min(capacity.saturating_sub(amount));
        if moved > 0 {
            let inserted = crate::fluid::insert(&mut entity.nbt, 0, &gas, moved, capacity);
            amount += inserted;
            contents = Some(gas.clone());
            cartridge = crate::item_with_fluid(&cartridge, &gas, held - inserted);
            entity.items[0] = Some(cartridge);
        } else if crate::item_fluid(&cartridge)
            .is_some_and(|(_, amount)| amount > cartridge_capacity)
        {
            entity.items[0] = Some(crate::item_with_fluid(&cartridge, &gas, cartridge_capacity));
        }
    }

    if let Some(mut cartridge) = entity.items[1].clone()
        && let Some((cartridge_capacity, rate)) = cartridge_spec(&cartridge.item)
        && (crate::item_fluid(&cartridge).is_none_or(|(gas, _)| {
            crate::fluid_conduit::is_arcforge_gas(&gas) && contents.as_deref() == Some(&gas)
        }))
        && let Some(gas) = contents.clone()
        && crate::fluid_conduit::is_arcforge_gas(&gas)
    {
        let held = crate::item_fluid(&cartridge).map_or(0, |(_, held)| held);
        let moved = amount
            .min(rate)
            .min(cartridge_capacity.saturating_sub(held));
        if moved > 0 {
            let extracted = crate::fluid::extract(&mut entity.nbt, 0, &gas, moved);
            cartridge = crate::item_with_fluid(&cartridge, &gas, held + extracted);
            entity.items[1] = Some(cartridge);
        }
    }

    cache_charge(dimension, pos, entity);
}

pub(crate) fn cache_charge(dimension: &str, pos: [i32; 3], entity: &ModBlockEntity) {
    let Some(capacity) = cylinder_capacity(&entity.id) else {
        return;
    };
    let amount = crate::fluid::amount(&entity.nbt, 0).min(capacity);
    let fluid = crate::fluid::name(&entity.nbt, 0);
    let mut pending = PENDING_CHARGES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    pending
        .entries
        .retain(|_, (_, _, _, created)| created.elapsed().as_secs() < CHARGE_LIFETIME_SECS);
    pending.entries.insert(
        (dimension.to_owned(), pos),
        (entity.id.clone(), fluid, amount, Instant::now()),
    );
}

pub(crate) fn take_charge(dimension: &str, pos: [i32; 3]) -> Option<(String, Option<String>, u32)> {
    let mut pending = PENDING_CHARGES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (item, fluid, amount, created) = pending.entries.remove(&(dimension.to_owned(), pos))?;
    (created.elapsed().as_secs() < CHARGE_LIFETIME_SECS).then_some((item, fluid, amount))
}

pub(crate) fn item_fluid(stack: &ItemStack) -> Option<(String, u32)> {
    let Tag::Compound(data) = stack.extra.as_deref()?.other("minecraft:custom_data")? else {
        return None;
    };
    let Tag::Compound(tank) = data.get(&"horizon_fluid".into())? else {
        return None;
    };
    let fluid = match tank.get(&"Fluid".into())? {
        Tag::String(value) => value.to_string_lossy(),
        _ => return None,
    };
    let amount = match tank.get(&"Amount".into())? {
        Tag::Int(value) => (*value).max(0) as u32,
        _ => return None,
    };
    (amount > 0).then_some((fluid, amount))
}

pub(crate) fn restore_from_item(
    host: &mut dyn horizon_client::mod_api::Host,
    pos: [i32; 3],
    stack: &ItemStack,
) {
    let Some(capacity) = cylinder_capacity(&stack.item) else {
        return;
    };
    let Some((gas, amount)) = item_fluid(stack) else {
        return;
    };
    if !crate::fluid_conduit::is_arcforge_gas(&gas) {
        return;
    }
    let item = stack.item.to_string();
    host.change_mod_block_entity(pos, &mut |entity| {
        if entity.id == item {
            crate::fluid::insert(&mut entity.nbt, 0, &gas, amount.min(capacity), capacity);
        }
    });
    host.block_entity_sync(pos);
}
