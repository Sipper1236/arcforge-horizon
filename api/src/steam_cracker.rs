//! Arcforge Steam Cracker: a formed 3x3x3 refinery that cracks feed gases with steam and heat.
use horizon_client::{
    containers::{ModBlockEntity, ModContainer},
    mod_api::{
        BlockEntityDraw, BlockEntityRenderBoundingBoxEvent, BlockEntitySubmitEvent, Compound,
        CubeFace, Host, ModFluidTanks, Tag, serde_json::Value,
    },
};

pub(crate) const CONTROLLER: &str = "arcforge:steam_cracker_controller";
pub(crate) const CASING: &str = "arcforge:steam_cracker_casing";

const WIDTH: i32 = 3;
const HEIGHT: i32 = 3;
const DEPTH: i32 = 3;
const HEAT_CAPACITY: u64 = 120_000;
const MAX_TEMPERATURE_C: u32 = 1_200;
const FEED_CAPACITY: u32 = 32_000;
const STEAM_CAPACITY: u32 = 16_000;
const PRODUCT_CAPACITY: u32 = 16_000;
const SPEED_SLOT: usize = 0;
const FEED_TANK: usize = 0;
const STEAM_TANK: usize = 1;
const PRODUCT_START: usize = 2;
const PRODUCT_COUNT: usize = 4;
const SPEED_UPGRADE: &str = "arcforge:speed_upgrade";
const STEAM: &str = "arcforge:steam";
const NAPHTHA: &str = "arcforge:naphtha";
const REFINERY_GAS: &str = "arcforge:refinery_gas";
const NATURAL_GAS: &str = "arcforge:natural_gas";
const ETHYLENE: &str = "arcforge:ethylene";
const PROPYLENE: &str = "arcforge:propylene";
const HYDROGEN: &str = "arcforge:hydrogen";
const HEAVY_OIL: &str = "arcforge:heavy_oil";

fn product_slot(fluid: &str) -> Option<usize> {
    match fluid {
        ETHYLENE => Some(0),
        PROPYLENE => Some(1),
        HYDROGEN => Some(2),
        HEAVY_OIL => Some(3),
        _ => None,
    }
}

#[derive(Clone, Copy)]
struct Facing {
    name: &'static str,
    vector: [i32; 3],
}

const FACINGS: [Facing; 4] = [
    Facing {
        name: "north",
        vector: [0, 0, -1],
    },
    Facing {
        name: "east",
        vector: [1, 0, 0],
    },
    Facing {
        name: "south",
        vector: [0, 0, 1],
    },
    Facing {
        name: "west",
        vector: [-1, 0, 0],
    },
];

fn facing(name: &str) -> Option<Facing> {
    FACINGS.into_iter().find(|value| value.name == name)
}

