//! Current-API Firebox: solid fuel, stored heat, and bounded adjacent output.

use mod_api::{
    BlockEntityTickEvent, BlockEntityTickReply, BlockSide, Compound, Direction, FluidResource,
    Host, Identifier, ItemStack, RandomSource, ResourceHandler, ServerHost, Tag, Text, Transaction,
};

const CAPACITY: u64 = 40_000;
const MAX_TEMPERATURE_C: u32 = 1_100;
const HEAT_PER_TICK: u64 = 80;
const OUTPUT_PER_TARGET: u64 = 100;
const OUTPUT_PER_TICK: u64 = 600;
const MAX_UPGRADES: u32 = 8;
const SPEED_SLOT: usize = 2;
const HEAT_SLOT: usize = 3;
const FLUE_TANK: usize = 0;
const OXYGEN_TANK: usize = 1;
pub(crate) const FLUE_TANK_CAPACITY: u32 = 16_000;
pub(crate) const OXYGEN_TANK_CAPACITY: u32 = 2_000;
const SPEED_UPGRADE: &str = "arcforge:speed_upgrade";
const HEAT_UPGRADE: &str = "arcforge:heat_upgrade";
const WOOD_ASH: &str = "arcforge:wood_ash";
const CARBON_DIOXIDE: &str = "arcforge:carbon_dioxide";
const OXYGEN: &str = "arcforge:oxygen";
const OXY_FUEL_HEAT_BONUS_C: u32 = 300;
const OXY_FUEL_HEAT_MULTIPLIER: f64 = 1.25;
const OXYGEN_MILLI_PER_TICK: u64 = 250;

const PORT_KEYS: [&str; 6] = [
    "FireboxPortUp",
    "FireboxPortDown",
    "FireboxPortNorth",
    "FireboxPortSouth",
    "FireboxPortWest",
    "FireboxPortEast",
];
const PORT_NAMES: [&str; 6] = ["Up", "Down", "North", "South", "West", "East"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum PortMode {
    Heat = 0,
    Oxygen = 1,
    FlueGas = 2,
    Disabled = 3,
}

impl PortMode {
    fn from_value(value: i32) -> Self {
        match value {
            1 => Self::Oxygen,
            2 => Self::FlueGas,
            3 => Self::Disabled,
            _ => Self::Heat,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Heat => "Heat",
            Self::Oxygen => "Oxygen",
            Self::FlueGas => "Flue Gas",
            Self::Disabled => "Off",
        }
    }
}

macro_rules! java_string {
    ($value:expr) => {{
        let Tag::String(value) = Tag::String($value.into()) else {
            unreachable!()
        };
        value
    }};
}

const DIRECTIONS: [[i32; 3]; 6] = [
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, -1],
    [0, 0, 1],
    [-1, 0, 0],
    [1, 0, 0],
];

pub(crate) fn accepts_fuel(item: &str) -> bool {
    matches!(
        item,
        "minecraft:coal"
            | "minecraft:charcoal"
            | "arcforge:bio_coal"
            | "minecraft:coal_block"
            | "arcforge:bio_coal_block"
    )
}

fn fuel_ticks(item: &str) -> Option<u32> {
    Some(match item {
        "minecraft:coal" | "minecraft:charcoal" | "arcforge:bio_coal" => 800,
        "minecraft:coal_block" | "arcforge:bio_coal_block" => 8_000,
        _ => return None,
    })
}

