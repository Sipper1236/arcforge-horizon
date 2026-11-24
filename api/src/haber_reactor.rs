//! Arcforge Haber Reactor: combines recipe gases into Ammonia using FE and heat.
use horizon_client::{
    containers::{ModBlockEntity, ModContainer},
    mod_api::{Compound, Direction, Host, ModEnergyBuffer, ModFluidTanks, Tag, serde_json::Value},
};

pub(crate) const BLOCK: &str = "arcforge:haber_reactor";

const INPUT_A: usize = 0;
const INPUT_B: usize = 1;
const OUTPUT_TANK: usize = 2;
const TANK_CAPACITY: u32 = 8_000;
const ENERGY_CAPACITY: u64 = 40_000;
const MAX_ENERGY_INPUT: u64 = 400;
const HEAT_CAPACITY: u64 = 20_000;
const MAX_TEMPERATURE_C: u32 = 800;
const OUTPUT_RATE: u32 = 200;
const SPEED_SLOT: usize = 0;
const ENERGY_SLOT: usize = 1;
const SPEED_UPGRADE: &str = "arcforge:speed_upgrade";
const ENERGY_UPGRADE: &str = "arcforge:energy_upgrade";

struct FluidAmount {
    fluid: String,
    amount: u32,
}

struct Recipe {
    inputs: Vec<FluidAmount>,
    output: FluidAmount,
    ticks: u32,
    energy_per_tick: u32,
    heat_per_tick: u32,
    min_temperature: u32,
}

fn parse_fluid(value: &Value) -> Option<FluidAmount> {
    Some(FluidAmount {
        fluid: value.get("fluid")?.as_str()?.to_owned(),
        amount: value.get("amount")?.as_u64()?.clamp(1, u64::from(u32::MAX)) as u32,
    })
}

fn recipe_from_json(json: &Value) -> Option<Recipe> {
    let inputs = json.get("inputs")?.as_array()?;
    if !(1..=2).contains(&inputs.len()) {
        return None;
    }
    let inputs = inputs.iter().map(parse_fluid).collect::<Option<Vec<_>>>()?;
    let output = json.get("output")?;
    let output = FluidAmount {
        fluid: output.get("id")?.as_str()?.to_owned(),
        amount: output
            .get("amount")?
            .as_u64()?
            .clamp(1, u64::from(u32::MAX)) as u32,
    };
    Some(Recipe {
        inputs,
        output,
        ticks: json
            .get("time")
            .and_then(Value::as_u64)
            .unwrap_or(20)
            .clamp(1, u64::from(u32::MAX)) as u32,
        energy_per_tick: json
            .get("energy_per_tick")
            .and_then(Value::as_u64)
            .unwrap_or(60)
            .clamp(1, u64::from(u32::MAX)) as u32,
        heat_per_tick: json
            .get("heat_per_tick")
            .and_then(Value::as_u64)
            .unwrap_or(10)
            .min(u64::from(u32::MAX)) as u32,
        min_temperature: json
            .get("min_temperature")
            .and_then(Value::as_u64)
            .unwrap_or(450)
            .min(u64::from(u32::MAX)) as u32,
    })
}

fn input_matches(entity: &ModBlockEntity, tank: usize, input: &FluidAmount) -> bool {
    crate::fluid::name(&entity.nbt, tank).as_deref() == Some(input.fluid.as_str())
        && crate::fluid::amount(&entity.nbt, tank) >= input.amount
}

/// Return the input tank supplying each recipe input, allowing the two gases in either order.
fn tanks_for(entity: &ModBlockEntity, recipe: &Recipe) -> Option<Vec<usize>> {
    match recipe.inputs.as_slice() {
        [input] => {
            if input_matches(entity, INPUT_A, input) {
                Some(vec![INPUT_A])
            } else if input_matches(entity, INPUT_B, input) {
                Some(vec![INPUT_B])
            } else {
                None
            }
        }
        [first, second] => {
            if input_matches(entity, INPUT_A, first) && input_matches(entity, INPUT_B, second) {
                Some(vec![INPUT_A, INPUT_B])
            } else if input_matches(entity, INPUT_B, first)
                && input_matches(entity, INPUT_A, second)
            {
                Some(vec![INPUT_B, INPUT_A])
            } else {
                None
            }
        }
        _ => None,
    }
}

