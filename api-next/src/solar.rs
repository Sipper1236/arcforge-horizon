//! Current-API Solar Thermal Array: a fixed tower, sun/weather heat model, and configurable ports.

use mod_api::{
    BlockEntityTickEvent, BlockEntityTickReply, Compound, Host, LightLayer, ServerHost, Tag, Text,
};

pub(crate) const CONTROLLER: &str = "arcforge:solar_thermal_array_controller";
pub(crate) const CASING: &str = "arcforge:solar_thermal_array_casing";
pub(crate) const COLLECTOR: &str = "arcforge:solar_collector";

const WIDTH: i32 = 2;
const HEIGHT: i32 = 4;
const HEAT_CAPACITY: u64 = 16_000;
const MAX_TEMPERATURE_C: u32 = 1_100;
const SCAN_INTERVAL_TICKS: i32 = 40;
const REDSTONE_CHECK_INTERVAL_TICKS: i64 = 10;
const COOLING_SHARE: u64 = 200;
const REDSTONE_MODE_KEY: &str = "SolarRedstoneMode";
const DIRECTIONS: [(&str, [i32; 3]); 6] = [
    ("up", [0, 1, 0]),
    ("down", [0, -1, 0]),
    ("north", [0, 0, -1]),
    ("south", [0, 0, 1]),
    ("west", [-1, 0, 0]),
    ("east", [1, 0, 0]),
];

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Tower {
    pub(crate) min: [i32; 3],
    /// `true` tracks north/south; `false` tracks east/west.
    pub(crate) north_south: bool,
}

impl Tower {
    pub(crate) fn contains(self, pos: [i32; 3]) -> bool {
        pos[0] >= self.min[0]
            && pos[0] < self.min[0] + WIDTH
            && pos[1] >= self.min[1]
            && pos[1] < self.min[1] + HEIGHT
            && pos[2] >= self.min[2]
            && pos[2] < self.min[2] + WIDTH
    }

    fn positions(self) -> impl Iterator<Item = [i32; 3]> {
        (self.min[0]..self.min[0] + WIDTH).flat_map(move |x| {
            (self.min[1]..self.min[1] + HEIGHT)
                .flat_map(move |y| (self.min[2]..self.min[2] + WIDTH).map(move |z| [x, y, z]))
        })
    }

    fn collector(self, index: usize) -> [i32; 3] {
        [
            self.min[0] + (index % 2) as i32,
            self.min[1] + HEIGHT - 1,
            self.min[2] + (index / 2) as i32,
        ]
    }
}

pub(crate) fn tower_from_nbt(nbt: &Compound) -> Option<Tower> {
    if get_i32(nbt, "SolarFormed") == 0 {
        return None;
    }
    Some(Tower {
        min: [
            get_i32(nbt, "SolarMinX"),
            get_i32(nbt, "SolarMinY"),
            get_i32(nbt, "SolarMinZ"),
        ],
        north_south: get_string(nbt, "SolarAxis").as_deref() != Some("x"),
    })
}

pub(crate) fn controller_for_part(
    host: &dyn Host,
    level: mod_api::LevelRef,
    part: [i32; 3],
) -> Option<([i32; 3], Tower)> {
    for dy in -(HEIGHT - 1)..=0 {
        for dx in -1..=1 {
            for dz in -1..=1 {
                let controller = [part[0] + dx, part[1] + dy, part[2] + dz];
                let Some(entity) = host.block_entity(level, controller) else {
                    continue;
                };
                if entity.kind(host).map(|kind| kind.id(host)).as_deref() != Some(CONTROLLER) {
                    continue;
                }
                let Some(nbt) = entity.data(host) else {
                    continue;
                };
                let Some(tower) = tower_from_nbt(&nbt) else {
                    continue;
                };
                if tower.contains(part) {
                    return Some((controller, tower));
                }
            }
        }
    }
    None
}

pub(crate) fn port_enabled(nbt: &Compound, part: [i32; 3], face: &str) -> bool {
    get_string(nbt, &port_key(part, face)).as_deref() == Some("heat")
}

pub(crate) fn port_property(face: &str) -> String {
    format!("port_{face}")
}

