//! Arcforge Sulfur Recovery Unit: converts Hydrogen Sulfide and Oxygen into Sulfur and Steam.
use horizon_client::{
    containers::{ModBlockEntity, ModContainer},
    mod_api::{Compound, Host, ModFluidTanks, Tag, serde_json::Value},
};

pub(crate) const BLOCK: &str = "arcforge:sulfur_recovery_unit";

const FEED_TANK: usize = 0;
const OXYGEN_TANK: usize = 1;
const STEAM_TANK: usize = 2;
const FEED_CAPACITY: u32 = 16_000;
const OXYGEN_CAPACITY: u32 = 16_000;
const STEAM_CAPACITY: u32 = 8_000;
const HEAT_CAPACITY: u64 = 20_000;
const MAX_TEMPERATURE_C: u32 = 700;
const SPEED_SLOT: usize = 0;
const SULFUR_SLOT: usize = 1;
const SPEED_UPGRADE: &str = "arcforge:speed_upgrade";
const SULFUR_DUST: &str = "arcforge:sulfur_dust";

struct Recipe {
    input: String,
    input_amount: u32,
    second_input: String,
    second_input_amount: u32,
    steam: String,
    steam_amount: u32,
    sulfur: String,
    sulfur_amount: u32,
    ticks: u32,
    heat_per_tick: u32,
    min_temperature: u32,
}

fn recipe_from_json(json: &Value) -> Option<Recipe> {
    let parse_amount = |value: &Value| {
        value
            .as_u64()
            .map(|amount| amount.clamp(1, u64::from(u32::MAX)) as u32)
    };
    let input = json.get("input")?;
    let second_input = json.get("second_input")?;
    let fluids = json.get("fluids")?.as_array()?;
    if fluids.len() != 1 {
        return None;
    }
    let fluid = fluids.first()?;
    let item = json.get("item")?;
    Some(Recipe {
        input: input.get("fluid")?.as_str()?.to_owned(),
        input_amount: parse_amount(input.get("amount")?)?,
        second_input: second_input.get("fluid")?.as_str()?.to_owned(),
        second_input_amount: parse_amount(second_input.get("amount")?)?,
        steam: fluid.get("id")?.as_str()?.to_owned(),
        steam_amount: parse_amount(fluid.get("amount")?)?,
        sulfur: item.get("id")?.as_str()?.to_owned(),
        sulfur_amount: item
            .get("count")
            .and_then(parse_amount)
            .unwrap_or(1),
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
    })
}

fn find_recipe(host: &dyn Host, input: &str) -> Option<Recipe> {
    host.recipes_of_type("arcforge:sulfur_recovery")
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
    get_i32(&entity.nbt, "SulfurRecoveryProgress").max(0) as u32
}

fn speed_upgrades(entity: &ModBlockEntity) -> u32 {
    entity
        .items
        .get(SPEED_SLOT)
        .and_then(Option::as_ref)
        .filter(|stack| stack.item.as_ref() == SPEED_UPGRADE)
        .map_or(0, |stack| stack.count.min(8))
}

fn duration(recipe: &Recipe, entity: &ModBlockEntity) -> u32 {
    crate::speed_upgrade_time(recipe.ticks, speed_upgrades(entity))
}

fn products_fit(entity: &ModBlockEntity, recipe: &Recipe) -> bool {
    let name = crate::fluid::name(&entity.nbt, STEAM_TANK);
    let amount = crate::fluid::amount(&entity.nbt, STEAM_TANK);
    name.as_deref()
        .is_none_or(|existing| existing == recipe.steam)
        && amount.saturating_add(recipe.steam_amount) <= STEAM_CAPACITY
        && crate::can_add(
            entity.items.get(SULFUR_SLOT).and_then(Option::as_ref),
            &recipe.sulfur,
            recipe.sulfur_amount,
        )
}

fn sync_screen(entity: &mut ModBlockEntity, heat_rate: u32, total: u32, status: i32) {
    entity.data.resize(10, 0);
    let stored_heat = get_i32(&entity.nbt, "HeatHU").max(0);
    entity.data[0] = stored_heat;
    entity.data[1] = crate::heat::HeatStore::new(
        HEAT_CAPACITY,
        MAX_TEMPERATURE_C,
        stored_heat as u64,
    )
    .temperature_c() as i32;
    entity.data[2] = progress(entity).min(i32::MAX as u32) as i32;
    entity.data[3] = total.min(i32::MAX as u32) as i32;
    entity.data[4] = crate::fluid::amount(&entity.nbt, FEED_TANK).min(i32::MAX as u32) as i32;
    entity.data[5] = crate::fluid::amount(&entity.nbt, OXYGEN_TANK).min(i32::MAX as u32) as i32;
    entity.data[6] = crate::fluid::amount(&entity.nbt, STEAM_TANK).min(i32::MAX as u32) as i32;
    entity.data[7] = entity
        .items
        .get(SULFUR_SLOT)
        .and_then(Option::as_ref)
        .map_or(0, |stack| stack.count.min(i32::MAX as u32) as i32);
    entity.data[8] = heat_rate.min(i32::MAX as u32) as i32;
    entity.data[9] = status;
}

