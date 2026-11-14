//! Horizon-native gameplay port of Arcforge machine systems.
//! Arcforge-derived behavior and data retain their upstream MIT attribution.

use horizon_client::{
    containers::{ModBlockEntity, ModContainer},
    mod_api::{
        BlockEntityContainerEvent, BlockEntityEvent, BlockEntityTickEvent, BlockEntityTicksEvent,
        BlocksEvent, ContainerAcceptsEvent, Direction, Flow, Hooks, Host, ItemEnergyBuffer,
        ItemEnergyBufferEvent, ItemFluidTankEvent, ItemStack, ItemsEvent, LootDropsEvent,
        MenuSlotRulesEvent, NativeScreen, RandomSource, Tag, Text, serde_json,
    },
};
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex},
    time::{Duration, Instant},
};

// Infer the concrete registry types from the public BlocksEvent and ItemsEvent
// vectors. The pinned native API exposes those event fields without exposing
// the horizon-mods crate as a dependency to mod code.
macro_rules! register_content {
    ($entries:expr, $($field:ident: $value:expr),+ $(,)?) => {{
        let mut entry = default_registry_entry($entries);
        $(entry.$field = $value;)+
        $entries.push(entry);
    }};
}

fn default_registry_entry<T: Default>(_: &[T]) -> T {
    T::default()
}

mod crushing_catalog;
mod energy;
mod energy_conduit;
mod fluid;
mod fluid_conduit;
mod heat;
mod heat_cell;
mod item_conduit;
mod pressurized_cylinder;
mod solar;
mod thermal;

const MACHINE: &str = "arcforge:steelworks";
const STEEL: &str = "arcforge:steel_ingot";
const IRON: &str = "minecraft:iron_ingot";
const COKE: &str = "arcforge:coal_coke";
const COKE_BLOCK: &str = "arcforge:coal_coke_block";
const SLAG: &str = "arcforge:slag";
const COMBUSTION: &str = "arcforge:combustion_plant";
const CARBONIZER: &str = "arcforge:carbonizer";
const CARBON_DUST: &str = "arcforge:carbon_dust";
const ARC_CRUSHER: &str = "arcforge:arc_crusher";
const ARC_CRUSHING_ARRAY: &str = "arcforge:arc_crushing_array";
const ARC_CRUSHING_ARRAY_CASING: &str = "arcforge:arc_crushing_array_casing";
const IRON_DUST: &str = "arcforge:iron_dust";
const COPPER_DUST: &str = "arcforge:copper_dust";
const GOLD_DUST: &str = "arcforge:gold_dust";
const SILICA_DUST: &str = "arcforge:silica_dust";
const INDUCTION_FURNACE: &str = "arcforge:induction_furnace";
const METAL_PRESS: &str = "arcforge:metal_press";
const PLATE_DIE: &str = "arcforge:plate_die";
const GEAR_DIE: &str = "arcforge:gear_die";
const ROD_DIE: &str = "arcforge:rod_die";
const PELLET_DIE: &str = "arcforge:pellet_die";
const SIFTER: &str = "arcforge:sifter";
const INFUSER: &str = "arcforge:infuser";
const FIREBOX: &str = "arcforge:firebox";
const FIREBOX_ARRAY_CONTROLLER: &str = "arcforge:firebox_array_controller";
const FIREBOX_ARRAY_CASING: &str = "arcforge:firebox_array_casing";
const PRESSURE_GLASS: &str = "arcforge:pressure_glass";
const SOLAR_ARRAY_CONTROLLER: &str = solar::CONTROLLER;
const SOLAR_ARRAY_CASING: &str = solar::CASING;
const SOLAR_COLLECTOR: &str = solar::COLLECTOR;
const SOLAR_ARRAY_HEAT_CAPACITY: u64 = 16_000;
const SOLAR_ARRAY_MAX_TEMPERATURE: u32 = 1_100;
const SOLAR_ARRAY_SCAN_INTERVAL_TICKS: i32 = 40;
const GEOTHERMAL_PLANT: &str = "arcforge:geothermal_plant";
const FUEL_BURNER: &str = "arcforge:fuel_burner";
const WROUGHT_FLUID_TANK: &str = "arcforge:wrought_fluid_tank";
const TEMPERED_FLUID_TANK: &str = "arcforge:tempered_fluid_tank";
const HARDENED_FLUID_TANK: &str = "arcforge:hardened_fluid_tank";
const ARCFORGED_FLUID_TANK: &str = "arcforge:arcforged_fluid_tank";
const RESIN_TAP: &str = "arcforge:resin_tap";
const FIBERIZER: &str = "arcforge:fiberizer";
const VULCANIZER: &str = "arcforge:vulcanizer";
const GRAIN_DRYER: &str = "arcforge:grain_dryer";
const THERMOELECTRIC_PLANT: &str = "arcforge:thermoelectric_plant";
const THERMOELECTRIC_UPGRADE: &str = "arcforge:thermoelectric_upgrade";
const THERMOELECTRIC_MAX_UPGRADES: u32 = 8;
const THERMOELECTRIC_UPGRADE_EFFICIENCY: f64 = 0.0625;
const HEAT_UPGRADE: &str = "arcforge:heat_upgrade";
const HEAT_MAX_UPGRADES: u32 = 8;
const HEAT_OUTPUT_PER_UPGRADE: f64 = 0.125;
const HEAT_EFFICIENCY_COST_FACTOR: f64 = 0.8;
const ENERGY_UPGRADE: &str = "arcforge:energy_upgrade";
const ENERGY_MAX_UPGRADES: u32 = 8;
const SPEED_UPGRADE: &str = "arcforge:speed_upgrade";
const SPEED_MAX_UPGRADES: u32 = 8;
const GEOTHERMAL_HEAT_CAPACITY: u64 = 60_000;
const GEOTHERMAL_MAX_TEMPERATURE: u32 = 1_400;
const GEOTHERMAL_LAVA_TANK_CAPACITY: u32 = 8_000;
const GEOTHERMAL_LAVA_HEAT_HU_PER_MB: u32 = 80;
const GEOTHERMAL_LAVA_PER_BURN: u32 = 100;
const GEOTHERMAL_LAVA_SOURCE_HEAT_HU_PER_TICK: u32 = 40;
const GEOTHERMAL_MAGMA_HEAT_HU_PER_TICK: u32 = 16;
const GEOTHERMAL_SCAN_INTERVAL_TICKS: i32 = 20;
const FIREBOX_ARRAY_MAX_SIZE: i32 = 7;
const FIREBOX_ARRAY_CHECK_INTERVAL: i32 = 40;
const FIREBOX_ARRAY_HEAT_AT_MAX_SIZE: u64 = 50_400;
const FIREBOX_ARRAY_HEAT_CAPACITY_PER_BLOCK: u64 = 10_000;
const FIREBOX_ARRAY_MAX_TEMPERATURE: u32 = 1_400;
const FIREBOX_ARRAY_SOLID_TEMPERATURE: u32 = 1_100;
const FIREBOX_ARRAY_COKE_TEMPERATURE: u32 = 1_300;
const WROUGHT_ENERGY_CONDUIT: &str = energy_conduit::WROUGHT;
const TEMPERED_ENERGY_CONDUIT: &str = energy_conduit::TEMPERED;
const HARDENED_ENERGY_CONDUIT: &str = energy_conduit::HARDENED;
const ARCFORGED_ENERGY_CONDUIT: &str = energy_conduit::ARCFORGED;
const WROUGHT_THERMAL_CONDUIT: &str = thermal::WROUGHT;
const TEMPERED_THERMAL_CONDUIT: &str = thermal::TEMPERED;
const HARDENED_THERMAL_CONDUIT: &str = thermal::HARDENED;
const ARCFORGED_THERMAL_CONDUIT: &str = thermal::ARCFORGED;
const ENERGY_CONDUIT_CAPACITY_FE: u64 = 50_000;
const WROUGHT_ENERGY_CELL: &str = "arcforge:wrought_energy_cell";
const TEMPERED_ENERGY_CELL: &str = "arcforge:tempered_energy_cell";
const HARDENED_ENERGY_CELL: &str = "arcforge:hardened_energy_cell";
const ARCFORGED_ENERGY_CELL: &str = "arcforge:arcforged_energy_cell";
const WROUGHT_BATTERY: &str = "arcforge:wrought_battery";
const TEMPERED_BATTERY: &str = "arcforge:tempered_battery";
const HARDENED_BATTERY: &str = "arcforge:hardened_battery";
const ARCFORGED_BATTERY: &str = "arcforge:arcforged_battery";
const WRENCH: &str = "arcforge:wrench";
const FURNACE_PORT: &str = "arcforge:arcforge_furnace_port";
const FURNACE_BRICKS: &str = "arcforge:arcforge_furnace_bricks";
const FURNACE_BRICK_WALL: &str = "arcforge:arcforge_furnace_brick_wall";
const VULCANIZER_HEAT_CAPACITY: u64 = 8_000;
const VULCANIZER_MAX_TEMPERATURE: u32 = 400;
const VULCANIZER_MIN_TEMPERATURE: u32 = 140;
const VULCANIZER_TICKS: u32 = 200;
const VULCANIZER_HU_PER_TICK: u64 = 12;
const GRAIN_DRYER_HEAT_CAPACITY: u64 = 4_000;
const GRAIN_DRYER_MAX_TEMPERATURE: u32 = 250;
const GRAIN_DRYER_MIN_TEMPERATURE: u32 = 60;
const GRAIN_DRYER_TANK_CAPACITY: u32 = 4_000;
const THERMOELECTRIC_HEAT_CAPACITY: u64 = 20_000;
const THERMOELECTRIC_MAX_TEMPERATURE: u32 = 1_400;
const THERMOELECTRIC_FE_CAPACITY: u64 = 50_000;
const THERMOELECTRIC_MAX_OUTPUT_FE: u64 = 200;
const WROUGHT_FLUID_TANK_CAPACITY: u32 = 16_000;
const TEMPERED_FLUID_TANK_CAPACITY: u32 = 64_000;
const HARDENED_FLUID_TANK_CAPACITY: u32 = 256_000;
const ARCFORGED_FLUID_TANK_CAPACITY: u32 = 1_024_000;
const FUEL_BURNER_HEAT_CAPACITY: u64 = 40_000;
const FUEL_BURNER_MAX_TEMPERATURE: u32 = 1_400;
const FUEL_BURNER_TANK_CAPACITY: u32 = 8_000;
const FUEL_BURNER_CREOSOTE_HU_PER_MB: u32 = 120;
const FUEL_BURNER_CREOSOTE_MILLI_MB_PER_TICK: u32 = 500;
const FUEL_BURNER_CREOSOTE_TEMPERATURE: u32 = 850;
const CREOSOTE_BUCKET: &str = "arcforge:creosote_bucket";
const LATEX_BUCKET: &str = "arcforge:latex_bucket";
const WROUGHT_CANISTER: &str = "arcforge:wrought_canister";
const TEMPERED_CANISTER: &str = "arcforge:tempered_canister";
const HARDENED_CANISTER: &str = "arcforge:hardened_canister";
const ARCFORGED_CANISTER: &str = "arcforge:arcforged_canister";
const PINE_RESIN: &str = "arcforge:pine_resin";
const PITCH: &str = "arcforge:pitch";
const STEEL_MESH: &str = "arcforge:steel_mesh";
const FIREBOX_BURN_SPEED: u32 = 2;
const RESIN_TAP_INTERVAL: i32 = 200;
const RESIN_TAP_LATEX_PER_DRIP: u32 = 25;
const RESIN_TAP_TANK_CAPACITY: u32 = 1000;
const RESIN_TAP_MAX_RESIN: u32 = 16;
const COMBUSTION_CAPACITY_FE: u64 = 50_000;
const COMBUSTION_FE_PER_TICK: u64 = 40;
const COMBUSTION_MAX_EXTRACT_FE: i32 = 200;
const COMBUSTION_BURN_SPEED: u32 = 2;
const COMBUSTION_WOOD_ASH_CHANCE: f32 = 0.5;
const WOOD_ASH: &str = "arcforge:wood_ash";
const INVAR_MESH: &str = "arcforge:invar_mesh";
const TUNGSTEN_MESH: &str = "arcforge:tungsten_mesh";
const TUNGSTEN_HEATING_COIL: &str = "arcforge:tungsten_heating_coil";
const SILICON_CARBIDE_MESH: &str = "arcforge:silicon_carbide_mesh";
const STEEL_PLATE: &str = "arcforge:steel_plate";
const STEEL_GEAR: &str = "arcforge:steel_gear";
const STEEL_ROD: &str = "arcforge:steel_rod";
const STEEL_BLOCK: &str = "arcforge:steel_block";
const BURN_TIME: i32 = 1600;

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Arcforge)
}

struct Arcforge;

#[derive(Default)]
struct ArcforgeFurnaceMenuRules {
    metal: Vec<horizon_client::mod_api::Ingredient>,
    additives: Vec<horizon_client::mod_api::Ingredient>,
}

static ARCFORGE_FURNACE_MENU_RULES: LazyLock<Mutex<ArcforgeFurnaceMenuRules>> =
    LazyLock::new(|| Mutex::new(ArcforgeFurnaceMenuRules::default()));

type CellChargeKey = (String, [i32; 3]);
#[derive(Default)]
struct PendingCellCharges {
    entries: HashMap<CellChargeKey, (String, i32, Instant)>,
    last_pruned: Option<Instant>,
}
static PENDING_CELL_CHARGES: LazyLock<Mutex<PendingCellCharges>> =
    LazyLock::new(|| Mutex::new(PendingCellCharges::default()));
const PENDING_CELL_CHARGE_LIFETIME: Duration = Duration::from_secs(2);

fn steel_equipment_components(
    durability: u32,
) -> std::collections::BTreeMap<String, serde_json::Value> {
    [
        (
            "minecraft:max_damage".to_owned(),
            serde_json::json!(durability),
        ),
        (
            "minecraft:enchantable".to_owned(),
            serde_json::json!({ "value": 12 }),
        ),
        (
            "minecraft:repairable".to_owned(),
            serde_json::json!({ "items": "#c:ingots/steel" }),
        ),
    ]
    .into_iter()
    .collect()
}

fn steel_attribute_modifiers(attributes: &[(&str, f64)], slot: &str) -> serde_json::Value {
    serde_json::Value::Array(
        attributes
            .iter()
            .map(|(attribute, amount)| {
                serde_json::json!({
                    "type": attribute,
                    "id": format!("arcforge:steel_{}", attribute.rsplit(':').next().unwrap()),
                    "amount": amount,
                    "operation": "add_value",
                    "slot": slot
                })
            })
            .collect(),
    )
}

impl Hooks for Arcforge {
    fn place_block(
        &self,
        host: &mut dyn Host,
        event: &mut horizon_client::mod_api::PlaceBlockEvent<'_>,
    ) {
        if thermal::spec(&event.stack.item).is_some() {
            thermal::invalidate_world(host);
        }
        if energy_conduit::spec(&event.stack.item).is_some() {
            energy_conduit::invalidate_world(host);
        }
        if fluid_conduit::spec(&event.stack.item).is_some() {
            fluid_conduit::invalidate_world(host);
            if let Some(entity) = host.mod_block_entity(event.pos) {
                fluid_conduit::connect_placed(host, event.pos, &entity);
            }
        }
        if item_conduit::spec(&event.stack.item).is_some()
            && let Some(entity) = host.mod_block_entity(event.pos)
        {
            item_conduit::invalidate_world(host);
            item_conduit::connect_placed(host, event.pos, &entity);
        }
        fluid_conduit::neighbor_placed(host, event.pos);
        if heat_cell::spec(&event.stack.item).is_some() {
            heat_cell::restore_from_item(host, event.pos, event.stack);
            host.set_block_property(event.pos, "heat", "0");
        }
        if pressurized_cylinder::cylinder_capacity(&event.stack.item).is_some() {
            pressurized_cylinder::restore_from_item(host, event.pos, event.stack);
            if let Some(level) = pressurized_cylinder::item_fill_level(event.stack) {
                host.set_block_property(event.pos, "level", &level);
            }
        }
        if let Some((capacity, _)) = energy_cell_spec(&event.stack.item) {
            let energy = item_energy_amount(event.stack, capacity);
            host.set_block_property(
                event.pos,
                "charge",
                &energy_cell_charge_level(energy, capacity).to_string(),
            );
            if energy > 0 {
                let item = event.stack.item.to_string();
                host.change_mod_block_entity(event.pos, &mut |entity| {
                    if entity.id == item {
                        set_i32(&mut entity.nbt, "energy", energy as i32);
                    }
                });
                host.block_entity_sync(event.pos);
            }
        }
        if host
            .mod_block_entity(event.pos)
            .is_some_and(|entity| entity.id == ARC_CRUSHING_ARRAY_CASING)
        {
            form_crushing_array_near(host, event.pos);
        }
    }

    fn destroy_block(
        &self,
        host: &mut dyn Host,
        event: &mut horizon_client::mod_api::DestroyBlockEvent,
    ) -> Flow {
        if let Some(entity) = host.mod_block_entity(event.pos) {
            if thermal::spec(&entity.id).is_some() {
                thermal::invalidate_world(host);
            }
            if energy_conduit::spec(&entity.id).is_some() {
                energy_conduit::invalidate_world(host);
            }
            if fluid_conduit::spec(&entity.id).is_some() {
                fluid_conduit::invalidate_world(host);
                fluid_conduit::disconnect_destroyed(host, event.pos, &entity);
            }
            if item_conduit::spec(&entity.id).is_some() {
                item_conduit::invalidate_world(host);
                item_conduit::disconnect_destroyed(host, event.pos, &entity);
            }
            if let Some((capacity, _)) = energy_cell_spec(&entity.id) {
                let stored = (get_i32(&entity.nbt, "energy").max(0) as u64).min(capacity);
                cache_cell_charge(host.dimension(), event.pos, &entity.id, stored as i32);
            }
            if heat_cell::spec(&entity.id).is_some() {
                heat_cell::cache_charge(host.dimension(), event.pos, &entity);
            }
            if entity.id == ARC_CRUSHING_ARRAY_CASING {
                unform_crushing_array_near(host, event.pos);
            }
            if entity.id == FIREBOX_ARRAY_CONTROLLER {
                unform_firebox_array(host, &entity.nbt);
            }
            if entity.id == SOLAR_ARRAY_CONTROLLER {
                if let Some(tower) = solar::tower_from_nbt(&entity.nbt) {
                    solar::set_formed(host, tower, false);
                }
            }
        }
        if matches!(
            host.block(event.pos).as_str(),
            FIREBOX_ARRAY_CASING | PRESSURE_GLASS
        ) {
            unform_firebox_array_near(host, event.pos);
        }
        if matches!(
            host.block(event.pos).as_str(),
            SOLAR_ARRAY_CASING | SOLAR_COLLECTOR
        ) {
            solar::unform_near(host, event.pos);
        }
        Flow::Continue
    }

    fn container_accepts(&self, host: &dyn Host, event: &mut ContainerAcceptsEvent<'_>) -> Flow {
        if event.entity.id == ARC_CRUSHING_ARRAY_CASING
            && host.block_property(event.pos, "part").as_deref() != Some("center")
        {
            return Flow::Cancel;
        }
        if event.entity.id == FURNACE_PORT {
            let valid = match event.slot {
                0 => arcforge_furnace_ingredient_accepts(host, event.stack, "metal"),
                1 | 2 => {
                    arcforge_furnace_ingredient_accepts(host, event.stack, "additive")
                        || arcforge_furnace_ingredient_accepts(host, event.stack, "additive_2")
                }
                3 => matches!(event.stack.item.as_ref(), COKE | COKE_BLOCK),
                _ => false,
            };
            return if valid { Flow::Continue } else { Flow::Cancel };
        }
        if event.entity.id == COMBUSTION {
            return if event.slot == 0 && combustion_fuel_ticks(event.stack.item.as_ref()).is_some()
            {
                Flow::Continue
            } else {
                Flow::Cancel
            };
        }
        if event.entity.id == FIREBOX {
            return if event.slot == 0 && firebox_fuel_ticks(event.stack.item.as_ref()).is_some() {
                Flow::Continue
            } else {
                Flow::Cancel
            };
        }
        if event.entity.id == FIREBOX_ARRAY_CONTROLLER {
            return if event.slot == 0 && firebox_array_fuel(event.stack.item.as_ref()).is_some() {
                Flow::Continue
            } else {
                Flow::Cancel
            };
        }
        if event.entity.id == GEOTHERMAL_PLANT {
            return if event.slot == 0 && event.stack.item.as_ref() == "minecraft:lava_bucket" {
                Flow::Continue
            } else {
                Flow::Cancel
            };
        }
        if event.entity.id == FUEL_BURNER {
            return if event.slot == 0
                && event.stack.item.as_ref() == CREOSOTE_BUCKET
                && item_fluid(event.stack)
                    .is_some_and(|(fluid, amount)| fluid == "arcforge:creosote" && amount >= 1000)
            {
                Flow::Continue
            } else {
                Flow::Cancel
            };
        }
        if event.entity.id == FIBERIZER {
            return if event.slot == 0 && fiberizer_recipe(host, event.stack).is_some() {
                Flow::Continue
            } else {
                Flow::Cancel
            };
        }
        if event.entity.id == VULCANIZER {
            let valid = match event.slot {
                0 => matches!(
                    event.stack.item.as_ref(),
                    "arcforge:raw_rubber" | "arcforge:synthetic_rubber"
                ),
                1 => event.stack.item.as_ref() == "arcforge:sulfur_dust",
                _ => false,
            };
            return if valid { Flow::Continue } else { Flow::Cancel };
        }
        if pressurized_cylinder::cylinder_capacity(&event.entity.id).is_some() {
            let valid = matches!(event.slot, 0 | 1)
                && pressurized_cylinder::cartridge_accepts_stack(event.stack);
            return if valid { Flow::Continue } else { Flow::Cancel };
        }
        if event.entity.id == GRAIN_DRYER {
            return if event.slot == 0 && drying_item_recipe(host, event.stack).is_some() {
                Flow::Continue
            } else {
                Flow::Cancel
            };
        }
        if event.entity.id == INFUSER {
            let valid = match event.slot {
                0 => {
                    event.stack.item.as_ref() == "minecraft:bucket"
                        || event.stack.item.as_ref() == CREOSOTE_BUCKET
                }
                2 => infuser_has_input(host, event.stack),
                4 => infuser_has_additive(host, event.stack),
                _ => false,
            };
            return if valid { Flow::Continue } else { Flow::Cancel };
        }
        if event.entity.id != SIFTER {
            return Flow::Continue;
        }
        let valid = match event.slot {
            0 => sifter_recipe(host, event.stack).is_some(),
            1 => sifter_mesh_tier(&event.stack.item).is_some(),
            _ => false,
        };
        if valid { Flow::Continue } else { Flow::Cancel }
    }

    fn menu_slot_rules(&self, event: &mut MenuSlotRulesEvent<'_>) {
        if event.menu != "arcforge:arcforge_furnace" || event.slot > 3 {
            return;
        }
        let rules = ARCFORGE_FURNACE_MENU_RULES
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let accepts = match event.slot {
            0 => rules
                .metal
                .iter()
                .any(|ingredient| ingredient.test(event.stack)),
            1 | 2 => rules
                .additives
                .iter()
                .any(|ingredient| ingredient.test(event.stack)),
            3 => matches!(event.stack.item.as_ref(), COKE | COKE_BLOCK),
            _ => false,
        };
        event.may_place &= accepts;
        if !accepts {
            event.max_stack = 1;
        }
    }

    fn use_item_on(
        &self,
        host: &mut dyn Host,
        event: &mut horizon_client::mod_api::UseItemOnEvent<'_>,
    ) -> Flow {
        if event.stack.item.as_ref() == SPEED_UPGRADE
            && host
                .mod_block_entity(event.pos)
                .is_some_and(|entity| supports_speed_upgrade(&entity.id))
        {
            let Some(player) = host.acting_player() else {
                return Flow::Continue;
            };
            return use_speed_upgrade(host, event.pos, event.stack, player.uuid);
        }
        if event.stack.item.as_ref() == ENERGY_UPGRADE
            && host
                .mod_block_entity(event.pos)
                .is_some_and(|entity| supports_energy_upgrade(&entity.id))
        {
            let Some(player) = host.acting_player() else {
                return Flow::Continue;
            };
            return use_energy_upgrade(host, event.pos, event.stack, player.uuid);
        }
        if event.stack.item.as_ref() == HEAT_UPGRADE {
            let Some(entity) = host.mod_block_entity(event.pos) else {
                return Flow::Continue;
            };
            if matches!(
                entity.id.as_str(),
                FIREBOX
                    | GEOTHERMAL_PLANT
                    | FUEL_BURNER
                    | THERMOELECTRIC_PLANT
                    | FIBERIZER
                    | VULCANIZER
                    | GRAIN_DRYER
            ) {
                let Some(player) = host.acting_player() else {
                    return Flow::Continue;
                };
                return use_heat_upgrade(host, event.pos, event.stack, player.uuid, &entity.id);
            }
        }
        if event.stack.item.as_ref() == THERMOELECTRIC_UPGRADE
            && host
                .mod_block_entity(event.pos)
                .is_some_and(|entity| entity.id == THERMOELECTRIC_PLANT)
        {
            let Some(player) = host.acting_player() else {
                return Flow::Continue;
            };
            return use_thermoelectric_upgrade(host, event.pos, event.stack, player.uuid);
        }
        if energy_item_spec(&event.stack.item).is_some()
            && host
                .mod_block_entity(event.pos)
                .is_some_and(|entity| energy_cell_spec(&entity.id).is_some())
        {
            let Some(player) = host.acting_player() else {
                return Flow::Continue;
            };
            return use_energy_item_on_cell(
                host,
                event.pos,
                event.stack,
                player.selected_slot as i32,
            );
        }
        if event.stack.item.as_ref() == "minecraft:bucket"
            || event.stack.item.as_ref() == LATEX_BUCKET
        {
            let Some(tap) = host.mod_block_entity(event.pos) else {
                return Flow::Continue;
            };
            if tap.id == RESIN_TAP {
                let player = host.acting_player().map(|player| player.uuid);
                return use_resin_tap_bucket(host, event.pos, event.stack, player);
            }
        }
        if event.stack.item.as_ref() == CREOSOTE_BUCKET
            && host
                .mod_block_entity(event.pos)
                .is_some_and(|entity| entity.id == FUEL_BURNER)
        {
            return use_fuel_burner_bucket(host, event.pos, event.stack);
        }
        if is_fluid_bucket(&event.stack.item) {
            if let Some(entity) = host.mod_block_entity(event.pos) {
                if fluid_tank_capacity(&entity.id).is_some() {
                    let Some(face) = fluid_bucket_face(event.face) else {
                        return Flow::Continue;
                    };
                    let actor = host.acting_player();
                    return use_fluid_tank_bucket(
                        host,
                        event.pos,
                        face,
                        event.stack,
                        actor.as_ref().map(|player| player.uuid),
                        actor.map_or(0, |player| player.selected_slot as i32),
                    );
                }
            }
        }
        if canister_spec(&event.stack.item).is_some()
            && host
                .mod_block_entity(event.pos)
                .is_some_and(|entity| fluid_tank_capacity(&entity.id).is_some())
        {
            let Some(face) = fluid_bucket_face(event.face) else {
                return Flow::Continue;
            };
            let Some(player) = host.acting_player() else {
                return Flow::Continue;
            };
            return use_arcforge_fluid_container(
                host,
                event.pos,
                face,
                event.stack,
                player.selected_slot as i32,
            );
        }
        if pressurized_cylinder::cartridge_spec(&event.stack.item).is_some()
            && host
                .mod_block_entity(event.pos)
                .is_some_and(|entity| pressurized_cylinder::cylinder_capacity(&entity.id).is_some())
        {
            let Some(face) = fluid_bucket_face(event.face) else {
                return Flow::Continue;
            };
            let Some(player) = host.acting_player() else {
                return Flow::Continue;
            };
            return use_gas_cartridge(
                host,
                event.pos,
                face,
                event.stack,
                player.selected_slot as i32,
            );
        }
        if event.stack.item.as_ref() != WRENCH {
            return Flow::Continue;
        }
        if host
            .mod_block_entity(event.pos)
            .is_some_and(|entity| pressurized_cylinder::cylinder_capacity(&entity.id).is_some())
        {
            show_cylinder_status(host, event.pos);
            return Flow::Cancel;
        }
        if host
            .mod_block_entity(event.pos)
            .is_some_and(|entity| energy_cell_spec(&entity.id).is_some())
        {
            show_energy_cell_status(host, event.pos);
            return Flow::Cancel;
        }
        if host.mod_block_entity(event.pos).is_some_and(|entity| {
            entity.id == ARC_CRUSHING_ARRAY_CASING
                && host.block_property(event.pos, "part").as_deref() == Some("center")
        }) {
            show_crushing_array_status(host, event.pos);
            return Flow::Cancel;
        }
        if host
            .mod_block_entity(event.pos)
            .is_some_and(|entity| entity.id == FURNACE_PORT)
        {
            show_arcforge_furnace_status(host, event.pos);
            return Flow::Cancel;
        }
        if host.mod_block_entity(event.pos).is_some_and(|entity| {
            matches!(
                entity.id.as_ref(),
                ARC_CRUSHER | INDUCTION_FURNACE | METAL_PRESS | SIFTER
            )
        }) {
            show_processor_status(host, event.pos);
            return Flow::Cancel;
        }
        if let Some((controller_pos, tower)) = solar::controller_for_part(host, event.pos) {
            let Some(side) = energy_conduit_side(event.face) else {
                return Flow::Continue;
            };
            let Some(controller) = host.mod_block_entity(controller_pos) else {
                return Flow::Continue;
            };
            let enable = !solar::port_enabled(&controller.nbt, event.pos, side.name);
            host.change_mod_block_entity(controller_pos, &mut |entity| {
                if entity.id == SOLAR_ARRAY_CONTROLLER {
                    solar::set_port_enabled(&mut entity.nbt, event.pos, side.name, enable);
                }
            });
            let adjacent = [
                event.pos[0] + side.offset[0],
                event.pos[1] + side.offset[1],
                event.pos[2] + side.offset[2],
            ];
            host.set_block_property(
                event.pos,
                &solar::port_property(side.name),
                if enable && !tower.contains(adjacent) {
                    "true"
                } else {
                    "false"
                },
            );
            host.block_entity_sync(controller_pos);
            return Flow::Cancel;
        }
        if thermal::cycle_face(host, event.pos, event.face) {
            return Flow::Cancel;
        }
        if item_conduit::cycle_face(host, event.pos, event.face) {
            return Flow::Cancel;
        }
        if fluid_conduit::cycle_face(host, event.pos, event.face) {
            return Flow::Cancel;
        }
        let Some(conduit) = host.mod_block_entity(event.pos) else {
            return Flow::Continue;
        };
        if energy_conduit_spec(&conduit.id).is_none() {
            if wrench_energy_output_machine(&conduit.id) {
                let Some(side) = energy_conduit_side(event.face) else {
                    return Flow::Continue;
                };
                let current = get_string(&conduit.nbt, &conduit_side_setting_key(side.name))
                    .unwrap_or_else(|| "auto".into());
                let next = next_conduit_setting(&current);
                set_conduit_side_setting(host, event.pos, side, next);
                return Flow::Cancel;
            }
            if !wrench_rotatable_machine(&conduit.id) {
                return Flow::Continue;
            }
            let Some(facing) = host.block_property(event.pos, "facing") else {
                return Flow::Continue;
            };
            let Some(next) = next_machine_facing(&facing) else {
                return Flow::Continue;
            };
            host.set_block_property(event.pos, "facing", next);
            return Flow::Cancel;
        }
        let Some(side) = energy_conduit_side(event.face) else {
            return Flow::Continue;
        };
        let neighbor_pos = [
            event.pos[0] + side.offset[0],
            event.pos[1] + side.offset[1],
            event.pos[2] + side.offset[2],
        ];
        let neighbor = host.mod_block_entity(neighbor_pos);
        let own_key = conduit_side_setting_key(side.name);
        let current = get_string(&conduit.nbt, &own_key).unwrap_or_else(|| "auto".into());
        let next = next_conduit_setting(&current);
        set_conduit_side_setting(host, event.pos, side, next);
        energy_conduit::invalidate_world(host);
        if neighbor
            .as_ref()
            .is_some_and(|neighbor| energy_conduit_spec(&neighbor.id).is_some())
        {
            let neighbor_side = energy_conduit_side(side.opposite).expect("opposite conduit side");
            let neighbor_setting = neighbor.as_ref().and_then(|entity| {
                get_string(&entity.nbt, &conduit_side_setting_key(neighbor_side.name))
            });
            let connected =
                energy_conduit::modes_connect(next, neighbor_setting.as_deref().unwrap_or("auto"));
            let connection = if connected { "pipe" } else { "none" };
            host.set_block_property(event.pos, side.name, connection);
            host.set_block_property(neighbor_pos, neighbor_side.name, connection);
            return Flow::Cancel;
        }
        let connection = conduit_connection_for(
            &next,
            neighbor.as_ref().map(|neighbor| neighbor.id.as_str()),
        );
        host.set_block_property(event.pos, side.name, connection);
        Flow::Cancel
    }

    fn right_click_block(
        &self,
        host: &mut dyn Host,
        event: &mut horizon_client::mod_api::RightClickBlockEvent,
    ) -> Flow {
        if host.held().is_some() {
            return Flow::Continue;
        }
        if host
            .mod_block_entity(event.pos)
            .is_some_and(|entity| entity.id == ARC_CRUSHING_ARRAY_CASING)
            && host.block_property(event.pos, "part").as_deref() != Some("center")
        {
            return Flow::Cancel;
        }
        let Some(tap) = host.mod_block_entity(event.pos) else {
            return Flow::Continue;
        };
        if tap.id != RESIN_TAP {
            return Flow::Continue;
        }
        let Some(resin) = tap
            .items
            .first()
            .and_then(Option::as_ref)
            .filter(|stack| stack.item.as_ref() == PINE_RESIN && stack.count > 0)
        else {
            return Flow::Continue;
        };

        let available = resin.count;

        let mut extracted = 0;
        let changed = host.change_mod_block_entity(event.pos, &mut |entity| {
            if entity.id != RESIN_TAP {
                return;
            }
            if let Some(Some(stack)) = entity.items.first_mut()
                && stack.item.as_ref() == PINE_RESIN
            {
                extracted = available.min(stack.count);
                consume_count(&mut entity.items[0], extracted);
            }
        });
        if !changed || extracted == 0 {
            return Flow::Continue;
        }

        let given = host.give_items(PINE_RESIN, extracted).min(extracted);
        let overflow = extracted - given;
        let dropped = if overflow == 0 {
            true
        } else {
            let mut nbt = horizon_client::mod_api::Compound::default();
            nbt.insert(
                "Item".into(),
                Tag::compound([
                    ("id", Tag::String(PINE_RESIN.into())),
                    ("count", Tag::Int(overflow as i32)),
                ]),
            );
            nbt.insert("PickupDelay".into(), Tag::Short(10));
            host.summon(
                host.dimension(),
                "minecraft:item",
                [
                    event.pos[0] as f64 + 0.5,
                    event.pos[1] as f64 + 0.5,
                    event.pos[2] as f64 + 0.5,
                ],
                nbt,
            )
            .is_some()
        };
        if !dropped {
            restore_resin_tap_resin(host, event.pos, overflow);
        }
        if given > 0 || dropped {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn blocks(&self, event: &mut BlocksEvent<'_>) {
        register_content!(&mut event.blocks, id: MACHINE.into(), template: "minecraft:blast_furnace".into(), item: true);
        register_content!(&mut event.blocks, id: CARBONIZER.into(), template: "minecraft:furnace".into(), item: true);
        register_content!(&mut event.blocks, id: COMBUSTION.into(), template: "minecraft:furnace".into(), item: true);
        register_content!(&mut event.blocks, id: ARC_CRUSHER.into(), template: "minecraft:blast_furnace".into(), item: true);
        register_content!(&mut event.blocks, id: ARC_CRUSHING_ARRAY.into(), template: "minecraft:stonecutter".into(), item: true);
        register_content!(&mut event.blocks, id: ARC_CRUSHING_ARRAY_CASING.into(), template: "minecraft:stone".into(), item: true);
        if let Some(block) = event
            .blocks
            .iter_mut()
            .find(|block| block.id == ARC_CRUSHING_ARRAY_CASING)
        {
            block.state_properties = serde_json::from_value(serde_json::json!({
                "facing": { "type": "enum", "values": ["north", "east", "south", "west"] },
                "lit": { "type": "boolean" },
                "part": { "type": "enum", "values": ["none", "center", "other"] }
            }))
            .expect("valid Crushing Array casing block states");
            block.default_state = [
                ("facing".to_owned(), "north".to_owned()),
                ("lit".to_owned(), "false".to_owned()),
                ("part".to_owned(), "none".to_owned()),
            ]
            .into_iter()
            .collect();
        }
        register_content!(&mut event.blocks, id: INDUCTION_FURNACE.into(), template: "minecraft:blast_furnace".into(), item: true);
        register_content!(&mut event.blocks, id: METAL_PRESS.into(), template: "minecraft:blast_furnace".into(), item: true);
        register_content!(&mut event.blocks, id: SIFTER.into(), template: "minecraft:blast_furnace".into(), item: true);
        register_content!(&mut event.blocks, id: INFUSER.into(), template: "minecraft:furnace".into(), item: true);
        register_content!(&mut event.blocks, id: FIREBOX.into(), template: "minecraft:blast_furnace".into(), item: true);
        register_content!(&mut event.blocks, id: FIREBOX_ARRAY_CONTROLLER.into(), template: "minecraft:blast_furnace".into(), item: true);
        register_content!(&mut event.blocks, id: PRESSURE_GLASS.into(), template: "minecraft:glass".into(), item: true);
        register_content!(&mut event.blocks, id: FIREBOX_ARRAY_CASING.into(), template: "minecraft:stone".into(), item: true);
        register_content!(&mut event.blocks, id: SOLAR_ARRAY_CONTROLLER.into(), template: "minecraft:blast_furnace".into(), item: true);
        register_content!(&mut event.blocks, id: SOLAR_ARRAY_CASING.into(), template: "minecraft:stone".into(), item: true);
        register_content!(&mut event.blocks, id: SOLAR_COLLECTOR.into(), template: "minecraft:glass".into(), item: true);
        register_content!(&mut event.blocks, id: FUEL_BURNER.into(), template: "minecraft:furnace".into(), item: true);
        register_content!(&mut event.blocks, id: GEOTHERMAL_PLANT.into(), template: "minecraft:blast_furnace".into(), item: true);
        register_content!(&mut event.blocks, id: WROUGHT_FLUID_TANK.into(), template: "minecraft:glass".into(), item: true);
        register_content!(&mut event.blocks, id: TEMPERED_FLUID_TANK.into(), template: "minecraft:glass".into(), item: true);
        register_content!(&mut event.blocks, id: HARDENED_FLUID_TANK.into(), template: "minecraft:glass".into(), item: true);
        register_content!(&mut event.blocks, id: ARCFORGED_FLUID_TANK.into(), template: "minecraft:glass".into(), item: true);
        for id in pressurized_cylinder::cylinder_ids() {
            register_content!(&mut event.blocks, id: id.into(), template: "minecraft:iron_block".into(), item: true);
            if let Some(block) = event.blocks.iter_mut().find(|block| block.id == id) {
                block.state_properties = serde_json::from_value(serde_json::json!({
                    "level": { "type": "enum", "values": ["0", "1", "2", "3", "4"] }
                }))
                .expect("valid Pressurized Cylinder gauge block state");
                block.default_state = [("level".to_owned(), "0".to_owned())].into_iter().collect();
            }
        }
        register_content!(&mut event.blocks, id: THERMOELECTRIC_PLANT.into(), template: "minecraft:blast_furnace".into(), item: true);
        register_content!(&mut event.blocks, id: RESIN_TAP.into(), template: "minecraft:tripwire_hook".into(), item: true);
        if let Some(block) = event.blocks.iter_mut().find(|block| block.id == RESIN_TAP) {
            block.state_properties = serde_json::from_value(serde_json::json!({
                "content": { "type": "enum", "values": ["empty", "latex", "resin"] }
            }))
            .expect("valid Resin Tap block states");
            block.default_state = [("content".to_owned(), "empty".to_owned())]
                .into_iter()
                .collect();
        }
        register_content!(&mut event.blocks, id: FIBERIZER.into(), template: "minecraft:blast_furnace".into(), item: true);
        register_content!(&mut event.blocks, id: VULCANIZER.into(), template: "minecraft:blast_furnace".into(), item: true);
        register_content!(&mut event.blocks, id: GRAIN_DRYER.into(), template: "minecraft:blast_furnace".into(), item: true);
        register_content!(&mut event.blocks, id: FURNACE_PORT.into(), template: "minecraft:blast_furnace".into(), item: true);
        register_content!(&mut event.blocks, id: FURNACE_BRICKS.into(), template: "minecraft:bricks".into(), item: true);
        register_content!(&mut event.blocks, id: FURNACE_BRICK_WALL.into(), template: "minecraft:brick_wall".into(), item: true);
        for id in [
            WROUGHT_ENERGY_CONDUIT,
            TEMPERED_ENERGY_CONDUIT,
            HARDENED_ENERGY_CONDUIT,
            ARCFORGED_ENERGY_CONDUIT,
        ] {
            register_content!(&mut event.blocks, id: id.into(), template: "minecraft:copper_block".into(), item: true);
        }
        for id in [
            item_conduit::WROUGHT,
            item_conduit::TEMPERED,
            item_conduit::HARDENED,
            item_conduit::ARCFORGED,
        ] {
            register_content!(&mut event.blocks, id: id.into(), template: "minecraft:copper_block".into(), item: true);
        }
        for id in [
            fluid_conduit::WROUGHT,
            fluid_conduit::TEMPERED,
            fluid_conduit::HARDENED,
            fluid_conduit::ARCFORGED,
            fluid_conduit::WROUGHT_PRESSURIZED,
            fluid_conduit::TEMPERED_PRESSURIZED,
            fluid_conduit::HARDENED_PRESSURIZED,
            fluid_conduit::ARCFORGED_PRESSURIZED,
        ] {
            register_content!(&mut event.blocks, id: id.into(), template: "minecraft:copper_block".into(), item: true);
        }
        for id in thermal::ids() {
            register_content!(&mut event.blocks, id: id.into(), template: "minecraft:copper_block".into(), item: true);
        }
        let mut set_custom_states =
            |id: &str, properties: serde_json::Value, default_state: Vec<(String, String)>| {
                if let Some(block) = event.blocks.iter_mut().find(|block| block.id == id) {
                    block.state_properties = serde_json::from_value(properties)
                        .expect("valid custom block state properties");
                    block.default_state = default_state.into_iter().collect();
                }
            };
        for id in [
            WROUGHT_ENERGY_CONDUIT,
            TEMPERED_ENERGY_CONDUIT,
            HARDENED_ENERGY_CONDUIT,
            ARCFORGED_ENERGY_CONDUIT,
            item_conduit::WROUGHT,
            item_conduit::TEMPERED,
            item_conduit::HARDENED,
            item_conduit::ARCFORGED,
            fluid_conduit::WROUGHT,
            fluid_conduit::TEMPERED,
            fluid_conduit::HARDENED,
            fluid_conduit::ARCFORGED,
            fluid_conduit::WROUGHT_PRESSURIZED,
            fluid_conduit::TEMPERED_PRESSURIZED,
            fluid_conduit::HARDENED_PRESSURIZED,
            fluid_conduit::ARCFORGED_PRESSURIZED,
            thermal::WROUGHT,
            thermal::TEMPERED,
            thermal::HARDENED,
            thermal::ARCFORGED,
        ] {
            let mut properties = serde_json::json!({
                "down": { "type": "enum", "values": ["none", "pipe", "input", "output"] },
                "up": { "type": "enum", "values": ["none", "pipe", "input", "output"] },
                "north": { "type": "enum", "values": ["none", "pipe", "input", "output"] },
                "south": { "type": "enum", "values": ["none", "pipe", "input", "output"] },
                "west": { "type": "enum", "values": ["none", "pipe", "input", "output"] },
                "east": { "type": "enum", "values": ["none", "pipe", "input", "output"] }
            });
            let active = energy_conduit_spec(id).is_some()
                || matches!(
                    id,
                    fluid_conduit::WROUGHT_PRESSURIZED
                        | fluid_conduit::TEMPERED_PRESSURIZED
                        | fluid_conduit::HARDENED_PRESSURIZED
                        | fluid_conduit::ARCFORGED_PRESSURIZED
                )
                || matches!(
                    id,
                    thermal::WROUGHT | thermal::TEMPERED | thermal::HARDENED | thermal::ARCFORGED
                );
            let mut defaults = ["down", "up", "north", "south", "west", "east"]
                .map(|face| (face.to_owned(), "none".to_owned()))
                .to_vec();
            if active {
                properties["active"] = serde_json::json!({ "type": "boolean" });
                defaults.push(("active".to_owned(), "false".to_owned()));
            }
            set_custom_states(id, properties, defaults);
        }

        let solar_ports = serde_json::json!({
            "port_down": { "type": "boolean" },
            "port_up": { "type": "boolean" },
            "port_north": { "type": "boolean" },
            "port_south": { "type": "boolean" },
            "port_west": { "type": "boolean" },
            "port_east": { "type": "boolean" }
        });
        let mut controller_properties = solar_ports.clone();
        controller_properties["base"] = serde_json::json!({ "type": "boolean" });
        controller_properties["formed"] = serde_json::json!({ "type": "boolean" });
        set_custom_states(
            SOLAR_ARRAY_CONTROLLER,
            controller_properties,
            ["down", "up", "north", "south", "west", "east"]
                .map(|face| (format!("port_{face}"), "false".to_owned()))
                .into_iter()
                .chain([
                    ("base".to_owned(), "false".to_owned()),
                    ("formed".to_owned(), "false".to_owned()),
                ])
                .collect(),
        );
        let mut casing_properties = solar_ports.clone();
        casing_properties["formed"] = serde_json::json!({ "type": "boolean" });
        casing_properties["part"] = serde_json::json!({
            "type": "enum",
            "values": ["none", "other", "base", "upper_ew", "upper_ns"]
        });
        set_custom_states(
            SOLAR_ARRAY_CASING,
            casing_properties,
            ["down", "up", "north", "south", "west", "east"]
                .map(|face| (format!("port_{face}"), "false".to_owned()))
                .into_iter()
                .chain([
                    ("formed".to_owned(), "false".to_owned()),
                    ("part".to_owned(), "none".to_owned()),
                ])
                .collect(),
        );
        let mut collector_properties = solar_ports;
        collector_properties["formed"] = serde_json::json!({ "type": "boolean" });
        set_custom_states(
            SOLAR_COLLECTOR,
            collector_properties,
            ["down", "up", "north", "south", "west", "east"]
                .map(|face| (format!("port_{face}"), "false".to_owned()))
                .into_iter()
                .chain([("formed".to_owned(), "false".to_owned())])
                .collect(),
        );
        for id in [
            WROUGHT_ENERGY_CELL,
            TEMPERED_ENERGY_CELL,
            HARDENED_ENERGY_CELL,
            ARCFORGED_ENERGY_CELL,
        ] {
            register_content!(&mut event.blocks, id: id.into(), template: "minecraft:iron_block".into(), item: true);
            if let Some(block) = event.blocks.iter_mut().find(|block| block.id == id) {
                block.components = [("minecraft:max_stack_size".to_owned(), serde_json::json!(1))]
                    .into_iter()
                    .collect();
                block.state_properties = serde_json::from_value(serde_json::json!({
                    "charge": { "type": "enum", "values": ["0", "1", "2", "3", "4"] }
                }))
                .expect("valid Energy Cell charge block state");
                block.default_state = [("charge".to_owned(), "0".to_owned())]
                    .into_iter()
                    .collect();
            }
        }
        for id in heat_cell::ids() {
            register_content!(&mut event.blocks, id: id.into(), template: "minecraft:iron_block".into(), item: true);
        }
        register_content!(&mut event.blocks, id: COKE_BLOCK.into(), template: "minecraft:coal_block".into(), item: true);
        for (id, template, hardness, resistance) in [
            ("arcforge:treated_log", "minecraft:crimson_stem", 2.0, 2.0),
            (
                "arcforge:treated_wood",
                "minecraft:crimson_hyphae",
                2.0,
                2.0,
            ),
            (
                "arcforge:stripped_treated_log",
                "minecraft:stripped_crimson_stem",
                2.0,
                2.0,
            ),
            (
                "arcforge:stripped_treated_wood",
                "minecraft:stripped_crimson_hyphae",
                2.0,
                2.0,
            ),
            (
                "arcforge:treated_planks",
                "minecraft:crimson_planks",
                2.0,
                3.0,
            ),
            (
                "arcforge:treated_stairs",
                "minecraft:crimson_stairs",
                2.0,
                3.0,
            ),
            ("arcforge:treated_slab", "minecraft:crimson_slab", 2.0, 3.0),
            (
                "arcforge:treated_fence",
                "minecraft:crimson_fence",
                2.0,
                3.0,
            ),
            (
                "arcforge:treated_fence_gate",
                "minecraft:crimson_fence_gate",
                2.0,
                3.0,
            ),
            ("arcforge:treated_door", "minecraft:crimson_door", 3.0, 3.0),
            (
                "arcforge:treated_trapdoor",
                "minecraft:crimson_trapdoor",
                3.0,
                3.0,
            ),
            (
                "arcforge:treated_pressure_plate",
                "minecraft:crimson_pressure_plate",
                0.5,
                0.5,
            ),
            (
                "arcforge:treated_button",
                "minecraft:crimson_button",
                0.5,
                0.5,
            ),
        ] {
            register_content!(&mut event.blocks, id: id.into(), template: template.into(), item: true);
            if let Some(block) = event.blocks.iter_mut().find(|block| block.id == id) {
                block.properties.hardness = Some(hardness);
                block.properties.explosion_resistance = Some(resistance);
                if id != "arcforge:treated_button" {
                    block.properties.map_color = Some(26);
                    block.properties.sounds = Some("minecraft:oak_planks".into());
                } else {
                    block.properties.map_color = Some(0);
                    block.properties.sounds = Some("minecraft:stone".into());
                }
            }
        }
        register_content!(&mut event.blocks, id: "arcforge:heavy_mineral_sand".into(), template: "minecraft:sand".into(), item: true);
        register_content!(&mut event.blocks, id: STEEL_BLOCK.into(), template: "minecraft:iron_block".into(), item: true);
        for (id, template) in crushing_catalog::BLOCKS {
            register_content!(&mut event.blocks, id: (*id).into(), template: (*template).into(), item: true);
        }
        for (id, light) in [
            ("arcforge:arcite_ore", 5),
            ("arcforge:deepslate_arcite_ore", 5),
            ("arcforge:raw_arcite_block", 9),
            ("arcforge:arcite_block", 12),
        ] {
            if let Some(block) = event.blocks.iter_mut().find(|block| block.id == id) {
                block.properties.light = Some(light);
            }
        }
    }

    fn items(&self, event: &mut ItemsEvent<'_>) {
        register_content!(&mut event.items, id: WRENCH.into(), template: "minecraft:iron_pickaxe".into(), max_stack_size: Some(1));
        register_content!(&mut event.items, id: PINE_RESIN.into(), template: "minecraft:slime_ball".into());
        register_content!(&mut event.items, id: WOOD_ASH.into(), template: "minecraft:bone_meal".into());
        register_content!(&mut event.items, id: "arcforge:thermocouple".into(), template: "minecraft:comparator".into());
        register_content!(&mut event.items, id: ENERGY_UPGRADE.into(), template: "minecraft:redstone".into());
        register_content!(&mut event.items, id: SPEED_UPGRADE.into(), template: "minecraft:sugar".into());
        register_content!(&mut event.items, id: HEAT_UPGRADE.into(), template: "minecraft:magma_cream".into());
        register_content!(&mut event.items, id: THERMOELECTRIC_UPGRADE.into(), template: "minecraft:comparator".into());
        register_content!(&mut event.items, id: "arcforge:receiver_tube".into(), template: "minecraft:glass_pane".into());
        register_content!(&mut event.items, id: "arcforge:trough_mirror".into(), template: "minecraft:iron_ingot".into());
        register_content!(&mut event.items, id: CREOSOTE_BUCKET.into(), template: "minecraft:bucket".into(), max_stack_size: Some(1));
        register_content!(&mut event.items, id: LATEX_BUCKET.into(), template: "minecraft:bucket".into(), max_stack_size: Some(1));
        for id in [
            WROUGHT_CANISTER,
            TEMPERED_CANISTER,
            HARDENED_CANISTER,
            ARCFORGED_CANISTER,
        ] {
            register_content!(&mut event.items, id: id.into(), template: "minecraft:bucket".into(), max_stack_size: Some(1));
        }
        for id in pressurized_cylinder::cartridge_ids() {
            register_content!(&mut event.items, id: id.into(), template: "minecraft:bucket".into(), max_stack_size: Some(1));
        }
        for id in [
            WROUGHT_BATTERY,
            TEMPERED_BATTERY,
            HARDENED_BATTERY,
            ARCFORGED_BATTERY,
        ] {
            register_content!(&mut event.items, id: id.into(), template: "minecraft:redstone".into(), max_stack_size: Some(1));
        }
        register_content!(&mut event.items, id: TUNGSTEN_HEATING_COIL.into(), template: "minecraft:iron_ingot".into());
        for (id, template) in [
            ("arcforge:hop_cones", "minecraft:wheat"),
            ("arcforge:dried_hops", "minecraft:wheat"),
            ("arcforge:dried_grain", "minecraft:wheat"),
            ("arcforge:sorghum_stalks", "minecraft:wheat"),
            ("arcforge:dried_sorghum", "minecraft:wheat"),
        ] {
            register_content!(&mut event.items, id: id.into(), template: template.into());
        }
        for (id, template) in [
            (PITCH, "minecraft:coal"),
            ("arcforge:carbon_fiber", "minecraft:string"),
            ("arcforge:rock_wool", "minecraft:white_wool"),
            ("arcforge:slag_wool", "minecraft:white_wool"),
        ] {
            register_content!(&mut event.items, id: id.into(), template: template.into());
        }
        register_content!(&mut event.items, id: STEEL.into(), template: IRON.into());
        for (id, template, durability, damage, speed, mining_speed) in [
            (
                "arcforge:steel_sword",
                "minecraft:iron_sword",
                500,
                5.5,
                -2.4,
                None,
            ),
            (
                "arcforge:steel_pickaxe",
                "minecraft:iron_pickaxe",
                500,
                3.5,
                -2.8,
                Some(("minecraft:mineable/pickaxe", 7.0)),
            ),
            (
                "arcforge:steel_axe",
                "minecraft:iron_axe",
                500,
                8.5,
                -3.1,
                Some(("minecraft:mineable/axe", 7.0)),
            ),
            (
                "arcforge:steel_shovel",
                "minecraft:iron_shovel",
                500,
                4.0,
                -3.0,
                Some(("minecraft:mineable/shovel", 7.0)),
            ),
            (
                "arcforge:steel_hoe",
                "minecraft:iron_hoe",
                500,
                0.0,
                -0.5,
                Some(("minecraft:mineable/hoe", 7.0)),
            ),
            (
                "arcforge:steel_hammer",
                "minecraft:iron_pickaxe",
                1_500,
                4.5,
                -3.2,
                Some(("minecraft:mineable/pickaxe", 4.9)),
            ),
            (
                "arcforge:steel_excavator",
                "minecraft:iron_shovel",
                1_500,
                4.0,
                -3.1,
                Some(("minecraft:mineable/shovel", 4.9)),
            ),
        ] {
            let mut components = steel_equipment_components(durability);
            components.insert(
                "minecraft:attribute_modifiers".into(),
                steel_attribute_modifiers(
                    &[
                        ("minecraft:attack_damage", damage),
                        ("minecraft:attack_speed", speed),
                    ],
                    "mainhand",
                ),
            );
            if let Some((mineable_tag, mining_speed)) = mining_speed {
                components.insert(
                    "minecraft:tool".into(),
                    serde_json::json!({
                        "can_destroy_blocks_in_creative": false,
                        "damage_per_block": 1,
                        "default_mining_speed": mining_speed,
                        "rules": [
                            { "blocks": "#minecraft:incorrect_for_iron_tool", "speed": 0.0, "correct_for_drops": false },
                            { "blocks": format!("#{mineable_tag}"), "speed": mining_speed, "correct_for_drops": true }
                        ]
                    }),
                );
            }
            register_content!(&mut event.items, id: id.into(), template: template.into(), max_stack_size: Some(1), components: components);
        }
        for (id, template, durability, armor, slot) in [
            (
                "arcforge:steel_helmet",
                "minecraft:iron_helmet",
                275,
                2.0,
                "head",
            ),
            (
                "arcforge:steel_chestplate",
                "minecraft:iron_chestplate",
                400,
                7.0,
                "chest",
            ),
            (
                "arcforge:steel_leggings",
                "minecraft:iron_leggings",
                375,
                5.0,
                "legs",
            ),
            (
                "arcforge:steel_boots",
                "minecraft:iron_boots",
                325,
                2.0,
                "feet",
            ),
        ] {
            let mut components = steel_equipment_components(durability);
            components.insert(
                "minecraft:attribute_modifiers".into(),
                steel_attribute_modifiers(
                    &[
                        ("minecraft:armor", armor),
                        ("minecraft:armor_toughness", 1.0),
                    ],
                    slot,
                ),
            );
            components.insert(
                "minecraft:equippable".into(),
                serde_json::json!({
                    "slot": slot,
                    "equip_sound": "minecraft:item.armor.equip_iron",
                    "asset_id": "arcforge:steel"
                }),
            );
            register_content!(&mut event.items, id: id.into(), template: template.into(), max_stack_size: Some(1), components: components);
        }
        register_content!(&mut event.items, id: COKE.into(), template: "minecraft:coal".into());
        register_content!(&mut event.items, id: SLAG.into(), template: "minecraft:gravel".into());
        register_content!(&mut event.items, id: CARBON_DUST.into(), template: "minecraft:gunpowder".into());
        register_content!(&mut event.items, id: IRON_DUST.into(), template: "minecraft:iron_nugget".into());
        for id in [PLATE_DIE, GEAR_DIE, ROD_DIE] {
            register_content!(&mut event.items, id: id.into(), template: "minecraft:iron_ingot".into(), max_stack_size: Some(1));
        }
        register_content!(&mut event.items, id: PELLET_DIE.into(), template: "minecraft:iron_ingot".into(), max_stack_size: Some(1));
        for (id, durability) in [
            (STEEL_MESH, 256),
            (INVAR_MESH, 512),
            (TUNGSTEN_MESH, 1024),
            (SILICON_CARBIDE_MESH, 2048),
        ] {
            register_content!(&mut event.items, id: id.into(), template: "minecraft:iron_hoe".into(), max_stack_size: Some(1), components: [(
                    "minecraft:max_damage".to_owned(),
                    serde_json::json!(durability),
                )]
                .into_iter()
                .collect());
        }
        for id in [STEEL_PLATE, STEEL_GEAR, STEEL_ROD] {
            register_content!(&mut event.items, id: id.into(), template: "minecraft:iron_ingot".into());
        }
        for (id, template) in [
            (COPPER_DUST, "minecraft:copper_ingot"),
            (GOLD_DUST, "minecraft:gold_nugget"),
            (SILICA_DUST, "minecraft:flint"),
        ] {
            register_content!(&mut event.items, id: id.into(), template: template.into());
        }
        for (id, template) in crushing_catalog::ITEMS {
            register_content!(&mut event.items, id: (*id).into(), template: (*template).into());
        }
    }

    fn item_fluid_tank(&self, _host: &dyn Host, event: &mut ItemFluidTankEvent<'_>) -> Option<i32> {
        if matches!(event.item, CREOSOTE_BUCKET | LATEX_BUCKET) {
            Some(1000)
        } else {
            canister_spec(event.item)
                .map(|(capacity, _)| capacity as i32)
                .or_else(|| {
                    pressurized_cylinder::cartridge_spec(event.item)
                        .map(|(capacity, _)| capacity as i32)
                })
        }
    }

    fn item_energy_buffer(
        &self,
        _host: &dyn Host,
        event: &mut ItemEnergyBufferEvent<'_>,
    ) -> Option<ItemEnergyBuffer> {
        portable_energy_item_buffer(event.item)
    }

    fn loot_drops(&self, _host: &mut dyn Host, event: &mut LootDropsEvent<'_>) {
        let Some((pos, _)) = event.context.block else {
            return;
        };
        if let Some((item, energy)) = take_cell_charge(event.context.dimension, pos) {
            for drop in event
                .drops
                .iter_mut()
                .filter(|drop| drop.item.as_ref() == item.as_str())
            {
                set_item_energy(drop, energy);
            }
        }
        if let Some((item, heat)) = heat_cell::take_charge(event.context.dimension, pos) {
            for drop in event
                .drops
                .iter_mut()
                .filter(|drop| drop.item.as_ref() == item.as_str())
            {
                heat_cell::set_item_heat(drop, heat);
            }
        }
        if let Some((item, fluid, amount)) =
            pressurized_cylinder::take_charge(event.context.dimension, pos)
        {
            if let Some(fluid) = fluid.as_deref() {
                for drop in event
                    .drops
                    .iter_mut()
                    .filter(|drop| drop.item.as_ref() == item.as_str())
                {
                    *drop = item_with_fluid(drop, fluid, amount);
                }
            }
        }
    }

    fn block_entity(
        &self,
        _host: &mut dyn horizon_client::mod_api::Host,
        event: &mut BlockEntityEvent<'_>,
    ) -> Option<(String, horizon_client::mod_api::Compound)> {
        if event.block == MACHINE
            || event.block == COMBUSTION
            || event.block == CARBONIZER
            || event.block == ARC_CRUSHER
            || event.block == ARC_CRUSHING_ARRAY
            || event.block == ARC_CRUSHING_ARRAY_CASING
            || event.block == INDUCTION_FURNACE
            || event.block == METAL_PRESS
            || event.block == SIFTER
            || event.block == INFUSER
            || event.block == FIREBOX
            || event.block == FIREBOX_ARRAY_CONTROLLER
            || event.block == SOLAR_ARRAY_CONTROLLER
            || event.block == GEOTHERMAL_PLANT
            || event.block == FUEL_BURNER
            || event.block == WROUGHT_FLUID_TANK
            || event.block == TEMPERED_FLUID_TANK
            || event.block == HARDENED_FLUID_TANK
            || event.block == ARCFORGED_FLUID_TANK
            || event.block == RESIN_TAP
            || event.block == FIBERIZER
            || event.block == VULCANIZER
            || event.block == GRAIN_DRYER
            || event.block == FURNACE_PORT
            || event.block == THERMOELECTRIC_PLANT
            || event.block == WROUGHT_ENERGY_CONDUIT
            || event.block == TEMPERED_ENERGY_CONDUIT
            || event.block == HARDENED_ENERGY_CONDUIT
            || event.block == ARCFORGED_ENERGY_CONDUIT
            || item_conduit::spec(event.block).is_some()
            || thermal::spec(event.block).is_some()
            || event.block == WROUGHT_ENERGY_CELL
            || event.block == TEMPERED_ENERGY_CELL
            || event.block == HARDENED_ENERGY_CELL
            || event.block == ARCFORGED_ENERGY_CELL
            || heat_cell::spec(event.block).is_some()
            || pressurized_cylinder::cylinder_capacity(event.block).is_some()
        {
            Some((event.block.to_owned(), Default::default()))
        } else {
            None
        }
    }

    fn block_entity_ticks(
        &self,
        _host: &dyn horizon_client::mod_api::Host,
        event: &mut BlockEntityTicksEvent<'_>,
    ) {
        if event.block == MACHINE
            || event.block == COMBUSTION
            || event.block == CARBONIZER
            || event.block == ARC_CRUSHER
            || event.block == ARC_CRUSHING_ARRAY
            || event.block == ARC_CRUSHING_ARRAY_CASING
            || event.block == INDUCTION_FURNACE
            || event.block == METAL_PRESS
            || event.block == SIFTER
            || event.block == INFUSER
            || event.block == FIREBOX
            || event.block == FIREBOX_ARRAY_CONTROLLER
            || event.block == SOLAR_ARRAY_CONTROLLER
            || event.block == GEOTHERMAL_PLANT
            || event.block == FUEL_BURNER
            || event.block == RESIN_TAP
            || event.block == FIBERIZER
            || event.block == VULCANIZER
            || event.block == GRAIN_DRYER
            || event.block == FURNACE_PORT
            || event.block == THERMOELECTRIC_PLANT
            || event.block == WROUGHT_ENERGY_CONDUIT
            || event.block == TEMPERED_ENERGY_CONDUIT
            || event.block == HARDENED_ENERGY_CONDUIT
            || event.block == ARCFORGED_ENERGY_CONDUIT
            || item_conduit::spec(event.block).is_some()
            || thermal::spec(event.block).is_some()
            || heat_cell::spec(event.block).is_some()
            || pressurized_cylinder::cylinder_capacity(event.block).is_some()
        {
            event.ticks = true;
        }
    }

    fn block_entity_container(
        &self,
        host: &dyn horizon_client::mod_api::Host,
        event: &mut BlockEntityContainerEvent<'_>,
    ) -> Option<ModContainer> {
        if pressurized_cylinder::cylinder_capacity(event.block).is_some() {
            Some(ModContainer {
                slots: 2,
                faces: [
                    vec![1],
                    vec![0],
                    vec![0, 1],
                    vec![0, 1],
                    vec![0, 1],
                    vec![0, 1],
                ],
                inputs: vec![0, 1],
                outputs: vec![0, 1],
                menu: Some("minecraft:hopper".into()),
            })
        } else if event.block == VULCANIZER {
            Some(ModContainer {
                slots: 3,
                faces: [
                    vec![2],
                    vec![0, 1],
                    vec![0, 1],
                    vec![0, 1],
                    vec![0, 1],
                    vec![0, 1],
                ],
                inputs: vec![0, 1],
                outputs: vec![2],
                menu: Some("minecraft:furnace".into()),
            })
        } else if event.block == GRAIN_DRYER {
            Some(ModContainer {
                slots: 2,
                faces: [vec![1], vec![0], vec![], vec![], vec![], vec![]],
                inputs: vec![0],
                outputs: vec![1],
                menu: Some("minecraft:furnace".into()),
            })
        } else if event.block == FURNACE_PORT {
            cache_arcforge_furnace_menu_rules(host);
            Some(ModContainer {
                slots: 6,
                faces: [
                    vec![4, 5],
                    vec![0, 1, 2, 3],
                    vec![0, 1, 2, 3],
                    vec![0, 1, 2, 3],
                    vec![0, 1, 2, 3],
                    vec![0, 1, 2, 3],
                ],
                inputs: vec![0, 1, 2, 3],
                outputs: vec![4, 5],
                menu: Some("arcforge:arcforge_furnace".into()),
            })
        } else if event.block == MACHINE {
            Some(ModContainer {
                slots: 7,
                faces: [
                    vec![5, 6],
                    vec![0, 1, 2, 3, 4],
                    vec![0, 1, 2, 3, 4],
                    vec![0, 1, 2, 3, 4],
                    vec![0, 1, 2, 3, 4],
                    vec![0, 1, 2, 3, 4],
                ],
                inputs: vec![0, 1, 2, 3, 4],
                outputs: vec![5, 6],
                menu: Some("minecraft:generic_3x3".into()),
            })
        } else if event.block == COMBUSTION {
            Some(ModContainer {
                slots: 3,
                faces: [vec![1], vec![0], vec![0], vec![0], vec![0], vec![0]],
                inputs: vec![0],
                outputs: vec![1],
                menu: Some("minecraft:furnace".into()),
            })
        } else if event.block == CARBONIZER {
            Some(ModContainer {
                slots: 3,
                faces: [vec![2], vec![0], vec![0], vec![0], vec![0], vec![0]],
                inputs: vec![0],
                outputs: vec![2],
                menu: Some("minecraft:furnace".into()),
            })
        } else if event.block == ARC_CRUSHER {
            Some(ModContainer {
                slots: 3,
                faces: [vec![1, 2], vec![0], vec![0], vec![0], vec![0], vec![0]],
                inputs: vec![0],
                outputs: vec![1, 2],
                menu: Some("minecraft:furnace".into()),
            })
        } else if event.block == ARC_CRUSHING_ARRAY || event.block == ARC_CRUSHING_ARRAY_CASING {
            Some(ModContainer {
                slots: 9,
                faces: [
                    vec![3, 4, 5, 6, 7, 8],
                    vec![0, 1, 2],
                    vec![],
                    vec![],
                    vec![],
                    vec![],
                ],
                inputs: vec![0, 1, 2],
                outputs: vec![3, 4, 5, 6, 7, 8],
                menu: Some("minecraft:generic_3x3".into()),
            })
        } else if event.block == INDUCTION_FURNACE {
            Some(ModContainer {
                slots: 3,
                faces: [vec![2], vec![0], vec![0], vec![0], vec![0], vec![0]],
                inputs: vec![0],
                outputs: vec![2],
                menu: Some("minecraft:furnace".into()),
            })
        } else if event.block == METAL_PRESS {
            Some(ModContainer {
                slots: 3,
                faces: [vec![2], vec![0], vec![0], vec![0], vec![0], vec![0]],
                inputs: vec![0],
                outputs: vec![2],
                menu: Some("minecraft:furnace".into()),
            })
        } else if event.block == SIFTER {
            Some(ModContainer {
                slots: 8,
                faces: [
                    vec![2, 3, 4, 5, 6, 7],
                    vec![0, 1],
                    vec![],
                    vec![],
                    vec![],
                    vec![],
                ],
                inputs: vec![0, 1],
                outputs: vec![2, 3, 4, 5, 6, 7],
                menu: Some("minecraft:generic_3x3".into()),
            })
        } else if event.block == INFUSER {
            Some(ModContainer {
                slots: 5,
                faces: [
                    vec![1, 3],
                    vec![0, 2, 4],
                    vec![0, 2, 4],
                    vec![0, 2, 4],
                    vec![0, 2, 4],
                    vec![0, 2, 4],
                ],
                inputs: vec![0, 2, 4],
                outputs: vec![1, 3],
                menu: Some("minecraft:hopper".into()),
            })
        } else if event.block == FIREBOX {
            Some(ModContainer {
                slots: 2,
                faces: [vec![1], vec![0], vec![0], vec![0], vec![0], vec![0]],
                inputs: vec![0],
                outputs: vec![1],
                menu: Some("minecraft:generic_3x3".into()),
            })
        } else if event.block == FIREBOX_ARRAY_CONTROLLER {
            Some(ModContainer {
                slots: 1,
                faces: [vec![0], vec![0], vec![0], vec![0], vec![0], vec![0]],
                inputs: vec![0],
                outputs: vec![],
                menu: Some("minecraft:furnace".into()),
            })
        } else if event.block == GEOTHERMAL_PLANT {
            Some(ModContainer {
                slots: 2,
                faces: [vec![1], vec![0], vec![0], vec![0], vec![0], vec![0]],
                inputs: vec![0],
                outputs: vec![1],
                menu: Some("minecraft:furnace".into()),
            })
        } else if event.block == FUEL_BURNER {
            Some(ModContainer {
                slots: 2,
                faces: [vec![0], vec![0], vec![0], vec![0], vec![0], vec![1]],
                inputs: vec![0],
                outputs: vec![1],
                menu: Some("minecraft:furnace".into()),
            })
        } else if event.block == FIBERIZER {
            Some(ModContainer {
                slots: 2,
                faces: [vec![1], vec![0], vec![0], vec![0], vec![0], vec![0]],
                inputs: vec![0],
                outputs: vec![1],
                menu: Some("minecraft:generic_3x3".into()),
            })
        } else if event.block == RESIN_TAP {
            Some(ModContainer {
                slots: 5,
                faces: [vec![0], vec![], vec![], vec![], vec![], vec![]],
                inputs: vec![],
                outputs: vec![0],
                menu: Some("minecraft:hopper".into()),
            })
        } else {
            None
        }
    }

    fn screen(
        &self,
        _host: &dyn Host,
        event: &mut horizon_client::mod_api::ScreenEvent<'_>,
    ) -> Option<NativeScreen> {
        match event.block {
            INFUSER => Some(NativeScreen {
                width: 176.0,
                height: 228.0,
                slots: vec![
                    Some([10.0, 30.0]),
                    Some([146.0, 30.0]),
                    Some([46.0, 48.0]),
                    Some([118.0, 48.0]),
                    Some([46.0, 72.0]),
                ],
                inventory_y: 142.0,
                inventory_x: 8.0,
                own_labels: true,
                own_background: false,
            }),
            FURNACE_PORT => Some(NativeScreen {
                width: 176.0,
                height: 202.0,
                slots: vec![
                    Some([20.0, 30.0]),
                    Some([44.0, 30.0]),
                    Some([68.0, 30.0]),
                    Some([44.0, 54.0]),
                    Some([128.0, 30.0]),
                    Some([128.0, 54.0]),
                ],
                inventory_y: 118.0,
                inventory_x: 8.0,
                own_labels: true,
                own_background: false,
            }),
            ARC_CRUSHER => Some(NativeScreen {
                width: 176.0,
                height: 196.0,
                slots: vec![Some([28.0, 31.0]), Some([100.0, 31.0]), Some([124.0, 31.0])],
                inventory_y: 112.0,
                inventory_x: 8.0,
                own_labels: true,
                own_background: false,
            }),
            _ => None,
        }
    }

    fn screen_draw(
        &self,
        host: &dyn Host,
        event: &mut horizon_client::mod_api::ScreenDrawEvent<'_>,
    ) {
        if event.block == INFUSER {
            let [ox, oy] = event.origin;
            let data = event.data;
            let batch = &mut *event.batch;
            macro_rules! infuser_text {
                ($label:expr, $x:expr, $y:expr, $color:expr $(,)?) => {
                    host.text(batch, $label, [ox + $x, oy + $y], $color, true)
                };
            }
            macro_rules! infuser_fill {
                ($rect:expr, $color:expr) => {
                    host.fill(
                        batch,
                        [ox + ($rect)[0], oy + ($rect)[1], ($rect)[2], ($rect)[3]],
                        $color,
                    )
                };
            }

            infuser_text!("Infuser", 8.0, 6.0, 0xFFFFFFFF);
            infuser_text!("Bucket In", 8.0, 16.0, 0xFFBBBBBB);
            infuser_text!("Bucket Out", 112.0, 16.0, 0xFFBBBBBB);
            infuser_text!("Input", 46.0, 34.0, 0xFFBBBBBB);
            infuser_text!("Output", 118.0, 34.0, 0xFFBBBBBB);
            infuser_text!("Additive", 46.0, 58.0, 0xFFBBBBBB);

            let fluid = data.first().copied().unwrap_or_default().clamp(0, 8_000);
            let energy = data.get(1).copied().unwrap_or_default().max(0);
            let progress = data.get(2).copied().unwrap_or_default().max(0);
            let duration = data.get(3).copied().unwrap_or_default().max(0);
            infuser_text!(&format!("FE {energy}/20000"), 8.0, 88.0, 0xFFE0E0E0,);
            infuser_text!(&format!("Fluid {fluid}/8000 mB"), 8.0, 99.0, 0xFFE0E0E0,);
            if duration > 0 {
                infuser_text!(
                    &format!("Progress {progress}/{duration} t"),
                    8.0,
                    110.0,
                    0xFFE0E0E0,
                );
                infuser_fill!([8.0, 122.0, 160.0, 6.0], 0xFF383838);
                let fraction = (progress as f32 / duration as f32).clamp(0.0, 1.0);
                infuser_fill!([8.0, 122.0, 160.0 * fraction, 6.0], 0xFF62B64F);
            } else {
                infuser_text!("Waiting for recipe inputs", 8.0, 110.0, 0xFFAAAAAA);
            }
            infuser_text!("Inventory", 8.0, 132.0, 0xFFFFFFFF);
            return;
        }
        if event.block == ARC_CRUSHER {
            let [ox, oy] = event.origin;
            let data = event.data;
            let batch = &mut *event.batch;
            macro_rules! crusher_text {
                ($label:expr, $x:expr, $y:expr, $color:expr $(,)?) => {
                    host.text(batch, $label, [ox + $x, oy + $y], $color, true)
                };
            }
            macro_rules! crusher_fill {
                ($rect:expr, $color:expr) => {
                    host.fill(
                        batch,
                        [ox + ($rect)[0], oy + ($rect)[1], ($rect)[2], ($rect)[3]],
                        $color,
                    )
                };
            }

            crusher_text!("Arc Crusher", 8.0, 6.0, 0xFFFFFFFF);
            crusher_text!("Input", 27.0, 19.0, 0xFFBBBBBB);
            crusher_text!("Out", 103.0, 19.0, 0xFFBBBBBB);
            crusher_text!("Bonus", 130.0, 19.0, 0xFFBBBBBB);
            let energy = data.first().copied().unwrap_or_default().max(0);
            let capacity = data.get(1).copied().unwrap_or(20_000).max(1);
            let progress = data.get(2).copied().unwrap_or_default().max(0);
            let duration = data.get(3).copied().unwrap_or_default().max(0);
            crusher_text!(
                &format!("Energy {energy}/{capacity} FE"),
                8.0,
                60.0,
                0xFFE0E0E0,
            );
            if duration > 0 {
                crusher_text!(
                    &format!("Progress {progress}/{duration} t"),
                    8.0,
                    72.0,
                    0xFFE0E0E0,
                );
            } else {
                crusher_text!("No matching recipe", 8.0, 72.0, 0xFFAAAAAA);
            }
            crusher_fill!([8.0, 88.0, 160.0, 8.0], 0xFF383838);
            let progress_fraction = if duration > 0 {
                (progress as f32 / duration as f32).clamp(0.0, 1.0)
            } else {
                0.0
            };
            crusher_fill!([8.0, 88.0, 160.0 * progress_fraction, 8.0], 0xFF62B64F);
            crusher_text!("Inventory", 8.0, 100.0, 0xFFFFFFFF);
            return;
        }
        if event.block != FURNACE_PORT {
            return;
        }
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

        text!("Arcforge Furnace", 8.0, 6.0, 0xFFFFFFFF);
        let formed = data.get(5).copied().unwrap_or_default() != 0;
        text!(
            if formed { "Formed" } else { "Incomplete" },
            8.0,
            18.0,
            if formed { 0xFF80D080 } else { 0xFFFF7777 },
        );

        let temperature = data.get(4).copied().unwrap_or_default().max(20);
        let burn = data.first().copied().unwrap_or_default().max(0);
        let burn_duration = data.get(1).copied().unwrap_or_default().max(1);
        let progress = data.get(2).copied().unwrap_or_default().max(0);
        let required_progress = data.get(3).copied().unwrap_or_default().max(0);
        let required_heat = data.get(6).copied().unwrap_or_default().max(0);
        let required_coke = data.get(7).copied().unwrap_or_default().max(0);
        text!(&format!("Heat {temperature} C"), 8.0, 78.0, 0xFFE0E0E0);
        text!(&format!("Fuel {burn} t"), 92.0, 78.0, 0xFFE0E0E0);
        if required_progress > 0 {
            text!(
                &format!("Progress {progress}/{required_progress}"),
                8.0,
                88.0,
                0xFFE0E0E0,
            );
            text!(
                &format!("Min {required_heat} C   Coke {required_coke}"),
                8.0,
                98.0,
                0xFFE0E0E0,
            );
        } else {
            text!("No matching recipe", 8.0, 88.0, 0xFFAAAAAA);
        }

        fill!([92.0, 37.0, 24.0, 8.0], 0xFF383838);
        let progress_fraction = if required_progress > 0 {
            (progress as f32 / required_progress as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };
        fill!([92.0, 37.0, 24.0 * progress_fraction, 8.0], 0xFF62B64F);
        fill!([92.0, 53.0, 8.0, 24.0], 0xFF383838);
        let burn_fraction = (burn as f32 / burn_duration as f32).clamp(0.0, 1.0);
        fill!(
            [94.0, 75.0 - 20.0 * burn_fraction, 4.0, 20.0 * burn_fraction],
            0xFFFF9B35
        );
        text!("Inventory", 8.0, 108.0, 0xFFFFFFFF);
    }

    fn block_entity_accepts_fluid(
        &self,
        _host: &dyn Host,
        event: &mut horizon_client::mod_api::BlockEntityAcceptsFluidEvent<'_>,
    ) -> Flow {
        if pressurized_cylinder::cylinder_capacity(event.block).is_some()
            && !pressurized_cylinder::accepts_gas(event.block, event.fluid)
        {
            Flow::Cancel
        } else if event.block == GEOTHERMAL_PLANT && event.fluid != "minecraft:lava" {
            Flow::Cancel
        } else if matches!(
            event.block,
            CARBONIZER
                | FUEL_BURNER
                | WROUGHT_FLUID_TANK
                | TEMPERED_FLUID_TANK
                | HARDENED_FLUID_TANK
                | ARCFORGED_FLUID_TANK
                | GRAIN_DRYER
                | INFUSER
                | RESIN_TAP
        ) && fluid_conduit::is_arcforge_gas(&event.fluid)
        {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn block_entity_energy(
        &self,
        _host: &dyn horizon_client::mod_api::Host,
        event: &mut horizon_client::mod_api::BlockEntityEnergyEvent<'_>,
    ) -> Option<horizon_client::mod_api::ModEnergyBuffer> {
        if let Some((capacity, rate)) = energy_conduit_spec(event.block) {
            return Some(horizon_client::mod_api::ModEnergyBuffer {
                capacity: capacity.min(i32::MAX as u64) as i32,
                max_insert: rate.min(i32::MAX as u64) as i32,
                max_extract: rate.min(i32::MAX as u64) as i32,
                insert_faces: [true; 6],
                extract_faces: [true; 6],
            });
        }
        if let Some((capacity, rate)) = energy_cell_spec(event.block) {
            return Some(horizon_client::mod_api::ModEnergyBuffer {
                capacity: capacity as i32,
                max_insert: rate as i32,
                max_extract: rate as i32,
                insert_faces: [true; 6],
                extract_faces: [true; 6],
            });
        }
        if event.block == COMBUSTION {
            Some(horizon_client::mod_api::ModEnergyBuffer {
                capacity: COMBUSTION_CAPACITY_FE as i32,
                max_insert: 0,
                max_extract: COMBUSTION_MAX_EXTRACT_FE,
                insert_faces: [false; 6],
                extract_faces: [true; 6],
            })
        } else if event.block == ARC_CRUSHER {
            Some(horizon_client::mod_api::ModEnergyBuffer {
                capacity: 20_000,
                max_insert: 200,
                max_extract: 0,
                insert_faces: [true; 6],
                extract_faces: [false; 6],
            })
        } else if event.block == ARC_CRUSHING_ARRAY || event.block == ARC_CRUSHING_ARRAY_CASING {
            Some(horizon_client::mod_api::ModEnergyBuffer {
                capacity: 100_000,
                max_insert: 1000,
                max_extract: 0,
                insert_faces: [false, false, false, true, false, false],
                extract_faces: [false; 6],
            })
        } else if event.block == INDUCTION_FURNACE {
            Some(horizon_client::mod_api::ModEnergyBuffer {
                capacity: 20_000,
                max_insert: 200,
                max_extract: 0,
                insert_faces: [true; 6],
                extract_faces: [false; 6],
            })
        } else if event.block == METAL_PRESS {
            Some(horizon_client::mod_api::ModEnergyBuffer {
                capacity: 20_000,
                max_insert: 200,
                max_extract: 0,
                insert_faces: [true; 6],
                extract_faces: [false; 6],
            })
        } else if event.block == SIFTER {
            Some(horizon_client::mod_api::ModEnergyBuffer {
                capacity: 20_000,
                max_insert: 1000,
                max_extract: 0,
                insert_faces: [true; 6],
                extract_faces: [false; 6],
            })
        } else if event.block == INFUSER {
            Some(horizon_client::mod_api::ModEnergyBuffer {
                capacity: 20_000,
                max_insert: 200,
                max_extract: 0,
                insert_faces: [true; 6],
                extract_faces: [false; 6],
            })
        } else if event.block == FIBERIZER {
            Some(horizon_client::mod_api::ModEnergyBuffer {
                capacity: 20_000,
                max_insert: 200,
                max_extract: 0,
                insert_faces: [true; 6],
                extract_faces: [false; 6],
            })
        } else if event.block == THERMOELECTRIC_PLANT {
            Some(horizon_client::mod_api::ModEnergyBuffer {
                capacity: THERMOELECTRIC_FE_CAPACITY as i32,
                max_insert: 0,
                max_extract: THERMOELECTRIC_MAX_OUTPUT_FE as i32,
                insert_faces: [false; 6],
                extract_faces: [true; 6],
            })
        } else {
            None
        }
    }

    fn block_entity_fluids(
        &self,
        _host: &dyn horizon_client::mod_api::Host,
        event: &mut horizon_client::mod_api::BlockEntityFluidsEvent<'_>,
    ) -> Option<horizon_client::mod_api::ModFluidTanks> {
        if event.block == CARBONIZER {
            Some(horizon_client::mod_api::ModFluidTanks {
                tanks: vec![8000],
                faces: [vec![0], vec![0], vec![0], vec![0], vec![0], vec![0]],
                inputs: vec![],
                outputs: vec![0],
            })
        } else if event.block == FUEL_BURNER {
            Some(horizon_client::mod_api::ModFluidTanks {
                tanks: vec![FUEL_BURNER_TANK_CAPACITY as i32],
                faces: [vec![0], vec![0], vec![0], vec![0], vec![0], vec![0]],
                inputs: vec![0],
                outputs: vec![],
            })
        } else if event.block == WROUGHT_FLUID_TANK {
            Some(horizon_client::mod_api::ModFluidTanks {
                tanks: vec![WROUGHT_FLUID_TANK_CAPACITY as i32],
                faces: [vec![0], vec![0], vec![0], vec![0], vec![0], vec![0]],
                inputs: vec![0],
                outputs: vec![0],
            })
        } else if event.block == TEMPERED_FLUID_TANK {
            Some(horizon_client::mod_api::ModFluidTanks {
                tanks: vec![TEMPERED_FLUID_TANK_CAPACITY as i32],
                faces: [vec![0], vec![0], vec![0], vec![0], vec![0], vec![0]],
                inputs: vec![0],
                outputs: vec![0],
            })
        } else if event.block == HARDENED_FLUID_TANK {
            Some(horizon_client::mod_api::ModFluidTanks {
                tanks: vec![HARDENED_FLUID_TANK_CAPACITY as i32],
                faces: [vec![0], vec![0], vec![0], vec![0], vec![0], vec![0]],
                inputs: vec![0],
                outputs: vec![0],
            })
        } else if event.block == ARCFORGED_FLUID_TANK {
            Some(horizon_client::mod_api::ModFluidTanks {
                tanks: vec![ARCFORGED_FLUID_TANK_CAPACITY as i32],
                faces: [vec![0], vec![0], vec![0], vec![0], vec![0], vec![0]],
                inputs: vec![0],
                outputs: vec![0],
            })
        } else if let Some(capacity) = pressurized_cylinder::cylinder_capacity(event.block) {
            Some(horizon_client::mod_api::ModFluidTanks {
                tanks: vec![capacity as i32],
                faces: [vec![0], vec![0], vec![0], vec![0], vec![0], vec![0]],
                inputs: vec![0],
                outputs: vec![0],
            })
        } else if event.block == GEOTHERMAL_PLANT {
            Some(horizon_client::mod_api::ModFluidTanks {
                tanks: vec![GEOTHERMAL_LAVA_TANK_CAPACITY as i32],
                faces: [vec![0], vec![0], vec![0], vec![0], vec![0], vec![0]],
                inputs: vec![0],
                outputs: vec![],
            })
        } else if event.block == GRAIN_DRYER {
            Some(horizon_client::mod_api::ModFluidTanks {
                tanks: vec![GRAIN_DRYER_TANK_CAPACITY as i32],
                faces: [vec![], vec![0], vec![], vec![], vec![], vec![]],
                inputs: vec![0],
                outputs: vec![],
            })
        } else if event.block == INFUSER {
            Some(horizon_client::mod_api::ModFluidTanks {
                tanks: vec![8000],
                faces: [vec![0], vec![0], vec![0], vec![0], vec![0], vec![0]],
                inputs: vec![0],
                outputs: vec![],
            })
        } else if event.block == RESIN_TAP {
            Some(horizon_client::mod_api::ModFluidTanks {
                tanks: vec![1000],
                faces: [vec![0], vec![0], vec![0], vec![0], vec![0], vec![0]],
                inputs: vec![],
                outputs: vec![0],
            })
        } else {
            None
        }
    }

    fn block_entity_tick(
        &self,
        host: &mut dyn horizon_client::mod_api::Host,
        event: &mut BlockEntityTickEvent<'_>,
    ) {
        match event.entity.id.as_str() {
            id if pressurized_cylinder::cylinder_capacity(id).is_some() => {
                pressurized_cylinder::tick(host.dimension(), event.pos, event.entity);
                let capacity = pressurized_cylinder::cylinder_capacity(&event.entity.id).unwrap();
                let stored = fluid::amount(&event.entity.nbt, 0).min(capacity);
                let level = ((stored as u64 * 5) / capacity as u64).min(4).to_string();
                if host.block_property(event.pos, "level").as_deref() != Some(&level) {
                    host.set_block_property(event.pos, "level", &level);
                }
            }
            MACHINE => tick_steelworks(host, event.entity),
            COMBUSTION => tick_combustion_plant(host, event.pos, event.entity),
            CARBONIZER => tick_carbonizer(host, event.entity),
            ARC_CRUSHER => tick_arc_crusher(host, event.entity),
            ARC_CRUSHING_ARRAY => tick_crushing_array(host, event.entity),
            ARC_CRUSHING_ARRAY_CASING => tick_crushing_array_casing(host, event.pos, event.entity),
            INDUCTION_FURNACE => tick_induction_furnace(host, event.entity),
            METAL_PRESS => tick_metal_press(host, event.entity),
            SIFTER => tick_sifter(host, event.entity),
            INFUSER => tick_infuser(host, event.entity),
            FIREBOX => tick_firebox(host, event.pos, event.entity),
            FIREBOX_ARRAY_CONTROLLER => tick_firebox_array(host, event.pos, event.entity),
            SOLAR_ARRAY_CONTROLLER => tick_solar_array(host, event.pos, event.entity),
            GEOTHERMAL_PLANT => tick_geothermal_plant(host, event.pos, event.entity),
            FUEL_BURNER => tick_fuel_burner(host, event.pos, event.entity),
            RESIN_TAP => tick_resin_tap(host, event.pos, event.entity),
            FIBERIZER => tick_fiberizer(host, event.pos, event.entity),
            VULCANIZER => tick_vulcanizer(event.entity),
            GRAIN_DRYER => tick_grain_dryer(host, event.pos, event.entity),
            FURNACE_PORT => tick_arcforge_furnace(host, event.pos, event.entity),
            THERMOELECTRIC_PLANT => tick_thermoelectric_plant(host, event.pos, event.entity),
            WROUGHT_ENERGY_CONDUIT
            | TEMPERED_ENERGY_CONDUIT
            | HARDENED_ENERGY_CONDUIT
            | ARCFORGED_ENERGY_CONDUIT => {
                tick_arcforge_energy_conduit(host, event.pos, event.entity)
            }
            item_conduit::WROUGHT
            | item_conduit::TEMPERED
            | item_conduit::HARDENED
            | item_conduit::ARCFORGED => item_conduit::tick(host, event.pos, event.entity),
            fluid_conduit::WROUGHT
            | fluid_conduit::TEMPERED
            | fluid_conduit::HARDENED
            | fluid_conduit::ARCFORGED
            | fluid_conduit::WROUGHT_PRESSURIZED
            | fluid_conduit::TEMPERED_PRESSURIZED
            | fluid_conduit::HARDENED_PRESSURIZED
            | fluid_conduit::ARCFORGED_PRESSURIZED => {
                fluid_conduit::tick(host, event.pos, event.entity)
            }
            WROUGHT_THERMAL_CONDUIT
            | TEMPERED_THERMAL_CONDUIT
            | HARDENED_THERMAL_CONDUIT
            | ARCFORGED_THERMAL_CONDUIT => thermal::tick(
                host,
                event.pos,
                event.entity,
                transfer_supported_heat_to_neighbor,
            ),
            id if heat_cell::spec(id).is_some() => heat_cell::tick(
                host,
                event.pos,
                event.entity,
                transfer_supported_heat_to_neighbor,
            ),
            WROUGHT_ENERGY_CELL
            | TEMPERED_ENERGY_CELL
            | HARDENED_ENERGY_CELL
            | ARCFORGED_ENERGY_CELL => {
                let (capacity, _) = energy_cell_spec(&event.entity.id).unwrap();
                let stored =
                    (get_i32(&event.entity.nbt, "energy").max(0) as u64).min(capacity) as i32;
                cache_cell_charge(host.dimension(), event.pos, &event.entity.id, stored);
                let charge = energy_cell_charge_level(stored as u64, capacity).to_string();
                if host.block_property(event.pos, "charge").as_deref() != Some(&charge) {
                    host.set_block_property(event.pos, "charge", &charge);
                }
            }
            _ => {}
        }
    }
}

fn furnace_facing_vector(facing: &str) -> Option<(i32, i32)> {
    match facing {
        "north" => Some((0, -1)),
        "east" => Some((1, 0)),
        "south" => Some((0, 1)),
        "west" => Some((-1, 0)),
        _ => None,
    }
}

fn furnace_expected_block(dx: i32, dy: i32, dz: i32, facing: &str) -> Option<&'static str> {
    if !(-1..=1).contains(&dx) || !(-1..=1).contains(&dz) || !(0..6).contains(&dy) {
        return None;
    }
    if dx == 0 && dz == 0 {
        return (dy == 0).then_some(FURNACE_BRICKS);
    }
    if dx != 0 && dz != 0 {
        return Some(FURNACE_BRICK_WALL);
    }
    if dy == 0 && furnace_facing_vector(facing) == Some((dx, dz)) {
        return Some(FURNACE_PORT);
    }
    Some(FURNACE_BRICKS)
}

fn arcforge_furnace_structure_valid(host: &dyn Host, port: [i32; 3]) -> bool {
    let Some(level) = host.level(host.dimension()) else {
        return false;
    };
    let Some(facing) = level.block_property(host, port, "facing") else {
        return false;
    };
    let Some((face_x, face_z)) = furnace_facing_vector(&facing) else {
        return false;
    };
    let center = [port[0] - face_x, port[1], port[2] - face_z];
    for dy in 0..6 {
        for dx in -1..=1 {
            for dz in -1..=1 {
                let pos = [center[0] + dx, center[1] + dy, center[2] + dz];
                let Some(actual) = level.block(host, pos) else {
                    return false;
                };
                match furnace_expected_block(dx, dy, dz, &facing) {
                    Some(expected) if actual == expected => {
                        if expected == FURNACE_PORT
                            && level.block_property(host, pos, "facing").as_deref()
                                != Some(facing.as_str())
                        {
                            return false;
                        }
                    }
                    None if actual == "minecraft:air" => {}
                    _ => return false,
                }
            }
        }
    }
    true
}

fn tick_arcforge_furnace(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    const STRUCTURE_RECHECK_TICKS: i32 = 20;
    let timer = get_i32(&entity.nbt, "FurnaceStructureCheck").saturating_sub(1);
    let formed = if timer <= 0 {
        let formed = arcforge_furnace_structure_valid(host, pos);
        set_i32(&mut entity.nbt, "FurnaceFormed", i32::from(formed));
        set_i32(
            &mut entity.nbt,
            "FurnaceStructureCheck",
            STRUCTURE_RECHECK_TICKS,
        );
        formed
    } else {
        set_i32(&mut entity.nbt, "FurnaceStructureCheck", timer);
        get_i32(&entity.nbt, "FurnaceFormed") != 0
    };
    if formed {
        tick_arcforge_furnace_process(host, entity);
        host.set_block_property(
            pos,
            "lit",
            if get_i32(&entity.nbt, "BurnTime") > 0 {
                "true"
            } else {
                "false"
            },
        );
    } else {
        host.set_block_property(pos, "lit", "false");
        entity.data.resize(8, 0);
        entity.data[0] = get_i32(&entity.nbt, "BurnTime").max(0);
        entity.data[1] = get_i32(&entity.nbt, "BurnDuration").max(800);
        entity.data[2] = 0;
        entity.data[4] = get_i32(&entity.nbt, "TemperatureC").max(20);
        entity.data[5] = 0;
        if let Some(recipe) = arcforge_furnace_recipe(host, entity) {
            entity.data[3] = recipe.ticks.min(i32::MAX as u32) as i32;
            entity.data[6] = recipe.min_temperature_c.min(i32::MAX as u32) as i32;
            entity.data[7] = recipe.coke.min(i32::MAX as u32) as i32;
        } else {
            entity.data[3] = 0;
            entity.data[6] = 0;
            entity.data[7] = 0;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct InductionRecipe {
    output: ItemStack,
    ticks: u32,
}

fn induction_recipe(host: &dyn Host, input: &ItemStack) -> Option<InductionRecipe> {
    host.recipes_of_type("arcforge:induction_smelting")
        .iter()
        .find_map(|recipe| {
            let ingredient = host.recipe_ingredient(&recipe.json.get("ingredient")?.clone())?;
            if !ingredient.test(input) {
                return None;
            }
            let output = host.recipe_result(recipe.json.get("result")?)?;
            let source_ticks = recipe
                .json
                .get("cookingtime")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(200)
                .max(1);
            Some(InductionRecipe {
                output,
                ticks: (source_ticks / 2).max(1).min(u32::MAX as u64) as u32,
            })
        })
}

fn tick_induction_furnace(host: &dyn Host, entity: &mut ModBlockEntity) {
    if entity.items.len() != 3 {
        return;
    }
    let Some(input) = entity.items[0].as_ref() else {
        set_i32(&mut entity.nbt, "CookTime", 0);
        return;
    };
    let recipe = induction_recipe(host, input).or_else(|| {
        smelting_recipe(input.item.as_ref()).map(|item| InductionRecipe {
            output: ItemStack {
                item: Arc::from(item),
                count: 1,
                damage: 0,
                extra: None,
            },
            ticks: 100,
        })
    });
    let Some(recipe) = recipe else {
        set_i32(&mut entity.nbt, "CookTime", 0);
        return;
    };
    if !can_add(
        entity.items[2].as_ref(),
        recipe.output.item.as_ref(),
        recipe.output.count,
    ) {
        return;
    }
    let speed_upgrades =
        get_i32(&entity.nbt, "SpeedUpgrades").clamp(0, SPEED_MAX_UPGRADES as i32) as u32;
    let speed = speed_upgrade_multiplier(speed_upgrades);
    let ticks = speed_upgrade_time(recipe.ticks, speed_upgrades);
    let mut energy = energy::EnergyStore::new(20_000, get_i32(&entity.nbt, "energy").max(0) as u64);
    let energy_cost = scaled_upgrade_cost(
        20,
        speed,
        get_i32(&entity.nbt, "EnergyUpgrades").clamp(0, ENERGY_MAX_UPGRADES as i32) as u32,
    );
    if energy.extract(energy_cost, energy_cost) != energy_cost {
        return;
    }
    let cook = get_i32(&entity.nbt, "CookTime").max(0) + 1;
    if cook >= ticks as i32 {
        consume_one(&mut entity.items[0]);
        add_items(
            &mut entity.items[2],
            recipe.output.item.as_ref(),
            recipe.output.count,
        );
        set_i32(&mut entity.nbt, "CookTime", 0);
    } else {
        set_i32(&mut entity.nbt, "CookTime", cook);
    }
    set_i32(&mut entity.nbt, "energy", energy.stored_fe() as i32);
    entity.data.resize(4, 0);
    entity.data[2] = cook.min(ticks as i32);
    entity.data[3] = ticks.min(i32::MAX as u32) as i32;
}

fn smelting_recipe(item: &str) -> Option<&'static str> {
    match item {
        "minecraft:iron_ore" | "minecraft:deepslate_iron_ore" | "minecraft:raw_iron" => {
            Some("minecraft:iron_ingot")
        }
        "minecraft:copper_ore" | "minecraft:deepslate_copper_ore" | "minecraft:raw_copper" => {
            Some("minecraft:copper_ingot")
        }
        "minecraft:gold_ore"
        | "minecraft:deepslate_gold_ore"
        | "minecraft:raw_gold"
        | "minecraft:nether_gold_ore" => Some("minecraft:gold_ingot"),
        "minecraft:ancient_debris" => Some("minecraft:netherite_scrap"),
        "minecraft:sand" => Some("minecraft:glass"),
        "minecraft:cobblestone" => Some("minecraft:stone"),
        "minecraft:clay_ball" => Some("minecraft:brick"),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq)]
struct PressRecipe {
    input_count: u32,
    output: ItemStack,
    ticks: u32,
}

fn press_recipe(host: &dyn Host, die: &str, input: &ItemStack) -> Option<PressRecipe> {
    host.recipes_of_type("arcforge:pressing")
        .iter()
        .find_map(|recipe| {
            let json = recipe.json.as_ref();
            if json.get("die")?.as_str()? != die {
                return None;
            }
            if !host.recipe_ingredient(json.get("ingredient")?)?.test(input) {
                return None;
            }
            Some(PressRecipe {
                input_count: json
                    .get("count")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(1)
                    .clamp(1, 64) as u32,
                output: host.recipe_result(json.get("result")?)?,
                ticks: json
                    .get("time")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(100)
                    .clamp(1, i32::MAX as u64) as u32,
            })
        })
}

fn tick_metal_press(host: &dyn Host, entity: &mut ModBlockEntity) {
    if entity.items.len() != 3 {
        return;
    }
    let Some(die) = entity.items[1].as_ref() else {
        set_i32(&mut entity.nbt, "CookTime", 0);
        return;
    };
    let Some(input) = entity.items[0].as_ref() else {
        set_i32(&mut entity.nbt, "CookTime", 0);
        return;
    };
    let Some(recipe) = press_recipe(host, die.item.as_ref(), input) else {
        set_i32(&mut entity.nbt, "CookTime", 0);
        return;
    };
    if input.count < recipe.input_count
        || !can_add(
            entity.items[2].as_ref(),
            &recipe.output.item,
            recipe.output.count,
        )
    {
        return;
    }
    let speed_upgrades =
        get_i32(&entity.nbt, "SpeedUpgrades").clamp(0, SPEED_MAX_UPGRADES as i32) as u32;
    let speed = speed_upgrade_multiplier(speed_upgrades);
    let ticks = speed_upgrade_time(recipe.ticks, speed_upgrades);
    let mut energy = energy::EnergyStore::new(20_000, get_i32(&entity.nbt, "energy").max(0) as u64);
    let energy_cost = scaled_upgrade_cost(
        20,
        speed,
        get_i32(&entity.nbt, "EnergyUpgrades").clamp(0, ENERGY_MAX_UPGRADES as i32) as u32,
    );
    if energy.stored_fe() < energy_cost {
        return;
    }
    energy.extract(energy_cost, energy_cost);
    let cook = get_i32(&entity.nbt, "CookTime").max(0) + 1;
    if cook >= ticks as i32 {
        for _ in 0..recipe.input_count {
            consume_one(&mut entity.items[0]);
        }
        add_items(
            &mut entity.items[2],
            &recipe.output.item,
            recipe.output.count,
        );
        set_i32(&mut entity.nbt, "CookTime", 0);
    } else {
        set_i32(&mut entity.nbt, "CookTime", cook);
    }
    set_i32(&mut entity.nbt, "energy", energy.stored_fe() as i32);
    entity.data.resize(4, 0);
    entity.data[2] = cook.min(ticks as i32);
    entity.data[3] = ticks as i32;
}

#[derive(Clone, Debug, PartialEq)]
struct CrusherRecipe {
    result: Option<(String, u32)>,
    bonus: Option<(String, u32)>,
    bonus_chance: f32,
    ticks: u32,
    ore: bool,
}

fn array_yield_multiplier(ore_recipe: bool) -> u32 {
    if ore_recipe { 2 } else { 1 }
}

fn crushing_recipe(host: &dyn Host, input: &ItemStack) -> Option<CrusherRecipe> {
    host.recipes_of_type("arcforge:crushing")
        .iter()
        .find_map(|recipe| {
            let json = recipe.json.as_ref();
            let ingredient = host.recipe_ingredient(json.get("ingredient")?)?;
            if !ingredient.test(input) {
                return None;
            }
            let product = |field: &str| {
                json.get(field)
                    .and_then(|value| host.recipe_result(value))
                    .and_then(|stack| Some((String::from(stack.item.as_ref()), stack.count)))
            };
            let result = product("result");
            let bonus = product("bonus");
            (result.is_some() || bonus.is_some()).then(|| CrusherRecipe {
                result,
                bonus,
                bonus_chance: json
                    .get("bonus_chance")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.25_f64)
                    .clamp(0.0, 1.0) as f32,
                ticks: json
                    .get("time")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(200)
                    .clamp(1, i32::MAX as u64) as u32,
                ore: json
                    .get("ore")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            })
        })
}

fn tick_arc_crusher(host: &mut dyn Host, entity: &mut ModBlockEntity) {
    if entity.items.len() != 3 {
        return;
    }
    let Some(input) = entity.items[0].as_ref() else {
        set_i32(&mut entity.nbt, "CookTime", 0);
        sync_arc_crusher_screen_data(entity, 0, 0);
        return;
    };
    let Some(recipe) = crushing_recipe(host, input) else {
        set_i32(&mut entity.nbt, "CookTime", 0);
        sync_arc_crusher_screen_data(entity, 0, 0);
        return;
    };
    let speed_upgrades =
        get_i32(&entity.nbt, "SpeedUpgrades").clamp(0, SPEED_MAX_UPGRADES as i32) as u32;
    let speed = speed_upgrade_multiplier(speed_upgrades);
    let ticks = speed_upgrade_time(recipe.ticks, speed_upgrades);
    let current_progress = get_i32(&entity.nbt, "CookTime").max(0) as u32;
    if recipe
        .result
        .as_ref()
        .is_some_and(|(item, count)| !can_add(entity.items[1].as_ref(), item, *count))
        || recipe
            .bonus
            .as_ref()
            .is_some_and(|(item, count)| !can_add(entity.items[2].as_ref(), item, *count))
    {
        sync_arc_crusher_screen_data(entity, current_progress, ticks);
        return;
    }
    let mut energy = energy::EnergyStore::new(20_000, get_i32(&entity.nbt, "energy").max(0) as u64);
    let energy_cost = scaled_upgrade_cost(
        20,
        speed,
        get_i32(&entity.nbt, "EnergyUpgrades").clamp(0, ENERGY_MAX_UPGRADES as i32) as u32,
    );
    if energy.extract(energy_cost, energy_cost) != energy_cost {
        sync_arc_crusher_screen_data(entity, current_progress, ticks);
        return;
    }
    let cook = get_i32(&entity.nbt, "CookTime").max(0) + 1;
    if cook >= ticks as i32 {
        consume_one(&mut entity.items[0]);
        if let Some((output, count)) = &recipe.result {
            add_items(&mut entity.items[1], output, *count);
        }
        let dimension = host.dimension();
        if recipe.bonus.is_some()
            && host
                .level_random(dimension)
                .is_some_and(|random| random.next_float() < recipe.bonus_chance)
        {
            if let Some((bonus, count)) = &recipe.bonus {
                add_items(&mut entity.items[2], bonus, *count);
            }
        }
        set_i32(&mut entity.nbt, "CookTime", 0);
    } else {
        set_i32(&mut entity.nbt, "CookTime", cook);
    }
    set_i32(&mut entity.nbt, "energy", energy.stored_fe() as i32);
    sync_arc_crusher_screen_data(
        entity,
        get_i32(&entity.nbt, "CookTime").max(0) as u32,
        ticks,
    );
}

fn sync_arc_crusher_screen_data(entity: &mut ModBlockEntity, progress: u32, duration: u32) {
    entity.data.resize(4, 0);
    entity.data[0] = get_i32(&entity.nbt, "energy").clamp(0, 20_000);
    entity.data[1] = 20_000;
    entity.data[2] = progress.min(duration) as i32;
    entity.data[3] = duration.min(i32::MAX as u32) as i32;
}

fn crushing_array_positions(center: [i32; 3]) -> impl Iterator<Item = [i32; 3]> {
    (-1..=1).flat_map(move |y| {
        (-1..=1)
            .flat_map(move |z| (-1..=1).map(move |x| [center[0] + x, center[1] + y, center[2] + z]))
    })
}

fn is_crushing_array_casing(host: &dyn Host, pos: [i32; 3]) -> bool {
    host.mod_block_entity(pos)
        .is_some_and(|entity| entity.id == ARC_CRUSHING_ARRAY_CASING)
}

fn crushing_array_is_complete(host: &dyn Host, center: [i32; 3]) -> bool {
    crushing_array_positions(center).all(|pos| is_crushing_array_casing(host, pos))
}

fn form_crushing_array(host: &mut dyn Host, center: [i32; 3]) {
    if !crushing_array_is_complete(host, center) {
        return;
    }
    for pos in crushing_array_positions(center) {
        host.set_block_property(pos, "part", if pos == center { "center" } else { "other" });
        host.set_block_property(pos, "facing", "north");
        host.set_block_property(pos, "lit", "false");
    }
}

fn form_crushing_array_near(host: &mut dyn Host, placed: [i32; 3]) {
    for center in crushing_array_positions(placed) {
        if center != placed && crushing_array_is_complete(host, center) {
            form_crushing_array(host, center);
            return;
        }
    }
    if crushing_array_is_complete(host, placed) {
        form_crushing_array(host, placed);
    }
}

fn unform_crushing_array_near(host: &mut dyn Host, pos: [i32; 3]) {
    let centers = crushing_array_positions(pos)
        .filter(|center| host.block_property(*center, "part").as_deref() == Some("center"))
        .collect::<Vec<_>>();
    for center in centers {
        for casing in crushing_array_positions(center) {
            if is_crushing_array_casing(host, casing) {
                host.set_block_property(casing, "part", "none");
                host.set_block_property(casing, "lit", "false");
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FireboxArrayBounds {
    min: [i32; 3],
    max: [i32; 3],
    front: &'static str,
}

impl FireboxArrayBounds {
    fn volume(self) -> u64 {
        (self.max[0] - self.min[0] + 1) as u64
            * (self.max[1] - self.min[1] + 1) as u64
            * (self.max[2] - self.min[2] + 1) as u64
    }

    fn contains(self, pos: [i32; 3]) -> bool {
        (0..3).all(|axis| (self.min[axis]..=self.max[axis]).contains(&pos[axis]))
    }

    fn positions(self) -> impl Iterator<Item = [i32; 3]> {
        (self.min[0]..=self.max[0]).flat_map(move |x| {
            (self.min[1]..=self.max[1])
                .flat_map(move |y| (self.min[2]..=self.max[2]).map(move |z| [x, y, z]))
        })
    }
}

fn firebox_array_bounds(nbt: &horizon_client::mod_api::Compound) -> Option<FireboxArrayBounds> {
    if get_i32(nbt, "ArrayFormed") == 0 {
        return None;
    }
    let bounds = FireboxArrayBounds {
        min: [
            get_i32(nbt, "ArrayMinX"),
            get_i32(nbt, "ArrayMinY"),
            get_i32(nbt, "ArrayMinZ"),
        ],
        max: [
            get_i32(nbt, "ArrayMaxX"),
            get_i32(nbt, "ArrayMaxY"),
            get_i32(nbt, "ArrayMaxZ"),
        ],
        front: match get_string(nbt, "ArrayFront").as_deref() {
            Some("north") => "north",
            Some("east") => "east",
            Some("south") => "south",
            Some("west") => "west",
            _ => "north",
        },
    };
    let valid_size = (0..3).all(|axis| {
        let size = i64::from(bounds.max[axis]) - i64::from(bounds.min[axis]) + 1;
        (3..=i64::from(FIREBOX_ARRAY_MAX_SIZE)).contains(&size)
    });
    valid_size.then_some(bounds)
}

fn firebox_array_direction(name: &str) -> Option<[i32; 3]> {
    match name {
        "north" => Some([0, 0, -1]),
        "east" => Some([1, 0, 0]),
        "south" => Some([0, 0, 1]),
        "west" => Some([-1, 0, 0]),
        _ => None,
    }
}

fn firebox_array_clockwise(name: &str) -> Option<&'static str> {
    match name {
        "north" => Some("east"),
        "east" => Some("south"),
        "south" => Some("west"),
        "west" => Some("north"),
        _ => None,
    }
}

fn firebox_array_opposite(name: &str) -> Option<&'static str> {
    match name {
        "north" => Some("south"),
        "east" => Some("west"),
        "south" => Some("north"),
        "west" => Some("east"),
        _ => None,
    }
}

fn firebox_array_shift(pos: [i32; 3], direction: [i32; 3], distance: i32) -> [i32; 3] {
    [
        pos[0] + direction[0] * distance,
        pos[1] + direction[1] * distance,
        pos[2] + direction[2] * distance,
    ]
}

fn firebox_array_casings_on(host: &dyn Host, pos: [i32; 3], direction: [i32; 3]) -> Vec<i32> {
    (1..FIREBOX_ARRAY_MAX_SIZE - 1)
        .filter(|distance| {
            host.block(firebox_array_shift(pos, direction, *distance))
                .as_str()
                == FIREBOX_ARRAY_CASING
        })
        .collect()
}

fn firebox_array_boundary_count(bounds: FireboxArrayBounds, pos: [i32; 3]) -> usize {
    (0..3)
        .filter(|axis| pos[*axis] == bounds.min[*axis] || pos[*axis] == bounds.max[*axis])
        .count()
}

fn firebox_array_structure_valid(
    host: &dyn Host,
    bounds: FireboxArrayBounds,
    controller: [i32; 3],
    current: Option<FireboxArrayBounds>,
) -> bool {
    let dimensions = (0..3)
        .map(|axis| bounds.max[axis] - bounds.min[axis] + 1)
        .collect::<Vec<_>>();
    if dimensions
        .iter()
        .any(|size| !(3..=FIREBOX_ARRAY_MAX_SIZE).contains(size))
    {
        return false;
    }
    let Some(front) = firebox_array_direction(bounds.front) else {
        return false;
    };
    let faces_controller = match bounds.front {
        "north" => controller[2] == bounds.min[2],
        "east" => controller[0] == bounds.max[0],
        "south" => controller[2] == bounds.max[2],
        "west" => controller[0] == bounds.min[0],
        _ => false,
    };
    if !faces_controller || front[1] != 0 || firebox_array_boundary_count(bounds, controller) != 1 {
        return false;
    }
    for pos in bounds.positions() {
        let id = host.block(pos);
        let claimed = current.is_some_and(|old| old.contains(pos));
        let formed = host.block_property(pos, "formed").as_deref() == Some("true");
        let boundary = firebox_array_boundary_count(bounds, pos);
        let fits = if boundary >= 2 {
            id == FIREBOX_ARRAY_CASING && (claimed || !formed)
        } else if boundary == 1 {
            if pos == controller {
                id == FIREBOX_ARRAY_CONTROLLER
            } else {
                (id == FIREBOX_ARRAY_CASING || id == PRESSURE_GLASS) && (claimed || !formed)
            }
        } else {
            matches!(
                id.as_str(),
                "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
            )
        };
        if !fits {
            return false;
        }
    }
    true
}

fn find_firebox_array(
    host: &dyn Host,
    controller: [i32; 3],
    preferred_front: &str,
    current: Option<FireboxArrayBounds>,
) -> Option<FireboxArrayBounds> {
    let preferred_front = match preferred_front {
        "north" => "north",
        "east" => "east",
        "south" => "south",
        "west" => "west",
        _ => "north",
    };
    let clockwise = firebox_array_clockwise(preferred_front).unwrap_or("east");
    let opposite = firebox_array_opposite(preferred_front).unwrap_or("south");
    let counter_clockwise = firebox_array_opposite(clockwise).unwrap_or("west");
    for front in [preferred_front, clockwise, opposite, counter_clockwise] {
        let Some(inward_name) = firebox_array_opposite(front) else {
            continue;
        };
        let Some(inward) = firebox_array_direction(inward_name) else {
            continue;
        };
        let Some(along_name) = firebox_array_clockwise(front) else {
            continue;
        };
        let Some(along) = firebox_array_direction(along_name) else {
            continue;
        };
        let down = firebox_array_casings_on(host, controller, [0, -1, 0]);
        let up = firebox_array_casings_on(host, controller, [0, 1, 0]);
        let left = firebox_array_casings_on(host, controller, [-along[0], 0, -along[2]]);
        let right = firebox_array_casings_on(host, controller, along);
        for down_distance in &down {
            for up_distance in &up {
                let height = *down_distance + *up_distance + 1;
                if !(3..=FIREBOX_ARRAY_MAX_SIZE).contains(&height) {
                    continue;
                }
                for left_distance in &left {
                    for right_distance in &right {
                        let width = *left_distance + *right_distance + 1;
                        if !(3..=FIREBOX_ARRAY_MAX_SIZE).contains(&width) {
                            continue;
                        }
                        let near = firebox_array_shift(
                            firebox_array_shift(
                                controller,
                                [-along[0], 0, -along[2]],
                                *left_distance,
                            ),
                            [0, -1, 0],
                            *down_distance,
                        );
                        for depth in 3..=FIREBOX_ARRAY_MAX_SIZE {
                            let far = firebox_array_shift(near, inward, depth - 1);
                            if host.block(far).as_str() != FIREBOX_ARRAY_CASING {
                                continue;
                            }
                            let other = firebox_array_shift(
                                firebox_array_shift(controller, along, *right_distance),
                                [0, 1, 0],
                                *up_distance,
                            );
                            let far_other = firebox_array_shift(other, inward, depth - 1);
                            let bounds = FireboxArrayBounds {
                                min: [
                                    near[0].min(far_other[0]),
                                    near[1].min(far_other[1]),
                                    near[2].min(far_other[2]),
                                ],
                                max: [
                                    near[0].max(far_other[0]),
                                    near[1].max(far_other[1]),
                                    near[2].max(far_other[2]),
                                ],
                                front,
                            };
                            if bounds.volume() == (width * height * depth) as u64
                                && firebox_array_structure_valid(host, bounds, controller, current)
                            {
                                return Some(bounds);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn write_firebox_array_bounds(
    nbt: &mut horizon_client::mod_api::Compound,
    bounds: Option<FireboxArrayBounds>,
) {
    set_i32(nbt, "ArrayFormed", if bounds.is_some() { 1 } else { 0 });
    if let Some(bounds) = bounds {
        set_i32(nbt, "ArrayMinX", bounds.min[0]);
        set_i32(nbt, "ArrayMinY", bounds.min[1]);
        set_i32(nbt, "ArrayMinZ", bounds.min[2]);
        set_i32(nbt, "ArrayMaxX", bounds.max[0]);
        set_i32(nbt, "ArrayMaxY", bounds.max[1]);
        set_i32(nbt, "ArrayMaxZ", bounds.max[2]);
        set_string(nbt, "ArrayFront", bounds.front);
        let volume = bounds.volume().clamp(27, 343) as i32;
        set_i32(nbt, "ArrayVolume", volume);
    }
}

fn set_firebox_array_bounds(
    host: &mut dyn Host,
    pos: [i32; 3],
    bounds: Option<FireboxArrayBounds>,
) {
    host.change_mod_block_entity(pos, &mut |entity| {
        if entity.id == FIREBOX_ARRAY_CONTROLLER {
            write_firebox_array_bounds(&mut entity.nbt, bounds);
        }
    });
}

fn set_firebox_array_formed(host: &mut dyn Host, bounds: FireboxArrayBounds, formed: bool) {
    for pos in bounds.positions() {
        match host.block(pos).as_str() {
            FIREBOX_ARRAY_CONTROLLER => {
                host.set_block_property(pos, "formed", if formed { "true" } else { "false" });
                if !formed {
                    host.set_block_property(pos, "lit", "false");
                }
            }
            FIREBOX_ARRAY_CASING | PRESSURE_GLASS => {
                host.set_block_property(pos, "formed", if formed { "true" } else { "false" });
            }
            _ => {}
        }
    }
}

fn unform_firebox_array(host: &mut dyn Host, nbt: &horizon_client::mod_api::Compound) {
    if let Some(bounds) = firebox_array_bounds(nbt) {
        set_firebox_array_formed(host, bounds, false);
    }
}

fn unform_firebox_array_near(host: &mut dyn Host, broken: [i32; 3]) {
    for x in -FIREBOX_ARRAY_MAX_SIZE..=FIREBOX_ARRAY_MAX_SIZE {
        for y in -FIREBOX_ARRAY_MAX_SIZE..=FIREBOX_ARRAY_MAX_SIZE {
            for z in -FIREBOX_ARRAY_MAX_SIZE..=FIREBOX_ARRAY_MAX_SIZE {
                let controller = [broken[0] + x, broken[1] + y, broken[2] + z];
                let Some(entity) = host.mod_block_entity(controller) else {
                    continue;
                };
                if entity.id != FIREBOX_ARRAY_CONTROLLER {
                    continue;
                }
                let Some(bounds) = firebox_array_bounds(&entity.nbt) else {
                    continue;
                };
                if !bounds.contains(broken) {
                    continue;
                }
                set_firebox_array_formed(host, bounds, false);
                set_firebox_array_bounds(host, controller, None);
                break;
            }
        }
    }
}

fn firebox_array_fuel(item: &str) -> Option<(u64, u32)> {
    if item == COKE || item == COKE_BLOCK {
        let base = u64::from(firebox_burn_duration(if item == COKE_BLOCK {
            "minecraft:coal_block"
        } else {
            "minecraft:coal"
        })?) * 80;
        return Some((base * 3 / 2, FIREBOX_ARRAY_COKE_TEMPERATURE));
    }
    Some((
        u64::from(firebox_burn_duration(item)?) * 80,
        FIREBOX_ARRAY_SOLID_TEMPERATURE,
    ))
}

fn tick_firebox_array(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    entity.items.resize(1, None);
    let elapsed = get_i32(&entity.nbt, "ArrayCheck").saturating_add(1);
    let old_bounds = firebox_array_bounds(&entity.nbt);
    let bounds = if elapsed >= FIREBOX_ARRAY_CHECK_INTERVAL {
        set_i32(&mut entity.nbt, "ArrayCheck", 0);
        let preferred = host
            .block_property(pos, "facing")
            .unwrap_or_else(|| "north".into());
        let found = find_firebox_array(host, pos, &preferred, old_bounds);
        if found != old_bounds {
            if let Some(old) = old_bounds {
                set_firebox_array_formed(host, old, false);
            }
            write_firebox_array_bounds(&mut entity.nbt, found);
            if let Some(found) = found {
                host.set_block_property(pos, "facing", found.front);
                set_firebox_array_formed(host, found, true);
            }
        }
        found
    } else {
        set_i32(&mut entity.nbt, "ArrayCheck", elapsed);
        old_bounds
    };
    let volume = get_i32(&entity.nbt, "ArrayVolume").clamp(27, 343) as u64;
    let capacity = FIREBOX_ARRAY_HEAT_CAPACITY_PER_BLOCK * volume;
    let max_rate = (FIREBOX_ARRAY_HEAT_AT_MAX_SIZE * volume + 171) / 343;
    let mut heat = heat::HeatStore::new(
        capacity,
        FIREBOX_ARRAY_MAX_TEMPERATURE,
        get_i32(&entity.nbt, "HeatHU").max(0) as u64,
    );
    let mut burn_hu = get_i32(&entity.nbt, "BurnHU").max(0) as u64;
    let mut burn_temperature = get_i32(&entity.nbt, "BurnTemperatureC")
        .clamp(20, FIREBOX_ARRAY_MAX_TEMPERATURE as i32) as u32;
    if bounds.is_some() && burn_hu == 0 {
        let fuel = entity.items[0]
            .as_ref()
            .and_then(|item| firebox_array_fuel(&item.item));
        if let Some((fuel_hu, temperature)) = fuel {
            if consume_one(&mut entity.items[0]) {
                burn_hu = fuel_hu;
                burn_temperature = temperature;
            }
        }
    }
    let burn_threshold =
        heat_at_temperature(capacity, FIREBOX_ARRAY_MAX_TEMPERATURE, burn_temperature);
    let produced = if bounds.is_some() && burn_hu > 0 && heat.stored_hu() < burn_threshold {
        let amount = burn_hu.min(max_rate).min(burn_threshold - heat.stored_hu());
        let accepted = heat.add_hu(amount);
        burn_hu -= accepted;
        accepted
    } else {
        0
    };
    heat.set_source_temperature(if produced > 0 { burn_temperature } else { 20 });
    if bounds.is_some() {
        transfer_firebox_heat_rate(host, pos, &mut heat, max_rate, max_rate);
        host.set_block_property(pos, "lit", if produced > 0 { "true" } else { "false" });
    } else {
        host.set_block_property(pos, "lit", "false");
    }
    set_i32(
        &mut entity.nbt,
        "BurnHU",
        burn_hu.min(i32::MAX as u64) as i32,
    );
    set_i32(&mut entity.nbt, "BurnTemperatureC", burn_temperature as i32);
    set_i32(
        &mut entity.nbt,
        "HeatHU",
        heat.stored_hu().min(i32::MAX as u64) as i32,
    );
    set_i32(&mut entity.nbt, "TemperatureC", heat.temperature_c() as i32);
    entity.data.resize(9, 0);
    entity.data[0] = heat.stored_hu().min(i32::MAX as u64) as i32;
    entity.data[1] = capacity.min(i32::MAX as u64) as i32;
    entity.data[2] = heat.temperature_c() as i32;
    entity.data[3] = produced.min(i32::MAX as u64) as i32;
    entity.data[4] = max_rate.min(i32::MAX as u64) as i32;
    entity.data[5] = burn_hu.min(i32::MAX as u64) as i32;
    entity.data[6] = burn_temperature as i32;
    entity.data[7] = volume as i32;
    entity.data[8] = if bounds.is_some() { 1 } else { 0 };
}

fn tick_solar_array(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let previous = solar::tower_from_nbt(&entity.nbt);
    let timer = get_i32(&entity.nbt, "SolarStructureCheck").saturating_sub(1);
    let tower = if timer <= 0 {
        let facing = host
            .block_property(pos, "facing")
            .unwrap_or_else(|| "north".to_owned());
        let (found, faces_in) = solar::find_tower(host, pos, &facing, previous);
        let found = if faces_in { None } else { found };
        if found != previous {
            if let Some(previous) = previous {
                solar::set_formed(host, previous, false);
            }
            if let Some(found) = found {
                solar::set_formed(host, found, true);
                solar::sync_port_properties(host, found, &entity.nbt);
            }
            solar::write_tower(&mut entity.nbt, found);
        }
        set_i32(
            &mut entity.nbt,
            "SolarStructureCheck",
            SOLAR_ARRAY_SCAN_INTERVAL_TICKS,
        );
        found
    } else {
        set_i32(&mut entity.nbt, "SolarStructureCheck", timer);
        previous
    };

    let mut heat = heat::HeatStore::new(
        SOLAR_ARRAY_HEAT_CAPACITY,
        SOLAR_ARRAY_MAX_TEMPERATURE,
        get_i32(&entity.nbt, "HeatHU").max(0) as u64,
    );
    let has_sky = host.dimension() == "minecraft:overworld";
    let sky_collectors = tower
        .filter(|_| has_sky)
        .map(|tower| {
            (0..4)
                .filter(|index| {
                    let collector = tower.collector(*index);
                    host.light([collector[0], collector[1] + 1, collector[2]]).1 == 15
                })
                .count() as u32
        })
        .unwrap_or(0);
    let (rain, thunder) = host.weather();
    let weather = if thunder > 0.0 {
        solar::Weather::Thunder
    } else if rain > 0.0 {
        solar::Weather::Rain
    } else {
        solar::Weather::Clear
    };
    let result = solar::compute(
        host.day_time(),
        weather,
        tower.is_some_and(|tower| tower.north_south),
        sky_collectors,
        has_sky,
    );

    let safe_temperature = result.temperature.min(SOLAR_ARRAY_MAX_TEMPERATURE);
    let temperature_limit = heat_at_temperature(
        SOLAR_ARRAY_HEAT_CAPACITY,
        SOLAR_ARRAY_MAX_TEMPERATURE,
        safe_temperature,
    );
    if result.heat_per_tick > 0 && heat.stored_hu() < temperature_limit {
        heat.add_hu(
            result
                .heat_per_tick
                .min(temperature_limit - heat.stored_hu()),
        );
    }
    heat.set_source_temperature(if result.heat_per_tick > 0 {
        safe_temperature
    } else {
        20
    });
    if heat.stored_hu() > temperature_limit {
        let cooling_rate = (SOLAR_ARRAY_HEAT_CAPACITY / 200).max(1);
        heat.remove_hu((heat.stored_hu() - temperature_limit).min(cooling_rate));
    }
    if tower.is_some() {
        if let Some(tower) = tower {
            transfer_solar_heat_ports(host, tower, &entity.nbt, &mut heat, result.heat_per_tick);
        }
        host.set_block_property(
            pos,
            "lit",
            if result.heat_per_tick > 0 {
                "true"
            } else {
                "false"
            },
        );
    } else {
        host.set_block_property(pos, "lit", "false");
    }

    set_i32(
        &mut entity.nbt,
        "HeatHU",
        heat.stored_hu().min(i32::MAX as u64) as i32,
    );
    set_i32(&mut entity.nbt, "TemperatureC", heat.temperature_c() as i32);
    set_i32(
        &mut entity.nbt,
        "HeatPerTick",
        result.heat_per_tick.min(i32::MAX as u64) as i32,
    );
    set_i32(
        &mut entity.nbt,
        "SkyCollectors",
        sky_collectors.min(i32::MAX as u32) as i32,
    );
    set_i32(&mut entity.nbt, "Stowed", i32::from(result.stowed));
    set_i32(&mut entity.nbt, "Night", i32::from(result.night));
    entity.data.resize(8, 0);
    entity.data[0] = heat.stored_hu().min(i32::MAX as u64) as i32;
    entity.data[1] = SOLAR_ARRAY_HEAT_CAPACITY as i32;
    entity.data[2] = heat.temperature_c() as i32;
    entity.data[3] = result.heat_per_tick.min(i32::MAX as u64) as i32;
    entity.data[4] = safe_temperature as i32;
    entity.data[5] = sky_collectors as i32;
    entity.data[6] = if result.stowed { 1 } else { 0 };
    entity.data[7] = if tower.is_some() { 1 } else { 0 };
}

fn consolidate_crushing_array_energy(
    host: &mut dyn Host,
    center: [i32; 3],
    entity: &mut ModBlockEntity,
) {
    let mut stored = (get_i32(&entity.nbt, "energy").max(0) as u64).min(100_000);
    for pos in crushing_array_positions(center).filter(|pos| *pos != center) {
        if stored >= 100_000 {
            break;
        }
        let capacity = 100_000 - stored;
        let mut moved = 0i32;
        host.change_mod_block_entity(pos, &mut |entity| {
            if entity.id != ARC_CRUSHING_ARRAY_CASING {
                return;
            }
            let available = get_i32(&entity.nbt, "energy").max(0) as u64;
            moved = available.min(capacity).min(i32::MAX as u64) as i32;
            set_i32(&mut entity.nbt, "energy", (available - moved as u64) as i32);
        });
        if moved > 0 {
            stored += moved as u64;
        }
    }
    set_i32(&mut entity.nbt, "energy", stored as i32);
}

fn tick_crushing_array_casing(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    match host.block_property(pos, "part").as_deref() {
        Some("center") => {
            if !crushing_array_is_complete(host, pos) {
                unform_crushing_array_near(host, pos);
                return;
            }
            consolidate_crushing_array_energy(host, pos, entity);
            tick_crushing_array(host, entity);
            let lit = (0..3).any(|lane| get_i32(&entity.nbt, &format!("Lane{lane}CookTime")) > 0);
            host.set_block_property(pos, "lit", if lit { "true" } else { "false" });
        }
        Some("other") => {
            let has_center = crushing_array_positions(pos).any(|candidate| {
                host.block_property(candidate, "part").as_deref() == Some("center")
            });
            if !has_center {
                host.set_block_property(pos, "part", "none");
                host.set_block_property(pos, "lit", "false");
            }
        }
        _ => {
            let elapsed = get_i32(&entity.nbt, "formation_check").saturating_add(1);
            if elapsed >= 20 {
                set_i32(&mut entity.nbt, "formation_check", 0);
                form_crushing_array_near(host, pos);
            } else {
                set_i32(&mut entity.nbt, "formation_check", elapsed);
            }
        }
    }
}

fn tick_crushing_array(host: &mut dyn Host, entity: &mut ModBlockEntity) {
    if entity.items.len() != 9 {
        return;
    }
    let mut energy =
        energy::EnergyStore::new(100_000, get_i32(&entity.nbt, "energy").max(0) as u64);
    let speed_upgrades =
        get_i32(&entity.nbt, "SpeedUpgrades").clamp(0, SPEED_MAX_UPGRADES as i32) as u32;
    let speed = speed_upgrade_multiplier(speed_upgrades);
    let energy_upgrades =
        get_i32(&entity.nbt, "EnergyUpgrades").clamp(0, ENERGY_MAX_UPGRADES as i32) as u32;
    let first_lane = get_i32(&entity.nbt, "NextLane").rem_euclid(3) as usize;
    for offset in 0..3 {
        let lane = (first_lane + offset) % 3;
        let input_slot = lane;
        let output_slot = lane + 3;
        let bonus_slot = lane + 6;
        let Some(input) = entity.items[input_slot].as_ref() else {
            set_i32(&mut entity.nbt, &format!("Lane{lane}CookTime"), 0);
            continue;
        };
        let Some(recipe) = crushing_recipe(host, input) else {
            set_i32(&mut entity.nbt, &format!("Lane{lane}CookTime"), 0);
            continue;
        };
        let yield_multiplier = array_yield_multiplier(recipe.ore);
        let energy_cost = scaled_upgrade_cost(16, speed, energy_upgrades);
        let ticks = speed_upgrade_time(recipe.ticks, speed_upgrades);
        if recipe.result.as_ref().is_some_and(|(item, count)| {
            !can_add(
                entity.items[output_slot].as_ref(),
                item,
                count.saturating_mul(yield_multiplier),
            )
        }) || recipe
            .bonus
            .as_ref()
            .is_some_and(|(item, count)| !can_add(entity.items[bonus_slot].as_ref(), item, *count))
            || energy.stored_fe() < energy_cost
        {
            continue;
        }
        energy.extract(energy_cost, energy_cost);
        let key = format!("Lane{lane}CookTime");
        let cook = get_i32(&entity.nbt, &key).max(0) + 1;
        if cook >= ticks as i32 {
            consume_one(&mut entity.items[input_slot]);
            if let Some((output, count)) = &recipe.result {
                add_items(
                    &mut entity.items[output_slot],
                    output,
                    count.saturating_mul(yield_multiplier),
                );
            }
            let dimension = host.dimension();
            if recipe.bonus.is_some()
                && host
                    .level_random(dimension)
                    .is_some_and(|random| random.next_float() < recipe.bonus_chance)
            {
                if let Some((bonus, count)) = &recipe.bonus {
                    add_items(&mut entity.items[bonus_slot], bonus, *count);
                }
            }
            set_i32(&mut entity.nbt, &key, 0);
        } else {
            set_i32(&mut entity.nbt, &key, cook);
        }
        entity.data.resize(12, 0);
        entity.data[lane] = cook.min(ticks as i32);
    }
    // Rotate priority so low FE input cannot permanently starve later lanes.
    set_i32(&mut entity.nbt, "NextLane", ((first_lane + 1) % 3) as i32);
    set_i32(&mut entity.nbt, "energy", energy.stored_fe() as i32);
    entity.data.resize(12, 0);
    entity.data[3] = 100;
}

fn add_items(slot: &mut Option<ItemStack>, item: &str, count: u32) {
    match slot.as_mut() {
        Some(stack) => stack.count += count,
        None => {
            *slot = Some(ItemStack {
                item: Arc::from(item),
                count,
                damage: 0,
                extra: None,
            })
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResinTapSource {
    Latex,
    Spruce,
    Other,
}

fn resin_tap_source(host: &dyn Host, log_pos: [i32; 3]) -> Option<ResinTapSource> {
    let level = host.level(host.dimension())?;
    let log = level.block(host, log_pos)?;
    if !host
        .block_name_in_tag(&log, "minecraft:logs")
        .unwrap_or(false)
    {
        return None;
    }
    if host
        .block_name_in_tag(&log, "minecraft:jungle_logs")
        .unwrap_or(false)
    {
        Some(ResinTapSource::Latex)
    } else if [
        "minecraft:spruce_log",
        "minecraft:spruce_wood",
        "minecraft:stripped_spruce_log",
        "minecraft:stripped_spruce_wood",
    ]
    .contains(&log.as_str())
    {
        Some(ResinTapSource::Spruce)
    } else {
        Some(ResinTapSource::Other)
    }
}

fn resin_tap_log_pos(host: &dyn Host, pos: [i32; 3]) -> Option<[i32; 3]> {
    let level = host.level(host.dimension())?;
    let facing = level.block_property(host, pos, "facing")?;
    let mut log = pos;
    match facing.as_str() {
        "north" => log[2] += 1,
        "south" => log[2] -= 1,
        "east" => log[0] -= 1,
        "west" => log[0] += 1,
        _ => return None,
    }
    Some(log)
}

fn resin_tap_has_living_tree(host: &dyn Host, log: [i32; 3]) -> bool {
    let Some(level) = host.level(host.dimension()) else {
        return false;
    };
    let mut gap = 0;
    for dy in 1..=12 {
        for dx in -1..=1 {
            for dz in -1..=1 {
                let pos = [log[0] + dx, log[1] + dy, log[2] + dz];
                let Some(block) = level.block(host, pos) else {
                    continue;
                };
                if host
                    .block_name_in_tag(&block, "minecraft:leaves")
                    .unwrap_or(false)
                    && level.block_property(host, pos, "persistent").as_deref() != Some("true")
                {
                    return true;
                }
            }
        }
        let trunk = [log[0], log[1] + dy, log[2]];
        let is_log = level.block(host, trunk).is_some_and(|block| {
            host.block_name_in_tag(&block, "minecraft:logs")
                .unwrap_or(false)
        });
        gap = if is_log { 0 } else { gap + 1 };
        if gap > 2 {
            return false;
        }
    }
    false
}

fn insert_resin_tap_resin(entity: &mut ModBlockEntity) -> bool {
    let Some(slot) = entity.items.first_mut() else {
        return false;
    };
    match slot {
        Some(stack) if stack.item.as_ref() == PINE_RESIN && stack.count < RESIN_TAP_MAX_RESIN => {
            stack.count += 1;
            true
        }
        None => {
            *slot = Some(ItemStack {
                item: Arc::from(PINE_RESIN),
                count: 1,
                damage: 0,
                extra: None,
            });
            true
        }
        _ => false,
    }
}

fn tick_resin_tap(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let dimension = host.dimension();
    let timer = get_i32(&entity.nbt, "ResinTapTimer")
        .max(0)
        .saturating_add(1);
    if timer >= RESIN_TAP_INTERVAL {
        set_i32(&mut entity.nbt, "ResinTapTimer", 0);
        if let Some(log_pos) = resin_tap_log_pos(host, pos)
            && let Some(source) = resin_tap_source(host, log_pos)
            && resin_tap_has_living_tree(host, log_pos)
        {
            match source {
                ResinTapSource::Latex => {
                    fluid::insert(
                        &mut entity.nbt,
                        0,
                        "arcforge:latex",
                        RESIN_TAP_LATEX_PER_DRIP,
                        RESIN_TAP_TANK_CAPACITY,
                    );
                }
                ResinTapSource::Spruce | ResinTapSource::Other => {
                    let chance = if source == ResinTapSource::Spruce {
                        0.5
                    } else {
                        0.15
                    };
                    if host
                        .level_random(dimension)
                        .is_some_and(|random| random.next_double() < chance)
                    {
                        insert_resin_tap_resin(entity);
                    }
                }
            }
        }
    } else {
        set_i32(&mut entity.nbt, "ResinTapTimer", timer);
    }

    let content = if fluid::amount(&entity.nbt, 0) > 0 {
        "latex"
    } else if entity
        .items
        .first()
        .and_then(Option::as_ref)
        .is_some_and(|stack| stack.item.as_ref() == PINE_RESIN && stack.count > 0)
    {
        "resin"
    } else {
        "empty"
    };
    if let Some(level) = host.level(dimension)
        && level.block_property(host, pos, "content").as_deref() != Some(content)
    {
        host.set_block_property(pos, "content", content);
    }
}

#[derive(Clone, Debug, PartialEq)]
struct CarbonizerRecipe {
    output: ItemStack,
    fluid: Option<(String, u32)>,
    ticks: u32,
}

fn carbonizer_recipe(host: &dyn Host, input: &ItemStack) -> Option<CarbonizerRecipe> {
    host.recipes_of_type("arcforge:carbonizing")
        .iter()
        .find_map(|recipe| {
            let json = recipe.json.as_ref();
            let ingredient = host.recipe_ingredient(json.get("ingredient")?)?;
            if !ingredient.test(input) {
                return None;
            }
            let output = host.recipe_result(json.get("result")?)?;
            let fluid = json.get("byproduct").and_then(|value| {
                Some((
                    value.get("id")?.as_str()?.to_owned(),
                    value.get("amount")?.as_u64()?.try_into().ok()?,
                ))
            });
            Some(CarbonizerRecipe {
                output,
                fluid,
                ticks: json
                    .get("time")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(200)
                    .clamp(1, i32::MAX as u64) as u32,
            })
        })
}

fn tick_carbonizer(host: &dyn Host, entity: &mut ModBlockEntity) {
    if entity.items.len() != 3 {
        return;
    }
    let Some(input) = entity.items[0].as_ref() else {
        set_i32(&mut entity.nbt, "CookTime", 0);
        return;
    };
    let Some(recipe) = carbonizer_recipe(host, input) else {
        set_i32(&mut entity.nbt, "CookTime", 0);
        return;
    };
    let fluid_fits = recipe
        .fluid
        .as_ref()
        .is_none_or(|(name, amount)| fluid::can_insert(&entity.nbt, 0, name, *amount, 8000));
    if !can_add(
        entity.items[2].as_ref(),
        &recipe.output.item,
        recipe.output.count,
    ) || !fluid_fits
    {
        return;
    }
    let cook = get_i32(&entity.nbt, "CookTime").max(0) + 1;
    if cook >= recipe.ticks as i32 {
        if consume_one(&mut entity.items[0]) {
            add_items(
                &mut entity.items[2],
                &recipe.output.item,
                recipe.output.count,
            );
            if let Some((fluid, amount)) = &recipe.fluid {
                fluid::insert(&mut entity.nbt, 0, fluid, *amount, 8000);
            }
        }
        set_i32(&mut entity.nbt, "CookTime", 0);
    } else {
        set_i32(&mut entity.nbt, "CookTime", cook);
    }
    entity.data.resize(4, 0);
    entity.data[2] = cook.min(recipe.ticks as i32);
    entity.data[3] = recipe.ticks as i32;
}

#[derive(Clone, Debug, PartialEq)]
struct InfuserRecipe {
    output: ItemStack,
    fluid: Option<(String, u32)>,
    additive_count: Option<u32>,
    ticks: u32,
}

#[derive(Clone, Debug)]
struct InfuserCandidate {
    recipe: InfuserRecipe,
    additive_ready: bool,
    fluid_ready: bool,
}

fn infuser_candidate(
    host: &dyn Host,
    input: &ItemStack,
    fluid_name: Option<&str>,
    fluid_amount: u32,
    additive: Option<&ItemStack>,
) -> Option<InfuserCandidate> {
    let candidates: Vec<_> = host
        .recipes_of_type("arcforge:infusing")
        .iter()
        .filter_map(|entry| {
            let json = entry.json.as_ref();
            if !host.recipe_ingredient(json.get("ingredient")?)?.test(input) {
                return None;
            }
            let output = host.recipe_result(json.get("result")?)?;
            let fluid = json.get("fluid").and_then(|value| {
                Some((
                    value.get("id")?.as_str()?.to_owned(),
                    u32::try_from(value.get("amount")?.as_u64()?).ok()?,
                ))
            });
            let additive_count = json.get("additive").and_then(|value| {
                u32::try_from(
                    value
                        .get("count")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(1),
                )
                .ok()
            });
            let additive_ready = json
                .get("additive")
                .and_then(|value| value.get("ingredient"))
                .and_then(|ingredient| host.recipe_ingredient(ingredient))
                .zip(additive)
                .is_some_and(|(ingredient, stack)| {
                    additive_count.is_some_and(|count| stack.count >= count)
                        && ingredient.test(stack)
                });
            let fluid_ready = fluid.as_ref().is_some_and(|(name, needed)| {
                fluid_name == Some(name.as_str()) && fluid_amount >= *needed
            });
            Some(InfuserCandidate {
                recipe: InfuserRecipe {
                    output,
                    fluid,
                    additive_count,
                    ticks: json
                        .get("time")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(100)
                        .clamp(1, i32::MAX as u64) as u32,
                },
                additive_ready,
                fluid_ready,
            })
        })
        .collect();
    candidates
        .iter()
        .find(|candidate| candidate.additive_ready)
        .or_else(|| candidates.iter().find(|candidate| candidate.fluid_ready))
        .or_else(|| candidates.first())
        .cloned()
}

fn infuser_has_input(host: &dyn Host, stack: &ItemStack) -> bool {
    host.recipes_of_type("arcforge:infusing")
        .iter()
        .any(|entry| {
            entry
                .json
                .get("ingredient")
                .and_then(|ingredient| host.recipe_ingredient(ingredient))
                .is_some_and(|ingredient| ingredient.test(stack))
        })
}

fn infuser_has_additive(host: &dyn Host, stack: &ItemStack) -> bool {
    host.recipes_of_type("arcforge:infusing")
        .iter()
        .any(|entry| {
            entry
                .json
                .get("additive")
                .and_then(|value| value.get("ingredient"))
                .and_then(|ingredient| host.recipe_ingredient(ingredient))
                .is_some_and(|ingredient| ingredient.test(stack))
        })
}

fn item_fluid(stack: &ItemStack) -> Option<(String, u32)> {
    let Tag::Compound(data) = stack.extra.as_deref()?.other("minecraft:custom_data")? else {
        return None;
    };
    let Tag::Compound(tank) = data.get(&"horizon_fluid".into())? else {
        return None;
    };
    let fluid = match tank.get(&"Fluid".into())? {
        Tag::String(value) => value.to_string_lossy(),
        _ => return None,
    };
    let amount = match tank.get(&"Amount".into())? {
        Tag::Int(value) => (*value).max(0) as u32,
        _ => return None,
    };
    (amount > 0).then_some((fluid, amount))
}

fn item_with_fluid(stack: &ItemStack, fluid: &str, amount: u32) -> ItemStack {
    let mut result = stack.clone();
    result.count = 1;
    let mut custom = match result
        .extra
        .as_deref()
        .and_then(|extra| extra.other("minecraft:custom_data"))
    {
        Some(Tag::Compound(data)) => data.clone(),
        _ => Default::default(),
    };
    if amount == 0 {
        custom.remove(&"horizon_fluid".into());
    } else {
        custom.insert(
            "horizon_fluid".into(),
            Tag::compound([
                ("Fluid", Tag::String(fluid.into())),
                ("Amount", Tag::Int(amount.min(i32::MAX as u32) as i32)),
            ]),
        );
    }
    let custom = (!custom.is_empty()).then_some(Tag::Compound(custom));
    if result.extra.is_none() && custom.is_some() {
        result.extra = Some(Box::default());
    }
    if let Some(extra) = result.extra.as_mut() {
        extra.set_other("minecraft:custom_data", custom);
    }
    result
}

fn fluid_tank_capacity(id: &str) -> Option<u32> {
    match id {
        GEOTHERMAL_PLANT => Some(GEOTHERMAL_LAVA_TANK_CAPACITY),
        WROUGHT_FLUID_TANK => Some(WROUGHT_FLUID_TANK_CAPACITY),
        TEMPERED_FLUID_TANK => Some(TEMPERED_FLUID_TANK_CAPACITY),
        HARDENED_FLUID_TANK => Some(HARDENED_FLUID_TANK_CAPACITY),
        ARCFORGED_FLUID_TANK => Some(ARCFORGED_FLUID_TANK_CAPACITY),
        _ => None,
    }
}

fn fluid_bucket_contents(item: &str) -> Option<&'static str> {
    match item {
        "minecraft:water_bucket" => Some("minecraft:water"),
        "minecraft:lava_bucket" => Some("minecraft:lava"),
        CREOSOTE_BUCKET => Some("arcforge:creosote"),
        LATEX_BUCKET => Some("arcforge:latex"),
        _ => None,
    }
}

fn canister_spec(item: &str) -> Option<(u32, u32)> {
    match item {
        WROUGHT_CANISTER => Some((16_000, 1_000)),
        TEMPERED_CANISTER => Some((64_000, 4_000)),
        HARDENED_CANISTER => Some((256_000, 16_000)),
        ARCFORGED_CANISTER => Some((1_024_000, 64_000)),
        _ => None,
    }
}

fn bucket_for_fluid(fluid: &str) -> Option<&'static str> {
    match fluid {
        "minecraft:water" => Some("minecraft:water_bucket"),
        "minecraft:lava" => Some("minecraft:lava_bucket"),
        "arcforge:creosote" => Some(CREOSOTE_BUCKET),
        "arcforge:latex" => Some(LATEX_BUCKET),
        _ => None,
    }
}

fn is_fluid_bucket(item: &str) -> bool {
    item == "minecraft:bucket" || fluid_bucket_contents(item).is_some()
}

fn fluid_bucket_face(face: [i32; 3]) -> Option<Direction> {
    match face {
        [0, -1, 0] => Some(Direction::Down),
        [0, 1, 0] => Some(Direction::Up),
        [0, 0, -1] => Some(Direction::North),
        [0, 0, 1] => Some(Direction::South),
        [-1, 0, 0] => Some(Direction::West),
        [1, 0, 0] => Some(Direction::East),
        _ => None,
    }
}

#[cfg(test)]
mod fluid_bucket_tests {
    use super::{
        ARCFORGED_CANISTER, ARCFORGED_FLUID_TANK, ARCFORGED_FLUID_TANK_CAPACITY, CREOSOTE_BUCKET,
        HARDENED_CANISTER, HARDENED_FLUID_TANK, HARDENED_FLUID_TANK_CAPACITY, LATEX_BUCKET,
        TEMPERED_CANISTER, TEMPERED_FLUID_TANK, TEMPERED_FLUID_TANK_CAPACITY, WROUGHT_CANISTER,
        WROUGHT_FLUID_TANK, WROUGHT_FLUID_TANK_CAPACITY, bucket_for_fluid, canister_spec,
        fluid_bucket, fluid_bucket_contents, fluid_bucket_face, fluid_tank_capacity, item_fluid,
        item_with_fluid, use_fluid_tank_bucket,
    };
    use horizon_client::{
        containers::ModBlockEntity,
        mod_api::{Direction, Flow, Host, ItemStack, Tag},
    };
    use std::{collections::HashMap, sync::Arc};

    struct FluidTankTestHost {
        capacity: u32,
        tank: Option<(String, u32)>,
        inventory: HashMap<String, u32>,
        given: Vec<ItemStack>,
        held: Option<ItemStack>,
    }

    impl Host for FluidTankTestHost {
        fn dimension(&self) -> &'static str {
            "minecraft:overworld"
        }

        fn mod_block_entity(&self, _pos: [i32; 3]) -> Option<ModBlockEntity> {
            Some(ModBlockEntity::from(
                horizon_client::containers::ModBlockEntityFields {
                    id: WROUGHT_FLUID_TANK.to_owned(),
                    ..Default::default()
                },
            ))
        }

        fn fluid_insert(
            &mut self,
            _pos: [i32; 3],
            _face: Direction,
            fluid: &str,
            amount: i32,
        ) -> i32 {
            let amount = amount.max(0) as u32;
            if self
                .tank
                .as_ref()
                .is_some_and(|(current, _)| current != fluid)
            {
                return amount as i32;
            }
            let current = self.tank.as_ref().map_or(0, |(_, amount)| *amount);
            let inserted = amount.min(self.capacity.saturating_sub(current));
            if inserted > 0 {
                self.tank = Some((fluid.to_owned(), current + inserted));
            }
            (amount - inserted) as i32
        }

        fn fluid_extract(
            &mut self,
            _dimension: &str,
            _pos: [i32; 3],
            _face: Direction,
            amount: i32,
        ) -> Option<(String, i32)> {
            let (fluid, stored) = self.tank.as_mut()?;
            let extracted = (*stored).min(amount.max(0) as u32);
            let result = (fluid.clone(), extracted as i32);
            *stored -= extracted;
            if *stored == 0 {
                self.tank = None;
            }
            Some(result)
        }

        fn take_items(&mut self, item: &str, count: u32) -> u32 {
            let Some(stored) = self.inventory.get_mut(item) else {
                return 0;
            };
            if *stored < count {
                return 0;
            }
            *stored -= count;
            count
        }

        fn give_items(&mut self, item: &str, count: u32) -> u32 {
            *self.inventory.entry(item.to_owned()).or_default() += count;
            count
        }

        fn give_stack(&mut self, _uuid: u128, stack: ItemStack) -> Option<ItemStack> {
            *self.inventory.entry(stack.item.to_string()).or_default() += stack.count;
            self.given.push(stack);
            None
        }

        fn carried_stack(&mut self, slot: i32) -> Option<&mut ItemStack> {
            (slot == 0).then_some(())?;
            self.held.as_mut()
        }
    }

    fn test_bucket(item: &str, count: u32) -> ItemStack {
        ItemStack {
            item: Arc::from(item),
            count,
            damage: 0,
            extra: None,
        }
    }

    fn fluid_tank_host(capacity: u32, item: &str) -> FluidTankTestHost {
        FluidTankTestHost {
            capacity,
            tank: None,
            inventory: HashMap::from([(item.to_owned(), 1)]),
            given: Vec::new(),
            held: Some(test_bucket(item, 1)),
        }
    }

    fn item_count(host: &FluidTankTestHost, item: &str) -> u32 {
        host.inventory.get(item).copied().unwrap_or(0)
    }

    #[test]
    fn fluid_bucket_mapping_is_bidirectional_for_supported_fluids() {
        for (item, fluid) in [
            ("minecraft:water_bucket", "minecraft:water"),
            ("minecraft:lava_bucket", "minecraft:lava"),
            ("arcforge:creosote_bucket", "arcforge:creosote"),
            (LATEX_BUCKET, "arcforge:latex"),
        ] {
            assert_eq!(fluid_bucket_contents(item), Some(fluid));
            assert_eq!(bucket_for_fluid(fluid), Some(item));
        }
        assert_eq!(fluid_bucket_contents("minecraft:milk_bucket"), None);
        assert_eq!(bucket_for_fluid("arcforge:unknown"), None);
    }

    #[test]
    fn each_fluid_tank_tier_uses_its_registered_capacity() {
        for (id, capacity) in [
            (WROUGHT_FLUID_TANK, WROUGHT_FLUID_TANK_CAPACITY),
            (TEMPERED_FLUID_TANK, TEMPERED_FLUID_TANK_CAPACITY),
            (HARDENED_FLUID_TANK, HARDENED_FLUID_TANK_CAPACITY),
            (ARCFORGED_FLUID_TANK, ARCFORGED_FLUID_TANK_CAPACITY),
        ] {
            assert_eq!(fluid_tank_capacity(id), Some(capacity));
        }
        assert_eq!(fluid_tank_capacity("arcforge:fuel_burner"), None);
        assert_eq!(canister_spec(WROUGHT_CANISTER), Some((16_000, 1_000)));
        assert_eq!(canister_spec(TEMPERED_CANISTER), Some((64_000, 4_000)));
        assert_eq!(canister_spec(HARDENED_CANISTER), Some((256_000, 16_000)));
        assert_eq!(canister_spec(ARCFORGED_CANISTER), Some((1_024_000, 64_000)));
    }

    #[test]
    fn clicked_block_face_maps_to_horizon_direction() {
        assert_eq!(fluid_bucket_face([0, -1, 0]), Some(Direction::Down));
        assert_eq!(fluid_bucket_face([0, 1, 0]), Some(Direction::Up));
        assert_eq!(fluid_bucket_face([0, 0, -1]), Some(Direction::North));
        assert_eq!(fluid_bucket_face([0, 0, 1]), Some(Direction::South));
        assert_eq!(fluid_bucket_face([-1, 0, 0]), Some(Direction::West));
        assert_eq!(fluid_bucket_face([1, 0, 0]), Some(Direction::East));
        assert_eq!(fluid_bucket_face([0, 0, 0]), None);
    }

    #[test]
    fn water_bucket_round_trips_through_fluid_tank() {
        let mut host = fluid_tank_host(16_000, "minecraft:water_bucket");
        let filled = test_bucket("minecraft:water_bucket", 1);
        assert_eq!(
            use_fluid_tank_bucket(&mut host, [0, 64, 0], Direction::Up, &filled, Some(7), 0,),
            Flow::Cancel
        );
        assert_eq!(host.tank, Some(("minecraft:water".into(), 1000)));
        assert_eq!(item_count(&host, "minecraft:water_bucket"), 0);
        assert_eq!(item_count(&host, "minecraft:bucket"), 1);

        let empty = test_bucket("minecraft:bucket", 1);
        assert_eq!(
            use_fluid_tank_bucket(&mut host, [0, 64, 0], Direction::North, &empty, Some(7), 0,),
            Flow::Cancel
        );
        assert_eq!(host.tank, None);
        assert_eq!(item_count(&host, "minecraft:water_bucket"), 1);
        assert_eq!(item_count(&host, "minecraft:bucket"), 0);
    }

    #[test]
    fn arcforge_fluid_containers_exchange_partial_amounts_with_tanks() {
        let mut host = fluid_tank_host(16_000, CREOSOTE_BUCKET);
        let filled = fluid_bucket(CREOSOTE_BUCKET, "arcforge:creosote", 400);
        host.held = Some(filled.clone());
        assert_eq!(
            use_fluid_tank_bucket(&mut host, [0, 64, 0], Direction::Up, &filled, Some(7), 0),
            Flow::Cancel
        );
        assert_eq!(host.tank, Some(("arcforge:creosote".into(), 400)));
        assert_eq!(item_fluid(host.held.as_ref().unwrap()), None);

        host.inventory.insert(LATEX_BUCKET.to_owned(), 1);
        host.tank = Some(("arcforge:latex".into(), 375));
        let empty = test_bucket(LATEX_BUCKET, 1);
        host.held = Some(empty.clone());
        assert_eq!(
            use_fluid_tank_bucket(&mut host, [0, 64, 0], Direction::North, &empty, Some(7), 0),
            Flow::Cancel
        );
        assert_eq!(host.tank, None);
        assert_eq!(
            item_fluid(host.held.as_ref().unwrap()),
            Some(("arcforge:latex".into(), 375))
        );
    }

    #[test]
    fn portable_canisters_transfer_at_tier_rate_and_keep_fluid_state() {
        let mut host = fluid_tank_host(1_024_000, WROUGHT_CANISTER);
        host.tank = Some(("arcforge:creosote".into(), 8_000));
        let empty = test_bucket(WROUGHT_CANISTER, 1);
        assert_eq!(
            use_fluid_tank_bucket(&mut host, [0, 64, 0], Direction::Up, &empty, Some(7), 0),
            Flow::Cancel
        );
        assert_eq!(host.tank, Some(("arcforge:creosote".into(), 7_000)));
        assert_eq!(
            item_fluid(host.held.as_ref().unwrap()),
            Some(("arcforge:creosote".into(), 1_000))
        );

        host.inventory.insert(TEMPERED_CANISTER.to_owned(), 1);
        host.tank = Some(("arcforge:latex".into(), 30_000));
        let partly_filled = fluid_bucket(TEMPERED_CANISTER, "arcforge:latex", 10_000);
        host.held = Some(partly_filled.clone());
        assert_eq!(
            use_fluid_tank_bucket(
                &mut host,
                [0, 64, 0],
                Direction::Up,
                &partly_filled,
                Some(7),
                0
            ),
            Flow::Cancel
        );
        assert_eq!(host.tank, Some(("arcforge:latex".into(), 34_000)));
        assert_eq!(
            item_fluid(host.held.as_ref().unwrap()),
            Some(("arcforge:latex".into(), 6_000))
        );
    }

    #[test]
    fn arcforge_fluid_container_refuses_mixing_and_preserves_custom_data() {
        let mut filled = fluid_bucket(LATEX_BUCKET, "arcforge:latex", 250);
        let custom = filled
            .extra
            .as_deref()
            .unwrap()
            .other("minecraft:custom_data")
            .unwrap();
        let Tag::Compound(mut custom) = custom.clone() else {
            panic!("custom data is a compound")
        };
        custom.insert("keep_me".into(), Tag::Int(42));
        filled
            .extra
            .as_mut()
            .unwrap()
            .set_other("minecraft:custom_data", Some(Tag::Compound(custom)));
        let mut host = fluid_tank_host(16_000, LATEX_BUCKET);
        host.held = Some(filled.clone());
        host.tank = Some(("minecraft:water".into(), 500));
        assert_eq!(
            use_fluid_tank_bucket(&mut host, [0, 64, 0], Direction::Up, &filled, Some(7), 0),
            Flow::Continue
        );
        assert_eq!(host.tank, Some(("minecraft:water".into(), 500)));
        assert_eq!(item_count(&host, LATEX_BUCKET), 1);
        let updated = item_with_fluid(&filled, "arcforge:latex", 100);
        let Tag::Compound(data) = updated
            .extra
            .as_deref()
            .unwrap()
            .other("minecraft:custom_data")
            .unwrap()
        else {
            panic!("custom data is a compound")
        };
        assert_eq!(data.get(&"keep_me".into()), Some(&Tag::Int(42)));
        assert_eq!(item_fluid(&updated), Some(("arcforge:latex".into(), 100)));

        let invalid = fluid_bucket(CREOSOTE_BUCKET, "arcforge:latex", 250);
        let mut empty_host = fluid_tank_host(16_000, CREOSOTE_BUCKET);
        assert_eq!(
            use_fluid_tank_bucket(
                &mut empty_host,
                [0, 64, 0],
                Direction::Up,
                &invalid,
                Some(7),
                0
            ),
            Flow::Continue
        );
        assert_eq!(empty_host.tank, None);
        assert_eq!(item_count(&empty_host, CREOSOTE_BUCKET), 1);
    }

    #[test]
    fn arcforge_fluid_container_restores_tank_when_selected_stack_changed() {
        let mut host = fluid_tank_host(16_000, LATEX_BUCKET);
        host.tank = Some(("arcforge:latex".into(), 375));
        let empty = test_bucket(LATEX_BUCKET, 1);
        host.held = Some(test_bucket("minecraft:stick", 1));
        assert_eq!(
            use_fluid_tank_bucket(&mut host, [0, 64, 0], Direction::Up, &empty, Some(7), 0),
            Flow::Continue
        );
        assert_eq!(host.tank, Some(("arcforge:latex".into(), 375)));
        assert_eq!(host.held.as_ref().unwrap().item.as_ref(), "minecraft:stick");
    }

    #[test]
    fn partial_bucket_insert_is_rolled_back_without_consuming_item() {
        let mut host = fluid_tank_host(400, "minecraft:water_bucket");
        let filled = test_bucket("minecraft:water_bucket", 1);
        assert_eq!(
            use_fluid_tank_bucket(&mut host, [0, 64, 0], Direction::Up, &filled, Some(7), 0,),
            Flow::Continue
        );
        assert_eq!(host.tank, None);
        assert_eq!(host.inventory.get("minecraft:water_bucket"), Some(&1));
        assert!(host.given.is_empty());
    }
}

fn use_fluid_tank_bucket(
    host: &mut dyn Host,
    pos: [i32; 3],
    face: Direction,
    held: &ItemStack,
    player: Option<u128>,
    selected_slot: i32,
) -> Flow {
    if canister_spec(&held.item).is_some() {
        if player.is_none() {
            return Flow::Continue;
        }
        return use_arcforge_fluid_container(host, pos, face, held, selected_slot);
    }
    let Some(player) = player else {
        return Flow::Continue;
    };
    let Some(entity) = host.mod_block_entity(pos) else {
        return Flow::Continue;
    };
    let Some(capacity) = fluid_tank_capacity(&entity.id) else {
        return Flow::Continue;
    };

    if matches!(held.item.as_ref(), CREOSOTE_BUCKET | LATEX_BUCKET) {
        return use_arcforge_fluid_container(host, pos, face, held, selected_slot);
    }

    if held.item.as_ref() == "minecraft:bucket" {
        let Some((fluid, amount)) = host.fluid_extract(host.dimension(), pos, face, 1000) else {
            return Flow::Continue;
        };
        let amount = amount.max(0);
        let Some(bucket) = bucket_for_fluid(&fluid).filter(|_| amount == 1000) else {
            if amount > 0 {
                host.fluid_insert(pos, face, &fluid, amount);
            }
            return Flow::Continue;
        };
        if host.take_items("minecraft:bucket", 1) == 0 {
            host.fluid_insert(pos, face, &fluid, amount);
            return Flow::Continue;
        }
        if host
            .give_stack(
                player,
                if bucket == CREOSOTE_BUCKET {
                    creosote_bucket(amount as u32)
                } else if bucket == LATEX_BUCKET {
                    latex_bucket(amount as u32)
                } else {
                    ItemStack {
                        item: Arc::from(bucket),
                        count: 1,
                        damage: 0,
                        extra: None,
                    }
                },
            )
            .is_some()
        {
            host.fluid_insert(pos, face, &fluid, amount);
            host.give_items("minecraft:bucket", 1);
            return Flow::Continue;
        }
        return Flow::Cancel;
    }

    let Some(fluid) = fluid_bucket_contents(&held.item) else {
        return Flow::Continue;
    };
    if matches!(held.item.as_ref(), CREOSOTE_BUCKET | LATEX_BUCKET)
        && item_fluid(held).is_none_or(|(stored, amount)| stored != fluid || amount < 1000)
    {
        return Flow::Continue;
    }
    // Buckets move as an all-or-nothing item exchange. Check the whole bucket
    // fits before changing either side; the Geothermal Plant has no drainable
    // output face that could roll a partial insertion back through Horizon.
    if !crate::fluid::can_insert(&entity.nbt, 0, fluid, 1000, capacity) {
        return Flow::Continue;
    }
    if entity.id == GEOTHERMAL_PLANT {
        if fluid != "minecraft:lava" {
            return Flow::Continue;
        }
        let empty_bucket = ItemStack {
            item: Arc::from("minecraft:bucket"),
            count: 1,
            damage: 0,
            extra: None,
        };
        let replaced_in_place = held.count == 1;
        if replaced_in_place {
            if !replace_carried_stack(host, selected_slot, held, empty_bucket.clone()) {
                return Flow::Continue;
            }
        } else {
            if host.take_items(&held.item, 1) == 0 {
                return Flow::Continue;
            }
            if host.give_stack(player, empty_bucket.clone()).is_some() {
                let mut restore = held.clone();
                restore.count = 1;
                host.give_stack(player, restore);
                return Flow::Continue;
            }
        }

        let mut inserted = 0;
        let changed = host.change_mod_block_entity(pos, &mut |target| {
            if target.id != GEOTHERMAL_PLANT {
                return;
            }
            inserted = crate::fluid::insert(
                &mut target.nbt,
                0,
                "minecraft:lava",
                1000,
                GEOTHERMAL_LAVA_TANK_CAPACITY,
            );
            if inserted != 1000 {
                crate::fluid::extract(&mut target.nbt, 0, "minecraft:lava", inserted);
                inserted = 0;
            }
        });
        if changed && inserted == 1000 {
            host.block_entity_sync(pos);
            return Flow::Cancel;
        }

        if replaced_in_place {
            replace_carried_stack(host, selected_slot, &empty_bucket, held.clone());
        } else {
            host.take_items("minecraft:bucket", 1);
            let mut restore = held.clone();
            restore.count = 1;
            host.give_stack(player, restore);
        }
        return Flow::Continue;
    }

    let empty_bucket = ItemStack {
        item: Arc::from("minecraft:bucket"),
        count: 1,
        damage: 0,
        extra: None,
    };
    let remaining = host.fluid_insert(pos, face, fluid, 1000);
    let inserted = 1000 - remaining.clamp(0, 1000);
    if inserted != 1000 {
        if inserted > 0 {
            host.fluid_extract(host.dimension(), pos, face, inserted);
        }
        return Flow::Continue;
    }
    if host.take_items(&held.item, 1) == 0 {
        host.fluid_extract(host.dimension(), pos, face, 1000);
        return Flow::Continue;
    }
    if host.give_stack(player, empty_bucket).is_some() {
        host.fluid_extract(host.dimension(), pos, face, 1000);
        let mut returned_bucket = held.clone();
        returned_bucket.count = 1;
        host.give_stack(player, returned_bucket);
        return Flow::Continue;
    }
    Flow::Cancel
}

fn use_arcforge_fluid_container(
    host: &mut dyn Host,
    pos: [i32; 3],
    face: Direction,
    held: &ItemStack,
    selected_slot: i32,
) -> Flow {
    let (capacity, rate, expected) = match held.item.as_ref() {
        CREOSOTE_BUCKET => (1000, 1000, Some("arcforge:creosote")),
        LATEX_BUCKET => (1000, 1000, Some("arcforge:latex")),
        item => match canister_spec(item) {
            Some((capacity, rate)) => (capacity, rate, None),
            None => return Flow::Continue,
        },
    };
    let contents = item_fluid(held);
    if contents
        .as_ref()
        .is_some_and(|(_, amount)| *amount > capacity)
    {
        return Flow::Continue;
    }

    let (fluid, stored) = match contents {
        Some((fluid, amount)) if expected.is_none_or(|name| fluid == name) => (fluid, amount),
        Some(_) => return Flow::Continue,
        None => (expected.unwrap_or_default().to_owned(), 0),
    };
    if stored > 0 {
        let offered = stored.min(rate);
        let remaining = host
            .fluid_insert(pos, face, &fluid, offered as i32)
            .clamp(0, offered as i32);
        let inserted = offered - remaining as u32;
        if inserted == 0 {
            return Flow::Continue;
        }
        let updated = item_with_fluid(held, &fluid, stored - inserted);
        if !replace_carried_stack(host, selected_slot, held, updated) {
            host.fluid_extract(host.dimension(), pos, face, inserted as i32);
            return Flow::Continue;
        }
        return Flow::Cancel;
    }

    let Some((actual, extracted)) =
        host.fluid_extract(host.dimension(), pos, face, capacity.min(rate) as i32)
    else {
        return Flow::Continue;
    };
    let extracted = extracted.clamp(0, capacity.min(rate) as i32) as u32;
    if extracted == 0 {
        return Flow::Continue;
    }
    if expected.is_some_and(|name| actual != name) {
        host.fluid_insert(pos, face, &actual, extracted as i32);
        return Flow::Continue;
    }
    let updated = item_with_fluid(held, &actual, extracted);
    if !replace_carried_stack(host, selected_slot, held, updated) {
        host.fluid_insert(pos, face, &actual, extracted as i32);
        return Flow::Continue;
    }
    Flow::Cancel
}

fn use_gas_cartridge(
    host: &mut dyn Host,
    pos: [i32; 3],
    face: Direction,
    held: &ItemStack,
    selected_slot: i32,
) -> Flow {
    let Some((capacity, rate)) = pressurized_cylinder::cartridge_spec(&held.item) else {
        return Flow::Continue;
    };
    let contents = pressurized_cylinder::item_fluid(held);
    if contents
        .as_ref()
        .is_some_and(|(gas, amount)| *amount > capacity || !fluid_conduit::is_arcforge_gas(gas))
    {
        return Flow::Continue;
    }

    if let Some((gas, amount)) = contents {
        let offered = amount.min(rate);
        let remaining = host
            .fluid_insert(pos, face, &gas, offered as i32)
            .clamp(0, offered as i32);
        let inserted = offered - remaining as u32;
        if inserted == 0 {
            return Flow::Continue;
        }
        let updated = item_with_fluid(held, &gas, amount - inserted);
        if !replace_carried_stack(host, selected_slot, held, updated) {
            host.fluid_extract(host.dimension(), pos, face, inserted as i32);
            return Flow::Continue;
        }
        show_cylinder_status(host, pos);
        return Flow::Cancel;
    }

    let Some((gas, extracted)) =
        host.fluid_extract(host.dimension(), pos, face, capacity.min(rate) as i32)
    else {
        return Flow::Continue;
    };
    let extracted = extracted.clamp(0, capacity.min(rate) as i32) as u32;
    if extracted == 0 {
        return Flow::Continue;
    }
    if !fluid_conduit::is_arcforge_gas(&gas) {
        host.fluid_insert(pos, face, &gas, extracted as i32);
        return Flow::Continue;
    }
    let updated = item_with_fluid(held, &gas, extracted);
    if !replace_carried_stack(host, selected_slot, held, updated) {
        host.fluid_insert(pos, face, &gas, extracted as i32);
        return Flow::Continue;
    }
    show_cylinder_status(host, pos);
    Flow::Cancel
}

fn show_cylinder_status(host: &mut dyn Host, pos: [i32; 3]) {
    let Some(entity) = host.mod_block_entity(pos) else {
        return;
    };
    let Some(capacity) = pressurized_cylinder::cylinder_capacity(&entity.id) else {
        return;
    };
    let amount = fluid::amount(&entity.nbt, 0).min(capacity);
    let name = fluid::name(&entity.nbt, 0);
    let level = ((amount as u64 * 5) / capacity as u64).min(4);
    let contents = if amount == 0 {
        "empty".to_owned()
    } else {
        name.unwrap_or_else(|| "unknown gas".into())
    };
    let line = format!("Cylinder: {contents}, {amount}/{capacity} mB (gauge {level}/4)");
    if let Some(player) = host.acting_player() {
        host.action_bar(player.uuid, Text::literal(line));
    }
}

fn show_energy_cell_status(host: &mut dyn Host, pos: [i32; 3]) {
    let Some(entity) = host.mod_block_entity(pos) else {
        return;
    };
    let Some((capacity, _)) = energy_cell_spec(&entity.id) else {
        return;
    };
    let amount = (get_i32(&entity.nbt, "energy").max(0) as u64).min(capacity);
    let level = energy_cell_charge_level(amount, capacity);
    if let Some(player) = host.acting_player() {
        host.action_bar(
            player.uuid,
            Text::literal(format!(
                "Energy Cell: {amount}/{capacity} FE (gauge {level}/4)"
            )),
        );
    }
}

fn show_crushing_array_status(host: &mut dyn Host, pos: [i32; 3]) {
    let Some(entity) = host.mod_block_entity(pos) else {
        return;
    };
    if entity.id != ARC_CRUSHING_ARRAY_CASING
        || host.block_property(pos, "part").as_deref() != Some("center")
    {
        return;
    }
    let speed_upgrades =
        get_i32(&entity.nbt, "SpeedUpgrades").clamp(0, SPEED_MAX_UPGRADES as i32) as u32;
    let lanes = (0..3)
        .map(|lane| {
            let input = entity.items.get(lane).and_then(Option::as_ref);
            let Some(recipe) = input.and_then(|input| crushing_recipe(host, input)) else {
                return format!("L{} idle", lane + 1);
            };
            let cook = get_i32(&entity.nbt, &format!("Lane{lane}CookTime")).max(0) as u32;
            let total = speed_upgrade_time(recipe.ticks, speed_upgrades);
            format!("L{} {}/{}t", lane + 1, cook.min(total), total)
        })
        .collect::<Vec<_>>()
        .join(" ");
    let energy = (get_i32(&entity.nbt, "energy").max(0) as u64).min(100_000);
    if let Some(player) = host.acting_player() {
        host.action_bar(
            player.uuid,
            Text::literal(format!("Crushing Array: {energy}/100000 FE | {lanes}")),
        );
    }
}

fn show_processor_status(host: &mut dyn Host, pos: [i32; 3]) {
    let line = {
        let Some(entity) = host.mod_block_entity(pos) else {
            return;
        };
        let speed_upgrades =
            get_i32(&entity.nbt, "SpeedUpgrades").clamp(0, SPEED_MAX_UPGRADES as i32) as u32;
        let base_ticks = match entity.id.as_ref() {
            ARC_CRUSHER => entity
                .items
                .first()
                .and_then(Option::as_ref)
                .and_then(|input| crushing_recipe(host, input))
                .map(|recipe| recipe.ticks),
            INDUCTION_FURNACE => entity
                .items
                .first()
                .and_then(Option::as_ref)
                .and_then(|input| {
                    induction_recipe(host, input)
                        .map(|recipe| recipe.ticks)
                        .or_else(|| smelting_recipe(input.item.as_ref()).map(|_| 100))
                }),
            METAL_PRESS => {
                let die = entity.items.get(1).and_then(Option::as_ref);
                let input = entity.items.first().and_then(Option::as_ref);
                die.zip(input)
                    .and_then(|(die, input)| press_recipe(host, die.item.as_ref(), input))
                    .map(|recipe| recipe.ticks)
            }
            SIFTER => {
                let recipe = entity
                    .items
                    .first()
                    .and_then(Option::as_ref)
                    .and_then(|input| sifter_recipe(host, input));
                recipe.map(|recipe| {
                    let mesh_speed = entity
                        .items
                        .get(1)
                        .and_then(Option::as_ref)
                        .and_then(|mesh| sifter_mesh_tier(mesh.item.as_ref()))
                        .map_or(1.0, |mesh| f64::from(mesh.speed_multiplier));
                    let mesh_ticks = (f64::from(recipe.ticks) / mesh_speed).round().max(1.0) as u32;
                    mesh_ticks
                })
            }
            _ => return,
        };
        let name = match entity.id.as_ref() {
            ARC_CRUSHER => "Arc Crusher",
            INDUCTION_FURNACE => "Induction Furnace",
            METAL_PRESS => "Metal Press",
            SIFTER => "Sifter",
            _ => return,
        };
        let capacity = 20_000;
        let energy = (get_i32(&entity.nbt, "energy").max(0) as u64).min(capacity);
        let cook = get_i32(&entity.nbt, "CookTime").max(0) as u32;
        let state = base_ticks
            .map(|ticks| speed_upgrade_time(ticks, speed_upgrades))
            .map_or_else(
                || "idle".to_owned(),
                |total| format!("{}/{}t", cook.min(total), total),
            );
        format!("{name}: {energy}/{capacity} FE | {state}")
    };
    if let Some(player) = host.acting_player() {
        host.action_bar(player.uuid, Text::literal(line));
    }
}

fn show_arcforge_furnace_status(host: &mut dyn Host, pos: [i32; 3]) {
    let Some(mut entity) = host.mod_block_entity(pos) else {
        return;
    };
    if entity.id != FURNACE_PORT {
        return;
    }
    let mut formed = arcforge_furnace_structure_valid(host, pos);
    if !formed {
        if let Some(facing) = host
            .block_property(pos, "facing")
            .and_then(|facing| next_machine_facing(&facing))
        {
            host.set_block_property(pos, "facing", facing);
            formed = arcforge_furnace_structure_valid(host, pos);
            host.change_mod_block_entity(pos, &mut |current| {
                if current.id == FURNACE_PORT {
                    set_i32(&mut current.nbt, "FurnaceFormed", i32::from(formed));
                    set_i32(&mut current.nbt, "FurnaceStructureCheck", 20);
                }
            });
            host.block_entity_sync(pos);
            entity = host.mod_block_entity(pos).unwrap_or(entity);
        }
    }
    let temperature = get_i32(&entity.nbt, "TemperatureC").max(20);
    let burn = get_i32(&entity.nbt, "BurnTime").max(0);
    let burn_duration = get_i32(&entity.nbt, "BurnDuration").max(800);
    let cook = get_i32(&entity.nbt, "CookTime").max(0) as u32;
    let state = if !formed {
        "incomplete".to_owned()
    } else if let Some(recipe) = arcforge_furnace_recipe(host, &entity) {
        format!(
            "formed, {}/{}t, needs {}°C, {} coke",
            cook.min(recipe.ticks),
            recipe.ticks,
            recipe.min_temperature_c,
            recipe.coke
        )
    } else {
        "formed, idle (check metal/additives/output)".to_owned()
    };
    let fuel = if burn == 0 {
        "fuel spent".to_owned()
    } else {
        format!("fuel {burn}/{burn_duration}t")
    };
    if let Some(player) = host.acting_player() {
        host.action_bar(
            player.uuid,
            Text::literal(format!(
                "Arcforge Furnace: {temperature}°C | {fuel} | {state}"
            )),
        );
    }
}

fn replace_carried_stack(
    host: &mut dyn Host,
    selected_slot: i32,
    expected: &ItemStack,
    replacement: ItemStack,
) -> bool {
    let Some(stack) = host.carried_stack(selected_slot) else {
        return false;
    };
    if stack != expected {
        return false;
    }
    *stack = replacement;
    true
}

fn creosote_bucket(amount: u32) -> ItemStack {
    fluid_bucket(CREOSOTE_BUCKET, "arcforge:creosote", amount)
}

fn use_fuel_burner_bucket(host: &mut dyn Host, pos: [i32; 3], held: &ItemStack) -> Flow {
    let Some(player) = host.acting_player().map(|player| player.uuid) else {
        return Flow::Continue;
    };
    let Some((fluid_name, amount)) = item_fluid(held) else {
        return Flow::Continue;
    };
    let Some(burner) = host.mod_block_entity(pos) else {
        return Flow::Continue;
    };
    if burner.id != FUEL_BURNER
        || fluid_name != "arcforge:creosote"
        || amount < 1000
        || !fluid::can_insert(
            &burner.nbt,
            0,
            "arcforge:creosote",
            1000,
            FUEL_BURNER_TANK_CAPACITY,
        )
        || host.take_items(CREOSOTE_BUCKET, 1) == 0
    {
        return Flow::Continue;
    }
    let mut inserted = 0;
    let changed = host.change_mod_block_entity(pos, &mut |entity| {
        if entity.id == FUEL_BURNER {
            inserted = fluid::insert(
                &mut entity.nbt,
                0,
                "arcforge:creosote",
                1000,
                FUEL_BURNER_TANK_CAPACITY,
            );
        }
    });
    if !changed || inserted != 1000 {
        host.give_stack(player, held.clone());
        return Flow::Continue;
    }
    let empty = ItemStack {
        item: Arc::from("minecraft:bucket"),
        count: 1,
        damage: 0,
        extra: None,
    };
    if host.give_stack(player, empty).is_some() {
        host.change_mod_block_entity(pos, &mut |entity| {
            if entity.id == FUEL_BURNER {
                fluid::extract(&mut entity.nbt, 0, "arcforge:creosote", inserted);
            }
        });
        host.give_stack(player, held.clone());
        Flow::Continue
    } else {
        Flow::Cancel
    }
}

fn latex_bucket(amount: u32) -> ItemStack {
    fluid_bucket(LATEX_BUCKET, "arcforge:latex", amount)
}

fn fluid_bucket(item: &str, fluid: &str, amount: u32) -> ItemStack {
    let mut stack = ItemStack {
        item: Arc::from(item),
        count: 1,
        damage: 0,
        extra: Some(Box::default()),
    };
    stack.extra.as_mut().unwrap().set_other(
        "minecraft:custom_data",
        Some(Tag::compound([(
            "horizon_fluid",
            Tag::compound([
                ("Fluid", Tag::String(fluid.into())),
                ("Amount", Tag::Int(amount.min(i32::MAX as u32) as i32)),
            ]),
        )])),
    );
    stack
}

fn use_resin_tap_bucket(
    host: &mut dyn Host,
    pos: [i32; 3],
    held: &ItemStack,
    player: Option<u128>,
) -> Flow {
    let Some(player) = player else {
        return Flow::Continue;
    };
    if held.item.as_ref() == "minecraft:bucket" {
        let Some(tap) = host.mod_block_entity(pos) else {
            return Flow::Continue;
        };
        if tap.id != RESIN_TAP
            || fluid::name(&tap.nbt, 0).as_deref() != Some("arcforge:latex")
            || fluid::amount(&tap.nbt, 0) < 1000
        {
            return Flow::Continue;
        }
        if host.take_items("minecraft:bucket", 1) == 0 {
            return Flow::Continue;
        }
        let mut extracted = 0;
        let changed = host.change_mod_block_entity(pos, &mut |entity| {
            if entity.id == RESIN_TAP {
                extracted = fluid::extract(&mut entity.nbt, 0, "arcforge:latex", 1000);
            }
        });
        if !changed || extracted != 1000 {
            host.give_items("minecraft:bucket", 1);
            return Flow::Continue;
        }
        if host.give_stack(player, latex_bucket(extracted)).is_some() {
            host.change_mod_block_entity(pos, &mut |entity| {
                if entity.id == RESIN_TAP {
                    fluid::insert(&mut entity.nbt, 0, "arcforge:latex", extracted, 1000);
                }
            });
            host.give_items("minecraft:bucket", 1);
            return Flow::Continue;
        }
        return Flow::Cancel;
    }

    let Some((fluid_name, amount)) = item_fluid(held) else {
        return Flow::Continue;
    };
    if fluid_name != "arcforge:latex"
        || amount < 1000
        || host
            .mod_block_entity(pos)
            .is_none_or(|entity| !fluid::can_insert(&entity.nbt, 0, &fluid_name, 1000, 1000))
        || host.take_items(LATEX_BUCKET, 1) == 0
    {
        return Flow::Continue;
    }
    let mut inserted = 0;
    let changed = host.change_mod_block_entity(pos, &mut |entity| {
        if entity.id == RESIN_TAP {
            inserted = fluid::insert(&mut entity.nbt, 0, &fluid_name, 1000, 1000);
        }
    });
    if !changed || inserted != 1000 {
        host.give_stack(player, held.clone());
        return Flow::Continue;
    }
    let empty_bucket = ItemStack {
        item: Arc::from("minecraft:bucket"),
        count: 1,
        damage: 0,
        extra: None,
    };
    if host.give_stack(player, empty_bucket).is_some() {
        host.change_mod_block_entity(pos, &mut |entity| {
            if entity.id == RESIN_TAP {
                fluid::extract(&mut entity.nbt, 0, "arcforge:latex", inserted);
            }
        });
        host.give_stack(player, held.clone());
        return Flow::Continue;
    }
    Flow::Cancel
}

fn bucket_output_fits(slot: Option<&ItemStack>, result: &ItemStack) -> bool {
    slot.is_none_or(|existing| {
        let max_stack = if result.item.as_ref() == CREOSOTE_BUCKET {
            1
        } else {
            16
        };
        existing.item == result.item
            && existing.extra == result.extra
            && existing.count.saturating_add(result.count) <= max_stack
    })
}

fn insert_bucket_output(slot: &mut Option<ItemStack>, result: ItemStack) {
    if let Some(existing) = slot {
        existing.count += result.count;
    } else {
        *slot = Some(result);
    }
}

fn tick_infuser_buckets(entity: &mut ModBlockEntity) {
    let Some(bucket) = entity.items.first().and_then(Option::as_ref) else {
        return;
    };
    if bucket.item.as_ref() == "minecraft:bucket" {
        if fluid::name(&entity.nbt, 0).as_deref() != Some("arcforge:creosote")
            || fluid::amount(&entity.nbt, 0) < 1000
        {
            return;
        }
        let filled = creosote_bucket(1000);
        if !bucket_output_fits(entity.items.get(1).and_then(Option::as_ref), &filled) {
            return;
        }
        if consume_one(&mut entity.items[0])
            && fluid::extract(&mut entity.nbt, 0, "arcforge:creosote", 1000) == 1000
        {
            insert_bucket_output(&mut entity.items[1], filled);
        }
    } else if bucket.item.as_ref() == CREOSOTE_BUCKET {
        let Some((fluid_name, amount)) = item_fluid(bucket) else {
            return;
        };
        if fluid_name != "arcforge:creosote"
            || amount < 1000
            || !fluid::can_insert(&entity.nbt, 0, &fluid_name, 1000, 8000)
            || !bucket_output_fits(
                entity.items.get(1).and_then(Option::as_ref),
                &ItemStack {
                    item: Arc::from("minecraft:bucket"),
                    count: 1,
                    damage: 0,
                    extra: None,
                },
            )
        {
            return;
        }
        if consume_one(&mut entity.items[0]) {
            insert_bucket_output(
                &mut entity.items[1],
                ItemStack {
                    item: Arc::from("minecraft:bucket"),
                    count: 1,
                    damage: 0,
                    extra: None,
                },
            );
            fluid::insert(&mut entity.nbt, 0, &fluid_name, 1000, 8000);
        }
    }
}

fn sync_infuser_screen_data(entity: &mut ModBlockEntity, ticks: u32) {
    entity.data.resize(4, 0);
    entity.data[0] = fluid::amount(&entity.nbt, 0).min(8_000) as i32;
    entity.data[1] = get_i32(&entity.nbt, "energy").max(0);
    entity.data[2] = get_i32(&entity.nbt, "CookTime")
        .max(0)
        .min(ticks.min(i32::MAX as u32) as i32);
    entity.data[3] = ticks.min(i32::MAX as u32) as i32;
}

fn tick_infuser(host: &dyn Host, entity: &mut ModBlockEntity) {
    if entity.items.len() != 5 {
        return;
    }
    tick_infuser_buckets(entity);
    sync_infuser_screen_data(entity, 0);
    let Some(input) = entity.items[2].as_ref() else {
        set_i32(&mut entity.nbt, "CookTime", 0);
        sync_infuser_screen_data(entity, 0);
        return;
    };
    let additive = entity.items[4].as_ref();
    let fluid_name = fluid::name(&entity.nbt, 0);
    let fluid_amount = fluid::amount(&entity.nbt, 0);
    let Some(candidate) =
        infuser_candidate(host, input, fluid_name.as_deref(), fluid_amount, additive)
    else {
        set_i32(&mut entity.nbt, "CookTime", 0);
        sync_infuser_screen_data(entity, 0);
        return;
    };
    let recipe = candidate.recipe;
    let speed_upgrades =
        get_i32(&entity.nbt, "SpeedUpgrades").clamp(0, SPEED_MAX_UPGRADES as i32) as u32;
    let speed = speed_upgrade_multiplier(speed_upgrades);
    let ticks = speed_upgrade_time(recipe.ticks, speed_upgrades);
    if !can_add(
        entity.items[3].as_ref(),
        &recipe.output.item,
        recipe.output.count,
    ) {
        sync_infuser_screen_data(entity, ticks);
        return;
    }
    if recipe.additive_count.is_some() && !candidate.additive_ready {
        sync_infuser_screen_data(entity, ticks);
        return;
    }
    if let Some((name, amount)) = &recipe.fluid {
        if fluid_name.as_deref() != Some(name.as_str()) || fluid_amount < *amount {
            sync_infuser_screen_data(entity, ticks);
            return;
        }
    }
    let mut energy = energy::EnergyStore::new(20_000, get_i32(&entity.nbt, "energy").max(0) as u64);
    let energy_cost = scaled_upgrade_cost(
        20,
        speed,
        get_i32(&entity.nbt, "EnergyUpgrades").clamp(0, ENERGY_MAX_UPGRADES as i32) as u32,
    );
    if energy.extract(energy_cost, energy_cost) != energy_cost {
        sync_infuser_screen_data(entity, ticks);
        return;
    }
    let cook = get_i32(&entity.nbt, "CookTime").max(0) + 1;
    if cook >= ticks as i32 {
        if let Some(count) = recipe.additive_count {
            if let Some(additive) = entity.items[4].as_mut() {
                additive.count -= count;
                if additive.count == 0 {
                    entity.items[4] = None;
                }
            }
        }
        if let Some((name, amount)) = &recipe.fluid {
            fluid::extract(&mut entity.nbt, 0, name, *amount);
        }
        consume_one(&mut entity.items[2]);
        add_items(
            &mut entity.items[3],
            &recipe.output.item,
            recipe.output.count,
        );
        set_i32(&mut entity.nbt, "CookTime", 0);
    } else {
        set_i32(&mut entity.nbt, "CookTime", cook);
    }
    set_i32(
        &mut entity.nbt,
        "energy",
        energy.stored_fe().min(i32::MAX as u64) as i32,
    );
    sync_infuser_screen_data(entity, ticks);
}

fn firebox_fuel_ticks(item: &str) -> Option<u32> {
    Some(match item {
        "minecraft:coal" | "minecraft:charcoal" | "arcforge:bio_coal" => 1600,
        "minecraft:coal_block" | "arcforge:bio_coal_block" => 16_000,
        _ => return None,
    })
}

fn firebox_burn_duration(item: &str) -> Option<u32> {
    firebox_fuel_ticks(item).map(|ticks| ticks / FIREBOX_BURN_SPEED)
}

fn firebox_burn_duration_with_speed(item: &str, speed: f64) -> Option<u32> {
    firebox_fuel_ticks(item).map(|ticks| {
        (f64::from(ticks) / (f64::from(FIREBOX_BURN_SPEED) * speed))
            .round()
            .max(1.0) as u32
    })
}

fn tick_fuel_burner(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    entity.items.resize(2, None);
    tick_fuel_burner_buckets(entity);

    let mut heat = heat::HeatStore::new(
        FUEL_BURNER_HEAT_CAPACITY,
        FUEL_BURNER_MAX_TEMPERATURE,
        get_i32(&entity.nbt, "HeatHU").max(0) as u64,
    );
    let speed_upgrades =
        get_i32(&entity.nbt, "SpeedUpgrades").clamp(0, SPEED_MAX_UPGRADES as i32) as u32;
    let speed = speed_upgrade_multiplier(speed_upgrades);
    let burn_milli = (FUEL_BURNER_CREOSOTE_MILLI_MB_PER_TICK as f64 * speed).round() as u32;
    let mut fuel_taken_milli = get_i32(&entity.nbt, "FuelTakenMilli").clamp(0, 16_000) as u32;
    let at_fuel_temperature = heat_at_temperature(
        FUEL_BURNER_HEAT_CAPACITY,
        FUEL_BURNER_MAX_TEMPERATURE,
        FUEL_BURNER_CREOSOTE_TEMPERATURE,
    );
    let has_creosote = fluid::name(&entity.nbt, 0).as_deref() == Some("arcforge:creosote");
    let mut heat_produced = 0;
    if (has_creosote || fuel_taken_milli > 0)
        && heat.stored_hu() < at_fuel_temperature
        && fuel_taken_milli < burn_milli
    {
        let need_milli = burn_milli - fuel_taken_milli;
        let need_mb = need_milli.div_ceil(1000);
        let taken = fluid::extract(&mut entity.nbt, 0, "arcforge:creosote", need_mb);
        fuel_taken_milli = fuel_taken_milli.saturating_add(taken.saturating_mul(1000));
    }
    if heat.stored_hu() < at_fuel_temperature && fuel_taken_milli >= burn_milli {
        let burned_milli = burn_milli;
        fuel_taken_milli -= burned_milli;
        let base_generated =
            (u64::from(burned_milli) * u64::from(FUEL_BURNER_CREOSOTE_HU_PER_MB) + 500) / 1000;
        let heat_upgrades =
            get_i32(&entity.nbt, "HeatUpgrades").clamp(0, HEAT_MAX_UPGRADES as i32) as u32;
        let generated =
            (base_generated as f64 * heat_upgrade_multiplier(heat_upgrades)).round() as u64;
        let room = at_fuel_temperature.saturating_sub(heat.stored_hu());
        heat_produced = heat.add_hu(generated.min(room));
    }
    heat.set_source_temperature(if heat_produced > 0 {
        FUEL_BURNER_CREOSOTE_TEMPERATURE
    } else {
        20
    });
    set_i32(&mut entity.nbt, "FuelTakenMilli", fuel_taken_milli as i32);
    set_i32(
        &mut entity.nbt,
        "HeatHU",
        heat.stored_hu().min(i32::MAX as u64) as i32,
    );
    set_i32(&mut entity.nbt, "TemperatureC", heat.temperature_c() as i32);
    entity.data.resize(4, 0);
    entity.data[0] = heat.stored_hu().min(i32::MAX as u64) as i32;
    entity.data[1] = heat.temperature_c() as i32;
    entity.data[2] = fluid::amount(&entity.nbt, 0).min(i32::MAX as u32) as i32;
    entity.data[3] = heat_produced.min(i32::MAX as u64) as i32;
    host.set_block_property(pos, "lit", if heat_produced > 0 { "true" } else { "false" });
    transfer_firebox_heat(host, pos, &mut heat);
    set_i32(
        &mut entity.nbt,
        "HeatHU",
        heat.stored_hu().min(i32::MAX as u64) as i32,
    );
    set_i32(&mut entity.nbt, "TemperatureC", heat.temperature_c() as i32);
}

fn tick_fuel_burner_buckets(entity: &mut ModBlockEntity) {
    if entity.items.len() < 2 {
        return;
    }
    let Some(bucket) = entity.items[0].as_ref() else {
        return;
    };
    let Some((fluid_name, amount)) = item_fluid(bucket) else {
        return;
    };
    if bucket.item.as_ref() != CREOSOTE_BUCKET
        || fluid_name != "arcforge:creosote"
        || amount < 1000
        || !fluid::can_insert(
            &entity.nbt,
            0,
            "arcforge:creosote",
            1000,
            FUEL_BURNER_TANK_CAPACITY,
        )
        || !can_add(entity.items[1].as_ref(), "minecraft:bucket", 1)
    {
        return;
    }
    if consume_one(&mut entity.items[0])
        && fluid::insert(
            &mut entity.nbt,
            0,
            "arcforge:creosote",
            1000,
            FUEL_BURNER_TANK_CAPACITY,
        ) == 1000
    {
        add_items(&mut entity.items[1], "minecraft:bucket", 1);
    }
}

fn heat_at_temperature(capacity: u64, max_temperature: u32, temperature: u32) -> u64 {
    if max_temperature <= 20 {
        return capacity;
    }
    capacity.saturating_mul(u64::from(temperature.saturating_sub(20)))
        / u64::from(max_temperature - 20)
}

fn tick_firebox(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    entity.items.resize(2, None);
    let mut heat = heat::HeatStore::new(40_000, 1100, get_i32(&entity.nbt, "HeatHU").max(0) as u64);
    let mut burn = get_i32(&entity.nbt, "BurnTime").max(0) as u32;
    let speed_upgrades =
        get_i32(&entity.nbt, "SpeedUpgrades").clamp(0, SPEED_MAX_UPGRADES as i32) as u32;
    let speed = speed_upgrade_multiplier(speed_upgrades);
    if burn == 0 && heat.stored_hu() < 40_000 {
        let fuel = entity.items[0].as_ref().and_then(|fuel| {
            firebox_burn_duration_with_speed(fuel.item.as_ref(), speed)
                .map(|ticks| (fuel.item.to_string(), ticks))
        });
        if let Some((fuel_item, ticks)) = fuel {
            if consume_one(&mut entity.items[0]) {
                burn = ticks;
                set_i32(&mut entity.nbt, "BurnDuration", burn as i32);
                set_string(&mut entity.nbt, "BurningItem", &fuel_item);
            }
        }
    }
    let heat_upgrades =
        get_i32(&entity.nbt, "HeatUpgrades").clamp(0, HEAT_MAX_UPGRADES as i32) as u32;
    let produced = if burn > 0 {
        heat.add_hu((80.0 * speed * heat_upgrade_multiplier(heat_upgrades)).round() as u64)
    } else {
        0
    };
    heat.set_source_temperature(if produced > 0 { 1100 } else { 20 });
    if produced > 0 {
        burn -= 1;
        if burn == 0 {
            if let Some(burning_item) = get_string(&entity.nbt, "BurningItem") {
                try_leave_wood_ash(host, &burning_item, &mut entity.items[1]);
            }
            entity.nbt.remove(&"BurningItem".into());
        }
    }
    set_i32(
        &mut entity.nbt,
        "BurnTime",
        burn.min(i32::MAX as u32) as i32,
    );
    set_i32(
        &mut entity.nbt,
        "HeatHU",
        heat.stored_hu().min(i32::MAX as u64) as i32,
    );
    set_i32(&mut entity.nbt, "TemperatureC", heat.temperature_c() as i32);
    entity.data.resize(5, 0);
    entity.data[0] = heat.stored_hu().min(i32::MAX as u64) as i32;
    entity.data[1] = 40_000;
    entity.data[2] = heat.temperature_c() as i32;
    entity.data[3] = burn.min(i32::MAX as u32) as i32;
    entity.data[4] = get_i32(&entity.nbt, "BurnDuration");
    let lit = burn > 0;
    host.set_block_property(pos, "lit", if lit { "true" } else { "false" });
    transfer_firebox_heat(host, pos, &mut heat);
    set_i32(
        &mut entity.nbt,
        "HeatHU",
        heat.stored_hu().min(i32::MAX as u64) as i32,
    );
    set_i32(&mut entity.nbt, "TemperatureC", heat.temperature_c() as i32);
}

fn tick_geothermal_plant(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    entity.items.resize(2, None);
    tick_geothermal_lava_bucket(entity);

    let scan_timer = get_i32(&entity.nbt, "SurroundingsScan").saturating_sub(1);
    let (lava_sources, magma_blocks) = if scan_timer <= 0 {
        let counts = geothermal_surroundings(host, pos);
        set_i32(
            &mut entity.nbt,
            "SurroundingsScan",
            GEOTHERMAL_SCAN_INTERVAL_TICKS,
        );
        set_i32(&mut entity.nbt, "LavaSources", counts.0 as i32);
        set_i32(&mut entity.nbt, "MagmaBlocks", counts.1 as i32);
        counts
    } else {
        set_i32(&mut entity.nbt, "SurroundingsScan", scan_timer);
        (
            get_i32(&entity.nbt, "LavaSources").clamp(0, 6) as u32,
            get_i32(&entity.nbt, "MagmaBlocks").clamp(0, 6) as u32,
        )
    };

    let mut heat = heat::HeatStore::new(
        GEOTHERMAL_HEAT_CAPACITY,
        GEOTHERMAL_MAX_TEMPERATURE,
        get_i32(&entity.nbt, "HeatHU").max(0) as u64,
    );
    let speed_upgrades =
        get_i32(&entity.nbt, "SpeedUpgrades").clamp(0, SPEED_MAX_UPGRADES as i32) as u32;
    let speed = speed_upgrade_multiplier(speed_upgrades);
    let mut lava_burning_milli = match entity.nbt.get(&"LavaBurningMilli".into()) {
        Some(Tag::Int(value)) => (*value).max(0) as u32,
        _ => get_i32(&entity.nbt, "LavaBurning").max(0) as u32 * 1000,
    };
    let mut lava_burn_total = get_i32(&entity.nbt, "LavaBurnTotal").max(0) as u32;
    let mut heat_per_tick = 0;

    if heat.stored_hu() < GEOTHERMAL_HEAT_CAPACITY {
        if lava_burning_milli == 0 {
            lava_burning_milli = crate::fluid::extract(
                &mut entity.nbt,
                0,
                "minecraft:lava",
                GEOTHERMAL_LAVA_PER_BURN,
            ) * 1000;
            lava_burn_total = lava_burning_milli / 1000;
        }

        let passive = lava_sources
            .saturating_mul(GEOTHERMAL_LAVA_SOURCE_HEAT_HU_PER_TICK)
            .saturating_add(magma_blocks.saturating_mul(GEOTHERMAL_MAGMA_HEAT_HU_PER_TICK));
        let lava_heat = if lava_burning_milli > 0 {
            let drain_milli = ((speed * 1000.0).round() as u32).min(lava_burning_milli);
            lava_burning_milli -= drain_milli;
            (f64::from(GEOTHERMAL_LAVA_HEAT_HU_PER_MB) * f64::from(drain_milli) / 1000.0).round()
                as u32
        } else {
            0
        };
        let heat_upgrades =
            get_i32(&entity.nbt, "HeatUpgrades").clamp(0, HEAT_MAX_UPGRADES as i32) as u32;
        let upgraded_heat = (f64::from(passive.saturating_add(lava_heat))
            * heat_upgrade_multiplier(heat_upgrades))
        .round() as u64;
        heat_per_tick = heat.add_hu(upgraded_heat);
    }

    heat.set_source_temperature(if heat_per_tick > 0 {
        GEOTHERMAL_MAX_TEMPERATURE
    } else {
        20
    });
    set_i32(&mut entity.nbt, "HeatHU", heat.stored_hu() as i32);
    set_i32(&mut entity.nbt, "TemperatureC", heat.temperature_c() as i32);
    set_i32(
        &mut entity.nbt,
        "LavaBurningMilli",
        lava_burning_milli.min(i32::MAX as u32) as i32,
    );
    set_i32(
        &mut entity.nbt,
        "LavaBurning",
        div_ceil_u64(u64::from(lava_burning_milli), 1000).min(i32::MAX as u64) as i32,
    );
    set_i32(
        &mut entity.nbt,
        "LavaBurnTotal",
        lava_burn_total.min(i32::MAX as u32) as i32,
    );
    set_i32(
        &mut entity.nbt,
        "HeatPerTick",
        heat_per_tick.min(i32::MAX as u64) as i32,
    );
    entity.data.resize(8, 0);
    entity.data[0] = heat.stored_hu().min(i32::MAX as u64) as i32;
    entity.data[1] = GEOTHERMAL_HEAT_CAPACITY as i32;
    entity.data[2] = heat.temperature_c() as i32;
    entity.data[3] = div_ceil_u64(u64::from(lava_burning_milli), 1000).min(i32::MAX as u64) as i32;
    entity.data[4] = lava_burn_total.min(i32::MAX as u32) as i32;
    entity.data[5] = heat_per_tick.min(i32::MAX as u64) as i32;
    entity.data[6] = lava_sources as i32;
    entity.data[7] = magma_blocks as i32;
    host.set_block_property(pos, "lit", if heat_per_tick > 0 { "true" } else { "false" });
    transfer_firebox_heat(host, pos, &mut heat);
    set_i32(&mut entity.nbt, "HeatHU", heat.stored_hu() as i32);
    set_i32(&mut entity.nbt, "TemperatureC", heat.temperature_c() as i32);
}

fn tick_geothermal_lava_bucket(entity: &mut ModBlockEntity) {
    let Some(input) = entity.items[0].as_ref() else {
        return;
    };
    if input.item.as_ref() != "minecraft:lava_bucket" {
        return;
    }
    if entity.items[1]
        .as_ref()
        .is_some_and(|output| output.item.as_ref() != "minecraft:bucket" || output.count >= 16)
    {
        return;
    }
    if !crate::fluid::can_insert(
        &entity.nbt,
        0,
        "minecraft:lava",
        1000,
        GEOTHERMAL_LAVA_TANK_CAPACITY,
    ) || crate::fluid::insert(
        &mut entity.nbt,
        0,
        "minecraft:lava",
        1000,
        GEOTHERMAL_LAVA_TANK_CAPACITY,
    ) != 1000
    {
        return;
    }
    consume_one(&mut entity.items[0]);
    match entity.items[1].as_mut() {
        Some(output) => output.count += 1,
        None => add_item(&mut entity.items[1], "minecraft:bucket"),
    }
}

fn geothermal_surroundings(host: &dyn Host, pos: [i32; 3]) -> (u32, u32) {
    const NEIGHBORS: [[i32; 3]; 6] = [
        [0, 1, 0],
        [0, -1, 0],
        [0, 0, -1],
        [0, 0, 1],
        [-1, 0, 0],
        [1, 0, 0],
    ];
    let mut lava_sources = 0;
    let mut magma_blocks = 0;
    for offset in NEIGHBORS {
        let neighbor = [pos[0] + offset[0], pos[1] + offset[1], pos[2] + offset[2]];
        match host.block(neighbor).as_str() {
            "minecraft:lava" if host.block_property(neighbor, "level").as_deref() == Some("0") => {
                lava_sources += 1;
            }
            "minecraft:magma_block" => magma_blocks += 1,
            _ => {}
        }
    }
    (lava_sources, magma_blocks)
}

fn transfer_firebox_heat(host: &mut dyn Host, pos: [i32; 3], heat: &mut heat::HeatStore) {
    transfer_firebox_heat_rate(host, pos, heat, 100, 600);
}

fn transfer_firebox_heat_rate(
    host: &mut dyn Host,
    pos: [i32; 3],
    heat: &mut heat::HeatStore,
    per_target_rate: u64,
    total_rate: u64,
) {
    const NEIGHBORS: [[i32; 3]; 6] = [
        [0, 1, 0],
        [0, -1, 0],
        [0, 0, -1],
        [0, 0, 1],
        [-1, 0, 0],
        [1, 0, 0],
    ];
    let mut output_left = total_rate;
    for direction in NEIGHBORS {
        if heat.temperature_c() <= 20 || output_left == 0 {
            break;
        }
        let neighbor = [
            pos[0] + direction[0],
            pos[1] + direction[1],
            pos[2] + direction[2],
        ];
        let moved = transfer_supported_heat_to_neighbor(
            host,
            pos,
            neighbor,
            heat,
            per_target_rate.min(output_left),
        );
        output_left = output_left.saturating_sub(moved);
    }
}

fn transfer_solar_heat_ports(
    host: &mut dyn Host,
    tower: solar::Tower,
    controller_nbt: &horizon_client::mod_api::Compound,
    heat: &mut heat::HeatStore,
    heat_per_tick: u64,
) {
    const NEIGHBORS: [(&str, [i32; 3]); 6] = [
        ("up", [0, 1, 0]),
        ("down", [0, -1, 0]),
        ("north", [0, 0, -1]),
        ("south", [0, 0, 1]),
        ("west", [-1, 0, 0]),
        ("east", [1, 0, 0]),
    ];
    let rate = heat_per_tick.max(100);
    for port in tower.positions() {
        for (face, offset) in NEIGHBORS {
            if heat.temperature_c() <= 20 {
                return;
            }
            if !solar::port_enabled(controller_nbt, port, face) {
                continue;
            }
            let neighbor = [
                port[0] + offset[0],
                port[1] + offset[1],
                port[2] + offset[2],
            ];
            transfer_supported_heat_to_neighbor(host, port, neighbor, heat, rate);
        }
    }
}

fn transfer_supported_heat_to_neighbor(
    host: &mut dyn Host,
    source_pos: [i32; 3],
    neighbor: [i32; 3],
    heat: &mut heat::HeatStore,
    rate: u64,
) -> u64 {
    let Some(target) = host.mod_block_entity(neighbor) else {
        return 0;
    };
    if heat_cell::spec(&target.id).is_some() {
        return heat_cell::accept_heat(host, neighbor, heat, rate);
    }
    if thermal::spec(&target.id).is_some() {
        return thermal::accept_heat(host, neighbor, source_pos, heat, rate);
    }
    let (min_temperature, capacity, max_temperature) = match target.id.as_str() {
        FIBERIZER => (800, 10_000, 1100),
        VULCANIZER => (
            VULCANIZER_MIN_TEMPERATURE,
            VULCANIZER_HEAT_CAPACITY,
            VULCANIZER_MAX_TEMPERATURE,
        ),
        GRAIN_DRYER => (
            GRAIN_DRYER_MIN_TEMPERATURE,
            GRAIN_DRYER_HEAT_CAPACITY,
            GRAIN_DRYER_MAX_TEMPERATURE,
        ),
        THERMOELECTRIC_PLANT => (
            20,
            THERMOELECTRIC_HEAT_CAPACITY,
            THERMOELECTRIC_MAX_TEMPERATURE,
        ),
        _ => return 0,
    };
    if heat.temperature_c() <= min_temperature {
        return 0;
    }
    let mut moved = 0;
    host.change_mod_block_entity(neighbor, &mut |entity| {
        if entity.id != target.id {
            return;
        }
        let mut sink = heat::HeatStore::new(
            capacity,
            max_temperature,
            get_i32(&entity.nbt, "HeatHU").max(0) as u64,
        );
        moved = heat.transfer_to(&mut sink, rate);
        if moved > 0 {
            set_i32(
                &mut entity.nbt,
                "HeatHU",
                sink.stored_hu().min(i32::MAX as u64) as i32,
            );
            set_i32(&mut entity.nbt, "TemperatureC", sink.temperature_c() as i32);
        }
    });
    moved
}

fn thermoelectric_efficiency(temperature_c: u32) -> f64 {
    if temperature_c <= 100 {
        0.0
    } else if temperature_c <= 1100 {
        f64::from(temperature_c - 100) / 1000.0
    } else {
        (1.0 + 0.15 * f64::from(temperature_c - 1100) / 300.0).min(1.15)
    }
}

fn heat_upgrade_multiplier(upgrades: u32) -> f64 {
    1.0 + HEAT_OUTPUT_PER_UPGRADE * f64::from(upgrades.min(HEAT_MAX_UPGRADES))
}

fn speed_upgrade_multiplier(upgrades: u32) -> f64 {
    2.0_f64.powf(f64::from(upgrades.min(SPEED_MAX_UPGRADES)) / 2.0)
}

fn speed_upgrade_time(base_time: u32, upgrades: u32) -> u32 {
    (f64::from(base_time) / speed_upgrade_multiplier(upgrades))
        .round()
        .clamp(1.0, f64::from(u32::MAX)) as u32
}

fn scaled_upgrade_cost(base_cost: u64, rate_multiplier: f64, cost_upgrades: u32) -> u64 {
    if base_cost == 0 {
        return 0;
    }
    (base_cost as f64
        * rate_multiplier
        * 0.8_f64.powf(f64::from(cost_upgrades.min(ENERGY_MAX_UPGRADES))))
    .ceil()
    .clamp(1.0, u64::MAX as f64) as u64
}

fn use_energy_upgrade(host: &mut dyn Host, pos: [i32; 3], held: &ItemStack, player: u128) -> Flow {
    let Some(entity) = host.mod_block_entity(pos) else {
        return Flow::Continue;
    };
    if !supports_energy_upgrade(&entity.id) {
        return Flow::Continue;
    }
    let upgrades =
        get_i32(&entity.nbt, "EnergyUpgrades").clamp(0, ENERGY_MAX_UPGRADES as i32) as u32;
    if upgrades == ENERGY_MAX_UPGRADES {
        host.action_bar(
            player,
            Text::literal(format!(
                "Energy Upgrades: {upgrades}/{ENERGY_MAX_UPGRADES} (maximum)"
            )),
        );
        return Flow::Cancel;
    }
    if host.take_items(ENERGY_UPGRADE, 1) == 0 {
        return Flow::Continue;
    }
    let machine = entity.id.clone();
    let mut installed = false;
    let changed = host.change_mod_block_entity(pos, &mut |target| {
        if target.id != machine {
            return;
        }
        let current =
            get_i32(&target.nbt, "EnergyUpgrades").clamp(0, ENERGY_MAX_UPGRADES as i32) as u32;
        if current < ENERGY_MAX_UPGRADES {
            set_i32(&mut target.nbt, "EnergyUpgrades", (current + 1) as i32);
            installed = true;
        }
    });
    if !changed || !installed {
        let mut refund = held.clone();
        refund.count = 1;
        host.give_stack(player, refund);
        return Flow::Continue;
    }
    host.block_entity_sync(pos);
    let count = upgrades + 1;
    host.action_bar(
        player,
        Text::literal(if entity.id == COMBUSTION {
            format!(
                "Energy Upgrades: {count}/{ENERGY_MAX_UPGRADES} ({:.1}% increased output)",
                f64::from(count) * 12.5
            )
        } else {
            format!(
                "Energy Upgrades: {count}/{ENERGY_MAX_UPGRADES} ({:.1}% of base FE cost)",
                0.8_f64.powf(f64::from(count)) * 100.0
            )
        }),
    );
    Flow::Cancel
}

fn supports_energy_upgrade(machine: &str) -> bool {
    matches!(
        machine,
        COMBUSTION
            | ARC_CRUSHER
            | ARC_CRUSHING_ARRAY
            | INDUCTION_FURNACE
            | METAL_PRESS
            | INFUSER
            | FIBERIZER
            | SIFTER
    )
}

fn supports_speed_upgrade(machine: &str) -> bool {
    supports_energy_upgrade(machine)
        || matches!(
            machine,
            FIREBOX
                | GEOTHERMAL_PLANT
                | FUEL_BURNER
                | THERMOELECTRIC_PLANT
                | VULCANIZER
                | GRAIN_DRYER
        )
}

fn use_speed_upgrade(host: &mut dyn Host, pos: [i32; 3], held: &ItemStack, player: u128) -> Flow {
    let Some(entity) = host.mod_block_entity(pos) else {
        return Flow::Continue;
    };
    if !supports_speed_upgrade(&entity.id) {
        return Flow::Continue;
    }
    let upgrades = get_i32(&entity.nbt, "SpeedUpgrades").clamp(0, SPEED_MAX_UPGRADES as i32) as u32;
    if upgrades == SPEED_MAX_UPGRADES {
        host.action_bar(
            player,
            Text::literal(format!(
                "Speed Upgrades: {upgrades}/{SPEED_MAX_UPGRADES} (maximum)"
            )),
        );
        return Flow::Cancel;
    }
    if host.take_items(SPEED_UPGRADE, 1) == 0 {
        return Flow::Continue;
    }
    let machine = entity.id.clone();
    let mut installed = false;
    let changed = host.change_mod_block_entity(pos, &mut |target| {
        if target.id != machine {
            return;
        }
        let current =
            get_i32(&target.nbt, "SpeedUpgrades").clamp(0, SPEED_MAX_UPGRADES as i32) as u32;
        if current < SPEED_MAX_UPGRADES {
            set_i32(&mut target.nbt, "SpeedUpgrades", (current + 1) as i32);
            installed = true;
        }
    });
    if !changed || !installed {
        let mut refund = held.clone();
        refund.count = 1;
        host.give_stack(player, refund);
        return Flow::Continue;
    }
    host.block_entity_sync(pos);
    let count = upgrades + 1;
    host.action_bar(
        player,
        Text::literal(format!(
            "Speed Upgrades: {count}/{SPEED_MAX_UPGRADES} (×{:.2} speed)",
            speed_upgrade_multiplier(count)
        )),
    );
    Flow::Cancel
}

fn use_heat_upgrade(
    host: &mut dyn Host,
    pos: [i32; 3],
    held: &ItemStack,
    player: u128,
    machine: &str,
) -> Flow {
    let Some(entity) = host.mod_block_entity(pos) else {
        return Flow::Continue;
    };
    if entity.id != machine {
        return Flow::Continue;
    }
    let upgrades = get_i32(&entity.nbt, "HeatUpgrades").clamp(0, HEAT_MAX_UPGRADES as i32) as u32;
    if upgrades == HEAT_MAX_UPGRADES {
        host.action_bar(
            player,
            Text::literal(format!(
                "Heat Upgrades: {upgrades}/{HEAT_MAX_UPGRADES} (maximum)"
            )),
        );
        return Flow::Cancel;
    }
    if host.take_items(HEAT_UPGRADE, 1) == 0 {
        return Flow::Continue;
    }
    let mut installed = false;
    let changed = host.change_mod_block_entity(pos, &mut |target| {
        if target.id != machine {
            return;
        }
        let current =
            get_i32(&target.nbt, "HeatUpgrades").clamp(0, HEAT_MAX_UPGRADES as i32) as u32;
        if current < HEAT_MAX_UPGRADES {
            set_i32(&mut target.nbt, "HeatUpgrades", (current + 1) as i32);
            installed = true;
        }
    });
    if !changed || !installed {
        let mut refund = held.clone();
        refund.count = 1;
        host.give_stack(player, refund);
        return Flow::Continue;
    }
    host.block_entity_sync(pos);
    let count = upgrades + 1;
    let message = if machine == THERMOELECTRIC_PLANT {
        format!("Thermoelectric Heat Upgrades: {count}/{HEAT_MAX_UPGRADES}")
    } else if matches!(machine, FIBERIZER | VULCANIZER | GRAIN_DRYER) {
        format!(
            "Heat Upgrades: {count}/{HEAT_MAX_UPGRADES} ({:.1}% of base HU cost)",
            HEAT_EFFICIENCY_COST_FACTOR.powf(f64::from(count)) * 100.0
        )
    } else {
        format!(
            "Heat Upgrades: {count}/{HEAT_MAX_UPGRADES} ({:.1}% increased output)",
            (heat_upgrade_multiplier(count) - 1.0) * 100.0
        )
    };
    host.action_bar(player, Text::literal(message));
    Flow::Cancel
}

fn use_thermoelectric_upgrade(
    host: &mut dyn Host,
    pos: [i32; 3],
    held: &ItemStack,
    player: u128,
) -> Flow {
    let Some(entity) = host.mod_block_entity(pos) else {
        return Flow::Continue;
    };
    if entity.id != THERMOELECTRIC_PLANT {
        return Flow::Continue;
    }
    let upgrades = get_i32(&entity.nbt, "ThermoelectricUpgrades")
        .clamp(0, THERMOELECTRIC_MAX_UPGRADES as i32) as u32;
    if upgrades == THERMOELECTRIC_MAX_UPGRADES {
        host.action_bar(
            player,
            Text::literal(format!(
                "Thermoelectric Plant upgrades: {upgrades}/{THERMOELECTRIC_MAX_UPGRADES} (maximum)"
            )),
        );
        return Flow::Cancel;
    }
    if host.take_items(THERMOELECTRIC_UPGRADE, 1) == 0 {
        return Flow::Continue;
    }
    let mut installed = false;
    let changed = host.change_mod_block_entity(pos, &mut |target| {
        if target.id != THERMOELECTRIC_PLANT {
            return;
        }
        let current = get_i32(&target.nbt, "ThermoelectricUpgrades")
            .clamp(0, THERMOELECTRIC_MAX_UPGRADES as i32) as u32;
        if current < THERMOELECTRIC_MAX_UPGRADES {
            set_i32(
                &mut target.nbt,
                "ThermoelectricUpgrades",
                (current + 1) as i32,
            );
            installed = true;
        }
    });
    if !changed || !installed {
        let mut refund = held.clone();
        refund.count = 1;
        host.give_stack(player, refund);
        return Flow::Continue;
    }
    host.block_entity_sync(pos);
    host.action_bar(
        player,
        Text::literal(format!(
            "Thermoelectric Plant upgrades: {}/{THERMOELECTRIC_MAX_UPGRADES} (+{:.2}% efficiency)",
            upgrades + 1,
            THERMOELECTRIC_UPGRADE_EFFICIENCY * 100.0
        )),
    );
    Flow::Cancel
}

fn thermoelectric_heat_throughput(speed: f64) -> u64 {
    (80.0 * f64::from(THERMOELECTRIC_MAX_TEMPERATURE - 20) / f64::from(1100 - 20) * speed)
        .round()
        .max(1.0) as u64
}

fn thermoelectric_convert(
    heat: &mut heat::HeatStore,
    energy: &mut energy::EnergyStore,
    thermoelectric_upgrades: u32,
    heat_upgrades: u32,
    speed: f64,
) -> (u64, u64) {
    if energy.stored_fe() >= THERMOELECTRIC_FE_CAPACITY || heat.stored_hu() == 0 {
        return (0, 0);
    }
    let base_efficiency = thermoelectric_efficiency(heat.temperature_c());
    let heat_upgraded_efficiency = (base_efficiency
        / HEAT_EFFICIENCY_COST_FACTOR.powf(f64::from(heat_upgrades.min(HEAT_MAX_UPGRADES))))
    .min(1.0)
    .max(base_efficiency);
    let efficiency = heat_upgraded_efficiency
        * (1.0
            + THERMOELECTRIC_UPGRADE_EFFICIENCY
                * f64::from(thermoelectric_upgrades.min(THERMOELECTRIC_MAX_UPGRADES)));
    let wanted = div_ceil_u64(
        thermoelectric_heat_throughput(speed).saturating_mul(heat.stored_hu()),
        THERMOELECTRIC_HEAT_CAPACITY,
    );
    let wanted_fe = (wanted as f64 * efficiency) as u64;
    let room = THERMOELECTRIC_FE_CAPACITY - energy.stored_fe();
    let permitted_heat = if wanted_fe > room && efficiency > 0.0 {
        (room as f64 / efficiency) as u64
    } else {
        wanted
    };
    let heat_used = heat.remove_hu(permitted_heat);
    let fe_generated = energy.insert(((heat_used as f64 * efficiency) as u64).min(room), room);
    (heat_used, fe_generated)
}

fn tick_thermoelectric_plant(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let mut heat = heat::HeatStore::new(
        THERMOELECTRIC_HEAT_CAPACITY,
        THERMOELECTRIC_MAX_TEMPERATURE,
        get_i32(&entity.nbt, "HeatHU").max(0) as u64,
    );
    let mut energy = energy::EnergyStore::new(
        THERMOELECTRIC_FE_CAPACITY,
        get_i32(&entity.nbt, "energy").max(0) as u64,
    );
    let upgrades = get_i32(&entity.nbt, "ThermoelectricUpgrades")
        .clamp(0, THERMOELECTRIC_MAX_UPGRADES as i32) as u32;
    let heat_upgrades =
        get_i32(&entity.nbt, "HeatUpgrades").clamp(0, HEAT_MAX_UPGRADES as i32) as u32;
    let speed_upgrades =
        get_i32(&entity.nbt, "SpeedUpgrades").clamp(0, SPEED_MAX_UPGRADES as i32) as u32;
    let (heat_used, fe_generated) = thermoelectric_convert(
        &mut heat,
        &mut energy,
        upgrades,
        heat_upgrades,
        speed_upgrade_multiplier(speed_upgrades),
    );

    push_adjacent_arcforge_energy(host, pos, &mut energy, THERMOELECTRIC_MAX_OUTPUT_FE, false);
    set_i32(
        &mut entity.nbt,
        "HeatHU",
        heat.stored_hu().min(i32::MAX as u64) as i32,
    );
    set_i32(&mut entity.nbt, "TemperatureC", heat.temperature_c() as i32);
    set_i32(
        &mut entity.nbt,
        "energy",
        energy.stored_fe().min(i32::MAX as u64) as i32,
    );
    entity.data.resize(5, 0);
    entity.data[0] = heat.stored_hu().min(i32::MAX as u64) as i32;
    entity.data[1] = heat.temperature_c() as i32;
    entity.data[2] = heat_used.min(i32::MAX as u64) as i32;
    entity.data[3] = fe_generated.min(i32::MAX as u64) as i32;
    entity.data[4] = energy.stored_fe().min(i32::MAX as u64) as i32;
    host.set_block_property(pos, "lit", if fe_generated > 0 { "true" } else { "false" });
}

fn div_ceil_u64(numerator: u64, denominator: u64) -> u64 {
    numerator / denominator + u64::from(numerator % denominator != 0)
}

#[derive(Clone, Debug, PartialEq)]
struct DryingRecipe {
    output: ItemStack,
    fluid: Option<(String, u32)>,
    ticks: u32,
    hu_per_tick: u32,
}

fn drying_item_recipe(host: &dyn Host, input: &ItemStack) -> Option<DryingRecipe> {
    host.recipes_of_type("arcforge:drying")
        .iter()
        .find_map(|entry| {
            let json = entry.json.as_ref();
            if !host.recipe_ingredient(json.get("ingredient")?)?.test(input) {
                return None;
            }
            Some(DryingRecipe {
                output: host.recipe_result(json.get("result")?)?,
                fluid: None,
                ticks: json
                    .get("time")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(200)
                    .clamp(1, i32::MAX as u64) as u32,
                hu_per_tick: json
                    .get("hu_per_tick")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(4)
                    .clamp(1, u32::MAX as u64) as u32,
            })
        })
}

fn drying_fluid_recipe(
    host: &dyn Host,
    fluid_name: &str,
    fluid_amount: u32,
) -> Option<DryingRecipe> {
    host.recipes_of_type("arcforge:drying")
        .iter()
        .find_map(|entry| {
            let json = entry.json.as_ref();
            let fluid = json.get("fluid")?;
            let name = fluid.get("fluid").or_else(|| fluid.get("id"))?.as_str()?;
            let amount = u32::try_from(fluid.get("amount")?.as_u64()?).ok()?;
            if name != fluid_name || fluid_amount < amount {
                return None;
            }
            Some(DryingRecipe {
                output: host.recipe_result(json.get("result")?)?,
                fluid: Some((name.to_owned(), amount)),
                ticks: json
                    .get("time")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(200)
                    .clamp(1, i32::MAX as u64) as u32,
                hu_per_tick: json
                    .get("hu_per_tick")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(4)
                    .clamp(1, u32::MAX as u64) as u32,
            })
        })
}

fn tick_grain_dryer(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    if entity.items.len() != 2 {
        return;
    }
    let item_recipe = entity.items[0]
        .as_ref()
        .and_then(|item| drying_item_recipe(host, item));
    let fluid_recipe = if item_recipe.is_none() {
        fluid::name(&entity.nbt, 0)
            .and_then(|name| drying_fluid_recipe(host, &name, fluid::amount(&entity.nbt, 0)))
    } else {
        None
    };
    let active = tick_grain_dryer_recipe(entity, item_recipe.or(fluid_recipe).as_ref());
    host.set_block_property(pos, "lit", if active { "true" } else { "false" });
}

fn tick_grain_dryer_recipe(entity: &mut ModBlockEntity, recipe: Option<&DryingRecipe>) -> bool {
    if entity.items.len() != 2 {
        return false;
    }
    let Some(recipe) = recipe else {
        set_i32(&mut entity.nbt, "CookTime", 0);
        return false;
    };
    if !can_add(
        entity.items[1].as_ref(),
        &recipe.output.item,
        recipe.output.count,
    ) {
        return false;
    }
    let mut heat = heat::HeatStore::new(
        GRAIN_DRYER_HEAT_CAPACITY,
        GRAIN_DRYER_MAX_TEMPERATURE,
        get_i32(&entity.nbt, "HeatHU").max(0) as u64,
    );
    let heat_upgrades =
        get_i32(&entity.nbt, "HeatUpgrades").clamp(0, HEAT_MAX_UPGRADES as i32) as u32;
    let speed_upgrades =
        get_i32(&entity.nbt, "SpeedUpgrades").clamp(0, SPEED_MAX_UPGRADES as i32) as u32;
    let speed = speed_upgrade_multiplier(speed_upgrades);
    let ticks = speed_upgrade_time(recipe.ticks, speed_upgrades);
    let heat_cost = scaled_upgrade_cost(u64::from(recipe.hu_per_tick), speed, heat_upgrades);
    if heat.temperature_c() <= GRAIN_DRYER_MIN_TEMPERATURE || heat.stored_hu() < heat_cost {
        return false;
    }
    heat.remove_hu(heat_cost);
    set_i32(&mut entity.nbt, "HeatHU", heat.stored_hu() as i32);
    set_i32(&mut entity.nbt, "TemperatureC", heat.temperature_c() as i32);
    let progress = get_i32(&entity.nbt, "CookTime").max(0) as u32 + 1;
    if progress >= ticks {
        if let Some((name, amount)) = &recipe.fluid {
            if fluid::extract(&mut entity.nbt, 0, name, *amount) != *amount {
                set_i32(&mut entity.nbt, "CookTime", 0);
                return false;
            }
        } else {
            consume_one(&mut entity.items[0]);
        }
        add_items(
            &mut entity.items[1],
            &recipe.output.item,
            recipe.output.count,
        );
        set_i32(&mut entity.nbt, "CookTime", 0);
    } else {
        set_i32(&mut entity.nbt, "CookTime", progress as i32);
    }
    entity.data.resize(5, 0);
    entity.data[0] = heat.stored_hu() as i32;
    entity.data[1] = GRAIN_DRYER_HEAT_CAPACITY as i32;
    entity.data[2] = heat.temperature_c() as i32;
    entity.data[3] = get_i32(&entity.nbt, "CookTime");
    entity.data[4] = ticks as i32;
    true
}

fn tick_vulcanizer(entity: &mut ModBlockEntity) {
    if entity.items.len() != 3 {
        return;
    }
    let mut heat = heat::HeatStore::new(
        VULCANIZER_HEAT_CAPACITY,
        VULCANIZER_MAX_TEMPERATURE,
        get_i32(&entity.nbt, "HeatHU").max(0) as u64,
    );
    let heat_upgrades =
        get_i32(&entity.nbt, "HeatUpgrades").clamp(0, HEAT_MAX_UPGRADES as i32) as u32;
    let speed_upgrades =
        get_i32(&entity.nbt, "SpeedUpgrades").clamp(0, SPEED_MAX_UPGRADES as i32) as u32;
    let speed = speed_upgrade_multiplier(speed_upgrades);
    let ticks = speed_upgrade_time(VULCANIZER_TICKS, speed_upgrades);
    let heat_cost = scaled_upgrade_cost(u64::from(VULCANIZER_HU_PER_TICK), speed, heat_upgrades);
    let input_a = entity.items[0].as_ref();
    let input_b = entity.items[1].as_ref();
    let valid = input_a.is_some_and(|item| {
        matches!(
            item.item.as_ref(),
            "arcforge:raw_rubber" | "arcforge:synthetic_rubber"
        ) && item.count >= 2
    }) && input_b
        .is_some_and(|item| item.item.as_ref() == "arcforge:sulfur_dust" && item.count >= 1);
    let progress = get_i32(&entity.nbt, "Progress").max(0) as u32;
    if !valid
        || heat.temperature_c() < VULCANIZER_MIN_TEMPERATURE
        || heat.stored_hu() < heat_cost
        || !can_add(entity.items[2].as_ref(), "arcforge:rubber", 2)
    {
        entity.data = vec![
            heat.stored_hu() as i32,
            VULCANIZER_HEAT_CAPACITY as i32,
            heat.temperature_c() as i32,
            progress as i32,
            ticks as i32,
        ];
        return;
    }
    heat.remove_hu(heat_cost);
    set_i32(&mut entity.nbt, "HeatHU", heat.stored_hu() as i32);
    set_i32(&mut entity.nbt, "TemperatureC", heat.temperature_c() as i32);
    if progress + 1 >= ticks {
        consume_count(&mut entity.items[0], 2);
        consume_count(&mut entity.items[1], 1);
        add_items(&mut entity.items[2], "arcforge:rubber", 2);
        set_i32(&mut entity.nbt, "Progress", 0);
    } else {
        set_i32(&mut entity.nbt, "Progress", (progress + 1) as i32);
    }
    entity.data = vec![
        heat.stored_hu() as i32,
        VULCANIZER_HEAT_CAPACITY as i32,
        heat.temperature_c() as i32,
        get_i32(&entity.nbt, "Progress"),
        ticks as i32,
    ];
}

#[derive(Clone, Debug, PartialEq)]
struct FiberizingRecipe {
    output: ItemStack,
    ticks: u32,
    fe_per_tick: u32,
    hu_per_tick: u32,
    min_temperature_c: u32,
}

fn fiberizer_recipe(host: &dyn Host, input: &ItemStack) -> Option<FiberizingRecipe> {
    host.recipes_of_type("arcforge:fiberizing")
        .iter()
        .find_map(|entry| {
            let json = entry.json.as_ref();
            if !host.recipe_ingredient(json.get("ingredient")?)?.test(input) {
                return None;
            }
            Some(FiberizingRecipe {
                output: host.recipe_result(json.get("result")?)?,
                ticks: json
                    .get("time")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(160)
                    .clamp(1, i32::MAX as u64) as u32,
                fe_per_tick: json
                    .get("fe_per_tick")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(30)
                    .min(u32::MAX as u64) as u32,
                hu_per_tick: json
                    .get("hu_per_tick")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(20)
                    .min(u32::MAX as u64) as u32,
                min_temperature_c: json
                    .get("min_temp")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(800)
                    .min(u32::MAX as u64) as u32,
            })
        })
}

fn tick_fiberizer(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    if entity.items.len() != 2 {
        return;
    }
    let Some(input) = entity.items[0].as_ref() else {
        set_i32(&mut entity.nbt, "CookTime", 0);
        host.set_block_property(pos, "lit", "false");
        return;
    };
    let Some(recipe) = fiberizer_recipe(host, input) else {
        set_i32(&mut entity.nbt, "CookTime", 0);
        host.set_block_property(pos, "lit", "false");
        return;
    };
    if !can_add(
        entity.items[1].as_ref(),
        &recipe.output.item,
        recipe.output.count,
    ) {
        host.set_block_property(pos, "lit", "false");
        return;
    }
    let mut heat = heat::HeatStore::new(10_000, 1100, get_i32(&entity.nbt, "HeatHU").max(0) as u64);
    let mut energy = energy::EnergyStore::new(20_000, get_i32(&entity.nbt, "energy").max(0) as u64);
    let speed_upgrades =
        get_i32(&entity.nbt, "SpeedUpgrades").clamp(0, SPEED_MAX_UPGRADES as i32) as u32;
    let speed = speed_upgrade_multiplier(speed_upgrades);
    let ticks = speed_upgrade_time(recipe.ticks, speed_upgrades);
    let heat_upgrades =
        get_i32(&entity.nbt, "HeatUpgrades").clamp(0, HEAT_MAX_UPGRADES as i32) as u32;
    let heat_cost = scaled_upgrade_cost(u64::from(recipe.hu_per_tick), speed, heat_upgrades);
    let energy_upgrades =
        get_i32(&entity.nbt, "EnergyUpgrades").clamp(0, ENERGY_MAX_UPGRADES as i32) as u32;
    let energy_cost = scaled_upgrade_cost(u64::from(recipe.fe_per_tick), speed, energy_upgrades);
    if heat.temperature_c() < recipe.min_temperature_c
        || heat.stored_hu() < heat_cost
        || energy.stored_fe() < energy_cost
    {
        host.set_block_property(pos, "lit", "false");
        return;
    }
    heat.remove_hu(heat_cost);
    energy.extract(energy_cost, energy_cost);
    let cook = get_i32(&entity.nbt, "CookTime").max(0).saturating_add(1);
    if cook >= ticks as i32 {
        consume_one(&mut entity.items[0]);
        add_items(
            &mut entity.items[1],
            &recipe.output.item,
            recipe.output.count,
        );
        set_i32(&mut entity.nbt, "CookTime", 0);
    } else {
        set_i32(&mut entity.nbt, "CookTime", cook);
    }
    set_i32(
        &mut entity.nbt,
        "HeatHU",
        heat.stored_hu().min(i32::MAX as u64) as i32,
    );
    set_i32(&mut entity.nbt, "TemperatureC", heat.temperature_c() as i32);
    set_i32(
        &mut entity.nbt,
        "energy",
        energy.stored_fe().min(i32::MAX as u64) as i32,
    );
    entity.data.resize(5, 0);
    entity.data[0] = energy.stored_fe().min(i32::MAX as u64) as i32;
    entity.data[1] = heat.stored_hu().min(i32::MAX as u64) as i32;
    entity.data[2] = get_i32(&entity.nbt, "CookTime").max(0);
    entity.data[3] = ticks as i32;
    entity.data[4] = heat.temperature_c() as i32;
    host.set_block_property(pos, "lit", "true");
}

struct SifterOutput {
    stack: ItemStack,
    chance: f32,
}

struct SifterRecipe {
    outputs: Vec<SifterOutput>,
    ticks: u32,
    energy_per_tick: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SifterMeshTier {
    durability: u32,
    chance_multiplier: f32,
    speed_multiplier: f32,
}

fn sifter_mesh_tier(item: &str) -> Option<SifterMeshTier> {
    Some(match item {
        STEEL_MESH => SifterMeshTier {
            durability: 256,
            chance_multiplier: 1.0,
            speed_multiplier: 1.0,
        },
        INVAR_MESH => SifterMeshTier {
            durability: 512,
            chance_multiplier: 1.25,
            speed_multiplier: 1.5,
        },
        TUNGSTEN_MESH => SifterMeshTier {
            durability: 1024,
            chance_multiplier: 1.5,
            speed_multiplier: 2.0,
        },
        SILICON_CARBIDE_MESH => SifterMeshTier {
            durability: 2048,
            chance_multiplier: 1.75,
            speed_multiplier: 2.5,
        },
        _ => return None,
    })
}

fn mesh_takes_damage(unbreaking: u32, roll: f32) -> bool {
    unbreaking == 0 || roll < 1.0 / (unbreaking.saturating_add(1) as f32)
}

fn sifter_recipe(host: &dyn Host, input: &ItemStack) -> Option<SifterRecipe> {
    host.recipes_of_type("arcforge:sifting")
        .iter()
        .find_map(|recipe| {
            let json = recipe.json.as_ref();
            let ingredient = host.recipe_ingredient(json.get("ingredient")?)?;
            if !ingredient.test(input) {
                return None;
            }
            let outputs = json
                .get("outputs")?
                .as_array()?
                .iter()
                .map(|output| {
                    Some(SifterOutput {
                        stack: host.recipe_result(output.get("item")?)?,
                        chance: output.get("chance")?.as_f64()?.clamp(0.0, 1.0) as f32,
                    })
                })
                .collect::<Option<Vec<_>>>()?;
            if outputs.is_empty() || outputs.len() > 6 {
                return None;
            }
            Some(SifterRecipe {
                outputs,
                ticks: json
                    .get("time")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(100)
                    .clamp(1, i32::MAX as u64) as u32,
                energy_per_tick: json
                    .get("energy_per_tick")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(20)
                    .min(i32::MAX as u64) as u32,
            })
        })
}

fn sifter_insert(slots: &mut [Option<ItemStack>], output: &ItemStack) -> bool {
    let mut left = output.count;
    for slot in slots.iter_mut() {
        if let Some(stack) = slot.as_mut()
            && stack.item == output.item
            && stack.damage == output.damage
        {
            let moved = left.min(64_u32.saturating_sub(stack.count));
            stack.count += moved;
            left -= moved;
            if left == 0 {
                return true;
            }
        }
    }
    for slot in slots.iter_mut() {
        if slot.is_none() {
            let moved = left.min(64);
            *slot = Some(ItemStack {
                item: output.item.clone(),
                count: moved,
                damage: output.damage,
                extra: output.extra.clone(),
            });
            left -= moved;
            if left == 0 {
                return true;
            }
        }
    }
    false
}

fn sifter_outputs_fit(entity: &ModBlockEntity, recipe: &SifterRecipe) -> bool {
    let Some(output_slots) = entity.items.get(2..8) else {
        return false;
    };
    let mut simulated = output_slots.to_vec();
    recipe
        .outputs
        .iter()
        .all(|output| sifter_insert(&mut simulated, &output.stack))
}

fn tick_sifter(host: &mut dyn Host, entity: &mut ModBlockEntity) {
    if entity.items.len() != 8 {
        return;
    }
    let Some(input) = entity.items[0].as_ref() else {
        set_i32(&mut entity.nbt, "CookTime", 0);
        entity.data.resize(5, 0);
        entity.data[2] = 0;
        entity.data[3] = 0;
        entity.data[4] = 0;
        return;
    };
    let Some(recipe) = sifter_recipe(host, input) else {
        set_i32(&mut entity.nbt, "CookTime", 0);
        return;
    };
    let Some(mesh) = entity.items[1]
        .as_ref()
        .and_then(|stack| sifter_mesh_tier(&stack.item))
    else {
        return;
    };
    if !sifter_outputs_fit(entity, &recipe) {
        return;
    }
    let speed_upgrades =
        get_i32(&entity.nbt, "SpeedUpgrades").clamp(0, SPEED_MAX_UPGRADES as i32) as u32;
    let speed = speed_upgrade_multiplier(speed_upgrades) * f64::from(mesh.speed_multiplier);
    let mesh_ticks = (f64::from(recipe.ticks) / f64::from(mesh.speed_multiplier))
        .round()
        .max(1.0) as u32;
    let ticks = speed_upgrade_time(mesh_ticks, speed_upgrades);
    let draw = scaled_upgrade_cost(
        u64::from(recipe.energy_per_tick),
        speed,
        get_i32(&entity.nbt, "EnergyUpgrades").clamp(0, ENERGY_MAX_UPGRADES as i32) as u32,
    )
    .min(u64::from(u32::MAX)) as u32;
    let mut energy = energy::EnergyStore::new(20_000, get_i32(&entity.nbt, "energy").max(0) as u64);
    if draw > 0 && energy.stored_fe() < draw as u64 {
        entity.data.resize(5, 0);
        entity.data[0] = energy.stored_fe().min(i32::MAX as u64) as i32;
        entity.data[1] = 20_000;
        entity.data[2] = get_i32(&entity.nbt, "CookTime").max(0);
        entity.data[3] = ticks as i32;
        entity.data[4] = 0;
        return;
    }
    if draw > 0 {
        energy.extract(draw as u64, draw as u64);
    }
    let cook = get_i32(&entity.nbt, "CookTime").max(0) + 1;
    if cook >= ticks as i32 {
        let dimension = host.dimension();
        if let Some(random) = host.level_random(dimension) {
            consume_one(&mut entity.items[0]);
            for output in &recipe.outputs {
                let chance = (output.chance * mesh.chance_multiplier).min(1.0);
                if random.next_float() < chance {
                    sifter_insert(&mut entity.items[2..8], &output.stack);
                }
            }
            if let Some(mesh_stack) = entity.items[1].as_mut() {
                let unbreaking = mesh_stack.enchantment_level("minecraft:unbreaking");
                if mesh_takes_damage(unbreaking, random.next_float()) {
                    mesh_stack.damage = mesh_stack.damage.saturating_add(1);
                    if mesh_stack.damage >= mesh.durability {
                        entity.items[1] = None;
                    }
                }
            }
            set_i32(&mut entity.nbt, "CookTime", 0);
        } else {
            set_i32(&mut entity.nbt, "CookTime", ticks.saturating_sub(1) as i32);
        }
    } else {
        set_i32(&mut entity.nbt, "CookTime", cook);
    }
    set_i32(
        &mut entity.nbt,
        "energy",
        energy.stored_fe().min(i32::MAX as u64) as i32,
    );
    entity.data.resize(5, 0);
    entity.data[0] = energy.stored_fe().min(i32::MAX as u64) as i32;
    entity.data[1] = 20_000;
    entity.data[2] = get_i32(&entity.nbt, "CookTime").max(0);
    entity.data[3] = ticks as i32;
    entity.data[4] = draw.min(i32::MAX as u32) as i32;
}

fn consume_one(slot: &mut Option<ItemStack>) -> bool {
    let Some(stack) = slot.as_mut() else {
        return false;
    };
    if stack.count == 0 {
        return false;
    }
    stack.count -= 1;
    if stack.count == 0 {
        *slot = None;
    }
    true
}

fn add_item(slot: &mut Option<ItemStack>, item: &str) {
    match slot.as_mut() {
        Some(stack) => stack.count += 1,
        None => {
            *slot = Some(ItemStack {
                item: Arc::from(item),
                count: 1,
                damage: 0,
                extra: None,
            })
        }
    }
}

fn tick_combustion_plant(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    if entity.items.len() < 3 {
        return;
    }
    let mut energy = energy::EnergyStore::new(
        COMBUSTION_CAPACITY_FE,
        get_i32(&entity.nbt, "energy").max(0) as u64,
    );
    let mut burn = get_i32(&entity.nbt, "BurnTime").max(0);
    let mut burn_total = get_i32(&entity.nbt, "BurnDuration").max(0);
    let speed_upgrades =
        get_i32(&entity.nbt, "SpeedUpgrades").clamp(0, SPEED_MAX_UPGRADES as i32) as u32;
    let speed = speed_upgrade_multiplier(speed_upgrades);
    let energy_upgrades =
        get_i32(&entity.nbt, "EnergyUpgrades").clamp(0, ENERGY_MAX_UPGRADES as i32) as u32;
    let energy_per_tick = (COMBUSTION_FE_PER_TICK as f64
        * speed
        * (1.0 + 0.125 * f64::from(energy_upgrades)))
    .round() as u64;

    if energy.stored_fe() < COMBUSTION_CAPACITY_FE {
        if burn == 0 {
            let fuel = entity.items[0].as_ref().and_then(|fuel| {
                (fuel.count > 0).then(|| {
                    (
                        fuel.item.to_string(),
                        combustion_fuel_ticks(fuel.item.as_ref()),
                    )
                })
            });
            if let Some((fuel_item, Some(furnace_ticks))) = fuel {
                let burn_speed = f64::from(COMBUSTION_BURN_SPEED) * speed;
                let duration = (f64::from(furnace_ticks) / burn_speed)
                    .round()
                    .clamp(1.0, f64::from(i32::MAX)) as i32;
                if consume_one(&mut entity.items[0]) {
                    burn = duration;
                    burn_total = duration;
                    set_string(&mut entity.nbt, "BurningItem", &fuel_item);
                }
            }
        }

        if burn > 0 {
            let produced = energy.insert(energy_per_tick, energy_per_tick);
            if produced > 0 {
                burn -= 1;
                if burn == 0 {
                    if let Some(burning_item) = get_string(&entity.nbt, "BurningItem") {
                        try_leave_wood_ash(host, &burning_item, &mut entity.items[1]);
                    }
                    entity.nbt.remove(&"BurningItem".into());
                }
            }
        }
    }
    push_adjacent_arcforge_energy(
        host,
        pos,
        &mut energy,
        COMBUSTION_MAX_EXTRACT_FE as u64,
        false,
    );
    set_i32(
        &mut entity.nbt,
        "energy",
        energy.stored_fe().min(i32::MAX as u64) as i32,
    );
    set_i32(&mut entity.nbt, "BurnTime", burn);
    set_i32(&mut entity.nbt, "BurnDuration", burn_total);
    entity.data.resize(4, 0);
    entity.data[0] = burn;
    entity.data[1] = burn_total;
    entity.data[2] = energy.stored_fe().min(i32::MAX as u64) as i32;
    entity.data[3] = COMBUSTION_CAPACITY_FE as i32;
}

fn energy_conduit_spec(id: &str) -> Option<(u64, u64)> {
    let rate = energy_conduit::spec(id)?;
    Some((ENERGY_CONDUIT_CAPACITY_FE, rate))
}

fn energy_cell_spec(id: &str) -> Option<(u64, u64)> {
    match id {
        WROUGHT_ENERGY_CELL => Some((500_000, 512)),
        TEMPERED_ENERGY_CELL => Some((4_000_000, 2_048)),
        HARDENED_ENERGY_CELL => Some((32_000_000, 16_384)),
        ARCFORGED_ENERGY_CELL => Some((256_000_000, 262_144)),
        _ => None,
    }
}

fn energy_cell_charge_level(stored: u64, capacity: u64) -> u32 {
    if stored == 0 || capacity == 0 {
        return 0;
    }
    (stored.saturating_mul(4).div_ceil(capacity)).clamp(1, 4) as u32
}

fn energy_item_spec(item: &str) -> Option<(u64, u64)> {
    energy_cell_spec(item).or_else(|| portable_battery_spec(item))
}

fn portable_energy_item_buffer(item: &str) -> Option<ItemEnergyBuffer> {
    let (capacity, rate) = energy_item_spec(item)?;
    Some(ItemEnergyBuffer {
        capacity: i32::try_from(capacity).ok()?,
        max_insert: i32::try_from(rate).ok()?,
        max_extract: i32::try_from(rate).ok()?,
    })
}

fn portable_battery_spec(item: &str) -> Option<(u64, u64)> {
    match item {
        WROUGHT_BATTERY => Some((250_000, 1_000)),
        TEMPERED_BATTERY => Some((1_000_000, 4_000)),
        HARDENED_BATTERY => Some((4_000_000, 16_000)),
        ARCFORGED_BATTERY => Some((16_000_000, 64_000)),
        _ => None,
    }
}

fn cache_cell_charge(dimension: &str, pos: [i32; 3], item: &str, energy: i32) {
    let now = Instant::now();
    let mut pending = PENDING_CELL_CHARGES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if pending
        .last_pruned
        .is_none_or(|last| now.duration_since(last) >= Duration::from_secs(1))
    {
        pending.entries.retain(|_, (_, _, captured)| {
            now.duration_since(*captured) <= PENDING_CELL_CHARGE_LIFETIME
        });
        pending.last_pruned = Some(now);
    }
    pending
        .entries
        .insert((dimension.to_owned(), pos), (item.to_owned(), energy, now));
}

fn item_energy_amount(stack: &ItemStack, capacity: u64) -> u64 {
    let Some(Tag::Compound(data)) = stack
        .extra
        .as_deref()
        .and_then(|extra| extra.other("minecraft:custom_data"))
    else {
        return 0;
    };
    match data.get(&"horizon_energy".into()) {
        Some(Tag::Int(amount)) => ((*amount).max(0) as u64).min(capacity),
        _ => 0,
    }
}

fn use_energy_item_on_cell(
    host: &mut dyn Host,
    pos: [i32; 3],
    held: &ItemStack,
    selected_slot: i32,
) -> Flow {
    let Some(item_buffer) = portable_energy_item_buffer(&held.item) else {
        return Flow::Continue;
    };
    let Some(cell) = host.mod_block_entity(pos) else {
        return Flow::Continue;
    };
    let Some((cell_capacity, cell_rate)) = energy_cell_spec(&cell.id) else {
        return Flow::Continue;
    };
    // A stack can contain identical charged items, but the energy component is
    // stored per item. Ask the player to split it first rather than applying
    // one transfer to every item in the stack.
    if held.count != 1 {
        if let Some(player) = host.acting_player() {
            host.action_bar(
                player.uuid,
                Text::literal("Split the energy item stack before transferring FE"),
            );
        }
        return Flow::Cancel;
    }
    let old_cell_energy = get_i32(&cell.nbt, "energy").max(0) as u64;
    let old_cell_energy = old_cell_energy.min(cell_capacity);
    let old_item_energy = item_energy_amount(held, item_buffer.capacity as u64);
    let (moved, new_cell_energy, new_item_energy) = if old_item_energy > 0 {
        let moved = old_item_energy
            .min(item_buffer.max_extract.max(0) as u64)
            .min(cell_rate)
            .min(cell_capacity - old_cell_energy);
        (moved, old_cell_energy + moved, old_item_energy - moved)
    } else {
        let moved = old_cell_energy
            .min(item_buffer.max_insert.max(0) as u64)
            .min(cell_rate)
            .min(item_buffer.capacity.max(0) as u64);
        (moved, old_cell_energy - moved, moved)
    };

    if moved == 0 {
        if let Some(player) = host.acting_player() {
            host.action_bar(
                player.uuid,
                Text::literal(format!(
                    "Energy Cell: {old_cell_energy}/{cell_capacity} FE; item: {old_item_energy}/{} FE",
                    item_buffer.capacity
                )),
            );
        }
        return Flow::Cancel;
    }

    let mut updated = held.clone();
    set_item_energy(&mut updated, new_item_energy as i32);
    let mut updated_cell = false;
    let changed = host.change_mod_block_entity(pos, &mut |entity| {
        if entity.id == cell.id
            && (get_i32(&entity.nbt, "energy").max(0) as u64).min(cell_capacity) == old_cell_energy
        {
            set_i32(&mut entity.nbt, "energy", new_cell_energy as i32);
            updated_cell = true;
        }
    });
    if !changed || !updated_cell {
        return Flow::Continue;
    }
    if !replace_carried_stack(host, selected_slot, held, updated) {
        host.change_mod_block_entity(pos, &mut |entity| {
            if entity.id == cell.id {
                set_i32(&mut entity.nbt, "energy", old_cell_energy as i32);
            }
        });
        host.block_entity_sync(pos);
        return Flow::Continue;
    }
    host.block_entity_sync(pos);
    host.set_block_property(
        pos,
        "charge",
        &energy_cell_charge_level(new_cell_energy, cell_capacity).to_string(),
    );
    if let Some(player) = host.acting_player() {
        host.action_bar(
            player.uuid,
            Text::literal(format!(
                "Energy Cell: {new_cell_energy}/{cell_capacity} FE; item: {new_item_energy}/{} FE; moved {moved} FE",
                item_buffer.capacity
            )),
        );
    }
    Flow::Cancel
}

fn take_cell_charge(dimension: &str, pos: [i32; 3]) -> Option<(String, i32)> {
    let mut pending = PENDING_CELL_CHARGES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (item, energy, captured) = pending.entries.remove(&(dimension.to_owned(), pos))?;
    (captured.elapsed() <= PENDING_CELL_CHARGE_LIFETIME).then_some((item, energy))
}

fn set_item_energy(stack: &mut ItemStack, amount: i32) {
    let mut custom_data = match stack
        .extra
        .as_deref()
        .and_then(|extra| extra.other("minecraft:custom_data"))
    {
        Some(Tag::Compound(data)) => data.clone(),
        _ => horizon_client::mod_api::Compound::default(),
    };
    if amount > 0 {
        custom_data.insert("horizon_energy".into(), Tag::Int(amount));
    } else {
        custom_data.remove(&"horizon_energy".into());
    }
    stack
        .extra
        .get_or_insert_with(Box::default)
        .set_other("minecraft:custom_data", Some(Tag::Compound(custom_data)));
}

#[cfg(test)]
mod energy_cell_item_tests {
    use super::{
        ItemStack, Tag, cache_cell_charge, item_energy_amount, set_item_energy, take_cell_charge,
    };
    use std::sync::Arc;

    #[test]
    fn energy_cell_item_round_trips_charge_and_preserves_existing_custom_data() {
        let mut stack = ItemStack {
            item: Arc::from("arcforge:wrought_energy_cell"),
            count: 1,
            damage: 0,
            extra: Some(Box::default()),
        };
        stack.extra.as_mut().unwrap().set_other(
            "minecraft:custom_data",
            Some(Tag::compound([(
                "custom_name",
                Tag::String("named".into()),
            )])),
        );
        set_item_energy(&mut stack, 1234);

        assert_eq!(item_energy_amount(&stack, 500_000), 1234);
        let Some(Tag::Compound(data)) = stack
            .extra
            .as_deref()
            .and_then(|extra| extra.other("minecraft:custom_data"))
        else {
            panic!("item custom data missing");
        };
        assert_eq!(
            data.get(&"custom_name".into()),
            Some(&Tag::String("named".into()))
        );
    }

    #[test]
    fn energy_cell_item_charge_is_clamped_to_tier_capacity() {
        let mut stack = ItemStack {
            item: Arc::from("arcforge:wrought_energy_cell"),
            count: 1,
            damage: 0,
            extra: Some(Box::default()),
        };
        set_item_energy(&mut stack, i32::MAX);
        assert_eq!(item_energy_amount(&stack, 500_000), 500_000);
    }

    #[test]
    fn cached_charge_is_consumed_once_for_the_matching_break() {
        let pos = [i32::MIN, 17, i32::MAX];
        cache_cell_charge(
            "arcforge:test_dimension",
            pos,
            "arcforge:wrought_energy_cell",
            1234,
        );
        assert_eq!(
            take_cell_charge("arcforge:test_dimension", pos),
            Some(("arcforge:wrought_energy_cell".to_owned(), 1234))
        );
        assert_eq!(take_cell_charge("arcforge:test_dimension", pos), None);
    }

    #[test]
    fn an_empty_cell_tick_replaces_a_previous_cached_charge() {
        let pos = [i32::MIN, 18, i32::MAX];
        cache_cell_charge(
            "arcforge:test_dimension",
            pos,
            "arcforge:wrought_energy_cell",
            1234,
        );
        cache_cell_charge(
            "arcforge:test_dimension",
            pos,
            "arcforge:wrought_energy_cell",
            0,
        );
        assert_eq!(
            take_cell_charge("arcforge:test_dimension", pos),
            Some(("arcforge:wrought_energy_cell".to_owned(), 0))
        );
    }
}

fn arcforge_energy_source(id: &str) -> Option<(u64, u64)> {
    match id {
        COMBUSTION => Some((COMBUSTION_CAPACITY_FE, COMBUSTION_MAX_EXTRACT_FE as u64)),
        THERMOELECTRIC_PLANT => Some((THERMOELECTRIC_FE_CAPACITY, THERMOELECTRIC_MAX_OUTPUT_FE)),
        _ => None,
    }
}

#[derive(Clone, Copy)]
struct ConduitSide {
    name: &'static str,
    offset: [i32; 3],
    opposite: [i32; 3],
}

fn energy_conduit_side(face: [i32; 3]) -> Option<ConduitSide> {
    let (name, opposite, offset) = match face {
        [0, 1, 0] => ("up", [0, -1, 0], [0, 1, 0]),
        [0, -1, 0] => ("down", [0, 1, 0], [0, -1, 0]),
        [0, 0, -1] => ("north", [0, 0, 1], [0, 0, -1]),
        [0, 0, 1] => ("south", [0, 0, -1], [0, 0, 1]),
        [1, 0, 0] => ("east", [-1, 0, 0], [1, 0, 0]),
        [-1, 0, 0] => ("west", [1, 0, 0], [-1, 0, 0]),
        _ => return None,
    };
    Some(ConduitSide {
        name,
        offset,
        opposite,
    })
}

#[cfg(test)]
mod crushing_array_tests {
    use super::crushing_array_positions;
    use std::collections::HashSet;

    #[test]
    fn crushing_array_structure_is_a_unique_three_by_three_by_three_cube() {
        let positions = crushing_array_positions([12, -4, 29]).collect::<Vec<_>>();
        let unique = positions.iter().copied().collect::<HashSet<_>>();

        assert_eq!(positions.len(), 27);
        assert_eq!(unique.len(), 27);
        assert!(unique.contains(&[12, -4, 29]));
        assert!(unique.contains(&[11, -5, 28]));
        assert!(unique.contains(&[13, -3, 30]));
        assert!(unique.iter().all(|pos| {
            (pos[0] - 12).abs() <= 1 && (pos[1] + 4).abs() <= 1 && (pos[2] - 29).abs() <= 1
        }));
    }
}

fn conduit_side_setting_key(side: &str) -> String {
    format!("setting_{side}")
}

fn wrench_rotatable_machine(id: &str) -> bool {
    matches!(
        id,
        CARBONIZER
            | COMBUSTION
            | ARC_CRUSHER
            | INDUCTION_FURNACE
            | METAL_PRESS
            | SIFTER
            | INFUSER
            | FIREBOX
            | FUEL_BURNER
            | THERMOELECTRIC_PLANT
            | FIBERIZER
            | VULCANIZER
            | GRAIN_DRYER
    )
}

fn wrench_energy_output_machine(id: &str) -> bool {
    matches!(id, COMBUSTION | THERMOELECTRIC_PLANT)
}

fn next_machine_facing(current: &str) -> Option<&'static str> {
    match current {
        "north" => Some("east"),
        "east" => Some("south"),
        "south" => Some("west"),
        "west" => Some("north"),
        _ => None,
    }
}

#[cfg(test)]
mod wrench_rotation_tests {
    use super::{
        ARC_CRUSHER, ARC_CRUSHING_ARRAY_CASING, COMBUSTION, FURNACE_PORT, RESIN_TAP,
        THERMOELECTRIC_PLANT, next_machine_facing, source_side_can_output,
        wrench_energy_output_machine, wrench_rotatable_machine,
    };

    #[test]
    fn wrench_rotates_only_supported_single_block_machines() {
        assert!(wrench_rotatable_machine(ARC_CRUSHER));
        assert!(!wrench_rotatable_machine(ARC_CRUSHING_ARRAY_CASING));
        assert!(!wrench_rotatable_machine(FURNACE_PORT));
        assert!(!wrench_rotatable_machine(RESIN_TAP));
    }

    #[test]
    fn wrench_configures_only_machines_that_push_energy() {
        assert!(wrench_energy_output_machine(COMBUSTION));
        assert!(wrench_energy_output_machine(THERMOELECTRIC_PLANT));
        assert!(!wrench_energy_output_machine(ARC_CRUSHER));
        assert!(!wrench_energy_output_machine(RESIN_TAP));
    }

    #[test]
    fn energy_output_faces_honor_the_source_side_mode() {
        assert!(source_side_can_output(None));
        assert!(source_side_can_output(Some("auto")));
        assert!(source_side_can_output(Some("output")));
        assert!(!source_side_can_output(Some("input")));
        assert!(!source_side_can_output(Some("disabled")));
    }

    #[test]
    fn wrench_facing_rotates_clockwise_and_ignores_non_horizontal_states() {
        assert_eq!(next_machine_facing("north"), Some("east"));
        assert_eq!(next_machine_facing("east"), Some("south"));
        assert_eq!(next_machine_facing("south"), Some("west"));
        assert_eq!(next_machine_facing("west"), Some("north"));
        assert_eq!(next_machine_facing("up"), None);
        assert_eq!(next_machine_facing("invalid"), None);
    }
}

fn next_conduit_setting(current: &str) -> &'static str {
    match current {
        "auto" => "input",
        "input" => "output",
        "output" => "disabled",
        _ => "auto",
    }
}

fn source_side_can_output(setting: Option<&str>) -> bool {
    matches!(setting, None | Some("auto") | Some("output"))
}

fn conduit_connection_for(setting: &str, neighbor_id: Option<&str>) -> &'static str {
    if setting == "disabled" {
        return "none";
    }
    if neighbor_id.is_some_and(|id| energy_conduit_spec(id).is_some()) {
        return "pipe";
    }
    match setting {
        "input" => "input",
        "output" => "output",
        "auto" if neighbor_id.is_some_and(|id| arcforge_energy_source(id).is_some()) => "output",
        "auto" if neighbor_id.is_some_and(|id| arcforge_energy_target(id).is_some()) => "input",
        _ => "none",
    }
}

fn set_conduit_side_setting(host: &mut dyn Host, pos: [i32; 3], side: ConduitSide, setting: &str) {
    let key = conduit_side_setting_key(side.name);
    host.change_mod_block_entity(pos, &mut |entity| {
        if energy_conduit_spec(&entity.id).is_some() || wrench_energy_output_machine(&entity.id) {
            set_string(&mut entity.nbt, &key, setting);
        }
    });
}

fn arcforge_energy_target(id: &str) -> Option<(u64, u64)> {
    match id {
        ARC_CRUSHER | METAL_PRESS | INDUCTION_FURNACE | INFUSER | FIBERIZER => Some((20_000, 200)),
        ARC_CRUSHING_ARRAY | ARC_CRUSHING_ARRAY_CASING => Some((100_000, 1000)),
        SIFTER => Some((20_000, 1000)),
        _ => energy_conduit_spec(id),
    }
}

fn tick_arcforge_energy_conduit(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    energy_conduit::tick(host, pos, entity, transfer_energy_to_neighbor);
}

fn transfer_energy_to_neighbor(
    host: &mut dyn Host,
    source_pos: [i32; 3],
    neighbor: [i32; 3],
    energy: &mut energy::EnergyStore,
    rate: u64,
) -> u64 {
    let Some(target) = host.mod_block_entity(neighbor) else {
        return 0;
    };
    if energy_conduit_spec(&target.id).is_some() {
        return energy_conduit::accept_energy(host, neighbor, source_pos, energy, rate);
    }
    let Some((capacity, max_insert)) = arcforge_energy_target(&target.id) else {
        return 0;
    };
    let current = get_i32(&target.nbt, "energy").max(0) as u64;
    let offered = energy
        .stored_fe()
        .min(rate.min(max_insert))
        .min(capacity.saturating_sub(current));
    if offered == 0 {
        return 0;
    }
    let mut accepted = 0;
    host.change_mod_block_entity(neighbor, &mut |recipient| {
        if recipient.id != target.id {
            return;
        }
        let stored = get_i32(&recipient.nbt, "energy").max(0) as u64;
        accepted = offered.min(capacity.saturating_sub(stored));
        if accepted > 0 {
            set_i32(
                &mut recipient.nbt,
                "energy",
                stored.saturating_add(accepted).min(i32::MAX as u64) as i32,
            );
        }
    });
    energy.extract(accepted, accepted);
    accepted
}

fn push_adjacent_arcforge_energy(
    host: &mut dyn Host,
    pos: [i32; 3],
    energy: &mut energy::EnergyStore,
    max_output_per_tick: u64,
    source_is_conduit: bool,
) {
    const NEIGHBORS: [[i32; 3]; 6] = [
        [0, 1, 0],
        [0, -1, 0],
        [0, 0, -1],
        [0, 0, 1],
        [-1, 0, 0],
        [1, 0, 0],
    ];
    let mut output_left = max_output_per_tick;
    let owner = host.mod_block_entity(pos);
    let owner_nbt = (source_is_conduit
        || owner
            .as_ref()
            .is_some_and(|owner| wrench_energy_output_machine(&owner.id)))
    .then(|| owner.map(|owner| owner.nbt.clone()))
    .flatten();
    // Drain into machines before forwarding to another conduit. This reduces
    // energy bouncing around loops while still allowing a simple multi-block line.
    for conduits_only in [false, true] {
        for direction in NEIGHBORS {
            if output_left == 0 || energy.stored_fe() == 0 {
                break;
            }
            let neighbor = [
                pos[0] + direction[0],
                pos[1] + direction[1],
                pos[2] + direction[2],
            ];
            let Some(target) = host.mod_block_entity(neighbor) else {
                continue;
            };
            if energy_conduit_spec(&target.id).is_some() != conduits_only {
                continue;
            }
            let Some((capacity, max_insert)) = arcforge_energy_target(&target.id) else {
                continue;
            };
            let owner_side = energy_conduit_side(direction).expect("six neighbor direction");
            let target_side = energy_conduit_side(owner_side.opposite).expect("opposite direction");
            let target_is_conduit = energy_conduit_spec(&target.id).is_some();
            if target_is_conduit {
                let target_setting =
                    get_string(&target.nbt, &conduit_side_setting_key(target_side.name));
                let owner_setting = owner_nbt
                    .as_ref()
                    .and_then(|nbt| get_string(nbt, &conduit_side_setting_key(owner_side.name)));
                let owner_can_output = if source_is_conduit || owner_nbt.is_some() {
                    source_side_can_output(owner_setting.as_deref())
                } else {
                    true
                };
                let target_can_input = matches!(
                    target_setting.as_deref(),
                    None | Some("auto") | Some("input")
                );
                if !owner_can_output || !target_can_input {
                    continue;
                }
            } else if source_is_conduit || owner_nbt.is_some() {
                let setting = owner_nbt
                    .as_ref()
                    .and_then(|nbt| get_string(nbt, &conduit_side_setting_key(owner_side.name)));
                if !source_side_can_output(setting.as_deref()) {
                    continue;
                }
            }
            if target_is_conduit {
                let accepted = energy_conduit::accept_energy(
                    host,
                    neighbor,
                    pos,
                    energy,
                    output_left.min(max_insert),
                );
                output_left = output_left.saturating_sub(accepted);
                continue;
            }
            let current = get_i32(&target.nbt, "energy").max(0) as u64;
            let offered = energy
                .stored_fe()
                .min(output_left)
                .min(max_insert)
                .min(capacity.saturating_sub(current));
            if offered == 0 {
                continue;
            }
            let mut accepted = 0;
            host.change_mod_block_entity(neighbor, &mut |recipient| {
                if recipient.id != target.id {
                    return;
                }
                let stored = get_i32(&recipient.nbt, "energy").max(0) as u64;
                accepted = offered.min(capacity.saturating_sub(stored));
                if accepted > 0 {
                    set_i32(
                        &mut recipient.nbt,
                        "energy",
                        stored.saturating_add(accepted).min(i32::MAX as u64) as i32,
                    );
                }
            });
            if accepted > 0 {
                energy.extract(accepted, accepted);
                output_left -= accepted;
            }
        }
    }
}

fn combustion_fuel_ticks(item: &str) -> Option<i32> {
    match item {
        "minecraft:coal" | "minecraft:charcoal" | "arcforge:bio_coal" => Some(1600),
        "minecraft:coal_block" | "arcforge:bio_coal_block" => Some(16_000),
        _ => None,
    }
}

fn combustion_leaves_ash(item: &str, roll: f32) -> bool {
    item == "minecraft:charcoal" && roll < COMBUSTION_WOOD_ASH_CHANCE
}

fn try_leave_wood_ash(host: &mut dyn Host, item: &str, slot: &mut Option<ItemStack>) -> bool {
    if item != "minecraft:charcoal" {
        return false;
    }
    let dimension = host.dimension();
    if host
        .level_random(dimension)
        .is_some_and(|random| combustion_leaves_ash(item, random.next_float()))
    {
        insert_wood_ash(slot)
    } else {
        false
    }
}

fn insert_wood_ash(slot: &mut Option<ItemStack>) -> bool {
    match slot {
        Some(stack) if stack.item.as_ref() == WOOD_ASH && stack.count < 64 => {
            stack.count += 1;
            true
        }
        Some(_) => false,
        None => {
            add_item(slot, WOOD_ASH);
            true
        }
    }
}

fn get_string(nbt: &horizon_client::mod_api::Compound, key: &str) -> Option<String> {
    match nbt.get(&key.into()) {
        Some(Tag::String(value)) => Some(value.to_string_lossy()),
        _ => None,
    }
}

fn set_string(nbt: &mut horizon_client::mod_api::Compound, key: &str, value: &str) {
    nbt.insert(key.into(), Tag::String(value.into()));
}

struct FurnaceRecipe {
    metal: (horizon_client::mod_api::Ingredient, u32),
    additive: Option<(horizon_client::mod_api::Ingredient, u32)>,
    additive_2: Option<(horizon_client::mod_api::Ingredient, u32)>,
    swapped: bool,
    coke: u32,
    output: ItemStack,
    byproduct: Option<ItemStack>,
    ticks: u32,
    min_temperature_c: u32,
}

fn furnace_ingredient(
    host: &dyn Host,
    json: &serde_json::Value,
) -> Option<(horizon_client::mod_api::Ingredient, u32)> {
    Some((
        host.recipe_ingredient(json.get("ingredient")?)?,
        json.get("count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1)
            .clamp(1, 64) as u32,
    ))
}

fn arcforge_furnace_ingredient_accepts(host: &dyn Host, stack: &ItemStack, key: &str) -> bool {
    host.recipes_of_type("arcforge:arcforge_smelting")
        .iter()
        .filter_map(|recipe| recipe.json.get(key))
        .filter_map(|value| value.get("ingredient"))
        .filter_map(|ingredient| host.recipe_ingredient(ingredient))
        .any(|ingredient| ingredient.test(stack))
}

fn cache_arcforge_furnace_menu_rules(host: &dyn Host) {
    let mut rules = ArcforgeFurnaceMenuRules::default();
    for recipe in host.recipes_of_type("arcforge:arcforge_smelting") {
        let json = recipe.json.as_ref();
        if let Some(ingredient) = json
            .get("metal")
            .and_then(|value| value.get("ingredient"))
            .and_then(|value| host.recipe_ingredient(value))
        {
            rules.metal.push(ingredient);
        }
        for key in ["additive", "additive_2"] {
            if let Some(ingredient) = json
                .get(key)
                .and_then(|value| value.get("ingredient"))
                .and_then(|value| host.recipe_ingredient(value))
            {
                rules.additives.push(ingredient);
            }
        }
    }
    *ARCFORGE_FURNACE_MENU_RULES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = rules;
}

fn steelworks_recipe(host: &dyn Host, entity: &ModBlockEntity) -> Option<FurnaceRecipe> {
    find_furnace_recipe(host, entity, 3, true)
}

fn arcforge_furnace_recipe(host: &dyn Host, entity: &ModBlockEntity) -> Option<FurnaceRecipe> {
    find_furnace_recipe(host, entity, 3, false)
}

fn find_furnace_recipe(
    host: &dyn Host,
    entity: &ModBlockEntity,
    coke_slot: usize,
    require_coke_reagent_in_slot: bool,
) -> Option<FurnaceRecipe> {
    let metal_stack = entity.items.first()?.as_ref()?;
    host.recipes_of_type("arcforge:arcforge_smelting")
        .iter()
        .find_map(|recipe| {
            let json = recipe.json.as_ref();
            let metal = furnace_ingredient(host, json.get("metal")?)?;
            if !metal.0.test(metal_stack) || metal_stack.count < metal.1 {
                return None;
            }

            let additive = if let Some(value) = json.get("additive") {
                Some(furnace_ingredient(host, value)?)
            } else {
                None
            };
            let additive_2 = if let Some(value) = json.get("additive_2") {
                Some(furnace_ingredient(host, value)?)
            } else {
                None
            };
            let slot_1 = entity.items.get(1)?.as_ref();
            let slot_2 = entity.items.get(2)?.as_ref();
            let in_order = furnace_input_fits(additive.as_ref(), slot_1)
                && furnace_input_fits(additive_2.as_ref(), slot_2);
            let swapped = !in_order
                && furnace_input_fits(additive.as_ref(), slot_2)
                && furnace_input_fits(additive_2.as_ref(), slot_1);
            if !in_order && !swapped {
                return None;
            }
            let coke = json
                .get("coke")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
                .min(64) as u32;
            if require_coke_reagent_in_slot {
                match (coke, entity.items.get(coke_slot)?.as_ref()) {
                    (0, None) => {}
                    (required, Some(stack))
                        if required > 0
                            && stack.item.as_ref() == COKE
                            && stack.count >= required => {}
                    _ => return None,
                }
            }
            let output = host.recipe_result(json.get("result")?)?;
            let byproduct = if let Some(value) = json.get("byproduct") {
                Some(host.recipe_result(value)?)
            } else {
                None
            };
            Some(FurnaceRecipe {
                metal,
                additive,
                additive_2,
                swapped,
                coke,
                output,
                byproduct,
                ticks: json
                    .get("time")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(200)
                    .clamp(1, i32::MAX as u64) as u32,
                min_temperature_c: json
                    .get("min_temp")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0)
                    .min(u32::MAX as u64) as u32,
            })
        })
}

fn furnace_input_fits(
    expected: Option<&(horizon_client::mod_api::Ingredient, u32)>,
    actual: Option<&ItemStack>,
) -> bool {
    match (expected, actual) {
        (None, None) => true,
        (Some((ingredient, count)), Some(stack)) => ingredient.test(stack) && stack.count >= *count,
        _ => false,
    }
}

fn tick_steelworks(host: &dyn Host, entity: &mut ModBlockEntity) {
    let schema = get_i32(&entity.nbt, "MachineSchema").max(0) as u32;
    migrate_steelworks_inventory(&mut entity.items, schema);
    set_i32(&mut entity.nbt, "MachineSchema", 1);
    if entity.items.len() < 7 {
        return;
    }
    let recipe = steelworks_recipe(host, entity);
    let can_process = recipe
        .as_ref()
        .is_some_and(|recipe| can_process(entity, recipe));
    let lit_total = get_i32(&entity.nbt, "BurnDuration");
    let mut burn = get_i32(&entity.nbt, "BurnTime").saturating_sub(1).max(0);
    let mut cook = get_i32(&entity.nbt, "CookTime");

    if burn == 0 && can_process && consume_fuel(entity) {
        burn = BURN_TIME;
        set_i32(&mut entity.nbt, "BurnDuration", BURN_TIME);
    }

    let mut heat = heat::HeatStore::new(40_000, 1600, get_i32(&entity.nbt, "HeatHU").max(0) as u64);
    if burn > 0 {
        heat.add_hu(80);
    } else {
        let mut ambient = heat::HeatStore::new(40_000, 1600, 0);
        heat.transfer_to(&mut ambient, 10);
    }
    let temperature = heat.temperature_c() as i32;
    if let Some(recipe) = recipe.as_ref().filter(|_| can_process) {
        if burn > 0 && temperature >= recipe.min_temperature_c as i32 {
            cook += 1;
            if cook >= recipe.ticks as i32 {
                if consume_coke(entity, recipe) {
                    process_one(entity, recipe);
                }
                cook = 0;
            }
        } else {
            cook = 0;
        }
    } else {
        cook = 0;
    }

    set_i32(&mut entity.nbt, "BurnTime", burn);
    set_i32(&mut entity.nbt, "CookTime", cook);
    set_i32(&mut entity.nbt, "TemperatureC", temperature);
    set_i32(
        &mut entity.nbt,
        "HeatHU",
        heat.stored_hu().min(i32::MAX as u64) as i32,
    );
    entity.data.resize(4, 0);
    entity.data[0] = burn;
    entity.data[1] = lit_total.max(BURN_TIME);
    entity.data[2] = cook;
    entity.data[3] = recipe.map_or(0, |recipe| recipe.ticks.min(i32::MAX as u32) as i32);
}

fn arcforge_furnace_coke_count(entity: &ModBlockEntity) -> u32 {
    let loose = get_i32(&entity.nbt, "LooseCoke").max(0) as u32;
    let Some(stack) = entity.items.get(3).and_then(Option::as_ref) else {
        return loose;
    };
    loose.saturating_add(if stack.item.as_ref() == COKE_BLOCK {
        stack.count.saturating_mul(9)
    } else if stack.item.as_ref() == COKE {
        stack.count
    } else {
        0
    })
}

fn arcforge_furnace_take_coke(entity: &mut ModBlockEntity) -> bool {
    let loose = get_i32(&entity.nbt, "LooseCoke").max(0);
    if loose > 0 {
        set_i32(&mut entity.nbt, "LooseCoke", loose - 1);
        return true;
    }
    let Some(stack) = entity.items.get_mut(3).and_then(Option::as_mut) else {
        return false;
    };
    if stack.item.as_ref() == COKE_BLOCK {
        stack.count -= 1;
        if stack.count == 0 {
            entity.items[3] = None;
        }
        set_i32(&mut entity.nbt, "LooseCoke", 8);
        true
    } else if stack.item.as_ref() == COKE && stack.count > 0 {
        stack.count -= 1;
        if stack.count == 0 {
            entity.items[3] = None;
        }
        true
    } else {
        false
    }
}

fn tick_arcforge_furnace_process(host: &dyn Host, entity: &mut ModBlockEntity) {
    entity.items.resize_with(6, || None);
    let recipe = arcforge_furnace_recipe(host, entity);
    let can_smelt = recipe.as_ref().is_some_and(|recipe| {
        arcforge_furnace_coke_count(entity) >= recipe.coke
            && can_add(
                entity.items.get(4).and_then(Option::as_ref),
                &recipe.output.item,
                recipe.output.count,
            )
            && recipe.byproduct.as_ref().is_none_or(|byproduct| {
                can_add(
                    entity.items.get(5).and_then(Option::as_ref),
                    &byproduct.item,
                    byproduct.count,
                )
            })
    });

    let mut burn = get_i32(&entity.nbt, "BurnTime").max(0);
    let mut cook = get_i32(&entity.nbt, "CookTime").max(0);
    if burn == 0 {
        if let Some(recipe) = recipe.as_ref().filter(|_| can_smelt) {
            if arcforge_furnace_coke_count(entity) > recipe.coke
                && arcforge_furnace_take_coke(entity)
            {
                burn = 800;
                set_i32(&mut entity.nbt, "BurnDuration", 800);
            }
        }
    }

    let previous_heat = get_i32(&entity.nbt, "TemperatureC").max(20);
    let (heat, burn) = if burn > 0 {
        (previous_heat.saturating_add(2).min(1600), burn - 1)
    } else {
        (previous_heat.saturating_sub(1).max(20), 0)
    };
    if let Some(recipe) = recipe.as_ref().filter(|_| can_smelt) {
        if heat >= recipe.min_temperature_c as i32 {
            cook += 1;
            if cook >= recipe.ticks as i32 {
                if (0..recipe.coke).all(|_| arcforge_furnace_take_coke(entity)) {
                    process_arcforge_furnace_recipe(entity, recipe);
                }
                cook = 0;
            }
        }
    } else {
        cook = 0;
    }

    set_i32(&mut entity.nbt, "BurnTime", burn);
    set_i32(&mut entity.nbt, "CookTime", cook);
    set_i32(&mut entity.nbt, "TemperatureC", heat);
    entity.data.resize(8, 0);
    entity.data[0] = burn;
    entity.data[1] = get_i32(&entity.nbt, "BurnDuration").max(800);
    entity.data[2] = cook;
    entity.data[3] = recipe
        .as_ref()
        .map_or(0, |recipe| recipe.ticks.min(i32::MAX as u32) as i32);
    entity.data[4] = heat;
    entity.data[5] = i32::from(get_i32(&entity.nbt, "FurnaceFormed") != 0);
    entity.data[6] = recipe.as_ref().map_or(0, |recipe| {
        recipe.min_temperature_c.min(i32::MAX as u32) as i32
    });
    entity.data[7] = recipe.map_or(0, |recipe| recipe.coke.min(i32::MAX as u32) as i32);
}

fn process_arcforge_furnace_recipe(entity: &mut ModBlockEntity, recipe: &FurnaceRecipe) {
    consume_count(&mut entity.items[0], recipe.metal.1);
    if let Some((_, count)) = &recipe.additive {
        consume_count(
            &mut entity.items[if recipe.swapped { 2 } else { 1 }],
            *count,
        );
    }
    if let Some((_, count)) = &recipe.additive_2 {
        consume_count(
            &mut entity.items[if recipe.swapped { 1 } else { 2 }],
            *count,
        );
    }
    add_items(
        &mut entity.items[4],
        &recipe.output.item,
        recipe.output.count,
    );
    if let Some(byproduct) = &recipe.byproduct {
        add_items(&mut entity.items[5], &byproduct.item, byproduct.count);
    }
}

fn migrate_steelworks_inventory(items: &mut Vec<Option<ItemStack>>, schema: u32) {
    if schema == 0 && items.len() == 5 {
        let mut old = std::mem::take(items).into_iter();
        let mut migrated = (0..7).map(|_| None).collect::<Vec<_>>();
        migrated[0] = old.next().flatten();
        migrated[3] = old.next().flatten();
        migrated[4] = old.next().flatten();
        migrated[5] = old.next().flatten();
        migrated[6] = old.next().flatten();
        *items = migrated;
    } else if items.len() < 7 {
        items.resize_with(7, || None);
    }
}

fn can_process(entity: &ModBlockEntity, recipe: &FurnaceRecipe) -> bool {
    can_add(
        entity.items.get(5).and_then(Option::as_ref),
        &recipe.output.item,
        recipe.output.count,
    ) && recipe.byproduct.as_ref().is_none_or(|item| {
        can_add(
            entity.items.get(6).and_then(Option::as_ref),
            &item.item,
            item.count,
        )
    })
}

fn consume_fuel(entity: &mut ModBlockEntity) -> bool {
    let Some(fuel) = entity.items[4].as_mut() else {
        return false;
    };
    if fuel.item.as_ref() != COKE || fuel.count == 0 {
        return false;
    }
    fuel.count -= 1;
    if fuel.count == 0 {
        entity.items[4] = None;
    }
    true
}

fn consume_coke(entity: &mut ModBlockEntity, recipe: &FurnaceRecipe) -> bool {
    consume_count(&mut entity.items[3], recipe.coke)
}

fn process_one(entity: &mut ModBlockEntity, recipe: &FurnaceRecipe) {
    consume_count(&mut entity.items[0], recipe.metal.1);
    if let Some((_, count)) = &recipe.additive {
        consume_count(
            &mut entity.items[if recipe.swapped { 2 } else { 1 }],
            *count,
        );
    }
    if let Some((_, count)) = &recipe.additive_2 {
        consume_count(
            &mut entity.items[if recipe.swapped { 1 } else { 2 }],
            *count,
        );
    }
    add_items(
        &mut entity.items[5],
        &recipe.output.item,
        recipe.output.count,
    );
    if let Some(item) = &recipe.byproduct {
        add_items(&mut entity.items[6], &item.item, item.count);
    }
}

fn consume_count(slot: &mut Option<ItemStack>, count: u32) -> bool {
    if count == 0 {
        return true;
    }
    let Some(stack) = slot.as_mut() else {
        return false;
    };
    if stack.count < count {
        return false;
    }
    stack.count -= count;
    if stack.count == 0 {
        *slot = None;
    }
    true
}

fn restore_resin_tap_resin(host: &mut dyn Host, pos: [i32; 3], count: u32) -> bool {
    if count == 0 {
        return true;
    }
    let mut restored = 0;
    let changed = host.change_mod_block_entity(pos, &mut |entity| {
        if entity.id != RESIN_TAP {
            return;
        }
        if entity.items.is_empty() {
            entity.items.push(None);
        }
        match &mut entity.items[0] {
            Some(stack) if stack.item.as_ref() == PINE_RESIN => {
                restored = count.min(16u32.saturating_sub(stack.count));
                stack.count += restored;
            }
            None => {
                restored = count.min(16);
                entity.items[0] = Some(ItemStack {
                    item: Arc::from(PINE_RESIN),
                    count: restored,
                    damage: 0,
                    extra: None,
                });
            }
            _ => {}
        }
    });
    changed && restored == count
}

fn can_add(stack: Option<&ItemStack>, item: &str, count: u32) -> bool {
    stack.is_none_or(|stack| {
        stack.item.as_ref() == item && stack.count.checked_add(count).is_some_and(|n| n <= 64)
    })
}

fn get_i32(nbt: &horizon_client::mod_api::Compound, key: &str) -> i32 {
    match nbt.get(&key.into()) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn set_i32(nbt: &mut horizon_client::mod_api::Compound, key: &str, value: i32) {
    nbt.insert(key.into(), Tag::Int(value));
}

#[cfg(test)]
mod tests {
    use super::*;

    struct InductionTestHost {
        recipes: Vec<horizon_client::mod_api::CustomRecipe>,
    }

    impl Host for InductionTestHost {
        fn dimension(&self) -> &'static str {
            "minecraft:overworld"
        }

        fn recipes_of_type(&self, kind: &str) -> &[horizon_client::mod_api::CustomRecipe] {
            if kind == "arcforge:induction_smelting" {
                &self.recipes
            } else {
                &[]
            }
        }

        fn recipe_ingredient(
            &self,
            json: &serde_json::Value,
        ) -> Option<horizon_client::mod_api::Ingredient> {
            let id = json.as_str()?.strip_prefix('#').unwrap_or(json.as_str()?);
            let items = match id {
                "c:ores/silver" => vec!["arcforge:silver_ore".to_owned()],
                _ => vec![id.to_owned()],
            };
            Some(horizon_client::mod_api::Ingredient::of(items))
        }

        fn recipe_result(&self, json: &serde_json::Value) -> Option<ItemStack> {
            let item = json.get("id")?.as_str()?;
            let count = json
                .get("count")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(1);
            Some(ItemStack {
                item: Arc::from(item),
                count: u32::try_from(count).ok()?,
                damage: 0,
                extra: None,
            })
        }
    }

    struct TestHost {
        random: horizon_client::mod_api::LegacyRandom,
    }

    impl Host for TestHost {
        fn dimension(&self) -> &'static str {
            "minecraft:overworld"
        }

        fn level_random(
            &mut self,
            _dimension: &str,
        ) -> Option<&mut horizon_client::mod_api::LegacyRandom> {
            Some(&mut self.random)
        }

        fn mod_block_entity(&self, _pos: [i32; 3]) -> Option<ModBlockEntity> {
            None
        }

        fn change_mod_block_entity(
            &mut self,
            _pos: [i32; 3],
            _change: &mut dyn FnMut(&mut ModBlockEntity),
        ) -> bool {
            false
        }
    }

    struct EnergyPushTestHost {
        random: horizon_client::mod_api::LegacyRandom,
        adjacent_pos: [i32; 3],
        adjacent: ModBlockEntity,
    }

    impl Host for EnergyPushTestHost {
        fn dimension(&self) -> &'static str {
            "minecraft:overworld"
        }

        fn level_random(
            &mut self,
            _dimension: &str,
        ) -> Option<&mut horizon_client::mod_api::LegacyRandom> {
            Some(&mut self.random)
        }

        fn mod_block_entity(&self, pos: [i32; 3]) -> Option<ModBlockEntity> {
            (pos == self.adjacent_pos).then(|| self.adjacent.clone())
        }

        fn change_mod_block_entity(
            &mut self,
            pos: [i32; 3],
            change: &mut dyn FnMut(&mut ModBlockEntity),
        ) -> bool {
            if pos != self.adjacent_pos {
                return false;
            }
            change(&mut self.adjacent);
            true
        }

        fn set_block_property(&mut self, _pos: [i32; 3], _key: &str, _value: &str) -> bool {
            true
        }
    }

    struct DryingTestHost {
        random: horizon_client::mod_api::LegacyRandom,
        recipes: Vec<horizon_client::mod_api::CustomRecipe>,
    }

    impl Host for DryingTestHost {
        fn dimension(&self) -> &'static str {
            "minecraft:overworld"
        }

        fn level_random(
            &mut self,
            _dimension: &str,
        ) -> Option<&mut horizon_client::mod_api::LegacyRandom> {
            Some(&mut self.random)
        }

        fn recipes_of_type(&self, kind: &str) -> &[horizon_client::mod_api::CustomRecipe] {
            let _ = kind;
            &self.recipes
        }

        fn recipe_ingredient(
            &self,
            json: &serde_json::Value,
        ) -> Option<horizon_client::mod_api::Ingredient> {
            Some(horizon_client::mod_api::Ingredient::of(vec![
                json.as_str()?.to_owned(),
            ]))
        }

        fn recipe_result(&self, json: &serde_json::Value) -> Option<ItemStack> {
            Some(ItemStack {
                item: Arc::from(json.get("id")?.as_str()?),
                count: json
                    .get("count")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(1) as u32,
                damage: 0,
                extra: None,
            })
        }
    }

    #[test]
    fn vulcanizer_cures_rubber_with_saved_heat_and_upstream_batch_values() {
        let mut entity = ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
            id: VULCANIZER.to_owned(),
            items: vec![
                Some(ItemStack {
                    item: Arc::from("arcforge:raw_rubber"),
                    count: 2,
                    damage: 0,
                    extra: None,
                }),
                Some(ItemStack {
                    item: Arc::from("arcforge:sulfur_dust"),
                    count: 1,
                    damage: 0,
                    extra: None,
                }),
                None,
            ],
            ..Default::default()
        });
        set_i32(&mut entity.nbt, "HeatHU", VULCANIZER_HEAT_CAPACITY as i32);
        for _ in 0..VULCANIZER_TICKS {
            tick_vulcanizer(&mut entity);
        }
        assert_eq!(entity.items[0], None);
        assert_eq!(entity.items[1], None);
        assert_eq!(
            entity.items[2].as_ref().unwrap().item.as_ref(),
            "arcforge:rubber"
        );
        assert_eq!(entity.items[2].as_ref().unwrap().count, 2);
        assert_eq!(get_i32(&entity.nbt, "HeatHU"), 5600);
        assert_eq!(get_i32(&entity.nbt, "Progress"), 0);
    }

    #[test]
    fn vulcanizer_pauses_below_minimum_temperature_and_on_full_output() {
        let mut entity = ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
            id: VULCANIZER.to_owned(),
            items: vec![
                Some(ItemStack {
                    item: Arc::from("arcforge:synthetic_rubber"),
                    count: 2,
                    damage: 0,
                    extra: None,
                }),
                Some(ItemStack {
                    item: Arc::from("arcforge:sulfur_dust"),
                    count: 1,
                    damage: 0,
                    extra: None,
                }),
                None,
            ],
            ..Default::default()
        });
        set_i32(&mut entity.nbt, "HeatHU", 1000);
        tick_vulcanizer(&mut entity);
        assert_eq!(get_i32(&entity.nbt, "Progress"), 0);
        assert_eq!(get_i32(&entity.nbt, "HeatHU"), 1000);

        set_i32(&mut entity.nbt, "HeatHU", VULCANIZER_HEAT_CAPACITY as i32);
        entity.items[2] = Some(ItemStack {
            item: Arc::from("arcforge:rubber"),
            count: 64,
            damage: 0,
            extra: None,
        });
        tick_vulcanizer(&mut entity);
        assert_eq!(get_i32(&entity.nbt, "Progress"), 0);
        assert_eq!(
            get_i32(&entity.nbt, "HeatHU"),
            VULCANIZER_HEAT_CAPACITY as i32
        );
    }

    #[test]
    fn grain_dryer_processes_item_and_latex_recipes_with_saved_heat() {
        let mut item_entity =
            ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
                id: GRAIN_DRYER.to_owned(),
                items: vec![
                    Some(ItemStack {
                        item: Arc::from("minecraft:wheat"),
                        count: 1,
                        damage: 0,
                        extra: None,
                    }),
                    None,
                ],
                ..Default::default()
            });
        set_i32(
            &mut item_entity.nbt,
            "HeatHU",
            GRAIN_DRYER_HEAT_CAPACITY as i32,
        );
        let grain = DryingRecipe {
            output: ItemStack {
                item: Arc::from("arcforge:dried_grain"),
                count: 1,
                damage: 0,
                extra: None,
            },
            fluid: None,
            ticks: 200,
            hu_per_tick: 4,
        };
        for _ in 0..grain.ticks {
            assert!(tick_grain_dryer_recipe(&mut item_entity, Some(&grain)));
        }
        assert_eq!(item_entity.items[0], None);
        assert_eq!(
            item_entity.items[1].as_ref().unwrap().item.as_ref(),
            "arcforge:dried_grain"
        );
        assert_eq!(get_i32(&item_entity.nbt, "HeatHU"), 3200);

        let mut latex_entity =
            ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
                id: GRAIN_DRYER.to_owned(),
                items: vec![None, None],
                ..Default::default()
            });
        set_i32(
            &mut latex_entity.nbt,
            "HeatHU",
            GRAIN_DRYER_HEAT_CAPACITY as i32,
        );
        assert_eq!(
            fluid::insert(
                &mut latex_entity.nbt,
                0,
                "arcforge:latex",
                250,
                GRAIN_DRYER_TANK_CAPACITY,
            ),
            250
        );
        let rubber = DryingRecipe {
            output: ItemStack {
                item: Arc::from("arcforge:raw_rubber"),
                count: 1,
                damage: 0,
                extra: None,
            },
            fluid: Some(("arcforge:latex".into(), 250)),
            ticks: 300,
            hu_per_tick: 4,
        };
        for _ in 0..rubber.ticks {
            assert!(tick_grain_dryer_recipe(&mut latex_entity, Some(&rubber)));
        }
        assert_eq!(fluid::amount(&latex_entity.nbt, 0), 0);
        assert_eq!(
            latex_entity.items[1].as_ref().unwrap().item.as_ref(),
            "arcforge:raw_rubber"
        );
        assert_eq!(get_i32(&latex_entity.nbt, "HeatHU"), 2800);
    }

    #[test]
    fn grain_dryer_preserves_progress_when_cold_or_output_is_full() {
        let mut entity = ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
            id: GRAIN_DRYER.to_owned(),
            items: vec![
                Some(ItemStack {
                    item: Arc::from("arcforge:hop_cones"),
                    count: 1,
                    damage: 0,
                    extra: None,
                }),
                None,
            ],
            ..Default::default()
        });
        let recipe = DryingRecipe {
            output: ItemStack {
                item: Arc::from("arcforge:dried_hops"),
                count: 1,
                damage: 0,
                extra: None,
            },
            fluid: None,
            ticks: 200,
            hu_per_tick: 4,
        };
        set_i32(&mut entity.nbt, "CookTime", 23);
        set_i32(&mut entity.nbt, "HeatHU", 100);
        assert!(!tick_grain_dryer_recipe(&mut entity, Some(&recipe)));
        assert_eq!(get_i32(&entity.nbt, "CookTime"), 23);
        assert_eq!(get_i32(&entity.nbt, "HeatHU"), 100);

        set_i32(&mut entity.nbt, "HeatHU", GRAIN_DRYER_HEAT_CAPACITY as i32);
        entity.items[1] = Some(ItemStack {
            item: Arc::from("arcforge:dried_hops"),
            count: 64,
            damage: 0,
            extra: None,
        });
        assert!(!tick_grain_dryer_recipe(&mut entity, Some(&recipe)));
        assert_eq!(get_i32(&entity.nbt, "CookTime"), 23);
        assert_eq!(
            get_i32(&entity.nbt, "HeatHU"),
            GRAIN_DRYER_HEAT_CAPACITY as i32
        );
    }

    #[test]
    fn grain_dryer_data_includes_upstream_recipe_set_and_crafting_path() {
        let data =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/data/arcforge");
        let expected = [
            (
                "dried_grain_from_wheat.json",
                "minecraft:wheat",
                "arcforge:dried_grain",
            ),
            (
                "dried_hops_from_hop_cones.json",
                "arcforge:hop_cones",
                "arcforge:dried_hops",
            ),
            (
                "dried_sorghum_from_sorghum_stalks.json",
                "arcforge:sorghum_stalks",
                "arcforge:dried_sorghum",
            ),
        ];
        for (file, ingredient, result) in expected {
            let recipe: serde_json::Value = serde_json::from_slice(
                &std::fs::read(data.join("recipe/drying").join(file)).expect("drying recipe"),
            )
            .expect("valid drying recipe");
            assert_eq!(recipe["type"], "arcforge:drying");
            assert_eq!(recipe["ingredient"], ingredient);
            assert_eq!(recipe["result"]["id"], result);
            assert_eq!(recipe["time"], 200);
            assert_eq!(recipe["hu_per_tick"], 4);
        }
        let latex: serde_json::Value = serde_json::from_slice(
            &std::fs::read(data.join("recipe/drying/raw_rubber_from_latex.json"))
                .expect("Latex recipe"),
        )
        .expect("valid Latex recipe");
        assert_eq!(latex["fluid"]["fluid"], "arcforge:latex");
        assert_eq!(latex["fluid"]["amount"], 250);
        assert_eq!(latex["result"]["id"], "arcforge:raw_rubber");
        assert_eq!(latex["time"], 300);
        assert_eq!(latex["hu_per_tick"], 4);

        let crafting: serde_json::Value = serde_json::from_slice(
            &std::fs::read(data.join("recipe/crafting/grain_dryer.json"))
                .expect("Grain Dryer craft"),
        )
        .expect("valid Grain Dryer craft");
        assert_eq!(crafting["result"]["id"], GRAIN_DRYER);

        let host = DryingTestHost {
            random: horizon_client::mod_api::LegacyRandom::new(0),
            recipes: [
                "dried_grain_from_wheat.json",
                "dried_hops_from_hop_cones.json",
                "dried_sorghum_from_sorghum_stalks.json",
                "raw_rubber_from_latex.json",
            ]
            .into_iter()
            .map(|file| horizon_client::mod_api::CustomRecipe {
                id: format!("arcforge:drying/{file}"),
                json: Arc::new(
                    serde_json::from_slice(
                        &std::fs::read(data.join("recipe/drying").join(file))
                            .expect("drying recipe"),
                    )
                    .expect("valid drying recipe"),
                ),
            })
            .collect(),
        };
        for (input, output) in [
            ("minecraft:wheat", "arcforge:dried_grain"),
            ("arcforge:hop_cones", "arcforge:dried_hops"),
            ("arcforge:sorghum_stalks", "arcforge:dried_sorghum"),
        ] {
            let recipe = drying_item_recipe(
                &host,
                &ItemStack {
                    item: Arc::from(input),
                    count: 1,
                    damage: 0,
                    extra: None,
                },
            )
            .expect("matching item drying recipe");
            assert_eq!(recipe.output.item.as_ref(), output);
            assert_eq!(recipe.ticks, 200);
            assert_eq!(recipe.hu_per_tick, 4);
        }
        let latex = drying_fluid_recipe(&host, "arcforge:latex", 250)
            .expect("matching Latex drying recipe");
        assert_eq!(latex.output.item.as_ref(), "arcforge:raw_rubber");
        assert_eq!(latex.fluid, Some(("arcforge:latex".into(), 250)));
        assert_eq!(latex.ticks, 300);
        assert_eq!(drying_fluid_recipe(&host, "arcforge:latex", 249), None);
    }

    #[test]
    fn combustion_fuel_matches_arcforge_furnace_burn_times() {
        assert_eq!(combustion_fuel_ticks("minecraft:coal"), Some(1600));
        assert_eq!(combustion_fuel_ticks("minecraft:charcoal"), Some(1600));
        assert_eq!(combustion_fuel_ticks("minecraft:coal_block"), Some(16_000));
        assert_eq!(combustion_fuel_ticks("arcforge:bio_coal"), Some(1600));
        assert_eq!(
            combustion_fuel_ticks("arcforge:bio_coal_block"),
            Some(16_000)
        );
        assert_eq!(combustion_fuel_ticks("minecraft:lava_bucket"), None);
        assert!(combustion_leaves_ash("minecraft:charcoal", 0.49));
        assert!(!combustion_leaves_ash("minecraft:charcoal", 0.5));
        assert!(!combustion_leaves_ash("arcforge:bio_coal", 0.25));
        assert!(!combustion_leaves_ash("minecraft:coal", 0.0));
        assert!(!combustion_leaves_ash("minecraft:coal_block", 0.0));
    }

    #[test]
    fn combustion_ash_output_stacks_and_obeys_slot_capacity() {
        let mut slot = None;
        assert!(insert_wood_ash(&mut slot));
        assert_eq!(slot.as_ref().unwrap().item.as_ref(), WOOD_ASH);
        assert_eq!(slot.as_ref().unwrap().count, 1);

        slot.as_mut().unwrap().count = 63;
        assert!(insert_wood_ash(&mut slot));
        assert_eq!(slot.as_ref().unwrap().count, 64);
        assert!(!insert_wood_ash(&mut slot));

        slot = Some(ItemStack {
            item: Arc::from("minecraft:stone"),
            count: 1,
            damage: 0,
            extra: None,
        });
        assert!(!insert_wood_ash(&mut slot));
        assert_eq!(slot.as_ref().unwrap().item.as_ref(), "minecraft:stone");
    }

    #[test]
    fn burner_wood_ash_tag_and_shared_roll_match_arcforge() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../content/data/arcforge/tags/item/leaves_wood_ash.json");
        let tag: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).expect("burner ash tag"))
                .expect("valid burner ash tag");
        assert_eq!(tag["values"], serde_json::json!(["minecraft:charcoal"]));

        let seed = 73;
        let mut expected_random = horizon_client::mod_api::LegacyRandom::new(seed);
        let expected_ash = expected_random.next_float() < COMBUSTION_WOOD_ASH_CHANCE;
        let mut host = TestHost {
            random: horizon_client::mod_api::LegacyRandom::new(seed),
        };
        let mut slot = None;
        assert_eq!(
            try_leave_wood_ash(&mut host, "minecraft:charcoal", &mut slot),
            expected_ash
        );
        assert_eq!(slot.is_some(), expected_ash);

        let mut slot = None;
        assert!(!try_leave_wood_ash(
            &mut host,
            "arcforge:bio_coal",
            &mut slot
        ));
        assert!(slot.is_none());
    }

    #[test]
    fn combustion_plant_rolls_wood_ash_when_charcoal_finishes() {
        let seed = 73;
        let mut expected_random = horizon_client::mod_api::LegacyRandom::new(seed);
        let expected_ash = expected_random.next_float() < COMBUSTION_WOOD_ASH_CHANCE;
        let mut host = TestHost {
            random: horizon_client::mod_api::LegacyRandom::new(seed),
        };
        let mut entity = ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
            id: COMBUSTION.to_owned(),
            items: vec![None, None, None],
            ..Default::default()
        });
        set_i32(&mut entity.nbt, "energy", 0);
        set_i32(&mut entity.nbt, "BurnTime", 1);
        set_i32(&mut entity.nbt, "BurnDuration", 1);
        set_string(&mut entity.nbt, "BurningItem", "minecraft:charcoal");

        tick_combustion_plant(&mut host, [0, 0, 0], &mut entity);
        assert_eq!(entity.items[1].is_some(), expected_ash);
        if expected_ash {
            assert_eq!(entity.items[1].as_ref().unwrap().item.as_ref(), WOOD_ASH);
        }
        assert_eq!(get_string(&entity.nbt, "BurningItem"), None);
    }

    #[test]
    fn combustion_plant_crafting_recipe_and_block_drop_are_present() {
        let data =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/data/arcforge");
        let recipe: serde_json::Value = serde_json::from_slice(
            &std::fs::read(data.join("recipe/crafting/combustion_plant.json"))
                .expect("combustion plant recipe"),
        )
        .expect("valid combustion plant recipe");
        assert_eq!(recipe["type"], "minecraft:crafting_shaped");
        assert_eq!(recipe["pattern"][0], "III");
        assert_eq!(recipe["pattern"][1], "RFR");
        assert_eq!(recipe["pattern"][2], "ICI");
        assert_eq!(recipe["result"]["id"], COMBUSTION);

        let loot: serde_json::Value = serde_json::from_slice(
            &std::fs::read(data.join("loot_table/blocks/combustion_plant.json"))
                .expect("combustion plant loot table"),
        )
        .expect("valid combustion plant loot table");
        assert_eq!(loot["pools"][0]["entries"][0]["name"], COMBUSTION);
        assert_eq!(
            loot["pools"][0]["entries"][0]["modifier"]["source"],
            "block_entity"
        );
    }

    #[test]
    fn combustion_plant_generates_at_double_burn_speed_and_pauses_when_full() {
        let mut entity = ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
            id: COMBUSTION.to_owned(),
            items: vec![
                Some(ItemStack {
                    item: Arc::from("minecraft:coal"),
                    count: 1,
                    damage: 0,
                    extra: None,
                }),
                None,
                None,
            ],
            ..Default::default()
        });
        set_i32(
            &mut entity.nbt,
            "energy",
            COMBUSTION_CAPACITY_FE as i32 - 10,
        );

        let mut host = TestHost {
            random: horizon_client::mod_api::LegacyRandom::new(0),
        };
        tick_combustion_plant(&mut host, [0, 0, 0], &mut entity);
        assert_eq!(
            get_i32(&entity.nbt, "energy"),
            COMBUSTION_CAPACITY_FE as i32
        );
        assert_eq!(get_i32(&entity.nbt, "BurnTime"), 799);
        assert_eq!(get_i32(&entity.nbt, "BurnDuration"), 800);
        assert_eq!(entity.items[0], None);

        tick_combustion_plant(&mut host, [0, 0, 0], &mut entity);
        assert_eq!(get_i32(&entity.nbt, "BurnTime"), 799);
        assert_eq!(
            get_i32(&entity.nbt, "energy"),
            COMBUSTION_CAPACITY_FE as i32
        );

        set_i32(&mut entity.nbt, "energy", 49_960);
        tick_combustion_plant(&mut host, [0, 0, 0], &mut entity);
        assert_eq!(get_i32(&entity.nbt, "BurnTime"), 798);
        assert_eq!(get_i32(&entity.nbt, "energy"), 50_000);
    }

    #[test]
    fn combustion_plant_pushes_energy_to_adjacent_consumers_with_backpressure() {
        let source_pos = [0, 64, 0];
        let adjacent_pos = [1, 64, 0];
        let mut host = EnergyPushTestHost {
            random: horizon_client::mod_api::LegacyRandom::new(0),
            adjacent_pos,
            adjacent: ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
                id: METAL_PRESS.to_owned(),
                nbt: {
                    let mut nbt = horizon_client::mod_api::Compound::default();
                    set_i32(&mut nbt, "energy", 19_900);
                    nbt
                },
                ..Default::default()
            }),
        };
        let mut source = ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
            id: COMBUSTION.to_owned(),
            items: vec![None, None, None],
            ..Default::default()
        });
        set_i32(&mut source.nbt, "energy", 500);

        tick_combustion_plant(&mut host, source_pos, &mut source);
        assert_eq!(get_i32(&host.adjacent.nbt, "energy"), 20_000);
        assert_eq!(get_i32(&source.nbt, "energy"), 400);

        tick_combustion_plant(&mut host, source_pos, &mut source);
        assert_eq!(get_i32(&host.adjacent.nbt, "energy"), 20_000);
        assert_eq!(get_i32(&source.nbt, "energy"), 400);
    }

    #[test]
    fn wrought_conduit_relays_energy_at_machine_rate_and_respects_backpressure() {
        let mut host = EnergyPushTestHost {
            random: horizon_client::mod_api::LegacyRandom::new(0),
            adjacent_pos: [1, 0, 0],
            adjacent: ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
                id: METAL_PRESS.to_owned(),
                nbt: {
                    let mut nbt = horizon_client::mod_api::Compound::default();
                    set_i32(&mut nbt, "energy", 19_900);
                    nbt
                },
                ..Default::default()
            }),
        };
        let mut conduit = ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
            id: WROUGHT_ENERGY_CONDUIT.to_owned(),
            ..Default::default()
        });
        set_i32(&mut conduit.nbt, "energy", 500);

        tick_arcforge_energy_conduit(&mut host, [0, 0, 0], &mut conduit);
        assert_eq!(get_i32(&host.adjacent.nbt, "energy"), 20_000);
        assert_eq!(get_i32(&conduit.nbt, "energy"), 400);

        tick_arcforge_energy_conduit(&mut host, [0, 0, 0], &mut conduit);
        assert_eq!(get_i32(&host.adjacent.nbt, "energy"), 20_000);
        assert_eq!(get_i32(&conduit.nbt, "energy"), 400);
    }

    #[test]
    fn wrought_conduit_recipe_and_drop_data_are_present() {
        let root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/data/arcforge");
        for (id, count) in [
            (WROUGHT_ENERGY_CONDUIT, 8),
            (TEMPERED_ENERGY_CONDUIT, 8),
            (HARDENED_ENERGY_CONDUIT, 8),
            (ARCFORGED_ENERGY_CONDUIT, 6),
        ] {
            let name = id.strip_prefix("arcforge:").expect("Arcforge id");
            let recipe: serde_json::Value = serde_json::from_slice(
                &std::fs::read(root.join(format!("recipe/crafting/{name}.json")))
                    .unwrap_or_else(|_| panic!("{name} recipe")),
            )
            .unwrap_or_else(|_| panic!("valid {name} recipe"));
            assert_eq!(recipe["result"]["id"], id);
            assert_eq!(recipe["result"]["count"], count);
            let loot: serde_json::Value = serde_json::from_slice(
                &std::fs::read(root.join(format!("loot_table/blocks/{name}.json")))
                    .unwrap_or_else(|_| panic!("{name} drops")),
            )
            .unwrap_or_else(|_| panic!("valid {name} drops"));
            assert!(loot.to_string().contains(id));
        }
    }

    #[test]
    fn conduit_tiers_keep_arcforge_throughput_rates() {
        assert_eq!(
            energy_conduit_spec(WROUGHT_ENERGY_CONDUIT),
            Some((ENERGY_CONDUIT_CAPACITY_FE, 256))
        );
        assert_eq!(
            energy_conduit_spec(TEMPERED_ENERGY_CONDUIT),
            Some((ENERGY_CONDUIT_CAPACITY_FE, 1_024))
        );
        assert_eq!(
            energy_conduit_spec(HARDENED_ENERGY_CONDUIT),
            Some((ENERGY_CONDUIT_CAPACITY_FE, 8_192))
        );
        assert_eq!(
            energy_conduit_spec(ARCFORGED_ENERGY_CONDUIT),
            Some((ENERGY_CONDUIT_CAPACITY_FE, 262_144))
        );
        assert_eq!(
            energy_conduit_spec("arcforge:superconducting_energy_conduit"),
            None
        );
    }

    #[test]
    fn conduit_side_configuration_cycles_and_maps_sources_and_sinks() {
        assert_eq!(next_conduit_setting("auto"), "input");
        assert_eq!(next_conduit_setting("input"), "output");
        assert_eq!(next_conduit_setting("output"), "disabled");
        assert_eq!(next_conduit_setting("disabled"), "auto");
        assert_eq!(conduit_connection_for("auto", Some(COMBUSTION)), "output");
        assert_eq!(conduit_connection_for("auto", Some(METAL_PRESS)), "input");
        assert_eq!(
            conduit_connection_for("output", Some(METAL_PRESS)),
            "output"
        );
        assert_eq!(
            conduit_connection_for("disabled", Some(METAL_PRESS)),
            "none"
        );
        let north = energy_conduit_side([0, 0, -1]).expect("north face");
        assert_eq!(north.name, "north");
        assert_eq!(north.offset, [0, 0, -1]);
        assert_eq!(north.opposite, [0, 0, 1]);
    }

    #[test]
    fn source_output_obeys_configured_conduit_face_mode() {
        let mut host = EnergyPushTestHost {
            random: horizon_client::mod_api::LegacyRandom::new(0),
            adjacent_pos: [1, 0, 0],
            adjacent: ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
                id: WROUGHT_ENERGY_CONDUIT.to_owned(),
                nbt: {
                    let mut nbt = horizon_client::mod_api::Compound::default();
                    set_string(&mut nbt, "setting_west", "disabled");
                    nbt
                },
                ..Default::default()
            }),
        };
        let mut source = energy::EnergyStore::new(1000, 500);

        push_adjacent_arcforge_energy(&mut host, [0, 0, 0], &mut source, 200, false);
        assert_eq!(source.stored_fe(), 500);
        assert_eq!(get_i32(&host.adjacent.nbt, "energy"), 0);

        set_string(&mut host.adjacent.nbt, "setting_west", "output");
        push_adjacent_arcforge_energy(&mut host, [0, 0, 0], &mut source, 200, false);
        assert_eq!(source.stored_fe(), 300);
        assert_eq!(get_i32(&host.adjacent.nbt, "energy"), 200);
    }

    #[test]
    fn firebox_transfers_burning_source_heat_to_a_thermoelectric_plant() {
        let mut host = EnergyPushTestHost {
            random: horizon_client::mod_api::LegacyRandom::new(0),
            adjacent_pos: [1, 0, 0],
            adjacent: ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
                id: THERMOELECTRIC_PLANT.to_owned(),
                ..Default::default()
            }),
        };
        let mut firebox = ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
            id: FIREBOX.to_owned(),
            items: vec![
                Some(ItemStack {
                    item: Arc::from("minecraft:coal"),
                    count: 1,
                    damage: 0,
                    extra: None,
                }),
                None,
            ],
            ..Default::default()
        });

        tick_firebox(&mut host, [0, 0, 0], &mut firebox);

        assert_eq!(get_i32(&host.adjacent.nbt, "HeatHU"), 80);
        assert_eq!(get_i32(&firebox.nbt, "HeatHU"), 0);
        assert_eq!(get_i32(&firebox.nbt, "BurnTime"), 799);
    }

    #[test]
    fn fuel_burner_matches_creosote_rate_and_retains_half_millibucket_tick() {
        let mut host = EnergyPushTestHost {
            random: horizon_client::mod_api::LegacyRandom::new(0),
            adjacent_pos: [1, 0, 0],
            adjacent: ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
                id: "minecraft:air".to_owned(),
                ..Default::default()
            }),
        };
        let mut burner = ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
            id: FUEL_BURNER.to_owned(),
            items: vec![None, None],
            ..Default::default()
        });
        assert_eq!(
            fluid::insert(
                &mut burner.nbt,
                0,
                "arcforge:creosote",
                1,
                FUEL_BURNER_TANK_CAPACITY
            ),
            1
        );

        tick_fuel_burner(&mut host, [0, 0, 0], &mut burner);
        assert_eq!(get_i32(&burner.nbt, "HeatHU"), 60);
        assert_eq!(get_i32(&burner.nbt, "FuelTakenMilli"), 500);
        assert_eq!(fluid::amount(&burner.nbt, 0), 0);
        assert_eq!(get_i32(&burner.nbt, "TemperatureC"), 850);

        tick_fuel_burner(&mut host, [0, 0, 0], &mut burner);
        assert_eq!(get_i32(&burner.nbt, "HeatHU"), 120);
        assert_eq!(get_i32(&burner.nbt, "FuelTakenMilli"), 0);
        assert_eq!(get_i32(&burner.nbt, "TemperatureC"), 850);

        tick_fuel_burner(&mut host, [0, 0, 0], &mut burner);
        assert_eq!(get_i32(&burner.nbt, "TemperatureC"), 24);
    }

    #[test]
    fn fuel_burner_bucket_slot_accepts_creosote_and_returns_empty_bucket() {
        let mut burner = ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
            id: FUEL_BURNER.to_owned(),
            items: vec![Some(creosote_bucket(1000)), None],
            ..Default::default()
        });
        tick_fuel_burner_buckets(&mut burner);
        assert_eq!(fluid::amount(&burner.nbt, 0), 1000);
        assert_eq!(burner.items[0], None);
        assert_eq!(
            burner.items[1].as_ref().unwrap().item.as_ref(),
            "minecraft:bucket"
        );
    }

    #[test]
    fn fluid_tank_tiers_match_source_capacities_and_have_upgrade_recipes() {
        assert_eq!(
            [
                WROUGHT_FLUID_TANK_CAPACITY,
                TEMPERED_FLUID_TANK_CAPACITY,
                HARDENED_FLUID_TANK_CAPACITY,
                ARCFORGED_FLUID_TANK_CAPACITY,
            ],
            [16_000, 64_000, 256_000, 1_024_000]
        );
        for (file, output) in [
            ("tempered_fluid_tank.json", TEMPERED_FLUID_TANK),
            ("hardened_fluid_tank.json", HARDENED_FLUID_TANK),
            ("arcforged_fluid_tank.json", ARCFORGED_FLUID_TANK),
        ] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../content/data/arcforge/recipe/crafting")
                .join(file);
            let recipe: serde_json::Value =
                serde_json::from_slice(&std::fs::read(path).expect("fluid tank tier recipe"))
                    .expect("valid fluid tank tier recipe");
            assert_eq!(recipe["type"], "minecraft:crafting_shaped");
            assert_eq!(recipe["result"]["id"], output);
            assert!(recipe.get("copy_components_from").is_none());
        }
    }

    #[test]
    fn thermoelectric_plant_converts_heat_at_source_efficiency_and_respects_fe_backpressure() {
        assert_eq!(thermoelectric_efficiency(100), 0.0);
        assert_eq!(thermoelectric_efficiency(600), 0.5);
        assert_eq!(thermoelectric_efficiency(1100), 1.0);
        assert!((thermoelectric_efficiency(1250) - 1.075).abs() < f64::EPSILON);
        assert_eq!(thermoelectric_efficiency(1400), 1.15);
        assert_eq!(thermoelectric_efficiency(1600), 1.15);
        assert_eq!(thermoelectric_heat_throughput(), 102);

        let mut heat_at_full_efficiency = heat::HeatStore::new(
            THERMOELECTRIC_HEAT_CAPACITY,
            THERMOELECTRIC_MAX_TEMPERATURE,
            15_653,
        );
        assert_eq!(heat_at_full_efficiency.temperature_c(), 1100);
        let mut energy = energy::EnergyStore::new(THERMOELECTRIC_FE_CAPACITY, 0);
        assert_eq!(
            thermoelectric_convert(&mut heat_at_full_efficiency, &mut energy, 0, 0),
            (80, 80)
        );
        assert_eq!(energy.stored_fe(), 80);

        let mut full_heat = heat::HeatStore::new(
            THERMOELECTRIC_HEAT_CAPACITY,
            THERMOELECTRIC_MAX_TEMPERATURE,
            THERMOELECTRIC_HEAT_CAPACITY,
        );
        let mut nearly_full_energy = energy::EnergyStore::new(THERMOELECTRIC_FE_CAPACITY, 49_999);
        assert_eq!(
            thermoelectric_convert(&mut full_heat, &mut nearly_full_energy, 0, 0),
            (0, 0)
        );
        assert_eq!(full_heat.stored_hu(), THERMOELECTRIC_HEAT_CAPACITY);
        assert_eq!(nearly_full_energy.stored_fe(), 49_999);
    }

    #[test]
    fn thermoelectric_plant_and_thermocouple_crafting_and_drops_are_present() {
        let data =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/data/arcforge");
        let thermocouple: serde_json::Value = serde_json::from_slice(
            &std::fs::read(data.join("recipe/crafting/thermocouple.json"))
                .expect("thermocouple recipe"),
        )
        .expect("valid thermocouple recipe");
        assert_eq!(thermocouple["type"], "minecraft:crafting_shaped");
        assert_eq!(thermocouple["result"]["id"], "arcforge:thermocouple");

        let plant: serde_json::Value = serde_json::from_slice(
            &std::fs::read(data.join("recipe/crafting/thermoelectric_plant.json"))
                .expect("thermoelectric plant recipe"),
        )
        .expect("valid thermoelectric plant recipe");
        assert_eq!(plant["type"], "minecraft:crafting_shaped");
        assert_eq!(plant["key"]["F"], FIREBOX);
        assert_eq!(plant["key"]["t"], "arcforge:thermocouple");
        assert_eq!(plant["result"]["id"], THERMOELECTRIC_PLANT);

        let loot: serde_json::Value = serde_json::from_slice(
            &std::fs::read(data.join("loot_table/blocks/thermoelectric_plant.json"))
                .expect("thermoelectric plant loot"),
        )
        .expect("valid thermoelectric plant loot");
        assert_eq!(loot["pools"][0]["entries"][0]["name"], THERMOELECTRIC_PLANT);
    }

    #[test]
    fn resin_tap_caps_pine_resin_at_the_upstream_limit() {
        let mut entity = ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
            id: RESIN_TAP.to_owned(),
            items: vec![None],
            ..Default::default()
        });
        for _ in 0..RESIN_TAP_MAX_RESIN {
            assert!(insert_resin_tap_resin(&mut entity));
        }
        assert_eq!(entity.items[0].as_ref().unwrap().item.as_ref(), PINE_RESIN);
        assert_eq!(entity.items[0].as_ref().unwrap().count, 16);
        assert!(!insert_resin_tap_resin(&mut entity));

        entity.items[0] = Some(ItemStack {
            item: Arc::from("minecraft:stone"),
            count: 1,
            damage: 0,
            extra: None,
        });
        assert!(!insert_resin_tap_resin(&mut entity));
    }

    struct ResinTapInteractionTestHost {
        entity: ModBlockEntity,
        inventory_room: u32,
        given: u32,
        held: Option<(String, u32, bool)>,
        dropped: Vec<(String, [f64; 3], horizon_client::mod_api::Compound)>,
        summon_success: bool,
    }

    impl Host for ResinTapInteractionTestHost {
        fn dimension(&self) -> &'static str {
            "minecraft:overworld"
        }

        fn held(&self) -> Option<(String, u32, bool)> {
            self.held.clone()
        }

        fn summon(
            &mut self,
            dimension: &str,
            kind: &str,
            pos: [f64; 3],
            nbt: horizon_client::mod_api::Compound,
        ) -> Option<u32> {
            assert_eq!(dimension, "minecraft:overworld");
            self.dropped.push((kind.to_owned(), pos, nbt));
            self.summon_success.then_some(1)
        }

        fn room_for(&mut self, item: &str) -> u32 {
            if item == PINE_RESIN {
                self.inventory_room
            } else {
                0
            }
        }

        fn give_items(&mut self, item: &str, count: u32) -> u32 {
            if item != PINE_RESIN {
                return 0;
            }
            let given = count.min(self.inventory_room);
            self.inventory_room -= given;
            self.given += given;
            given
        }

        fn mod_block_entity(&self, _pos: [i32; 3]) -> Option<ModBlockEntity> {
            Some(self.entity.clone())
        }

        fn change_mod_block_entity(
            &mut self,
            _pos: [i32; 3],
            change: &mut dyn FnMut(&mut ModBlockEntity),
        ) -> bool {
            change(&mut self.entity);
            true
        }
    }

    fn resin_tap_interaction_host(
        resin_count: u32,
        inventory_room: u32,
        held: Option<(String, u32, bool)>,
    ) -> ResinTapInteractionTestHost {
        ResinTapInteractionTestHost {
            entity: ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
                id: RESIN_TAP.to_owned(),
                items: vec![Some(ItemStack {
                    item: Arc::from(PINE_RESIN),
                    count: resin_count,
                    damage: 0,
                    extra: None,
                })],
                ..Default::default()
            }),
            inventory_room,
            given: 0,
            held,
            dropped: Vec::new(),
            summon_success: true,
        }
    }

    #[test]
    fn empty_hand_collects_resin_and_drops_inventory_overflow() {
        let mut host = resin_tap_interaction_host(8, 3, None);
        let mut canceled = false;
        horizon_client::mod_api::dispatch_right_click_block(
            &mut host,
            &[Box::new(Arcforge)],
            [0, 64, 0],
            [0, 1, 0],
            &mut canceled,
        );

        assert!(canceled);
        assert_eq!(host.given, 3);
        assert_eq!(host.entity.items[0], None);
        assert_eq!(host.dropped.len(), 1);
        assert_eq!(host.dropped[0].0, "minecraft:item");
        assert_eq!(host.dropped[0].1, [0.5, 64.5, 0.5]);
        assert_eq!(
            host.dropped[0].2.get(&"Item".into()),
            Some(&Tag::compound([
                ("id", Tag::String(PINE_RESIN.into())),
                ("count", Tag::Int(5)),
            ]))
        );
        assert_eq!(
            host.dropped[0].2.get(&"PickupDelay".into()),
            Some(&Tag::Short(10))
        );
    }

    #[test]
    fn empty_hand_collection_restores_resin_when_item_spawn_fails() {
        let mut host = resin_tap_interaction_host(8, 3, None);
        host.summon_success = false;
        let mut canceled = false;
        horizon_client::mod_api::dispatch_right_click_block(
            &mut host,
            &[Box::new(Arcforge)],
            [0, 64, 0],
            [0, 1, 0],
            &mut canceled,
        );

        assert!(canceled);
        assert_eq!(host.given, 3);
        assert_eq!(host.entity.items[0].as_ref().unwrap().count, 5);
    }

    #[test]
    fn resin_tap_does_not_collect_with_an_item_in_hand() {
        let mut host = resin_tap_interaction_host(8, 8, Some(("minecraft:stick".into(), 1, false)));
        let mut canceled = false;
        horizon_client::mod_api::dispatch_right_click_block(
            &mut host,
            &[Box::new(Arcforge)],
            [0, 64, 0],
            [0, 1, 0],
            &mut canceled,
        );

        assert!(!canceled);
        assert_eq!(host.given, 0);
        assert_eq!(host.entity.items[0].as_ref().unwrap().count, 8);
    }

    struct ResinTapBucketTestHost {
        entity: ModBlockEntity,
        inventory_item: String,
        inventory_count: u32,
        given: Vec<ItemStack>,
        reject_stack: bool,
    }

    impl Host for ResinTapBucketTestHost {
        fn mod_block_entity(&self, _pos: [i32; 3]) -> Option<ModBlockEntity> {
            Some(self.entity.clone())
        }

        fn change_mod_block_entity(
            &mut self,
            _pos: [i32; 3],
            change: &mut dyn FnMut(&mut ModBlockEntity),
        ) -> bool {
            change(&mut self.entity);
            true
        }

        fn take_items(&mut self, item: &str, count: u32) -> u32 {
            if self.inventory_item != item {
                return 0;
            }
            let taken = count.min(self.inventory_count);
            self.inventory_count -= taken;
            taken
        }

        fn give_items(&mut self, item: &str, count: u32) -> u32 {
            if item != "minecraft:bucket" {
                return 0;
            }
            self.inventory_item = item.to_owned();
            self.inventory_count += count;
            count
        }

        fn give_stack(&mut self, uuid: u128, stack: ItemStack) -> Option<ItemStack> {
            assert_eq!(uuid, 7);
            if self.reject_stack {
                return Some(stack);
            }
            self.given.push(stack);
            None
        }
    }

    fn resin_tap_bucket_test_host(fluid_amount: u32, item: &str) -> ResinTapBucketTestHost {
        let mut nbt = horizon_client::mod_api::Compound::default();
        if fluid_amount > 0 {
            fluid::insert(&mut nbt, 0, "arcforge:latex", fluid_amount, 1000);
        }
        ResinTapBucketTestHost {
            entity: ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
                id: RESIN_TAP.to_owned(),
                nbt,
                ..Default::default()
            }),
            inventory_item: item.to_owned(),
            inventory_count: 1,
            given: Vec::new(),
            reject_stack: false,
        }
    }

    #[test]
    fn resin_tap_fills_latex_bucket_and_preserves_partial_tank_contents() {
        let mut host = resin_tap_bucket_test_host(1000, "minecraft:bucket");
        let empty_bucket = ItemStack {
            item: Arc::from("minecraft:bucket"),
            count: 1,
            damage: 0,
            extra: None,
        };

        assert!(matches!(
            use_resin_tap_bucket(&mut host, [0, 64, 0], &empty_bucket, Some(7)),
            Flow::Cancel
        ));
        assert_eq!(fluid::amount(&host.entity.nbt, 0), 0);
        assert_eq!(host.given.len(), 1);
        assert_eq!(host.given[0], latex_bucket(1000));
        assert_eq!(host.inventory_count, 0);
    }

    #[test]
    fn resin_tap_accepts_latex_bucket_and_returns_empty_bucket() {
        let mut host = resin_tap_bucket_test_host(0, LATEX_BUCKET);
        let filled_bucket = latex_bucket(1000);

        assert!(matches!(
            use_resin_tap_bucket(&mut host, [0, 64, 0], &filled_bucket, Some(7)),
            Flow::Cancel
        ));
        assert_eq!(
            fluid::name(&host.entity.nbt, 0).as_deref(),
            Some("arcforge:latex")
        );
        assert_eq!(fluid::amount(&host.entity.nbt, 0), 1000);
        assert_eq!(host.given.len(), 1);
        assert_eq!(host.given[0].item.as_ref(), "minecraft:bucket");
        assert_eq!(host.inventory_count, 0);
    }

    #[test]
    fn resin_tap_bucket_fill_rolls_back_when_inventory_rejects_output() {
        let mut host = resin_tap_bucket_test_host(1000, "minecraft:bucket");
        host.reject_stack = true;
        let empty_bucket = ItemStack {
            item: Arc::from("minecraft:bucket"),
            count: 1,
            damage: 0,
            extra: None,
        };

        assert!(matches!(
            use_resin_tap_bucket(&mut host, [0, 64, 0], &empty_bucket, Some(7)),
            Flow::Continue
        ));
        assert_eq!(fluid::amount(&host.entity.nbt, 0), 1000);
        assert_eq!(host.inventory_item, "minecraft:bucket");
        assert_eq!(host.inventory_count, 1);
    }

    #[test]
    fn resin_tap_data_and_recipe_match_the_pinned_source() {
        let data =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/data/arcforge");
        let block: serde_json::Value = serde_json::from_slice(
            &std::fs::read(data.join("horizon/block/resin_tap.json")).expect("tap block data"),
        )
        .expect("valid tap block data");
        assert_eq!(block["template"], "minecraft:tripwire_hook");
        assert_eq!(block["state_properties"]["content"]["values"][0], "empty");
        assert_eq!(block["state_properties"]["content"]["values"][1], "latex");
        assert_eq!(block["state_properties"]["content"]["values"][2], "resin");

        let recipe: serde_json::Value = serde_json::from_slice(
            &std::fs::read(data.join("recipe/crafting/resin_tap.json")).expect("tap recipe"),
        )
        .expect("valid tap recipe");
        assert_eq!(recipe["pattern"][0], " SC");
        assert_eq!(recipe["pattern"][1], "P P");
        assert_eq!(recipe["pattern"][2], "PPP");
        assert_eq!(recipe["key"]["C"], "#c:ingots/copper");
        assert_eq!(recipe["key"]["P"], "arcforge:treated_planks");
        assert_eq!(recipe["key"]["S"], "#c:nuggets/iron");
        assert_eq!(recipe["result"]["id"], RESIN_TAP);
        assert_eq!(RESIN_TAP_INTERVAL, 200);
        assert_eq!(RESIN_TAP_LATEX_PER_DRIP, 25);
        assert_eq!(RESIN_TAP_TANK_CAPACITY, 1000);
    }

    #[test]
    fn silver_ore_worldgen_drops_and_smelting_are_connected() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/data");
        let feature: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join("arcforge/worldgen/feature/ore_silver.json"))
                .expect("silver feature"),
        )
        .expect("valid silver feature");
        assert_eq!(feature["type"], "minecraft:ore");
        assert_eq!(feature["size"], 9);
        assert_eq!(feature["targets"][0]["state"], "arcforge:silver_ore");
        assert_eq!(
            feature["targets"][1]["state"],
            "arcforge:deepslate_silver_ore"
        );

        let placed: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join("arcforge/worldgen/placed_feature/ore_silver.json"))
                .expect("silver placement"),
        )
        .expect("valid silver placement");
        assert_eq!(placed["placement"][0]["count"], 8);
        assert_eq!(
            placed["placement"][2]["height"]["min_inclusive"]["absolute"],
            -32
        );
        assert_eq!(
            placed["placement"][2]["height"]["max_inclusive"]["absolute"],
            64
        );

        for name in ["silver_ore", "deepslate_silver_ore"] {
            let loot: serde_json::Value = serde_json::from_slice(
                &std::fs::read(root.join(format!("arcforge/loot_table/blocks/{name}.json")))
                    .expect("silver ore loot table"),
            )
            .expect("valid silver ore loot table");
            let text = loot.to_string();
            assert!(text.contains("minecraft:tool/can_silk_touch"));
            assert!(text.contains("minecraft:fortune"));
            assert!(text.contains("arcforge:raw_silver"));
        }

        let raw_smelting: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join("arcforge/recipe/silver_ingot_from_smelting_raw.json"))
                .expect("raw silver smelting"),
        )
        .expect("valid raw silver recipe");
        assert_eq!(raw_smelting["ingredient"], "#c:raw_materials/silver");
        assert_eq!(raw_smelting["result"]["id"], "arcforge:silver_ingot");
    }

    #[test]
    fn overworld_ore_family_uses_arcforge_defaults_and_drops() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/data");
        let ores = [
            ("nickel", "nickel", 6, 8, -64, 16, "trapezoid", 0.0),
            ("fluorite", "fluorite", 8, 8, -16, 48, "uniform", 0.0),
            ("bismuth", "bismuth", 8, 8, 0, 56, "uniform", 0.0),
            ("tungsten", "wolframite", 6, 6, -64, -16, "uniform", 0.0),
            ("arcite", "arcite", 4, 5, -64, -40, "uniform", 0.2),
            ("spodumene", "spodumene", 3, 7, -48, 48, "trapezoid", 0.0),
            ("uraninite", "uraninite", 2, 4, -64, 24, "trapezoid", 0.0),
            ("galena", "galena", 8, 9, -16, 64, "trapezoid", 0.0),
        ];

        for (name, block, count, size, min_y, max_y, shape, discard) in ores {
            let configured: serde_json::Value = serde_json::from_slice(
                &std::fs::read(root.join(format!("arcforge/worldgen/feature/ore_{name}.json")))
                    .expect("configured ore feature"),
            )
            .expect("valid configured ore feature");
            assert_eq!(configured["type"], "minecraft:ore");
            assert_eq!(configured["size"], size);
            assert_eq!(configured["discard_chance_on_air_exposure"], discard);
            assert_eq!(configured["targets"].as_array().unwrap().len(), 2);

            let placed: serde_json::Value = serde_json::from_slice(
                &std::fs::read(
                    root.join(format!("arcforge/worldgen/placed_feature/ore_{name}.json")),
                )
                .expect("placed ore feature"),
            )
            .expect("valid placed ore feature");
            assert_eq!(placed["placement"][0]["count"], count);
            assert_eq!(
                placed["placement"][2]["height"]["type"],
                format!("minecraft:{shape}")
            );
            assert_eq!(
                placed["placement"][2]["height"]["min_inclusive"]["absolute"],
                min_y
            );
            assert_eq!(
                placed["placement"][2]["height"]["max_inclusive"]["absolute"],
                max_y
            );

            let modifier: serde_json::Value = serde_json::from_slice(
                &std::fs::read(
                    root.join(format!("arcforge/neoforge/biome_modifier/ore_{name}.json")),
                )
                .expect("ore biome modifier"),
            )
            .expect("valid ore biome modifier");
            assert_eq!(modifier["features"], format!("arcforge:ore_{name}"));
            assert_eq!(modifier["step"], "underground_ores");

            for block in [format!("{block}_ore"), format!("deepslate_{block}_ore")] {
                let loot: serde_json::Value = serde_json::from_slice(
                    &std::fs::read(root.join(format!("arcforge/loot_table/blocks/{block}.json")))
                        .expect("ore loot table"),
                )
                .expect("valid ore loot table");
                let loot_text = loot.to_string();
                assert!(loot_text.contains("minecraft:tool/can_silk_touch"));
                assert!(loot_text.contains("minecraft:fortune"));
            }
        }

        for recipe in [
            "nickel_ingot_from_smelting_ore",
            "nickel_ingot_from_smelting_raw",
            "fluorite_crystal_from_smelting_ore",
            "fluorite_crystal_from_smelting_raw",
            "bismuth_ingot_from_smelting_ore",
            "bismuth_ingot_from_smelting_raw",
            "tungsten_ingot_from_smelting_ore",
            "tungsten_ingot_from_smelting_raw",
            "arcite_crystal_from_smelting_ore",
            "arcite_crystal_from_smelting_raw",
            "lead_ingot_from_smelting_ore",
            "lead_ingot_from_smelting_raw",
        ] {
            let path = root.join(format!("arcforge/recipe/{recipe}.json"));
            let data: serde_json::Value =
                serde_json::from_slice(&std::fs::read(path).expect("ore smelting recipe"))
                    .expect("valid ore smelting recipe");
            assert_eq!(data["type"], "minecraft:smelting");
        }
    }

    #[test]
    fn special_ore_worldgen_covers_nether_end_and_surface_deposits() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/data");
        let features = [
            (
                "halite",
                "ore_halite",
                1,
                12,
                -32,
                40,
                "uniform",
                "minecraft:is_overworld",
            ),
            (
                "pegmatite",
                "ore_spodumene_pegmatite",
                6,
                10,
                0,
                160,
                "trapezoid",
                "minecraft:is_mountain",
            ),
            (
                "sulfur",
                "ore_nether_sulfur",
                12,
                10,
                10,
                117,
                "uniform",
                "minecraft:is_nether",
            ),
            (
                "iolite",
                "ore_iolite",
                3,
                6,
                0,
                80,
                "uniform",
                "minecraft:the_end",
            ),
            (
                "iolite_outer",
                "ore_iolite_outer",
                8,
                7,
                0,
                80,
                "uniform",
                "minecraft:end_highlands",
            ),
            (
                "heavy_mineral_sand",
                "ore_heavy_mineral_sand",
                5,
                14,
                40,
                90,
                "uniform",
                "arcforge:has_heavy_mineral_sand",
            ),
        ];

        for (name, feature_name, count, size, min_y, max_y, shape, biome) in features {
            let configured: serde_json::Value = serde_json::from_slice(
                &std::fs::read(root.join(format!("arcforge/worldgen/feature/{feature_name}.json")))
                    .expect("special configured feature"),
            )
            .expect("valid special configured feature");
            assert_eq!(configured["type"], "minecraft:ore");
            assert_eq!(configured["size"], size);

            let placed: serde_json::Value = serde_json::from_slice(
                &std::fs::read(root.join(format!(
                    "arcforge/worldgen/placed_feature/{feature_name}.json"
                )))
                .expect("special placed feature"),
            )
            .expect("valid special placed feature");
            let placements = placed["placement"].as_array().expect("placement list");
            let count_placement = placements
                .iter()
                .find(|placement| placement["type"] == "minecraft:count")
                .expect("count placement");
            assert_eq!(count_placement["count"], count);
            let height = placements
                .iter()
                .find(|placement| placement["type"] == "minecraft:height_range")
                .expect("height placement");
            assert_eq!(height["height"]["type"], format!("minecraft:{shape}"));
            assert_eq!(height["height"]["min_inclusive"]["absolute"], min_y);
            assert_eq!(height["height"]["max_inclusive"]["absolute"], max_y);

            let modifiers: Vec<(String, String)> = match name {
                "pegmatite" => vec![
                    (
                        "ore_spodumene_pegmatite_mountains".into(),
                        "#minecraft:is_mountain".into(),
                    ),
                    (
                        "ore_spodumene_pegmatite_badlands".into(),
                        "#minecraft:is_badlands".into(),
                    ),
                ],
                "iolite_outer" => vec![
                    (
                        "ore_iolite_outer_highlands".into(),
                        "minecraft:end_highlands".into(),
                    ),
                    (
                        "ore_iolite_outer_midlands".into(),
                        "minecraft:end_midlands".into(),
                    ),
                    (
                        "ore_iolite_outer_barrens".into(),
                        "minecraft:end_barrens".into(),
                    ),
                    (
                        "ore_iolite_outer_small_islands".into(),
                        "minecraft:small_end_islands".into(),
                    ),
                ],
                _ => {
                    let biome = if biome.starts_with("minecraft:is_")
                        || biome.starts_with("arcforge:has_")
                    {
                        format!("#{biome}")
                    } else {
                        biome.to_owned()
                    };
                    vec![(feature_name.to_owned(), biome)]
                }
            };
            for (modifier_name, expected_biome) in modifiers {
                let modifier: serde_json::Value = serde_json::from_slice(
                    &std::fs::read(root.join(format!(
                        "arcforge/neoforge/biome_modifier/{modifier_name}.json"
                    )))
                    .expect("special biome modifier"),
                )
                .expect("valid special biome modifier");
                assert_eq!(modifier["features"], format!("arcforge:{feature_name}"));
                let selected_biomes = &modifier["biomes"];
                assert!(
                    selected_biomes.as_str() == Some(expected_biome.as_str())
                        || selected_biomes.as_array().is_some_and(|values| {
                            values
                                .iter()
                                .any(|value| value.as_str() == Some(expected_biome.as_str()))
                        })
                );
            }
        }

        let halite =
            std::fs::read_to_string(root.join("arcforge/worldgen/placed_feature/ore_halite.json"))
                .expect("halite placement");
        assert!(halite.contains("minecraft:rarity_filter"));
        assert!(halite.contains("\"chance\": 4"));

        for (block, drop) in [
            ("halite_ore", "arcforge:rock_salt"),
            ("deepslate_halite_ore", "arcforge:rock_salt"),
            ("nether_sulfur_ore", "arcforge:sulfur_dust"),
            ("iolite_ore", "arcforge:iolite_crystal"),
            ("heavy_mineral_sand", "arcforge:heavy_mineral_sand"),
        ] {
            let loot: serde_json::Value = serde_json::from_slice(
                &std::fs::read(root.join(format!("arcforge/loot_table/blocks/{block}.json")))
                    .expect("special ore loot table"),
            )
            .expect("valid special ore loot table");
            assert!(loot.to_string().contains(drop));
        }
    }

    #[test]
    fn ore_raw_material_storage_blocks_pack_and_unpack() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/data");
        let forms = [
            (
                "arcite",
                "raw_arcite_block",
                "raw_arcite_from_raw_arcite_block",
                "raw_arcite",
                "raw_arcite",
            ),
            (
                "bismuth",
                "raw_bismuth_block",
                "raw_bismuth_from_raw_bismuth_block",
                "raw_bismuth",
                "raw_bismuth",
            ),
            (
                "fluorite",
                "raw_fluorite_block",
                "raw_fluorite_from_raw_fluorite_block",
                "raw_fluorite",
                "raw_fluorite",
            ),
            (
                "lead",
                "raw_galena_block",
                "raw_galena_from_raw_galena_block",
                "raw_galena",
                "raw_lead",
            ),
            (
                "nickel",
                "raw_nickel_block",
                "raw_nickel_from_raw_nickel_block",
                "raw_nickel",
                "raw_nickel",
            ),
            (
                "silver",
                "raw_silver_block",
                "raw_silver_from_raw_silver_block",
                "raw_silver",
                "raw_silver",
            ),
            (
                "spodumene",
                "raw_spodumene_block",
                "raw_spodumene_from_raw_spodumene_block",
                "raw_spodumene",
                "raw_spodumene",
            ),
            (
                "uranium",
                "raw_uraninite_block",
                "raw_uraninite_from_raw_uraninite_block",
                "raw_uraninite",
                "raw_uranium",
            ),
            (
                "tungsten",
                "raw_wolframite_block",
                "raw_wolframite_from_raw_wolframite_block",
                "raw_wolframite",
                "raw_tungsten",
            ),
            (
                "rock_salt",
                "raw_rock_salt_block",
                "rock_salt_from_raw_rock_salt_block",
                "rock_salt",
                "raw_rock_salt",
            ),
        ];

        for (tag, block, unpack, item, storage_tag) in forms {
            let pack_name = format!("{block}");
            let pack: serde_json::Value = serde_json::from_slice(
                &std::fs::read(root.join(format!("arcforge/recipe/crafting/{pack_name}.json")))
                    .expect("raw-material packing recipe"),
            )
            .expect("valid raw-material packing recipe");
            assert_eq!(pack["type"], "minecraft:crafting_shaped");
            assert_eq!(pack["key"]["#"], format!("#c:raw_materials/{tag}"));
            assert_eq!(pack["result"]["id"], format!("arcforge:{block}"));
            assert_eq!(pack["result"]["count"], 1);
            assert_eq!(pack["pattern"].as_array().unwrap().len(), 3);

            let unpack_recipe: serde_json::Value = serde_json::from_slice(
                &std::fs::read(root.join(format!("arcforge/recipe/crafting/{unpack}.json")))
                    .expect("raw-material unpacking recipe"),
            )
            .expect("valid raw-material unpacking recipe");
            assert_eq!(unpack_recipe["type"], "minecraft:crafting_shapeless");
            assert_eq!(
                unpack_recipe["ingredients"][0],
                format!("#c:storage_blocks/{storage_tag}")
            );
            assert_eq!(unpack_recipe["result"]["id"], format!("arcforge:{item}"));
            assert_eq!(unpack_recipe["result"]["count"], 9);

            let raw_tag: serde_json::Value = serde_json::from_slice(
                &std::fs::read(root.join(format!("c/tags/item/raw_materials/{tag}.json")))
                    .expect("raw material tag"),
            )
            .expect("valid raw material tag");
            assert!(
                raw_tag["values"]
                    .as_array()
                    .unwrap()
                    .contains(&format!("arcforge:{item}").into())
            );

            for kind in ["block", "item"] {
                let storage: serde_json::Value = serde_json::from_slice(
                    &std::fs::read(
                        root.join(format!("c/tags/{kind}/storage_blocks/{storage_tag}.json")),
                    )
                    .expect("storage block tag"),
                )
                .expect("valid storage block tag");
                assert!(
                    storage["values"]
                        .as_array()
                        .unwrap()
                        .contains(&format!("arcforge:{block}").into())
                );
            }

            assert!(
                crushing_catalog::BLOCKS
                    .iter()
                    .any(|(registered, _)| *registered == format!("arcforge:{block}"))
            );
            assert!(
                crushing_catalog::ITEMS
                    .iter()
                    .any(|(registered, _)| *registered == format!("arcforge:{item}"))
            );
        }
    }

    #[test]
    fn infuser_recipe_data_covers_upstream_recipes_and_outputs() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../content/data/arcforge/recipe/infusing");
        let files = std::fs::read_dir(&dir)
            .expect("Arcforge infusing recipe directory")
            .map(|entry| entry.expect("recipe file").path())
            .collect::<Vec<_>>();
        assert_eq!(files.len(), 10);
        let outputs: std::collections::HashSet<&str> = [
            "arcforge:treated_log",
            "arcforge:stripped_treated_log",
            "arcforge:treated_wood",
            "arcforge:stripped_treated_wood",
            "arcforge:treated_planks",
        ]
        .into_iter()
        .collect();
        let mut registered: std::collections::HashSet<String> = [
            "arcforge:treated_log",
            "arcforge:stripped_treated_log",
            "arcforge:treated_wood",
            "arcforge:stripped_treated_wood",
            "arcforge:treated_planks",
            PINE_RESIN,
            INFUSER,
            CREOSOTE_BUCKET,
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        registered.extend([STEEL, COKE, SLAG].into_iter().map(str::to_owned));
        let mut fluids = 0;
        let mut additives = 0;
        for path in files {
            let value: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&path).expect("read infusing recipe"))
                    .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            assert_eq!(value["type"], "arcforge:infusing", "{}", path.display());
            let result = value["result"]["id"].as_str().expect("result id");
            assert!(outputs.contains(result), "unknown output {result}");
            assert!(registered.contains(result), "unregistered output {result}");
            assert!(value["time"].as_u64().is_some_and(|time| time > 0));
            if value.get("fluid").is_some() {
                assert_eq!(value["fluid"]["id"], "arcforge:creosote");
                assert!(value["fluid"]["amount"].as_u64().is_some_and(|n| n > 0));
                fluids += 1;
            }
            if value.get("additive").is_some() {
                assert_eq!(value["additive"]["ingredient"], PINE_RESIN);
                assert!(value["additive"]["count"].as_u64().is_some_and(|n| n > 0));
                additives += 1;
            }
            assert_ne!(
                value.get("fluid").is_some(),
                value.get("additive").is_some()
            );
        }
        assert_eq!(fluids, 5);
        assert_eq!(additives, 5);
    }

    #[test]
    fn infuser_bucket_round_trips_custom_fluid_and_stacks_only_when_equal() {
        let bucket = creosote_bucket(1000);
        assert_eq!(
            item_fluid(&bucket),
            Some(("arcforge:creosote".to_owned(), 1000))
        );
        assert!(bucket_output_fits(None, &bucket));
        assert!(!bucket_output_fits(Some(&bucket), &creosote_bucket(500)));
    }

    #[test]
    fn arcforge_furnace_recipes_cover_upstream_smelting_data() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../content/data/arcforge/recipe/arcforge_smelting");
        let mut files = std::fs::read_dir(&dir)
            .expect("Arcforge smelting recipe directory")
            .map(|entry| entry.expect("recipe file").path())
            .collect::<Vec<_>>();
        files.sort();
        assert_eq!(files.len(), 12);
        let mut registered: std::collections::HashSet<String> = crushing_catalog::BLOCKS
            .iter()
            .chain(crushing_catalog::ITEMS)
            .map(|(id, _)| (*id).to_owned())
            .collect();
        registered.extend([STEEL, COKE, SLAG].into_iter().map(str::to_owned));
        let mut steel_recipe = None;
        for path in files {
            let value: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&path).expect("read recipe"))
                    .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            assert_eq!(
                value["type"],
                "arcforge:arcforge_smelting",
                "{}",
                path.display()
            );
            assert!(
                value["metal"]["ingredient"].is_string(),
                "{}",
                path.display()
            );
            assert!(
                value["metal"]["count"]
                    .as_u64()
                    .is_some_and(|count| count > 0)
            );
            assert!(value["coke"].as_u64().is_some());
            assert!(value["result"]["id"].is_string(), "{}", path.display());
            assert!(value["time"].as_u64().is_some_and(|ticks| ticks > 0));
            assert!(value["min_temp"].as_u64().is_some());
            for field in ["additive", "additive_2"] {
                if let Some(input) = value.get(field) {
                    assert!(input["ingredient"].is_string(), "{}", path.display());
                    assert!(input["count"].as_u64().is_some_and(|count| count > 0));
                }
            }
            let mut ids = Vec::new();
            for field in ["metal", "additive", "additive_2", "result", "byproduct"] {
                if let Some(value) = value.get(field) {
                    collect_arcforge_ids(value, &mut ids);
                }
            }
            for id in ids {
                assert!(
                    registered.contains(&id),
                    "unregistered {id} in {}",
                    path.display()
                );
            }
            if path
                .file_name()
                .is_some_and(|name| name == "steel_ingot.json")
            {
                steel_recipe = Some(value);
            }
        }
        let steel = steel_recipe.expect("upstream steel ingot recipe");
        assert_eq!(steel["coke"], 1);
        assert_eq!(steel["time"], 400);
        assert_eq!(steel["min_temp"], 1200);
        assert_eq!(steel["byproduct"]["id"], SLAG);
    }

    #[test]
    fn arcforge_furnace_uses_one_coke_for_fuel_and_keeps_one_for_the_recipe() {
        let host = DryingTestHost {
            random: horizon_client::mod_api::LegacyRandom::new(0),
            recipes: vec![horizon_client::mod_api::CustomRecipe {
                id: "arcforge:arcforge_smelting/steel_ingot".into(),
                json: Arc::new(serde_json::json!({
                    "metal": {"ingredient": "minecraft:iron_ingot", "count": 1},
                    "coke": 1,
                    "result": {"id": STEEL, "count": 1},
                    "byproduct": {"id": SLAG, "count": 1},
                    "time": 400,
                    "min_temp": 1200
                })),
            }],
        };
        let mut entity = ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
            id: FURNACE_PORT.to_owned(),
            items: vec![
                Some(ItemStack {
                    item: Arc::from("minecraft:iron_ingot"),
                    count: 1,
                    damage: 0,
                    extra: None,
                }),
                None,
                None,
                Some(ItemStack {
                    item: Arc::from(COKE),
                    count: 2,
                    damage: 0,
                    extra: None,
                }),
                None,
                None,
            ],
            ..Default::default()
        });

        for _ in 0..1000 {
            tick_arcforge_furnace_process(&host, &mut entity);
        }

        assert!(entity.items[0].is_none());
        assert!(entity.items[3].is_none());
        assert_eq!(entity.items[4].as_ref().unwrap().item.as_ref(), STEEL);
        assert_eq!(entity.items[5].as_ref().unwrap().item.as_ref(), SLAG);
        assert_eq!(get_i32(&entity.nbt, "TemperatureC"), 1400);
    }

    #[test]
    fn arcforge_furnace_breaks_a_coke_block_into_nine_usable_coke() {
        let mut entity = ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
            id: FURNACE_PORT.to_owned(),
            items: vec![
                None,
                None,
                None,
                Some(ItemStack {
                    item: Arc::from(COKE_BLOCK),
                    count: 1,
                    damage: 0,
                    extra: None,
                }),
                None,
                None,
            ],
            ..Default::default()
        });

        assert_eq!(arcforge_furnace_coke_count(&entity), 9);
        assert!(arcforge_furnace_take_coke(&mut entity));
        assert_eq!(arcforge_furnace_coke_count(&entity), 8);
        assert!(entity.items[3].is_none());
        assert_eq!(get_i32(&entity.nbt, "LooseCoke"), 8);
    }

    #[test]
    fn arcforge_furnace_matches_and_consumes_additives_in_reversed_slots() {
        let host = DryingTestHost {
            random: horizon_client::mod_api::LegacyRandom::new(0),
            recipes: vec![horizon_client::mod_api::CustomRecipe {
                id: "arcforge:arcforge_smelting/test_swapped".into(),
                json: Arc::new(serde_json::json!({
                    "metal": {"ingredient": "minecraft:iron_ingot", "count": 1},
                    "additive": {"ingredient": "minecraft:gold_ingot", "count": 2},
                    "additive_2": {"ingredient": "minecraft:copper_ingot", "count": 3},
                    "coke": 1,
                    "result": {"id": STEEL, "count": 1},
                    "time": 20,
                    "min_temp": 20
                })),
            }],
        };
        let mut entity = ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
            id: FURNACE_PORT.to_owned(),
            items: vec![
                Some(ItemStack {
                    item: Arc::from("minecraft:iron_ingot"),
                    count: 1,
                    damage: 0,
                    extra: None,
                }),
                Some(ItemStack {
                    item: Arc::from("minecraft:copper_ingot"),
                    count: 3,
                    damage: 0,
                    extra: None,
                }),
                Some(ItemStack {
                    item: Arc::from("minecraft:gold_ingot"),
                    count: 2,
                    damage: 0,
                    extra: None,
                }),
                None,
                None,
                None,
            ],
            ..Default::default()
        });

        let recipe = arcforge_furnace_recipe(&host, &entity).expect("swapped match");
        assert!(recipe.swapped);
        process_arcforge_furnace_recipe(&mut entity, &recipe);
        assert!(entity.items[0].is_none());
        assert!(entity.items[1].is_none());
        assert!(entity.items[2].is_none());
        assert_eq!(entity.items[4].as_ref().unwrap().item.as_ref(), STEEL);
    }

    #[test]
    fn arcforge_furnace_geometry_matches_the_upstream_structure_in_each_direction() {
        for facing in ["north", "east", "south", "west"] {
            let (front_x, front_z) = furnace_facing_vector(facing).expect("horizontal facing");
            let mut counts = std::collections::HashMap::new();
            for dy in 0..6 {
                for dx in -1..=1 {
                    for dz in -1..=1 {
                        *counts
                            .entry(furnace_expected_block(dx, dy, dz, facing))
                            .or_insert(0) += 1;
                    }
                }
            }
            assert_eq!(counts.get(&Some(FURNACE_BRICKS)), Some(&24));
            assert_eq!(counts.get(&Some(FURNACE_BRICK_WALL)), Some(&24));
            assert_eq!(counts.get(&Some(FURNACE_PORT)), Some(&1));
            assert_eq!(counts.get(&None), Some(&5));
            assert_eq!(
                furnace_expected_block(front_x, 0, front_z, facing),
                Some(FURNACE_PORT)
            );
            assert_eq!(
                furnace_expected_block(0, 0, 0, facing),
                Some(FURNACE_BRICKS)
            );
            assert_eq!(furnace_expected_block(0, 1, 0, facing), None);
        }
        assert_eq!(furnace_facing_vector("up"), None);
    }

    #[test]
    fn arcforge_furnace_building_recipes_and_drops_are_present() {
        let data =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/data/arcforge");
        for (file, result) in [
            ("arcforge_furnace_bricks.json", FURNACE_BRICKS),
            ("arcforge_furnace_brick_wall.json", FURNACE_BRICK_WALL),
            ("arcforge_furnace_port.json", FURNACE_PORT),
        ] {
            let recipe: serde_json::Value = serde_json::from_slice(
                &std::fs::read(data.join("recipe").join(file))
                    .expect("furnace construction recipe"),
            )
            .expect("valid furnace construction recipe");
            assert_eq!(recipe["type"], "minecraft:crafting_shaped");
            assert_eq!(recipe["result"]["id"], result);
            let loot: serde_json::Value = serde_json::from_slice(
                &std::fs::read(data.join("loot_table/blocks").join(file))
                    .expect("furnace construction loot table"),
            )
            .expect("valid furnace construction loot table");
            assert_eq!(loot["pools"][0]["entries"][0]["name"], result);
        }

        let coke_block: serde_json::Value = serde_json::from_slice(
            &std::fs::read(data.join("recipe/coal_coke_block.json"))
                .expect("coke block crafting recipe"),
        )
        .expect("valid coke block recipe");
        assert_eq!(coke_block["type"], "minecraft:crafting_shaped");
        assert_eq!(coke_block["key"]["#"], COKE);
        assert_eq!(coke_block["result"]["id"], COKE_BLOCK);

        let unpack: serde_json::Value = serde_json::from_slice(
            &std::fs::read(data.join("recipe/coal_coke_from_coal_coke_block.json"))
                .expect("coke block unpacking recipe"),
        )
        .expect("valid coke unpacking recipe");
        assert_eq!(unpack["type"], "minecraft:crafting_shapeless");
        assert_eq!(unpack["ingredients"][0], COKE_BLOCK);
        assert_eq!(unpack["result"]["id"], COKE);
        assert_eq!(unpack["result"]["count"], 9);
    }

    #[test]
    fn output_capacity_check_rejects_overflow_and_mismatched_items() {
        let steel = ItemStack {
            item: Arc::from(STEEL),
            count: 63,
            damage: 0,
            extra: None,
        };
        let other = ItemStack {
            item: Arc::from("minecraft:iron_ingot"),
            count: 1,
            damage: 0,
            extra: None,
        };
        assert!(can_add(None, STEEL, 1));
        assert!(can_add(Some(&steel), STEEL, 1));
        assert!(!can_add(Some(&steel), STEEL, 2));
        assert!(!can_add(Some(&other), STEEL, 1));
    }

    #[test]
    fn steelworks_inventory_migration_preserves_all_legacy_slots() {
        let mut old = (0..5)
            .map(|index| {
                Some(ItemStack {
                    item: Arc::from(format!("test:item_{index}")),
                    count: index + 1,
                    damage: 0,
                    extra: None,
                })
            })
            .collect::<Vec<_>>();
        migrate_steelworks_inventory(&mut old, 0);
        assert_eq!(old.len(), 7);
        assert_eq!(old[0].as_ref().unwrap().item.as_ref(), "test:item_0");
        assert!(old[1].is_none());
        assert!(old[2].is_none());
        assert_eq!(old[3].as_ref().unwrap().item.as_ref(), "test:item_1");
        assert_eq!(old[4].as_ref().unwrap().item.as_ref(), "test:item_2");
        assert_eq!(old[5].as_ref().unwrap().item.as_ref(), "test:item_3");
        assert_eq!(old[6].as_ref().unwrap().item.as_ref(), "test:item_4");
        migrate_steelworks_inventory(&mut old, 1);
        assert_eq!(old.len(), 7);
        assert_eq!(old[3].as_ref().unwrap().item.as_ref(), "test:item_1");
    }

    #[test]
    fn carbonizer_item_helpers_preserve_empty_and_decrement_stacks() {
        let mut coal = Some(ItemStack {
            item: Arc::from("minecraft:coal"),
            count: 1,
            damage: 0,
            extra: None,
        });
        assert!(consume_one(&mut coal));
        assert!(coal.is_none());
        assert!(!consume_one(&mut coal));
        let mut coke = None;
        add_item(&mut coke, COKE);
        assert_eq!(coke.unwrap().item.as_ref(), COKE);
    }

    #[test]
    fn crusher_recipe_records_retain_upstream_time_bonus_and_array_yield_fields() {
        let recipe = CrusherRecipe {
            result: Some((IRON_DUST.into(), 2)),
            bonus: Some((IRON_DUST.into(), 1)),
            bonus_chance: 0.25,
            ticks: 200,
            ore: true,
        };
        assert_eq!(recipe.result, Some((IRON_DUST.into(), 2)));
        assert_eq!(recipe.bonus, Some((IRON_DUST.into(), 1)));
        assert_eq!(recipe.bonus_chance, 0.25);
        assert_eq!(recipe.ticks, 200);
        assert!(recipe.ore);
        assert_eq!(array_yield_multiplier(recipe.ore), 2);
        assert_eq!(array_yield_multiplier(false), 1);
    }

    #[test]
    fn crushing_recipe_documents_have_the_custom_type_and_machine_fields() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../content/data/arcforge/recipe/crushing");
        let files = std::fs::read_dir(dir).expect("crushing recipe directory");
        let mut count = 0;
        for file in files {
            let file = file.expect("recipe file").path();
            let value: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&file).expect("read recipe file"))
                    .unwrap_or_else(|error| panic!("{}: {error}", file.display()));
            assert_eq!(value["type"], "arcforge:crushing", "{}", file.display());
            assert!(value.get("ingredient").is_some(), "{}", file.display());
            assert!(
                value.get("result").is_some() || value.get("bonus").is_some(),
                "{}",
                file.display()
            );
            assert!(
                value["ingredient"].is_string() || value["ingredient"].is_array(),
                "{}",
                file.display()
            );
            assert!(value["time"].as_u64().is_some_and(|ticks| ticks > 0));
            assert!(value["ore"].is_boolean());
            count += 1;
        }
        assert_eq!(count, 83);
    }

    #[test]
    fn carbonizing_recipes_cover_the_upstream_recipe_set_and_fields() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../content/data/arcforge/recipe/carbonizing");
        let mut files = std::fs::read_dir(dir)
            .expect("carbonizing recipe directory")
            .map(|entry| entry.expect("recipe file").path())
            .collect::<Vec<_>>();
        files.sort();
        assert_eq!(files.len(), 4);
        let mut names = Vec::new();
        for path in files {
            let value: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&path).expect("read recipe"))
                    .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            assert_eq!(value["type"], "arcforge:carbonizing", "{}", path.display());
            assert!(value["ingredient"].is_string(), "{}", path.display());
            assert!(value["result"]["id"].is_string(), "{}", path.display());
            assert!(value["time"].as_u64().is_some_and(|ticks| ticks > 0));
            if let Some(byproduct) = value.get("byproduct") {
                assert!(byproduct["id"].is_string(), "{}", path.display());
                assert!(
                    byproduct["amount"]
                        .as_u64()
                        .is_some_and(|amount| amount > 0)
                );
            }
            names.push(path.file_name().unwrap().to_string_lossy().into_owned());
        }
        assert_eq!(
            names,
            [
                "carbon_dust.json",
                "charcoal_from_logs.json",
                "coal_coke_block_from_bio_coal_block.json",
                "coal_coke_from_bio_coal.json",
            ]
        );
    }

    #[test]
    fn firebox_and_fiberizer_recipes_match_the_upstream_machine_contract() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../content/data/arcforge/recipe");
        let mut fiberizing = std::fs::read_dir(root.join("fiberizing"))
            .expect("fiberizing recipe directory")
            .map(|entry| entry.expect("recipe file").path())
            .collect::<Vec<_>>();
        fiberizing.sort();
        assert_eq!(fiberizing.len(), 3);
        let mut outputs = std::collections::HashSet::new();
        for path in fiberizing {
            let recipe: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&path).expect("read recipe"))
                    .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            assert_eq!(recipe["type"], "arcforge:fiberizing", "{}", path.display());
            assert!(recipe["ingredient"].is_string() || recipe["ingredient"].is_array());
            assert!(recipe["result"]["id"].is_string());
            assert!(recipe["time"].as_u64().is_some_and(|ticks| ticks > 0));
            assert!(recipe["fe_per_tick"].as_u64().is_some_and(|fe| fe > 0));
            assert!(recipe["hu_per_tick"].as_u64().is_some_and(|hu| hu > 0));
            assert!(recipe["min_temp"].as_u64().is_some_and(|temp| temp > 0));
            outputs.insert(recipe["result"]["id"].as_str().unwrap().to_owned());
        }
        assert_eq!(
            outputs,
            [
                "arcforge:carbon_fiber".to_owned(),
                "arcforge:rock_wool".to_owned(),
                "arcforge:slag_wool".to_owned(),
            ]
            .into_iter()
            .collect()
        );

        let firebox = std::fs::read(root.join("firebox.json")).expect("upstream firebox recipe");
        let firebox: serde_json::Value = serde_json::from_slice(&firebox).expect("valid recipe");
        assert_eq!(firebox["result"]["id"], FIREBOX);
        assert_eq!(firebox["key"]["B"], "arcforge:arcforge_furnace_bricks");
        assert_eq!(firebox_burn_duration("minecraft:coal"), Some(800));
        assert_eq!(firebox_burn_duration("minecraft:charcoal"), Some(800));
        assert_eq!(firebox_burn_duration("minecraft:coal_block"), Some(8_000));
        assert_eq!(firebox_burn_duration("arcforge:bio_coal"), Some(800));
        assert_eq!(
            firebox_burn_duration("arcforge:bio_coal_block"),
            Some(8_000)
        );
        assert_eq!(firebox_burn_duration("arcforge:coal_coke"), None);
    }

    #[test]
    fn sifting_recipes_cover_upstream_outputs_and_mesh_tiers() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../content/data/arcforge/recipe/sifting");
        let mut files = std::fs::read_dir(&dir)
            .expect("sifting recipe directory")
            .map(|entry| entry.expect("recipe file").path())
            .collect::<Vec<_>>();
        files.sort();
        assert_eq!(files.len(), 4);
        let mut registered: std::collections::HashSet<String> = crushing_catalog::BLOCKS
            .iter()
            .chain(crushing_catalog::ITEMS)
            .map(|(id, _)| (*id).to_owned())
            .collect();
        registered.extend(
            [
                SIFTER,
                STEEL_MESH,
                INVAR_MESH,
                TUNGSTEN_MESH,
                SILICON_CARBIDE_MESH,
                IRON_DUST,
                COPPER_DUST,
                GOLD_DUST,
            ]
            .into_iter()
            .map(str::to_owned),
        );
        registered.insert("arcforge:heavy_mineral_sand".to_owned());
        for path in files {
            let value: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&path).expect("read recipe"))
                    .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            assert_eq!(value["type"], "arcforge:sifting", "{}", path.display());
            assert!(value["ingredient"].is_string() || value["ingredient"].is_array());
            let outputs = value["outputs"].as_array().expect("output list");
            assert!(!outputs.is_empty() && outputs.len() <= 6);
            let mut ids = Vec::new();
            collect_arcforge_ids(&value["ingredient"], &mut ids);
            for output in outputs {
                assert!(output["item"]["id"].is_string());
                let chance = output["chance"].as_f64().expect("output chance");
                assert!((0.0..=1.0).contains(&chance));
                collect_arcforge_ids(&output["item"], &mut ids);
            }
            for id in ids {
                assert!(
                    registered.contains(&id),
                    "unregistered {id} in {}",
                    path.display()
                );
            }
        }
        assert_eq!(
            sifter_mesh_tier(STEEL_MESH),
            Some(SifterMeshTier {
                durability: 256,
                chance_multiplier: 1.0,
                speed_multiplier: 1.0,
            })
        );
        assert_eq!(sifter_mesh_tier(INVAR_MESH).unwrap().durability, 512);
        assert_eq!(sifter_mesh_tier(TUNGSTEN_MESH).unwrap().durability, 1024);
        assert_eq!(
            sifter_mesh_tier(SILICON_CARBIDE_MESH).unwrap().durability,
            2048
        );
        assert_eq!(sifter_mesh_tier("minecraft:iron_ingot"), None);
        assert!(mesh_takes_damage(0, 0.99));
        assert!(mesh_takes_damage(1, 0.49));
        assert!(!mesh_takes_damage(1, 0.5));
        assert!(mesh_takes_damage(3, 0.24));
        assert!(!mesh_takes_damage(3, 0.25));
    }

    #[test]
    fn sifter_outputs_stack_across_six_slots_without_overwrite() {
        let output = ItemStack {
            item: Arc::from("arcforge:iron_dust"),
            count: 3,
            damage: 0,
            extra: None,
        };
        let mut slots = vec![
            Some(ItemStack {
                item: Arc::from("arcforge:iron_dust"),
                count: 63,
                damage: 0,
                extra: None,
            }),
            None,
            None,
            None,
            None,
            None,
        ];
        assert!(sifter_insert(&mut slots, &output));
        assert_eq!(slots[0].as_ref().unwrap().count, 64);
        assert_eq!(slots[1].as_ref().unwrap().count, 2);
        let mut full = (0..6)
            .map(|_| {
                Some(ItemStack {
                    item: Arc::from("minecraft:stone"),
                    count: 64,
                    damage: 0,
                    extra: None,
                })
            })
            .collect::<Vec<_>>();
        assert!(!sifter_insert(&mut full, &output));
    }

    #[test]
    fn every_arcforge_item_in_crusher_recipes_is_registered() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../content/data/arcforge/recipe/crushing");
        let mut registered: std::collections::HashSet<String> = crushing_catalog::BLOCKS
            .iter()
            .chain(crushing_catalog::ITEMS)
            .map(|(id, _)| (*id).to_owned())
            .collect();
        registered.extend(
            [
                STEEL,
                COKE,
                SLAG,
                STEEL_BLOCK,
                CARBON_DUST,
                IRON_DUST,
                COPPER_DUST,
                GOLD_DUST,
                SILICA_DUST,
            ]
            .into_iter()
            .map(str::to_owned),
        );
        for file in std::fs::read_dir(dir).expect("crushing recipe directory") {
            let path = file.expect("recipe file").path();
            let value: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&path).expect("read recipe"))
                    .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            let mut arcforge_ids = Vec::new();
            collect_arcforge_ids(&value["ingredient"], &mut arcforge_ids);
            for field in ["result", "bonus"] {
                if let Some(id) = value[field]["id"].as_str() {
                    if id.starts_with("arcforge:") {
                        arcforge_ids.push(id.to_owned());
                    }
                }
            }
            for id in arcforge_ids {
                assert!(
                    registered.contains(&id),
                    "unregistered {id} in {}",
                    path.display()
                );
            }
        }
    }

    #[test]
    fn early_survival_crafting_recipes_close_the_steel_and_crusher_path() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../content/data/arcforge/recipe");
        let paths = [
            "carbonizer.json",
            "steelworks.json",
            "metal_press.json",
            "plate_die.json",
            "gear_die.json",
            "rod_die.json",
            "pellet_die.json",
            "sifter.json",
            "steel_mesh.json",
            "invar_mesh.json",
            "tungsten_mesh.json",
            "silicon_carbide_mesh.json",
            "steel_block.json",
            "steel_ingot_from_steel_block.json",
            "crafting/arc_crusher.json",
            "crafting/tungsten_heating_coil.json",
            "crafting/induction_furnace.json",
            "crafting/infuser.json",
            "crafting/fiberizer.json",
            "crafting/slime_ball_from_pine_resin.json",
            "crafting/wrench.json",
        ];
        let mut registered: std::collections::HashSet<String> = crushing_catalog::BLOCKS
            .iter()
            .chain(crushing_catalog::ITEMS)
            .map(|(id, _)| (*id).to_owned())
            .collect();
        registered.extend(
            [
                MACHINE,
                WRENCH,
                CARBONIZER,
                METAL_PRESS,
                ARC_CRUSHER,
                STEEL,
                STEEL_BLOCK,
                COKE,
                SLAG,
                PLATE_DIE,
                GEAR_DIE,
                ROD_DIE,
                PELLET_DIE,
                SIFTER,
                STEEL_MESH,
                INVAR_MESH,
                TUNGSTEN_MESH,
                TUNGSTEN_HEATING_COIL,
                SILICON_CARBIDE_MESH,
                INDUCTION_FURNACE,
                INFUSER,
                FIBERIZER,
                PINE_RESIN,
                FIREBOX,
                WROUGHT_FLUID_TANK,
                STEEL_PLATE,
                STEEL_GEAR,
                STEEL_ROD,
            ]
            .into_iter()
            .map(str::to_owned),
        );
        for path in paths {
            let file = root.join(path);
            let value: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&file).expect("read crafting recipe"))
                    .unwrap_or_else(|error| panic!("{}: {error}", file.display()));
            assert!(
                matches!(
                    value["type"].as_str(),
                    Some("minecraft:crafting_shaped" | "minecraft:crafting_shapeless")
                ),
                "{}",
                file.display()
            );
            let mut ids = Vec::new();
            collect_arcforge_ids(&value, &mut ids);
            for id in ids {
                assert!(
                    registered.contains(&id),
                    "unregistered {id} in {}",
                    file.display()
                );
            }
        }

        let hooks: [Box<dyn Hooks>; 1] = [Box::new(Arcforge)];
        let mut items = Vec::new();
        horizon_client::mod_api::dispatch_items(&hooks, &mut items);
        assert!(items.iter().any(|item| item.id == TUNGSTEN_HEATING_COIL));
        let tag_root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/data/c/tags/item");
        for (tag, value) in [
            ("rods/copper.json", "arcforge:copper_rod"),
            ("plates/tungsten.json", "arcforge:tungsten_plate"),
        ] {
            let path = tag_root.join(tag);
            let data: serde_json::Value = serde_json::from_slice(
                &std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display())),
            )
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            assert!(
                data["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| item == value)
            );
        }
    }

    #[test]
    fn steel_equipment_matches_source_stats_and_survival_recipes() {
        let hooks: [Box<dyn Hooks>; 1] = [Box::new(Arcforge)];
        let mut registered = Vec::new();
        horizon_client::mod_api::dispatch_items(&hooks, &mut registered);

        for (id, template, durability, slot, attribute, value) in [
            (
                "arcforge:steel_sword",
                "minecraft:iron_sword",
                500,
                "mainhand",
                "minecraft:attack_damage",
                5.5,
            ),
            (
                "arcforge:steel_pickaxe",
                "minecraft:iron_pickaxe",
                500,
                "mainhand",
                "minecraft:attack_damage",
                3.5,
            ),
            (
                "arcforge:steel_axe",
                "minecraft:iron_axe",
                500,
                "mainhand",
                "minecraft:attack_damage",
                8.5,
            ),
            (
                "arcforge:steel_shovel",
                "minecraft:iron_shovel",
                500,
                "mainhand",
                "minecraft:attack_damage",
                4.0,
            ),
            (
                "arcforge:steel_hoe",
                "minecraft:iron_hoe",
                500,
                "mainhand",
                "minecraft:attack_damage",
                0.0,
            ),
            (
                "arcforge:steel_hammer",
                "minecraft:iron_pickaxe",
                1_500,
                "mainhand",
                "minecraft:attack_damage",
                4.5,
            ),
            (
                "arcforge:steel_excavator",
                "minecraft:iron_shovel",
                1_500,
                "mainhand",
                "minecraft:attack_damage",
                4.0,
            ),
            (
                "arcforge:steel_helmet",
                "minecraft:iron_helmet",
                275,
                "head",
                "minecraft:armor",
                2.0,
            ),
            (
                "arcforge:steel_chestplate",
                "minecraft:iron_chestplate",
                400,
                "chest",
                "minecraft:armor",
                7.0,
            ),
            (
                "arcforge:steel_leggings",
                "minecraft:iron_leggings",
                375,
                "legs",
                "minecraft:armor",
                5.0,
            ),
            (
                "arcforge:steel_boots",
                "minecraft:iron_boots",
                325,
                "feet",
                "minecraft:armor",
                2.0,
            ),
        ] {
            let item = registered
                .iter()
                .find(|item| item.id == id)
                .unwrap_or_else(|| panic!("missing item {id}"));
            assert_eq!(item.template, template, "{id}");
            assert_eq!(item.max_stack_size, Some(1), "{id}");
            assert_eq!(item.components["minecraft:max_damage"], durability, "{id}");
            assert_eq!(
                item.components["minecraft:repairable"]["items"], "#c:ingots/steel",
                "{id}"
            );
            assert_eq!(
                item.components["minecraft:enchantable"]["value"], 12,
                "{id}"
            );
            assert!(
                item.components["minecraft:attribute_modifiers"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|modifier| {
                        modifier["type"] == attribute
                            && modifier["slot"] == slot
                            && modifier["amount"] == value
                    }),
                "missing {attribute}={value} in {id}"
            );
            if id == "arcforge:steel_hammer" || id == "arcforge:steel_excavator" {
                let expected_speed = 4.9;
                let expected_tag = if id == "arcforge:steel_hammer" {
                    "#minecraft:mineable/pickaxe"
                } else {
                    "#minecraft:mineable/shovel"
                };
                assert_eq!(
                    item.components["minecraft:tool"]["default_mining_speed"], expected_speed,
                    "{id}"
                );
                assert!(
                    item.components["minecraft:tool"]["rules"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|rule| rule["blocks"] == expected_tag
                            && rule["speed"] == expected_speed
                            && rule["correct_for_drops"] == true)
                );
            }
            if slot != "mainhand" {
                assert_eq!(
                    item.components["minecraft:equippable"]["slot"], slot,
                    "{id}"
                );
                assert_eq!(
                    item.components["minecraft:equippable"]["asset_id"], "arcforge:steel",
                    "{id}"
                );
                assert!(
                    item.components["minecraft:attribute_modifiers"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|modifier| {
                            modifier["type"] == "minecraft:armor_toughness"
                                && modifier["amount"] == 1.0
                        }),
                    "missing steel armor toughness on {id}"
                );
            }
        }

        for (name, output) in [
            ("steel_sword", "steel_sword"),
            ("steel_pickaxe", "steel_pickaxe"),
            ("steel_axe", "steel_axe"),
            ("steel_shovel", "steel_shovel"),
            ("steel_hoe", "steel_hoe"),
            ("steel_hammer", "steel_hammer"),
            ("steel_excavator", "steel_excavator"),
            ("steel_helmet", "steel_helmet"),
            ("steel_chestplate", "steel_chestplate"),
            ("steel_leggings", "steel_leggings"),
            ("steel_boots", "steel_boots"),
        ] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("../content/data/arcforge/recipe/{name}.json"));
            let recipe: serde_json::Value = serde_json::from_slice(
                &std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display())),
            )
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            assert_eq!(recipe["result"]["id"], format!("arcforge:{output}"));
            assert_eq!(recipe["category"], "equipment");
        }
        let wooden_rods = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../content/data/c/tags/item/rods/wooden.json");
        let tag: serde_json::Value = serde_json::from_slice(
            &std::fs::read(&wooden_rods).expect("wooden rod compatibility tag"),
        )
        .expect("valid wooden rod compatibility tag");
        assert_eq!(tag["values"][0], "minecraft:stick");
    }

    #[test]
    fn treated_wood_set_matches_source_registration_and_crafting_paths() {
        let blocks = [
            ("treated_log", "minecraft:crimson_stem", 2.0_f32, 2.0),
            ("treated_wood", "minecraft:crimson_hyphae", 2.0, 2.0),
            (
                "stripped_treated_log",
                "minecraft:stripped_crimson_stem",
                2.0,
                2.0,
            ),
            (
                "stripped_treated_wood",
                "minecraft:stripped_crimson_hyphae",
                2.0,
                2.0,
            ),
            ("treated_planks", "minecraft:crimson_planks", 2.0, 3.0),
            ("treated_stairs", "minecraft:crimson_stairs", 2.0, 3.0),
            ("treated_slab", "minecraft:crimson_slab", 2.0, 3.0),
            ("treated_fence", "minecraft:crimson_fence", 2.0, 3.0),
            (
                "treated_fence_gate",
                "minecraft:crimson_fence_gate",
                2.0,
                3.0,
            ),
            ("treated_door", "minecraft:crimson_door", 3.0, 3.0),
            ("treated_trapdoor", "minecraft:crimson_trapdoor", 3.0, 3.0),
            (
                "treated_pressure_plate",
                "minecraft:crimson_pressure_plate",
                0.5,
                0.5,
            ),
            ("treated_button", "minecraft:crimson_button", 0.5, 0.5),
        ];
        let hooks: [Box<dyn Hooks>; 1] = [Box::new(Arcforge)];
        let mut registered = Vec::new();
        horizon_client::mod_api::dispatch_blocks(&hooks, &mut registered);
        for (name, template, hardness, resistance) in blocks {
            let id = format!("arcforge:{name}");
            let block = registered
                .iter()
                .find(|block| block.id == id)
                .unwrap_or_else(|| panic!("missing block {id}"));
            assert_eq!(block.template, template, "{id}");
            assert_eq!(block.properties.hardness, Some(hardness), "{id}");
            assert_eq!(
                block.properties.explosion_resistance,
                Some(resistance),
                "{id}"
            );
            let (map_color, sounds) = if name == "treated_button" {
                (0, "minecraft:stone")
            } else {
                (26, "minecraft:oak_planks")
            };
            assert_eq!(block.properties.map_color, Some(map_color), "{id}");
            assert_eq!(block.properties.sounds.as_deref(), Some(sounds));
            let loot = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
                "../content/data/arcforge/loot_table/blocks/{name}.json"
            ));
            let _: serde_json::Value = serde_json::from_slice(
                &std::fs::read(&loot).unwrap_or_else(|error| panic!("{}: {error}", loot.display())),
            )
            .unwrap_or_else(|error| panic!("{}: {error}", loot.display()));
        }

        let recipe_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../content/data/arcforge/recipe");
        for name in [
            "treated_planks",
            "treated_stairs",
            "treated_slab",
            "treated_fence",
            "treated_fence_gate",
            "treated_door",
            "treated_trapdoor",
            "treated_pressure_plate",
            "treated_button",
        ] {
            let path = recipe_root.join(format!("{name}.json"));
            let recipe: serde_json::Value = serde_json::from_slice(
                &std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display())),
            )
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            assert!(
                recipe["type"]
                    .as_str()
                    .unwrap()
                    .starts_with("minecraft:crafting_")
            );
            assert_eq!(recipe["result"]["id"], format!("arcforge:{name}"));
        }

        let tags =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/data/arcforge/tags");
        for kind in ["block", "item"] {
            let path = tags.join(kind).join("treated_logs.json");
            let tag: serde_json::Value = serde_json::from_slice(
                &std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display())),
            )
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            assert_eq!(tag["values"].as_array().unwrap().len(), 4);
        }
    }

    fn collect_arcforge_ids(value: &serde_json::Value, out: &mut Vec<String>) {
        match value {
            serde_json::Value::String(id) if id.starts_with("arcforge:") => {
                out.push(id.clone());
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    collect_arcforge_ids(value, out);
                }
            }
            serde_json::Value::Object(values) => {
                for value in values.values() {
                    collect_arcforge_ids(value, out);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn induction_furnace_covers_common_vanilla_smelting_paths() {
        assert_eq!(
            smelting_recipe("minecraft:iron_ore"),
            Some("minecraft:iron_ingot")
        );
        assert_eq!(
            smelting_recipe("minecraft:raw_copper"),
            Some("minecraft:copper_ingot")
        );
        assert_eq!(
            smelting_recipe("minecraft:ancient_debris"),
            Some("minecraft:netherite_scrap")
        );
        assert_eq!(smelting_recipe("minecraft:sand"), Some("minecraft:glass"));
        assert_eq!(smelting_recipe("minecraft:stone"), None);
    }

    #[test]
    fn induction_furnace_uses_arcforge_smelting_catalog_and_source_time() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../content/data/arcforge/recipe/induction_smelting")
            .join("quartz_from_smelting_nether_quartz_dust.json");
        let json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).expect("Arcforge Induction recipe"))
                .expect("valid Arcforge Induction recipe");
        let host = InductionTestHost {
            recipes: vec![horizon_client::mod_api::CustomRecipe {
                id: "arcforge:induction_smelting/quartz_from_nether_quartz_dust".to_owned(),
                json: Arc::new(json),
            }],
        };
        let input = ItemStack {
            item: Arc::from("arcforge:nether_quartz_dust"),
            count: 1,
            damage: 0,
            extra: None,
        };
        let recipe = induction_recipe(&host, &input).expect("matching custom recipe");
        assert_eq!(recipe.output.item.as_ref(), "minecraft:quartz");
        assert_eq!(recipe.ticks, 100);

        let mut furnace = ModBlockEntity::from(horizon_client::containers::ModBlockEntityFields {
            id: INDUCTION_FURNACE.to_owned(),
            items: vec![Some(input), None, None],
            nbt: {
                let mut nbt = horizon_client::mod_api::Compound::default();
                set_i32(&mut nbt, "energy", 20_000);
                nbt
            },
            ..Default::default()
        });
        for _ in 0..100 {
            tick_induction_furnace(&host, &mut furnace);
        }
        assert!(furnace.items[0].is_none());
        assert_eq!(
            furnace.items[2].as_ref().unwrap().item.as_ref(),
            "minecraft:quartz"
        );
        assert_eq!(get_i32(&furnace.nbt, "energy"), 18_000);
    }

    #[test]
    fn pressing_recipe_documents_cover_upstream_machine_recipes() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../content/data/arcforge/recipe/pressing");
        let mut files = std::fs::read_dir(&dir)
            .expect("pressing recipe directory")
            .map(|entry| entry.expect("recipe file").path())
            .collect::<Vec<_>>();
        files.sort();
        assert_eq!(files.len(), 25);
        let registered: std::collections::HashSet<String> = crushing_catalog::BLOCKS
            .iter()
            .chain(crushing_catalog::ITEMS)
            .map(|(id, _)| (*id).to_owned())
            .chain(
                [
                    STEEL,
                    STEEL_PLATE,
                    STEEL_GEAR,
                    STEEL_ROD,
                    PLATE_DIE,
                    GEAR_DIE,
                    ROD_DIE,
                    PELLET_DIE,
                ]
                .into_iter()
                .map(str::to_owned),
            )
            .collect();
        for path in files {
            let value: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&path).expect("read recipe"))
                    .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            assert_eq!(value["type"], "arcforge:pressing", "{}", path.display());
            assert!(value["ingredient"].is_string(), "{}", path.display());
            assert!(value["count"].as_u64().is_some_and(|count| count > 0));
            assert!(value["time"].as_u64().is_some_and(|ticks| ticks > 0));
            assert!(value["die"].is_string(), "{}", path.display());
            assert!(value["result"]["id"].is_string(), "{}", path.display());
            for field in ["die", "result"] {
                let id = if field == "die" {
                    value[field].as_str().unwrap()
                } else {
                    value[field]["id"].as_str().unwrap()
                };
                if id.starts_with("arcforge:") {
                    assert!(
                        registered.contains(id),
                        "unregistered {id} in {}",
                        path.display()
                    );
                }
            }
            if let Some(id) = value["ingredient"]
                .as_str()
                .and_then(|s| s.strip_prefix("arcforge:"))
            {
                assert!(
                    registered.contains(&format!("arcforge:{id}")),
                    "unregistered ingredient in {}",
                    path.display()
                );
            }
        }
    }

    #[test]
    fn energy_cell_tiers_have_upstream_storage_and_transfer_limits() {
        assert_eq!(energy_cell_spec(WROUGHT_ENERGY_CELL), Some((500_000, 512)));
        assert_eq!(
            energy_cell_spec(TEMPERED_ENERGY_CELL),
            Some((4_000_000, 2_048))
        );
        assert_eq!(
            energy_cell_spec(HARDENED_ENERGY_CELL),
            Some((32_000_000, 16_384))
        );
        assert_eq!(
            energy_cell_spec(ARCFORGED_ENERGY_CELL),
            Some((256_000_000, 262_144))
        );
        assert_eq!(energy_cell_spec("arcforge:creative_energy_cell"), None);
        for (item, capacity, rate) in [
            (WROUGHT_ENERGY_CELL, 500_000, 512),
            (TEMPERED_ENERGY_CELL, 4_000_000, 2_048),
            (HARDENED_ENERGY_CELL, 32_000_000, 16_384),
            (ARCFORGED_ENERGY_CELL, 256_000_000, 262_144),
        ] {
            assert_eq!(
                portable_energy_item_buffer(item),
                Some(ItemEnergyBuffer {
                    capacity,
                    max_insert: rate,
                    max_extract: rate,
                })
            );
        }
        assert_eq!(portable_energy_item_buffer("minecraft:iron_ingot"), None);

        let recipe_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../content/data/arcforge/recipe/crafting");
        for tier in ["wrought", "tempered", "hardened", "arcforged"] {
            let path = recipe_dir.join(format!("{tier}_energy_cell.json"));
            let recipe: serde_json::Value = serde_json::from_slice(
                &std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display())),
            )
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            assert_eq!(
                recipe["type"],
                "minecraft:crafting_shaped",
                "{}",
                path.display()
            );
            assert!(
                recipe["result"]["id"]
                    .as_str()
                    .unwrap()
                    .ends_with("_energy_cell")
            );
            assert!(recipe["copy_components_from"].is_null());
        }
    }

    #[test]
    fn arcite_blocks_keep_upstream_light_emission() {
        let hooks: [Box<dyn Hooks>; 1] = [Box::new(Arcforge)];
        let mut blocks = Vec::new();
        horizon_client::mod_api::dispatch_blocks(&hooks, &mut blocks);
        let arcite_ore = blocks
            .iter()
            .find(|block| block.id == "arcforge:arcite_ore")
            .expect("registered Arcite ore");
        let deepslate_arcite_ore = blocks
            .iter()
            .find(|block| block.id == "arcforge:deepslate_arcite_ore")
            .expect("registered deepslate Arcite ore");
        let raw_arcite = blocks
            .iter()
            .find(|block| block.id == "arcforge:raw_arcite_block")
            .expect("registered raw Arcite storage block");
        let arcite = blocks
            .iter()
            .find(|block| block.id == "arcforge:arcite_block")
            .expect("registered Arcite storage block");
        assert_eq!(arcite_ore.properties.light, Some(5));
        assert_eq!(deepslate_arcite_ore.properties.light, Some(5));
        assert_eq!(raw_arcite.properties.light, Some(9));
        assert_eq!(arcite.properties.light, Some(12));
    }
}