fn sync_port_properties(
    host: &mut dyn ServerHost,
    tower: Tower,
    controller_nbt: &Compound,
) -> bool {
    let mut changed = false;
    for part in tower.positions() {
        for (face, offset) in DIRECTIONS {
            let neighbor = [
                part[0] + offset[0],
                part[1] + offset[1],
                part[2] + offset[2],
            ];
            let enabled = port_enabled(controller_nbt, part, face) && !tower.contains(neighbor);
            changed |= host.set_block_property(
                part,
                &port_property(face),
                if enabled { "true" } else { "false" },
            );
        }
    }
    changed
}

fn set_port(nbt: &mut Compound, part: [i32; 3], face: &str, enabled: bool) {
    let key = port_key(part, face);
    if enabled {
        set_string(nbt, &key, "heat");
    } else {
        nbt.remove(&key.as_str().into());
    }
}

fn port_key(part: [i32; 3], face: &str) -> String {
    format!("SolarPort_{}_{}_{}_{}", part[0], part[1], part[2], face)
}

fn write_tower(nbt: &mut Compound, tower: Option<Tower>) {
    set_i32(nbt, "SolarFormed", bool_i32(tower.is_some()));
    if let Some(tower) = tower {
        set_i32(nbt, "SolarMinX", tower.min[0]);
        set_i32(nbt, "SolarMinY", tower.min[1]);
        set_i32(nbt, "SolarMinZ", tower.min[2]);
        set_string(nbt, "SolarAxis", if tower.north_south { "z" } else { "x" });
    }
}

pub(crate) fn find_tower(
    host: &dyn Host,
    level: mod_api::LevelRef,
    controller: [i32; 3],
    facing: &str,
    current: Option<Tower>,
) -> (Option<Tower>, bool) {
    let north_south = matches!(facing, "north" | "south");
    let mut faces_in = false;
    for dx in -1..=0 {
        for dz in -1..=0 {
            let tower = Tower {
                min: [controller[0] + dx, controller[1], controller[2] + dz],
                north_south,
            };
            if !tower_valid(host, level, tower, controller, current) {
                continue;
            }
            let direction = match facing {
                "north" => [0, 0, -1],
                "east" => [1, 0, 0],
                "south" => [0, 0, 1],
                "west" => [-1, 0, 0],
                _ => [0, 0, -1],
            };
            let inside = [
                controller[0] + direction[0],
                controller[1] + direction[1],
                controller[2] + direction[2],
            ];
            if tower.contains(inside) {
                faces_in = true;
                continue;
            }
            return (Some(tower), faces_in);
        }
    }
    (None, faces_in)
}

fn tower_valid(
    host: &dyn Host,
    level: mod_api::LevelRef,
    tower: Tower,
    controller: [i32; 3],
    current: Option<Tower>,
) -> bool {
    for pos in tower.positions() {
        if !level.is_loaded(host, pos) {
            return false;
        }
        let Some(block) = level.block(host, pos) else {
            return false;
        };
        let layer = pos[1] - tower.min[1];
        let expected = if layer == HEIGHT - 1 {
            COLLECTOR
        } else if pos == controller {
            CONTROLLER
        } else {
            CASING
        };
        if block.id(host) != expected {
            return false;
        }
        if level.block_property(host, pos, "formed").as_deref() == Some("true")
            && !current.is_some_and(|old| old.contains(pos))
        {
            return false;
        }
    }
    true
}

fn set_formed(host: &mut dyn ServerHost, tower: Tower, controller: [i32; 3], formed: bool) -> bool {
    let mut changed = false;
    for pos in tower.positions() {
        if !formed {
            for (face, _) in DIRECTIONS {
                changed |= host.set_block_property(pos, &port_property(face), "false");
            }
        }
        if pos == controller {
            changed |=
                host.set_block_property(pos, "formed", if formed { "true" } else { "false" });
            changed |= host.set_block_property(
                pos,
                "base",
                if formed && pos == tower.min {
                    "true"
                } else {
                    "false"
                },
            );
            if !formed {
                changed |= host.set_block_property(pos, "lit", "false");
            }
        } else if pos[1] == tower.min[1] + HEIGHT - 1 {
            changed |=
                host.set_block_property(pos, "formed", if formed { "true" } else { "false" });
        } else {
            changed |=
                host.set_block_property(pos, "formed", if formed { "true" } else { "false" });
            let part = if !formed {
                "none"
            } else if pos == tower.min {
                "base"
            } else if pos == [tower.min[0], tower.min[1] + 2, tower.min[2]] {
                if tower.north_south {
                    "upper_ns"
                } else {
                    "upper_ew"
                }
            } else {
                "other"
            };
            changed |= host.set_block_property(pos, "part", part);
        }
    }
    changed
}