fn set_status(entity: &mut ModBlockEntity, status: i32, heat_rate: u32, total: u32) {
    set_i32(&mut entity.nbt, "SulfurRecoveryStatus", status);
    set_i32(
        &mut entity.nbt,
        "SulfurRecoveryHeatRate",
        heat_rate.min(i32::MAX as u32) as i32,
    );
    set_i32(
        &mut entity.nbt,
        "SulfurRecoveryTotal",
        total.min(i32::MAX as u32) as i32,
    );
    sync_screen(entity, heat_rate, total, status);
}

pub(crate) fn tick(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    entity.items.resize(2, None);
    let stored_heat = (get_i32(&entity.nbt, "HeatHU").max(0) as u64).min(HEAT_CAPACITY);
    set_i32(&mut entity.nbt, "HeatHU", stored_heat as i32);
    let heat = crate::heat::HeatStore::new(HEAT_CAPACITY, MAX_TEMPERATURE_C, stored_heat);

    let feed_name = crate::fluid::name(&entity.nbt, FEED_TANK);
    let recipe = feed_name
        .as_deref()
        .and_then(|name| find_recipe(host, name));
    let Some(recipe) = recipe else {
        set_i32(&mut entity.nbt, "SulfurRecoveryProgress", 0);
        set_i32(&mut entity.nbt, "SulfurRecoveryTotal", 0);
        set_status(entity, if feed_name.is_some() { 2 } else { 1 }, 0, 0);
        set_lit(host, pos, false);
        return;
    };

    let total = duration(&recipe, entity).max(1);
    if crate::fluid::amount(&entity.nbt, FEED_TANK) < recipe.input_amount {
        set_i32(&mut entity.nbt, "SulfurRecoveryProgress", 0);
        set_status(entity, 1, 0, total);
        set_lit(host, pos, false);
        return;
    }
    if crate::fluid::name(&entity.nbt, OXYGEN_TANK).as_deref()
        != Some(recipe.second_input.as_str())
        || crate::fluid::amount(&entity.nbt, OXYGEN_TANK) < recipe.second_input_amount
    {
        set_status(entity, 3, 0, total);
        set_lit(host, pos, false);
        return;
    }
    if !products_fit(entity, &recipe) {
        set_status(entity, 4, 0, total);
        set_lit(host, pos, false);
        return;
    }

    let upgrades = speed_upgrades(entity);
    let heat_per_tick = (f64::from(recipe.heat_per_tick)
        * crate::speed_upgrade_multiplier(upgrades))
    .ceil() as u64;
    if heat.temperature_c() < recipe.min_temperature || stored_heat < heat_per_tick {
        set_status(entity, 5, 0, total);
        set_lit(host, pos, false);
        return;
    }

    set_i32(
        &mut entity.nbt,
        "HeatHU",
        stored_heat.saturating_sub(heat_per_tick) as i32,
    );
    let next = progress(entity).saturating_add(1);
    set_i32(
        &mut entity.nbt,
        "SulfurRecoveryProgress",
        next.min(i32::MAX as u32) as i32,
    );
    set_status(
        entity,
        0,
        heat_per_tick.min(u64::from(u32::MAX)) as u32,
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
            OXYGEN_TANK,
            &recipe.second_input,
            recipe.second_input_amount,
        );
        crate::fluid::insert(
            &mut entity.nbt,
            STEAM_TANK,
            &recipe.steam,
            recipe.steam_amount,
            STEAM_CAPACITY,
        );
        crate::add_items(
            &mut entity.items[SULFUR_SLOT],
            &recipe.sulfur,
            recipe.sulfur_amount,
        );
        set_i32(&mut entity.nbt, "SulfurRecoveryProgress", 0);
        set_status(entity, 0, 0, total);
        set_lit(host, pos, false);
    }
}

pub(crate) fn accepts(tank: usize, fluid: &str) -> bool {
    match tank {
        FEED_TANK => fluid == "arcforge:hydrogen_sulfide",
        OXYGEN_TANK => fluid == "arcforge:oxygen",
        _ => false,
    }
}

pub(crate) fn fluid_tanks() -> ModFluidTanks {
    ModFluidTanks {
        tanks: vec![
            FEED_CAPACITY as i32,
            OXYGEN_CAPACITY as i32,
            STEAM_CAPACITY as i32,
        ],
        faces: [
            vec![0, 1, 2],
            vec![0, 1, 2],
            vec![0, 1, 2],
            vec![0, 1, 2],
            vec![0, 1, 2],
            vec![0, 1, 2],
        ],
        inputs: vec![FEED_TANK, OXYGEN_TANK],
        outputs: vec![STEAM_TANK],
    }
}

