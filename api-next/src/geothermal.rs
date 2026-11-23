//! Current-API Geothermal Plant: bucket-fed lava, passive lava/magma heat, and sided output.

use mod_api::{
    BlockEntityTickEvent, BlockEntityTickReply, BlockSide, Compound, Direction, FluidResource,
    Host, HostBlockStates, Identifier, ItemResource, ItemStack, ResourceHandler, ServerHost, Tag,
    Text, Transaction,
};

const PLANT: &str = "arcforge:geothermal_plant";
const LAVA: &str = "minecraft:lava";
const LAVA_BUCKET: &str = "minecraft:lava_bucket";
const EMPTY_BUCKET: &str = "minecraft:bucket";
const SPEED_UPGRADE: &str = "arcforge:speed_upgrade";
const HEAT_UPGRADE: &str = "arcforge:heat_upgrade";
const CAPACITY: u64 = 60_000;
const MAX_TEMPERATURE_C: u32 = 1_400;
pub(crate) const LAVA_TANK_CAPACITY: u32 = 8_000;
const LAVA_HEAT_HU_PER_MB: u32 = 80;
const LAVA_PER_BURN: u32 = 100;
const LAVA_SOURCE_HEAT_HU_PER_TICK: u32 = 40;
const MAGMA_HEAT_HU_PER_TICK: u32 = 16;
const SCAN_INTERVAL_TICKS: i32 = 20;
const OUTPUT_PER_TARGET: u64 = 100;
const OUTPUT_PER_TICK: u64 = 600;
const MAX_UPGRADES: u32 = 8;
const LAVA_TANK: usize = 0;