// The controller sits at the center of the bottom row on the front side.
fn box_positions(controller: [i32; 3], facing: Facing) -> Vec<[i32; 3]> {
    let inward = [-facing.vector[0], 0, -facing.vector[2]];
    let right = [-inward[2], 0, inward[0]];
    let mut positions = Vec::with_capacity((WIDTH * HEIGHT * DEPTH) as usize);
    for depth in 0..DEPTH {
        for y in 0..HEIGHT {
            for side in -1..=1 {
                positions.push([
                    controller[0] + inward[0] * depth + right[0] * side,
                    controller[1] + y,
                    controller[2] + inward[2] * depth + right[2] * side,
                ]);
            }
        }
    }
    positions
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

fn get_string(nbt: &Compound, name: &str) -> Option<String> {
    let Tag::String(key) = Tag::String(name.into()) else {
        unreachable!()
    };
    match nbt.get(&key) {
        Some(Tag::String(value)) => Some(value.to_string_lossy()),
        _ => None,
    }
}

fn set_string(nbt: &mut Compound, name: &str, value: &str) {
    let Tag::String(key) = Tag::String(name.into()) else {
        unreachable!()
    };
    nbt.insert(key, Tag::String(value.into()));
}

fn set_bool(host: &mut dyn Host, pos: [i32; 3], key: &str, value: bool) {
    let value = if value { "true" } else { "false" };
    if host.block_property(pos, key).as_deref() != Some(value) {
        host.set_block_property(pos, key, value);
    }
}

fn formed(entity: &ModBlockEntity) -> bool {
    get_i32(&entity.nbt, "SteamCrackerFormed") != 0
}

struct Product {
    fluid: String,
    amount: u32,
}

struct Recipe {
    input: String,
    input_amount: u32,
    steam_amount: u32,
    products: Vec<Product>,
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
    if second_input.get("fluid")?.as_str()? != STEAM {
        return None;
    }
    let products = json
        .get("fluids")?
        .as_array()?
        .iter()
        .map(|product| {
            Some(Product {
                fluid: product.get("id")?.as_str()?.to_owned(),
                amount: parse_amount(product.get("amount")?)?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    if products.is_empty() || products.len() > PRODUCT_COUNT {
        return None;
    }
    Some(Recipe {
        input: input.get("fluid")?.as_str()?.to_owned(),
        input_amount: parse_amount(input.get("amount")?)?,
        steam_amount: parse_amount(second_input.get("amount")?)?,
        products,
        ticks: json
            .get("time")
            .and_then(Value::as_u64)
            .unwrap_or(20)
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
    host.recipes_of_type("arcforge:steam_cracking")
        .iter()
        .filter_map(|recipe| recipe_from_json(&recipe.json))
        .find(|recipe| recipe.input == input)
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
    crate::speed_upgrade_time(recipe.ticks, speed_upgrades(entity)).max(1)
}

fn products_fit(entity: &ModBlockEntity, recipe: &Recipe) -> bool {
    let mut pending = [0_u32; PRODUCT_COUNT];
    for product in &recipe.products {
        let Some(index) = product_slot(&product.fluid) else {
            return false;
        };
        pending[index] = pending[index].saturating_add(product.amount);
    }
    pending.iter().enumerate().all(|(index, amount)| {
        let tank = PRODUCT_START + index;
        let expected = match index {
            0 => ETHYLENE,
            1 => PROPYLENE,
            2 => HYDROGEN,
            _ => HEAVY_OIL,
        };
        let existing = crate::fluid::name(&entity.nbt, tank);
        existing.as_deref().is_none_or(|fluid| fluid == expected)
            && crate::fluid::amount(&entity.nbt, tank).saturating_add(*amount) <= PRODUCT_CAPACITY
    })
}

fn progress(entity: &ModBlockEntity) -> u32 {
    get_i32(&entity.nbt, "SteamCrackerProgress").max(0) as u32
}

fn sync_screen(entity: &mut ModBlockEntity, heat_rate: u32, total: u32, status: i32) {
    entity.data.resize(14, 0);
    let stored_heat = get_i32(&entity.nbt, "HeatHU").max(0);
    entity.data[0] = stored_heat;
    entity.data[1] =
        crate::heat::HeatStore::new(HEAT_CAPACITY, MAX_TEMPERATURE_C, stored_heat as u64)
            .temperature_c() as i32;
    entity.data[2] = progress(entity).min(i32::MAX as u32) as i32;
    entity.data[3] = total.min(i32::MAX as u32) as i32;
    entity.data[4] = crate::fluid::amount(&entity.nbt, FEED_TANK).min(i32::MAX as u32) as i32;
    entity.data[5] = crate::fluid::amount(&entity.nbt, STEAM_TANK).min(i32::MAX as u32) as i32;
    for index in 0..PRODUCT_COUNT {
        entity.data[6 + index] =
            crate::fluid::amount(&entity.nbt, PRODUCT_START + index).min(i32::MAX as u32) as i32;
    }
    entity.data[10] = heat_rate.min(i32::MAX as u32) as i32;
    entity.data[11] = status;
    entity.data[12] = if formed(entity) { 1 } else { 0 };
    entity.data[13] = speed_upgrades(entity) as i32;
}

fn set_status(entity: &mut ModBlockEntity, status: i32, heat_rate: u32, total: u32) {
    set_i32(&mut entity.nbt, "SteamCrackerStatus", status);
    set_i32(
        &mut entity.nbt,
        "SteamCrackerHeatRate",
        heat_rate.min(i32::MAX as u32) as i32,
    );
    set_i32(
        &mut entity.nbt,
        "SteamCrackerTotal",
        total.min(i32::MAX as u32) as i32,
    );
    sync_screen(entity, heat_rate, total, status);
}

fn pause(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity, status: i32, total: u32) {
    set_status(entity, status, 0, total);
    set_bool(host, pos, "lit", false);
}

fn form(host: &mut dyn Host, pos: [i32; 3], facing: Facing, entity: &mut ModBlockEntity) -> bool {
    let positions = box_positions(pos, facing);
    if positions.len() != (WIDTH * HEIGHT * DEPTH) as usize {
        return false;
    }
    for part in &positions {
        if *part == pos {
            if host.block(*part) != CONTROLLER {
                return false;
            }
        } else if host.block(*part) != CASING
            || host.block_property(*part, "formed").as_deref() == Some("true")
        {
            return false;
        }
    }
    for part in positions {
        if part != pos {
            set_bool(host, part, "formed", true);
        }
    }
    set_bool(host, pos, "formed", true);
    set_bool(host, pos, "lit", false);
    set_i32(&mut entity.nbt, "SteamCrackerFormed", 1);
    set_i32(&mut entity.nbt, "SteamCrackerProgress", 0);
    set_i32(&mut entity.nbt, "SteamCrackerScan", 0);
    set_string(&mut entity.nbt, "SteamCrackerFacing", facing.name);
    host.block_entity_sync(pos);
    true
}

fn scan(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) -> bool {
    for direction in FACINGS {
        if form(host, pos, direction, entity) {
            return true;
        }
    }
    false
}

pub(crate) fn notify_placed(host: &mut dyn Host, placed: [i32; 3]) {
    for y in 0..=HEIGHT {
        for z in -WIDTH..=WIDTH {
            for x in -WIDTH..=WIDTH {
                let pos = [placed[0] + x, placed[1] - y, placed[2] + z];
                if host
                    .mod_block_entity(pos)
                    .is_some_and(|entity| entity.id == CONTROLLER)
                {
                    host.change_mod_block_entity(pos, &mut |entity| {
                        if entity.id == CONTROLLER {
                            set_i32(&mut entity.nbt, "SteamCrackerScan", 1);
                        }
                    });
                }
            }
        }
    }
}

fn clear_box(host: &mut dyn Host, pos: [i32; 3], facing: Facing) {
    for part in box_positions(pos, facing) {
        if part != pos && host.block(part) == CASING {
            set_bool(host, part, "formed", false);
        }
    }
    set_bool(host, pos, "formed", false);
    set_bool(host, pos, "lit", false);
    host.change_mod_block_entity(pos, &mut |entity| {
        if entity.id == CONTROLLER {
            set_i32(&mut entity.nbt, "SteamCrackerFormed", 0);
            set_i32(&mut entity.nbt, "SteamCrackerProgress", 0);
            set_i32(&mut entity.nbt, "SteamCrackerScan", 0);
        }
    });
    host.block_entity_sync(pos);
}

pub(crate) fn unform(host: &mut dyn Host, pos: [i32; 3], nbt: &Compound) {
    let name = get_string(nbt, "SteamCrackerFacing").or_else(|| host.block_property(pos, "facing"));
    if let Some(direction) = name.as_deref().and_then(facing) {
        clear_box(host, pos, direction);
    }
}

pub(crate) fn unform_near(host: &mut dyn Host, broken: [i32; 3]) {
    for y in 0..=HEIGHT {
        for z in -WIDTH..=WIDTH {
            for x in -WIDTH..=WIDTH {
                let pos = [broken[0] + x, broken[1] - y, broken[2] + z];
                let Some(entity) = host.mod_block_entity(pos) else {
                    continue;
                };
                if entity.id != CONTROLLER || !formed(&entity) {
                    continue;
                }
                let Some(direction) =
                    get_string(&entity.nbt, "SteamCrackerFacing").and_then(|name| facing(&name))
                else {
                    continue;
                };
                if box_positions(pos, direction).contains(&broken) {
                    clear_box(host, pos, direction);
                }
            }
        }
    }
}

pub(crate) fn tick(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    entity.items.resize(1, None);
    let stored_heat = (get_i32(&entity.nbt, "HeatHU").max(0) as u64).min(HEAT_CAPACITY);
    set_i32(&mut entity.nbt, "HeatHU", stored_heat as i32);

    if !formed(entity) {
        let scan_timer = get_i32(&entity.nbt, "SteamCrackerScan").max(0);
        if scan_timer == 0 {
            let _ = scan(host, pos, entity);
            if !formed(entity) {
                set_i32(&mut entity.nbt, "SteamCrackerScan", 20);
            }
        } else {
            set_i32(&mut entity.nbt, "SteamCrackerScan", scan_timer - 1);
        }
        if !formed(entity) {
            set_i32(&mut entity.nbt, "SteamCrackerProgress", 0);
            pause(host, pos, entity, 1, 0);
            return;
        }
    }

    let feed_name = crate::fluid::name(&entity.nbt, FEED_TANK);
    let recipe = feed_name
        .as_deref()
        .and_then(|name| find_recipe(host, name));
    let Some(recipe) = recipe else {
        set_i32(&mut entity.nbt, "SteamCrackerProgress", 0);
        set_string(&mut entity.nbt, "SteamCrackerRecipe", "");
        pause(
            host,
            pos,
            entity,
            if feed_name.is_some() { 2 } else { 0 },
            0,
        );
        return;
    };

    if get_string(&entity.nbt, "SteamCrackerRecipe").as_deref() != Some(recipe.input.as_str()) {
        set_i32(&mut entity.nbt, "SteamCrackerProgress", 0);
        set_string(&mut entity.nbt, "SteamCrackerRecipe", &recipe.input);
    }
    let total = duration(&recipe, entity);
    if crate::fluid::amount(&entity.nbt, FEED_TANK) < recipe.input_amount
        || crate::fluid::name(&entity.nbt, STEAM_TANK).as_deref() != Some(STEAM)
        || crate::fluid::amount(&entity.nbt, STEAM_TANK) < recipe.steam_amount
    {
        pause(host, pos, entity, 3, total);
        return;
    }
    if !products_fit(entity, &recipe) {
        pause(host, pos, entity, 4, total);
        return;
    }

    let heat = crate::heat::HeatStore::new(HEAT_CAPACITY, MAX_TEMPERATURE_C, stored_heat);
    let heat_per_tick = (f64::from(recipe.heat_per_tick)
        * crate::speed_upgrade_multiplier(speed_upgrades(entity)))
    .ceil() as u64;
    if heat.temperature_c() < recipe.min_temperature || stored_heat < heat_per_tick {
        pause(host, pos, entity, 5, total);
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
        "SteamCrackerProgress",
        next.min(i32::MAX as u32) as i32,
    );
    set_status(
        entity,
        0,
        heat_per_tick.min(u64::from(u32::MAX)) as u32,
        total,
    );
    set_bool(host, pos, "lit", true);

    if next >= total {
        crate::fluid::extract(
            &mut entity.nbt,
            FEED_TANK,
            &recipe.input,
            recipe.input_amount,
        );
        crate::fluid::extract(&mut entity.nbt, STEAM_TANK, STEAM, recipe.steam_amount);
        for product in &recipe.products {
            let Some(index) = product_slot(&product.fluid) else {
                continue;
            };
            crate::fluid::insert(
                &mut entity.nbt,
                PRODUCT_START + index,
                &product.fluid,
                product.amount,
                PRODUCT_CAPACITY,
            );
        }
        set_i32(&mut entity.nbt, "SteamCrackerProgress", 0);
        set_status(entity, 0, 0, total);
        set_bool(host, pos, "lit", false);
    }
}

pub(crate) fn accepts(tank: usize, fluid: &str) -> bool {
    (tank == FEED_TANK && (fluid == NAPHTHA || fluid == REFINERY_GAS || fluid == NATURAL_GAS))
        || (tank == STEAM_TANK && fluid == STEAM)
}

pub(crate) fn fluid_tanks() -> ModFluidTanks {
    ModFluidTanks {
        tanks: vec![
            FEED_CAPACITY as i32,
            STEAM_CAPACITY as i32,
            PRODUCT_CAPACITY as i32,
            PRODUCT_CAPACITY as i32,
            PRODUCT_CAPACITY as i32,
            PRODUCT_CAPACITY as i32,
        ],
        faces: [
            vec![0, 1, 2, 3, 4, 5],
            vec![0, 1, 2, 3, 4, 5],
            vec![0, 1, 2, 3, 4, 5],
            vec![0, 1, 2, 3, 4, 5],
            vec![0, 1, 2, 3, 4, 5],
            vec![0, 1, 2, 3, 4, 5],
        ],
        inputs: vec![FEED_TANK, STEAM_TANK],
        outputs: vec![2, 3, 4, 5],
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
    if entity.id != CONTROLLER || !formed(&entity) {
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
            if current.id == CONTROLLER {
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

pub(crate) fn wrench(host: &mut dyn Host, pos: [i32; 3], face_hit: [i32; 3]) {
    if let Some(name) = match face_hit {
        [0, 0, -1] => Some("north"),
        [1, 0, 0] => Some("east"),
        [0, 0, 1] => Some("south"),
        [-1, 0, 0] => Some("west"),
        _ => None,
    } {
        if let Some(entity) = host.mod_block_entity(pos) {
            if formed(&entity) {
                let old_facing = get_string(&entity.nbt, "SteamCrackerFacing")
                    .or_else(|| host.block_property(pos, "facing"));
                if let Some(old_facing) = old_facing.as_deref().and_then(facing) {
                    clear_box(host, pos, old_facing);
                }
            }
        }
        host.set_block_property(pos, "facing", name);
        host.change_mod_block_entity(pos, &mut |entity| {
            if entity.id == CONTROLLER {
                set_string(&mut entity.nbt, "SteamCrackerFacing", name);
                set_i32(&mut entity.nbt, "SteamCrackerScan", 1);
            }
        });
    }
    if let Some(entity) = host.mod_block_entity(pos) {
        if let Some(player) = host.acting_player() {
            host.action_bar(
                player.uuid,
                horizon_client::mod_api::Text::literal(status(&entity)),
            );
        }
    }
}

pub(crate) fn status(entity: &ModBlockEntity) -> String {
    let stored_heat = get_i32(&entity.nbt, "HeatHU").max(0) as u64;
    let temperature =
        crate::heat::HeatStore::new(HEAT_CAPACITY, MAX_TEMPERATURE_C, stored_heat).temperature_c();
    let state = match get_i32(&entity.nbt, "SteamCrackerStatus") {
        1 => "needs a formed 3x3x3 casing",
        2 => "no recipe for this feed",
        3 => "needs feed or Steam",
        4 => "product tanks full",
        5 => "heating or needs more HU",
        _ if !formed(entity) => "not formed",
        _ if get_i32(&entity.nbt, "SteamCrackerTotal") > 0 => "cracking",
        _ => "idle",
    };
    format!(
        "Steam Cracker: {state} | Feed {}/{}mB | Steam {}/{}mB | Ethylene {}mB | Propylene {}mB | Hydrogen {}mB | Heavy Oil {}mB | {temperature}°C | Heat {stored_heat}/{HEAT_CAPACITY} HU",
        crate::fluid::amount(&entity.nbt, FEED_TANK),
        FEED_CAPACITY,
        crate::fluid::amount(&entity.nbt, STEAM_TANK),
        STEAM_CAPACITY,
        crate::fluid::amount(&entity.nbt, PRODUCT_START),
        crate::fluid::amount(&entity.nbt, PRODUCT_START + 1),
        crate::fluid::amount(&entity.nbt, PRODUCT_START + 2),
        crate::fluid::amount(&entity.nbt, PRODUCT_START + 3),
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

    let labels = ["Ethylene", "Propylene", "Hydrogen", "Heavy Oil"];
    text!("Steam Cracker", 8.0, 6.0, 0xFFFFFFFF);
    text!("Speed", 80.0, 17.0, 0xFFBBBBBB);
    text!("Feed", 8.0, 48.0, 0xFFBBBBBB);
    text!("Steam", 90.0, 48.0, 0xFFBBBBBB);
    text!(
        &format!(
            "{}/{} mB",
            data.get(4).copied().unwrap_or_default(),
            FEED_CAPACITY
        ),
        8.0,
        61.0,
        0xFFE0E0E0,
    );
    text!(
        &format!(
            "{}/{} mB",
            data.get(5).copied().unwrap_or_default(),
            STEAM_CAPACITY
        ),
        90.0,
        61.0,
        0xFFE0E0E0,
    );
    for (index, label) in labels.into_iter().enumerate() {
        let [x, y] = if index % 2 == 0 {
            [8.0, 86.0 + 27.0 * (index / 2) as f32]
        } else {
            [90.0, 86.0 + 27.0 * (index / 2) as f32]
        };
        text!(label, x, y, 0xFFBBBBBB);
        text!(
            &format!("{} mB", data.get(6 + index).copied().unwrap_or_default()),
            x,
            y + 13.0,
            0xFFE0E0E0,
        );
    }
    let heat = data.first().copied().unwrap_or_default().max(0);
    let temperature = data.get(1).copied().unwrap_or_default().max(0);
    let progress = data.get(2).copied().unwrap_or_default().max(0);
    let total = data.get(3).copied().unwrap_or_default().max(0);
    let heat_rate = data.get(10).copied().unwrap_or_default().max(0);
    let formed = data.get(12).copied().unwrap_or_default() != 0;
    let status = match data.get(11).copied().unwrap_or_default() {
        1 => "Needs formed 3x3x3 casing",
        2 => "No recipe for this feed",
        3 => "Needs feed or Steam",
        4 => "Product tanks full",
        5 => "Heating or needs more HU",
        _ if !formed => "Not formed",
        _ if total > 0 => "Cracking",
        _ => "Idle",
    };
    text!(
        &format!("Heat {heat}/{HEAT_CAPACITY} HU"),
        8.0,
        145.0,
        0xFFE0E0E0,
    );
    text!(
        &format!("{temperature}°C | Draw {heat_rate} HU/t"),
        8.0,
        158.0,
        0xFFE0E0E0,
    );
    text!(status, 8.0, 172.0, 0xFFBBBBBB);
    if total > 0 {
        text!(
            &format!("Progress {progress}/{total} t"),
            8.0,
            186.0,
            0xFFE0E0E0,
        );
        fill!([8.0, 197.0, 160.0, 6.0], 0xFF383838);
        let fraction = (progress as f32 / total as f32).clamp(0.0, 1.0);
        fill!([8.0, 197.0, 160.0 * fraction, 6.0], 0xFF62B64F);
    }
    text!("Inventory", 8.0, 207.0, 0xFFFFFFFF);
}

const CASING_SPRITE: &str = "arcforge:block/refinery/steam_cracker_casing";
const CONTROLLER_SPRITE: &str = "arcforge:block/refinery/steam_cracker_controller";
const HOT_SPRITE: &str = "arcforge:block/refinery/steam_cracker_controller_on";

fn draw_model_box(
    host: &dyn Host,
    draw: &mut BlockEntityDraw,
    x0: f32,
    y0: f32,
    u0: f32,
    x1: f32,
    y1: f32,
    u1: f32,
    sprite: &str,
) {
    // Model coordinates use texels; the controller is centered across the
    // three-block width, so shift that axis to the block-entity origin.
    let face = CubeFace { sprite, uv: None };
    let _ = host.draw_cube(
        draw,
        [x0 - 16.0, y0, u0],
        [x1 - 16.0, y1, u1],
        [Some(face); 6],
        0x00FF_FFFF,
    );
}

fn renderer_yaw(facing: Facing) -> f32 {
    match facing.name {
        "east" => -90.0,
        "south" => 180.0,
        "west" => 90.0,
        _ => 0.0,
    }
}

pub(crate) fn render_bounds(event: &mut BlockEntityRenderBoundingBoxEvent<'_>) {
    if formed(event.entity) {
        // This box includes the 3x3x3 model after it turns around the controller.
        event
            .bounds
            .copy_from_slice(&[-2.0, 0.0, -2.0, 3.0, 3.0, 3.0]);
    }
}

pub(crate) fn draw_model(host: &dyn Host, event: &mut BlockEntitySubmitEvent<'_>) {
    if !formed(event.entity) {
        return;
    }
    let Some(direction) =
        get_string(&event.entity.nbt, "SteamCrackerFacing").and_then(|name| facing(&name))
    else {
        return;
    };
    let heat = get_i32(&event.entity.nbt, "HeatHU").clamp(0, HEAT_CAPACITY as i32) as u64;
    let temperature = 20 + (heat * u64::from(MAX_TEMPERATURE_C - 20) / HEAT_CAPACITY) as u32;
    let hot_sprite = if temperature >= 800 {
        HOT_SPRITE
    } else {
        CONTROLLER_SPRITE
    };

    let draw = &mut *event.draw;
    draw.push();
    draw.rotate_about([0.0, 1.0, 0.0], renderer_yaw(direction), [0.5, 0.5, 0.5]);

    // Deck, firebox and its structural bands.
    draw_model_box(host, draw, 0.0, 0.0, 0.0, 48.0, 3.0, 48.0, CASING_SPRITE);
    draw_model_box(host, draw, 3.0, 3.0, 4.0, 31.0, 34.0, 44.0, CASING_SPRITE);
    draw_model_box(
        host,
        draw,
        2.0,
        3.0,
        3.0,
        32.0,
        5.0,
        45.0,
        CONTROLLER_SPRITE,
    );
    draw_model_box(
        host,
        draw,
        2.0,
        32.5,
        3.0,
        32.0,
        34.0,
        45.0,
        CONTROLLER_SPRITE,
    );
    draw_model_box(
        host,
        draw,
        1.6,
        18.0,
        2.6,
        32.4,
        20.0,
        45.4,
        CONTROLLER_SPRITE,
    );

    for u in [12.0, 24.0, 36.0] {
        draw_model_box(
            host,
            draw,
            2.0,
            5.0,
            u - 1.0,
            3.0,
            32.5,
            u + 1.0,
            CONTROLLER_SPRITE,
        );
        draw_model_box(
            host,
            draw,
            31.0,
            5.0,
            u - 1.0,
            32.0,
            32.5,
            u + 1.0,
            CONTROLLER_SPRITE,
        );
    }
    for x in [10.0, 24.0] {
        draw_model_box(
            host,
            draw,
            x - 1.0,
            5.0,
            3.0,
            x + 1.0,
            32.5,
            4.0,
            CONTROLLER_SPRITE,
        );
        draw_model_box(
            host,
            draw,
            x - 1.0,
            5.0,
            44.0,
            x + 1.0,
            32.5,
            45.0,
            CONTROLLER_SPRITE,
        );
    }

    // Refractory peep holes on the front and left walls; the inner panel
    // changes to the hot texture after the firebox reaches cracking heat.
    for (y, x) in [(11.0, 17.0), (26.0, 17.0)] {
        draw_model_box(
            host,
            draw,
            x - 3.5,
            y - 3.0,
            3.0,
            x + 3.5,
            y + 3.0,
            4.4,
            CONTROLLER_SPRITE,
        );
        draw_model_box(
            host,
            draw,
            x - 2.0,
            y - 2.0,
            2.8,
            x + 2.0,
            y + 2.0,
            3.2,
            hot_sprite,
        );
    }
    for (u, y) in [(18.0, 11.0), (30.0, 11.0), (18.0, 26.0), (30.0, 26.0)] {
        draw_model_box(
            host,
            draw,
            2.5,
            y - 3.0,
            u - 3.5,
            4.4,
            y + 3.0,
            u + 3.5,
            CONTROLLER_SPRITE,
        );
        draw_model_box(
            host,
            draw,
            2.8,
            y - 2.0,
            u - 2.0,
            3.2,
            y + 2.0,
            u + 2.0,
            hot_sprite,
        );
    }

    // Radiant tubes and twin headers rising from the firebox roof.
    for x in [11.0, 23.0] {
        draw_model_box(
            host,
            draw,
            x - 1.6,
            37.0,
            6.0,
            x + 1.6,
            41.0,
            29.0,
            CONTROLLER_SPRITE,
        );
        for u in [8.0, 12.0, 16.0, 20.0, 24.0, 28.0] {
            draw_model_box(
                host,
                draw,
                x - 0.6,
                34.0,
                u - 0.6,
                x + 0.6,
                39.0,
                u + 0.6,
                CONTROLLER_SPRITE,
            );
        }
    }
    draw_model_box(host, draw, 9.0, 34.0, 30.0, 25.0, 39.0, 42.0, CASING_SPRITE);
    draw_model_box(
        host,
        draw,
        14.0,
        39.0,
        33.0,
        20.0,
        48.0,
        39.0,
        CONTROLLER_SPRITE,
    );

    // Quench drum, base rings and transfer line from the collector.
    draw_model_box(host, draw, 33.5, 3.0, 16.0, 45.5, 32.0, 28.0, CASING_SPRITE);
    draw_model_box(
        host,
        draw,
        33.0,
        8.0,
        15.5,
        46.0,
        10.0,
        28.5,
        CONTROLLER_SPRITE,
    );
    draw_model_box(
        host,
        draw,
        33.0,
        20.0,
        15.5,
        46.0,
        22.0,
        28.5,
        CONTROLLER_SPRITE,
    );
    draw_model_box(
        host,
        draw,
        33.0,
        30.0,
        15.5,
        46.0,
        32.0,
        28.5,
        CONTROLLER_SPRITE,
    );
    draw_model_box(
        host,
        draw,
        22.0,
        38.0,
        5.0,
        41.0,
        41.0,
        8.0,
        CONTROLLER_SPRITE,
    );
    draw_model_box(
        host,
        draw,
        38.0,
        31.0,
        20.0,
        41.0,
        40.0,
        23.0,
        CONTROLLER_SPRITE,
    );
    draw_model_box(
        host,
        draw,
        35.0,
        30.0,
        20.0,
        44.0,
        32.0,
        23.0,
        CONTROLLER_SPRITE,
    );

    draw.pop();
}
