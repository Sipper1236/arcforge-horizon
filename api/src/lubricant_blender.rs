//! Arcforge Lubricant Blender: powers the conversion of Heavy Oil into Lubricant.
use horizon_client::{
    containers::{ModBlockEntity, ModContainer},
    mod_api::{Compound, Host, ModFluidTanks, Tag, serde_json::Value},
};

pub(crate) const BLOCK: &str = "arcforge:lubricant_blender";

const FEED_TANK: usize = 0;
const PRODUCT_TANK: usize = 1;
pub(crate) const FEED_CAPACITY: u32 = 16_000;
pub(crate) const PRODUCT_CAPACITY: u32 = 8_000;
pub(crate) const ENERGY_CAPACITY: u64 = 50_000;
const MAX_ENERGY_INPUT: u64 = 1_000;
const SPEED_SLOT: usize = 0;
const ENERGY_SLOT: usize = 1;
const SPEED_UPGRADE: &str = "arcforge:speed_upgrade";
const ENERGY_UPGRADE: &str = "arcforge:energy_upgrade";

struct Recipe {
    input: String,
    input_amount: u32,
    output: String,
    output_amount: u32,
    ticks: u32,
    energy_per_tick: u32,
}

fn recipe_from_json(json: &Value) -> Option<Recipe> {
    let input = json.get("input")?;
    let products = json.get("fluids")?.as_array()?;
    if products.len() != 1 {
        return None;
    }
    let product = products.first()?;
    Some(Recipe {
        input: input.get("fluid")?.as_str()?.to_owned(),
        input_amount: input
            .get("amount")?
            .as_u64()?
            .clamp(1, u64::from(u32::MAX)) as u32,
        output: product.get("id")?.as_str()?.to_owned(),
        output_amount: product
            .get("amount")?
            .as_u64()?
            .clamp(1, u64::from(u32::MAX)) as u32,
        ticks: json
            .get("time")
            .and_then(Value::as_u64)
            .unwrap_or(60)
            .clamp(1, u64::from(u32::MAX)) as u32,
        energy_per_tick: json
            .get("energy_per_tick")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .min(u64::from(u32::MAX)) as u32,
    })
}

fn find_recipe(host: &dyn Host, input: &str) -> Option<Recipe> {
    host.recipes_of_type("arcforge:lubricant_blending")
        .iter()
        .filter_map(|recipe| recipe_from_json(&recipe.json))
        .find(|recipe| recipe.input == input)
}

