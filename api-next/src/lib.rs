//! Staged port of Arcforge's native behavior to Horizon's current mod API.
//!
//! The production package remains under `api/` until the gameplay hooks and content
//! declarations have been migrated. New API work starts here so each slice can be checked
//! against the pinned Horizon SDK without breaking the existing package.

use mod_api::{
    Block, BlockEntityPart, BlockEntityTickEvent, BlockEntityTicksEvent, BlockKey, BlockPart,
    BlockStateProperty, Flow, Hooks, Host, HostSide, InteractionResult, Item, MenuContainer,
    MenuPart, MenuSlotRulesEvent, ModContainer, ModEnergyBuffer, ModFluidTanks, Registrar,
    RegistrationContext, ServerHost,
};
use std::collections::BTreeMap;

mod cracking_unit;
mod firebox;
mod geothermal;
mod heat_cell;
mod solar;
mod thermal;
mod thermoelectric;

const CONTROLLER: &str = "arcforge:cracking_unit_controller";
const CASING: &str = "arcforge:cracking_unit_casing";
const CATALYST: &str = "arcforge:cracking_catalyst";
const SPEED_UPGRADE: &str = "arcforge:speed_upgrade";
const HEAT_UPGRADE: &str = "arcforge:heat_upgrade";
const THERMOELECTRIC_UPGRADE: &str = "arcforge:thermoelectric_upgrade";
const WRENCH: &str = "arcforge:wrench";
const MACHINE_MENU: &str = "arcforge:cracking_unit";
const FIREBOX: &str = "arcforge:firebox";
const FIREBOX_MENU: &str = "arcforge:firebox";
const GEOTHERMAL_PLANT: &str = "arcforge:geothermal_plant";
const GEOTHERMAL_MENU: &str = "arcforge:geothermal_plant";
const SOLAR_CONTROLLER: &str = solar::CONTROLLER;
const SOLAR_CASING: &str = solar::CASING;
const SOLAR_COLLECTOR: &str = solar::COLLECTOR;
const SOLAR_MENU: &str = "arcforge:solar_thermal_array";
const THERMOELECTRIC_PLANT: &str = "arcforge:thermoelectric_plant";
const THERMOELECTRIC_MENU: &str = "arcforge:thermoelectric_plant";
const BIO_COAL: &str = "arcforge:bio_coal";
const BIO_COAL_BLOCK: &str = "arcforge:bio_coal_block";
const WOOD_ASH: &str = "arcforge:wood_ash";

#[derive(Default)]
struct Arcforge;

struct CrackingUnit {
    controller: BlockKey,
    casing: BlockKey,
}

struct HeatCell {
    id: &'static str,
}

struct Firebox;

struct GeothermalPlant;

struct SolarThermalArray;

struct ThermalConduit {
    id: &'static str,
}

struct ThermoelectricPlant;

#[mod_api::horizon_part]
impl BlockPart for HeatCell {}

#[mod_api::horizon_part]
impl BlockPart for Firebox {}

#[mod_api::horizon_part]
impl BlockPart for GeothermalPlant {}

#[mod_api::horizon_part]
impl BlockPart for SolarThermalArray {}

#[mod_api::horizon_part]
impl BlockPart for ThermalConduit {}

#[mod_api::horizon_part]
impl BlockPart for ThermoelectricPlant {}

#[mod_api::horizon_part]
impl BlockPart for CrackingUnit {}

#[mod_api::horizon_part]
impl BlockEntityPart for CrackingUnit {
    fn screen(&self, _: &dyn Host, _: &mut mod_api::ScreenEvent<'_>) -> Option<mod_api::Screen> {
        Some(cracking_unit::screen())
    }

    fn screen_draw(&self, _: &dyn mod_api::ClientHost, event: &mut mod_api::ScreenDrawEvent<'_>) {
        cracking_unit::draw_screen(event);
    }

    fn screen_click(
        &self,
        _: &dyn mod_api::ClientHost,
        event: &mut mod_api::ScreenClickEvent<'_>,
    ) -> Option<i32> {
        cracking_unit::screen_click(event.at, event.right)
    }

    fn screen_tooltip(
        &self,
        _: &dyn mod_api::ClientHost,
        event: &mut mod_api::ScreenTooltipEvent<'_>,
    ) {
        cracking_unit::screen_tooltip(event.at, event.lines);
    }