const DIRECTIONS: [[i32; 3]; 6] = [
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, -1],
    [0, 0, 1],
    [-1, 0, 0],
    [1, 0, 0],
];
const PORT_KEYS: [&str; 6] = [
    "GeothermalPortUp",
    "GeothermalPortDown",
    "GeothermalPortNorth",
    "GeothermalPortSouth",
    "GeothermalPortWest",
    "GeothermalPortEast",
];
const PORT_NAMES: [&str; 6] = ["Up", "Down", "North", "South", "West", "East"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum PortMode {
    None = 0,
    Input = 1,
    Heat = 2,
}

impl PortMode {
    fn from_value(value: i32) -> Self {
        match value {
            1 => Self::Input,
            2 => Self::Heat,
            _ => Self::None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Input => "Input",
            Self::Heat => "Heat",
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

pub(crate) struct GeothermalItems(BlockSide);

impl GeothermalItems {
    pub(crate) fn new(at: BlockSide) -> Self {
        Self(at)
    }

    fn mode(&self, host: &mut dyn ServerHost) -> Option<PortMode> {
        let index = direction_index(self.0.side)?;
        let entity = host.block_entity(self.0.level, self.0.pos)?;
        let nbt = entity.data(&*host)?;
        Some(port_mode(host, Some(self.0.level), self.0.pos, &nbt, index))
    }

    fn route(&self, host: &mut dyn ServerHost, index: usize) -> Option<usize> {
        if self.0.side.is_none() {
            return (index < 2).then_some(index);
        }
        if self.mode(host)? != PortMode::Input {
            return None;
        }
        (index < 2).then_some(index)
    }

    fn accessible_size(&self, host: &mut dyn ServerHost) -> usize {
        if mod_api::backend_of(host)
            .and_then(|backend| backend.container_size(self.0))
            .is_none()
        {
            return 0;
        }
        if self.0.side.is_none() || self.mode(host) == Some(PortMode::Input) {
            2
        } else {
            0
        }
    }
}

impl ResourceHandler<ItemResource> for GeothermalItems {
    fn size(&self, host: &mut dyn ServerHost) -> usize {
        self.accessible_size(host)
    }

    fn resource(&self, host: &mut dyn ServerHost, index: usize) -> Option<ItemResource> {
        let slot = self.route(host, index)?;
        let held = mod_api::backend_of(host)?.container_stack(self.0, slot)?;
        Some(ItemResource::of(&held))
    }

    fn amount(&self, host: &mut dyn ServerHost, index: usize) -> u32 {
        let Some(slot) = self.route(host, index) else {
            return 0;
        };
        mod_api::backend_of(host)
            .and_then(|backend| backend.container_stack(self.0, slot))
            .map_or(0, |held| held.count())
    }

    fn capacity(&self, host: &mut dyn ServerHost, index: usize, resource: &ItemResource) -> u32 {
        let Some(slot) = self.route(host, index) else {
            return 0;
        };
        if slot != 0 || resource.as_stack().item().id(host) != LAVA_BUCKET {
            return 0;
        }
        mod_api::backend_of(host).map_or(0, |backend| {
            backend.container_capacity(self.0, slot, resource.as_stack())
        })
    }

    fn insert(
        &self,
        host: &mut dyn ServerHost,
        tx: &Transaction,
        index: usize,
        resource: &ItemResource,
        amount: u32,
    ) -> u32 {
        let Some(slot) = self.route(host, index) else {
            return 0;
        };
        if slot != 0 || resource.as_stack().item().id(host) != LAVA_BUCKET {
            return 0;
        }
        let Some(backend) = mod_api::backend_mut_of(host) else {
            return 0;
        };
        match backend.container_insert(self.0, slot, resource, amount, tx) {
            Ok(rest) => amount - rest.map_or(0, |rest| rest.count().min(amount)),
            Err(_) => 0,
        }
    }

    fn extract(
        &self,
        host: &mut dyn ServerHost,
        tx: &Transaction,
        index: usize,
        resource: &ItemResource,
        amount: u32,
    ) -> u32 {
        let Some(slot) = self.route(host, index) else {
            return 0;
        };
        if slot != 1 || resource.as_stack().item().id(host) != EMPTY_BUCKET {
            return 0;
        }
        let matches_bucket = mod_api::backend_of(host)
            .and_then(|backend| backend.container_stack(self.0, slot))
            .is_some_and(|held| held.item().id(host) == EMPTY_BUCKET);
        if !matches_bucket {
            return 0;
        }
        mod_api::backend_mut_of(host)
            .and_then(|backend| backend.container_extract(self.0, slot, amount, tx))
            .map_or(0, |out| out.count().min(amount))
    }
}

/// Fluid automation accepts only Lava on selected Input faces and unsided queries.
pub(crate) struct GeothermalFluids(BlockSide);

impl GeothermalFluids {
    pub(crate) fn new(at: BlockSide) -> Self {
        Self(at)
    }

    fn route(&self, host: &mut dyn ServerHost, external_index: usize) -> Option<usize> {
        if external_index != 0 {
            return None;
        }
        if let Some(side) = direction_index(self.0.side) {
            let entity = host.block_entity(self.0.level, self.0.pos)?;
            let nbt = entity.data(&*host)?;
            if port_mode(host, Some(self.0.level), self.0.pos, &nbt, side) != PortMode::Input {
                return None;
            }
        }
        Some(LAVA_TANK)
    }

    fn has_tank(&self, host: &mut dyn ServerHost, index: usize) -> bool {
        self.route(host, index).is_some_and(|tank| {
            mod_api::backend_of(host)
                .and_then(|backend| backend.fluid_tank_count(self.0))
                .is_some_and(|count| count > tank)
        })
    }
}

impl ResourceHandler<FluidResource> for GeothermalFluids {
    fn size(&self, host: &mut dyn ServerHost) -> usize {
        usize::from(self.has_tank(host, 0))
    }

    fn resource(&self, host: &mut dyn ServerHost, index: usize) -> Option<FluidResource> {
        let tank = self.route(host, index)?;
        let held = mod_api::backend_of(host)?.fluid_tank(self.0, tank)?;
        Some(FluidResource::of(&held))
    }

    fn amount(&self, host: &mut dyn ServerHost, index: usize) -> u32 {
        let Some(tank) = self.route(host, index) else {
            return 0;
        };
        mod_api::backend_of(host)
            .and_then(|backend| backend.fluid_tank(self.0, tank))
            .map_or(0, |held| held.amount)
    }

    fn capacity(&self, host: &mut dyn ServerHost, index: usize, resource: &FluidResource) -> u32 {
        let Some(tank) = self.route(host, index) else {
            return 0;
        };
        if !is_fluid(host, resource, LAVA) {
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
        let Some(tank) = self.route(host, index) else {
            return 0;
        };
        if !is_fluid(host, resource, LAVA) {
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
        _: &mut dyn ServerHost,
        _: &Transaction,
        _: usize,
        _: &FluidResource,
        _: u32,
    ) -> u32 {
        0
    }
}

fn is_fluid(host: &dyn Host, resource: &FluidResource, id: &str) -> bool {
    host.fluid_key(id)
        .is_some_and(|key| resource.fluid().key() == Some(key))
}

pub(crate) fn tick(host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
    *event.reply = BlockEntityTickReply::Unchanged;
    let mut changed = event.entity.items.len() < 4;
    if changed {
        event.entity.items.resize(4, None);
    }
    let mut items = std::mem::take(&mut event.entity.items);
    changed |= tick_lava_bucket(host, &mut event.entity.nbt, &mut items);
    event.entity.items = items;

    let Some(level) = host.level(host.dimension()) else {
        if changed {
            *event.reply = BlockEntityTickReply::Changed;
        }
        return;
    };
    let scan_timer = get_i32(&event.entity.nbt, "SurroundingsScan").saturating_sub(1);
    let (lava_sources, magma_blocks) = if scan_timer <= 0 {
        let counts = geothermal_surroundings(host, level, event.pos);
        changed |= set_i32(
            &mut event.entity.nbt,
            "SurroundingsScan",
            SCAN_INTERVAL_TICKS,
        );
        changed |= set_i32(&mut event.entity.nbt, "LavaSources", counts.0 as i32);
        changed |= set_i32(&mut event.entity.nbt, "MagmaBlocks", counts.1 as i32);
        counts
    } else {
        changed |= set_i32(&mut event.entity.nbt, "SurroundingsScan", scan_timer);
        (
            get_i32(&event.entity.nbt, "LavaSources").clamp(0, 6) as u32,
            get_i32(&event.entity.nbt, "MagmaBlocks").clamp(0, 6) as u32,
        )
    };

    let mut heat = super::heat_cell::HeatStore::new(
        CAPACITY,
        MAX_TEMPERATURE_C,
        get_i32(&event.entity.nbt, "HeatHU").max(0) as u64,
    );
    let speed_upgrades = upgrade_count(
        host,
        event.entity.items.get(2).and_then(Option::as_ref),
        SPEED_UPGRADE,
    );
    let heat_upgrades = upgrade_count(
        host,
        event.entity.items.get(3).and_then(Option::as_ref),
        HEAT_UPGRADE,
    );
    let speed = speed_upgrade_multiplier(speed_upgrades);
    let mut lava_burning_milli = get_i32(&event.entity.nbt, "LavaBurningMilli").max(0) as u32;
    let mut lava_burn_total = get_i32(&event.entity.nbt, "LavaBurnTotal").max(0) as u32;
    let mut heat_per_tick = 0_u64;

    if heat.stored_hu() < CAPACITY {
        if lava_burning_milli == 0 {
            let taken = fluid_extract(&mut event.entity.nbt, LAVA_TANK, LAVA, LAVA_PER_BURN);
            lava_burning_milli = taken.saturating_mul(1000);
            lava_burn_total = taken;
            changed |= taken > 0;
        }

        let passive = lava_sources
            .saturating_mul(LAVA_SOURCE_HEAT_HU_PER_TICK)
            .saturating_add(magma_blocks.saturating_mul(MAGMA_HEAT_HU_PER_TICK));
        let lava_heat = if lava_burning_milli > 0 {
            let drain_milli = ((speed * 1000.0).round() as u32).min(lava_burning_milli);
            lava_burning_milli -= drain_milli;
            (f64::from(LAVA_HEAT_HU_PER_MB) * f64::from(drain_milli) / 1000.0).round() as u32
        } else {
            0
        };
        let raw = passive.saturating_add(lava_heat);
        heat_per_tick =
            heat.add_hu((f64::from(raw) * heat_upgrade_multiplier(heat_upgrades)).round() as u64);
    }

    heat.set_source_temperature(if heat_per_tick > 0 {
        MAX_TEMPERATURE_C
    } else {
        20
    });
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
    changed |= set_i32(
        &mut event.entity.nbt,
        "LavaBurningMilli",
        lava_burning_milli.min(i32::MAX as u32) as i32,
    );
    changed |= set_i32(
        &mut event.entity.nbt,
        "LavaBurning",
        div_ceil(u64::from(lava_burning_milli), 1000).min(i32::MAX as u64) as i32,
    );
    changed |= set_i32(
        &mut event.entity.nbt,
        "LavaBurnTotal",
        lava_burn_total.min(i32::MAX as u32) as i32,
    );
    changed |= set_i32(
        &mut event.entity.nbt,
        "HeatPerTick",
        heat_per_tick.min(i32::MAX as u64) as i32,
    );

    changed |= host.set_block_property(
        event.pos,
        "lit",
        if heat_per_tick > 0 { "true" } else { "false" },
    );
    changed |= transfer_heat(host, level, event.pos, &event.entity.nbt, &mut heat);
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

    let modes = std::array::from_fn::<_, 6, _>(|index| {
        port_mode(host, Some(level), event.pos, &event.entity.nbt, index) as i32
    });
    let data = [
        (heat.stored_hu() / 10).min(i16::MAX as u64) as i32,
        (CAPACITY / 10).min(i16::MAX as u64) as i32,
        heat.temperature_c().min(i16::MAX as u32) as i32,
        fluid_amount(&event.entity.nbt, LAVA_TANK).min(i16::MAX as u32) as i32,
        LAVA_TANK_CAPACITY as i32,
        div_ceil(u64::from(lava_burning_milli), 1000).min(i16::MAX as u64) as i32,
        lava_burn_total.min(i16::MAX as u32) as i32,
        heat_per_tick.min(i16::MAX as u64) as i32,
        lava_sources as i32,
        magma_blocks as i32,
        speed_upgrades as i32,
        heat_upgrades as i32,
        modes[0],
        modes[1],
        modes[2],
        modes[3],
        modes[4],
        modes[5],
    ];
    if event.entity.data.as_slice() != data.as_slice() {
        event.entity.data = data.into();
        changed = true;
    }

    if changed {
        *event.reply = BlockEntityTickReply::Changed;
    }
}

fn tick_lava_bucket(host: &dyn Host, nbt: &mut Compound, items: &mut [Option<ItemStack>]) -> bool {
    let Some(input) = items.first().and_then(Option::as_ref) else {
        return false;
    };
    if input.item().id(host) != LAVA_BUCKET {
        return false;
    }
    let output = items.get(1).and_then(Option::as_ref);
    if output.is_some_and(|stack| stack.item().id(host) != EMPTY_BUCKET || stack.count() >= 64) {
        return false;
    }
    let Some(bucket_key) = Identifier::parse(EMPTY_BUCKET)
        .ok()
        .and_then(|id| host.item_key(&id))
    else {
        return false;
    };
    let inserted = fluid_insert(nbt, LAVA_TANK, LAVA, 1000, LAVA_TANK_CAPACITY);
    if inserted != 1000 {
        fluid_extract(nbt, LAVA_TANK, LAVA, inserted);
        return false;
    }
    if !consume_one(&mut items[0]) {
        fluid_extract(nbt, LAVA_TANK, LAVA, 1000);
        return false;
    }
    match items.get_mut(1) {
        Some(slot @ None) => *slot = Some(ItemStack::of(bucket_key)),
        Some(Some(stack)) => {
            if stack.set_count(stack.count() + 1).is_err() {
                fluid_extract(nbt, LAVA_TANK, LAVA, 1000);
                return false;
            }
        }
        None => return false,
    }
    true
}

fn geothermal_surroundings(host: &dyn Host, level: mod_api::LevelRef, pos: [i32; 3]) -> (u32, u32) {
    let mut lava_sources = 0_u32;
    let mut magma_blocks = 0_u32;
    for direction in DIRECTIONS {
        let neighbor = [
            pos[0] + direction[0],
            pos[1] + direction[1],
            pos[2] + direction[2],
        ];
        let Some(block) = level.block(host, neighbor) else {
            continue;
        };
        let id = block.id(host);
        match id.as_str() {
            "minecraft:lava" => {
                let Some(state) = level.block_state(host, neighbor) else {
                    continue;
                };
                let metadata = host.block_states();
                if state.property(&metadata, "level").ok().flatten() == Some("0") {
                    lava_sources += 1;
                }
            }
            "minecraft:magma_block" => magma_blocks += 1,
            _ => {}
        }
    }
    (lava_sources, magma_blocks)
}

fn transfer_heat(
    host: &mut dyn ServerHost,
    level: mod_api::LevelRef,
    pos: [i32; 3],
    nbt: &Compound,
    heat: &mut super::heat_cell::HeatStore,
) -> bool {
    let mut budget = OUTPUT_PER_TICK;
    let mut changed = false;
    for (index, offset) in DIRECTIONS.into_iter().enumerate() {
        if budget == 0 || heat.stored_hu() == 0 || heat.temperature_c() <= 20 {
            break;
        }
        if port_mode(host, Some(level), pos, nbt, index) != PortMode::Heat {
            continue;
        }
        let neighbor = [pos[0] + offset[0], pos[1] + offset[1], pos[2] + offset[2]];
        let limit = budget.min(OUTPUT_PER_TARGET);
        let moved = super::heat_cell::accept_heat(host, level, neighbor, heat, limit)
            + super::cracking_unit::accept_heat(host, level, neighbor, heat, limit)
            + super::thermoelectric::accept_heat(
                host,
                level,
                neighbor,
                super::thermoelectric::opposite_face_index(index),
                heat,
                limit,
            );
        changed |= moved > 0;
        budget = budget.saturating_sub(moved);
    }
    changed
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
    let heat = data.first().copied().unwrap_or_default().max(0) * 10;
    let capacity = data.get(1).copied().unwrap_or_default().max(0) * 10;
    let temperature = data.get(2).copied().unwrap_or_default().max(0);
    let lava = data.get(3).copied().unwrap_or_default().max(0);
    let lava_capacity = data.get(4).copied().unwrap_or_default().max(0);
    let burn = data.get(5).copied().unwrap_or_default().max(0);
    let burn_total = data.get(6).copied().unwrap_or_default().max(0);
    let heat_per_tick = data.get(7).copied().unwrap_or_default().max(0);
    let lava_sources = data.get(8).copied().unwrap_or_default().max(0);
    let magma_blocks = data.get(9).copied().unwrap_or_default().max(0);
    let speed_upgrades = data.get(10).copied().unwrap_or_default().clamp(0, 8);
    let heat_upgrades = data.get(11).copied().unwrap_or_default().clamp(0, 8);
    let ports = (0..6)
        .map(|index| PortMode::from_value(data.get(12 + index).copied().unwrap_or_default()))
        .collect::<Vec<_>>();
    let draw = &mut *event.draw;
    draw.text("Geothermal Plant", [ox + 8.0, oy + 6.0], 0xFFFFFFFF, true);
    draw.text(
        &format!("Temperature: {temperature} C"),
        [ox + 8.0, oy + 19.0],
        0xFFBBBBBB,
        true,
    );
    draw.fill([ox + 8.0, oy + 33.0, 160.0, 8.0], 0xFF343434);
    let heat_width = if capacity > 0 {
        (160.0 * heat.min(capacity) as f32 / capacity as f32).round()
    } else {
        0.0
    };
    if heat_width > 0.0 {
        draw.fill([ox + 8.0, oy + 33.0, heat_width, 8.0], 0xFFE37B32);
    }
    draw.text(
        &format!("Heat: {heat} / {capacity} HU"),
        [ox + 8.0, oy + 45.0],
        0xFFFFFFFF,
        true,
    );
    draw.text(
        &format!("Lava tank: {lava} / {lava_capacity} mB"),
        [ox + 8.0, oy + 59.0],
        0xFFFFFFFF,
        true,
    );
    draw.text(
        &format!("Burn: {burn} / {burn_total} mB"),
        [ox + 8.0, oy + 71.0],
        0xFFFFFFFF,
        true,
    );
    draw.text(
        &format!("Heat output: {heat_per_tick} HU/t"),
        [ox + 8.0, oy + 83.0],
        0xFFFFFFFF,
        true,
    );
    draw.text(
        &format!("Touching: {lava_sources} lava, {magma_blocks} magma"),
        [ox + 8.0, oy + 95.0],
        0xFFBBBBBB,
        true,
    );
    draw.text(
        &format!(
            "Ports U:{} D:{} N:{}",
            ports[0].label(),
            ports[1].label(),
            ports[2].label()
        ),
        [ox + 8.0, oy + 110.0],
        0xFFBBBBBB,
        true,
    );
    draw.text(
        &format!(
            "S:{} W:{} E:{}",
            ports[3].label(),
            ports[4].label(),
            ports[5].label()
        ),
        [ox + 8.0, oy + 122.0],
        0xFFBBBBBB,
        true,
    );
    draw.text(
        &format!("S:{speed_upgrades}/{MAX_UPGRADES}"),
        [ox + 8.0, oy + 138.0],
        0xFFBBBBBB,
        true,
    );
    draw.text(
        &format!("H:{heat_upgrades}/{MAX_UPGRADES}"),
        [ox + 88.0, oy + 138.0],
        0xFFBBBBBB,
        true,
    );
    draw.text("Lava", [ox + 16.0, oy + 154.0], 0xFFBBBBBB, true);
    draw.text("Bucket", [ox + 40.0, oy + 154.0], 0xFFBBBBBB, true);
    draw.text("Speed", [ox + 104.0, oy + 154.0], 0xFFBBBBBB, true);
    draw.text("Heat", [ox + 128.0, oy + 154.0], 0xFFBBBBBB, true);
}

pub(crate) fn screen_tooltip(at: [f32; 2], lines: &mut Vec<String>) {
    if (33.0..41.0).contains(&at[1]) && (8.0..168.0).contains(&at[0]) {
        lines.push("Stored heat HU / capacity".to_owned());
    } else if (59.0..69.0).contains(&at[1]) && (8.0..168.0).contains(&at[0]) {
        lines.push("Lava stored in the input tank".to_owned());
    }
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
    let current = port_mode(host, Some(level), pos, &nbt, index);
    let next = PortMode::from_value((current as i32 + 1) % 3);
    let mut updates = Compound::new();
    set_i32(&mut updates, PORT_KEYS[index], next as i32);
    let _ = entity.merge_data(host, updates);
    if let Some(player) = host.player(player_uuid) {
        let _ = player.action_bar(
            host,
            Text::literal(format!(
                "Geothermal {} port: {}",
                PORT_NAMES[index],
                next.label()
            )),
        );
    }
}

fn port_mode(
    host: &dyn Host,
    level: Option<mod_api::LevelRef>,
    pos: [i32; 3],
    nbt: &Compound,
    index: usize,
) -> PortMode {
    let Some(key) = PORT_KEYS.get(index) else {
        return PortMode::None;
    };
    if let Some(value) = get_i32_option(nbt, key) {
        return PortMode::from_value(value);
    }
    if index == 0 {
        return PortMode::Input;
    }
    let default_heat_face = level
        .and_then(|level| {
            let state = level.block_state(host, pos)?;
            let metadata = host.block_states();
            match state.property(&metadata, "facing").ok().flatten()? {
                "north" => Some(3),
                "south" => Some(2),
                "east" => Some(4),
                "west" => Some(5),
                _ => None,
            }
        })
        .unwrap_or(3);
    if index == default_heat_face {
        PortMode::Heat
    } else {
        PortMode::None
    }
}

fn direction_index(direction: Option<Direction>) -> Option<usize> {
    Some(match direction? {
        Direction::Up => 0,
        Direction::Down => 1,
        Direction::North => 2,
        Direction::South => 3,
        Direction::West => 4,
        Direction::East => 5,
    })
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

fn div_ceil(value: u64, divisor: u64) -> u64 {
    value.saturating_add(divisor - 1) / divisor
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

fn get_i32(nbt: &Compound, name: &str) -> i32 {
    match nbt.get(&name.into()) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn get_i32_option(nbt: &Compound, name: &str) -> Option<i32> {
    match nbt.get(&name.into()) {
        Some(Tag::Int(value)) => Some(*value),
        _ => None,
    }
}

fn set_i32(nbt: &mut Compound, name: &str, value: i32) -> bool {
    if get_i32(nbt, name) == value {
        return false;
    }
    nbt.insert(name.into(), Tag::Int(value));
    true
}
