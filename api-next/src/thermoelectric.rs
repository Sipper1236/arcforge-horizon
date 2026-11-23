//! Current-API Thermoelectric Plant: converts stored HU to FE and routes both resources by face.

use mod_api::{
    BlockEntityTickEvent, BlockEntityTickReply, Compound, Direction, Host, HostBlockStates,
    HostTransaction, ServerHost, Tag, Text,
};

const PLANT: &str = "arcforge:thermoelectric_plant";
const SPEED_UPGRADE: &str = "arcforge:speed_upgrade";
const HEAT_UPGRADE: &str = "arcforge:heat_upgrade";
const THERMOELECTRIC_UPGRADE: &str = "arcforge:thermoelectric_upgrade";
const REDSTONE_MODE_KEY: &str = "ThermoelectricRedstoneMode";

const HEAT_CAPACITY: u64 = 20_000;
const MAX_TEMPERATURE_C: u32 = 1_400;
const FULL_EFFICIENCY_TEMPERATURE_C: u32 = 1_100;
const ZERO_EFFICIENCY_TEMPERATURE_C: u32 = 100;
const MAX_EFFICIENCY: f64 = 1.15;
pub(crate) const FE_CAPACITY: u64 = 50_000;
pub(crate) const MAX_FE_OUTPUT: u64 = 200;
const BASE_HEAT_THROUGHPUT: u64 = 80;
const MAX_UPGRADES: u32 = 8;
const THERMOELECTRIC_EFFICIENCY_PER_UPGRADE: f64 = 0.0625;