pub(crate) fn unform_near(host: &mut dyn ServerHost, level: mod_api::LevelRef, broken: [i32; 3]) {
    for dy in -(HEIGHT - 1)..=0 {
        for dx in -1..=1 {
            for dz in -1..=1 {
                let controller = [broken[0] + dx, broken[1] + dy, broken[2] + dz];
                let Some(entity) = host.block_entity(level, controller) else {
                    continue;
                };
                if entity.kind(&*host).map(|kind| kind.id(&*host)).as_deref() != Some(CONTROLLER) {
                    continue;
                }
                let Some(nbt) = entity.data(&*host) else {
                    continue;
                };
                let Some(tower) = tower_from_nbt(&nbt) else {
                    continue;
                };
                if !tower.contains(broken) {
                    continue;
                }
                set_formed(host, tower, controller, false);
                let mut updates = Compound::new();
                write_tower(&mut updates, None);
                let _ = entity.merge_data(host, updates);
                break;
            }
        }
    }
}

pub(crate) fn unform_broken_controller(
    host: &mut dyn ServerHost,
    level: mod_api::LevelRef,
    controller: [i32; 3],
) {
    for north_south in [true, false] {
        for dx in -1..=0 {
            for dz in -1..=0 {
                let tower = Tower {
                    min: [controller[0] + dx, controller[1], controller[2] + dz],
                    north_south,
                };
                if broken_controller_tower_valid(host, level, tower, controller) {
                    set_formed(host, tower, controller, false);
                    return;
                }
            }
        }
    }
}

fn broken_controller_tower_valid(
    host: &dyn Host,
    level: mod_api::LevelRef,
    tower: Tower,
    controller: [i32; 3],
) -> bool {
    if !tower.contains(controller) {
        return false;
    }
    for pos in tower.positions().filter(|pos| *pos != controller) {
        if !level.is_loaded(host, pos) {
            return false;
        }
        let Some(block) = level.block(host, pos) else {
            return false;
        };
        let expected = if pos[1] == tower.min[1] + HEIGHT - 1 {
            COLLECTOR
        } else {
            CASING
        };
        if block.id(host) != expected
            || level.block_property(host, pos, "formed").as_deref() != Some("true")
        {
            return false;
        }
    }
    true
}