pub(crate) fn tick(host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
    *event.reply = BlockEntityTickReply::Unchanged;
    let mut changed = event.entity.items.len() < 4;
    if changed {
        event.entity.items.resize(4, None);
    }
    let mut heat = super::heat_cell::HeatStore::new(
        CAPACITY,
        MAX_TEMPERATURE_C,
        get_i32(&event.entity.nbt, "HeatHU").max(0) as u64,
    );
    let mut burn = get_i32(&event.entity.nbt, "BurnTime").max(0) as u32;
    let speed_upgrades = upgrade_count(
        host,
        event.entity.items.get(SPEED_SLOT).and_then(Option::as_ref),
        SPEED_UPGRADE,
    );
    let heat_upgrades = upgrade_count(
        host,
        event.entity.items.get(HEAT_SLOT).and_then(Option::as_ref),
        HEAT_UPGRADE,
    );
    let speed = speed_upgrade_multiplier(speed_upgrades);

    if burn == 0 && heat.stored_hu() < CAPACITY {
        let fuel = event.entity.items[0].as_ref().and_then(|stack| {
            let item = stack.item().id(host);
            fuel_ticks(&item)
                .map(|ticks| scaled_burn_duration(ticks, speed))
                .map(|duration| (item, duration))
        });
        if let Some((fuel_item, ticks)) = fuel
            && consume_one(&mut event.entity.items[0])
        {
            burn = ticks;
            changed |= set_i32(&mut event.entity.nbt, "BurnDuration", burn as i32);
            changed |= set_string(&mut event.entity.nbt, "BurningItem", &fuel_item);
        }
    }

    let oxygen_available = fluid_is(&event.entity.nbt, OXYGEN_TANK, OXYGEN);
    let oxygen_active = burn > 0 && oxygen_available;
    let oxygen_multiplier = if oxygen_active {
        OXY_FUEL_HEAT_MULTIPLIER
    } else {
        1.0
    };
    let output_per_tick =
        (HEAT_PER_TICK as f64 * speed * heat_upgrade_multiplier(heat_upgrades) * oxygen_multiplier)
            .round() as u64;
    let produced = if burn > 0 {
        heat.add_hu(output_per_tick)
    } else {
        0
    };
    let oxygen_active = oxygen_active && produced > 0;
    heat.set_source_temperature(if produced > 0 {
        MAX_TEMPERATURE_C
            + if oxygen_active {
                OXY_FUEL_HEAT_BONUS_C
            } else {
                0
            }
    } else {
        20
    });

    if produced > 0 {
        if has_port_mode(&event.entity.nbt, PortMode::FlueGas) {
            changed |= add_flue_gas(&mut event.entity.nbt, produced);
        } else {
            changed |= set_i32(&mut event.entity.nbt, "FlueMilli", 0);
        }
        if oxygen_active {
            changed |= consume_oxygen(&mut event.entity.nbt);
        }
        burn = burn.saturating_sub(1);
        if burn == 0 {
            if let Some(item) = get_string(&event.entity.nbt, "BurningItem") {
                changed |= try_leave_wood_ash(host, &item, &mut event.entity.items[1]);
            }
            changed |= remove_tag(&mut event.entity.nbt, "BurningItem");
        }
    }

    changed |= set_i32(
        &mut event.entity.nbt,
        "BurnTime",
        burn.min(i32::MAX as u32) as i32,
    );
    let lit = burn > 0;
    changed |= host.set_block_property(event.pos, "lit", if lit { "true" } else { "false" });

    let level = host.level(host.dimension());
    if let Some(level) = level {
        transfer_heat(host, level, event.pos, &event.entity.nbt, &mut heat);
    }

    changed |= set_i32(
        &mut event.entity.nbt,
        "HeatHU",
        heat.stored_hu().min(i32::MAX as u64) as i32,
    );
    changed |= set_i32(
        &mut event.entity.nbt,
        "TemperatureC",
        heat.temperature_c().min(i32::MAX as u32) as i32,
    );

    let burn_duration = get_i32(&event.entity.nbt, "BurnDuration");
    let data = [
        (heat.stored_hu() / 10).min(i16::MAX as u64) as i32,
        (CAPACITY / 10).min(i16::MAX as u64) as i32,
        heat.temperature_c().min(i16::MAX as u32) as i32,
        burn.min(i16::MAX as u32) as i32,
        burn_duration.clamp(0, i16::MAX as i32),
        produced.min(i16::MAX as u64) as i32,
        speed_upgrades as i32,
        heat_upgrades as i32,
        fluid_amount(&event.entity.nbt, FLUE_TANK).min(i16::MAX as u32) as i32,
        fluid_amount(&event.entity.nbt, OXYGEN_TANK).min(i16::MAX as u32) as i32,
        if oxygen_active { 1 } else { 0 },
        port_mode_value(&event.entity.nbt, 0),
        port_mode_value(&event.entity.nbt, 1),
        port_mode_value(&event.entity.nbt, 2),
        port_mode_value(&event.entity.nbt, 3),
        port_mode_value(&event.entity.nbt, 4),
        port_mode_value(&event.entity.nbt, 5),
    ];
    if event.entity.data.as_slice() != data.as_slice() {
        event.entity.data = data.into();
        changed = true;
    }

    if changed {
        *event.reply = BlockEntityTickReply::Changed;
    }
}

