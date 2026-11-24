//! First-pass Arcforge Chemical Reactor for data-driven chemical_reacting recipes.
use horizon_client::{
    containers::{ModBlockEntity, ModContainer},
    mod_api::{Compound, Host, ItemStack, ModEnergyBuffer, ModFluidTanks, Tag, serde_json::Value},
};

pub(crate) const BLOCK: &str = "arcforge:chemical_reactor";
pub(crate) const NPK_FERTILIZER: &str = "arcforge:npk_fertilizer";

const TANK_CAPACITY: u32 = 8_000;
const ENERGY_CAPACITY: u64 = 40_000;
const MAX_ENERGY_INPUT: u64 = 1_000;
const HEAT_CAPACITY: u64 = 40_000;
const MAX_TEMPERATURE_C: u32 = 1_200;
const FLUID_INPUTS: [usize; 3] = [0, 1, 2];
const FLUID_OUTPUT: usize = 3;
const FLUID_BYPRODUCT: usize = 4;
const ITEM_INPUTS: [usize; 3] = [0, 3, 4];
const ITEM_OUTPUT: usize = 1;
const ITEM_BYPRODUCT: usize = 2;
const SPEED_SLOT: usize = 5;
const ENERGY_SLOT: usize = 6;
const SPEED_UPGRADE: &str = "arcforge:speed_upgrade";
const ENERGY_UPGRADE: &str = "arcforge:energy_upgrade";

#[derive(Clone)]
struct ItemInput {
    ingredient: Value,
    count: u32,
}

#[derive(Clone)]
struct FluidInput {
    fluid: String,
    amount: u32,
}

#[derive(Clone)]
struct ItemProduct {
    item: String,
    count: u32,
}

#[derive(Clone)]
struct FluidProduct {
    fluid: String,
    amount: u32,
}

struct Recipe {
    item_inputs: Vec<ItemInput>,
    fluid_inputs: Vec<FluidInput>,
    item_output: Option<ItemProduct>,
    fluid_output: Option<FluidProduct>,
    item_byproduct: Option<ItemProduct>,
    fluid_byproduct: Option<FluidProduct>,
    ticks: u32,
    energy_per_tick: u32,
    heat_per_tick: u32,
    min_temperature: u32,
}

fn number(value: &Value, key: &str, default: u32) -> u32 {
    value
        .get(key)
        .and_then(Value::as_u64)
        .unwrap_or(u64::from(default))
        .min(u64::from(u32::MAX)) as u32
}

fn amount(value: &Value, key: &str, default: u32) -> u32 {
    number(value, key, default).max(1)
}

fn item_input(value: &Value) -> Option<ItemInput> {
    // Catalysts are not consumed probabilistically in this first slice.
    if value
        .get("consume_chance")
        .and_then(Value::as_f64)
        .is_some_and(|chance| chance < 1.0)
    {
        return None;
    }
    Some(ItemInput {
        ingredient: value.get("ingredient")?.clone(),
        count: amount(value, "count", 1),
    })
}

fn fluid_input(value: &Value) -> Option<FluidInput> {
    // This first slice resolves named fluids. Fluid-tag recipe inputs are not yet supported.
    Some(FluidInput {
        fluid: value.get("fluid")?.as_str()?.to_owned(),
        amount: amount(value, "amount", 1),
    })
}

fn item_product(value: &Value) -> Option<ItemProduct> {
    Some(ItemProduct {
        item: value.get("id")?.as_str()?.to_owned(),
        count: amount(value, "count", 1),
    })
}

fn fluid_product(value: &Value) -> Option<FluidProduct> {
    Some(FluidProduct {
        fluid: value.get("id")?.as_str()?.to_owned(),
        amount: amount(value, "amount", 1),
    })
}

