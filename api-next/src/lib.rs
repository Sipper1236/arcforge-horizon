//! Staged port of Arcforge's native behavior to Horizon's current mod API.
//!
//! The production package remains under `api/` until the gameplay hooks and content
//! declarations have been migrated. New API work starts here so each slice can be checked
//! against the pinned Horizon SDK without breaking the existing package.

use mod_api::{
    Block, BlockEntityPart, BlockEntityTickEvent, BlockEntityTicksEvent, BlockKey, BlockPart,
    BlockStateProperty, Flow, Hooks, Host, HostSide, InteractionResult, Item, MenuContainer,
    MenuPart, MenuSlotRulesEvent, ModContainer, ModFluidTanks, Registrar, RegistrationContext,
    ServerHost,
};

mod cracking_unit;
mod heat_cell;

const CONTROLLER: &str = "arcforge:cracking_unit_controller";
const CASING: &str = "arcforge:cracking_unit_casing";
const CATALYST: &str = "arcforge:cracking_catalyst";
const SPEED_UPGRADE: &str = "arcforge:speed_upgrade";
const WRENCH: &str = "arcforge:wrench";
const MACHINE_MENU: &str = "arcforge:cracking_unit";

#[derive(Default)]
struct Arcforge;

struct CrackingUnit {
    controller: BlockKey,
    casing: BlockKey,
}

struct HeatCell {
    id: &'static str,
}

#[mod_api::horizon_part]
impl BlockPart for HeatCell {}

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
        event
            .items
            .push(Item::new(SPEED_UPGRADE, "minecraft:sugar"));
        let mut wrench = Item::new(WRENCH, "minecraft:iron_pickaxe");
        wrench.max_stack_size = Some(1);
        event.items.push(wrench);
    }

    fn right_click_block(
        &self,
        host: HostSide<'_>,
        event: &mut mod_api::RightClickBlockEvent<'_>,
    ) -> Flow {
        let is_wrench_on_controller = {
            let host: &dyn Host = match &host {
                HostSide::Server(host) => &**host,
                HostSide::Client(host) => &**host,
            };
            let held_wrench = event
                .click
                .stack
                .is_some_and(|stack| stack.item().id(host) == WRENCH);
            held_wrench
                && host
                    .level(event.click.dimension.key().id())
                    .and_then(|level| level.block(host, event.click.pos))
                    .is_some_and(|block| block.id(host) == CONTROLLER)
        };

        if !is_wrench_on_controller {
            return Flow::Continue;
        }

        if let HostSide::Server(host) = host {
            if let Some(level) = host.level(event.click.dimension.key().id()) {
                cracking_unit::wrench(
                    host,
                    level,
                    event.click.pos,
                    event.click.hit.face,
                    event.click.player,
                );
            }
        }
        *event.cancellation_result = InteractionResult::Success;
        Flow::Cancel
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
    }
}