    fn menu_button(&self, _: &mut dyn ServerHost, event: &mut mod_api::MenuButtonEvent<'_>) {
        cracking_unit::select_mode(&mut event.entity.nbt, event.context.id);
    }

    fn block_entity_ticks(&self, _: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        cracking_unit::tick(host, event, self.controller, self.casing);
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for HeatCell {
    fn block_entity_ticks(&self, _: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        heat_cell::tick(host, event, self.id);
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for ThermalConduit {
    fn block_entity_ticks(&self, _: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        thermal::tick(host, event, self.id);
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for Firebox {
    fn screen(&self, _: &dyn Host, _: &mut mod_api::ScreenEvent<'_>) -> Option<mod_api::Screen> {
        Some(firebox::screen())
    }

    fn screen_draw(&self, _: &dyn mod_api::ClientHost, event: &mut mod_api::ScreenDrawEvent<'_>) {
        firebox::draw_screen(event);
    }

    fn screen_tooltip(
        &self,
        _: &dyn mod_api::ClientHost,
        event: &mut mod_api::ScreenTooltipEvent<'_>,
    ) {
        firebox::screen_tooltip(event.at, event.lines);
    }

    fn block_entity_ticks(&self, _: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        firebox::tick(host, event);
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for GeothermalPlant {
    fn screen(&self, _: &dyn Host, _: &mut mod_api::ScreenEvent<'_>) -> Option<mod_api::Screen> {
        Some(geothermal::screen())
    }

    fn screen_draw(&self, _: &dyn mod_api::ClientHost, event: &mut mod_api::ScreenDrawEvent<'_>) {
        geothermal::draw_screen(event);
    }

    fn screen_tooltip(
        &self,
        _: &dyn mod_api::ClientHost,
        event: &mut mod_api::ScreenTooltipEvent<'_>,
    ) {
        geothermal::screen_tooltip(event.at, event.lines);
    }

    fn block_entity_ticks(&self, _: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        geothermal::tick(host, event);
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for SolarThermalArray {
    fn screen(&self, _: &dyn Host, _: &mut mod_api::ScreenEvent<'_>) -> Option<mod_api::Screen> {
        Some(solar::screen())
    }

    fn screen_draw(&self, _: &dyn mod_api::ClientHost, event: &mut mod_api::ScreenDrawEvent<'_>) {
        solar::draw_screen(event);
    }

    fn screen_tooltip(
        &self,
        _: &dyn mod_api::ClientHost,
        event: &mut mod_api::ScreenTooltipEvent<'_>,
    ) {
        solar::screen_tooltip(event.at, event.lines);
    }

    fn block_entity_ticks(&self, _: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        solar::tick(host, event);
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for ThermoelectricPlant {
    fn screen(&self, _: &dyn Host, _: &mut mod_api::ScreenEvent<'_>) -> Option<mod_api::Screen> {
        Some(thermoelectric::screen())
    }

    fn screen_draw(&self, _: &dyn mod_api::ClientHost, event: &mut mod_api::ScreenDrawEvent<'_>) {
        thermoelectric::draw_screen(event);
    }

    fn screen_tooltip(
        &self,
        _: &dyn mod_api::ClientHost,
        event: &mut mod_api::ScreenTooltipEvent<'_>,
    ) {
        thermoelectric::screen_tooltip(event.at, event.lines);
    }

    fn block_entity_ticks(&self, _: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        thermoelectric::tick(host, event);
    }
}

#[mod_api::horizon_part]
impl MenuPart for CrackingUnit {
    fn menu_slot_rules(&self, host: &dyn Host, event: &mut MenuSlotRulesEvent<'_>) {
        if event.container != MenuContainer::Main {
            return;
        }

        let expected_item = match event.slot {
            0 => {
                event.max_stack = 8;
                SPEED_UPGRADE
            }
            1 => CATALYST,
            _ => return,
        };
        event.may_place &= event.stack.item().id(host) == expected_item;
    }
}

#[mod_api::horizon_part]
impl MenuPart for Firebox {
    fn menu_slot_rules(&self, host: &dyn Host, event: &mut MenuSlotRulesEvent<'_>) {
        if event.container != MenuContainer::Main {
            return;
        }
        let item = event.stack.item().id(host);
        match event.slot {
            0 => event.may_place &= firebox::accepts_fuel(&item),
            1 => event.may_place = false,
            2 => {
                event.max_stack = 8;
                event.may_place &= item == SPEED_UPGRADE;
            }
            3 => {
                event.max_stack = 8;
                event.may_place &= item == HEAT_UPGRADE;
            }
            _ => event.may_place = false,
        }
    }
}

#[mod_api::horizon_part]
impl MenuPart for GeothermalPlant {
    fn menu_slot_rules(&self, host: &dyn Host, event: &mut MenuSlotRulesEvent<'_>) {
        if event.container != MenuContainer::Main {
            return;
        }
        let item = event.stack.item().id(host);
        match event.slot {
            0 => event.may_place &= item == "minecraft:lava_bucket",
            1 => event.may_place = false,
            2 => {
                event.max_stack = 8;
                event.may_place &= item == SPEED_UPGRADE;
            }
            3 => {
                event.max_stack = 8;
                event.may_place &= item == HEAT_UPGRADE;
            }
            _ => event.may_place = false,
        }
    }
}

#[mod_api::horizon_part]
impl MenuPart for ThermoelectricPlant {
    fn menu_slot_rules(&self, host: &dyn Host, event: &mut MenuSlotRulesEvent<'_>) {
        if event.container != MenuContainer::Main {
            return;
        }
        event.max_stack = 8;
        let item = event.stack.item().id(host);
        let expected = match event.slot {
            0 => SPEED_UPGRADE,
            1 => HEAT_UPGRADE,
            2 => THERMOELECTRIC_UPGRADE,
            _ => return,
        };
        event.may_place &= item == expected;
    }
}

#[mod_api::horizon_part]
impl MenuPart for SolarThermalArray {
    fn menu_slot_rules(&self, _: &dyn Host, event: &mut MenuSlotRulesEvent<'_>) {
        if event.container == MenuContainer::Main && event.slot == 0 {
            event.may_place = false;
        }
    }
}

#[mod_api::horizon_mod]
impl Hooks for Arcforge {
    fn blocks(&self, _: &RegistrationContext, event: &mut mod_api::BlocksEvent<'_>) {
        let mut controller = Block::new(
            "arcforge:cracking_unit_controller",
            "minecraft:blast_furnace",
        );
        controller.state_properties = [
            (
                "facing".into(),
                BlockStateProperty::Enum {
                    values: vec!["north".into(), "east".into(), "south".into(), "west".into()],
                },
            ),
            ("formed".into(), BlockStateProperty::Boolean),
            ("lit".into(), BlockStateProperty::Boolean),
        ]
        .into();
        controller.default_state = [
            ("facing".into(), "north".into()),
            ("formed".into(), "false".into()),
            ("lit".into(), "false".into()),
        ]
        .into();
        event.blocks.push(controller);

        let mut casing = Block::new(CASING, "minecraft:iron_block");
        casing.state_properties = [("formed".into(), BlockStateProperty::Boolean)].into();
        casing.default_state = [("formed".into(), "false".into())].into();
        event.blocks.push(casing);

        let mut firebox = Block::new(FIREBOX, "minecraft:blast_furnace");
        firebox.state_properties = [
            (
                "facing".into(),
                BlockStateProperty::Enum {
                    values: ["north", "east", "south", "west"]
                        .into_iter()
                        .map(str::to_owned)
                        .collect(),
                },
            ),
            ("lit".into(), BlockStateProperty::Boolean),
        ]
        .into();
        firebox.default_state = [
            ("facing".into(), "north".into()),
            ("lit".into(), "false".into()),
        ]
        .into();
        event.blocks.push(firebox);
        let mut geothermal = Block::new(GEOTHERMAL_PLANT, "minecraft:blast_furnace");
        geothermal.state_properties = [
            (
                "facing".into(),
                BlockStateProperty::Enum {
                    values: ["north", "east", "south", "west"]
                        .into_iter()
                        .map(str::to_owned)
                        .collect(),
                },
            ),
            ("lit".into(), BlockStateProperty::Boolean),
        ]
        .into();
        geothermal.default_state = [
            ("facing".into(), "north".into()),
            ("lit".into(), "false".into()),
        ]
        .into();
        event.blocks.push(geothermal);

        let mut thermoelectric = Block::new(THERMOELECTRIC_PLANT, "minecraft:blast_furnace");
        thermoelectric.state_properties = [
            (
                "facing".into(),
                BlockStateProperty::Enum {
                    values: ["north", "east", "south", "west"]
                        .into_iter()
                        .map(str::to_owned)
                        .collect(),
                },
            ),
            ("lit".into(), BlockStateProperty::Boolean),
        ]
        .into();
        thermoelectric.default_state = [
            ("facing".into(), "north".into()),
            ("lit".into(), "false".into()),
        ]
        .into();
        event.blocks.push(thermoelectric);

        for id in thermal::ids() {
            let properties: BTreeMap<String, BlockStateProperty> = [
                (
                    "up".into(),
                    BlockStateProperty::Enum {
                        values: ["none", "pipe", "input", "output"]
                            .into_iter()
                            .map(str::to_owned)
                            .collect(),
                    },
                ),
                (
                    "down".into(),
                    BlockStateProperty::Enum {
                        values: ["none", "pipe", "input", "output"]
                            .into_iter()
                            .map(str::to_owned)
                            .collect(),
                    },
                ),
                (
                    "north".into(),
                    BlockStateProperty::Enum {
                        values: ["none", "pipe", "input", "output"]
                            .into_iter()
                            .map(str::to_owned)
                            .collect(),
                    },
                ),
                (
                    "south".into(),
                    BlockStateProperty::Enum {
                        values: ["none", "pipe", "input", "output"]
                            .into_iter()
                            .map(str::to_owned)
                            .collect(),
                    },
                ),
                (
                    "west".into(),
                    BlockStateProperty::Enum {
                        values: ["none", "pipe", "input", "output"]
                            .into_iter()
                            .map(str::to_owned)
                            .collect(),
                    },
                ),
                (
                    "east".into(),
                    BlockStateProperty::Enum {
                        values: ["none", "pipe", "input", "output"]
                            .into_iter()
                            .map(str::to_owned)
                            .collect(),
                    },
                ),
                ("active".into(), BlockStateProperty::Boolean),
            ]
            .into();
            let defaults: BTreeMap<String, String> = [
                ("up".into(), "none".into()),
                ("down".into(), "none".into()),
                ("north".into(), "none".into()),
                ("south".into(), "none".into()),
                ("west".into(), "none".into()),
                ("east".into(), "none".into()),
                ("active".into(), "false".into()),
            ]
            .into();
            let mut conduit = Block::new(id, "minecraft:copper_block");
            conduit.state_properties = properties;
            conduit.default_state = defaults;
            event.blocks.push(conduit);
        }

        let faces = ["up", "down", "north", "south", "west", "east"];
        let mut solar_controller = Block::new(SOLAR_CONTROLLER, "minecraft:blast_furnace");
        let mut controller_properties: BTreeMap<String, BlockStateProperty> = [
            (
                "facing".into(),
                BlockStateProperty::Enum {
                    values: ["north", "east", "south", "west"]
                        .into_iter()
                        .map(str::to_owned)
                        .collect(),
                },
            ),
            ("lit".into(), BlockStateProperty::Boolean),
            ("formed".into(), BlockStateProperty::Boolean),
            ("base".into(), BlockStateProperty::Boolean),
        ]
        .into();
        let mut controller_defaults: BTreeMap<String, String> = [
            ("facing".into(), "north".into()),
            ("lit".into(), "false".into()),
            ("formed".into(), "false".into()),
            ("base".into(), "false".into()),
        ]
        .into();
        for face in faces {
            controller_properties.insert(format!("port_{face}"), BlockStateProperty::Boolean);
            controller_defaults.insert(format!("port_{face}"), "false".into());
        }
        solar_controller.state_properties = controller_properties;
        solar_controller.default_state = controller_defaults;
        event.blocks.push(solar_controller);

        let mut solar_casing = Block::new(SOLAR_CASING, "minecraft:stone");
        let mut casing_properties: BTreeMap<String, BlockStateProperty> = [
            ("formed".into(), BlockStateProperty::Boolean),
            (
                "part".into(),
                BlockStateProperty::Enum {
                    values: ["none", "other", "base", "upper_ew", "upper_ns"]
                        .into_iter()
                        .map(str::to_owned)
                        .collect(),
                },
            ),
        ]
        .into();
        let mut casing_defaults: BTreeMap<String, String> = [
            ("formed".into(), "false".into()),
            ("part".into(), "none".into()),
        ]
        .into();
        for face in faces {
            casing_properties.insert(format!("port_{face}"), BlockStateProperty::Boolean);
            casing_defaults.insert(format!("port_{face}"), "false".into());
        }
        solar_casing.state_properties = casing_properties;
        solar_casing.default_state = casing_defaults;
        event.blocks.push(solar_casing);

        let mut solar_collector = Block::new(SOLAR_COLLECTOR, "minecraft:glass");
        let mut collector_properties: BTreeMap<String, BlockStateProperty> =
            [("formed".into(), BlockStateProperty::Boolean)].into();
        let mut collector_defaults: BTreeMap<String, String> =
            [("formed".into(), "false".into())].into();
        for face in faces {
            collector_properties.insert(format!("port_{face}"), BlockStateProperty::Boolean);
            collector_defaults.insert(format!("port_{face}"), "false".into());
        }
        solar_collector.state_properties = collector_properties;
        solar_collector.default_state = collector_defaults;
        event.blocks.push(solar_collector);

        event
            .blocks
            .push(Block::new(BIO_COAL_BLOCK, "minecraft:coal_block"));

        for id in heat_cell::ids() {
            let mut cell = Block::new(id, "minecraft:iron_block");
            cell.state_properties = [(
                "heat".into(),
                BlockStateProperty::Enum {
                    values: ["0", "1", "2", "3", "4"]
                        .into_iter()
                        .map(str::to_owned)
                        .collect(),
                },
            )]
            .into();
            cell.default_state = [("heat".into(), "0".into())].into();
            event.blocks.push(cell);
        }
    }

    fn items(&self, _: &RegistrationContext, event: &mut mod_api::ItemsEvent<'_>) {
        event.items.push(Item::new(CATALYST, "minecraft:clay_ball"));
        event.items.push(Item::new(BIO_COAL, "minecraft:coal"));
        event.items.push(Item::new(WOOD_ASH, "minecraft:bone_meal"));
        event
            .items
            .push(Item::new(SPEED_UPGRADE, "minecraft:sugar"));
        event
            .items
            .push(Item::new(HEAT_UPGRADE, "minecraft:magma_cream"));
        event
            .items
            .push(Item::new(THERMOELECTRIC_UPGRADE, "minecraft:lightning_rod"));
        let mut wrench = Item::new(WRENCH, "minecraft:iron_pickaxe");
        wrench.max_stack_size = Some(1);
        event.items.push(wrench);
    }

    fn right_click_block(
        &self,
        host: HostSide<'_>,
        event: &mut mod_api::RightClickBlockEvent<'_>,
    ) -> Flow {
        let wrench_target = {
            let host: &dyn Host = match &host {
                HostSide::Server(host) => &**host,
                HostSide::Client(host) => &**host,
            };
            let held_wrench = event
                .click
                .stack
                .is_some_and(|stack| stack.item().id(host) == WRENCH);
            if held_wrench {
                host.level(event.click.dimension.key().id())
                    .and_then(|level| level.block(host, event.click.pos))
                    .map(|block| block.id(host).to_owned())
            } else {
                None
            }
        };

        if !wrench_target.as_deref().is_some_and(|id| {
            id == CONTROLLER
                || id == FIREBOX
                || id == GEOTHERMAL_PLANT
                || id == THERMOELECTRIC_PLANT
                || thermal::spec(id).is_some()
                || id == SOLAR_CONTROLLER
                || id == SOLAR_CASING
                || id == SOLAR_COLLECTOR
        }) {
            return Flow::Continue;
        }

        if let HostSide::Server(host) = host {
            if let Some(level) = host.level(event.click.dimension.key().id()) {
                match wrench_target.as_deref() {
                    Some(CONTROLLER) => cracking_unit::wrench(
                        host,
                        level,
                        event.click.pos,
                        event.click.hit.face,
                        event.click.player,
                    ),
                    Some(FIREBOX) => firebox::wrench(
                        host,
                        level,
                        event.click.pos,
                        event.click.hit.face,
                        event.click.player,
                    ),
                    Some(GEOTHERMAL_PLANT) => geothermal::wrench(
                        host,
                        level,
                        event.click.pos,
                        event.click.hit.face,
                        event.click.player,
                    ),
                    Some(THERMOELECTRIC_PLANT) => thermoelectric::wrench(
                        host,
                        level,
                        event.click.pos,
                        event.click.hit.face,
                        event.click.player,
                    ),
                    Some(id) if thermal::spec(id).is_some() => thermal::wrench(
                        host,
                        level,
                        event.click.pos,
                        event.click.hit.face,
                        event.click.player,
                    ),
                    Some(SOLAR_CONTROLLER | SOLAR_CASING | SOLAR_COLLECTOR) => solar::wrench(
                        host,
                        level,
                        event.click.pos,
                        event.click.hit.face,
                        event.click.player,
                    ),
                    _ => {}
                }
            }
        }
        *event.cancellation_result = InteractionResult::Success;
        Flow::Cancel
    }

    fn player_destroyed_block(
        &self,
        host: &mut dyn ServerHost,
        event: &mut mod_api::PlayerDestroyedBlockEvent,
    ) {
        let block = host.block_state_string(event.state).unwrap_or_default();
        let Some(level) = host.level(host.dimension()) else {
            return;
        };
        if thermal::ids().iter().any(|id| block.starts_with(id)) {
            thermal::invalidate_level(level);
        }
        if block.starts_with(SOLAR_CONTROLLER) {
            solar::unform_broken_controller(host, level, event.pos);
        } else if block.starts_with(SOLAR_CASING) || block.starts_with(SOLAR_COLLECTOR) {
            solar::unform_near(host, level, event.pos);
        }
    }

    fn register(&self, registrar: &mut dyn Registrar, _: &RegistrationContext) {
        registrar.register_recipe_type("arcforge:cracking");
        registrar.register_raw_recipe_serializer("arcforge:cracking", "arcforge:cracking");

        let controller = registrar.block_key(CONTROLLER);
        let casing = registrar.block_key(CASING);
        let cracking_unit = registrar
            .block_entity_type("cracking_unit_controller")
            .blocks(&["cracking_unit_controller"])
            .build();
        registrar
            .block_entity_part(&cracking_unit, CrackingUnit { controller, casing })
            .expect("Arcforge's Cracking Unit block-entity Part should register");

        for id in heat_cell::ids() {
            let block_path = id.strip_prefix("arcforge:").unwrap_or(id);
            let heat_cell = registrar
                .block_entity_type(block_path)
                .blocks(&[block_path])
                .build();
            registrar
                .block_entity_part(&heat_cell, HeatCell { id })
                .expect("Arcforge heat-cell block-entity Part should register");
        }

        for id in thermal::ids() {
            let block_path = id.strip_prefix("arcforge:").unwrap_or(id);
            let conduit = registrar
                .block_entity_type(block_path)
                .blocks(&[block_path])
                .build();
            registrar
                .block_entity_part(&conduit, ThermalConduit { id })
                .expect("Arcforge thermal-conduit block-entity Part should register");
        }

        let firebox = registrar
            .block_entity_type("firebox")
            .blocks(&["firebox"])
            .build();
        registrar
            .block_entity_part(&firebox, Firebox)
            .expect("Arcforge's Firebox block-entity Part should register");

        let geothermal = registrar
            .block_entity_type("geothermal_plant")
            .blocks(&["geothermal_plant"])
            .build();
        registrar
            .block_entity_part(&geothermal, GeothermalPlant)
            .expect("Arcforge's Geothermal Plant block-entity Part should register");

        let thermoelectric_key = registrar.block_key(THERMOELECTRIC_PLANT);
        let thermoelectric = registrar
            .block_entity_type("thermoelectric_plant")
            .blocks(&["thermoelectric_plant"])
            .build();
        registrar
            .block_entity_part(&thermoelectric, ThermoelectricPlant)
            .expect("Arcforge's Thermoelectric Plant block-entity Part should register");
        registrar.block_container(
            &thermoelectric_key,
            ModContainer {
                slots: 3,
                faces: std::array::from_fn(|_| vec![0, 1, 2]),
                inputs: vec![0, 1, 2],
                outputs: vec![],
                menu: Some(THERMOELECTRIC_MENU.into()),
            },
        );
        registrar.block_energy(
            &thermoelectric_key,
            ModEnergyBuffer {
                capacity: thermoelectric::FE_CAPACITY as i32,
                max_insert: 0,
                max_extract: thermoelectric::MAX_FE_OUTPUT as i32,
                insert_faces: [false; 6],
                // The current storage declaration has static faces; Wrench-selected ports still
                // govern the plant's scripted adjacent transfer.
                extract_faces: [true; 6],
            },
        );
        registrar.menu_part(THERMOELECTRIC_MENU, ThermoelectricPlant);

        registrar.block_container(
            &controller,
            ModContainer {
                slots: 2,
                faces: [
                    vec![0, 1],
                    vec![0, 1],
                    vec![0, 1],
                    vec![0, 1],
                    vec![0, 1],
                    vec![0, 1],
                ],
                inputs: vec![0, 1],
                outputs: vec![],
                menu: Some(MACHINE_MENU.into()),
            },
        );
        let mut fluids = ModFluidTanks::default();
        fluids.tanks = vec![32_000, 16_000, 16_000, 16_000, 16_000, 16_000];
        fluids.faces = std::array::from_fn(|_| vec![0, 1, 2, 3, 4, 5]);
        fluids.inputs = vec![0, 1];
        fluids.outputs = vec![2, 3, 4, 5];
        registrar.block_fluids(&controller, fluids);
        registrar.menu_part(MACHINE_MENU, CrackingUnit { controller, casing });

        let firebox_key = registrar.block_key(FIREBOX);
        registrar.block_container(
            &firebox_key,
            ModContainer {
                slots: 4,
                faces: std::array::from_fn(|_| vec![0, 1]),
                inputs: vec![0],
                outputs: vec![1],
                menu: Some(FIREBOX_MENU.into()),
            },
        );
        let mut firebox_fluids = ModFluidTanks::default();
        firebox_fluids.tanks = vec![
            firebox::FLUE_TANK_CAPACITY as i32,
            firebox::OXYGEN_TANK_CAPACITY as i32,
        ];
        firebox_fluids.faces = std::array::from_fn(|_| vec![0, 1]);
        firebox_fluids.inputs = vec![1];
        firebox_fluids.outputs = vec![0];
        registrar.block_fluids(&firebox_key, firebox_fluids);
        registrar.block_entity_capability_before_game(&mod_api::FLUIDS, &firebox, |_, at| {
            Some(Box::new(firebox::FireboxFluids::new(at))
                as Box<dyn mod_api::ResourceHandler<mod_api::FluidResource>>)
        });
        registrar.block_entity_capability_before_game(&mod_api::ITEMS, &firebox, |_, at| {
            Some(Box::new(firebox::FireboxItems::new(at))
                as Box<dyn mod_api::ResourceHandler<mod_api::ItemResource>>)
        });
        registrar.menu_part(FIREBOX_MENU, Firebox);

        let geothermal_key = registrar.block_key(GEOTHERMAL_PLANT);
        registrar.block_container(
            &geothermal_key,
            ModContainer {
                slots: 4,
                faces: std::array::from_fn(|_| vec![0, 1]),
                inputs: vec![0, 2, 3],
                outputs: vec![1],
                menu: Some(GEOTHERMAL_MENU.into()),
            },
        );
        let mut geothermal_fluids = ModFluidTanks::default();
        geothermal_fluids.tanks = vec![geothermal::LAVA_TANK_CAPACITY as i32];
        geothermal_fluids.faces = std::array::from_fn(|_| vec![0]);
        geothermal_fluids.inputs = vec![0];
        geothermal_fluids.outputs = vec![];
        registrar.block_fluids(&geothermal_key, geothermal_fluids);
        registrar.block_entity_capability_before_game(&mod_api::FLUIDS, &geothermal, |_, at| {
            Some(Box::new(geothermal::GeothermalFluids::new(at))
                as Box<dyn mod_api::ResourceHandler<mod_api::FluidResource>>)
        });
        registrar.block_entity_capability_before_game(&mod_api::ITEMS, &geothermal, |_, at| {
            Some(Box::new(geothermal::GeothermalItems::new(at))
                as Box<dyn mod_api::ResourceHandler<mod_api::ItemResource>>)
        });
        registrar.menu_part(GEOTHERMAL_MENU, GeothermalPlant);

        let solar_key = registrar.block_key(SOLAR_CONTROLLER);
        let solar_controller = registrar
            .block_entity_type("solar_thermal_array_controller")
            .blocks(&["solar_thermal_array_controller"])
            .build();
        registrar
            .block_entity_part(&solar_controller, SolarThermalArray)
            .expect("Arcforge's Solar Thermal Array Part should register");
        registrar.block_container(
            &solar_key,
            ModContainer {
                slots: 1,
                faces: std::array::from_fn(|_| Vec::new()),
                inputs: vec![],
                outputs: vec![],
                menu: Some(SOLAR_MENU.into()),
            },
        );
        registrar.menu_part(SOLAR_MENU, SolarThermalArray);
    }
}
