//! Arcforge Electrolyzer: splits water, brine, and hydrofluoric acid with FE.
use horizon_client::{
    containers::{ModBlockEntity, ModContainer},
    mod_api::{Compound, Direction, Host, ModEnergyBuffer, ModFluidTanks, Tag, serde_json::Value},
};

pub(crate) const BLOCK: &str = "arcforge:electrolyzer";

const WATER_TANK: usize = 0;
const HYDROGEN_TANK: usize = 1;
const SECONDARY_GAS_TANK: usize = 2;
const LIQUID_TANK: usize = 3;
const WATER_CAPACITY: u32 = 8_000;
const GAS_CAPACITY: u32 = 16_000;
const LIQUID_CAPACITY: u32 = 8_000;
const ENERGY_CAPACITY: u64 = 100_000;
const MAX_ENERGY_INPUT: u64 = 8_000;
const BASE_ENERGY_RATE: u64 = 400;
const DEFAULT_ENERGY_PER_MB: u64 = 1_200;
const HYDROGEN_FLOOR_PER_MB: u64 = 708;
const OUTPUT_RATE: i32 = 1_000;
const SPEED_SLOT: usize = 0;
const ENERGY_SLOT: usize = 1;
const SPEED_UPGRADE: &str = "arcforge:speed_upgrade";
const ENERGY_UPGRADE: &str = "arcforge:energy_upgrade";
const RECIPE_TYPE: &str = "arcforge:electrolyzing";

struct Input {
    fluid: Option<String>,
    tag: Option<String>,
    amount: u32,
}

struct Product {
    fluid: String,
    amount: u32,
}

struct Recipe {
    input: Input,
    hydrogen: Product,
    secondary: Option<Product>,
    liquid: Option<Product>,
    energy: u64,
}

fn positive(value: &Value) -> Option<u32> {
    value
        .as_u64()
        .filter(|amount| *amount > 0)
        .map(|amount| amount.min(u64::from(u32::MAX)) as u32)
}

fn product(value: &Value) -> Option<Product> {
    Some(Product {
        fluid: value.get("id")?.as_str()?.to_owned(),
        amount: positive(value.get("amount")?)?,
    })
}

fn recipe_from_json(json: &Value) -> Option<Recipe> {
    let source = json.get("input")?;
    let input = Input {
        fluid: source
            .get("fluid")
            .and_then(Value::as_str)
            .map(str::to_owned),
        tag: source.get("tag").and_then(Value::as_str).map(str::to_owned),
        amount: positive(source.get("amount")?)?,
    };
    if input.fluid.is_none() && input.tag.as_deref() != Some("minecraft:water") {
        return None;
    }
    let hydrogen = product(json.get("primary")?)?;
    let secondary = json.get("secondary").and_then(product);
    let liquid = json.get("tertiary").and_then(product);
    let base_energy = json
        .get("energy")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| u64::from(input.amount).saturating_mul(DEFAULT_ENERGY_PER_MB));
    let energy_floor = u64::from(hydrogen.amount).saturating_mul(HYDROGEN_FLOOR_PER_MB);
    Some(Recipe {
        input,
        hydrogen,
        secondary,
        liquid,
        energy: base_energy.max(energy_floor).min(i32::MAX as u64),
    })
}

fn matches_fluid(input: &Input, fluid: &str) -> bool {
    input.fluid.as_deref() == Some(fluid)
        || (input.tag.as_deref() == Some("minecraft:water") && fluid == "minecraft:water")
}

fn matching_recipes(host: &dyn Host) -> Vec<(Value, Recipe)> {
    host.recipes_of_type(RECIPE_TYPE)
        .into_iter()
        .filter_map(|source| {
            recipe_from_json(&source.json).map(|recipe| (source.json.as_ref().clone(), recipe))
        })
        .collect()
}

