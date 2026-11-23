//! Current-Horizon structure detection for the Cracking Unit.
use mod_api::{
    BlockEntityTickEvent, BlockEntityTickReply, BlockKey, Compound, Host, HostBlockStates,
    LevelRef, ServerHost, Tag, Text,
};

const WIDTH: i32 = 3;
const HEIGHT: i32 = 5;
const LENGTH: i32 = 3;
const HEAT_CAPACITY: u64 = 80_000;
const MAX_TEMP_C: u32 = 800;
const PRODUCT_CAPACITY: u32 = 16_000;
const MAX_CATALYST_USES: u32 = 50;
const HEAVY_OIL: &str = "arcforge:heavy_oil";
const HYDROGEN: &str = "arcforge:hydrogen";
const CATALYST: &str = "arcforge:cracking_catalyst";
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
    FACINGS.iter().copied().find(|value| value.name == name)
}

fn positions(controller: [i32; 3], facing: Facing) -> Vec<[i32; 3]> {
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

fn get_i32(nbt: &Compound, name: &str) -> i32 {
    let Tag::String(key) = Tag::String(name.into()) else {
        unreachable!()
    };
    match nbt.get(&key) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn set_i32(nbt: &mut Compound, name: &str, value: i32) -> bool {
    if get_i32(nbt, name) == value {
        return false;
    }
    let Tag::String(key) = Tag::String(name.into()) else {
        unreachable!()
    };
    nbt.insert(key, Tag::Int(value));
    true
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

fn set_string(nbt: &mut Compound, name: &str, value: &str) -> bool {
    if get_string(nbt, name).as_deref() == Some(value) {
        return false;
    }
    let Tag::String(key) = Tag::String(name.into()) else {
        unreachable!()
    };
    nbt.insert(key, Tag::String(value.into()));
    true
}

pub(crate) fn wrench(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    face: [i32; 3],
    player_uuid: u128,
) {
    let Some(entity) = host.block_entity(level, pos) else {
        return;
    };
    let Some(nbt) = entity.data(&*host) else {
        return;
    };

    let mut updates = Compound::new();
    let mode = if get_string(&nbt, "CrackingMode").as_deref() == Some("hydrocracking") {
        "hydrocracking"
    } else {
        "catalytic"
    };
    let message = if face == [0, 1, 0] {
        let next_mode = if mode == "catalytic" {
            "hydrocracking"
        } else {
            "catalytic"
        };
        set_string(&mut updates, "CrackingMode", next_mode);
        set_i32(&mut updates, "CrackingProgress", 0);
        format!("Cracking Unit mode: {next_mode}")
    } else if let Some(facing) = match face {
        [0, 0, -1] => Some("north"),
        [1, 0, 0] => Some("east"),
        [0, 0, 1] => Some("south"),
        [-1, 0, 0] => Some("west"),
        _ => None,
    } {
        host.set_block_property(pos, "facing", facing);
        set_i32(&mut updates, "CrackingScan", 0);
        set_i32(&mut updates, "CrackingProgress", 0);
        format!("Cracking Unit facing: {facing}")
    } else {
        format!("Cracking Unit mode: {mode}")
    };

    let _ = entity.merge_data(host, updates);
    if let Some(player) = host.player(player_uuid) {
        let _ = player.action_bar(host, Text::literal(message));
    }
}

fn set_property(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    key: &str,
    value: &str,
) -> bool {
    if block_property(&*host, level, pos, key).as_deref() == Some(value) {
        return false;
    }
    host.set_block_property(pos, key, value)
}

fn block_property(host: &dyn Host, level: LevelRef, pos: [i32; 3], key: &str) -> Option<String> {
    let state = level.block_state(host, pos)?;
    let metadata = host.block_states();
    state
        .property(&metadata, key)
        .ok()
        .flatten()
        .map(str::to_owned)
}

fn has_structure(
    host: &dyn Host,
    level: LevelRef,
    controller: [i32; 3],
    facing: Facing,
    controller_block: BlockKey,
    casing_block: BlockKey,
    reject_formed_casing: bool,
) -> bool {
    positions(controller, facing).into_iter().all(|pos| {
        if pos == controller {
            level.block(host, pos) == Some(controller_block)
        } else if level.block(host, pos) != Some(casing_block) {
            false
        } else {
            !reject_formed_casing
                || block_property(host, level, pos, "formed").as_deref() != Some("true")
        }
    })
}

fn mark_formed(
    host: &mut dyn ServerHost,
    level: LevelRef,
    controller: [i32; 3],
    facing: Facing,
    event: &mut BlockEntityTickEvent<'_>,
) -> bool {
    let mut changed = false;
    for pos in positions(controller, facing) {
        if pos != controller {
            set_property(host, level, pos, "formed", "true");
        }
    }
    set_property(host, level, controller, "formed", "true");
    changed |= set_i32(&mut event.entity.nbt, "CrackingFormed", 1);
    changed |= set_i32(&mut event.entity.nbt, "CrackingProgress", 0);
    changed |= set_i32(&mut event.entity.nbt, "CrackingScan", 0);
    changed |= set_string(&mut event.entity.nbt, "CrackingFacing", facing.name);
    changed
}

fn clear_formed(
    host: &mut dyn ServerHost,
    level: LevelRef,
    controller: [i32; 3],
    facing: Facing,
    casing_block: BlockKey,
    event: &mut BlockEntityTickEvent<'_>,
) -> bool {
    for pos in positions(controller, facing) {
        if pos != controller && level.block(host, pos) == Some(casing_block) {
            set_property(host, level, pos, "formed", "false");
        }
    }
    set_property(host, level, controller, "formed", "false");
    set_property(host, level, controller, "lit", "false");
    let mut changed = false;
    changed |= set_i32(&mut event.entity.nbt, "CrackingFormed", 0);
    changed |= set_i32(&mut event.entity.nbt, "CrackingProgress", 0);
    changed |= set_i32(&mut event.entity.nbt, "CrackingScan", 0);
    changed
}

macro_rules! java_string {
    ($value:expr) => {{
        let Tag::String(value) = Tag::String($value.into()) else {
            unreachable!()
        };
        value
    }};
}

fn fluid_tanks(nbt: &Compound) -> Vec<(Option<String>, u32)> {
    let Some(Tag::List(list)) = nbt.get(&java_string!("fluids")) else {
        return Vec::new();
    };
    list.iter()
        .map(|entry| match entry {
            Tag::Compound(map) => {
                let fluid = match map.get(&java_string!("Fluid")) {
                    Some(Tag::String(name)) if name.to_string_lossy() != "minecraft:empty" => {
                        Some(name.to_string_lossy())
                    }
                    _ => None,
                };
                let amount = match map.get(&java_string!("Amount")) {
                    Some(Tag::Int(value)) => (*value).max(0) as u32,
                    _ => 0,
                };
                (fluid, amount)
            }
            _ => (None, 0),
        })
        .collect()
}

fn fluid_amount(nbt: &Compound, index: usize) -> u32 {
    fluid_tanks(nbt).get(index).map_or(0, |(_, amount)| *amount)
}

fn fluid_name(nbt: &Compound, index: usize) -> Option<String> {
    fluid_tanks(nbt)
        .get(index)
        .and_then(|(fluid, amount)| (*amount > 0).then(|| fluid.clone()).flatten())
}

fn write_fluid_tanks(nbt: &mut Compound, tanks: &[(Option<String>, u32)]) {
    let list = tanks
        .iter()
        .map(|(fluid, amount)| {
            let mut tank = Compound::new();
            tank.insert(
                java_string!("Fluid"),
                Tag::String(java_string!(fluid.as_deref().unwrap_or("minecraft:empty"))),
            );
            tank.insert(
                java_string!("Amount"),
                Tag::Int((*amount).min(i32::MAX as u32) as i32),
            );
            Tag::Compound(tank)
        })
        .collect();
    nbt.insert(java_string!("fluids"), Tag::List(list));
}

fn fluid_insert(nbt: &mut Compound, index: usize, fluid: &str, amount: u32, capacity: u32) -> u32 {
    let mut tanks = fluid_tanks(nbt);
    if tanks.len() <= index {
        tanks.resize(index + 1, (None, 0));
    }
    let (current, stored) = &mut tanks[index];
    if current.as_deref().is_some_and(|name| name != fluid) {
        return 0;
    }
    let accepted = amount.min(capacity.saturating_sub(*stored));
    if accepted > 0 {
        *current = Some(fluid.to_owned());
        *stored += accepted;
        write_fluid_tanks(nbt, &tanks);
    }
    accepted
}

fn fluid_extract(nbt: &mut Compound, index: usize, fluid: &str, amount: u32) -> u32 {
    let mut tanks = fluid_tanks(nbt);
    let Some((current, stored)) = tanks.get_mut(index) else {
        return 0;
    };
    if current.as_deref() != Some(fluid) {
        return 0;
    }
    let extracted = amount.min(*stored);
    *stored -= extracted;
    if *stored == 0 {
        *current = None;
    }
    if extracted > 0 {
        write_fluid_tanks(nbt, &tanks);
    }
    extracted
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

fn recipe_from_json(json: &mod_api::serde_json::Value) -> Option<Recipe> {
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
            .and_then(mod_api::serde_json::Value::as_str)
            .map(str::to_owned),
        catalyst_uses: json
            .get("catalyst_uses")
            .and_then(mod_api::serde_json::Value::as_u64)
            .unwrap_or(0)
            .clamp(0, u64::from(u32::MAX)) as u32,
        ticks: json
            .get("time")
            .and_then(mod_api::serde_json::Value::as_u64)
            .unwrap_or(20)
            .clamp(1, u64::from(u32::MAX)) as u32,
        heat_per_tick: json
            .get("heat_per_tick")
            .and_then(mod_api::serde_json::Value::as_u64)
            .unwrap_or(0),
        min_temp: json
            .get("min_temp")
            .and_then(mod_api::serde_json::Value::as_u64)
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

fn speed_count(event: &BlockEntityTickEvent<'_>, host: &dyn Host) -> u32 {
    event
        .entity
        .items
        .get(SPEED_SLOT)
        .and_then(Option::as_ref)
        .filter(|item| item.item().id(host) == SPEED_UPGRADE)
        .map_or(0, |item| item.count().min(8))
}

fn speed_multiplier(upgrades: u32) -> f64 {
    2.0_f64.powf(f64::from(upgrades.min(8)) / 2.0)
}

fn total_ticks(recipe: &Recipe, upgrades: u32) -> u32 {
    (f64::from(recipe.ticks) / speed_multiplier(upgrades))
        .round()
        .clamp(1.0, f64::from(u32::MAX)) as u32
}

fn catalyst_uses(event: &BlockEntityTickEvent<'_>, host: &dyn Host) -> u32 {
    if event
        .entity
        .items
        .get(CATALYST_SLOT)
        .and_then(Option::as_ref)
        .is_none_or(|item| item.item().id(host) != CATALYST)
    {
        return 0;
    }
    get_i32(&event.entity.nbt, "CrackingCatalystUses").clamp(0, MAX_CATALYST_USES as i32) as u32
}

fn products_fit(event: &BlockEntityTickEvent<'_>, recipe: &Recipe) -> bool {
    recipe.products.len() == 4
        && recipe.products.iter().enumerate().all(|(index, product)| {
            let tank = OUTPUT_START + index;
            fluid_name(&event.entity.nbt, tank)
                .as_deref()
                .is_none_or(|existing| existing == product.fluid)
                && fluid_amount(&event.entity.nbt, tank).saturating_add(product.amount)
                    <= PRODUCT_CAPACITY
        })
}

fn set_status(event: &mut BlockEntityTickEvent<'_>, value: i32, duration: u32) -> bool {
    let mut changed = set_i32(&mut event.entity.nbt, "CrackingStatus", value);
    changed |= set_i32(
        &mut event.entity.nbt,
        "CrackingTotal",
        duration.min(i32::MAX as u32) as i32,
    );
    changed
}

fn set_lit(host: &mut dyn ServerHost, level: LevelRef, controller: [i32; 3], lit: bool) -> bool {
    set_property(
        host,
        level,
        controller,
        "lit",
        if lit { "true" } else { "false" },
    )
}

fn tick_machine(
    host: &mut dyn ServerHost,
    level: LevelRef,
    controller: [i32; 3],
    event: &mut BlockEntityTickEvent<'_>,
) -> bool {
    let mut changed = false;
    if event.entity.items.len() < 2 {
        event.entity.items.resize(2, None);
        changed = true;
    }
    if event
        .entity
        .items
        .get(CATALYST_SLOT)
        .is_some_and(Option::is_none)
    {
        changed |= set_i32(&mut event.entity.nbt, "CrackingCatalystUses", 0);
    }

    if get_i32(&event.entity.nbt, "CrackingFormed") == 0 {
        changed |= set_i32(&mut event.entity.nbt, "CrackingProgress", 0);
        changed |= set_status(event, 1, 0);
        changed |= set_lit(host, level, controller, false);
        return changed;
    }

    let selected_mode = match get_string(&event.entity.nbt, "CrackingMode").as_deref() {
        Some("hydrocracking") => "hydrocracking",
        _ => "catalytic",
    };
    let Some(recipe) = find_recipe(host, selected_mode) else {
        changed |= set_i32(&mut event.entity.nbt, "CrackingProgress", 0);
        changed |= set_status(event, 2, 0);
        changed |= set_lit(host, level, controller, false);
        return changed;
    };
    let upgrades = speed_count(event, host);
    let duration = total_ticks(&recipe, upgrades);
    if fluid_name(&event.entity.nbt, FEED_TANK).as_deref() != Some(recipe.input.as_str())
        || fluid_amount(&event.entity.nbt, FEED_TANK) < recipe.amount
        || recipe.second_input.as_ref().is_some_and(|(fluid, amount)| {
            fluid != HYDROGEN
                || fluid_name(&event.entity.nbt, HYDROGEN_TANK).as_deref() != Some(fluid)
                || fluid_amount(&event.entity.nbt, HYDROGEN_TANK) < *amount
        })
    {
        changed |= set_status(event, 3, duration);
        changed |= set_lit(host, level, controller, false);
        return changed;
    }

    if recipe.catalyst.as_deref() == Some(CATALYST) && catalyst_uses(event, host) == 0 {
        let catalyst_present = event
            .entity
            .items
            .get(CATALYST_SLOT)
            .and_then(Option::as_ref)
            .is_some_and(|item| item.item().id(host) == CATALYST);
        if catalyst_present {
            changed |= set_i32(
                &mut event.entity.nbt,
                "CrackingCatalystUses",
                MAX_CATALYST_USES as i32,
            );
        }
        if catalyst_uses(event, host) == 0 {
            changed |= set_status(event, 4, duration);
            changed |= set_lit(host, level, controller, false);
            return changed;
        }
    }

    if !products_fit(event, &recipe) {
        changed |= set_status(event, 5, duration);
        changed |= set_lit(host, level, controller, false);
        return changed;
    }

    let stored = get_i32(&event.entity.nbt, "HeatHU")
        .max(0)
        .min(HEAT_CAPACITY as i32) as u64;
    let temperature = 20 + ((stored * u64::from(MAX_TEMP_C - 20)) / HEAT_CAPACITY) as u32;
    let heat_cost = (recipe.heat_per_tick as f64 * speed_multiplier(upgrades)).ceil() as u64;
    if temperature < recipe.min_temp || stored < heat_cost {
        changed |= set_status(event, 6, duration);
        changed |= set_lit(host, level, controller, false);
        return changed;
    }

    changed |= set_i32(
        &mut event.entity.nbt,
        "HeatHU",
        stored.saturating_sub(heat_cost).min(i32::MAX as u64) as i32,
    );
    let progress = get_i32(&event.entity.nbt, "CrackingProgress").max(0) as u32 + 1;
    changed |= set_i32(
        &mut event.entity.nbt,
        "CrackingProgress",
        progress.min(i32::MAX as u32) as i32,
    );
    changed |= set_status(event, 0, duration);
    changed |= set_lit(host, level, controller, true);

    if progress >= duration {
        fluid_extract(
            &mut event.entity.nbt,
            FEED_TANK,
            &recipe.input,
            recipe.amount,
        );
        if let Some((fluid, amount)) = &recipe.second_input {
            fluid_extract(&mut event.entity.nbt, HYDROGEN_TANK, fluid, *amount);
        }
        for (index, product) in recipe.products.iter().enumerate() {
            fluid_insert(
                &mut event.entity.nbt,
                OUTPUT_START + index,
                &product.fluid,
                product.amount,
                PRODUCT_CAPACITY,
            );
        }
        if recipe.catalyst_uses > 0 && catalyst_uses(event, host) > 0 {
            let remaining = catalyst_uses(event, host).saturating_sub(recipe.catalyst_uses);
            if remaining == 0 {
                let should_remove = event
                    .entity
                    .items
                    .get(CATALYST_SLOT)
                    .and_then(Option::as_ref)
                    .is_some_and(|item| item.count() <= 1);
                if should_remove {
                    event.entity.items[CATALYST_SLOT] = None;
                } else if let Some(item) = event
                    .entity
                    .items
                    .get_mut(CATALYST_SLOT)
                    .and_then(Option::as_mut)
                {
                    let _ = item.set_count(item.count() - 1);
                }
            }
            changed |= set_i32(
                &mut event.entity.nbt,
                "CrackingCatalystUses",
                remaining as i32,
            );
        }
        changed |= set_i32(&mut event.entity.nbt, "CrackingProgress", 0);
        changed |= set_lit(host, level, controller, false);
    }
    changed
}

fn sync_menu_data(host: &dyn Host, event: &mut BlockEntityTickEvent<'_>) -> bool {
    let before = event.entity.data.clone();
    event.entity.data.resize(13, 0);
    let selected_mode = get_string(&event.entity.nbt, "CrackingMode");
    event.entity.data[0] = if selected_mode.as_deref() == Some("hydrocracking") {
        1
    } else {
        0
    };
    event.entity.data[1] = get_i32(&event.entity.nbt, "CrackingProgress").clamp(0, i16::MAX as i32);
    event.entity.data[2] = get_i32(&event.entity.nbt, "CrackingTotal").clamp(0, i16::MAX as i32);
    let stored = get_i32(&event.entity.nbt, "HeatHU")
        .max(0)
        .min(HEAT_CAPACITY as i32) as u64;
    event.entity.data[3] = (20 + ((stored * u64::from(MAX_TEMP_C - 20)) / HEAT_CAPACITY) as u32)
        .min(i16::MAX as u32) as i32;
    event.entity.data[4] = (stored / 10).min(i16::MAX as u64) as i32;
    event.entity.data[5] = get_i32(&event.entity.nbt, "CrackingStatus").clamp(0, i16::MAX as i32);
    event.entity.data[6] = find_recipe(
        host,
        if event.entity.data[0] == 1 {
            "hydrocracking"
        } else {
            "catalytic"
        },
    )
    .map_or(0, |recipe| recipe.min_temp.min(i16::MAX as u32) as i32);
    event.entity.data[7] = fluid_amount(&event.entity.nbt, FEED_TANK).min(i16::MAX as u32) as i32;
    event.entity.data[8] =
        fluid_amount(&event.entity.nbt, HYDROGEN_TANK).min(i16::MAX as u32) as i32;
    for index in 0..4 {
        event.entity.data[9 + index] =
            fluid_amount(&event.entity.nbt, OUTPUT_START + index).min(i16::MAX as u32) as i32;
    }
    event.entity.data != before
}

pub(crate) fn tick(
    host: &mut dyn ServerHost,
    event: &mut BlockEntityTickEvent<'_>,
    controller_block: BlockKey,
    casing_block: BlockKey,
) {
    *event.reply = BlockEntityTickReply::Unchanged;
    let Some(level) = host.level(host.dimension()) else {
        return;
    };
    let controller = event.pos;
    let facing_name =
        block_property(host, level, controller, "facing").unwrap_or_else(|| "north".to_owned());
    let current_facing = facing(&facing_name).unwrap_or(FACINGS[0]);
    let mut changed = false;

    if get_i32(&event.entity.nbt, "CrackingFormed") != 0 {
        let saved_facing = get_string(&event.entity.nbt, "CrackingFacing")
            .and_then(|name| facing(&name))
            .unwrap_or(current_facing);
        if saved_facing.name != current_facing.name
            || !has_structure(
                host,
                level,
                controller,
                saved_facing,
                controller_block,
                casing_block,
                false,
            )
        {
            changed |= clear_formed(host, level, controller, saved_facing, casing_block, event);
        } else {
            for pos in positions(controller, saved_facing) {
                if pos != controller {
                    set_property(host, level, pos, "formed", "true");
                }
            }
            set_property(host, level, controller, "formed", "true");
            set_property(host, level, controller, "lit", "false");
        }
    } else {
        set_property(host, level, controller, "formed", "false");
        set_property(host, level, controller, "lit", "false");
        let scan = get_i32(&event.entity.nbt, "CrackingScan").max(0);
        if scan == 0 {
            if has_structure(
                host,
                level,
                controller,
                current_facing,
                controller_block,
                casing_block,
                true,
            ) {
                changed |= mark_formed(host, level, controller, current_facing, event);
            } else {
                changed |= set_i32(&mut event.entity.nbt, "CrackingScan", 19);
            }
        } else {
            changed |= set_i32(&mut event.entity.nbt, "CrackingScan", scan - 1);
        }
    }

    changed |= tick_machine(host, level, controller, event);
    changed |= sync_menu_data(host, event);

    if changed {
        *event.reply = BlockEntityTickReply::Changed;
    }
}
