//! First-pass Arcforge Chemical Reactor Array: three shared-fluid processing lanes in a 3x3x3 cube.
use horizon_client::{
    containers::{ModBlockEntity, ModContainer},
    mod_api::{
        Direction, Host, ItemStack, ModEnergyBuffer, ModFluidTanks, ScreenDrawEvent, Tag,
        serde_json::Value,
    },
};

pub(crate) const CASING: &str = "arcforge:chemical_reactor_array_casing";

const LANES: usize = 3;
const SLOTS_PER_LANE: usize = 5;
const LANE_SLOTS: usize = LANES * SLOTS_PER_LANE;
const SPEED_SLOT: usize = 15;
const ENERGY_SLOT: usize = 16;
const INVENTORY_SLOTS: usize = 18;
const TANK_CAPACITY: u32 = 32_000;
const ENERGY_CAPACITY: u64 = 200_000;
const MAX_ENERGY_INPUT: u64 = 4_000;
const HEAT_CAPACITY: u64 = 160_000;
const MAX_TEMPERATURE_C: u32 = 1_200;
const BASE_TIME_MULTIPLIER: f64 = 0.5;
const BASE_ENERGY_MULTIPLIER: f64 = 1.6;
const OUTPUT_RATE: i32 = 4_000;
const FORMATION_CHECK_TICKS: i32 = 20;
const INPUT_TANKS: [usize; 3] = [0, 1, 2];
const OUTPUT_TANK: usize = 3;
const BYPRODUCT_TANK: usize = 4;
const INPUT_SLOTS: [usize; 9] = [0, 1, 2, 5, 6, 7, 10, 11, 12];
const OUTPUT_SLOTS: [usize; 6] = [3, 4, 8, 9, 13, 14];
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
    for key in ["item_input", "second_item_input", "third_item_input"] {
        if let Some(value) = json.get(key) {
            item_inputs.push(item_input(value)?);
        }
    }
    let fluid_inputs = match json.get("fluid_inputs").and_then(Value::as_array) {
        Some(values) => values.iter().map(fluid_input).collect::<Option<Vec<_>>>()?,
        None => Vec::new(),
    };
    if item_inputs.len() > 3
        || fluid_inputs.len() > INPUT_TANKS.len()
        || (item_inputs.is_empty() && fluid_inputs.is_empty())
    {
        return None;
    }

    let item_output = match json.get("item_output") {
        Some(value) => Some(item_product(value)?),
        None => None,
    };
    let fluid_output = match json.get("fluid_output") {
        Some(value) => Some(fluid_product(value)?),
        None => None,
    };
    let item_byproduct = match json.get("byproduct") {
        Some(value) => Some(item_product(value)?),
        None => None,
    };
    let fluid_byproduct = match json.get("fluid_byproduct") {
        Some(value) => Some(fluid_product(value)?),
        None => None,
    };
    if item_output.is_none() && fluid_output.is_none() {
        return None;
    }
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

fn lane_input_slots(lane: usize) -> [usize; 3] {
    let start = lane * SLOTS_PER_LANE;
    [start, start + 1, start + 2]
}

fn lane_output_slot(lane: usize) -> usize {
    lane * SLOTS_PER_LANE + 3
}