fn recipe_from_json(json: &Value) -> Option<Recipe> {
    let mut item_inputs = Vec::new();
    for name in ["item_input", "second_item_input", "third_item_input"] {
        if let Some(value) = json.get(name) {
            item_inputs.push(item_input(value)?);
        }
    }
    let fluid_inputs = match json.get("fluid_inputs").and_then(Value::as_array) {
        Some(values) => values.iter().map(fluid_input).collect::<Option<Vec<_>>>()?,
        None => Vec::new(),
    };
    if item_inputs.len() > ITEM_INPUTS.len()
        || fluid_inputs.len() > FLUID_INPUTS.len()
        || (item_inputs.is_empty() && fluid_inputs.is_empty())
    {
        return None;
    }

    let item_output = json
        .get("item_output")
        .map(item_product)
        .transpose_option()?;
    let fluid_output = json
        .get("fluid_output")
        .map(fluid_product)
        .transpose_option()?;
    let item_byproduct = json.get("byproduct").map(item_product).transpose_option()?;
    let fluid_byproduct = json
        .get("fluid_byproduct")
        .map(fluid_product)
        .transpose_option()?;
    if item_output.is_none() && fluid_output.is_none() {
        return None;
    }
    // A chance-based by-product needs the server's recipe RNG stream; skip it until that
    // operation can be represented by the pinned Host API without changing Horizon.
    if item_byproduct.is_some()
        && json
            .get("byproduct_chance")
            .and_then(Value::as_f64)
            .is_some_and(|chance| chance < 1.0)
    {
        return None;
    }

    Some(Recipe {
        item_inputs,
        fluid_inputs,
        item_output,
        fluid_output,
        item_byproduct,
        fluid_byproduct,
        ticks: amount(json, "time", 100),
        energy_per_tick: number(json, "energy_per_tick", 60),
        heat_per_tick: number(json, "heat_per_tick", 0),
        min_temperature: number(json, "min_temperature", 0),
    })
}

trait OptionTranspose<T> {
    fn transpose_option(self) -> Option<T>;
}