fn find_recipe(host: &dyn Host, entity: &ModBlockEntity) -> Option<(Recipe, String)> {
    let fluid = crate::fluid::name(&entity.nbt, WATER_TANK)?;
    let stored = crate::fluid::amount(&entity.nbt, WATER_TANK);
    matching_recipes(host)
        .into_iter()
        .find(|(_, recipe)| matches_fluid(&recipe.input, &fluid) && stored >= recipe.input.amount)
        .map(|(json, recipe)| (recipe, json.to_string()))
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

fn vent_enabled(entity: &ModBlockEntity, key: &str) -> bool {
    let Tag::String(key) = Tag::String(key.into()) else {
        unreachable!()
    };
    matches!(entity.nbt.get(&key), Some(Tag::Byte(value)) if *value != 0)
}

fn set_vent(entity: &mut ModBlockEntity, key: &str, enabled: bool) {
    let Tag::String(key) = Tag::String(key.into()) else {
        unreachable!()
    };
    entity
        .nbt
        .insert(key, Tag::Byte(if enabled { 1 } else { 0 }));
}

fn upgrade_count(entity: &ModBlockEntity, slot: usize, item: &str) -> u32 {
    entity
        .items
        .get(slot)
        .and_then(Option::as_ref)
        .filter(|stack| stack.item.as_ref() == item)
        .map_or(0, |stack| stack.count.min(8))
}

fn set_lit(host: &mut dyn Host, pos: [i32; 3], lit: bool) {
    let value = if lit { "true" } else { "false" };
    if host.block_property(pos, "lit").as_deref() != Some(value) {
        host.set_block_property(pos, "lit", value);
    }
}

fn progress(entity: &ModBlockEntity) -> u32 {
    get_i32(&entity.nbt, "ElectrolyzerSpent").max(0) as u32
}

fn product_fits(
    entity: &ModBlockEntity,
    tank: usize,
    product: &Product,
    capacity: u32,
    vent: bool,
) -> bool {
    vent || crate::fluid::can_insert(&entity.nbt, tank, &product.fluid, product.amount, capacity)
}

fn outputs_fit(entity: &ModBlockEntity, recipe: &Recipe) -> bool {
    product_fits(
        entity,
        HYDROGEN_TANK,
        &recipe.hydrogen,
        GAS_CAPACITY,
        vent_enabled(entity, "ElectrolyzerVentHydrogen"),
    ) && recipe.secondary.as_ref().is_none_or(|product| {
        product_fits(
            entity,
            SECONDARY_GAS_TANK,
            product,
            GAS_CAPACITY,
            vent_enabled(entity, "ElectrolyzerVentSecondary"),
        )
    }) && recipe
        .liquid
        .as_ref()
        .is_none_or(|product| product_fits(entity, LIQUID_TANK, product, LIQUID_CAPACITY, false))
}

fn insert_product(
    entity: &mut ModBlockEntity,
    tank: usize,
    product: &Product,
    capacity: u32,
    vent: bool,
) {
    if vent {
        let current_fluid = crate::fluid::name(&entity.nbt, tank);
        let stored = crate::fluid::amount(&entity.nbt, tank);
        if current_fluid
            .as_deref()
            .is_some_and(|fluid| fluid != product.fluid)
        {
            return;
        }
        let remaining = capacity.saturating_sub(stored);
        let inserted = product.amount.min(remaining);
        if inserted > 0 {
            crate::fluid::insert(&mut entity.nbt, tank, &product.fluid, inserted, capacity);
        }
    } else {
        crate::fluid::insert(
            &mut entity.nbt,
            tank,
            &product.fluid,
            product.amount,
            capacity,
        );
    }
}

fn push_output(host: &mut dyn Host, pos: [i32; 3], face: Direction) {
    let dimension = host.dimension().to_owned();
    let [dx, dy, dz] = face.normal();
    let neighbor = [pos[0] + dx, pos[1] + dy, pos[2] + dz];
    let _ = host.move_fluids(
        &dimension,
        pos,
        face,
        neighbor,
        face.opposite(),
        OUTPUT_RATE,
    );
}

fn push_outputs(host: &mut dyn Host, pos: [i32; 3]) {
    push_output(host, pos, Direction::West);
    push_output(host, pos, Direction::East);
    push_output(host, pos, Direction::Down);
}

fn sync_screen(entity: &mut ModBlockEntity, rate: u64, total: u64, state: i32) {
    entity.data.resize(9, 0);
    entity.data[0] = get_i32(&entity.nbt, "energy").max(0);
    entity.data[1] = progress(entity).min(i32::MAX as u32) as i32;
    entity.data[2] = total.min(i32::MAX as u64) as i32;
    entity.data[3] = crate::fluid::amount(&entity.nbt, WATER_TANK).min(i32::MAX as u32) as i32;
    entity.data[4] = crate::fluid::amount(&entity.nbt, HYDROGEN_TANK).min(i32::MAX as u32) as i32;
    entity.data[5] =
        crate::fluid::amount(&entity.nbt, SECONDARY_GAS_TANK).min(i32::MAX as u32) as i32;
    entity.data[6] = crate::fluid::amount(&entity.nbt, LIQUID_TANK).min(i32::MAX as u32) as i32;
    entity.data[7] = state;
    entity.data[8] = (if vent_enabled(entity, "ElectrolyzerVentHydrogen") {
        1
    } else {
        0
    }) | ((if vent_enabled(entity, "ElectrolyzerVentSecondary") {
        1
    } else {
        0
    }) << 1);
    set_i32(
        &mut entity.nbt,
        "ElectrolyzerRate",
        rate.min(i32::MAX as u64) as i32,
    );
    set_i32(
        &mut entity.nbt,
        "ElectrolyzerTotal",
        total.min(i32::MAX as u64) as i32,
    );
}

fn total_cost(recipe: &Recipe, energy_upgrades: u32) -> u64 {
    crate::scaled_upgrade_cost(recipe.energy, 1.0, energy_upgrades)
        .max(u64::from(recipe.hydrogen.amount).saturating_mul(HYDROGEN_FLOOR_PER_MB))
        .min(i32::MAX as u64)
}

fn has_matching_fluid(host: &dyn Host, entity: &ModBlockEntity) -> bool {
    let Some(fluid) = crate::fluid::name(&entity.nbt, WATER_TANK) else {
        return false;
    };
    matching_recipes(host)
        .iter()
        .any(|(_, recipe)| matches_fluid(&recipe.input, &fluid))
}

pub(crate) fn tick(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    entity.items.resize(2, None);
    let energy = (get_i32(&entity.nbt, "energy").max(0) as u64).min(ENERGY_CAPACITY);
    set_i32(&mut entity.nbt, "energy", energy as i32);
    push_outputs(host, pos);

    let Some((recipe, signature)) = find_recipe(host, entity) else {
        set_i32(&mut entity.nbt, "ElectrolyzerSpent", 0);
        set_string(&mut entity.nbt, "ElectrolyzerRecipe", "");
        let state = if crate::fluid::amount(&entity.nbt, WATER_TANK) == 0 {
            1
        } else if !has_matching_fluid(host, entity) {
            2
        } else {
            5
        };
        sync_screen(entity, 0, 0, state);
        set_lit(host, pos, false);
        return;
    };

    if get_string(&entity.nbt, "ElectrolyzerRecipe").as_deref() != Some(signature.as_str()) {
        set_i32(&mut entity.nbt, "ElectrolyzerSpent", 0);
        set_string(&mut entity.nbt, "ElectrolyzerRecipe", &signature);
    }
    let speed_upgrades = upgrade_count(entity, SPEED_SLOT, SPEED_UPGRADE);
    let energy_upgrades = upgrade_count(entity, ENERGY_SLOT, ENERGY_UPGRADE);
    let rate = (BASE_ENERGY_RATE as f64 * crate::speed_upgrade_multiplier(speed_upgrades))
        .ceil()
        .clamp(1.0, MAX_ENERGY_INPUT as f64) as u64;
    let total = total_cost(&recipe, energy_upgrades);
    if !outputs_fit(entity, &recipe) {
        sync_screen(entity, 0, total, 3);
        set_lit(host, pos, false);
        return;
    }

    let spent = u64::from(progress(entity)).min(total);
    let remaining = total.saturating_sub(spent);
    let draw = rate.min(remaining).min(energy);
    if draw == 0 {
        sync_screen(entity, 0, total, 4);
        set_lit(host, pos, false);
        return;
    }
    set_i32(
        &mut entity.nbt,
        "energy",
        energy.saturating_sub(draw) as i32,
    );
    let spent = spent.saturating_add(draw);
    set_i32(
        &mut entity.nbt,
        "ElectrolyzerSpent",
        spent.min(i32::MAX as u64) as i32,
    );
    sync_screen(entity, draw, total, 0);
    set_lit(host, pos, true);

    if spent >= total {
        let vent_hydrogen = vent_enabled(entity, "ElectrolyzerVentHydrogen");
        let vent_secondary = vent_enabled(entity, "ElectrolyzerVentSecondary");
        crate::fluid::extract(
            &mut entity.nbt,
            WATER_TANK,
            &recipe
                .input
                .fluid
                .unwrap_or_else(|| "minecraft:water".to_owned()),
            recipe.input.amount,
        );
        insert_product(
            entity,
            HYDROGEN_TANK,
            &recipe.hydrogen,
            GAS_CAPACITY,
            vent_hydrogen,
        );
        if let Some(product) = &recipe.secondary {
            insert_product(
                entity,
                SECONDARY_GAS_TANK,
                product,
                GAS_CAPACITY,
                vent_secondary,
            );
        }
        if let Some(product) = &recipe.liquid {
            insert_product(entity, LIQUID_TANK, product, LIQUID_CAPACITY, false);
        }
        set_i32(&mut entity.nbt, "ElectrolyzerSpent", 0);
        sync_screen(entity, 0, total, 0);
        set_lit(host, pos, false);
    }
}

pub(crate) fn accepts(host: &dyn Host, tank: usize, fluid: &str) -> bool {
    tank == WATER_TANK
        && matching_recipes(host)
            .iter()
            .any(|(_, recipe)| matches_fluid(&recipe.input, fluid))
}

pub(crate) fn item_allowed(slot: usize, item: &str, count: u32) -> bool {
    count <= 8
        && ((slot == SPEED_SLOT && item == SPEED_UPGRADE)
            || (slot == ENERGY_SLOT && item == ENERGY_UPGRADE))
}

pub(crate) fn fluid_tanks() -> ModFluidTanks {
    ModFluidTanks {
        tanks: vec![
            WATER_CAPACITY as i32,
            GAS_CAPACITY as i32,
            GAS_CAPACITY as i32,
            LIQUID_CAPACITY as i32,
        ],
        // Static north-facing ports follow Arcforge's default: water up, H2 west,
        // secondary gas east, lye down. FE enters from the back (south).
        faces: [
            vec![LIQUID_TANK],
            vec![WATER_TANK],
            vec![],
            vec![],
            vec![HYDROGEN_TANK],
            vec![SECONDARY_GAS_TANK],
        ],
        inputs: vec![WATER_TANK],
        outputs: vec![HYDROGEN_TANK, SECONDARY_GAS_TANK, LIQUID_TANK],
    }
}

pub(crate) fn energy_buffer() -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: ENERGY_CAPACITY as i32,
        max_insert: MAX_ENERGY_INPUT as i32,
        max_extract: 0,
        // Default north-facing machine: FE enters from the back (south).
        insert_faces: [false, false, false, true, false, false],
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

pub(crate) fn toggle_vent(host: &mut dyn Host, pos: [i32; 3], face: [i32; 3]) -> Option<bool> {
    let key = match face {
        [0, 1, 0] => "ElectrolyzerVentHydrogen",
        [0, -1, 0] => "ElectrolyzerVentSecondary",
        _ => return None,
    };
    let mut enabled = false;
    host.change_mod_block_entity(pos, &mut |entity| {
        if entity.id == BLOCK {
            enabled = !vent_enabled(entity, key);
            set_vent(entity, key, enabled);
        }
    });
    host.block_entity_sync(pos);
    Some(enabled)
}

pub(crate) fn status(entity: &ModBlockEntity) -> String {
    format!(
        "Electrolyzer: water {}/{} mB | H2 {}/{} mB (vent {}) | secondary {}/{} mB (vent {}) | lye {}/{} mB | FE {}/{}",
        crate::fluid::amount(&entity.nbt, WATER_TANK),
        WATER_CAPACITY,
        crate::fluid::amount(&entity.nbt, HYDROGEN_TANK),
        GAS_CAPACITY,
        if vent_enabled(entity, "ElectrolyzerVentHydrogen") {
            "on"
        } else {
            "off"
        },
        crate::fluid::amount(&entity.nbt, SECONDARY_GAS_TANK),
        GAS_CAPACITY,
        if vent_enabled(entity, "ElectrolyzerVentSecondary") {
            "on"
        } else {
            "off"
        },
        crate::fluid::amount(&entity.nbt, LIQUID_TANK),
        LIQUID_CAPACITY,
        get_i32(&entity.nbt, "energy").max(0),
        ENERGY_CAPACITY,
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

    let energy = data.first().copied().unwrap_or_default().max(0);
    let spent = data.get(1).copied().unwrap_or_default().max(0);
    let total = data.get(2).copied().unwrap_or_default().max(0);
    let water = data.get(3).copied().unwrap_or_default().max(0);
    let hydrogen = data.get(4).copied().unwrap_or_default().max(0);
    let secondary = data.get(5).copied().unwrap_or_default().max(0);
    let liquid = data.get(6).copied().unwrap_or_default().max(0);
    let state = match data.get(7).copied().unwrap_or_default() {
        1 => "Waiting for fluid input",
        2 => "No recipe for input fluid",
        3 => "Output full",
        4 => "Waiting for FE",
        5 => "Need more input fluid",
        _ if total > 0 => "Electrolyzing",
        _ => "Idle",
    };
    let vents = data.get(8).copied().unwrap_or_default();

    text!("Electrolyzer", 8.0, 6.0, 0xFFFFFFFF);
    text!("Water input", 8.0, 20.0, 0xFFBBBBBB);
    text!(
        &format!("{water}/{WATER_CAPACITY} mB"),
        8.0,
        33.0,
        0xFFE0E0E0
    );
    text!("Hydrogen", 8.0, 51.0, 0xFFBBBBBB);
    text!(
        &format!("{hydrogen}/{GAS_CAPACITY} mB"),
        8.0,
        64.0,
        0xFFE0E0E0
    );
    text!(
        if vents & 1 != 0 {
            "Vent: ON"
        } else {
            "Vent: OFF"
        },
        8.0,
        77.0,
        0xFFBBBBBB,
    );
    text!("Oxygen / other gas", 90.0, 51.0, 0xFFBBBBBB);
    text!(
        &format!("{secondary}/{GAS_CAPACITY} mB"),
        90.0,
        64.0,
        0xFFE0E0E0
    );
    text!(
        if vents & 2 != 0 {
            "Vent: ON"
        } else {
            "Vent: OFF"
        },
        90.0,
        77.0,
        0xFFBBBBBB,
    );
    text!("Lye output", 8.0, 91.0, 0xFFBBBBBB);
    text!(
        &format!("{liquid}/{LIQUID_CAPACITY} mB"),
        8.0,
        104.0,
        0xFFE0E0E0
    );
    text!(
        &format!("FE {energy}/{ENERGY_CAPACITY}"),
        8.0,
        121.0,
        0xFFE0E0E0,
    );
    text!(state, 8.0, 136.0, 0xFFBBBBBB);
    if total > 0 {
        text!(
            &format!("Energy progress {spent}/{total} FE"),
            8.0,
            150.0,
            0xFFE0E0E0,
        );
        fill!([8.0, 162.0, 160.0, 6.0], 0xFF383838);
        let fraction = (spent as f32 / total as f32).clamp(0.0, 1.0);
        fill!([8.0, 162.0, 160.0 * fraction, 6.0], 0xFF62B64F);
    }
    text!("Speed", 38.0, 184.0, 0xFFBBBBBB);
    text!("Energy", 122.0, 184.0, 0xFFBBBBBB);
    text!(
        "Use a wrench on top/bottom to toggle gas vents",
        8.0,
        198.0,
        0xFFBBBBBB
    );
    text!("Inventory", 8.0, 213.0, 0xFFFFFFFF);
}