pub(crate) fn container() -> ModContainer {
    ModContainer {
        slots: 2,
        faces: [
            vec![SULFUR_SLOT],
            vec![SPEED_SLOT],
            vec![SPEED_SLOT],
            vec![SPEED_SLOT],
            vec![SPEED_SLOT],
            vec![SPEED_SLOT],
        ],
        inputs: vec![SPEED_SLOT],
        outputs: vec![SULFUR_SLOT],
        menu: Some("arcforge:machine_2_slot".into()),
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
    let sulfur = entity
        .items
        .get(SULFUR_SLOT)
        .and_then(Option::as_ref)
        .map_or(0, |stack| stack.count);
    let state = match get_i32(&entity.nbt, "SulfurRecoveryStatus") {
        1 => "needs Hydrogen Sulfide",
        2 => "no matching recipe",
        3 => "needs Oxygen",
        4 => "Steam tank or Sulfur Dust slot full",
        5 => "heating or needs HU",
        _ => "recovering sulfur",
    };
    format!(
        "Sulfur Recovery Unit: {state} | Hydrogen Sulfide {}/{}mB | Oxygen {}/{}mB | Steam {}/{}mB | Sulfur Dust {sulfur} | {}/{} ticks | {temperature}°C | Heat {stored_heat}/{HEAT_CAPACITY} HU",
        crate::fluid::amount(&entity.nbt, FEED_TANK),
        FEED_CAPACITY,
        crate::fluid::amount(&entity.nbt, OXYGEN_TANK),
        OXYGEN_CAPACITY,
        crate::fluid::amount(&entity.nbt, STEAM_TANK),
        STEAM_CAPACITY,
        progress(entity),
        get_i32(&entity.nbt, "SulfurRecoveryTotal").max(0),
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

    text!("Sulfur Recovery Unit", 8.0, 6.0, 0xFFFFFFFF);
    text!("Speed", 38.0, 17.0, 0xFFBBBBBB);
    text!("Sulfur Dust", 122.0, 17.0, 0xFFBBBBBB);
    text!("H2S", 8.0, 44.0, 0xFFBBBBBB);
    text!("Oxygen", 90.0, 44.0, 0xFFBBBBBB);
    text!("Steam", 8.0, 77.0, 0xFFBBBBBB);

    let heat = data.first().copied().unwrap_or_default().max(0);
    let temperature = data.get(1).copied().unwrap_or_default().max(0);
    let progress = data.get(2).copied().unwrap_or_default().max(0);
    let duration = data.get(3).copied().unwrap_or_default().max(0);
    let feed = data.get(4).copied().unwrap_or_default().max(0);
    let oxygen = data.get(5).copied().unwrap_or_default().max(0);
    let steam = data.get(6).copied().unwrap_or_default().max(0);
    let sulfur = data.get(7).copied().unwrap_or_default().max(0);
    let heat_rate = data.get(8).copied().unwrap_or_default().max(0);
    let state = match data.get(9).copied().unwrap_or_default() {
        1 => "Waiting for Hydrogen Sulfide",
        2 => "No matching recipe",
        3 => "Waiting for Oxygen",
        4 => "Steam tank or sulfur slot full",
        5 => "Heating or needs HU",
        _ if duration > 0 => "Recovering sulfur",
        _ => "Idle",
    };

    text!(
        &format!("{feed}/{} mB", FEED_CAPACITY),
        8.0,
        58.0,
        0xFFE0E0E0,
    );
    text!(
        &format!("{oxygen}/{} mB", OXYGEN_CAPACITY),
        90.0,
        58.0,
        0xFFE0E0E0,
    );
    text!(
        &format!("{steam}/{} mB", STEAM_CAPACITY),
        8.0,
        91.0,
        0xFFE0E0E0,
    );
    text!("Sulfur Dust", 90.0, 77.0, 0xFFBBBBBB);
    text!(&format!("{sulfur} items"), 90.0, 91.0, 0xFFE0E0E0);
    text!(
        &format!("Heat {heat}/{HEAT_CAPACITY} HU"),
        8.0,
        107.0,
        0xFFE0E0E0,
    );
    text!(
        &format!("{temperature}°C | Draw {heat_rate} HU/t"),
        8.0,
        119.0,
        0xFFE0E0E0,
    );
    text!(state, 8.0, 131.0, 0xFFBBBBBB);
    if duration > 0 {
        text!(
            &format!("Progress {progress}/{duration} t"),
            8.0,
            143.0,
            0xFFE0E0E0,
        );
        fill!([8.0, 154.0, 160.0, 6.0], 0xFF383838);
        let fraction = (progress as f32 / duration as f32).clamp(0.0, 1.0);
        fill!([8.0, 154.0, 160.0 * fraction, 6.0], 0xFF62B64F);
    }
    text!("Inventory", 8.0, 164.0, 0xFFFFFFFF);
}