fn find_recipe(host: &dyn Host, entity: &ModBlockEntity) -> Option<(Recipe, Vec<usize>)> {
    host.recipes_of_type("arcforge:synthesizing")
        .iter()
        .filter_map(|recipe| recipe_from_json(&recipe.json))
        .find_map(|recipe| tanks_for(entity, &recipe).map(|tanks| (recipe, tanks)))
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
    get_i32(&entity.nbt, "HaberReactorProgress").max(0) as u32
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
    crate::speed_upgrade_time(recipe.ticks, upgrades(entity, SPEED_SLOT, SPEED_UPGRADE)).max(1)
}

fn output_fits(entity: &ModBlockEntity, recipe: &Recipe) -> bool {
    let existing = crate::fluid::name(&entity.nbt, OUTPUT_TANK);
    let amount = crate::fluid::amount(&entity.nbt, OUTPUT_TANK);
    existing
        .as_deref()
        .is_none_or(|name| name == recipe.output.fluid)
        && amount.saturating_add(recipe.output.amount) <= TANK_CAPACITY
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
    entity.data[5] = crate::fluid::amount(&entity.nbt, INPUT_A).min(i32::MAX as u32) as i32;
    entity.data[6] = crate::fluid::amount(&entity.nbt, INPUT_B).min(i32::MAX as u32) as i32;
    entity.data[7] = crate::fluid::amount(&entity.nbt, OUTPUT_TANK).min(i32::MAX as u32) as i32;
    entity.data[8] = energy_rate.min(i32::MAX as u32) as i32;
    entity.data[9] = heat_rate.min(i32::MAX as u32) as i32;
    entity.data[10] = TANK_CAPACITY.min(i32::MAX as u32) as i32;
    entity.data[11] = status;
}

fn set_status(
    entity: &mut ModBlockEntity,
    status: i32,
    energy_rate: u32,
    heat_rate: u32,
    total: u32,
) {
    set_i32(&mut entity.nbt, "HaberReactorStatus", status);
    set_i32(
        &mut entity.nbt,
        "HaberReactorEnergyRate",
        energy_rate.min(i32::MAX as u32) as i32,
    );
    set_i32(
        &mut entity.nbt,
        "HaberReactorHeatRate",
        heat_rate.min(i32::MAX as u32) as i32,
    );
    set_i32(
        &mut entity.nbt,
        "HaberReactorTotal",
        total.min(i32::MAX as u32) as i32,
    );
    sync_screen(entity, energy_rate, heat_rate, total, status);
}

/// Push ammonia from the default north-facing Gas Output (west) at the source rate.
fn push_output(host: &mut dyn Host, pos: [i32; 3]) {
    let dimension = host.dimension().to_owned();
    let west = [pos[0] - 1, pos[1], pos[2]];
    let _ = host.move_fluids(
        &dimension,
        pos,
        Direction::West,
        west,
        Direction::East,
        OUTPUT_RATE,
    );
}