pub(crate) fn wrench(
    host: &mut dyn ServerHost,
    level: mod_api::LevelRef,
    pos: [i32; 3],
    face: [i32; 3],
    player_uuid: u128,
) {
    let Some((controller, tower)) = controller_for_part(host, level, pos) else {
        if let Some(player) = host.player(player_uuid) {
            let _ = player.action_bar(host, Text::literal("Solar Thermal Array is not formed"));
        }
        return;
    };
    let Some(index) = DIRECTIONS
        .iter()
        .position(|(_, direction)| *direction == face)
    else {
        return;
    };
    let (face_name, offset) = DIRECTIONS[index];
    let Some(entity) = host.block_entity(level, controller) else {
        return;
    };
    let Some(nbt) = entity.data(&*host) else {
        return;
    };
    let enabled = !port_enabled(&nbt, pos, face_name);
    let mut updates = Compound::new();
    set_port(&mut updates, pos, face_name, enabled);
    let _ = entity.merge_data(host, updates);
    let neighbor = [pos[0] + offset[0], pos[1] + offset[1], pos[2] + offset[2]];
    let active = enabled && !tower.contains(neighbor);
    let _ = host.set_block_property(
        pos,
        &port_property(face_name),
        if active { "true" } else { "false" },
    );
    if let Some(player) = host.player(player_uuid) {
        let label = if enabled { "enabled" } else { "disabled" };
        let _ = player.action_bar(
            host,
            Text::literal(format!("Solar {face_name} heat port {label}")),
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Weather {
    Clear,
    Rain,
    Thunder,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ResultState {
    heat_per_tick: u64,
    temperature: u32,
    stowed: bool,
    night: bool,
}

fn compute(
    day_time: i64,
    weather: Weather,
    north_south: bool,
    sky_count: u32,
    has_sky: bool,
) -> ResultState {
    let day_tick = day_time.rem_euclid(24_000);
    let night = day_tick >= 12_000;
    let sun = if night {
        0.0
    } else {
        (std::f64::consts::PI * day_tick as f64 / 12_000.0).sin()
    };
    let weather_multiplier = match weather {
        Weather::Clear => 1.0,
        Weather::Rain => 0.3,
        Weather::Thunder => 0.1,
    };
    let axis_multiplier = if north_south { 1.0 } else { 0.7 };
    let intensity = if has_sky {
        sun * weather_multiplier * axis_multiplier
    } else {
        0.0
    };
    let sky_count = sky_count.min(4);
    let heat_per_tick = (600.0 * intensity * f64::from(sky_count) / 4.0).round() as u64;
    let temperature = if sky_count > 0 {
        (20.0 + (1_100.0 - 20.0) * intensity.min(1.0)).round() as u32
    } else {
        20
    };
    ResultState {
        heat_per_tick,
        temperature,
        stowed: night || weather == Weather::Thunder,
        night,
    }
}

pub(crate) fn tick(host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
    *event.reply = BlockEntityTickReply::Unchanged;
    let Some(level) = host.level(host.dimension()) else {
        return;
    };

    let mut changed = false;
    let previous = tower_from_nbt(&event.entity.nbt);
    let timer = get_i32(&event.entity.nbt, "SolarStructureCheck").saturating_sub(1);
    let tower = if timer <= 0 {
        let facing = level
            .block_property(host, event.pos, "facing")
            .unwrap_or_else(|| "north".to_owned());
        let (found, faces_in) = find_tower(host, level, event.pos, &facing, previous);
        let found = if faces_in { None } else { found };
        if found != previous {
            if let Some(old) = previous {
                set_formed(host, old, event.pos, false);
            }
            if let Some(found) = found {
                set_formed(host, found, event.pos, true);
                sync_port_properties(host, found, &event.entity.nbt);
            }
            write_tower(&mut event.entity.nbt, found);
            changed = true;
        }
        changed |= set_i32(
            &mut event.entity.nbt,
            "SolarStructureCheck",
            SCAN_INTERVAL_TICKS,
        );
        found
    } else {
        changed |= set_i32(&mut event.entity.nbt, "SolarStructureCheck", timer);
        previous
    };

    let mut heat = super::heat_cell::HeatStore::new(
        HEAT_CAPACITY,
        MAX_TEMPERATURE_C,
        get_i32(&event.entity.nbt, "HeatHU").max(0) as u64,
    );
    let has_sky = host.dimension() == "minecraft:overworld";
    let sky_count = tower
        .filter(|_| has_sky)
        .map(|tower| {
            (0..4)
                .filter(|index| {
                    let collector = tower.collector(*index);
                    level.brightness(
                        host,
                        LightLayer::Sky,
                        [collector[0], collector[1] + 1, collector[2]],
                    ) == Some(15)
                })
                .count() as u32
        })
        .unwrap_or(0);
    let (rain, thunder) = level.weather(host).unwrap_or((0.0, 0.0));
    let weather = if thunder > 0.0 {
        Weather::Thunder
    } else if rain > 0.0 {
        Weather::Rain
    } else {
        Weather::Clear
    };
    let result = compute(
        level.day_time(host).unwrap_or(0),
        weather,
        tower.is_some_and(|tower| tower.north_south),
        sky_count,
        has_sky,
    );

    let redstone_mode = RedstoneMode::from_value(get_i32(&event.entity.nbt, REDSTONE_MODE_KEY));
    changed |= set_i32(
        &mut event.entity.nbt,
        REDSTONE_MODE_KEY,
        redstone_mode as i32,
    );
    let time = level.game_time(host).unwrap_or_default();
    let mut powered = get_i32(&event.entity.nbt, "SolarPowered") != 0;
    if time % REDSTONE_CHECK_INTERVAL_TICKS == 0 {
        powered = tower.is_some_and(|tower| {
            tower
                .positions()
                .any(|pos| level.has_neighbor_signal(host, pos))
        });
        changed |= set_i32(&mut event.entity.nbt, "SolarPowered", bool_i32(powered));
    }
    let enabled = redstone_mode.can_run(powered);
    let heat_per_tick = if enabled { result.heat_per_tick } else { 0 };

    let safe_temperature = result.temperature.min(MAX_TEMPERATURE_C);
    let temperature_limit = heat_at_temperature(HEAT_CAPACITY, MAX_TEMPERATURE_C, safe_temperature);
    if heat_per_tick > 0 && heat.stored_hu() < temperature_limit {
        heat.add_hu(heat_per_tick.min(temperature_limit - heat.stored_hu()));
    }
    heat.set_source_temperature(if heat_per_tick > 0 {
        safe_temperature
    } else {
        20
    });
    if heat.stored_hu() > temperature_limit {
        let cooling_rate = (HEAT_CAPACITY / COOLING_SHARE).max(1);
        heat.remove_hu((heat.stored_hu() - temperature_limit).min(cooling_rate));
    }
    if let Some(tower) = tower {
        changed |= transfer_heat_ports(
            host,
            level,
            tower,
            &event.entity.nbt,
            &mut heat,
            heat_per_tick,
        );
    }
    let lit = tower.is_some() && heat_per_tick > 0;
    changed |= host.set_block_property(event.pos, "lit", if lit { "true" } else { "false" });

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
        "HeatPerTick",
        heat_per_tick.min(i32::MAX as u64) as i32,
    );
    changed |= set_i32(&mut event.entity.nbt, "SkyCollectors", sky_count as i32);
    changed |= set_i32(&mut event.entity.nbt, "Stowed", bool_i32(result.stowed));
    changed |= set_i32(&mut event.entity.nbt, "Night", bool_i32(result.night));

    let axis = bool_i32(tower.is_some_and(|tower| !tower.north_south));
    let weather_index = match weather {
        Weather::Clear => 0,
        Weather::Rain => 1,
        Weather::Thunder => 2,
    };
    let data = [
        (heat.stored_hu() / 10).min(i16::MAX as u64) as i32,
        (HEAT_CAPACITY / 10) as i32,
        heat.temperature_c().min(i16::MAX as u32) as i32,
        heat_per_tick.min(i16::MAX as u64) as i32,
        safe_temperature.min(i16::MAX as u32) as i32,
        sky_count as i32,
        bool_i32(result.stowed),
        bool_i32(tower.is_some()),
        weather_index,
        axis,
        redstone_mode as i32,
        bool_i32(enabled),
    ];
    if event.entity.data.as_slice() != data.as_slice() {
        event.entity.data = data.into();
        changed = true;
    }

    if changed {
        *event.reply = BlockEntityTickReply::Changed;
    }
}

fn transfer_heat_ports(
    host: &mut dyn ServerHost,
    level: mod_api::LevelRef,
    tower: Tower,
    controller_nbt: &Compound,
    heat: &mut super::heat_cell::HeatStore,
    heat_per_tick: u64,
) -> bool {
    let rate = heat_per_tick.max(100);
    let mut changed = false;
    for part in tower.positions() {
        for (face_index, (face, offset)) in DIRECTIONS.into_iter().enumerate() {
            if heat.temperature_c() <= 20 || heat.stored_hu() == 0 {
                return changed;
            }
            if !port_enabled(controller_nbt, part, face) {
                continue;
            }
            let neighbor = [
                part[0] + offset[0],
                part[1] + offset[1],
                part[2] + offset[2],
            ];
            if tower.contains(neighbor) {
                continue;
            }
            let moved = super::heat_cell::accept_heat(
                host,
                level,
                neighbor,
                super::heat_cell::opposite_face_index(face_index),
                heat,
                rate,
            )
                + super::cracking_unit::accept_heat(host, level, neighbor, heat, rate)
                + super::thermal::accept_heat(host, level, neighbor, part, heat, rate)
                + super::thermoelectric::accept_heat(
                    host,
                    level,
                    neighbor,
                    super::thermoelectric::opposite_face_index(face_index),
                    heat,
                    rate,
                );
            changed |= moved > 0;
        }
    }
    changed
}

pub(crate) fn screen() -> mod_api::Screen {
    mod_api::Screen {
        width: 176.0,
        height: 180.0,
        slots: Vec::new(),
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
    let output = data.get(3).copied().unwrap_or_default().max(0);
    let receiver = data.get(4).copied().unwrap_or_default().max(0);
    let sky = data.get(5).copied().unwrap_or_default().clamp(0, 4);
    let stowed = data.get(6).copied().unwrap_or_default() != 0;
    let formed = data.get(7).copied().unwrap_or_default() != 0;
    let weather = match data.get(8).copied().unwrap_or_default() {
        1 => "Rain",
        2 => "Thunder",
        _ => "Clear",
    };
    let axis = if data.get(9).copied().unwrap_or_default() == 0 {
        "North/South"
    } else {
        "East/West"
    };
    let redstone_mode = RedstoneMode::from_value(data.get(10).copied().unwrap_or_default());
    let enabled = data.get(11).copied().unwrap_or(1) != 0;
    let draw = &mut *event.draw;
    draw.text(
        "Solar Thermal Array",
        [ox + 8.0, oy + 6.0],
        0xFFFFFFFF,
        true,
    );
    draw.text(
        if formed {
            "Structure: Formed"
        } else {
            "Structure: Incomplete"
        },
        [ox + 8.0, oy + 19.0],
        0xFFFFFFFF,
        true,
    );
    draw.text(
        &format!("Receiver: {receiver} C ({axis})"),
        [ox + 8.0, oy + 32.0],
        0xFFBBBBBB,
        true,
    );
    draw.text(
        &format!("Sky: {sky}/4   Weather: {weather}"),
        [ox + 8.0, oy + 45.0],
        0xFFBBBBBB,
        true,
    );
    draw.fill([ox + 8.0, oy + 60.0, 160.0, 8.0], 0xFF343434);
    let heat_width = if capacity > 0 {
        (160.0 * heat.min(capacity) as f32 / capacity as f32).round()
    } else {
        0.0
    };
    if heat_width > 0.0 {
        draw.fill([ox + 8.0, oy + 60.0, heat_width, 8.0], 0xFFE37B32);
    }
    draw.text(
        &format!("Heat: {heat} / {capacity} HU"),
        [ox + 8.0, oy + 73.0],
        0xFFFFFFFF,
        true,
    );
    draw.text(
        &format!("Temperature: {temperature} C"),
        [ox + 8.0, oy + 86.0],
        0xFFFFFFFF,
        true,
    );
    draw.text(
        &format!("Current output: {output} HU/t"),
        [ox + 8.0, oy + 99.0],
        0xFFFFFFFF,
        true,
    );
    draw.text(
        if stowed {
            "Collector: Stowed"
        } else {
            "Collector: Tracking"
        },
        [ox + 8.0, oy + 112.0],
        0xFFBBBBBB,
        true,
    );
    draw.text(
        "Wrench: toggle a tower face's heat port",
        [ox + 8.0, oy + 130.0],
        0xFFBBBBBB,
        true,
    );
    draw.text("Redstone:", [ox + 8.0, oy + 141.0], 0xFFBBBBBB, true);
    for (mode, x, width) in [
        (RedstoneMode::Ignore, 62.0, 35.0),
        (RedstoneMode::High, 99.0, 28.0),
        (RedstoneMode::Low, 129.0, 27.0),
    ] {
        draw.fill(
            [ox + x, oy + 138.0, width, 12.0],
            if mode == redstone_mode {
                0xFF526A7A
            } else {
                0xFF343434
            },
        );
        draw.text(mode.label(), [ox + x + 3.0, oy + 140.0], 0xFFFFFFFF, true);
    }
    if formed && !enabled {
        draw.text(
            "Paused by redstone",
            [ox + 8.0, oy + 155.0],
            0xFFFF7777,
            true,
        );
    }
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
    }
}

fn heat_at_temperature(capacity: u64, maximum: u32, temperature: u32) -> u64 {
    capacity.saturating_mul(u64::from(temperature.saturating_sub(20)))
        / u64::from(maximum.saturating_sub(20).max(1))
}

fn get_i32(nbt: &Compound, key: &str) -> i32 {
    match nbt.get(&key.into()) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn bool_i32(value: bool) -> i32 {
    if value { 1 } else { 0 }
}

fn get_string(nbt: &Compound, key: &str) -> Option<String> {
    match nbt.get(&key.into()) {
        Some(Tag::String(value)) => Some(value.to_string_lossy()),
        _ => None,
    }
}

fn set_i32(nbt: &mut Compound, key: &str, value: i32) -> bool {
    if get_i32(nbt, key) == value {
        return false;
    }
    nbt.insert(key.into(), Tag::Int(value));
    true
}

fn set_string(nbt: &mut Compound, key: &str, value: &str) {
    nbt.insert(key.into(), Tag::String(value.into()));
}
