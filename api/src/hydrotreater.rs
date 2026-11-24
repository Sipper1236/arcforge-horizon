//! Arcforge Hydrotreater: uses Hydrogen, heat, and FE to clean sour refinery fuels.
use horizon_client::{
    containers::{ModBlockEntity, ModContainer},
    mod_api::{Compound, Host, ModFluidTanks, Tag, serde_json::Value},
};

pub(crate) const BLOCK: &str = "arcforge:hydrotreater";

const FEED_TANK: usize = 0;
const HYDROGEN_TANK: usize = 1;
const FIRST_PRODUCT_TANK: usize = 2;
const FEED_CAPACITY: u32 = 16_000;
const HYDROGEN_CAPACITY: u32 = 16_000;
const PRODUCT_CAPACITY: u32 = 8_000;
const ENERGY_CAPACITY: u64 = 50_000;
const MAX_ENERGY_INPUT: u64 = 1_000;
const HEAT_CAPACITY: u64 = 20_000;
const MAX_TEMPERATURE_C: u32 = 700;
const SPEED_SLOT: usize = 0;
const ENERGY_SLOT: usize = 1;
const SPEED_UPGRADE: &str = "arcforge:speed_upgrade";
const ENERGY_UPGRADE: &str = "arcforge:energy_upgrade";

struct Product {
    fluid: String,
    amount: u32,
}

struct Recipe {
    input: String,
    input_amount: u32,
    second_input: String,
    second_input_amount: u32,
    products: Vec<Product>,
    ticks: u32,
    heat_per_tick: u32,
    min_temperature: u32,
    energy_per_tick: u32,
}