fn lane_byproduct_slot(lane: usize) -> usize {
    lane * SLOTS_PER_LANE + 4
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
    slots: &[usize; 3],
    inputs: &[ItemInput],
    index: usize,
    used: u8,
    assigned: &mut Vec<usize>,
) -> bool {
    if index >= inputs.len() {
        return true;
    }
    for (input_index, slot) in slots.iter().copied().enumerate() {
        if used & (1 << input_index) != 0 {
            continue;
        }
        let Some(stack) = entity.items.get(slot).and_then(Option::as_ref) else {
            continue;
        };
        if !item_matches(host, stack, &inputs[index]) {
            continue;
        }
        assigned.push(slot);
        if assign_items(
            host,
            entity,
            slots,
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

fn items_for(
    host: &dyn Host,
    entity: &ModBlockEntity,
    lane: usize,
    recipe: &Recipe,
) -> Option<Vec<usize>> {
    let mut assigned = Vec::with_capacity(recipe.item_inputs.len());
    assign_items(
        host,
        entity,
        &lane_input_slots(lane),
        &recipe.item_inputs,
        0,
        0,
        &mut assigned,
    )
    .then_some(assigned)
}

fn fluids_for(entity: &ModBlockEntity, recipe: &Recipe) -> Option<Vec<usize>> {
    fn assign(
        entity: &ModBlockEntity,
        inputs: &[FluidInput],
        index: usize,
        used: u8,
        assigned: &mut Vec<usize>,
    ) -> bool {
        if index >= inputs.len() {
            return true;
        }
        for tank in INPUT_TANKS {
            if used & (1 << tank) != 0
                || crate::fluid::name(&entity.nbt, tank).as_deref()
                    != Some(inputs[index].fluid.as_str())
                || crate::fluid::amount(&entity.nbt, tank) < inputs[index].amount
            {
                continue;
            }
            assigned.push(tank);
            if assign(entity, inputs, index + 1, used | (1 << tank), assigned) {
                return true;
            }
            assigned.pop();
        }
        false
    }
    let mut assigned = Vec::with_capacity(recipe.fluid_inputs.len());
    assign(entity, &recipe.fluid_inputs, 0, 0, &mut assigned).then_some(assigned)
}

fn find_recipe(
    host: &dyn Host,
    entity: &ModBlockEntity,
    lane: usize,
) -> Option<(Recipe, Vec<usize>, Vec<usize>, String)> {
    for source in host.recipes_of_type("arcforge:chemical_reacting") {
        let json = source.json.as_ref();
        let Some(recipe) = recipe_from_json(json) else {
            continue;
        };
        let Some(items) = items_for(host, entity, lane, &recipe) else {
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

fn outputs_fit(entity: &ModBlockEntity, lane: usize, recipe: &Recipe) -> bool {
    recipe.item_output.as_ref().is_none_or(|product| {
        item_stack_fits(
            entity
                .items
                .get(lane_output_slot(lane))
                .and_then(Option::as_ref),
            product,
        )
    }) && recipe.item_byproduct.as_ref().is_none_or(|product| {
        item_stack_fits(
            entity
                .items
                .get(lane_byproduct_slot(lane))
                .and_then(Option::as_ref),
            product,
        )
    }) && recipe.fluid_output.as_ref().is_none_or(|product| {
        crate::fluid::can_insert(
            &entity.nbt,
            OUTPUT_TANK,
            &product.fluid,
            product.amount,
            TANK_CAPACITY,
        )
    }) && recipe.fluid_byproduct.as_ref().is_none_or(|product| {
        crate::fluid::can_insert(
            &entity.nbt,
            BYPRODUCT_TANK,
            &product.fluid,
            product.amount,
            TANK_CAPACITY,
        )
    })
}

fn add_item(entity: &mut ModBlockEntity, slot: usize, product: &ItemProduct) {
    if let Some(stack) = entity.items.get_mut(slot).and_then(Option::as_mut) {
        stack.count = stack.count.saturating_add(product.count);
    } else if let Some(target) = entity.items.get_mut(slot) {
        *target = Some(ItemStack {
            item: product.item.clone().into(),
            count: product.count,
            damage: 0,
            extra: None,
        });
    }
}

fn finish(host: &dyn Host, entity: &mut ModBlockEntity, lane: usize, recipe: &Recipe) -> bool {
    let Some(item_slots) = items_for(host, entity, lane, recipe) else {
        return false;
    };
    let Some(fluid_tanks) = fluids_for(entity, recipe) else {
        return false;
    };
    if !outputs_fit(entity, lane, recipe) {
        return false;
    }

    for (input, slot) in recipe.item_inputs.iter().zip(item_slots) {
        let emptied = if let Some(stack) = entity.items.get_mut(slot).and_then(Option::as_mut) {
            stack.count = stack.count.saturating_sub(input.count);
            stack.count == 0
        } else {
            false
        };
        if emptied {
            entity.items[slot] = None;
        }
    }
    for (input, tank) in recipe.fluid_inputs.iter().zip(fluid_tanks) {
        crate::fluid::extract(&mut entity.nbt, tank, &input.fluid, input.amount);
    }
    if let Some(product) = &recipe.item_output {
        add_item(entity, lane_output_slot(lane), product);
    }
    if let Some(product) = &recipe.item_byproduct {
        add_item(entity, lane_byproduct_slot(lane), product);
    }
    if let Some(product) = &recipe.fluid_output {
        crate::fluid::insert(
            &mut entity.nbt,
            OUTPUT_TANK,
            &product.fluid,
            product.amount,
            TANK_CAPACITY,
        );
    }
    if let Some(product) = &recipe.fluid_byproduct {
        crate::fluid::insert(
            &mut entity.nbt,
            BYPRODUCT_TANK,
            &product.fluid,
            product.amount,
            TANK_CAPACITY,
        );
    }
    true
}

fn lane_progress(entity: &ModBlockEntity, lane: usize) -> u32 {
    crate::get_i32(&entity.nbt, &format!("ChemicalArrayProgress{lane}")).max(0) as u32
}

fn lane_total(entity: &ModBlockEntity, lane: usize) -> u32 {
    crate::get_i32(&entity.nbt, &format!("ChemicalArrayTotal{lane}")).max(0) as u32
}

fn lane_recipe(entity: &ModBlockEntity, lane: usize) -> Option<String> {
    let key = Tag::String(format!("ChemicalArrayRecipe{lane}").into());
    match entity.nbt.get(&key) {
        Some(Tag::String(value)) if !value.to_string_lossy().is_empty() => {
            Some(value.to_string_lossy())
        }
        _ => None,
    }
}

fn set_lane_recipe(entity: &mut ModBlockEntity, lane: usize, recipe: &str) {
    let key = Tag::String(format!("ChemicalArrayRecipe{lane}").into());
    entity.nbt.insert(key, Tag::String(recipe.into()));
}

fn set_lane_progress(entity: &mut ModBlockEntity, lane: usize, progress: u32) {
    crate::set_i32(
        &mut entity.nbt,
        &format!("ChemicalArrayProgress{lane}"),
        progress.min(i32::MAX as u32) as i32,
    );
}

fn set_lane_total(entity: &mut ModBlockEntity, lane: usize, total: u32) {
    crate::set_i32(
        &mut entity.nbt,
        &format!("ChemicalArrayTotal{lane}"),
        total.min(i32::MAX as u32) as i32,
    );
}

fn ticks_for(recipe: &Recipe, speed_upgrades: u32) -> u32 {
    (f64::from(crate::speed_upgrade_time(recipe.ticks, speed_upgrades)) * BASE_TIME_MULTIPLIER)
        .round()
        .max(1.0) as u32
}

fn energy_per_tick(recipe: &Recipe, speed: f64, energy_upgrades: u32) -> u64 {
    crate::scaled_upgrade_cost(
        u64::from(recipe.energy_per_tick),
        BASE_ENERGY_MULTIPLIER * speed,
        energy_upgrades,
    )
}

fn heat_per_tick(recipe: &Recipe, speed: f64) -> u64 {
    (f64::from(recipe.heat_per_tick) * speed / BASE_TIME_MULTIPLIER)
        .ceil()
        .clamp(0.0, u64::MAX as f64) as u64
}

fn temperature(heat: u64) -> u32 {
    crate::heat::HeatStore::new(HEAT_CAPACITY, MAX_TEMPERATURE_C, heat).temperature_c()
}

fn set_lit(host: &mut dyn Host, pos: [i32; 3], lit: bool) {
    let value = if lit { "true" } else { "false" };
    if host.block_property(pos, "lit").as_deref() != Some(value) {
        host.set_block_property(pos, "lit", value);
    }
}

fn sync_screen(
    entity: &mut ModBlockEntity,
    running: i32,
    usage: u64,
    heat_usage: u64,
    status: i32,
) {
    entity.data.resize(24, 0);
    entity.data[0] = crate::get_i32(&entity.nbt, "energy").max(0);
    entity.data[1] = ENERGY_CAPACITY as i32;
    entity.data[2] = usage.min(i32::MAX as u64) as i32;
    let heat = crate::get_i32(&entity.nbt, "HeatHU").max(0) as u64;
    entity.data[3] = heat.min(i32::MAX as u64) as i32;
    entity.data[4] = temperature(heat) as i32;
    entity.data[5] = heat_usage.min(i32::MAX as u64) as i32;
    entity.data[6] = TANK_CAPACITY as i32;
    entity.data[7] = status;
    for tank in 0..5 {
        entity.data[8 + tank] = crate::fluid::amount(&entity.nbt, tank).min(i32::MAX as u32) as i32;
    }
    for lane in 0..LANES {
        entity.data[13 + lane] = lane_progress(entity, lane).min(i32::MAX as u32) as i32;
        entity.data[16 + lane] = lane_total(entity, lane).min(i32::MAX as u32) as i32;
    }
    entity.data[19] = running;
    entity.data[20] = crate::get_i32(&entity.nbt, "SpeedUpgrades").clamp(0, 8);
    entity.data[21] = crate::get_i32(&entity.nbt, "EnergyUpgrades").clamp(0, 8);
    entity.data[22] = crate::get_i32(&entity.nbt, "ChemicalArrayFormation");
    entity.data[23] = 0;
}

fn has_matching_items(host: &dyn Host, entity: &ModBlockEntity) -> bool {
    INPUT_SLOTS.iter().any(|slot| {
        entity
            .items
            .get(*slot)
            .and_then(Option::as_ref)
            .is_some_and(|stack| {
                host.recipes_of_type("arcforge:chemical_reacting")
                    .iter()
                    .filter_map(|source| recipe_from_json(&source.json))
                    .flat_map(|recipe| recipe.item_inputs)
                    .any(|input| {
                        host.recipe_ingredient(&input.ingredient)
                            .is_some_and(|ingredient| ingredient.test(stack))
                    })
            })
    })
}

fn process(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    entity.items.resize(INVENTORY_SLOTS, None);
    let mut energy = crate::energy::EnergyStore::new(
        ENERGY_CAPACITY,
        crate::get_i32(&entity.nbt, "energy").max(0) as u64,
    );
    let mut heat = crate::heat::HeatStore::new(
        HEAT_CAPACITY,
        MAX_TEMPERATURE_C,
        crate::get_i32(&entity.nbt, "HeatHU").max(0) as u64,
    );
    let speed_upgrades = entity
        .items
        .get(SPEED_SLOT)
        .and_then(Option::as_ref)
        .filter(|stack| stack.item.as_ref() == SPEED_UPGRADE)
        .map_or(0, |stack| stack.count.min(8));
    let energy_upgrades = entity
        .items
        .get(ENERGY_SLOT)
        .and_then(Option::as_ref)
        .filter(|stack| stack.item.as_ref() == ENERGY_UPGRADE)
        .map_or(0, |stack| stack.count.min(8));
    crate::set_i32(&mut entity.nbt, "SpeedUpgrades", speed_upgrades as i32);
    crate::set_i32(&mut entity.nbt, "EnergyUpgrades", energy_upgrades as i32);
    let speed = crate::speed_upgrade_multiplier(speed_upgrades);
    let mut usage = 0_u64;
    let mut heat_usage = 0_u64;
    let mut running = 0_i32;
    let mut any_recipe = false;
    let mut missing_inputs = false;
    let mut full = false;
    let mut cold = false;
    let mut no_power = false;

    for lane in 0..LANES {
        let progress = lane_progress(entity, lane);
        let previous_recipe = lane_recipe(entity, lane);
        let Some((recipe, _, _, signature)) = find_recipe(host, entity, lane) else {
            let waiting_for_shared_inputs =
                previous_recipe.is_some() && progress >= lane_total(entity, lane) && progress > 0;
            if !waiting_for_shared_inputs {
                set_lane_progress(entity, lane, 0);
                set_lane_total(entity, lane, 0);
                set_lane_recipe(entity, lane, "");
            } else {
                missing_inputs = true;
            }
            continue;
        };
        any_recipe = true;
        if previous_recipe.as_deref() != Some(signature.as_str()) {
            set_lane_progress(entity, lane, 0);
            set_lane_recipe(entity, lane, &signature);
        }
        let total = ticks_for(&recipe, speed_upgrades);
        set_lane_total(entity, lane, total);
        if !outputs_fit(entity, lane, &recipe) {
            full = true;
            continue;
        }
        if lane_progress(entity, lane) >= total {
            if finish(host, entity, lane, &recipe) {
                set_lane_progress(entity, lane, 0);
            } else {
                missing_inputs = true;
            }
            continue;
        }
        let lane_heat = heat_per_tick(&recipe, speed);
        if (recipe.heat_per_tick > 0 || recipe.min_temperature > 0)
            && (heat.temperature_c() < recipe.min_temperature || heat.stored_hu() < lane_heat)
        {
            cold = true;
            continue;
        }
        let lane_energy = energy_per_tick(&recipe, speed, energy_upgrades);
        if energy.extract(lane_energy, lane_energy) < lane_energy {
            no_power = true;
            continue;
        }
        usage = usage.saturating_add(lane_energy);
        if lane_heat > 0 {
            heat_usage = heat_usage.saturating_add(heat.remove(lane_heat));
        }
        let next = lane_progress(entity, lane).saturating_add(1);
        set_lane_progress(entity, lane, next);
        running |= 1 << lane;
        if next >= total && finish(host, entity, lane, &recipe) {
            set_lane_progress(entity, lane, 0);
        }
    }

    crate::set_i32(
        &mut entity.nbt,
        "energy",
        energy.stored_fe().min(i32::MAX as u64) as i32,
    );
    crate::set_i32(
        &mut entity.nbt,
        "HeatHU",
        heat.stored_hu().min(i32::MAX as u64) as i32,
    );
    crate::set_i32(&mut entity.nbt, "TemperatureC", heat.temperature_c() as i32);
    let status = if running != 0 {
        0
    } else if full {
        3
    } else if cold {
        4
    } else if no_power {
        5
    } else if missing_inputs || (!any_recipe && has_matching_items(host, entity)) {
        2
    } else {
        1
    };
    crate::set_i32(&mut entity.nbt, "ChemicalArrayStatus", status);
    crate::set_i32(
        &mut entity.nbt,
        "ChemicalArrayUsage",
        usage.min(i32::MAX as u64) as i32,
    );
    crate::set_i32(
        &mut entity.nbt,
        "ChemicalArrayHeatUsage",
        heat_usage.min(i32::MAX as u64) as i32,
    );
    set_lit(host, pos, running != 0);
    sync_screen(entity, running, usage, heat_usage, status);
    push_outputs(host, pos);
}

fn positions(center: [i32; 3]) -> impl Iterator<Item = [i32; 3]> {
    (-1..=1).flat_map(move |y| {
        (-1..=1)
            .flat_map(move |z| (-1..=1).map(move |x| [center[0] + x, center[1] + y, center[2] + z]))
    })
}

fn complete(host: &dyn Host, center: [i32; 3]) -> bool {
    positions(center).all(|pos| host.block(pos).as_str() == CASING)
}

fn set_formed(host: &mut dyn Host, center: [i32; 3], formed: bool) {
    for pos in positions(center) {
        if host.block(pos).as_str() == CASING {
            host.set_block_property(
                pos,
                "part",
                if formed {
                    if pos == center { "center" } else { "other" }
                } else {
                    "none"
                },
            );
            host.set_block_property(pos, "facing", "north");
            if !formed {
                set_lit(host, pos, false);
            }
        }
    }
}

fn form_near(host: &mut dyn Host, placed: [i32; 3]) {
    for center in positions(placed) {
        if complete(host, center) {
            set_formed(host, center, true);
            return;
        }
    }
}

pub(crate) fn unform_near(host: &mut dyn Host, broken: [i32; 3]) {
    let centers = positions(broken)
        .filter(|center| {
            host.block(*center).as_str() == CASING
                && host.block_property(*center, "part").as_deref() == Some("center")
        })
        .collect::<Vec<_>>();
    for center in centers {
        set_formed(host, center, false);
    }
}

pub(crate) fn notify_placed(host: &mut dyn Host, pos: [i32; 3]) {
    if host.block(pos).as_str() == CASING {
        form_near(host, pos);
    }
}

pub(crate) fn is_center(host: &dyn Host, pos: [i32; 3]) -> bool {
    host.block(pos).as_str() == CASING
        && host.block_property(pos, "part").as_deref() == Some("center")
}

fn insert_item(entity: &mut ModBlockEntity, incoming: &ItemStack) -> u32 {
    let mut order = [0_usize, 1, 2];
    order.sort_by_key(|lane| {
        lane_input_slots(*lane)
            .iter()
            .filter_map(|slot| entity.items.get(*slot).and_then(Option::as_ref))
            .map(|stack| stack.count)
            .sum::<u32>()
    });
    let mut remaining = incoming.count;
    for lane in order {
        let slots = lane_input_slots(lane);
        for slot in slots {
            let Some(existing) = entity.items.get(slot).and_then(Option::as_ref) else {
                continue;
            };
            if existing.item.as_ref() != incoming.item.as_ref()
                || existing.extra.is_some()
                || incoming.extra.is_some()
            {
                continue;
            }
            let moved = remaining.min(64_u32.saturating_sub(existing.count));
            if moved > 0 {
                if let Some(Some(target)) = entity.items.get_mut(slot) {
                    target.count += moved;
                }
                remaining -= moved;
            }
            if remaining == 0 {
                return incoming.count;
            }
        }
        for slot in slots {
            if entity.items.get(slot).is_some_and(Option::is_none) {
                let moved = remaining.min(64);
                if let Some(target) = entity.items.get_mut(slot) {
                    let mut stack = incoming.clone();
                    stack.count = moved;
                    *target = Some(stack);
                }
                remaining -= moved;
            }
            if remaining == 0 {
                return incoming.count;
            }
        }
    }
    incoming.count - remaining
}

fn consolidate_items(host: &mut dyn Host, center: [i32; 3], entity: &mut ModBlockEntity) {
    for source_pos in positions(center).filter(|source| *source != center) {
        if host.block(source_pos).as_str() != CASING {
            continue;
        }
        host.change_mod_block_entity(source_pos, &mut |source| {
            if source.id != CASING {
                return;
            }
            source.items.resize(INVENTORY_SLOTS, None);
            for slot in INPUT_SLOTS {
                let Some(stack) = source.items.get(slot).and_then(Option::as_ref).cloned() else {
                    continue;
                };
                let moved = insert_item(entity, &stack);
                if moved == 0 {
                    continue;
                }
                let empty =
                    if let Some(current) = source.items.get_mut(slot).and_then(Option::as_mut) {
                        current.count = current.count.saturating_sub(moved);
                        current.count == 0
                    } else {
                        false
                    };
                if empty {
                    source.items[slot] = None;
                }
            }
        });
    }
}

fn consolidate_fluids(host: &mut dyn Host, center: [i32; 3], entity: &mut ModBlockEntity) {
    for source_pos in positions(center).filter(|source| *source != center) {
        if host.block(source_pos).as_str() != CASING {
            continue;
        }
        let Some(source) = host.mod_block_entity(source_pos) else {
            continue;
        };
        if source.id != CASING {
            continue;
        }
        for tank in INPUT_TANKS {
            let Some(fluid_name) = crate::fluid::name(&source.nbt, tank) else {
                continue;
            };
            let stored = crate::fluid::amount(&source.nbt, tank);
            let current = crate::fluid::amount(&entity.nbt, tank);
            let compatible = crate::fluid::name(&entity.nbt, tank)
                .is_none_or(|current_fluid| current_fluid == fluid_name);
            let accepted = if compatible {
                stored.min(TANK_CAPACITY.saturating_sub(current))
            } else {
                0
            };
            if accepted == 0 {
                continue;
            }
            let mut moved = 0;
            host.change_mod_block_entity(source_pos, &mut |current_source| {
                if current_source.id == CASING {
                    moved =
                        crate::fluid::extract(&mut current_source.nbt, tank, &fluid_name, accepted);
                }
            });
            if moved > 0 {
                crate::fluid::insert(&mut entity.nbt, tank, &fluid_name, moved, TANK_CAPACITY);
            }
        }
    }
}

fn consolidate_energy(host: &mut dyn Host, center: [i32; 3], entity: &mut ModBlockEntity) {
    let mut stored = (crate::get_i32(&entity.nbt, "energy").max(0) as u64).min(ENERGY_CAPACITY);
    for source_pos in positions(center).filter(|source| *source != center) {
        if stored >= ENERGY_CAPACITY || host.block(source_pos).as_str() != CASING {
            continue;
        }
        let Some(source) = host.mod_block_entity(source_pos) else {
            continue;
        };
        if source.id != CASING {
            continue;
        }
        let available = crate::get_i32(&source.nbt, "energy").max(0) as u64;
        let moved = available.min(ENERGY_CAPACITY - stored).min(i32::MAX as u64) as i32;
        if moved == 0 {
            continue;
        }
        let mut removed = 0;
        host.change_mod_block_entity(source_pos, &mut |current| {
            if current.id == CASING {
                let amount = crate::get_i32(&current.nbt, "energy").max(0);
                removed = amount.min(moved);
                crate::set_i32(&mut current.nbt, "energy", amount - removed);
            }
        });
        stored += removed.max(0) as u64;
    }
    crate::set_i32(
        &mut entity.nbt,
        "energy",
        stored.min(i32::MAX as u64) as i32,
    );
}

fn consolidate_heat(host: &mut dyn Host, center: [i32; 3], entity: &mut ModBlockEntity) {
    let mut target = crate::heat::HeatStore::new(
        HEAT_CAPACITY,
        MAX_TEMPERATURE_C,
        crate::get_i32(&entity.nbt, "HeatHU").max(0) as u64,
    );
    for source_pos in positions(center).filter(|source| *source != center) {
        if host.block(source_pos).as_str() != CASING {
            continue;
        }
        let Some(source_entity) = host.mod_block_entity(source_pos) else {
            continue;
        };
        if source_entity.id != CASING {
            continue;
        }
        let amount = crate::get_i32(&source_entity.nbt, "HeatHU").max(0) as u64;
        let mut source = crate::heat::HeatStore::new(HEAT_CAPACITY, MAX_TEMPERATURE_C, amount);
        let moved = source.transfer_to(&mut target, u64::MAX);
        if moved == 0 {
            continue;
        }
        host.change_mod_block_entity(source_pos, &mut |current| {
            if current.id == CASING {
                crate::set_i32(
                    &mut current.nbt,
                    "HeatHU",
                    source.stored_hu().min(i32::MAX as u64) as i32,
                );
            }
        });
    }
    crate::set_i32(
        &mut entity.nbt,
        "HeatHU",
        target.stored_hu().min(i32::MAX as u64) as i32,
    );
    crate::set_i32(
        &mut entity.nbt,
        "TemperatureC",
        target.temperature_c() as i32,
    );
}

pub(crate) fn tick(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    entity.items.resize(INVENTORY_SLOTS, None);
    match host.block_property(pos, "part").as_deref() {
        Some("center") => {
            if !complete(host, pos) {
                unform_near(host, pos);
                return;
            }
            consolidate_items(host, pos, entity);
            consolidate_fluids(host, pos, entity);
            consolidate_energy(host, pos, entity);
            consolidate_heat(host, pos, entity);
            process(host, pos, entity);
        }
        Some("other") => {
            let center_exists = positions(pos).any(|candidate| is_center(host, candidate));
            if !center_exists {
                host.set_block_property(pos, "part", "none");
                set_lit(host, pos, false);
            }
        }
        _ => {
            let elapsed = crate::get_i32(&entity.nbt, "ChemicalArrayFormation").saturating_add(1);
            if elapsed >= FORMATION_CHECK_TICKS {
                crate::set_i32(&mut entity.nbt, "ChemicalArrayFormation", 0);
                form_near(host, pos);
            } else {
                crate::set_i32(&mut entity.nbt, "ChemicalArrayFormation", elapsed);
            }
        }
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
    push_output(host, pos, Direction::Down);
    push_output(host, pos, Direction::South);
}

pub(crate) fn item_allowed(host: &dyn Host, slot: usize, stack: &ItemStack) -> bool {
    if slot == SPEED_SLOT {
        return stack.item.as_ref() == SPEED_UPGRADE && stack.count <= 8;
    }
    if slot == ENERGY_SLOT {
        return stack.item.as_ref() == ENERGY_UPGRADE && stack.count <= 8;
    }
    if !INPUT_SLOTS.contains(&slot) {
        return false;
    }
    host.recipes_of_type("arcforge:chemical_reacting")
        .iter()
        .filter_map(|source| recipe_from_json(&source.json))
        .flat_map(|recipe| recipe.item_inputs)
        .any(|input| {
            host.recipe_ingredient(&input.ingredient)
                .is_some_and(|ingredient| ingredient.test(stack))
        })
}

pub(crate) fn accepts(host: &dyn Host, tank: usize, fluid: &str) -> bool {
    INPUT_TANKS.contains(&tank)
        && host
            .recipes_of_type("arcforge:chemical_reacting")
            .iter()
            .filter_map(|source| recipe_from_json(&source.json))
            .any(|recipe| recipe.fluid_inputs.iter().any(|input| input.fluid == fluid))
}

pub(crate) fn container() -> ModContainer {
    ModContainer {
        slots: INVENTORY_SLOTS,
        faces: [
            OUTPUT_SLOTS.to_vec(),
            INPUT_SLOTS.to_vec(),
            vec![],
            vec![4, 9, 14],
            vec![3, 8, 13],
            INPUT_SLOTS.to_vec(),
        ],
        inputs: INPUT_SLOTS
            .into_iter()
            .chain([SPEED_SLOT, ENERGY_SLOT])
            .collect(),
        outputs: OUTPUT_SLOTS.to_vec(),
        menu: Some("minecraft:generic_9x2".into()),
    }
}

pub(crate) fn fluid_tanks() -> ModFluidTanks {
    ModFluidTanks {
        tanks: vec![TANK_CAPACITY as i32; 5],
        faces: [
            vec![OUTPUT_TANK, BYPRODUCT_TANK],
            INPUT_TANKS.to_vec(),
            vec![],
            vec![BYPRODUCT_TANK],
            vec![OUTPUT_TANK],
            INPUT_TANKS.to_vec(),
        ],
        inputs: INPUT_TANKS.to_vec(),
        outputs: vec![OUTPUT_TANK, BYPRODUCT_TANK],
    }
}

pub(crate) fn energy_buffer() -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: ENERGY_CAPACITY as i32,
        max_insert: MAX_ENERGY_INPUT as i32,
        max_extract: 0,
        insert_faces: [false, false, false, true, false, false],
        extract_faces: [false; 6],
    }
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
    if entity.id != CASING {
        return 0;
    }
    let mut sink = crate::heat::HeatStore::new(
        HEAT_CAPACITY,
        MAX_TEMPERATURE_C,
        crate::get_i32(&entity.nbt, "HeatHU").max(0) as u64,
    );
    let moved = source.transfer_to(&mut sink, rate);
    if moved > 0 {
        host.change_mod_block_entity(pos, &mut |current| {
            if current.id == CASING {
                crate::set_i32(
                    &mut current.nbt,
                    "HeatHU",
                    sink.stored_hu().min(i32::MAX as u64) as i32,
                );
                crate::set_i32(
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
    let state = match crate::get_i32(&entity.nbt, "ChemicalArrayStatus") {
        2 => "waiting for inputs",
        3 => "output full",
        4 => "too cold or needs HU",
        5 => "needs FE",
        0 => "reacting",
        _ => "idle",
    };
    let active = (0..LANES)
        .filter(|lane| lane_progress(entity, *lane) > 0)
        .count();
    let heat = crate::get_i32(&entity.nbt, "HeatHU").max(0) as u64;
    format!(
        "Chemical Reactor Array: {state} | lanes {active}/{LANES} | FE {}/{} | heat {heat}/{HEAT_CAPACITY} HU at {}°C",
        crate::get_i32(&entity.nbt, "energy").max(0),
        ENERGY_CAPACITY,
        temperature(heat),
    )
}

pub(crate) fn draw_screen(host: &dyn Host, event: &mut ScreenDrawEvent<'_>) {
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
    let capacity = data
        .get(1)
        .copied()
        .unwrap_or(ENERGY_CAPACITY as i32)
        .max(0);
    let usage = data.get(2).copied().unwrap_or_default().max(0);
    let heat = data.get(3).copied().unwrap_or_default().max(0);
    let temperature = data.get(4).copied().unwrap_or_default().max(0);
    let heat_usage = data.get(5).copied().unwrap_or_default().max(0);
    let tank_capacity = data.get(6).copied().unwrap_or(TANK_CAPACITY as i32).max(0);
    let status = match data.get(7).copied().unwrap_or_default() {
        2 => "Waiting for inputs",
        3 => "Output full",
        4 => "Heating / needs HU",
        5 => "Waiting for FE",
        0 => "Reacting",
        _ => "Idle",
    };

    text!("Chemical Reactor Array", 8.0, 6.0, 0xFFFFFFFF);
    text!(
        &format!("FE {energy}/{capacity} ({usage}/t)"),
        8.0,
        18.0,
        0xFFE0E0E0
    );
    text!(
        &format!(
            "Heat {heat}/{} HU ({temperature}°C, {heat_usage}/t)",
            HEAT_CAPACITY
        ),
        8.0,
        30.0,
        0xFFE0E0E0
    );
    for tank in 0..5 {
        let label = match tank {
            0 => "Input A",
            1 => "Input B",
            2 => "Input C",
            3 => "Product",
            _ => "By-product",
        };
        let amount = data.get(8 + tank).copied().unwrap_or_default().max(0);
        text!(
            &format!("{label} {amount}/{tank_capacity} mB"),
            8.0 + (tank % 3) as f32 * 72.0,
            45.0 + (tank / 3) as f32 * 12.0,
            0xFFBBBBBB
        );
    }
    text!(status, 8.0, 72.0, 0xFFBBBBBB);
    for lane in 0..LANES {
        let progress = data.get(13 + lane).copied().unwrap_or_default().max(0);
        let total = data.get(16 + lane).copied().unwrap_or_default().max(0);
        let y = 88.0 + lane as f32 * 22.0;
        text!(
            &format!("Lane {}: {progress}/{total} t", lane + 1),
            8.0,
            y,
            0xFFE0E0E0
        );
        fill!([8.0, y + 10.0, 160.0, 5.0], 0xFF383838);
        if total > 0 {
            let fraction = (progress as f32 / total as f32).clamp(0.0, 1.0);
            fill!([8.0, y + 10.0, 160.0 * fraction, 5.0], 0xFF62B64F);
        }
    }
    let speed = data.get(20).copied().unwrap_or_default().clamp(0, 8);
    let energy_upgrades = data.get(21).copied().unwrap_or_default().clamp(0, 8);
    text!(
        &format!("Speed {speed}/8   Energy {energy_upgrades}/8"),
        8.0,
        157.0,
        0xFFBBBBBB
    );
    text!("Inventory", 8.0, 182.0, 0xFFFFFFFF);
}