pub(crate) fn tick(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    entity.items.resize(2, None);
    let stored_heat = (get_i32(&entity.nbt, "HeatHU").max(0) as u64).min(HEAT_CAPACITY);
    let stored_energy = (get_i32(&entity.nbt, "energy").max(0) as u64).min(ENERGY_CAPACITY);
    set_i32(&mut entity.nbt, "HeatHU", stored_heat as i32);
    set_i32(&mut entity.nbt, "energy", stored_energy as i32);
    let heat = crate::heat::HeatStore::new(HEAT_CAPACITY, MAX_TEMPERATURE_C, stored_heat);
    let has_input = crate::fluid::amount(&entity.nbt, INPUT_A) > 0
        || crate::fluid::amount(&entity.nbt, INPUT_B) > 0;

    let Some((recipe, input_tanks)) = find_recipe(host, entity) else {
        set_i32(&mut entity.nbt, "HaberReactorProgress", 0);
        set_status(entity, if has_input { 2 } else { 1 }, 0, 0, 0);
        set_lit(host, pos, false);
        push_output(host, pos);
        return;
    };

    let total = duration(&recipe, entity);
    if !output_fits(entity, &recipe) {
        set_status(entity, 3, 0, 0, total);
        set_lit(host, pos, false);
        push_output(host, pos);
        return;
    }

    let speed_upgrades = upgrades(entity, SPEED_SLOT, SPEED_UPGRADE);
    let energy_upgrades = upgrades(entity, ENERGY_SLOT, ENERGY_UPGRADE);
    let speed = crate::speed_upgrade_multiplier(speed_upgrades);
    let heat_cost = (f64::from(recipe.heat_per_tick) * speed).ceil() as u64;
    if heat.temperature_c() < recipe.min_temperature || stored_heat < heat_cost {
        set_status(entity, 4, 0, 0, total);
        set_lit(host, pos, false);
        push_output(host, pos);
        return;
    }

    let energy_cost =
        crate::scaled_upgrade_cost(u64::from(recipe.energy_per_tick), speed, energy_upgrades);
    let mut energy = crate::energy::EnergyStore::new(ENERGY_CAPACITY, stored_energy);
    if energy.stored_fe() < energy_cost {
        set_status(entity, 5, 0, 0, total);
        set_lit(host, pos, false);
        push_output(host, pos);
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
        "HaberReactorProgress",
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
        for (input, tank) in recipe.inputs.iter().zip(input_tanks) {
            crate::fluid::extract(&mut entity.nbt, tank, &input.fluid, input.amount);
        }
        crate::fluid::insert(
            &mut entity.nbt,
            OUTPUT_TANK,
            &recipe.output.fluid,
            recipe.output.amount,
            TANK_CAPACITY,
        );
        set_i32(&mut entity.nbt, "HaberReactorProgress", 0);
        set_status(entity, 0, 0, 0, total);
        set_lit(host, pos, false);
    }

    push_output(host, pos);
}

pub(crate) fn accepts(tank: usize, fluid: &str) -> bool {
    matches!(tank, INPUT_A | INPUT_B) && crate::fluid_conduit::is_arcforge_gas(fluid)
}

pub(crate) fn fluid_tanks() -> ModFluidTanks {
    ModFluidTanks {
        tanks: vec![
            TANK_CAPACITY as i32,
            TANK_CAPACITY as i32,
            TANK_CAPACITY as i32,
        ],
        // Fixed to source default north-facing ports: inputs up/east, output west.
        faces: [
            vec![],
            vec![INPUT_A],
            vec![],
            vec![],
            vec![OUTPUT_TANK],
            vec![INPUT_B],
        ],
        inputs: vec![INPUT_A, INPUT_B],
        outputs: vec![OUTPUT_TANK],
    }
}

pub(crate) fn energy_buffer() -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: ENERGY_CAPACITY as i32,
        max_insert: MAX_ENERGY_INPUT as i32,
        max_extract: 0,
        // Fixed to the source default facing (north): FE enters through the back (south).
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

pub(crate) fn item_allowed(slot: usize, item: &str, count: u32) -> bool {
    count <= 8
        && ((slot == SPEED_SLOT && item == SPEED_UPGRADE)
            || (slot == ENERGY_SLOT && item == ENERGY_UPGRADE))
}