fn recipe_from_json(json: &Value) -> Option<Recipe> {
    let input = json.get("input")?;
    let second_input = json.get("second_input")?;
    let fluids = json.get("fluids")?.as_array()?;
    if fluids.len() != 2 {
        return None;
    }
    let parse_amount = |value: &Value| {
        value
            .as_u64()
            .map(|amount| amount.clamp(1, u64::from(u32::MAX)) as u32)
    };
    let products = fluids
        .iter()
        .map(|product| {
            Some(Product {
                fluid: product.get("id")?.as_str()?.to_owned(),
                amount: parse_amount(product.get("amount")?)?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Recipe {
        input: input.get("fluid")?.as_str()?.to_owned(),
        input_amount: parse_amount(input.get("amount")?)?,
        second_input: second_input.get("fluid")?.as_str()?.to_owned(),
        second_input_amount: parse_amount(second_input.get("amount")?)?,
        products,
        ticks: json
            .get("time")
            .and_then(Value::as_u64)
            .unwrap_or(40)
            .clamp(1, u64::from(u32::MAX)) as u32,
        heat_per_tick: json
            .get("heat_per_tick")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .min(u64::from(u32::MAX)) as u32,
        min_temperature: json
            .get("min_temp")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .min(u64::from(u32::MAX)) as u32,
        energy_per_tick: json
            .get("energy_per_tick")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .min(u64::from(u32::MAX)) as u32,
    })
}

fn find_recipe(host: &dyn Host, input: &str) -> Option<Recipe> {
    host.recipes_of_type("arcforge:hydrotreating")
        .iter()
        .filter_map(|recipe| recipe_from_json(&recipe.json))
        .find(|recipe| recipe.input == input)
}

fn get_i32(nbt: &Compound, name: &str) -> i32 {
    let Tag::String(key) = Tag::String(name.into()) else {
        unreachable!()
    };
    match nbt.get(&key) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn set_i32(nbt: &mut Compound, name: &str, value: i32) {
    let Tag::String(key) = Tag::String(name.into()) else {
        unreachable!()
    };
    nbt.insert(key, Tag::Int(value));
}

fn set_lit(host: &mut dyn Host, pos: [i32; 3], lit: bool) {
    let value = if lit { "true" } else { "false" };
    if host.block_property(pos, "lit").as_deref() != Some(value) {
        host.set_block_property(pos, "lit", value);
    }
}

fn progress(entity: &ModBlockEntity) -> u32 {
    get_i32(&entity.nbt, "HydrotreaterProgress").max(0) as u32
}

fn upgrades(entity: &ModBlockEntity, slot: usize, item: &str) -> u32 {
    entity
        .items
        .get(slot)
        .and_then(Option::as_ref)
        .filter(|stack| stack.item.as_ref() == item)
        .map_or(0, |stack| stack.count.min(8))
}

fn duration(recipe: &Recipe, entity: &ModBlockEntity) -> u32 {
    crate::speed_upgrade_time(recipe.ticks, upgrades(entity, SPEED_SLOT, SPEED_UPGRADE))
}

fn products_fit(entity: &ModBlockEntity, recipe: &Recipe) -> bool {
    recipe.products.iter().enumerate().all(|(index, product)| {
        let tank = FIRST_PRODUCT_TANK + index;
        let existing = crate::fluid::name(&entity.nbt, tank);
        let amount = crate::fluid::amount(&entity.nbt, tank);
        existing
            .as_deref()
            .is_none_or(|name| name == product.fluid)
            && amount.saturating_add(product.amount) <= PRODUCT_CAPACITY
    })
}

fn sync_screen(
    entity: &mut ModBlockEntity,
    energy_rate: u32,
    heat_rate: u32,
    total: u32,
    status: i32,
) {
    entity.data.resize(12, 0);
    entity.data[0] = get_i32(&entity.nbt, "energy").max(0);
    entity.data[1] = get_i32(&entity.nbt, "HeatHU").max(0);
    entity.data[2] = crate::heat::HeatStore::new(
        HEAT_CAPACITY,
        MAX_TEMPERATURE_C,
        get_i32(&entity.nbt, "HeatHU").max(0) as u64,
    )
    .temperature_c() as i32;
    entity.data[3] = progress(entity).min(i32::MAX as u32) as i32;
    entity.data[4] = total.min(i32::MAX as u32) as i32;
    entity.data[5] = crate::fluid::amount(&entity.nbt, FEED_TANK).min(i32::MAX as u32) as i32;
    entity.data[6] = crate::fluid::amount(&entity.nbt, HYDROGEN_TANK).min(i32::MAX as u32) as i32;
    entity.data[7] = crate::fluid::amount(&entity.nbt, FIRST_PRODUCT_TANK).min(i32::MAX as u32) as i32;
    entity.data[8] = crate::fluid::amount(&entity.nbt, FIRST_PRODUCT_TANK + 1)
        .min(i32::MAX as u32) as i32;
    entity.data[9] = energy_rate.min(i32::MAX as u32) as i32;
    entity.data[10] = heat_rate.min(i32::MAX as u32) as i32;
    entity.data[11] = status;
}

fn set_status(
    entity: &mut ModBlockEntity,
    status: i32,
    energy_rate: u32,
    heat_rate: u32,
    total: u32,
) {
    set_i32(&mut entity.nbt, "HydrotreaterStatus", status);
    set_i32(
        &mut entity.nbt,
        "HydrotreaterEnergyRate",
        energy_rate.min(i32::MAX as u32) as i32,
    );
    set_i32(
        &mut entity.nbt,
        "HydrotreaterHeatRate",
        heat_rate.min(i32::MAX as u32) as i32,
    );
    set_i32(
        &mut entity.nbt,
        "HydrotreaterTotal",
        total.min(i32::MAX as u32) as i32,
    );
    sync_screen(entity, energy_rate, heat_rate, total, status);
}

pub(crate) fn tick(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    entity.items.resize(2, None);
    let stored_heat = (get_i32(&entity.nbt, "HeatHU").max(0) as u64).min(HEAT_CAPACITY);
    let stored_energy = (get_i32(&entity.nbt, "energy").max(0) as u64).min(ENERGY_CAPACITY);
    set_i32(&mut entity.nbt, "HeatHU", stored_heat as i32);
    set_i32(&mut entity.nbt, "energy", stored_energy as i32);
    let heat = crate::heat::HeatStore::new(HEAT_CAPACITY, MAX_TEMPERATURE_C, stored_heat);

    let feed_name = crate::fluid::name(&entity.nbt, FEED_TANK);
    let recipe = feed_name
        .as_deref()
        .and_then(|name| find_recipe(host, name));
    let Some(recipe) = recipe else {
        set_i32(&mut entity.nbt, "HydrotreaterProgress", 0);
        set_i32(&mut entity.nbt, "HydrotreaterTotal", 0);
        set_status(
            entity,
            if feed_name.is_some() { 2 } else { 1 },
            0,
            0,
            0,
        );
        set_lit(host, pos, false);
        return;
    };

    let total = duration(&recipe, entity).max(1);
    if crate::fluid::amount(&entity.nbt, FEED_TANK) < recipe.input_amount {
        set_i32(&mut entity.nbt, "HydrotreaterProgress", 0);
        set_status(entity, 1, 0, 0, 0);
        set_lit(host, pos, false);
        return;
    }
    if crate::fluid::name(&entity.nbt, HYDROGEN_TANK).as_deref()
        != Some(recipe.second_input.as_str())
        || crate::fluid::amount(&entity.nbt, HYDROGEN_TANK) < recipe.second_input_amount
    {
        set_status(entity, 3, 0, 0, total);
        set_lit(host, pos, false);
        return;
    }
    if !products_fit(entity, &recipe) {
        set_status(entity, 4, 0, 0, total);
        set_lit(host, pos, false);
        return;
    }

    let speed_upgrades = upgrades(entity, SPEED_SLOT, SPEED_UPGRADE);
    let energy_upgrades = upgrades(entity, ENERGY_SLOT, ENERGY_UPGRADE);
    let speed = crate::speed_upgrade_multiplier(speed_upgrades);
    let heat_cost = (f64::from(recipe.heat_per_tick) * speed).ceil() as u64;
    if heat.temperature_c() < recipe.min_temperature || stored_heat < heat_cost {
        set_status(entity, 5, 0, 0, total);
        set_lit(host, pos, false);
        return;
    }

    let energy_cost = crate::scaled_upgrade_cost(
        u64::from(recipe.energy_per_tick),
        speed,
        energy_upgrades,
    );
    let mut energy = crate::energy::EnergyStore::new(ENERGY_CAPACITY, stored_energy);
    if energy.stored_fe() < energy_cost {
        set_status(entity, 6, 0, 0, total);
        set_lit(host, pos, false);
        return;
    }

    energy.extract(energy_cost, energy_cost);
    set_i32(&mut entity.nbt, "energy", energy.stored_fe() as i32);
    set_i32(
        &mut entity.nbt,
        "HeatHU",
        stored_heat.saturating_sub(heat_cost) as i32,
    );
    let next = progress(entity).saturating_add(1);
    set_i32(
        &mut entity.nbt,
        "HydrotreaterProgress",
        next.min(i32::MAX as u32) as i32,
    );
    set_status(
        entity,
        0,
        energy_cost.min(u64::from(u32::MAX)) as u32,
        heat_cost.min(u64::from(u32::MAX)) as u32,
        total,
    );
    set_lit(host, pos, true);

    if next >= total {
        crate::fluid::extract(
            &mut entity.nbt,
            FEED_TANK,
            &recipe.input,
            recipe.input_amount,
        );
        crate::fluid::extract(
            &mut entity.nbt,
            HYDROGEN_TANK,
            &recipe.second_input,
            recipe.second_input_amount,
        );
        for (index, product) in recipe.products.iter().enumerate() {
            crate::fluid::insert(
                &mut entity.nbt,
                FIRST_PRODUCT_TANK + index,
                &product.fluid,
                product.amount,
                PRODUCT_CAPACITY,
            );
        }
        set_i32(&mut entity.nbt, "HydrotreaterProgress", 0);
        set_status(entity, 0, 0, 0, total);
        set_lit(host, pos, false);
    }
}

pub(crate) fn accepts(tank: usize, fluid: &str) -> bool {
    match tank {
        FEED_TANK => matches!(fluid, "arcforge:sour_kerosene" | "arcforge:sour_diesel"),
        HYDROGEN_TANK => fluid == "arcforge:hydrogen",
        _ => false,
    }
}

pub(crate) fn fluid_tanks() -> ModFluidTanks {
    ModFluidTanks {
        tanks: vec![
            FEED_CAPACITY as i32,
            HYDROGEN_CAPACITY as i32,
            PRODUCT_CAPACITY as i32,
            PRODUCT_CAPACITY as i32,
        ],
        faces: [
            vec![0, 1, 2, 3],
            vec![0, 1, 2, 3],
            vec![0, 1, 2, 3],
            vec![0, 1, 2, 3],
            vec![0, 1, 2, 3],
            vec![0, 1, 2, 3],
        ],
        inputs: vec![FEED_TANK, HYDROGEN_TANK],
        outputs: vec![FIRST_PRODUCT_TANK, FIRST_PRODUCT_TANK + 1],
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
    if matches!(name, "north" | "east" | "south" | "west") {
        host.set_block_property(pos, "facing", name);
    }
}

pub(crate) fn status(entity: &ModBlockEntity) -> String {
    let stored_heat = get_i32(&entity.nbt, "HeatHU").max(0) as u64;
    let temperature = crate::heat::HeatStore::new(
        HEAT_CAPACITY,
        MAX_TEMPERATURE_C,
        stored_heat,
    )
    .temperature_c();
    let energy = get_i32(&entity.nbt, "energy").max(0);
    let state = match get_i32(&entity.nbt, "HydrotreaterStatus") {
        1 => "needs sour fuel",
        2 => "no matching recipe",
        3 => "needs Hydrogen",
        4 => "product tank full",
        5 => "heating or needs HU",
        6 => "needs FE",
        _ => "hydrotreating",
    };
    format!(
        "Hydrotreater: {state} | Sour fuel {}/{}mB | Hydrogen {}/{}mB | Products {}/{}mB, {}/{}mB | {}/{} ticks | {temperature}°C | Heat {stored_heat}/{HEAT_CAPACITY} HU | FE {energy}/{ENERGY_CAPACITY}",
        crate::fluid::amount(&entity.nbt, FEED_TANK),
        FEED_CAPACITY,
        crate::fluid::amount(&entity.nbt, HYDROGEN_TANK),
        HYDROGEN_CAPACITY,
        crate::fluid::amount(&entity.nbt, FIRST_PRODUCT_TANK),
        PRODUCT_CAPACITY,
        crate::fluid::amount(&entity.nbt, FIRST_PRODUCT_TANK + 1),
        PRODUCT_CAPACITY,
        progress(entity),
        get_i32(&entity.nbt, "HydrotreaterTotal").max(0),
    )
}

pub(crate) fn draw_screen(
    host: &dyn Host,
    event: &mut horizon_client::mod_api::ScreenDrawEvent<'_>,
) {
    let [ox, oy] = event.origin;
    let data = event.data;
    let batch = &mut *event.batch;
    macro_rules! text {
        ($label:expr, $x:expr, $y:expr, $color:expr $(,)?) => {
            host.text(batch, $label, [ox + $x, oy + $y], $color, true)
        };
    }
    macro_rules! fill {
        ($rect:expr, $color:expr) => {
            host.fill(
                batch,
                [ox + ($rect)[0], oy + ($rect)[1], ($rect)[2], ($rect)[3]],
                $color,
            )
        };
    }

    text!("Hydrotreater", 8.0, 6.0, 0xFFFFFFFF);
    text!("Speed", 38.0, 17.0, 0xFFBBBBBB);
    text!("Energy", 122.0, 17.0, 0xFFBBBBBB);
    text!("Sour fuel", 8.0, 44.0, 0xFFBBBBBB);
    text!("Hydrogen", 90.0, 44.0, 0xFFBBBBBB);
    text!("Clean fuel", 8.0, 77.0, 0xFFBBBBBB);
    text!("H2S", 90.0, 77.0, 0xFFBBBBBB);

    let energy = data.first().copied().unwrap_or_default().max(0);
    let heat = data.get(1).copied().unwrap_or_default().max(0);
    let temperature = data.get(2).copied().unwrap_or_default().max(0);
    let progress = data.get(3).copied().unwrap_or_default().max(0);
    let duration = data.get(4).copied().unwrap_or_default().max(0);
    let feed = data.get(5).copied().unwrap_or_default().max(0);
    let hydrogen = data.get(6).copied().unwrap_or_default().max(0);
    let clean_fuel = data.get(7).copied().unwrap_or_default().max(0);
    let hydrogen_sulfide = data.get(8).copied().unwrap_or_default().max(0);
    let energy_rate = data.get(9).copied().unwrap_or_default().max(0);
    let heat_rate = data.get(10).copied().unwrap_or_default().max(0);
    let state = match data.get(11).copied().unwrap_or_default() {
        1 => "Waiting for sour fuel",
        2 => "No matching recipe",
        3 => "Waiting for Hydrogen",
        4 => "Product tank full",
        5 => "Heating or needs HU",
        6 => "Waiting for FE",
        _ if duration > 0 => "Hydrotreating",
        _ => "Idle",
    };

    text!(
        &format!("{feed}/{} mB", FEED_CAPACITY),
        8.0,
        58.0,
        0xFFE0E0E0,
    );
    text!(
        &format!("{hydrogen}/{} mB", HYDROGEN_CAPACITY),
        90.0,
        58.0,
        0xFFE0E0E0,
    );
    text!(
        &format!("{clean_fuel}/{} mB", PRODUCT_CAPACITY),
        8.0,
        91.0,
        0xFFE0E0E0,
    );
    text!(
        &format!("{hydrogen_sulfide}/{} mB", PRODUCT_CAPACITY),
        90.0,
        91.0,
        0xFFE0E0E0,
    );
    text!(
        &format!("FE {energy}/{ENERGY_CAPACITY}"),
        8.0,
        107.0,
        0xFFE0E0E0,
    );
    text!(
        &format!("Heat {heat}/{HEAT_CAPACITY} HU"),
        8.0,
        119.0,
        0xFFE0E0E0,
    );
    text!(
        &format!("{temperature}°C | Draw {energy_rate} FE/t"),
        8.0,
        131.0,
        0xFFE0E0E0,
    );
    text!(
        &format!("Heat draw {heat_rate} HU/t"),
        8.0,
        143.0,
        0xFFE0E0E0,
    );
    text!(state, 8.0, 155.0, 0xFFBBBBBB);
    if duration > 0 {
        text!(
            &format!("Progress {progress}/{duration} t"),
            8.0,
            167.0,
            0xFFE0E0E0,
        );
        fill!([8.0, 178.0, 160.0, 6.0], 0xFF383838);
        let fraction = (progress as f32 / duration as f32).clamp(0.0, 1.0);
        fill!([8.0, 178.0, 160.0 * fraction, 6.0], 0xFF62B64F);
    }
    text!("Inventory", 8.0, 188.0, 0xFFFFFFFF);
}