fn get_i32(nbt: &Compound, key: &str) -> i32 {
    let Tag::String(key) = Tag::String(key.into()) else {
        unreachable!()
    };
    match nbt.get(&key) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn set_i32(nbt: &mut Compound, key: &str, value: i32) {
    let Tag::String(key) = Tag::String(key.into()) else {
        unreachable!()
    };
    nbt.insert(key, Tag::Int(value));
}

fn set_bool(host: &mut dyn Host, pos: [i32; 3], value: bool) {
    let value = if value { "true" } else { "false" };
    if host.block_property(pos, "lit").as_deref() != Some(value) {
        host.set_block_property(pos, "lit", value);
    }
}

fn progress(entity: &ModBlockEntity) -> u32 {
    get_i32(&entity.nbt, "BlenderProgress").max(0) as u32
}

fn upgrades(entity: &ModBlockEntity, slot: usize, item: &str) -> u32 {
    entity
        .items
        .get(slot)
        .and_then(Option::as_ref)
        .filter(|stack| stack.item.as_ref() == item)
        .map_or(0, |stack| stack.count.min(8))
}

fn products_fit(entity: &ModBlockEntity, recipe: &Recipe) -> bool {
    let name = crate::fluid::name(&entity.nbt, PRODUCT_TANK);
    let amount = crate::fluid::amount(&entity.nbt, PRODUCT_TANK);
    name.as_deref()
        .is_none_or(|existing| existing == recipe.output)
        && amount.saturating_add(recipe.output_amount) <= PRODUCT_CAPACITY
}

fn sync_screen(entity: &mut ModBlockEntity, energy_rate: u32, total: u32, status: i32) {
    entity.data.resize(7, 0);
    entity.data[0] = get_i32(&entity.nbt, "energy").max(0);
    entity.data[1] = progress(entity).min(i32::MAX as u32) as i32;
    entity.data[2] = total.min(i32::MAX as u32) as i32;
    entity.data[3] = crate::fluid::amount(&entity.nbt, FEED_TANK).min(i32::MAX as u32) as i32;
    entity.data[4] = crate::fluid::amount(&entity.nbt, PRODUCT_TANK).min(i32::MAX as u32) as i32;
    entity.data[5] = energy_rate.min(i32::MAX as u32) as i32;
    entity.data[6] = status;
}

fn set_status(entity: &mut ModBlockEntity, status: i32, rate: u32, total: u32) {
    set_i32(&mut entity.nbt, "BlenderStatus", status);
    set_i32(&mut entity.nbt, "BlenderRate", rate.min(i32::MAX as u32) as i32);
    set_i32(&mut entity.nbt, "BlenderTotal", total.min(i32::MAX as u32) as i32);
    sync_screen(entity, rate, total, status);
}

fn total(recipe: &Recipe, entity: &ModBlockEntity) -> u32 {
    crate::speed_upgrade_time(recipe.ticks, upgrades(entity, SPEED_SLOT, SPEED_UPGRADE))
}

pub(crate) fn tick(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    entity.items.resize(2, None);
    let stored = (get_i32(&entity.nbt, "energy").max(0) as u64).min(ENERGY_CAPACITY);
    set_i32(&mut entity.nbt, "energy", stored as i32);

    let input_name = crate::fluid::name(&entity.nbt, FEED_TANK);
    let recipe = input_name
        .as_deref()
        .and_then(|name| find_recipe(host, name));
    let Some(recipe) = recipe else {
        set_i32(&mut entity.nbt, "BlenderProgress", 0);
        set_i32(&mut entity.nbt, "BlenderTotal", 0);
        let status = if input_name.is_some() { 2 } else { 1 };
        set_status(entity, status, 0, 0);
        set_bool(host, pos, false);
        return;
    };

    let duration = total(&recipe, entity).max(1);
    set_i32(&mut entity.nbt, "BlenderTotal", duration.min(i32::MAX as u32) as i32);
    if crate::fluid::amount(&entity.nbt, FEED_TANK) < recipe.input_amount {
        set_i32(&mut entity.nbt, "BlenderProgress", 0);
        set_status(entity, 1, 0, 0);
        set_bool(host, pos, false);
        return;
    }
    if !products_fit(entity, &recipe) {
        set_status(entity, 3, 0, duration);
        set_bool(host, pos, false);
        return;
    }

    let speed_upgrades = upgrades(entity, SPEED_SLOT, SPEED_UPGRADE);
    let energy_upgrades = upgrades(entity, ENERGY_SLOT, ENERGY_UPGRADE);
    let speed = crate::speed_upgrade_multiplier(speed_upgrades);
    let energy_cost = crate::scaled_upgrade_cost(
        u64::from(recipe.energy_per_tick),
        speed,
        energy_upgrades,
    );
    let mut energy = crate::energy::EnergyStore::new(ENERGY_CAPACITY, stored);
    if energy.stored_fe() < energy_cost {
        set_status(entity, 4, 0, duration);
        set_bool(host, pos, false);
        return;
    }

    energy.extract(energy_cost, energy_cost);
    set_i32(&mut entity.nbt, "energy", energy.stored_fe() as i32);
    let next = progress(entity).saturating_add(1);
    set_i32(
        &mut entity.nbt,
        "BlenderProgress",
        next.min(i32::MAX as u32) as i32,
    );
    set_status(
        entity,
        0,
        energy_cost.min(u64::from(u32::MAX)) as u32,
        duration,
    );
    set_bool(host, pos, true);

    if next >= duration {
        crate::fluid::extract(
            &mut entity.nbt,
            FEED_TANK,
            &recipe.input,
            recipe.input_amount,
        );
        crate::fluid::insert(
            &mut entity.nbt,
            PRODUCT_TANK,
            &recipe.output,
            recipe.output_amount,
            PRODUCT_CAPACITY,
        );
        set_i32(&mut entity.nbt, "BlenderProgress", 0);
        set_status(entity, 0, 0, duration);
        set_bool(host, pos, false);
    }
}

pub(crate) fn accepts(tank: usize, fluid: &str) -> bool {
    tank == FEED_TANK && fluid == "arcforge:heavy_oil"
}

pub(crate) fn fluid_tanks() -> ModFluidTanks {
    ModFluidTanks {
        tanks: vec![FEED_CAPACITY as i32, PRODUCT_CAPACITY as i32],
        faces: [
            vec![0, 1],
            vec![0, 1],
            vec![0, 1],
            vec![0, 1],
            vec![0, 1],
            vec![0, 1],
        ],
        inputs: vec![FEED_TANK],
        outputs: vec![PRODUCT_TANK],
    }
}

pub(crate) fn energy_buffer() -> horizon_client::mod_api::ModEnergyBuffer {
    horizon_client::mod_api::ModEnergyBuffer {
        capacity: ENERGY_CAPACITY as i32,
        max_insert: MAX_ENERGY_INPUT as i32,
        max_extract: 0,
        insert_faces: [true; 6],
        extract_faces: [false; 6],
    }
}

pub(crate) fn container() -> ModContainer {
    ModContainer {
        slots: 2,
        faces: [
            vec![SPEED_SLOT, ENERGY_SLOT],
            vec![SPEED_SLOT, ENERGY_SLOT],
            vec![SPEED_SLOT, ENERGY_SLOT],
            vec![SPEED_SLOT, ENERGY_SLOT],
            vec![SPEED_SLOT, ENERGY_SLOT],
            vec![SPEED_SLOT, ENERGY_SLOT],
        ],
        inputs: vec![SPEED_SLOT, ENERGY_SLOT],
        outputs: vec![],
        menu: Some("arcforge:machine_2_slot".into()),
    }
}

pub(crate) fn item_allowed(slot: usize, item: &str, count: u32) -> bool {
    count <= 8
        && ((slot == SPEED_SLOT && item == SPEED_UPGRADE)
            || (slot == ENERGY_SLOT && item == ENERGY_UPGRADE))
}

pub(crate) fn face(host: &mut dyn Host, pos: [i32; 3], name: &str) {
    if matches!(name, "north" | "east" | "south" | "west") {
        host.set_block_property(pos, "facing", name);
    }
}

pub(crate) fn status(entity: &ModBlockEntity) -> String {
    let feed = crate::fluid::amount(&entity.nbt, FEED_TANK);
    let product = crate::fluid::amount(&entity.nbt, PRODUCT_TANK);
    let energy = get_i32(&entity.nbt, "energy").max(0);
    let state = match get_i32(&entity.nbt, "BlenderStatus") {
        1 => "needs Heavy Oil",
        2 => "no matching recipe",
        3 => "Lubricant tank full",
        4 => "needs FE",
        _ => "blending",
    };
    format!(
        "Lubricant Blender: {state} | Heavy Oil {feed}/{FEED_CAPACITY}mB | Lubricant {product}/{PRODUCT_CAPACITY}mB | {}/{} ticks | FE {energy}/{ENERGY_CAPACITY}",
        progress(entity),
        get_i32(&entity.nbt, "BlenderTotal").max(0)
    )
}