fn transfer_heat(
    host: &mut dyn ServerHost,
    level: mod_api::LevelRef,
    pos: [i32; 3],
    nbt: &Compound,
    heat: &mut super::heat_cell::HeatStore,
) {
    let mut output_left = OUTPUT_PER_TICK;
    for (index, direction) in DIRECTIONS.into_iter().enumerate() {
        if heat.temperature_c() <= 20 || output_left == 0 {
            break;
        }
        if port_mode(nbt, index) != PortMode::Heat {
            continue;
        }
        let neighbor = [
            pos[0] + direction[0],
            pos[1] + direction[1],
            pos[2] + direction[2],
        ];
        let rate = OUTPUT_PER_TARGET.min(output_left);
        let moved = super::heat_cell::accept_heat(host, level, neighbor, heat, rate)
            + super::cracking_unit::accept_heat(host, level, neighbor, heat, rate);
        output_left = output_left.saturating_sub(moved);
    }
}

fn upgrade_count(host: &dyn Host, stack: Option<&ItemStack>, item_id: &str) -> u32 {
    stack
        .filter(|stack| stack.item().id(host) == item_id)
        .map_or(0, |stack| stack.count().min(MAX_UPGRADES))
}

fn speed_upgrade_multiplier(upgrades: u32) -> f64 {
    2.0_f64.powf(f64::from(upgrades.min(MAX_UPGRADES)) / 2.0)
}

fn heat_upgrade_multiplier(upgrades: u32) -> f64 {
    1.0 + 0.125 * f64::from(upgrades.min(MAX_UPGRADES))
}

fn scaled_burn_duration(base_ticks: u32, speed: f64) -> u32 {
    (f64::from(base_ticks) / speed)
        .round()
        .clamp(1.0, f64::from(u32::MAX)) as u32
}

pub(crate) fn screen() -> mod_api::Screen {
    mod_api::Screen {
        width: 176.0,
        height: 280.0,
        slots: vec![
            Some([16.0, 166.0]),
            Some([40.0, 166.0]),
            Some([104.0, 166.0]),
            Some([128.0, 166.0]),
        ],
        inventory_y: 206.0,
        inventory_x: 8.0,
        own_labels: true,
        own_background: false,
        ..mod_api::Screen::default()
    }
}

