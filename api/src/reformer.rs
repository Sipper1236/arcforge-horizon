//! Arcforge Catalytic Reformer: heats Naphtha into Gasoline and Hydrogen.
use horizon_client::{
    containers::{ModBlockEntity, ModContainer},
    mod_api::{Compound, Host, ModFluidTanks, Tag, serde_json::Value},
};

pub(crate) const BLOCK: &str = "arcforge:catalytic_reformer";

const FEED_TANK: usize = 0;
const FIRST_PRODUCT_TANK: usize = 1;
const FEED_CAPACITY: u32 = 16_000;
const PRODUCT_CAPACITY: u32 = 8_000;
const HEAT_CAPACITY: u64 = 20_000;
const MAX_TEMPERATURE_C: u32 = 700;
const SPEED_SLOT: usize = 0;
const SPEED_UPGRADE: &str = "arcforge:speed_upgrade";

struct Product {
    fluid: String,
    amount: u32,
}

struct Recipe {
    input: String,
    amount: u32,
    products: Vec<Product>,
    ticks: u32,
    heat_per_tick: u64,
    min_temperature: u32,
}

fn recipe_from_json(json: &Value) -> Option<Recipe> {
    let input = json.get("input")?;
    let input_fluid = input.get("fluid")?.as_str()?.to_owned();
    let input_amount = input.get("amount")?.as_u64()?.clamp(1, u64::from(u32::MAX)) as u32;
    let products = json
        .get("fluids")?
        .as_array()?
        .iter()
        .map(|value| {
            Some(Product {
                fluid: value.get("id")?.as_str()?.to_owned(),
                amount: value.get("amount")?.as_u64()?.clamp(1, u64::from(u32::MAX)) as u32,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Recipe {
        input: input_fluid,
        amount: input_amount,
        products,
        ticks: json
            .get("time")
            .and_then(Value::as_u64)
            .unwrap_or(40)
            .clamp(1, i32::MAX as u64) as u32,
        heat_per_tick: json
            .get("heat_per_tick")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .min(i32::MAX as u64),
        min_temperature: json
            .get("min_temp")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .min(u64::from(u32::MAX)) as u32,
    })
}

fn find_recipe(host: &dyn Host, input: &str) -> Option<Recipe> {
    host.recipes_of_type("arcforge:reforming")
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
    get_i32(&entity.nbt, "ReformerProgress").max(0) as u32
}

fn total(recipe: &Recipe, entity: &ModBlockEntity) -> u32 {
    let upgrades = entity
        .items
        .get(SPEED_SLOT)
        .and_then(Option::as_ref)
        .filter(|item| item.item.as_ref() == SPEED_UPGRADE)
        .map_or(0, |item| item.count.min(8));
    crate::speed_upgrade_time(recipe.ticks, upgrades)
}

fn products_fit(entity: &ModBlockEntity, recipe: &Recipe) -> bool {
    recipe.products.len() <= 2
        && recipe.products.iter().enumerate().all(|(index, product)| {
            let tank = FIRST_PRODUCT_TANK + index;
            let name = crate::fluid::name(&entity.nbt, tank);
            let amount = crate::fluid::amount(&entity.nbt, tank);
            name.as_deref()
                .is_none_or(|existing| existing == product.fluid)
                && amount.saturating_add(product.amount) <= PRODUCT_CAPACITY
        })
}

fn set_status(entity: &mut ModBlockEntity, status: i32, rate: u32, total: u32) {
    set_i32(&mut entity.nbt, "ReformerStatus", status);
    set_i32(
        &mut entity.nbt,
        "ReformerRate",
        rate.min(i32::MAX as u32) as i32,
    );
    set_i32(
        &mut entity.nbt,
        "ReformerTotal",
        total.min(i32::MAX as u32) as i32,
    );
}

pub(crate) fn tick(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    entity.items.resize(1, None);
    let stored = (get_i32(&entity.nbt, "HeatHU").max(0) as u64).min(HEAT_CAPACITY);
    set_i32(&mut entity.nbt, "HeatHU", stored as i32);
    let heat = crate::heat::HeatStore::new(HEAT_CAPACITY, MAX_TEMPERATURE_C, stored);
    let input_name = crate::fluid::name(&entity.nbt, FEED_TANK);
    let recipe = input_name
        .as_deref()
        .and_then(|name| find_recipe(host, name));
    let Some(recipe) = recipe else {
        set_i32(&mut entity.nbt, "ReformerProgress", 0);
        set_i32(&mut entity.nbt, "ReformerTotal", 0);
        set_status(entity, if input_name.is_some() { 2 } else { 1 }, 0, 0);
        set_bool(host, pos, false);
        return;
    };
    if crate::fluid::amount(&entity.nbt, FEED_TANK) < recipe.amount {
        set_status(entity, 1, 0, total(&recipe, entity));
        set_bool(host, pos, false);
        return;
    }
    let duration = total(&recipe, entity).max(1);
    set_i32(
        &mut entity.nbt,
        "ReformerTotal",
        duration.min(i32::MAX as u32) as i32,
    );
    if !products_fit(entity, &recipe) {
        set_status(entity, 3, 0, duration);
        set_bool(host, pos, false);
        return;
    }

    let speed_upgrades = entity
        .items
        .get(SPEED_SLOT)
        .and_then(Option::as_ref)
        .filter(|item| item.item.as_ref() == SPEED_UPGRADE)
        .map_or(0, |item| item.count.min(8));
    let heat_per_tick = (recipe.heat_per_tick as f64
        * crate::speed_upgrade_multiplier(speed_upgrades))
    .ceil() as u64;
    if heat.temperature_c() < recipe.min_temperature || stored < heat_per_tick {
        set_status(entity, 4, 0, duration);
        set_bool(host, pos, false);
        return;
    }

    let current = progress(entity);
    set_i32(
        &mut entity.nbt,
        "HeatHU",
        stored.saturating_sub(heat_per_tick).min(i32::MAX as u64) as i32,
    );
    let next = current.saturating_add(1);
    set_i32(
        &mut entity.nbt,
        "ReformerProgress",
        next.min(i32::MAX as u32) as i32,
    );
    set_status(entity, 0, 1, duration);
    set_bool(host, pos, true);
    if next >= duration {
        crate::fluid::extract(&mut entity.nbt, FEED_TANK, &recipe.input, recipe.amount);
        for (index, product) in recipe.products.iter().enumerate() {
            crate::fluid::insert(
                &mut entity.nbt,
                FIRST_PRODUCT_TANK + index,
                &product.fluid,
                product.amount,
                PRODUCT_CAPACITY,
            );
        }
        set_i32(&mut entity.nbt, "ReformerProgress", 0);
        set_status(entity, 0, 0, duration);
        set_bool(host, pos, false);
    }
}

pub(crate) fn accepts(tank: usize, fluid: &str) -> bool {
    tank == FEED_TANK && fluid == "arcforge:naphtha"
}

pub(crate) fn fluid_tanks() -> ModFluidTanks {
    ModFluidTanks {
        tanks: vec![
            FEED_CAPACITY as i32,
            PRODUCT_CAPACITY as i32,
            PRODUCT_CAPACITY as i32,
        ],
        faces: [
            vec![0, 1, 2],
            vec![0, 1, 2],
            vec![0, 1, 2],
            vec![0, 1, 2],
            vec![0, 1, 2],
            vec![0, 1, 2],
        ],
        inputs: vec![FEED_TANK],
        outputs: vec![FIRST_PRODUCT_TANK, FIRST_PRODUCT_TANK + 1],
    }
}

pub(crate) fn container() -> ModContainer {
    ModContainer {
        slots: 1,
        faces: [
            vec![SPEED_SLOT],
            vec![SPEED_SLOT],
            vec![SPEED_SLOT],
            vec![SPEED_SLOT],
            vec![SPEED_SLOT],
            vec![SPEED_SLOT],
        ],
        inputs: vec![SPEED_SLOT],
        outputs: vec![],
        menu: Some("arcforge:machine_1_slot".into()),
    }
}

pub(crate) fn item_allowed(slot: usize, item: &str, count: u32) -> bool {
    slot == SPEED_SLOT && item == SPEED_UPGRADE && count <= 8
}

pub(crate) fn accept_heat(
    host: &mut dyn Host,
    pos: [i32; 3],
    source: &mut crate::heat::HeatStore,
    rate: u64,
) -> u64 {
    let Some(entity) = host.mod_block_entity(pos) else {
        return 0;
    };
    if entity.id != BLOCK {
        return 0;
    }
    let mut sink = crate::heat::HeatStore::new(
        HEAT_CAPACITY,
        MAX_TEMPERATURE_C,
        get_i32(&entity.nbt, "HeatHU").max(0) as u64,
    );
    let moved = source.transfer_to(&mut sink, rate);
    if moved > 0 {
        host.change_mod_block_entity(pos, &mut |current| {
            if current.id == BLOCK {
                set_i32(
                    &mut current.nbt,
                    "HeatHU",
                    sink.stored_hu().min(i32::MAX as u64) as i32,
                );
                set_i32(
                    &mut current.nbt,
                    "TemperatureC",
                    sink.temperature_c() as i32,
                );
            }
        });
        host.block_entity_sync(pos);
    }
    moved
}

pub(crate) fn face(host: &mut dyn Host, pos: [i32; 3], name: &str) {
    if !matches!(name, "north" | "east" | "south" | "west") {
        return;
    }
    host.set_block_property(pos, "facing", name);
}

pub(crate) fn status(entity: &ModBlockEntity) -> String {
    let stored = get_i32(&entity.nbt, "HeatHU").max(0) as u64;
    let temperature =
        crate::heat::HeatStore::new(HEAT_CAPACITY, MAX_TEMPERATURE_C, stored).temperature_c();
    let input = crate::fluid::name(&entity.nbt, FEED_TANK).unwrap_or_else(|| "empty".to_owned());
    let amount = crate::fluid::amount(&entity.nbt, FEED_TANK);
    let state = match get_i32(&entity.nbt, "ReformerStatus") {
        1 => "needs a full fluid recipe input",
        2 => "no matching reforming recipe",
        3 => "product tank full",
        4 => "heating or needs more HU",
        _ => "reforming",
    };
    format!(
        "Catalytic Reformer: {state} | {input} {amount}mB | {}/{} ticks | {temperature}°C | Heat {stored}/{HEAT_CAPACITY} HU",
        progress(entity),
        get_i32(&entity.nbt, "ReformerTotal").max(0)
    )
}
