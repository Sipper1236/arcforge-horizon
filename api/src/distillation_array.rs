//! Arcforge Distillation Array: a bounded 2x2 or 3x3 refinery column.
use horizon_client::{
    containers::{ModBlockEntity, ModContainer},
    mod_api::{Compound, Host, ModFluidTanks, Tag, serde_json::Value},
};
use std::collections::BTreeMap;

pub(crate) const CONTROLLER: &str = "arcforge:distillation_array_controller";
pub(crate) const CASING: &str = "arcforge:distillation_array_casing";
pub(crate) const TRAY: &str = "arcforge:tray_level_casing";

const MIN_WIDTH: i32 = 2;
const MAX_WIDTH: i32 = 3;
const MIN_HEIGHT: i32 = 4;
const MAX_HEIGHT: i32 = 16;
const BASE_HEAT_HU: u64 = 40_000;
const BASE_FEED_RATE: u64 = 10;
const BASE_FEED_CAPACITY: u32 = 16_000;
const BASE_STEAM_CAPACITY: u32 = 8_000;
const BASE_OUTPUT_CAPACITY: u32 = 8_000;
// Horizon declares tank capacities by block ID, not block-entity position. Keep its
// exposed capacity at the smallest valid source column; rate and heat still scale.
const FEED_CAPACITY: u32 = BASE_FEED_CAPACITY * 4 * 4 * 4 / (4 * 8);
const STEAM_CAPACITY: u32 = BASE_STEAM_CAPACITY * 4 * 4 * 4 / (4 * 8);
const OUTPUT_CAPACITY: u32 = BASE_OUTPUT_CAPACITY * 4 * 4 * 4 / (4 * 8);
const MAX_TEMP_C: u32 = 1_400;
const MIN_SCAN_TICKS: i32 = 20;
const PITCH_SLOT: usize = 2;
const SPEED_SLOT: usize = 0;
const HEAT_SLOT: usize = 1;
const SPEED_UPGRADE: &str = "arcforge:speed_upgrade";
const HEAT_UPGRADE: &str = "arcforge:heat_upgrade";
const FEED_TANK: usize = 0;
const STEAM_TANK: usize = 1;
const OUTPUT_TANK_START: usize = 2;
const PRODUCT_FLUIDS: [&str; 7] = [
    "arcforge:refinery_gas",
    "arcforge:naphtha",
    "arcforge:kerosene",
    "arcforge:light_oil",
    "arcforge:diesel",
    "arcforge:heavy_oil",
    "arcforge:bitumen",
];
const STEAM_FLUIDS: [&str; 3] = [
    "arcforge:steam",
    "arcforge:high_pressure_steam",
    "arcforge:superheated_steam",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Column {
    min: [i32; 3],
    width: i32,
    height: i32,
}
impl Column {
    fn contains(self, pos: [i32; 3]) -> bool {
        pos[0] >= self.min[0]
            && pos[0] < self.min[0] + self.width
            && pos[1] >= self.min[1]
            && pos[1] < self.min[1] + self.height
            && pos[2] >= self.min[2]
            && pos[2] < self.min[2] + self.width
    }
    fn layer(self, pos: [i32; 3]) -> i32 {
        pos[1] - self.min[1]
    }
    fn outer(self, pos: [i32; 3]) -> bool {
        let x = pos[0] - self.min[0];
        let z = pos[2] - self.min[2];
        x == 0 || z == 0 || x == self.width - 1 || z == self.width - 1
    }
    fn positions(self) -> Vec<[i32; 3]> {
        let mut positions = Vec::with_capacity((self.width * self.width * self.height) as usize);
        for y in self.min[1]..self.min[1] + self.height {
            for x in self.min[0]..self.min[0] + self.width {
                for z in self.min[2]..self.min[2] + self.width {
                    positions.push([x, y, z]);
                }
            }
        }
        positions
    }
    fn max_y(self) -> i32 {
        self.min[1] + self.height - 1
    }
    fn controller_pos(self, pos: [i32; 3]) -> bool {
        pos[1] > self.min[1] && pos[1] < self.max_y() && self.outer(pos)
    }
}
#[derive(Clone, Debug)]
struct Fractions {
    fluids: Vec<(String, u32)>,
    pitch: u32,
}
#[derive(Clone, Debug)]
struct Stripping {
    steam_per_batch: u32,
    boosts: String,
    taken_from: Option<String>,
    bonus_bp: BTreeMap<String, u32>,
}
#[derive(Clone, Debug)]
struct Recipe {
    input: String,
    amount: u32,
    heat: u64,
    min_temp: u32,
    by_height: BTreeMap<i32, Fractions>,
    item_output: String,
    stripping: Option<Stripping>,
}
impl Recipe {
    fn fractions(&self, height: i32) -> Option<&Fractions> {
        self.by_height
            .range(..=height)
            .next_back()
            .map(|(_, fractions)| fractions)
    }
    fn outputs(&self, height: i32, bonus_bp: u32) -> Option<Vec<(String, u32)>> {
        let fractions = self.fractions(height)?;
        let mut outputs = fractions.fluids.clone();
        if bonus_bp > 0 {
            if let Some(stripping) = &self.stripping {
                let base = outputs
                    .iter()
                    .find(|(fluid, _)| fluid == &stripping.boosts)
                    .map_or(0, |(_, amount)| *amount);
                let mut extra = (u64::from(base) * u64::from(bonus_bp) + 5_000) / 10_000;
                if let Some(from) = &stripping.taken_from
                    && let Some((_, amount)) = outputs.iter_mut().find(|(fluid, _)| fluid == from)
                {
                    extra = extra.min(u64::from(*amount));
                    *amount -= extra as u32;
                }
                if extra > 0 {
                    if let Some((_, amount)) = outputs
                        .iter_mut()
                        .find(|(fluid, _)| fluid == &stripping.boosts)
                    {
                        *amount = amount.saturating_add(extra as u32);
                    } else {
                        outputs.push((stripping.boosts.clone(), extra as u32));
                    }
                }
            }
        }
        outputs.retain(|(_, amount)| *amount > 0);
        Some(outputs)
    }
    fn pitch_count(&self, height: i32) -> u32 {
        self.fractions(height)
            .map_or(0, |fractions| fractions.pitch)
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
    FACINGS.into_iter().find(|f| f.name == name)
}
fn vec_add(a: [i32; 3], b: [i32; 3]) -> [i32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
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
fn set_bool(host: &mut dyn Host, pos: [i32; 3], key: &str, value: bool) {
    let value = if value { "true" } else { "false" };
    if host.block_property(pos, key).as_deref() != Some(value) {
        host.set_block_property(pos, key, value);
    }
}
fn read_column(nbt: &Compound) -> Option<Column> {
    if get_i32(nbt, "DistillationFormed") == 0 {
        return None;
    }
    let width = get_i32(nbt, "DistillationWidth");
    let height = get_i32(nbt, "DistillationHeight");
    if !(MIN_WIDTH..=MAX_WIDTH).contains(&width) || !(MIN_HEIGHT..=MAX_HEIGHT).contains(&height) {
        return None;
    }
    Some(Column {
        min: [
            get_i32(nbt, "DistillationMinX"),
            get_i32(nbt, "DistillationMinY"),
            get_i32(nbt, "DistillationMinZ"),
        ],
        width,
        height,
    })
}
fn save_column(nbt: &mut Compound, column: Option<Column>) {
    set_i32(nbt, "DistillationFormed", i32::from(column.is_some()));
    if let Some(column) = column {
        set_i32(nbt, "DistillationMinX", column.min[0]);
        set_i32(nbt, "DistillationMinY", column.min[1]);
        set_i32(nbt, "DistillationMinZ", column.min[2]);
        set_i32(nbt, "DistillationWidth", column.width);
        set_i32(nbt, "DistillationHeight", column.height);
    }
}
fn block_is_part(host: &dyn Host, pos: [i32; 3]) -> bool {
    matches!(host.block(pos).as_str(), CONTROLLER | CASING | TRAY)
}
fn full_layer(host: &dyn Host, min_x: i32, y: i32, min_z: i32, width: i32) -> bool {
    for x in min_x..min_x + width {
        for z in min_z..min_z + width {
            if !block_is_part(host, [x, y, z]) {
                return false;
            }
        }
    }
    true
}
fn own_layer(
    host: &dyn Host,
    min_x: i32,
    y: i32,
    min_z: i32,
    width: i32,
    controller: [i32; 3],
    previous: Option<Column>,
) -> bool {
    for x in min_x..min_x + width {
        for z in min_z..min_z + width {
            let pos = [x, y, z];
            if (host.block(pos) == CONTROLLER && pos != controller)
                || (host.block_property(pos, "formed").as_deref() == Some("true")
                    && !previous.is_some_and(|column| column.contains(pos)))
            {
                return false;
            }
        }
    }
    true
}
fn valid_column(
    host: &dyn Host,
    column: Column,
    controller: [i32; 3],
    previous: Option<Column>,
) -> bool {
    if !column.contains(controller)
        || !column.controller_pos(controller)
        || host.block(controller) != CONTROLLER
    {
        return false;
    }
    for pos in column.positions() {
        let block = host.block(pos);
        if !matches!(block.as_str(), CONTROLLER | CASING | TRAY) {
            return false;
        }
        if pos != controller && block == CONTROLLER {
            return false;
        }
        let layer = column.layer(pos);
        if (block == CONTROLLER || block == TRAY) && (layer == 0 || layer == column.height - 1) {
            return false;
        }
        if host.block_property(pos, "formed").as_deref() == Some("true")
            && !previous.is_some_and(|old| old.contains(pos))
        {
            return false;
        }
    }
    true
}
fn find_column(host: &dyn Host, controller: [i32; 3], previous: Option<Column>) -> Option<Column> {
    let mut wide = false;
    for width in [MAX_WIDTH, MIN_WIDTH] {
        for dx in 0..width {
            for dz in 0..width {
                let min_x = controller[0] - dx;
                let min_z = controller[2] - dz;
                if !full_layer(host, min_x, controller[1], min_z, width) {
                    continue;
                }
                if width == MAX_WIDTH {
                    wide |= own_layer(
                        host,
                        min_x,
                        controller[1],
                        min_z,
                        width,
                        controller,
                        previous,
                    );
                }
                let mut bottom = controller[1];
                while bottom > controller[1] - MAX_HEIGHT
                    && full_layer(host, min_x, bottom - 1, min_z, width)
                {
                    bottom -= 1;
                }
                let mut top = controller[1];
                while top < controller[1] + MAX_HEIGHT
                    && full_layer(host, min_x, top + 1, min_z, width)
                {
                    top += 1;
                }
                let height = top - bottom + 1;
                if !(MIN_HEIGHT..=MAX_HEIGHT).contains(&height) {
                    continue;
                }
                let column = Column {
                    min: [min_x, bottom, min_z],
                    width,
                    height,
                };
                if valid_column(host, column, controller, previous) {
                    return Some(column);
                }
            }
        }
        if wide {
            return None;
        }
    }
    None
}
fn outward_facing(column: Column, pos: [i32; 3], current: &str) -> &'static str {
    let initial = facing(current).unwrap_or(FACINGS[0]);
    let index = FACINGS
        .iter()
        .position(|candidate| candidate.name == initial.name)
        .unwrap_or(0);
    for offset in [0, 2, 1, 3] {
        let f = FACINGS[(index + offset) % FACINGS.len()];
        let neighbor = vec_add(pos, f.vector);
        if !column.contains(neighbor) {
            return f.name;
        }
    }
    initial.name
}
fn tray_fraction(layer: i32, height: i32) -> &'static str {
    let sequence: &[&str] = if height >= 8 {
        &["heavy", "light", "naphtha", "vapor"]
    } else if height >= 6 {
        &["heavy", "naphtha", "vapor"]
    } else {
        &["naphtha", "vapor"]
    };
    sequence[((sequence.len() as i32 * layer) / height).min(sequence.len() as i32 - 1) as usize]
}
fn set_column(host: &mut dyn Host, column: Column, controller: [i32; 3], formed: bool) {
    let current = host
        .block_property(controller, "facing")
        .unwrap_or_else(|| "north".to_owned());
    let outward = outward_facing(column, controller, &current);
    for pos in column.positions() {
        let id = host.block(pos);
        if id == CONTROLLER {
            if host.block_property(pos, "facing").as_deref() != Some(outward) {
                host.set_block_property(pos, "facing", outward);
            }
            set_bool(host, pos, "formed", formed);
            set_bool(host, pos, "lit", false);
        } else if id == TRAY {
            let fraction = tray_fraction(column.layer(pos), column.height);
            if host.block_property(pos, "fraction").as_deref() != Some(fraction) {
                host.set_block_property(pos, "fraction", fraction);
            }
            set_bool(host, pos, "formed", formed);
            set_bool(host, pos, "lit", false);
        } else if id == CASING {
            set_bool(host, pos, "formed", formed);
        }
    }
}
fn unform_parts(host: &mut dyn Host, column: Column) {
    for pos in column.positions() {
        match host.block(pos).as_str() {
            CONTROLLER | TRAY => {
                set_bool(host, pos, "formed", false);
                set_bool(host, pos, "lit", false);
            }
            CASING => set_bool(host, pos, "formed", false),
            _ => {}
        }
    }
}
fn heat_capacity(column: Option<Column>) -> u64 {
    let Some(column) = column else {
        return BASE_HEAT_HU;
    };
    BASE_HEAT_HU * column.width as u64 * column.width as u64 * column.height as u64 / 16
}
fn scaled_rate(column: Column) -> u32 {
    (BASE_FEED_RATE * column.width as u64 * column.width as u64 * column.height as u64 / 16)
        .max(1)
        .min(u64::from(u32::MAX)) as u32
}
fn recipe_from_json(json: &Value) -> Option<Recipe> {
    let input_object = json.get("input")?;
    let input = input_object.get("fluid")?.as_str()?.to_owned();
    let amount = input_object
        .get("amount")
        .and_then(Value::as_u64)
        .unwrap_or(1_000)
        .clamp(1, u64::from(u32::MAX)) as u32;
    let heat = json.get("heat")?.as_u64()?.max(1);
    let min_temp = json
        .get("min_temp")
        .and_then(Value::as_u64)
        .unwrap_or(350)
        .min(u64::from(u32::MAX)) as u32;
    let item_output = json.get("item_output")?.as_str()?.to_owned();
    let by_height_json = json.get("by_height")?.as_object()?;
    let mut by_height = BTreeMap::new();
    for (height, value) in by_height_json {
        let height = height.parse::<i32>().ok()?;
        let mut fluids = Vec::new();
        if let Some(outputs) = value.get("fluids").and_then(Value::as_object) {
            for (fluid, amount) in outputs {
                let amount = amount.as_u64()?.min(u64::from(u32::MAX)) as u32;
                if amount > 0 {
                    fluids.push((fluid.clone(), amount));
                }
            }
        }
        let pitch = value
            .get("pitch")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .min(u64::from(u32::MAX)) as u32;
        by_height.insert(height, Fractions { fluids, pitch });
    }
    let stripping = json
        .get("steam_stripping")
        .and_then(Value::as_object)
        .map(|raw| {
            let bonus_bp = raw
                .get("bonus")
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
                .filter_map(|(fluid, value)| {
                    let amount = (value.as_f64()? * 10_000.0).round().clamp(0.0, 100_000.0) as u32;
                    Some((fluid.clone(), amount))
                })
                .collect();
            Stripping {
                steam_per_batch: raw
                    .get("steam_per_batch")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    .min(u64::from(u32::MAX)) as u32,
                boosts: raw
                    .get("boosts")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                taken_from: raw
                    .get("taken_from")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                bonus_bp,
            }
        });
    Some(Recipe {
        input,
        amount,
        heat,
        min_temp,
        by_height,
        item_output,
        stripping,
    })
}
fn find_recipe(host: &dyn Host, fluid: &str) -> Option<Recipe> {
    host.recipes_of_type("arcforge:distilling")
        .iter()
        .filter_map(|custom| recipe_from_json(&custom.json))
        .find(|recipe| recipe.input == fluid)
}
fn product_tank(fluid: &str) -> Option<usize> {
    match fluid {
        "arcforge:refinery_gas" => Some(OUTPUT_TANK_START),
        "arcforge:naphtha" => Some(OUTPUT_TANK_START + 1),
        "arcforge:kerosene" | "arcforge:sour_kerosene" => Some(OUTPUT_TANK_START + 2),
        "arcforge:light_oil" => Some(OUTPUT_TANK_START + 3),
        "arcforge:diesel" | "arcforge:sour_diesel" => Some(OUTPUT_TANK_START + 4),
        "arcforge:heavy_oil" => Some(OUTPUT_TANK_START + 5),
        "arcforge:bitumen" => Some(OUTPUT_TANK_START + 6),
        _ => None,
    }
}
fn tank_capacity(tank: usize) -> u32 {
    match tank {
        FEED_TANK => FEED_CAPACITY,
        STEAM_TANK => STEAM_CAPACITY,
        OUTPUT_TANK_START..=8 => OUTPUT_CAPACITY,
        _ => 0,
    }
}
pub(crate) fn accepts(tank: usize, fluid: &str) -> bool {
    match tank {
        FEED_TANK => matches!(
            fluid,
            "arcforge:creosote" | "arcforge:crude_oil" | "arcforge:sour_crude_oil"
        ),
        STEAM_TANK => STEAM_FLUIDS.contains(&fluid),
        _ => false,
    }
}
pub(crate) fn fluid_tanks() -> ModFluidTanks {
    ModFluidTanks {
        tanks: vec![
            FEED_CAPACITY as i32,
            STEAM_CAPACITY as i32,
            OUTPUT_CAPACITY as i32,
            OUTPUT_CAPACITY as i32,
            OUTPUT_CAPACITY as i32,
            OUTPUT_CAPACITY as i32,
            OUTPUT_CAPACITY as i32,
            OUTPUT_CAPACITY as i32,
            OUTPUT_CAPACITY as i32,
        ],
        faces: [
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8],
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8],
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8],
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8],
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8],
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8],
        ],
        inputs: vec![FEED_TANK, STEAM_TANK],
        outputs: (OUTPUT_TANK_START..=8).collect(),
    }
}
pub(crate) fn container() -> ModContainer {
    ModContainer {
        slots: 3,
        faces: [
            vec![PITCH_SLOT],
            vec![PITCH_SLOT],
            vec![],
            vec![],
            vec![],
            vec![],
        ],
        inputs: vec![SPEED_SLOT, HEAT_SLOT],
        outputs: vec![PITCH_SLOT],
        menu: Some("arcforge:machine_3_slot".into()),
    }
}
pub(crate) fn item_allowed(slot: usize, item: &str, count: u32) -> bool {
    match slot {
        SPEED_SLOT => item == SPEED_UPGRADE && count <= 8,
        HEAT_SLOT => item == HEAT_UPGRADE && count <= 8,
        _ => false,
    }
}
fn speed_upgrades(entity: &ModBlockEntity) -> u32 {
    entity
        .items
        .get(SPEED_SLOT)
        .and_then(Option::as_ref)
        .filter(|item| item.item.as_ref() == SPEED_UPGRADE)
        .map_or(0, |item| item.count.min(8))
}
fn heat_upgrades(entity: &ModBlockEntity) -> u32 {
    entity
        .items
        .get(HEAT_SLOT)
        .and_then(Option::as_ref)
        .filter(|item| item.item.as_ref() == HEAT_UPGRADE)
        .map_or(0, |item| item.count.min(8))
}
fn fluid_outputs(recipe: &Recipe, height: i32, bonus_bp: u32) -> Option<Vec<(String, u32)>> {
    let outputs = recipe.outputs(height, bonus_bp)?;
    outputs
        .into_iter()
        .map(|(fluid, amount)| product_tank(&fluid).map(|_| (fluid, amount)))
        .collect()
}
fn outputs_have_room(
    entity: &ModBlockEntity,
    recipe: &Recipe,
    column: Column,
    bonus_bp: u32,
) -> bool {
    let Some(outputs) = fluid_outputs(recipe, column.height, bonus_bp) else {
        return false;
    };
    for (fluid, needed) in outputs {
        let Some(tank) = product_tank(&fluid) else {
            return false;
        };
        let current = crate::fluid::name(&entity.nbt, tank);
        let amount = crate::fluid::amount(&entity.nbt, tank);
        if current.as_deref().is_some_and(|current| current != fluid)
            || amount.saturating_add(needed) > tank_capacity(tank)
        {
            return false;
        }
    }
    let pitch = recipe.pitch_count(column.height);
    pitch == 0
        || crate::can_add(
            entity.items.get(PITCH_SLOT).and_then(Option::as_ref),
            &recipe.item_output,
            pitch,
        )
}
fn available_steam_bonus(entity: &ModBlockEntity, recipe: &Recipe) -> (u32, u32) {
    let Some(stripping) = &recipe.stripping else {
        return (0, 0);
    };
    if stripping.steam_per_batch == 0
        || crate::fluid::amount(&entity.nbt, STEAM_TANK) < stripping.steam_per_batch
    {
        return (0, 0);
    }
    let Some(fluid) = crate::fluid::name(&entity.nbt, STEAM_TANK) else {
        return (0, 0);
    };
    (
        stripping.steam_per_batch,
        *stripping.bonus_bp.get(&fluid).unwrap_or(&0),
    )
}
fn start_batch(entity: &mut ModBlockEntity, recipe: &Recipe, column: Column) -> bool {
    if crate::fluid::amount(&entity.nbt, FEED_TANK) < recipe.amount
        || recipe.fractions(column.height).is_none()
    {
        return false;
    }
    let (steam_used, bonus_bp) = available_steam_bonus(entity, recipe);
    if !outputs_have_room(entity, recipe, column, bonus_bp) {
        return false;
    }
    let Some(feed) = crate::fluid::name(&entity.nbt, FEED_TANK) else {
        return false;
    };
    if crate::fluid::extract(&mut entity.nbt, FEED_TANK, &feed, recipe.amount) != recipe.amount {
        return false;
    }
    if steam_used > 0 {
        if let Some(fluid) = crate::fluid::name(&entity.nbt, STEAM_TANK) {
            crate::fluid::extract(&mut entity.nbt, STEAM_TANK, &fluid, steam_used);
        }
    }
    set_string(&mut entity.nbt, "DistillationBatchFeed", &feed);
    set_i32(&mut entity.nbt, "DistillationProgress", 0);
    set_i32(&mut entity.nbt, "DistillationHeatRemainder", 0);
    set_i32(&mut entity.nbt, "DistillationBatchBonus", bonus_bp as i32);
    true
}
fn finish_batch(entity: &mut ModBlockEntity, recipe: &Recipe, column: Column, bonus_bp: u32) {
    if let Some(outputs) = fluid_outputs(recipe, column.height, bonus_bp) {
        for (fluid, amount) in outputs {
            if let Some(tank) = product_tank(&fluid) {
                crate::fluid::insert(&mut entity.nbt, tank, &fluid, amount, tank_capacity(tank));
            }
        }
    }
    let pitch = recipe.pitch_count(column.height);
    if pitch > 0 {
        crate::add_items(&mut entity.items[PITCH_SLOT], &recipe.item_output, pitch);
    }
    set_i32(&mut entity.nbt, "DistillationProgress", 0);
    set_i32(&mut entity.nbt, "DistillationBatchBonus", 0);
    set_i32(&mut entity.nbt, "DistillationHeatRemainder", 0);
    set_i32(&mut entity.nbt, "DistillationStatus", 0);
    set_string(&mut entity.nbt, "DistillationLastFeed", &recipe.input);
    let Tag::String(key) = Tag::String("DistillationBatchFeed".into()) else {
        unreachable!()
    };
    entity.nbt.remove(&key);
}
pub(crate) fn tick(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let facing_name = host
        .block_property(pos, "facing")
        .unwrap_or_else(|| "north".to_owned());
    let old = read_column(&entity.nbt);
    let elapsed = get_i32(&entity.nbt, "DistillationScan").saturating_add(1);
    let facing_changed =
        get_string(&entity.nbt, "DistillationFacing").as_deref() != Some(&facing_name);
    let rescan = elapsed >= MIN_SCAN_TICKS
        || facing_changed
        || get_i32(&entity.nbt, "DistillationInit") == 0;
    let column = if rescan {
        set_i32(&mut entity.nbt, "DistillationScan", 0);
        set_i32(&mut entity.nbt, "DistillationInit", 1);
        set_string(&mut entity.nbt, "DistillationFacing", &facing_name);
        find_column(host, pos, old)
    } else {
        set_i32(&mut entity.nbt, "DistillationScan", elapsed);
        old
    };
    if column != old || facing_changed {
        if let Some(previous) = old {
            unform_parts(host, previous);
        }
        save_column(&mut entity.nbt, column);
        if let Some(column) = column {
            set_column(host, column, pos, true);
        }
    }
    let Some(column) = column else {
        set_i32(&mut entity.nbt, "DistillationStatus", 1);
        set_i32(&mut entity.nbt, "DistillationRate", 0);
        set_bool(host, pos, "lit", false);
        return;
    };
    let cap = heat_capacity(Some(column));
    let stored = (get_i32(&entity.nbt, "HeatHU").max(0) as u64).min(cap);
    set_i32(
        &mut entity.nbt,
        "HeatHU",
        stored.min(i32::MAX as u64) as i32,
    );
    let heat_store = crate::heat::HeatStore::new(cap, MAX_TEMP_C, stored);
    let temperature = heat_store.temperature_c();
    let speed = speed_upgrades(entity);
    let rate = ((scaled_rate(column) as f64) * 2.0_f64.powf(f64::from(speed) / 2.0))
        .round()
        .max(1.0)
        .min(u32::MAX as f64) as u32;
    entity.items.resize(3, None);
    let batch_feed = get_string(&entity.nbt, "DistillationBatchFeed");
    let feed = batch_feed
        .clone()
        .or_else(|| crate::fluid::name(&entity.nbt, FEED_TANK));
    let Some(feed) = feed else {
        set_i32(&mut entity.nbt, "DistillationStatus", 2);
        set_i32(&mut entity.nbt, "DistillationRate", 0);
        set_bool(host, pos, "lit", false);
        return;
    };
    let Some(recipe) = find_recipe(host, &feed) else {
        if batch_feed.is_some() {
            let Tag::String(key) = Tag::String("DistillationBatchFeed".into()) else {
                unreachable!()
            };
            entity.nbt.remove(&key);
            set_i32(&mut entity.nbt, "DistillationProgress", 0);
            set_i32(&mut entity.nbt, "DistillationBatchBonus", 0);
        }
        set_i32(&mut entity.nbt, "DistillationStatus", 2);
        set_i32(&mut entity.nbt, "DistillationRate", 0);
        set_bool(host, pos, "lit", false);
        return;
    };
    if recipe.fractions(column.height).is_none() {
        set_i32(&mut entity.nbt, "DistillationStatus", 3);
        set_i32(&mut entity.nbt, "DistillationRate", 0);
        set_bool(host, pos, "lit", false);
        return;
    }
    if batch_feed.is_none() {
        if crate::fluid::amount(&entity.nbt, FEED_TANK) < recipe.amount {
            set_i32(&mut entity.nbt, "DistillationStatus", 2);
            set_i32(&mut entity.nbt, "DistillationRate", 0);
            set_bool(host, pos, "lit", false);
            return;
        }
        let (_, bonus) = available_steam_bonus(entity, &recipe);
        if !outputs_have_room(entity, &recipe, column, bonus) {
            set_i32(&mut entity.nbt, "DistillationStatus", 4);
            set_i32(&mut entity.nbt, "DistillationRate", 0);
            set_bool(host, pos, "lit", false);
            return;
        }
        if !start_batch(entity, &recipe, column) {
            set_i32(&mut entity.nbt, "DistillationStatus", 4);
            set_i32(&mut entity.nbt, "DistillationRate", 0);
            set_bool(host, pos, "lit", false);
            return;
        }
    }
    if temperature < recipe.min_temp {
        set_i32(&mut entity.nbt, "DistillationStatus", 5);
        set_i32(&mut entity.nbt, "DistillationRate", 0);
        set_bool(host, pos, "lit", false);
        return;
    }
    let progress = get_i32(&entity.nbt, "DistillationProgress").max(0) as u32;
    let fed = rate.min(recipe.amount.saturating_sub(progress));
    let heat_upgrades = heat_upgrades(entity);
    let heat_multiplier = 0.8_f64.powf(f64::from(heat_upgrades));
    let heat_total = (recipe.heat as f64 * heat_multiplier).round().max(1.0) as u64;
    let heat_denominator = u64::from(recipe.amount);
    let remainder = get_i32(&entity.nbt, "DistillationHeatRemainder").max(0) as u64;
    let accumulated = u64::from(fed)
        .saturating_mul(heat_total)
        .saturating_add(remainder);
    let heat_needed = accumulated / heat_denominator;
    if stored < heat_needed {
        set_i32(&mut entity.nbt, "DistillationStatus", 5);
        set_i32(&mut entity.nbt, "DistillationRate", 0);
        set_bool(host, pos, "lit", false);
        return;
    }
    let new_remainder = accumulated % heat_denominator;
    set_i32(
        &mut entity.nbt,
        "HeatHU",
        stored.saturating_sub(heat_needed).min(i32::MAX as u64) as i32,
    );
    set_i32(
        &mut entity.nbt,
        "DistillationHeatRemainder",
        new_remainder as i32,
    );
    let new_progress = progress.saturating_add(fed);
    set_i32(
        &mut entity.nbt,
        "DistillationProgress",
        new_progress.min(i32::MAX as u32) as i32,
    );
    set_i32(
        &mut entity.nbt,
        "DistillationRate",
        fed.min(i32::MAX as u32) as i32,
    );
    set_i32(&mut entity.nbt, "DistillationStatus", 0);
    set_bool(host, pos, "lit", true);
    for part in column.positions() {
        if host.block(part) == TRAY {
            set_bool(host, part, "lit", true);
        }
    }
    if new_progress >= recipe.amount {
        let bonus = get_i32(&entity.nbt, "DistillationBatchBonus").max(0) as u32;
        finish_batch(entity, &recipe, column, bonus);
        set_bool(host, pos, "lit", false);
        for part in column.positions() {
            if host.block(part) == TRAY {
                set_bool(host, part, "lit", false);
            }
        }
    }
}
pub(crate) fn accept_heat(
    host: &mut dyn Host,
    pos: [i32; 3],
    source: &mut crate::heat::HeatStore,
    rate: u64,
) -> u64 {
    let Some(mut entity) = host.mod_block_entity(pos) else {
        return 0;
    };
    if entity.id != CONTROLLER {
        return 0;
    }
    let column = read_column(&entity.nbt);
    let cap = heat_capacity(column);
    let mut sink = crate::heat::HeatStore::new(
        cap,
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
pub(crate) fn notify_placed(host: &mut dyn Host, placed: [i32; 3]) {
    for dy in -MAX_HEIGHT..=MAX_HEIGHT {
        for dx in -MAX_WIDTH..=MAX_WIDTH {
            for dz in -MAX_WIDTH..=MAX_WIDTH {
                let pos = [placed[0] + dx, placed[1] + dy, placed[2] + dz];
                if host
                    .mod_block_entity(pos)
                    .is_some_and(|entity| entity.id == CONTROLLER)
                {
                    host.change_mod_block_entity(pos, &mut |entity| {
                        if entity.id == CONTROLLER {
                            set_i32(&mut entity.nbt, "DistillationScan", MIN_SCAN_TICKS);
                        }
                    });
                }
            }
        }
    }
}
pub(crate) fn unform(host: &mut dyn Host, pos: [i32; 3], nbt: &Compound) {
    if let Some(column) = read_column(nbt) {
        unform_parts(host, column);
    }
    set_bool(host, pos, "formed", false);
}
pub(crate) fn unform_near(host: &mut dyn Host, broken: [i32; 3]) {
    for dy in -MAX_HEIGHT..=MAX_HEIGHT {
        for dx in -MAX_WIDTH..=MAX_WIDTH {
            for dz in -MAX_WIDTH..=MAX_WIDTH {
                let pos = [broken[0] + dx, broken[1] + dy, broken[2] + dz];
                let Some(controller) = host.mod_block_entity(pos) else {
                    continue;
                };
                if controller.id != CONTROLLER {
                    continue;
                }
                let Some(column) = read_column(&controller.nbt) else {
                    continue;
                };
                if !column.contains(broken) {
                    continue;
                }
                unform_parts(host, column);
                host.change_mod_block_entity(pos, &mut |entity| {
                    if entity.id == CONTROLLER {
                        save_column(&mut entity.nbt, None);
                        set_i32(&mut entity.nbt, "DistillationInit", 0);
                    }
                });
                host.block_entity_sync(pos);
            }
        }
    }
}
pub(crate) fn face(host: &mut dyn Host, pos: [i32; 3], name: &str) {
    if facing(name).is_none() || host.block(pos) != CONTROLLER {
        return;
    }
    host.set_block_property(pos, "facing", name);
    host.change_mod_block_entity(pos, &mut |entity| {
        if entity.id == CONTROLLER {
            set_i32(&mut entity.nbt, "DistillationScan", MIN_SCAN_TICKS);
        }
    });
    host.block_entity_sync(pos);
}
pub(crate) fn status(entity: &ModBlockEntity) -> String {
    let state = match get_i32(&entity.nbt, "DistillationStatus") {
        1 => "needs a 2x2/3x3 column, 4-16 high",
        2 => "needs a whole valid feed batch",
        3 => "column is too short for this feed",
        4 => "product tank or Pitch slot is full",
        5 => "heating or needs more HU",
        _ => "distilling",
    };
    let width = get_i32(&entity.nbt, "DistillationWidth").max(0);
    let height = get_i32(&entity.nbt, "DistillationHeight").max(0);
    let cap = heat_capacity(read_column(&entity.nbt));
    let heat = (get_i32(&entity.nbt, "HeatHU").max(0) as u64).min(cap);
    let feed_name =
        crate::fluid::name(&entity.nbt, FEED_TANK).unwrap_or_else(|| "empty".to_owned());
    let progress = get_i32(&entity.nbt, "DistillationProgress").max(0);
    let feed_amount = crate::fluid::amount(&entity.nbt, FEED_TANK);
    let steam_amount = crate::fluid::amount(&entity.nbt, STEAM_TANK);
    format!(
        "Distillation Array: {state} | {width}x{width}x{height} | {feed_name} {feed_amount}mB ({progress}/1000) | Heat {heat}/{cap} HU | Steam {steam_amount}mB"
    )
}