pub(crate) fn draw_screen(event: &mut mod_api::ScreenDrawEvent<'_>) {
    let [ox, oy] = event.origin;
    let data = event.data;
    let stored_hu = data.first().copied().unwrap_or_default().max(0) * 10;
    let capacity_hu = data.get(1).copied().unwrap_or_default().max(0) * 10;
    let temperature = data.get(2).copied().unwrap_or_default().max(0);
    let burn = data.get(3).copied().unwrap_or_default().max(0);
    let duration = data.get(4).copied().unwrap_or_default().max(0);
    let output = data.get(5).copied().unwrap_or_default().max(0);
    let speed_upgrades = data
        .get(6)
        .copied()
        .unwrap_or_default()
        .clamp(0, MAX_UPGRADES as i32);
    let heat_upgrades = data
        .get(7)
        .copied()
        .unwrap_or_default()
        .clamp(0, MAX_UPGRADES as i32);
    let flue_gas = data.get(8).copied().unwrap_or_default().max(0);
    let oxygen = data.get(9).copied().unwrap_or_default().max(0);
    let oxy_fuel = data.get(10).copied().unwrap_or_default() != 0;
    let port_modes = (0..6)
        .map(|index| PortMode::from_value(data.get(11 + index).copied().unwrap_or_default()))
        .collect::<Vec<_>>();
    let draw = &mut *event.draw;
    draw.text("Firebox", [ox + 8.0, oy + 6.0], 0xFFFFFFFF, true);
    draw.text(
        &format!("Temperature: {temperature} C"),
        [ox + 8.0, oy + 19.0],
        0xFFBBBBBB,
        true,
    );

    draw.fill([ox + 8.0, oy + 33.0, 160.0, 8.0], 0xFF343434);
    let heat_width = if capacity_hu > 0 {
        (160.0 * stored_hu.min(capacity_hu) as f32 / capacity_hu as f32).round()
    } else {
        0.0
    };
    if heat_width > 0.0 {
        draw.fill([ox + 8.0, oy + 33.0, heat_width, 8.0], 0xFFE37B32);
    }
    draw.text(
        &format!("Heat: {stored_hu} / {capacity_hu} HU"),
        [ox + 8.0, oy + 45.0],
        0xFFFFFFFF,
        true,
    );

    draw.fill([ox + 8.0, oy + 61.0, 160.0, 6.0], 0xFF343434);
    let burn_width = if duration > 0 {
        (160.0 * burn.min(duration) as f32 / duration as f32).round()
    } else {
        0.0
    };
    if burn_width > 0.0 {
        draw.fill([ox + 8.0, oy + 61.0, burn_width, 6.0], 0xFFB8492D);
    }
    draw.text(
        &format!("Burn: {burn} / {duration} ticks"),
        [ox + 8.0, oy + 72.0],
        0xFFFFFFFF,
        true,
    );
    draw.text(
        &format!("Heat output: {output} HU/t"),
        [ox + 8.0, oy + 84.0],
        0xFFFFFFFF,
        true,
    );
    draw.text(
        &format!("Flue gas: {flue_gas} mB CO2"),
        [ox + 8.0, oy + 94.0],
        0xFFBBBBBB,
        true,
    );
    draw.text(
        &format!("Oxygen: {oxygen} mB"),
        [ox + 8.0, oy + 104.0],
        0xFFBBBBBB,
        true,
    );
    draw.text(
        if oxy_fuel { "Oxy-fuel" } else { "Air flame" },
        [ox + 112.0, oy + 104.0],
        if oxy_fuel { 0xFFDDF8FF } else { 0xFFBBBBBB },
        true,
    );
    draw.text(
        "Ports (wrench cycles):",
        [ox + 8.0, oy + 118.0],
        0xFFBBBBBB,
        true,
    );
    draw.text(
        &format!(
            "U:{} D:{} N:{}",
            port_modes[0].label(),
            port_modes[1].label(),
            port_modes[2].label()
        ),
        [ox + 8.0, oy + 130.0],
        0xFFBBBBBB,
        true,
    );
    draw.text(
        &format!(
            "S:{} W:{} E:{}",
            port_modes[3].label(),
            port_modes[4].label(),
            port_modes[5].label()
        ),
        [ox + 8.0, oy + 142.0],
        0xFFBBBBBB,
        true,
    );
    draw.text("Fuel", [ox + 16.0, oy + 154.0], 0xFFBBBBBB, true);
    draw.text("Ash", [ox + 40.0, oy + 154.0], 0xFFBBBBBB, true);
    draw.text(
        &format!("S:{speed_upgrades}/{MAX_UPGRADES}"),
        [ox + 92.0, oy + 154.0],
        0xFFBBBBBB,
        true,
    );
    draw.text(
        &format!("H:{heat_upgrades}/{MAX_UPGRADES}"),
        [ox + 132.0, oy + 154.0],
        0xFFBBBBBB,
        true,
    );
}

