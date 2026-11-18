//! Arcforge Cracking Unit: a 3x3x5 refinery box for catalytic and hydrogen cracking.
use horizon_client::{
    containers::{ModBlockEntity, ModContainer},
    mod_api::{Compound, Host, ModFluidTanks, Tag, serde_json::Value},
};

pub(crate) const CONTROLLER: &str = "arcforge:cracking_unit_controller";
pub(crate) const CASING: &str = "arcforge:cracking_unit_casing";
pub(crate) const CATALYST: &str = "arcforge:cracking_catalyst";
const WIDTH: i32 = 3;
const HEIGHT: i32 = 5;
const LENGTH: i32 = 3;
const HEAT_CAPACITY: u64 = 80_000;
const MAX_TEMP_C: u32 = 800;
const FEED_CAPACITY: u32 = 32_000;
const HYDROGEN_CAPACITY: u32 = 16_000;
const PRODUCT_CAPACITY: u32 = 16_000;
const MAX_CATALYST_USES: u32 = 50;
const HEAVY_OIL: &str = "arcforge:heavy_oil";
const HYDROGEN: &str = "arcforge:hydrogen";
const SPEED_UPGRADE: &str = "arcforge:speed_upgrade";
const SPEED_SLOT: usize = 0;
const CATALYST_SLOT: usize = 1;
const FEED_TANK: usize = 0;
const HYDROGEN_TANK: usize = 1;
const OUTPUT_START: usize = 2;

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
fn box_positions(controller: [i32; 3], facing: Facing) -> Vec<[i32; 3]> {
    let back = [-facing.vector[0], 0, -facing.vector[2]];
    let right = [-back[2], 0, back[0]];
    let mut positions = Vec::with_capacity((WIDTH * HEIGHT * LENGTH) as usize);
    for depth in 0..LENGTH {
        for y in 0..HEIGHT {
            for side in -1..=1 {
                positions.push([
                    controller[0] + back[0] * depth + right[0] * side,
                    controller[1] + y,
                    controller[2] + back[2] * depth + right[2] * side,
                ]);
            }
        }
    }
    positions
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
fn get_string(nbt: &Compound, key: &str) -> Option<String> {
    let Tag::String(key) = Tag::String(key.into()) else {
        unreachable!()
    };
    match nbt.get(&key) {
        Some(Tag::String(value)) => Some(value.to_string_lossy()),
        _ => None,
    }
}
fn set_string(nbt: &mut Compound, key: &str, value: &str) {
    let Tag::String(key) = Tag::String(key.into()) else {
        unreachable!()
    };
    nbt.insert(key, Tag::String(value.into()));
}
fn mode(entity: &ModBlockEntity) -> &'static str {
    match get_string(&entity.nbt, "CrackingMode").as_deref() {
        Some("hydrocracking") => "hydrocracking",
        _ => "catalytic",
    }
}
fn set_bool(host: &mut dyn Host, pos: [i32; 3], key: &str, value: bool) {
    let value = if value { "true" } else { "false" };
    if host.block_property(pos, key).as_deref() != Some(value) {
        host.set_block_property(pos, key, value);
    }
}