pub(crate) fn accept_heat(
    host: &mut dyn Host,
    pos: [i32; 3],
    source_pos: [i32; 3],
    source: &mut crate::heat::HeatStore,
    rate: u64,
) -> u64 {
    // The source machine has a bottom heat face; the pinned hook supplies positions, not faces.
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

pub(crate) fn face(host: &mut dyn Host, pos: [i32; 3], name: &str) {
    if matches!(name, "north" | "east" | "south" | "west") {
        host.set_block_property(pos, "facing", name);
    }
}

pub(crate) fn status(entity: &ModBlockEntity) -> String {
    let stored_heat = get_i32(&entity.nbt, "HeatHU").max(0) as u64;
    let temperature =
        crate::heat::HeatStore::new(HEAT_CAPACITY, MAX_TEMPERATURE_C, stored_heat).temperature_c();
    let energy = get_i32(&entity.nbt, "energy").max(0);
    let state = match get_i32(&entity.nbt, "HaberReactorStatus") {
        1 => "needs Hydrogen and Nitrogen",
        2 => "no matching synthesizing recipe",
        3 => "Ammonia output full",
        4 => "heating or needs HU",
        5 => "needs FE",
        _ => "synthesizing Ammonia",
    };
    let input_a = crate::fluid::name(&entity.nbt, INPUT_A).unwrap_or_else(|| "empty".into());
    let input_b = crate::fluid::name(&entity.nbt, INPUT_B).unwrap_or_else(|| "empty".into());
    format!(
        "Haber Reactor: {state} | {input_a} {}/{}mB | {input_b} {}/{}mB | Ammonia {}/{}mB | {}/{} ticks | {temperature}°C | Heat {stored_heat}/{HEAT_CAPACITY} HU | FE {energy}/{ENERGY_CAPACITY}",
        crate::fluid::amount(&entity.nbt, INPUT_A),
        TANK_CAPACITY,
        crate::fluid::amount(&entity.nbt, INPUT_B),
        TANK_CAPACITY,
        crate::fluid::amount(&entity.nbt, OUTPUT_TANK),
        TANK_CAPACITY,
        progress(entity),
        get_i32(&entity.nbt, "HaberReactorTotal").max(0),
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

    text!("Haber Reactor", 8.0, 6.0, 0xFFFFFFFF);
    text!("Speed", 38.0, 17.0, 0xFFBBBBBB);
    text!("Energy", 122.0, 17.0, 0xFFBBBBBB);
    text!("Input A", 8.0, 44.0, 0xFFBBBBBB);
    text!("Input B", 90.0, 44.0, 0xFFBBBBBB);
    text!("Ammonia", 8.0, 77.0, 0xFFBBBBBB);

    let energy = data.first().copied().unwrap_or_default().max(0);
    let heat = data.get(1).copied().unwrap_or_default().max(0);
    let temperature = data.get(2).copied().unwrap_or_default().max(0);
    let progress = data.get(3).copied().unwrap_or_default().max(0);
    let duration = data.get(4).copied().unwrap_or_default().max(0);
    let input_a = data.get(5).copied().unwrap_or_default().max(0);
    let input_b = data.get(6).copied().unwrap_or_default().max(0);
    let output = data.get(7).copied().unwrap_or_default().max(0);
    let energy_rate = data.get(8).copied().unwrap_or_default().max(0);
    let heat_rate = data.get(9).copied().unwrap_or_default().max(0);
    let capacity = data.get(10).copied().unwrap_or(TANK_CAPACITY as i32).max(0);
    let state = match data.get(11).copied().unwrap_or_default() {
        1 => "Waiting for Hydrogen and Nitrogen",
        2 => "No matching recipe",
        3 => "Ammonia output full",
        4 => "Heating or needs HU",
        5 => "Waiting for FE",
        _ if duration > 0 => "Synthesizing Ammonia",
        _ => "Idle",
    };

    text!(
        &format!("{input_a}/{TANK_CAPACITY} mB"),
        8.0,
        58.0,
        0xFFE0E0E0
    );
    text!(
        &format!("{input_b}/{TANK_CAPACITY} mB"),
        90.0,
        58.0,
        0xFFE0E0E0
    );
    text!(&format!("{output}/{capacity} mB"), 8.0, 91.0, 0xFFE0E0E0);
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