impl<T> OptionTranspose<T> for Option<Option<T>> {
    fn transpose_option(self) -> Option<T> {
        self.flatten()
    }
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

fn progress(entity: &ModBlockEntity) -> u32 {
    get_i32(&entity.nbt, "ChemicalReactorProgress").max(0) as u32
}

fn upgrade_count(entity: &ModBlockEntity, slot: usize, id: &str) -> u32 {
    entity
        .items
        .get(slot)
        .and_then(Option::as_ref)
        .filter(|stack| stack.item.as_ref() == id)
        .map_or(0, |stack| stack.count.min(8))
}

fn item_matches(host: &dyn Host, stack: &ItemStack, input: &ItemInput) -> bool {
    stack.count >= input.count
        && host
            .recipe_ingredient(&input.ingredient)
            .is_some_and(|ingredient| ingredient.test(stack))
}

fn assign_items(
    host: &dyn Host,
    entity: &ModBlockEntity,
    inputs: &[ItemInput],
    index: usize,
    used: u8,
    assigned: &mut Vec<usize>,
) -> bool {
    if index >= inputs.len() {
        return true;
    }
    for (input_index, slot) in ITEM_INPUTS.iter().copied().enumerate() {
        if used & (1 << input_index) != 0 {
            continue;
        }
        let Some(stack) = entity.items.get(slot).and_then(Option::as_ref) else {
            continue;
        };
        if !item_matches(host, stack, &inputs[index]) {
            continue;
        }
        assigned.push(input_index);
        if assign_items(
            host,
            entity,
            inputs,
            index + 1,
            used | (1 << input_index),
            assigned,
        ) {
            return true;
        }
        assigned.pop();
    }
    false
}

fn items_for(host: &dyn Host, entity: &ModBlockEntity, recipe: &Recipe) -> Option<Vec<usize>> {
    let mut assigned = Vec::with_capacity(recipe.item_inputs.len());
    assign_items(host, entity, &recipe.item_inputs, 0, 0, &mut assigned).then_some(assigned)
}

fn fluid_matches(entity: &ModBlockEntity, tank_index: usize, input: &FluidInput) -> bool {
    crate::fluid::name(&entity.nbt, FLUID_INPUTS[tank_index]).as_deref()
        == Some(input.fluid.as_str())
        && crate::fluid::amount(&entity.nbt, FLUID_INPUTS[tank_index]) >= input.amount
}

fn assign_fluids(
    entity: &ModBlockEntity,
    inputs: &[FluidInput],
    index: usize,
    used: u8,
    assigned: &mut Vec<usize>,
) -> bool {
    if index >= inputs.len() {
        return true;
    }
    for tank_index in 0..FLUID_INPUTS.len() {
        if used & (1 << tank_index) != 0 || !fluid_matches(entity, tank_index, &inputs[index]) {
            continue;
        }
        assigned.push(tank_index);
        if assign_fluids(
            entity,
            inputs,
            index + 1,
            used | (1 << tank_index),
            assigned,
        ) {
            return true;
        }
        assigned.pop();
    }
    false
}

fn fluids_for(entity: &ModBlockEntity, recipe: &Recipe) -> Option<Vec<usize>> {
    let mut assigned = Vec::with_capacity(recipe.fluid_inputs.len());
    assign_fluids(entity, &recipe.fluid_inputs, 0, 0, &mut assigned).then_some(assigned)
}

fn find_recipe(
    host: &dyn Host,
    entity: &ModBlockEntity,
) -> Option<(Recipe, Vec<usize>, Vec<usize>, String)> {
    for source in host.recipes_of_type("arcforge:chemical_reacting") {
        let json = source.json.as_ref();
        let Some(recipe) = recipe_from_json(json) else {
            continue;
        };
        let Some(items) = items_for(host, entity, &recipe) else {
            continue;
        };
        let Some(fluids) = fluids_for(entity, &recipe) else {
            continue;
        };
        return Some((recipe, items, fluids, json.to_string()));
    }
    None
}

fn item_stack_fits(current: Option<&ItemStack>, product: &ItemProduct) -> bool {
    current.is_none_or(|stack| {
        stack.item.as_ref() == product.item && stack.count.saturating_add(product.count) <= 64
    })
}

fn fluid_product_fits(entity: &ModBlockEntity, tank: usize, product: &FluidProduct) -> bool {
    crate::fluid::can_insert(
        &entity.nbt,
        tank,
        &product.fluid,
        product.amount,
        TANK_CAPACITY,
    )
}

fn outputs_fit(entity: &ModBlockEntity, recipe: &Recipe) -> bool {
    recipe.item_output.as_ref().is_none_or(|product| {
        item_stack_fits(
            entity.items.get(ITEM_OUTPUT).and_then(Option::as_ref),
            product,
        )
    }) && recipe.item_byproduct.as_ref().is_none_or(|product| {
        item_stack_fits(
            entity.items.get(ITEM_BYPRODUCT).and_then(Option::as_ref),
            product,
        )
    }) && recipe
        .fluid_output
        .as_ref()
        .is_none_or(|product| fluid_product_fits(entity, FLUID_OUTPUT, product))
        && recipe
            .fluid_byproduct
            .as_ref()
            .is_none_or(|product| fluid_product_fits(entity, FLUID_BYPRODUCT, product))
}

fn add_item(entity: &mut ModBlockEntity, slot: usize, product: &ItemProduct) {
    if let Some(stack) = entity.items.get_mut(slot).and_then(Option::as_mut) {
        stack.count = stack.count.saturating_add(product.count);
    } else if let Some(target) = entity.items.get_mut(slot) {
        *target = Some(ItemStack {
            item: product.item.clone().into(),
            count: product.count,
        });
    }
}

fn set_lit(host: &mut dyn Host, pos: [i32; 3], lit: bool) {
    let value = if lit { "true" } else { "false" };
    if host.block_property(pos, "lit").as_deref() != Some(value) {
        host.set_block_property(pos, "lit", value);
    }
}

fn temperature(heat: u64) -> u32 {
    crate::heat::HeatStore::new(HEAT_CAPACITY, MAX_TEMPERATURE_C, heat).temperature_c()
}

fn sync_screen(
    entity: &mut ModBlockEntity,
    energy_rate: u32,
    heat_rate: u32,
    total: u32,
    status: i32,
) {
    entity.data.resize(14, 0);
    entity.data[0] = get_i32(&entity.nbt, "energy").max(0);
    entity.data[1] = get_i32(&entity.nbt, "HeatHU").max(0);
    entity.data[2] = temperature(entity.data[1] as u64) as i32;
    entity.data[3] = progress(entity).min(i32::MAX as u32) as i32;
    entity.data[4] = total.min(i32::MAX as u32) as i32;
    for (index, tank) in FLUID_INPUTS.iter().copied().enumerate() {
        entity.data[5 + index] =
            crate::fluid::amount(&entity.nbt, tank).min(i32::MAX as u32) as i32;
    }
    entity.data[8] = crate::fluid::amount(&entity.nbt, FLUID_OUTPUT).min(i32::MAX as u32) as i32;
    entity.data[9] = crate::fluid::amount(&entity.nbt, FLUID_BYPRODUCT).min(i32::MAX as u32) as i32;
    entity.data[10] = energy_rate.min(i32::MAX as u32) as i32;
    entity.data[11] = heat_rate.min(i32::MAX as u32) as i32;
    entity.data[12] = TANK_CAPACITY.min(i32::MAX as u32) as i32;
    entity.data[13] = status;
}

fn set_status(entity: &mut ModBlockEntity, status: i32, energy: u32, heat: u32, total: u32) {
    set_i32(&mut entity.nbt, "ChemicalReactorStatus", status);
    set_i32(
        &mut entity.nbt,
        "ChemicalReactorEnergyRate",
        energy.min(i32::MAX as u32) as i32,
    );
    set_i32(
        &mut entity.nbt,
        "ChemicalReactorHeatRate",
        heat.min(i32::MAX as u32) as i32,
    );
    set_i32(
        &mut entity.nbt,
        "ChemicalReactorTotal",
        total.min(i32::MAX as u32) as i32,
    );
    sync_screen(entity, energy, heat, total, status);
}

fn has_recipe_items(host: &dyn Host, entity: &ModBlockEntity) -> bool {
    entity.items.iter().enumerate().any(|(slot, stack)| {
        ITEM_INPUTS.contains(&slot)
            && stack.as_ref().is_some_and(|stack| {
                host.recipes_of_type("arcforge:chemical_reacting")
                    .iter()
                    .filter_map(|source| recipe_from_json(&source.json))
                    .flat_map(|recipe| recipe.item_inputs)
                    .any(|input| {
                        stack.count > 0
                            && host
                                .recipe_ingredient(&input.ingredient)
                                .is_some_and(|ingredient| ingredient.test(stack))
                    })
            })
    })
}

pub(crate) fn tick(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let stored_energy = get_i32(&entity.nbt, "energy").max(0) as u64;
    let stored_heat = get_i32(&entity.nbt, "HeatHU").max(0) as u64;
    let Some((recipe, item_slots, fluid_tanks, signature)) = find_recipe(host, entity) else {
        set_i32(&mut entity.nbt, "ChemicalReactorProgress", 0);
        set_string(&mut entity.nbt, "ChemicalReactorRecipe", "");
        let status = if has_recipe_items(host, entity) { 2 } else { 1 };
        set_status(entity, status, 0, 0, 0);
        set_lit(host, pos, false);
        return;
    };

    if get_string(&entity.nbt, "ChemicalReactorRecipe").as_deref() != Some(signature.as_str()) {
        set_i32(&mut entity.nbt, "ChemicalReactorProgress", 0);
        set_string(&mut entity.nbt, "ChemicalReactorRecipe", &signature);
    }
    let speed_upgrades = upgrade_count(entity, SPEED_SLOT, SPEED_UPGRADE);
    let energy_upgrades = upgrade_count(entity, ENERGY_SLOT, ENERGY_UPGRADE);
    let total = crate::speed_upgrade_time(recipe.ticks, speed_upgrades).max(1);
    if !outputs_fit(entity, &recipe) {
        set_status(entity, 3, 0, 0, total);
        set_lit(host, pos, false);
        return;
    }

    let speed = crate::speed_upgrade_multiplier(speed_upgrades);
    let heat_cost = (f64::from(recipe.heat_per_tick) * speed).ceil() as u64;
    if recipe.heat_per_tick > 0
        && (temperature(stored_heat) < recipe.min_temperature || stored_heat < heat_cost)
    {
        set_status(entity, 4, 0, 0, total);
        set_lit(host, pos, false);
        return;
    }
    let energy_cost = crate::scaled_upgrade_cost(
        u64::from(recipe.energy_per_tick),
        speed_upgrades,
        energy_upgrades,
    );
    let mut energy = crate::energy::EnergyStore::new(ENERGY_CAPACITY, stored_energy);
    if energy.stored_fe() < energy_cost {
        set_status(entity, 5, 0, 0, total);
        set_lit(host, pos, false);
        return;
    }

    energy.extract(energy_cost, energy_cost);
    set_i32(
        &mut entity.nbt,
        "energy",
        energy.stored_fe().min(i32::MAX as u64) as i32,
    );
    set_i32(
        &mut entity.nbt,
        "HeatHU",
        stored_heat.saturating_sub(heat_cost).min(i32::MAX as u64) as i32,
    );
    let next = progress(entity).saturating_add(1);
    set_i32(
        &mut entity.nbt,
        "ChemicalReactorProgress",
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
        for (input_index, input) in recipe.item_inputs.iter().enumerate() {
            let slot = ITEM_INPUTS[item_slots[input_index]];
            let empty = if let Some(stack) = entity.items.get_mut(slot).and_then(Option::as_mut) {
                stack.count = stack.count.saturating_sub(input.count);
                stack.count == 0
            } else {
                false
            };
            if empty {
                entity.items[slot] = None;
            }
        }
        for (input_index, input) in recipe.fluid_inputs.iter().enumerate() {
            crate::fluid::extract(
                &mut entity.nbt,
                FLUID_INPUTS[fluid_tanks[input_index]],
                &input.fluid,
                input.amount,
            );
        }
        if let Some(product) = &recipe.item_output {
            add_item(entity, ITEM_OUTPUT, product);
        }
        if let Some(product) = &recipe.item_byproduct {
            add_item(entity, ITEM_BYPRODUCT, product);
        }
        if let Some(product) = &recipe.fluid_output {
            crate::fluid::insert(
                &mut entity.nbt,
                FLUID_OUTPUT,
                &product.fluid,
                product.amount,
                TANK_CAPACITY,
            );
        }
        if let Some(product) = &recipe.fluid_byproduct {
            crate::fluid::insert(
                &mut entity.nbt,
                FLUID_BYPRODUCT,
                &product.fluid,
                product.amount,
                TANK_CAPACITY,
            );
        }
        set_i32(&mut entity.nbt, "ChemicalReactorProgress", 0);
        set_status(entity, 0, 0, 0, total);
        set_lit(host, pos, false);
    }
}

pub(crate) fn accepts(host: &dyn Host, tank: usize, fluid: &str) -> bool {
    FLUID_INPUTS.contains(&tank)
        && host
            .recipes_of_type("arcforge:chemical_reacting")
            .iter()
            .filter_map(|source| recipe_from_json(&source.json))
            .any(|recipe| recipe.fluid_inputs.iter().any(|input| input.fluid == fluid))
}

pub(crate) fn item_allowed(host: &dyn Host, slot: usize, stack: &ItemStack) -> bool {
    if slot == SPEED_SLOT {
        return stack.item.as_ref() == SPEED_UPGRADE && stack.count <= 8;
    }
    if slot == ENERGY_SLOT {
        return stack.item.as_ref() == ENERGY_UPGRADE && stack.count <= 8;
    }
    ITEM_INPUTS.contains(&slot)
        && host
            .recipes_of_type("arcforge:chemical_reacting")
            .iter()
            .filter_map(|source| recipe_from_json(&source.json))
            .flat_map(|recipe| recipe.item_inputs)
            .any(|input| {
                host.recipe_ingredient(&input.ingredient)
                    .is_some_and(|ingredient| ingredient.test(stack))
            })
}

pub(crate) fn fluid_tanks() -> ModFluidTanks {
    ModFluidTanks {
        tanks: vec![TANK_CAPACITY as i32; 5],
        // Static north-facing approximation: inputs up/east, output west, byproduct south.
        faces: [
            vec![],
            FLUID_INPUTS.to_vec(),
            vec![],
            vec![FLUID_BYPRODUCT],
            vec![FLUID_OUTPUT],
            FLUID_INPUTS.to_vec(),
        ],
        inputs: FLUID_INPUTS.to_vec(),
        outputs: vec![FLUID_OUTPUT, FLUID_BYPRODUCT],
    }
}

pub(crate) fn energy_buffer() -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: ENERGY_CAPACITY as i32,
        max_insert: MAX_ENERGY_INPUT as i32,
        max_extract: 0,
        // Static north-facing approximation: FE enters from the back (south).
        insert_faces: [false, false, false, true, false, false],
        extract_faces: [false; 6],
    }
}