fn add_flue_gas(nbt: &mut Compound, produced_hu: u64) -> bool {
    let fractional = get_i32(nbt, "FlueMilli").clamp(0, 999) as u64;
    let total_milli = fractional.saturating_add(produced_hu.saturating_mul(4));
    let amount = (total_milli / 1000).min(u64::from(u32::MAX)) as u32;
    let mut changed = set_i32(nbt, "FlueMilli", (total_milli % 1000) as i32);
    if amount > 0 {
        changed |= fluid_insert(nbt, FLUE_TANK, CARBON_DIOXIDE, amount, FLUE_TANK_CAPACITY) > 0;
    }
    changed
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

fn fluid_is(nbt: &Compound, index: usize, fluid: &str) -> bool {
    fluid_tanks(nbt)
        .get(index)
        .is_some_and(|(held, amount)| *amount > 0 && held.as_deref() == Some(fluid))
}

fn fluid_extract(nbt: &mut Compound, index: usize, fluid: &str, amount: u32) -> u32 {
    let mut tanks = fluid_tanks(nbt);
    let Some((held, stored)) = tanks.get_mut(index) else {
        return 0;
    };
    if held.as_deref() != Some(fluid) {
        return 0;
    }
    let removed = amount.min(*stored);
    if removed == 0 {
        return 0;
    }
    *stored -= removed;
    if *stored == 0 {
        *held = None;
    }
    write_fluid_tanks(nbt, &tanks);
    removed
}

fn consume_oxygen(nbt: &mut Compound) -> bool {
    if !fluid_is(nbt, OXYGEN_TANK, OXYGEN) {
        return false;
    }
    let fractional = get_i32(nbt, "OxygenMilli").clamp(0, 999) as u64;
    let total = fractional + OXYGEN_MILLI_PER_TICK;
    let consumed = (total / 1000).min(u64::from(u32::MAX)) as u32;
    let mut changed = set_i32(nbt, "OxygenMilli", (total % 1000) as i32);
    if consumed > 0 {
        changed |= fluid_extract(nbt, OXYGEN_TANK, OXYGEN, consumed) > 0;
    }
    changed
}

pub(crate) fn wrench(
    host: &mut dyn ServerHost,
    level: mod_api::LevelRef,
    pos: [i32; 3],
    face: [i32; 3],
    player_uuid: u128,
) {
    let Some(index) = DIRECTIONS.iter().position(|direction| *direction == face) else {
        return;
    };
    let Some(entity) = host.block_entity(level, pos) else {
        return;
    };
    let Some(nbt) = entity.data(&*host) else {
        return;
    };
    let current = port_mode(&nbt, index);
    let next = PortMode::from_value((current as i32 + 1) % 4);
    let mut updates = Compound::new();
    set_i32(&mut updates, PORT_KEYS[index], next as i32);
    let _ = entity.merge_data(host, updates);
    if let Some(player) = host.player(player_uuid) {
        let _ = player.action_bar(
            host,
            Text::literal(format!(
                "Firebox {} port: {}",
                PORT_NAMES[index],
                next.label()
            )),
        );
    }
}

fn port_mode(nbt: &Compound, index: usize) -> PortMode {
    PORT_KEYS.get(index).map_or(PortMode::Heat, |key| {
        PortMode::from_value(get_i32(nbt, key))
    })
}

fn port_mode_value(nbt: &Compound, index: usize) -> i32 {
    port_mode(nbt, index) as i32
}

fn has_port_mode(nbt: &Compound, mode: PortMode) -> bool {
    (0..PORT_KEYS.len()).any(|index| port_mode(nbt, index) == mode)
}

fn direction_index(direction: Option<Direction>) -> Option<usize> {
    Some(match direction? {
        Direction::Up => 0,
        Direction::Down => 1,
        Direction::North => 2,
        Direction::South => 3,
        Direction::West => 4,
        Direction::East => 5,
        _ => return None,
    })
}

/// A per-side fluid handler: Oxygen enters its selected port and Flue Gas leaves its own.
pub(crate) struct FireboxFluids(BlockSide);

impl FireboxFluids {
    pub(crate) fn new(at: BlockSide) -> Self {
        Self(at)
    }

    fn route(&self, host: &mut dyn ServerHost, external_index: usize) -> Option<(usize, PortMode)> {
        if external_index != 0 {
            return None;
        }
        let side = direction_index(self.0.side)?;
        let entity = host.block_entity(self.0.level, self.0.pos)?;
        let nbt = entity.data(&*host)?;
        match port_mode(&nbt, side) {
            PortMode::Oxygen => Some((OXYGEN_TANK, PortMode::Oxygen)),
            PortMode::FlueGas => Some((FLUE_TANK, PortMode::FlueGas)),
            PortMode::Heat | PortMode::Disabled => None,
        }
    }

    fn has_tank(&self, host: &mut dyn ServerHost, index: usize) -> bool {
        self.route(host, index).is_some_and(|(tank, _)| {
            mod_api::backend_of(host)
                .and_then(|backend| backend.fluid_tank_count(self.0))
                .is_some_and(|count| count > tank)
        })
    }
}

impl ResourceHandler<FluidResource> for FireboxFluids {
    fn size(&self, host: &mut dyn ServerHost) -> usize {
        usize::from(self.has_tank(host, 0))
    }

    fn resource(&self, host: &mut dyn ServerHost, index: usize) -> Option<FluidResource> {
        let (tank, _) = self.route(host, index)?;
        let held = mod_api::backend_of(host)?.fluid_tank(self.0, tank)?;
        Some(FluidResource::of(&held))
    }

    fn amount(&self, host: &mut dyn ServerHost, index: usize) -> u32 {
        let Some((tank, _)) = self.route(host, index) else {
            return 0;
        };
        mod_api::backend_of(host)
            .and_then(|backend| backend.fluid_tank(self.0, tank))
            .map_or(0, |held| held.amount)
    }

    fn capacity(&self, host: &mut dyn ServerHost, index: usize, resource: &FluidResource) -> u32 {
        let Some((tank, mode)) = self.route(host, index) else {
            return 0;
        };
        if mode != PortMode::Oxygen || !is_fluid(host, resource, OXYGEN) {
            return 0;
        }
        mod_api::backend_of(host).map_or(0, |backend| {
            backend.fluid_tank_capacity(self.0, tank, &resource.to_stack(1))
        })
    }

    fn insert(
        &self,
        host: &mut dyn ServerHost,
        tx: &Transaction,
        index: usize,
        resource: &FluidResource,
        amount: u32,
    ) -> u32 {
        let Some((tank, mode)) = self.route(host, index) else {
            return 0;
        };
        if mode != PortMode::Oxygen || !is_fluid(host, resource, OXYGEN) {
            return 0;
        }
        let Some(backend) = mod_api::backend_mut_of(host) else {
            return 0;
        };
        let rest = backend.fluid_tank_insert(self.0, tank, resource.to_stack(amount), tx);
        amount - rest.map_or(0, |rest| rest.amount.min(amount))
    }

    fn extract(
        &self,
        host: &mut dyn ServerHost,
        tx: &Transaction,
        index: usize,
        resource: &FluidResource,
        amount: u32,
    ) -> u32 {
        let Some((tank, mode)) = self.route(host, index) else {
            return 0;
        };
        if mode != PortMode::FlueGas || !is_fluid(host, resource, CARBON_DIOXIDE) {
            return 0;
        }
        let held = mod_api::backend_of(host).and_then(|backend| backend.fluid_tank(self.0, tank));
        if !held.is_some_and(|held| resource.matches(&held)) {
            return 0;
        }
        mod_api::backend_mut_of(host)
            .and_then(|backend| backend.fluid_tank_extract(self.0, tank, amount, tx))
            .map_or(0, |out| out.amount.min(amount))
    }
}

fn is_fluid(host: &dyn Host, resource: &FluidResource, id: &str) -> bool {
    host.fluid_key(id)
        .is_some_and(|key| resource.fluid().key() == Some(key))
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

pub(crate) fn screen_tooltip(at: [f32; 2], lines: &mut Vec<String>) {
    if (33.0..41.0).contains(&at[1]) && (8.0..168.0).contains(&at[0]) {
        lines.push("Stored heat HU / capacity".to_owned());
    } else if (61.0..67.0).contains(&at[1]) && (8.0..168.0).contains(&at[0]) {
        lines.push("Current fuel burn progress".to_owned());
    }
}

fn try_leave_wood_ash(host: &mut dyn ServerHost, item: &str, slot: &mut Option<ItemStack>) -> bool {
    if item != "minecraft:charcoal" {
        return false;
    }
    let dimension = host.dimension();
    let Some(level) = host.level(dimension) else {
        return false;
    };
    if level
        .with_random(host, |random| random.next_float() < 0.5)
        .unwrap_or(false)
    {
        insert_wood_ash(host, slot)
    } else {
        false
    }
}

fn insert_wood_ash(host: &dyn Host, slot: &mut Option<ItemStack>) -> bool {
    match slot {
        Some(stack) if stack.item().id(host) == WOOD_ASH && stack.count() < 64 => {
            stack.set_count(stack.count() + 1).is_ok()
        }
        Some(_) => false,
        None => {
            let Ok(id) = Identifier::parse(WOOD_ASH) else {
                return false;
            };
            let Some(item) = host.item_key(&id) else {
                return false;
            };
            *slot = Some(ItemStack::of(item));
            true
        }
    }
}

fn consume_one(slot: &mut Option<ItemStack>) -> bool {
    let Some(stack) = slot.as_mut() else {
        return false;
    };
    if stack.count() <= 1 {
        *slot = None;
    } else if stack.set_count(stack.count() - 1).is_err() {
        return false;
    }
    true
}

fn get_i32(nbt: &Compound, name: &str) -> i32 {
    match nbt.get(&name.into()) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn set_i32(nbt: &mut Compound, name: &str, value: i32) -> bool {
    if get_i32(nbt, name) == value {
        return false;
    }
    nbt.insert(name.into(), Tag::Int(value));
    true
}

fn get_string(nbt: &Compound, name: &str) -> Option<String> {
    match nbt.get(&name.into()) {
        Some(Tag::String(value)) => Some(value.to_string_lossy()),
        _ => None,
    }
}

fn set_string(nbt: &mut Compound, name: &str, value: &str) -> bool {
    if get_string(nbt, name).as_deref() == Some(value) {
        return false;
    }
    nbt.insert(name.into(), Tag::String(value.into()));
    true
}

fn remove_tag(nbt: &mut Compound, name: &str) -> bool {
    nbt.remove(&name.into()).is_some()
}
