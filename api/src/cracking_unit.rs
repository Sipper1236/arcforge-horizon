//! Arcforge Cracking Unit: a 3x3x5 refinery box for catalytic and hydrogen cracking.
use horizon_client::{
    containers::{ModBlockEntity, ModContainer},
    mod_api::{Compound, Host, ModFluidTanks, NativeScreen, Tag, serde_json::Value},
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
const MODE_BUTTON_CATALYTIC: i32 = 420;
const MODE_BUTTON_HYDROCRACKING: i32 = 421;
const MODE_BUTTONS: [(i32, &'static str, f32); 2] = [
    (MODE_BUTTON_CATALYTIC, "catalytic", 40.0),
    (MODE_BUTTON_HYDROCRACKING, "hydrocracking", 64.0),
];
const MODE_BUTTON_Y: f32 = 17.0;
const MODE_BUTTON_SIZE: f32 = 20.0;
const SCREEN_HEIGHT: f32 = 254.0;
const SCREEN_INVENTORY_Y: f32 = 168.0;

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
fn mode_button_at(at: [f32; 2]) -> Option<i32> {
    MODE_BUTTONS
        .iter()
        .find(|(_, _, x)| {
            at[0] >= *x
                && at[0] < *x + MODE_BUTTON_SIZE
                && at[1] >= MODE_BUTTON_Y
                && at[1] < MODE_BUTTON_Y + MODE_BUTTON_SIZE
        })
        .map(|(id, _, _)| *id)
}
pub(crate) fn screen_click(at: [f32; 2], right: bool) -> Option<i32> {
    if right { None } else { mode_button_at(at) }
}
pub(crate) fn screen_tooltip(at: [f32; 2], lines: &mut Vec<String>) {
    match mode_button_at(at) {
        Some(MODE_BUTTON_CATALYTIC) => {
            lines.push("Catalytic cracking".to_owned());
            lines.push("Consumes Cracking Catalyst; no Hydrogen input".to_owned());
        }
        Some(MODE_BUTTON_HYDROCRACKING) => {
            lines.push("Hydrocracking".to_owned());
            lines.push("Consumes Hydrogen; no catalyst required".to_owned());
        }
        _ => {}
    }
}
pub(crate) fn select_mode(entity: &mut ModBlockEntity, button: i32) {
    if entity.id != CONTROLLER {
        return;
    }
    let Some((_, selected, _)) = MODE_BUTTONS.iter().find(|(id, _, _)| *id == button) else {
        return;
    };
    if mode(entity) != *selected {
        set_string(&mut entity.nbt, "CrackingMode", selected);
        set_i32(&mut entity.nbt, "CrackingProgress", 0);
        set_i32(&mut entity.nbt, "CrackingTotal", 0);
    }
}
pub(crate) fn screen() -> NativeScreen {
    NativeScreen {
        width: 176.0,
        height: SCREEN_HEIGHT,
        slots: vec![Some([16.0, 52.0]), Some([40.0, 52.0])],
        inventory_y: SCREEN_INVENTORY_Y,
        inventory_x: 8.0,
        own_labels: true,
        own_background: false,
    }
}
fn screen_status(status: i32) -> &'static str {
    match status {
        1 => "Structure incomplete",
        2 => "No recipe for selected mode",
        3 => "Waiting for recipe inputs",
        4 => "Needs Cracking Catalyst",
        5 => "Product tanks full",
        6 => "Heating / needs more heat",
        _ => "Processing",
    }
}
pub(crate) fn draw_screen(
    host: &dyn Host,
    event: &mut horizon_client::mod_api::ScreenDrawEvent<'_>,
) {
    let [ox, oy] = event.origin;
    let data = event.data;
    let mode_index = data.first().copied().unwrap_or_default();
    let progress = data.get(1).copied().unwrap_or_default().max(0);
    let duration = data.get(2).copied().unwrap_or_default().max(0);
    let temperature = data.get(3).copied().unwrap_or_default().max(0);
    let heat_tens = data.get(4).copied().unwrap_or_default().max(0);
    let status = data.get(5).copied().unwrap_or(1);
    let minimum_temperature = data.get(6).copied().unwrap_or_default().max(0);
    let feed = data.get(7).copied().unwrap_or_default().max(0);
    let hydrogen = data.get(8).copied().unwrap_or_default().max(0);
    let naphtha = data.get(9).copied().unwrap_or_default().max(0);
    let diesel = data.get(10).copied().unwrap_or_default().max(0);
    let third_product = data.get(11).copied().unwrap_or_default().max(0);
    let light_oil = data.get(12).copied().unwrap_or_default().max(0);
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

    text!("Cracking Unit", 8.0, 6.0, 0xFFFFFFFF);
    text!("Mode", 8.0, 20.0, 0xFFBBBBBB);
    let selected_mode = if mode_index == 1 {
        "hydrocracking"
    } else {
        "catalytic"
    };
    for (_, button_mode, x) in MODE_BUTTONS {
        let selected = button_mode == selected_mode;
        let button_sprite = if selected {
            "arcforge:gui/sprites/widget/arcforge/button_pressed"
        } else {
            "arcforge:gui/sprites/widget/arcforge/button"
        };
        let _ = host.sprite(
            batch,
            button_sprite,
            [0.0, 0.0, MODE_BUTTON_SIZE, MODE_BUTTON_SIZE],
            [
                ox + x,
                oy + MODE_BUTTON_Y,
                MODE_BUTTON_SIZE,
                MODE_BUTTON_SIZE,
            ],
        );
        let icon = if button_mode == "hydrocracking" {
            "arcforge:gui/sprites/widget/arcforge/cracking_hydrocracking"
        } else {
            "arcforge:gui/sprites/widget/arcforge/cracking_catalytic"
        };
        let offset = if selected { 3.0 } else { 2.0 };
        let _ = host.sprite(
            batch,
            icon,
            [0.0, 0.0, 16.0, 16.0],
            [ox + x + offset, oy + MODE_BUTTON_Y + offset, 16.0, 16.0],
        );
        if selected {
            fill!(
                [x - 1.0, MODE_BUTTON_Y - 1.0, MODE_BUTTON_SIZE + 2.0, 1.0],
                0xFF62B64F
            );
            fill!(
                [
                    x - 1.0,
                    MODE_BUTTON_Y + MODE_BUTTON_SIZE,
                    MODE_BUTTON_SIZE + 2.0,
                    1.0
                ],
                0xFF62B64F
            );
            fill!([x - 1.0, MODE_BUTTON_Y, 1.0, MODE_BUTTON_SIZE], 0xFF62B64F);
            fill!(
                [x + MODE_BUTTON_SIZE, MODE_BUTTON_Y, 1.0, MODE_BUTTON_SIZE],
                0xFF62B64F
            );
        }
    }
    text!(
        if selected_mode == "hydrocracking" {
            "Hydrocracking"
        } else {
            "Catalytic"
        },
        90.0,
        22.0,
        0xFFE0E0E0,
    );
    text!("Speed", 8.0, 41.0, 0xFFBBBBBB);
    text!("Catalyst", 40.0, 41.0, 0xFFBBBBBB);
    text!(screen_status(status), 8.0, 70.0, 0xFFE0E0E0);
    text!(&format!("Oil {feed}/32k mB"), 8.0, 82.0, 0xFFE0E0E0);
    text!(&format!("H2 {hydrogen}/16k mB"), 90.0, 82.0, 0xFFE0E0E0,);
    text!(
        &format!("Naphtha {:.1}k", naphtha as f32 / 1_000.0),
        8.0,
        94.0,
        0xFFE0E0E0,
    );
    text!(
        &format!("Diesel {:.1}k", diesel as f32 / 1_000.0),
        90.0,
        94.0,
        0xFFE0E0E0,
    );
    text!(
        &format!(
            "{} {:.1}k",
            if selected_mode == "hydrocracking" {
                "Kerosene"
            } else {
                "Refinery Gas"
            },
            third_product as f32 / 1_000.0
        ),
        8.0,
        105.0,
        0xFFE0E0E0,
    );
    text!(
        &format!("Light Oil {:.1}k", light_oil as f32 / 1_000.0),
        90.0,
        105.0,
        0xFFE0E0E0,
    );
    text!(
        &format!("Temp {temperature} C / {minimum_temperature} C"),
        8.0,
        116.0,
        0xFFE0E0E0,
    );
    text!(
        &format!(
            "Heat {:.1}/{:.0} kHU",
            heat_tens as f32 / 100.0,
            HEAT_CAPACITY as f32 / 1_000.0
        ),
        8.0,
        127.0,
        0xFFE0E0E0,
    );
    if duration > 0 {
        text!(
            &format!("Progress {progress}/{duration} t"),
            8.0,
            138.0,
            0xFFE0E0E0,
        );
        fill!([8.0, 149.0, 160.0, 5.0], 0xFF383838);
        let fraction = (progress as f32 / duration as f32).clamp(0.0, 1.0);
        fill!([8.0, 149.0, 160.0 * fraction, 5.0], 0xFF62B64F);
    } else {
        text!("Progress 0/0 t", 8.0, 138.0, 0xFFAAAAAA);
    }
    text!("Inventory", 8.0, 157.0, 0xFFFFFFFF);
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
fn tick_machine(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
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
fn screen_number(value: i32) -> i32 {
    value.clamp(0, i16::MAX as i32)
}
fn sync_screen_data(host: &dyn Host, entity: &mut ModBlockEntity) {
    let heat = crate::heat::HeatStore::new(
        HEAT_CAPACITY,
        MAX_TEMP_C,
        get_i32(&entity.nbt, "HeatHU").max(0) as u64,
    );
    let temperature = heat.temperature_c();
    let stored = heat.stored_hu();
    let minimum_temperature = find_recipe(host, mode(entity)).map_or(0, |recipe| recipe.min_temp);
    entity.data.resize(13, 0);
    entity.data[0] = if mode(entity) == "hydrocracking" {
        1
    } else {
        0
    };
    entity.data[1] = screen_number(get_i32(&entity.nbt, "CrackingProgress"));
    entity.data[2] = screen_number(get_i32(&entity.nbt, "CrackingTotal"));
    entity.data[3] = screen_number(temperature as i32);
    entity.data[4] = screen_number((stored / 10).min(i32::MAX as u64) as i32);
    entity.data[5] = screen_number(get_i32(&entity.nbt, "CrackingStatus"));
    entity.data[6] = screen_number(minimum_temperature.min(i32::MAX as u32) as i32);
    entity.data[7] = screen_number(crate::fluid::amount(&entity.nbt, FEED_TANK) as i32);
    entity.data[8] = screen_number(crate::fluid::amount(&entity.nbt, HYDROGEN_TANK) as i32);
    for index in 0..4 {
        entity.data[9 + index] =
            screen_number(crate::fluid::amount(&entity.nbt, OUTPUT_START + index) as i32);
    }
}
pub(crate) fn tick(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    tick_machine(host, pos, entity);
    sync_screen_data(host, entity);
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
        menu: Some("arcforge:cracking_unit".into()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashSet, path::Path, sync::Arc};

    fn test_entity() -> ModBlockEntity {
        ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
            id: CONTROLLER.to_owned(),
            ..Default::default()
        })
    }

    fn source_recipe(filename: &str) -> Recipe {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../content/data/arcforge/recipe/cracking")
            .join(filename);
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|error| panic!("could not read {}: {error}", path.display()));
        let json = horizon_client::mod_api::serde_json::from_slice::<Value>(&bytes)
            .unwrap_or_else(|error| panic!("could not parse {}: {error}", path.display()));
        recipe_from_json(&json)
            .unwrap_or_else(|| panic!("invalid cracking recipe {}", path.display()))
    }

    #[test]
    fn source_recipes_keep_both_modes_and_their_exact_costs_and_products() {
        let catalytic = source_recipe("heavy_oil.json");
        assert_eq!(catalytic.mode, "catalytic");
        assert_eq!(catalytic.input, HEAVY_OIL);
        assert_eq!(catalytic.amount, 100);
        assert_eq!(catalytic.second_input, None);
        assert_eq!(catalytic.catalyst.as_deref(), Some(CATALYST));
        assert_eq!(catalytic.catalyst_uses, MAX_CATALYST_USES);
        assert_eq!(catalytic.ticks, 20);
        assert_eq!(catalytic.heat_per_tick, 30);
        assert_eq!(catalytic.min_temp, 480);
        assert_eq!(
            catalytic
                .products
                .iter()
                .map(|product| (product.fluid.as_str(), product.amount))
                .collect::<Vec<_>>(),
            [
                ("arcforge:naphtha", 40),
                ("arcforge:diesel", 40),
                ("arcforge:refinery_gas", 55),
                ("arcforge:light_oil", 10),
            ]
        );

        let hydrocracking = source_recipe("heavy_oil_hydrocracking.json");
        assert_eq!(hydrocracking.mode, "hydrocracking");
        assert_eq!(hydrocracking.input, HEAVY_OIL);
        assert_eq!(hydrocracking.amount, 100);
        assert_eq!(hydrocracking.second_input, Some((HYDROGEN.to_owned(), 30)));
        assert_eq!(hydrocracking.catalyst, None);
        assert_eq!(hydrocracking.catalyst_uses, 0);
        assert_eq!(hydrocracking.ticks, 20);
        assert_eq!(hydrocracking.heat_per_tick, 36);
        assert_eq!(hydrocracking.min_temp, 520);
        assert_eq!(
            hydrocracking
                .products
                .iter()
                .map(|product| (product.fluid.as_str(), product.amount))
                .collect::<Vec<_>>(),
            [
                ("arcforge:naphtha", 10),
                ("arcforge:diesel", 50),
                ("arcforge:kerosene", 30),
                ("arcforge:light_oil", 10),
            ]
        );
    }

    #[test]
    fn cracking_box_has_45_unique_positions_in_each_horizontal_facing() {
        let controller = [10, 20, 30];
        for (name, expected_x, expected_z) in [
            ("north", (9, 11), (30, 32)),
            ("east", (8, 10), (29, 31)),
            ("south", (9, 11), (28, 30)),
            ("west", (10, 12), (29, 31)),
        ] {
            let positions = box_positions(controller, facing(name).expect("valid facing"));
            let unique = positions.iter().copied().collect::<HashSet<_>>();
            assert_eq!(positions.len(), 45, "{name}");
            assert_eq!(unique.len(), 45, "{name}");
            assert!(unique.contains(&controller), "{name}");
            assert!(
                unique.iter().all(|pos| (20..=24).contains(&pos[1])),
                "{name}"
            );
            assert_eq!(
                (
                    unique.iter().map(|pos| pos[0]).min(),
                    unique.iter().map(|pos| pos[0]).max()
                ),
                (Some(expected_x.0), Some(expected_x.1)),
                "{name} x extent"
            );
            assert_eq!(
                (
                    unique.iter().map(|pos| pos[2]).min(),
                    unique.iter().map(|pos| pos[2]).max()
                ),
                (Some(expected_z.0), Some(expected_z.1)),
                "{name} z extent"
            );
        }
        assert!(facing("up").is_none());
    }

    #[test]
    fn fluid_tanks_accept_only_the_two_recipe_inputs() {
        assert!(accepts(FEED_TANK, HEAVY_OIL));
        assert!(accepts(HYDROGEN_TANK, HYDROGEN));
        assert!(!accepts(FEED_TANK, HYDROGEN));
        assert!(!accepts(HYDROGEN_TANK, HEAVY_OIL));
        assert!(!accepts(OUTPUT_START, HEAVY_OIL));

        let tanks = fluid_tanks();
        assert_eq!(
            tanks.tanks,
            vec![32_000, 16_000, 16_000, 16_000, 16_000, 16_000]
        );
        assert_eq!(tanks.inputs, vec![FEED_TANK, HYDROGEN_TANK]);
        assert_eq!(tanks.outputs, vec![2, 3, 4, 5]);
    }

    #[test]
    fn product_backpressure_rejects_mixed_or_full_output_tanks() {
        let recipe = source_recipe("heavy_oil.json");
        let mut entity = test_entity();
        assert!(products_fit(&entity, &recipe));

        crate::fluid::insert(
            &mut entity.nbt,
            OUTPUT_START,
            "arcforge:diesel",
            1,
            PRODUCT_CAPACITY,
        );
        assert!(!products_fit(&entity, &recipe));

        entity.nbt.clear();
        crate::fluid::insert(
            &mut entity.nbt,
            OUTPUT_START,
            "arcforge:naphtha",
            PRODUCT_CAPACITY - 40,
            PRODUCT_CAPACITY,
        );
        assert!(products_fit(&entity, &recipe));
        crate::fluid::insert(
            &mut entity.nbt,
            OUTPUT_START,
            "arcforge:naphtha",
            1,
            PRODUCT_CAPACITY + 1,
        );
        assert!(!products_fit(&entity, &recipe));
    }

    #[test]
    fn catalyst_wear_is_bounded_and_speed_slot_respects_its_eight_card_cap() {
        let mut entity = test_entity();
        entity.items.resize(2, None);
        entity.items[CATALYST_SLOT] = Some(horizon_client::mod_api::ItemStack {
            item: Arc::from(CATALYST),
            count: 1,
            damage: 0,
            extra: None,
        });
        set_i32(&mut entity.nbt, "CrackingCatalystUses", 27);
        assert_eq!(catalyst_uses(&entity), 27);
        set_i32(&mut entity.nbt, "CrackingCatalystUses", -1);
        assert_eq!(catalyst_uses(&entity), 0);
        set_i32(&mut entity.nbt, "CrackingCatalystUses", 100);
        assert_eq!(catalyst_uses(&entity), MAX_CATALYST_USES);

        assert!(item_allowed(SPEED_SLOT, SPEED_UPGRADE, 8));
        assert!(!item_allowed(SPEED_SLOT, SPEED_UPGRADE, 9));
        assert!(item_allowed(CATALYST_SLOT, CATALYST, 64));
        assert!(!item_allowed(CATALYST_SLOT, SPEED_UPGRADE, 1));
        assert!(!item_allowed(SPEED_SLOT, CATALYST, 1));

        entity.items[SPEED_SLOT] = Some(horizon_client::mod_api::ItemStack {
            item: Arc::from(SPEED_UPGRADE),
            count: 2,
            damage: 0,
            extra: None,
        });
        assert_eq!(total_ticks(&source_recipe("heavy_oil.json"), &entity), 10);
    }

    #[test]
    fn native_screen_reserves_two_upgrade_slots_and_inventory_space() {
        let screen = screen();
        assert_eq!(screen.width, 176.0);
        assert_eq!(screen.height, SCREEN_HEIGHT);
        assert_eq!(screen.slots, vec![Some([16.0, 52.0]), Some([40.0, 52.0])]);
        assert_eq!(screen.inventory_y, SCREEN_INVENTORY_Y);
        assert!(screen.own_labels);
        assert!(!screen.own_background);
    }

    #[test]
    fn mode_buttons_accept_left_clicks_only_inside_their_bounds() {
        assert_eq!(
            screen_click([40.0, MODE_BUTTON_Y], false),
            Some(MODE_BUTTON_CATALYTIC)
        );
        assert_eq!(
            screen_click([83.99, MODE_BUTTON_Y + 19.99], false),
            Some(MODE_BUTTON_HYDROCRACKING)
        );
        assert_eq!(screen_click([60.0, MODE_BUTTON_Y], false), None);
        assert_eq!(screen_click([39.99, MODE_BUTTON_Y], false), None);
        assert_eq!(screen_click([40.0, MODE_BUTTON_Y], true), None);
    }

    #[test]
    fn mode_button_selection_changes_mode_and_resets_only_changed_recipe_progress() {
        let mut entity = test_entity();
        set_string(&mut entity.nbt, "CrackingMode", "hydrocracking");
        set_i32(&mut entity.nbt, "CrackingProgress", 12);
        set_i32(&mut entity.nbt, "CrackingTotal", 20);

        select_mode(&mut entity, MODE_BUTTON_CATALYTIC);
        assert_eq!(mode(&entity), "catalytic");
        assert_eq!(get_i32(&entity.nbt, "CrackingProgress"), 0);
        assert_eq!(get_i32(&entity.nbt, "CrackingTotal"), 0);

        set_i32(&mut entity.nbt, "CrackingProgress", 7);
        select_mode(&mut entity, MODE_BUTTON_CATALYTIC);
        assert_eq!(get_i32(&entity.nbt, "CrackingProgress"), 7);

        select_mode(&mut entity, MODE_BUTTON_HYDROCRACKING);
        assert_eq!(mode(&entity), "hydrocracking");
        assert_eq!(get_i32(&entity.nbt, "CrackingProgress"), 0);

        set_i32(&mut entity.nbt, "CrackingProgress", 9);
        select_mode(&mut entity, -1);
        assert_eq!(mode(&entity), "hydrocracking");
        assert_eq!(get_i32(&entity.nbt, "CrackingProgress"), 9);
    }

    #[test]
    fn screen_data_values_fit_horizons_signed_sixteen_bit_transport() {
        assert_eq!(screen_number(-1), 0);
        assert_eq!(screen_number(32_000), 32_000);
        assert_eq!(screen_number(90_000), i16::MAX as i32);
    }
}