pub(crate) fn container() -> ModContainer {
    ModContainer {
        slots: 7,
        faces: [
            vec![],
            vec![0, 3, 4],
            vec![],
            vec![],
            vec![1, 2],
            vec![0, 3, 4],
        ],
        inputs: vec![0, 3, 4, SPEED_SLOT, ENERGY_SLOT],
        outputs: vec![ITEM_OUTPUT, ITEM_BYPRODUCT],
        menu: Some("arcforge:machine_7_slot".into()),
    }
}

pub(crate) fn face(host: &mut dyn Host, pos: [i32; 3], name: &str) {
    if matches!(name, "north" | "east" | "south" | "west") {
        host.set_block_property(pos, "facing", name);
    }
}

pub(crate) fn accept_heat(
    host: &mut dyn Host,
    pos: [i32; 3],
    source_pos: [i32; 3],
    source: &mut crate::heat::HeatStore,
    rate: u64,
) -> u64 {
    if source_pos != [pos[0], pos[1] - 1, pos[2]] {
        return 0;
    }
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

pub(crate) fn status(entity: &ModBlockEntity) -> String {
    let state = match get_i32(&entity.nbt, "ChemicalReactorStatus") {
        1 => "idle",
        2 => "no matching recipe inputs",
        3 => "output full",
        4 => "too cold or needs HU",
        5 => "needs FE",
        _ => "reacting",
    };
    let heat = get_i32(&entity.nbt, "HeatHU").max(0) as u64;
    let input_fluid =
        crate::fluid::name(&entity.nbt, FLUID_INPUTS[0]).unwrap_or_else(|| "empty".to_owned());
    format!(
        "Chemical Reactor: {state} | Input A {input_fluid} {}/{}mB | progress {}/{} | {}°C | heat {heat}/{HEAT_CAPACITY} HU | FE {}/{}",
        crate::fluid::amount(&entity.nbt, FLUID_INPUTS[0]),
        TANK_CAPACITY,
        progress(entity),
        get_i32(&entity.nbt, "ChemicalReactorTotal").max(0),
        temperature(heat),
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
    let heat = data.get(1).copied().unwrap_or_default().max(0);
    let temperature = data.get(2).copied().unwrap_or_default().max(0);
    let progress = data.get(3).copied().unwrap_or_default().max(0);
    let total = data.get(4).copied().unwrap_or_default().max(0);
    let input_a = data.get(5).copied().unwrap_or_default().max(0);
    let input_b = data.get(6).copied().unwrap_or_default().max(0);
    let input_c = data.get(7).copied().unwrap_or_default().max(0);
    let out = data.get(8).copied().unwrap_or_default().max(0);
    let byproduct = data.get(9).copied().unwrap_or_default().max(0);
    let energy_rate = data.get(10).copied().unwrap_or_default().max(0);
    let heat_rate = data.get(11).copied().unwrap_or_default().max(0);
    let capacity = data.get(12).copied().unwrap_or(TANK_CAPACITY as i32).max(0);
    let status = match data.get(13).copied().unwrap_or_default() {
        1 => "Idle",
        2 => "No matching inputs",
        3 => "Output full",
        4 => "Heating / needs HU",
        5 => "Waiting for FE",
        _ if total > 0 => "Reacting",
        _ => "Idle",
    };

    text!("Chemical Reactor", 8.0, 6.0, 0xFFFFFFFF);
    text!(
        &format!("Input A {input_a}/{capacity} mB"),
        8.0,
        16.0,
        0xFFBBBBBB
    );
    text!(
        &format!("Input B {input_b}/{capacity} mB"),
        8.0,
        34.0,
        0xFFBBBBBB
    );
    text!(
        &format!("Input C {input_c}/{capacity} mB"),
        8.0,
        52.0,
        0xFFBBBBBB
    );
    text!("Product tank", 96.0, 34.0, 0xFFBBBBBB);
    text!(&format!("{}/{}", out, capacity), 96.0, 47.0, 0xFFE0E0E0);
    text!("By-product", 96.0, 62.0, 0xFFBBBBBB);
    text!(
        &format!("{}/{} mB", byproduct, capacity),
        96.0,
        75.0,
        0xFFE0E0E0
    );
    text!(
        &format!("FE {energy}/{ENERGY_CAPACITY} ({energy_rate}/t)"),
        8.0,
        95.0,
        0xFFE0E0E0
    );
    text!(
        &format!("Heat {heat}/{HEAT_CAPACITY} HU ({heat_rate}/t)"),
        8.0,
        107.0,
        0xFFE0E0E0
    );
    text!(&format!("{temperature}°C"), 8.0, 119.0, 0xFFE0E0E0);
    text!(status, 8.0, 133.0, 0xFFBBBBBB);
    if total > 0 {
        text!(
            &format!("Progress {progress}/{total} t"),
            8.0,
            146.0,
            0xFFE0E0E0
        );
        fill!([8.0, 158.0, 160.0, 6.0], 0xFF383838);
        let fraction = (progress as f32 / total as f32).clamp(0.0, 1.0);
        fill!([8.0, 158.0, 160.0 * fraction, 6.0], 0xFF62B64F);
    }
    text!("Speed", 35.0, 184.0, 0xFFBBBBBB);
    text!("Energy", 116.0, 184.0, 0xFFBBBBBB);
    text!("Inventory", 8.0, 197.0, 0xFFFFFFFF);
}