const DIRECTIONS: [[i32; 3]; 6] = [
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, -1],
    [0, 0, 1],
    [-1, 0, 0],
    [1, 0, 0],
];
const PORT_KEYS: [&str; 6] = [
    "ThermoelectricPortUp",
    "ThermoelectricPortDown",
    "ThermoelectricPortNorth",
    "ThermoelectricPortSouth",
    "ThermoelectricPortWest",
    "ThermoelectricPortEast",
];
const PORT_NAMES: [&str; 6] = ["Up", "Down", "North", "South", "West", "East"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum PortMode {
    None = 0,
    Heat = 1,
    Energy = 2,
}

impl PortMode {
    fn from_value(value: i32) -> Self {
        match value {
            1 => Self::Heat,
            2 => Self::Energy,
            _ => Self::None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Heat => "Heat",
            Self::Energy => "Energy",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RedstoneMode {
    Ignore = 0,
    High = 1,
    Low = 2,
}

impl RedstoneMode {
    fn from_value(value: i32) -> Self {
        match value {
            1 => Self::High,
            2 => Self::Low,
            _ => Self::Ignore,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Ignore => "Ignore",
            Self::High => "High",
            Self::Low => "Low",
        }
    }

    fn can_run(self, powered: bool) -> bool {
        match self {
            Self::Ignore => true,
            Self::High => powered,
            Self::Low => !powered,
        }
    }
}

/// Receives heat from staged heat sources when this plant's receiving face is set to Heat.
pub(crate) fn accept_heat(
    host: &mut dyn ServerHost,
    level: mod_api::LevelRef,
    pos: [i32; 3],
    receiving_face: usize,
    source: &mut super::heat_cell::HeatStore,
    rate: u64,
) -> u64 {
    let Some(block) = level.block(host, pos) else {
        return 0;
    };
    if block.id(host) != PLANT {
        return 0;
    }
    let Some(entity) = host.block_entity(level, pos) else {
        return 0;
    };
    let Some(nbt) = entity.data(&*host) else {
        return 0;
    };
    if port_mode(host, Some(level), pos, &nbt, receiving_face) != PortMode::Heat {
        return 0;
    }

    let tick = level.game_time(host).unwrap_or_default();
    let previous_tick = get_i64(&nbt, "ThermoelectricReceiveTick");
    let received = if previous_tick == tick {
        get_i32(&nbt, "ThermoelectricReceivedThisTick").max(0) as u64
    } else {
        0
    };
    let speed_upgrades = get_i32(&nbt, "SpeedUpgrades").clamp(0, MAX_UPGRADES as i32) as u32;
    let budget = heat_throughput(speed_upgrades)
        .saturating_sub(received)
        .min(rate);
    if budget == 0 {
        return 0;
    }

    let mut heat = super::heat_cell::HeatStore::new(
        HEAT_CAPACITY,
        MAX_TEMPERATURE_C,
        get_i32(&nbt, "HeatHU").max(0) as u64,
    );
    let moved = source.transfer_to(&mut heat, budget);
    if moved == 0 {
        return 0;
    }

    let mut updates = Compound::new();
    set_i32(
        &mut updates,
        "HeatHU",
        heat.stored_hu().min(i32::MAX as u64) as i32,
    );
    set_i32(
        &mut updates,
        "TemperatureC",
        heat.temperature_c().min(i32::MAX as u32) as i32,
    );
    set_i64(&mut updates, "ThermoelectricReceiveTick", tick);
    set_i32(
        &mut updates,
        "ThermoelectricReceivedThisTick",
        received.saturating_add(moved).min(i32::MAX as u64) as i32,
    );
    let _ = entity.merge_data(host, updates);
    moved
}

pub(crate) fn opposite_face_index(index: usize) -> usize {
    match index {
        0 => 1,
        1 => 0,
        2 => 3,
        3 => 2,
        4 => 5,
        5 => 4,
        _ => index,
    }
}

pub(crate) fn tick(host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
    *event.reply = BlockEntityTickReply::Unchanged;
    let mut changed = event.entity.items.len() < 3;
    if changed {
        event.entity.items.resize(3, None);
    }

    let speed_upgrades = upgrade_count(
        host,
        event.entity.items.first().and_then(Option::as_ref),
        SPEED_UPGRADE,
    );
    let heat_upgrades = upgrade_count(
        host,
        event.entity.items.get(1).and_then(Option::as_ref),
        HEAT_UPGRADE,
    );
    let thermoelectric_upgrades = upgrade_count(
        host,
        event.entity.items.get(2).and_then(Option::as_ref),
        THERMOELECTRIC_UPGRADE,
    );
    let redstone_mode = RedstoneMode::from_value(get_i32(&event.entity.nbt, REDSTONE_MODE_KEY));
    let level = host.level(host.dimension());
    let powered = level.is_some_and(|level| level.has_neighbor_signal(host, event.pos));
    let enabled = redstone_mode.can_run(powered);

    let mut heat = super::heat_cell::HeatStore::new(
        HEAT_CAPACITY,
        MAX_TEMPERATURE_C,
        get_i32(&event.entity.nbt, "HeatHU").max(0) as u64,
    );
    let mut energy = get_i32(&event.entity.nbt, "energy").max(0) as u64;
    let temperature = heat.temperature_c();
    let efficiency = upgraded_efficiency(temperature, heat_upgrades, thermoelectric_upgrades);
    let throughput = heat_throughput(speed_upgrades);
    let mut heat_used = 0_u64;
    let mut fe_generated = 0_u64;

    if enabled && energy < FE_CAPACITY && heat.stored_hu() > 0 {
        let wanted = div_ceil(throughput.saturating_mul(heat.stored_hu()), HEAT_CAPACITY);
        let wanted_fe = (wanted as f64 * efficiency) as u64;
        let room = FE_CAPACITY.saturating_sub(energy);
        let permitted_heat = if wanted_fe > room && efficiency > 0.0 {
            (room as f64 / efficiency) as u64
        } else {
            wanted
        };
        heat_used = heat.remove_hu(permitted_heat);
        fe_generated = ((heat_used as f64 * efficiency) as u64).min(room);
        energy = energy.saturating_add(fe_generated).min(FE_CAPACITY);
    }

    if let Some(level) = level {
        let transferred = transfer_energy(host, level, event.pos, &event.entity.nbt, energy);
        energy = energy.saturating_sub(transferred);
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
    changed |= set_i32(
        &mut event.entity.nbt,
        "energy",
        energy.min(i32::MAX as u64) as i32,
    );
    changed |= set_i32(
        &mut event.entity.nbt,
        "SpeedUpgrades",
        speed_upgrades as i32,
    );
    changed |= set_i32(&mut event.entity.nbt, "HeatUpgrades", heat_upgrades as i32);
    changed |= set_i32(
        &mut event.entity.nbt,
        "ThermoelectricUpgrades",
        thermoelectric_upgrades as i32,
    );
    changed |= set_i32(
        &mut event.entity.nbt,
        REDSTONE_MODE_KEY,
        redstone_mode as i32,
    );
    changed |= set_i32(
        &mut event.entity.nbt,
        "ThermoelectricEfficiencyPercent",
        (efficiency * 100.0).round().min(i32::MAX as f64) as i32,
    );
    changed |= host.set_block_property(
        event.pos,
        "lit",
        if fe_generated > 0 { "true" } else { "false" },
    );

    let mut data = [0_i32; 18];
    data[0] = (heat.stored_hu() / 10).min(i32::MAX as u64) as i32;
    data[1] = (HEAT_CAPACITY / 10) as i32;
    data[2] = heat.temperature_c().min(i32::MAX as u32) as i32;
    data[3] = (efficiency * 100.0).round().min(i32::MAX as f64) as i32;
    data[4] = heat_used.min(i32::MAX as u64) as i32;
    data[5] = (energy / 10).min(i32::MAX as u64) as i32;
    data[6] = (FE_CAPACITY / 10) as i32;
    data[7] = fe_generated.min(i32::MAX as u64) as i32;
    data[8] = speed_upgrades as i32;
    data[9] = heat_upgrades as i32;
    data[10] = thermoelectric_upgrades as i32;
    for index in 0..6 {
        data[11 + index] = port_mode(host, level, event.pos, &event.entity.nbt, index) as i32;
    }
    data[17] = redstone_mode as i32;
    if event.entity.data.as_slice() != data.as_slice() {
        event.entity.data = data.into();
        changed = true;
    }

    if changed {
        *event.reply = BlockEntityTickReply::Changed;
    }
}

fn transfer_energy(
    host: &mut dyn ServerHost,
    level: mod_api::LevelRef,
    pos: [i32; 3],
    nbt: &Compound,
    stored: u64,
) -> u64 {
    let mut budget = MAX_FE_OUTPUT.min(stored);
    let mut transferred = 0_u64;
    for (index, offset) in DIRECTIONS.into_iter().enumerate() {
        if budget == 0 {
            break;
        }
        if port_mode(host, Some(level), pos, nbt, index) != PortMode::Energy {
            continue;
        }
        let neighbor = [pos[0] + offset[0], pos[1] + offset[1], pos[2] + offset[2]];
        let Some(target) = level.capability(
            host,
            &mod_api::ENERGY,
            neighbor,
            Some(opposite(direction_at(index))),
        ) else {
            continue;
        };
        let offer = budget.min(u64::from(u32::MAX)) as u32;
        let inserted = host
            .transaction(|host, tx| {
                let inserted = target.insert(host, tx, offer);
                if inserted > offer {
                    Err(())
                } else {
                    Ok(inserted)
                }
            })
            .unwrap_or_default();
        transferred = transferred.saturating_add(u64::from(inserted));
        budget = budget.saturating_sub(u64::from(inserted));
    }
    transferred
}

fn upgraded_efficiency(
    temperature_c: u32,
    heat_upgrades: u32,
    thermoelectric_upgrades: u32,
) -> f64 {
    let base = efficiency(temperature_c);
    let heat_factor = 0.8_f64.powf(f64::from(heat_upgrades.min(MAX_UPGRADES)));
    let improved = (base / heat_factor).min(1.0).max(base);
    improved
        * (1.0
            + THERMOELECTRIC_EFFICIENCY_PER_UPGRADE
                * f64::from(thermoelectric_upgrades.min(MAX_UPGRADES)))
}

fn efficiency(temperature_c: u32) -> f64 {
    if temperature_c <= ZERO_EFFICIENCY_TEMPERATURE_C {
        0.0
    } else if temperature_c <= FULL_EFFICIENCY_TEMPERATURE_C {
        f64::from(temperature_c - ZERO_EFFICIENCY_TEMPERATURE_C)
            / f64::from(FULL_EFFICIENCY_TEMPERATURE_C - ZERO_EFFICIENCY_TEMPERATURE_C)
    } else {
        (1.0 + (MAX_EFFICIENCY - 1.0) * f64::from(temperature_c - FULL_EFFICIENCY_TEMPERATURE_C)
            / f64::from(MAX_TEMPERATURE_C - FULL_EFFICIENCY_TEMPERATURE_C))
        .min(MAX_EFFICIENCY)
    }
}

fn heat_throughput(speed_upgrades: u32) -> u64 {
    let speed = 2.0_f64.powf(f64::from(speed_upgrades.min(MAX_UPGRADES)) / 2.0);
    (BASE_HEAT_THROUGHPUT as f64 * f64::from(MAX_TEMPERATURE_C - 20)
        / f64::from(FULL_EFFICIENCY_TEMPERATURE_C - 20)
        * speed)
        .round()
        .max(1.0) as u64
}

pub(crate) fn screen() -> mod_api::Screen {
    mod_api::Screen {
        width: 176.0,
        height: 280.0,
        slots: vec![
            Some([40.0, 166.0]),
            Some([80.0, 166.0]),
            Some([120.0, 166.0]),
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
    let heat_capacity = data.get(1).copied().unwrap_or_default().max(0) * 10;
    let temperature = data.get(2).copied().unwrap_or_default().max(0);
    let efficiency = data.get(3).copied().unwrap_or_default().max(0);
    let heat_output = data.get(4).copied().unwrap_or_default().max(0);
    let energy = data.get(5).copied().unwrap_or_default().max(0) * 10;
    let energy_capacity = data.get(6).copied().unwrap_or_default().max(0) * 10;
    let fe_output = data.get(7).copied().unwrap_or_default().max(0);
    let upgrades = [
        data.get(8).copied().unwrap_or_default().clamp(0, 8),
        data.get(9).copied().unwrap_or_default().clamp(0, 8),
        data.get(10).copied().unwrap_or_default().clamp(0, 8),
    ];
    let ports = (0..6)
        .map(|index| PortMode::from_value(data.get(11 + index).copied().unwrap_or_default()))
        .collect::<Vec<_>>();
    let redstone_mode = RedstoneMode::from_value(data.get(17).copied().unwrap_or_default());
    let draw = &mut *event.draw;
    draw.text(
        "Thermoelectric Plant",
        [ox + 8.0, oy + 6.0],
        0xFFFFFFFF,
        true,
    );
    draw.text(
        &format!("Temperature: {temperature} C   Efficiency: {efficiency}%"),
        [ox + 8.0, oy + 19.0],
        0xFFBBBBBB,
        true,
    );
    draw.fill([ox + 8.0, oy + 33.0, 160.0, 8.0], 0xFF343434);
    if heat_capacity > 0 {
        let width = (160.0 * heat.min(heat_capacity) as f32 / heat_capacity as f32).round();
        if width > 0.0 {
            draw.fill([ox + 8.0, oy + 33.0, width, 8.0], 0xFFE37B32);
        }
    }
    draw.text(
        &format!("Heat: {heat} / {heat_capacity} HU"),
        [ox + 8.0, oy + 45.0],
        0xFFFFFFFF,
        true,
    );
    draw.text(
        &format!("Converted: {heat_output} HU/t   Output: {fe_output} FE/t"),
        [ox + 8.0, oy + 59.0],
        0xFFFFFFFF,
        true,
    );
    draw.text(
        &format!("FE: {energy} / {energy_capacity}"),
        [ox + 8.0, oy + 73.0],
        0xFFFFFFFF,
        true,
    );
    draw.fill([ox + 8.0, oy + 87.0, 160.0, 8.0], 0xFF343434);
    if energy_capacity > 0 {
        let width = (160.0 * energy.min(energy_capacity) as f32 / energy_capacity as f32).round();
        if width > 0.0 {
            draw.fill([ox + 8.0, oy + 87.0, width, 8.0], 0xFF43A9D5);
        }
    }
    draw.text(
        &format!(
            "Speed {}   Heat {}   Efficiency {}",
            upgrades[0], upgrades[1], upgrades[2]
        ),
        [ox + 8.0, oy + 101.0],
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
        [ox + 8.0, oy + 116.0],
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
        [ox + 8.0, oy + 128.0],
        0xFFBBBBBB,
        true,
    );
    draw.text("Redstone:", [ox + 8.0, oy + 141.0], 0xFFBBBBBB, true);
    for (mode, x, width) in [
        (RedstoneMode::Ignore, 62.0, 35.0),
        (RedstoneMode::High, 99.0, 28.0),
        (RedstoneMode::Low, 129.0, 27.0),
    ] {
        let selected = mode == redstone_mode;
        draw.fill(
            [ox + x, oy + 138.0, width, 12.0],
            if selected { 0xFF526A7A } else { 0xFF343434 },
        );
        draw.text(mode.label(), [ox + x + 3.0, oy + 140.0], 0xFFFFFFFF, true);
    }
    draw.text("Speed", [ox + 40.0, oy + 154.0], 0xFFBBBBBB, true);
    draw.text("Heat", [ox + 80.0, oy + 154.0], 0xFFBBBBBB, true);
    draw.text("Efficiency", [ox + 120.0, oy + 154.0], 0xFFBBBBBB, true);
}

pub(crate) fn screen_click(at: [f32; 2], right: bool) -> Option<i32> {
    if right || !(138.0..150.0).contains(&at[1]) {
        return None;
    }
    if (62.0..97.0).contains(&at[0]) {
        Some(RedstoneMode::Ignore as i32)
    } else if (99.0..127.0).contains(&at[0]) {
        Some(RedstoneMode::High as i32)
    } else if (129.0..156.0).contains(&at[0]) {
        Some(RedstoneMode::Low as i32)
    } else {
        None
    }
}

pub(crate) fn select_redstone_mode(nbt: &mut Compound, button: i32) {
    if (0..=2).contains(&button) {
        let _ = set_i32(
            nbt,
            REDSTONE_MODE_KEY,
            RedstoneMode::from_value(button) as i32,
        );
    }
}

pub(crate) fn screen_tooltip(at: [f32; 2], lines: &mut Vec<String>) {
    if screen_click(at, false).is_some() {
        lines.push("Select Ignore, High, or Low redstone control".to_owned());
    } else if (33.0..41.0).contains(&at[1]) && (8.0..168.0).contains(&at[0]) {
        lines.push("Stored heat HU / capacity".to_owned());
    } else if (87.0..95.0).contains(&at[1]) && (8.0..168.0).contains(&at[0]) {
        lines.push("Stored Forge Energy / capacity".to_owned());
    } else if (145.0..178.0).contains(&at[1]) && (24.0..152.0).contains(&at[0]) {
        lines.push("Install up to eight of each upgrade".to_owned());
    } else if (185.0..200.0).contains(&at[1]) && (8.0..168.0).contains(&at[0]) {
        lines.push("Wrench cycles a face through None, Heat and Energy".to_owned());
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
                "Thermoelectric {} port: {}",
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
    match index {
        0 | 1 => PortMode::Heat,
        _ if index == default_back_face(host, level, pos) => PortMode::Energy,
        _ => PortMode::None,
    }
}

fn default_back_face(host: &dyn Host, level: Option<mod_api::LevelRef>, pos: [i32; 3]) -> usize {
    level
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
        .unwrap_or(3)
}

fn upgrade_count(host: &dyn Host, stack: Option<&mod_api::ItemStack>, item_id: &str) -> u32 {
    stack
        .filter(|stack| stack.item().id(host) == item_id)
        .map_or(0, |stack| stack.count().min(MAX_UPGRADES))
}

fn div_ceil(value: u64, divisor: u64) -> u64 {
    value.saturating_add(divisor - 1) / divisor
}

fn direction_at(index: usize) -> Direction {
    match index {
        0 => Direction::Up,
        1 => Direction::Down,
        2 => Direction::North,
        3 => Direction::South,
        4 => Direction::West,
        _ => Direction::East,
    }
}

fn opposite(direction: Direction) -> Direction {
    match direction {
        Direction::Up => Direction::Down,
        Direction::Down => Direction::Up,
        Direction::North => Direction::South,
        Direction::South => Direction::North,
        Direction::West => Direction::East,
        Direction::East => Direction::West,
    }
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

fn get_i64(nbt: &Compound, name: &str) -> i64 {
    match nbt.get(&name.into()) {
        Some(Tag::Long(value)) => *value,
        _ => i64::MIN,
    }
}

fn set_i32(nbt: &mut Compound, name: &str, value: i32) -> bool {
    if get_i32(nbt, name) == value {
        return false;
    }
    nbt.insert(name.into(), Tag::Int(value));
    true
}

fn set_i64(nbt: &mut Compound, name: &str, value: i64) -> bool {
    if get_i64(nbt, name) == value {
        return false;
    }
    nbt.insert(name.into(), Tag::Long(value));
    true
}