struct Product {
    fluid: String,
    amount: u32,
}
struct Recipe {
    mode: String,
    input: String,
    amount: u32,
    second_input: Option<(String, u32)>,
    products: Vec<Product>,
    catalyst: Option<String>,
    catalyst_uses: u32,
    ticks: u32,
    heat_per_tick: u64,
    min_temp: u32,
}
fn recipe_from_json(json: &Value) -> Option<Recipe> {
    let input = json.get("input")?;
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
    let second_input = json.get("second_input").and_then(|value| {
        Some((
            value.get("fluid")?.as_str()?.to_owned(),
            value.get("amount")?.as_u64()?.clamp(1, u64::from(u32::MAX)) as u32,
        ))
    });
    Some(Recipe {
        mode: json.get("mode")?.as_str()?.to_owned(),
        input: input.get("fluid")?.as_str()?.to_owned(),
        amount: input.get("amount")?.as_u64()?.clamp(1, u64::from(u32::MAX)) as u32,
        second_input,
        products,
        catalyst: json
            .get("catalyst")
            .and_then(Value::as_str)
            .map(str::to_owned),
        catalyst_uses: json
            .get("catalyst_uses")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .clamp(0, u64::from(u32::MAX)) as u32,
        ticks: json
            .get("time")
            .and_then(Value::as_u64)
            .unwrap_or(20)
            .clamp(1, u64::from(u32::MAX)) as u32,
        heat_per_tick: json
            .get("heat_per_tick")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        min_temp: json
            .get("min_temp")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .min(u64::from(u32::MAX)) as u32,
    })
}
fn find_recipe(host: &dyn Host, selected_mode: &str) -> Option<Recipe> {
    host.recipes_of_type("arcforge:cracking")
        .iter()
        .filter_map(|recipe| recipe_from_json(&recipe.json))
        .find(|recipe| recipe.mode == selected_mode && recipe.input == HEAVY_OIL)
}
fn catalyst_uses(entity: &ModBlockEntity) -> u32 {
    if entity
        .items
        .get(CATALYST_SLOT)
        .and_then(Option::as_ref)
        .is_none_or(|item| item.item.as_ref() != CATALYST)
    {
        return 0;
    }
    get_i32(&entity.nbt, "CrackingCatalystUses").clamp(0, MAX_CATALYST_USES as i32) as u32
}
fn speed_count(entity: &ModBlockEntity) -> u32 {
    entity
        .items
        .get(SPEED_SLOT)
        .and_then(Option::as_ref)
        .filter(|item| item.item.as_ref() == SPEED_UPGRADE)
        .map_or(0, |item| item.count.min(8))
}
fn total_ticks(recipe: &Recipe, entity: &ModBlockEntity) -> u32 {
    crate::speed_upgrade_time(recipe.ticks, speed_count(entity)).max(1)
}
fn products_fit(entity: &ModBlockEntity, recipe: &Recipe) -> bool {
    if recipe.products.len() != 4 {
        return false;
    }
    recipe.products.iter().enumerate().all(|(index, product)| {
        let tank = OUTPUT_START + index;
        let name = crate::fluid::name(&entity.nbt, tank);
        name.as_deref()
            .is_none_or(|existing| existing == product.fluid)
            && crate::fluid::amount(&entity.nbt, tank).saturating_add(product.amount)
                <= PRODUCT_CAPACITY
    })
}
fn set_status(entity: &mut ModBlockEntity, value: i32, duration: u32) {
    set_i32(&mut entity.nbt, "CrackingStatus", value);
    set_i32(
        &mut entity.nbt,
        "CrackingTotal",
        duration.min(i32::MAX as u32) as i32,
    );
}
fn formed(entity: &ModBlockEntity) -> bool {
    get_i32(&entity.nbt, "CrackingFormed") != 0
}
fn form(host: &mut dyn Host, controller: [i32; 3], face: Facing) -> bool {
    let positions = box_positions(controller, face);
    if positions.len() != 45 {
        return false;
    }
    for pos in &positions {
        if *pos == controller {
            if host.block(*pos) != CONTROLLER {
                return false;
            }
        } else if host.block(*pos) != CASING
            || host.block_property(*pos, "formed").as_deref() == Some("true")
        {
            return false;
        }
    }
    for pos in positions {
        if pos != controller {
            set_bool(host, pos, "formed", true);
        }
    }
    set_bool(host, controller, "formed", true);
    let current_face = host
        .block_property(controller, "facing")
        .unwrap_or_else(|| face.name.to_owned());
    host.change_mod_block_entity(controller, &mut |current| {
        if current.id == CONTROLLER {
            set_i32(&mut current.nbt, "CrackingFormed", 1);
            set_i32(&mut current.nbt, "CrackingProgress", 0);
            set_i32(&mut current.nbt, "CrackingScan", 0);
            set_string(&mut current.nbt, "CrackingFacing", &current_face);
        }
    });
    host.block_entity_sync(controller);
    true
}
fn clear_box(host: &mut dyn Host, controller: [i32; 3], face: Facing) {
    for pos in box_positions(controller, face) {
        if pos != controller && host.block(pos) == CASING {
            set_bool(host, pos, "formed", false);
        }
    }
    set_bool(host, controller, "formed", false);
    host.change_mod_block_entity(controller, &mut |entity| {
        if entity.id == CONTROLLER {
            set_i32(&mut entity.nbt, "CrackingFormed", 0);
            set_i32(&mut entity.nbt, "CrackingProgress", 0);
            set_i32(&mut entity.nbt, "CrackingScan", 0);
        }
    });
    host.block_entity_sync(controller);
}
fn scan(host: &mut dyn Host, pos: [i32; 3]) -> bool {
    for face in FACINGS {
        if form(host, pos, face) {
            return true;
        }
    }
    false
}
pub(crate) fn notify_placed(host: &mut dyn Host, placed: [i32; 3]) {
    for y in 0..=HEIGHT {
        for z in -LENGTH..=LENGTH {
            for x in -WIDTH..=WIDTH {
                let pos = [placed[0] + x, placed[1] - y, placed[2] + z];
                if host
                    .mod_block_entity(pos)
                    .is_some_and(|entity| entity.id == CONTROLLER)
                {
                    host.change_mod_block_entity(pos, &mut |entity| {
                        if entity.id == CONTROLLER {
                            set_i32(&mut entity.nbt, "CrackingScan", 1);
                        }
                    });
                }
            }
        }
    }
}
pub(crate) fn unform(host: &mut dyn Host, pos: [i32; 3], nbt: &Compound) {
    if let Some(name) =
        get_string(nbt, "CrackingFacing").or_else(|| host.block_property(pos, "facing"))
    {
        if let Some(face) = facing(&name) {
            clear_box(host, pos, face);
        }
    }
}
pub(crate) fn unform_near(host: &mut dyn Host, broken: [i32; 3]) {
    for dy in 0..=HEIGHT {
        for dz in -LENGTH..=LENGTH {
            for dx in -WIDTH..=WIDTH {
                let pos = [broken[0] + dx, broken[1] - dy, broken[2] + dz];
                let Some(entity) = host.mod_block_entity(pos) else {
                    continue;
                };
                if entity.id != CONTROLLER || !formed(&entity) {
                    continue;
                }
                let Some(name) = get_string(&entity.nbt, "CrackingFacing") else {
                    continue;
                };
                let Some(face) = facing(&name) else {
                    continue;
                };
                if box_positions(pos, face).contains(&broken) {
                    clear_box(host, pos, face);
                }
            }
        }
    }
}
pub(crate) fn tick(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    entity.items.resize(2, None);
    if entity
        .items
        .get(CATALYST_SLOT)
        .and_then(Option::as_ref)
        .is_none()
    {
        set_i32(&mut entity.nbt, "CrackingCatalystUses", 0);
    }
    let mut scan_ticks = get_i32(&entity.nbt, "CrackingScan").max(0);
    if !formed(entity) {
        if scan_ticks <= 0 {
            scan_ticks = 20;
            let _ = scan(host, pos);
        }
        set_i32(&mut entity.nbt, "CrackingScan", scan_ticks - 1);
        set_i32(&mut entity.nbt, "CrackingProgress", 0);
        set_status(entity, 1, 0);
        set_bool(host, pos, "lit", false);
        return;
    }
    let stored = get_i32(&entity.nbt, "HeatHU").max(0) as u64;
    let heat = crate::heat::HeatStore::new(HEAT_CAPACITY, MAX_TEMP_C, stored);
    let selected_mode = mode(entity);
    let recipe = find_recipe(host, selected_mode);
    let Some(recipe) = recipe else {
        set_i32(&mut entity.nbt, "CrackingProgress", 0);
        set_status(entity, 2, 0);
        set_bool(host, pos, "lit", false);
        return;
    };
    if crate::fluid::amount(&entity.nbt, FEED_TANK) < recipe.amount {
        set_status(entity, 3, total_ticks(&recipe, entity));
        set_bool(host, pos, "lit", false);
        return;
    }
    if let Some((fluid, amount)) = &recipe.second_input {
        if crate::fluid::name(&entity.nbt, HYDROGEN_TANK).as_deref() != Some(fluid)
            || crate::fluid::amount(&entity.nbt, HYDROGEN_TANK) < *amount
        {
            set_status(entity, 3, total_ticks(&recipe, entity));
            set_bool(host, pos, "lit", false);
            return;
        }
    }
    if recipe.catalyst.as_deref() == Some(CATALYST) && catalyst_uses(entity) == 0 {
        if let Some(item) = entity.items.get(CATALYST_SLOT).and_then(Option::as_ref) {
            if item.item.as_ref() == CATALYST {
                set_i32(
                    &mut entity.nbt,
                    "CrackingCatalystUses",
                    MAX_CATALYST_USES as i32,
                );
            }
        }
        if catalyst_uses(entity) == 0 {
            set_status(entity, 4, total_ticks(&recipe, entity));
            set_bool(host, pos, "lit", false);
            return;
        }
    }
    if !products_fit(entity, &recipe) {
        set_status(entity, 5, total_ticks(&recipe, entity));
        set_bool(host, pos, "lit", false);
        return;
    }
    let duration = total_ticks(&recipe, entity);
    set_i32(
        &mut entity.nbt,
        "CrackingTotal",
        duration.min(i32::MAX as u32) as i32,
    );
    if heat.temperature_c() < recipe.min_temp {
        set_status(entity, 6, duration);
        set_bool(host, pos, "lit", false);
        return;
    }
    let heat_cost = (recipe.heat_per_tick as f64
        * crate::speed_upgrade_multiplier(speed_count(entity)))
    .ceil() as u64;
    if stored < heat_cost {
        set_status(entity, 6, duration);
        set_bool(host, pos, "lit", false);
        return;
    }
    set_i32(
        &mut entity.nbt,
        "HeatHU",
        stored.saturating_sub(heat_cost).min(i32::MAX as u64) as i32,
    );
    let next = get_i32(&entity.nbt, "CrackingProgress").max(0) as u32 + 1;
    set_i32(
        &mut entity.nbt,
        "CrackingProgress",
        next.min(i32::MAX as u32) as i32,
    );
    set_status(entity, 0, duration);
    set_bool(host, pos, "lit", true);
    if next >= duration {
        crate::fluid::extract(&mut entity.nbt, FEED_TANK, &recipe.input, recipe.amount);
        if let Some((fluid, amount)) = &recipe.second_input {
            crate::fluid::extract(&mut entity.nbt, HYDROGEN_TANK, fluid, *amount);
        }
        for (index, product) in recipe.products.iter().enumerate() {
            crate::fluid::insert(
                &mut entity.nbt,
                OUTPUT_START + index,
                &product.fluid,
                product.amount,
                PRODUCT_CAPACITY,
            );
        }
        if recipe.catalyst_uses > 0 && catalyst_uses(entity) > 0 {
            let remaining = catalyst_uses(entity).saturating_sub(recipe.catalyst_uses);
            if remaining == 0 {
                let remove = entity
                    .items
                    .get(CATALYST_SLOT)
                    .and_then(Option::as_ref)
                    .is_some_and(|item| item.count <= 1);
                if remove {
                    entity.items[CATALYST_SLOT] = None;
                } else if let Some(item) =
                    entity.items.get_mut(CATALYST_SLOT).and_then(Option::as_mut)
                {
                    item.count -= 1;
                }
            }
            set_i32(&mut entity.nbt, "CrackingCatalystUses", remaining as i32);
        }
        set_i32(&mut entity.nbt, "CrackingProgress", 0);
        set_bool(host, pos, "lit", false);
    }
}
pub(crate) fn accepts(tank: usize, fluid: &str) -> bool {
    matches!(
        (tank, fluid),
        (FEED_TANK, HEAVY_OIL) | (HYDROGEN_TANK, HYDROGEN)
    )
}
pub(crate) fn fluid_tanks() -> ModFluidTanks {
    ModFluidTanks {
        tanks: vec![
            FEED_CAPACITY as i32,
            HYDROGEN_CAPACITY as i32,
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
        inputs: vec![FEED_TANK, HYDROGEN_TANK],
        outputs: vec![2, 3, 4, 5],
    }
}
pub(crate) fn container() -> ModContainer {
    ModContainer {
        slots: 2,
        faces: [
            vec![SPEED_SLOT, CATALYST_SLOT],
            vec![SPEED_SLOT, CATALYST_SLOT],
            vec![SPEED_SLOT, CATALYST_SLOT],
            vec![SPEED_SLOT, CATALYST_SLOT],
            vec![SPEED_SLOT, CATALYST_SLOT],
            vec![SPEED_SLOT, CATALYST_SLOT],
        ],
        inputs: vec![SPEED_SLOT, CATALYST_SLOT],
        outputs: vec![],
        menu: Some("arcforge:machine_2_slot".into()),
    }
}
pub(crate) fn item_allowed(slot: usize, item: &str, count: u32) -> bool {
    (slot == SPEED_SLOT && item == SPEED_UPGRADE && count <= 8)
        || (slot == CATALYST_SLOT && item == CATALYST)
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
        MAX_TEMP_C,
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
    if face_hit == [0, 1, 0] {
        host.change_mod_block_entity(pos, &mut |entity| {
            if entity.id == CONTROLLER {
                let next = if mode(entity) == "catalytic" {
                    "hydrocracking"
                } else {
                    "catalytic"
                };
                set_string(&mut entity.nbt, "CrackingMode", next);
                set_i32(&mut entity.nbt, "CrackingProgress", 0);
            }
        });
    } else if let Some(name) = match face_hit {
        [0, 0, -1] => Some("north"),
        [1, 0, 0] => Some("east"),
        [0, 0, 1] => Some("south"),
        [-1, 0, 0] => Some("west"),
        _ => None,
    } {
        if let Some(entity) = host.mod_block_entity(pos) {
            if formed(&entity) {
                if let Some(old_face) =
                    get_string(&entity.nbt, "CrackingFacing").and_then(|value| facing(&value))
                {
                    clear_box(host, pos, old_face);
                }
            }
        }
        host.set_block_property(pos, "facing", name);
        host.change_mod_block_entity(pos, &mut |entity| {
            if entity.id == CONTROLLER {
                set_string(&mut entity.nbt, "CrackingFacing", name);
                set_i32(&mut entity.nbt, "CrackingScan", 1);
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
    let stored = get_i32(&entity.nbt, "HeatHU").max(0) as u64;
    let temperature =
        crate::heat::HeatStore::new(HEAT_CAPACITY, MAX_TEMP_C, stored).temperature_c();
    let feed = crate::fluid::amount(&entity.nbt, FEED_TANK);
    let hydrogen = crate::fluid::amount(&entity.nbt, HYDROGEN_TANK);
    let state = match get_i32(&entity.nbt, "CrackingStatus") {
        1 => "needs a formed 3x3x5 casing",
        2 => "no recipe for the selected mode",
        3 => "needs more Heavy Oil or Hydrogen",
        4 => "needs Cracking Catalyst",
        5 => "product tanks full",
        6 => "heating or needs more HU",
        _ => "cracking",
    };
    format!(
        "Cracking Unit ({}) {state} | Heavy Oil {feed}mB, H2 {hydrogen}mB | {}°C | Heat {stored}/{HEAT_CAPACITY} HU | {}/{} ticks",
        mode(entity),
        temperature,
        get_i32(&entity.nbt, "CrackingProgress").max(0),
        get_i32(&entity.nbt, "CrackingTotal").max(0)
    )
}
