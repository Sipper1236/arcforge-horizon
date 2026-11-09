//! Solar Thermal Array geometry and the source-derived sun/heat model.

use horizon_client::mod_api::{Compound, Host, Tag};

pub(crate) const CONTROLLER: &str = "arcforge:solar_thermal_array_controller";
pub(crate) const CASING: &str = "arcforge:solar_thermal_array_casing";
pub(crate) const COLLECTOR: &str = "arcforge:solar_collector";

const WIDTH: i32 = 2;
const HEIGHT: i32 = 4;

const PORT_FACES: [(&str, [i32; 3]); 6] = [
    ("up", [0, 1, 0]),
    ("down", [0, -1, 0]),
    ("north", [0, 0, -1]),
    ("south", [0, 0, 1]),
    ("west", [-1, 0, 0]),
    ("east", [1, 0, 0]),
];

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

    pub(crate) fn positions(self) -> impl Iterator<Item = [i32; 3]> {
        (self.min[0]..self.min[0] + WIDTH).flat_map(move |x| {
            (self.min[1]..self.min[1] + HEIGHT)
                .flat_map(move |y| (self.min[2]..self.min[2] + WIDTH).map(move |z| [x, y, z]))
        })
    }

    pub(crate) fn collector(self, index: usize) -> [i32; 3] {
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
    let tower = Tower {
        min: [
            get_i32(nbt, "SolarMinX"),
            get_i32(nbt, "SolarMinY"),
            get_i32(nbt, "SolarMinZ"),
        ],
        north_south: get_string(nbt, "SolarAxis").as_deref() != Some("x"),
    };
    Some(tower)
}

pub(crate) fn controller_for_part(host: &dyn Host, part: [i32; 3]) -> Option<([i32; 3], Tower)> {
    for dy in -(HEIGHT - 1)..=0 {
        for dx in -1..=1 {
            for dz in -1..=1 {
                let controller = [part[0] + dx, part[1] + dy, part[2] + dz];
                let Some(entity) = host.mod_block_entity(controller) else {
                    continue;
                };
                if entity.id != CONTROLLER {
                    continue;
                }
                let Some(tower) = tower_from_nbt(&entity.nbt) else {
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

pub(crate) fn sync_port_properties(host: &mut dyn Host, tower: Tower, nbt: &Compound) {
    for part in tower.positions() {
        for (face, offset) in PORT_FACES {
            let neighbor = [
                part[0] + offset[0],
                part[1] + offset[1],
                part[2] + offset[2],
            ];
            let enabled = port_enabled(nbt, part, face) && !tower.contains(neighbor);
            host.set_block_property(
                part,
                &port_property(face),
                if enabled { "true" } else { "false" },
            );
        }
    }
}

pub(crate) fn set_port_enabled(nbt: &mut Compound, part: [i32; 3], face: &str, enabled: bool) {
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

pub(crate) fn write_tower(nbt: &mut Compound, tower: Option<Tower>) {
    set_i32(nbt, "SolarFormed", if tower.is_some() { 1 } else { 0 });
    if let Some(tower) = tower {
        set_i32(nbt, "SolarMinX", tower.min[0]);
        set_i32(nbt, "SolarMinY", tower.min[1]);
        set_i32(nbt, "SolarMinZ", tower.min[2]);
        set_string(nbt, "SolarAxis", if tower.north_south { "z" } else { "x" });
    }
}

pub(crate) fn find_tower(
    host: &dyn Host,
    controller: [i32; 3],
    facing: &str,
    current: Option<Tower>,
) -> (Option<Tower>, bool) {
    let Some(level) = host.level(host.dimension()) else {
        return (None, false);
    };
    let mut faces_in = false;
    let north_south = matches!(facing, "north" | "south");
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
    level: horizon_client::mod_api::LevelRef,
    tower: Tower,
    controller: [i32; 3],
    current: Option<Tower>,
) -> bool {
    for pos in tower.positions() {
        if !level.is_loaded(host, pos) {
            return false;
        }
        let Some(id) = level.block(host, pos) else {
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
        if id != expected {
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

pub(crate) fn set_formed(host: &mut dyn Host, tower: Tower, formed: bool) {
    for pos in tower.positions() {
        if !formed {
            for (face, _) in PORT_FACES {
                host.set_block_property(pos, &port_property(face), "false");
            }
        }
        match host.block(pos).as_str() {
            CONTROLLER => {
                host.set_block_property(pos, "formed", if formed { "true" } else { "false" });
                host.set_block_property(
                    pos,
                    "base",
                    if formed && pos == tower.min {
                        "true"
                    } else {
                        "false"
                    },
                );
                if !formed {
                    host.set_block_property(pos, "lit", "false");
                }
            }
            CASING => {
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
                host.set_block_property(pos, "part", part);
            }
            COLLECTOR => {
                host.set_block_property(pos, "formed", if formed { "true" } else { "false" });
            }
            _ => {}
        }
    }
}

pub(crate) fn unform_near(host: &mut dyn Host, broken: [i32; 3]) {
    for dy in -(HEIGHT - 1)..=0 {
        for dx in -1..=1 {
            for dz in -1..=1 {
                let controller = [broken[0] + dx, broken[1] + dy, broken[2] + dz];
                let Some(entity) = host.mod_block_entity(controller) else {
                    continue;
                };
                if entity.id != CONTROLLER {
                    continue;
                }
                let Some(tower) = tower_from_nbt(&entity.nbt) else {
                    continue;
                };
                if !tower.contains(broken) {
                    continue;
                }
                set_formed(host, tower, false);
                host.change_mod_block_entity(controller, &mut |target| {
                    if target.id == CONTROLLER {
                        write_tower(&mut target.nbt, None);
                    }
                });
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Weather {
    Clear,
    Rain,
    Thunder,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ResultState {
    pub(crate) intensity: f64,
    pub(crate) heat_per_tick: u64,
    pub(crate) temperature: u32,
    pub(crate) stowed: bool,
    pub(crate) night: bool,
}

pub(crate) fn compute(
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
        intensity,
        heat_per_tick,
        temperature,
        stowed: night || weather == Weather::Thunder,
        night,
    }
}

fn get_i32(nbt: &Compound, key: &str) -> i32 {
    match nbt.get(&key.into()) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn get_string(nbt: &Compound, key: &str) -> Option<String> {
    match nbt.get(&key.into()) {
        Some(Tag::String(value)) => Some(value.to_string_lossy()),
        _ => None,
    }
}

fn set_i32(nbt: &mut Compound, key: &str, value: i32) {
    nbt.insert(key.into(), Tag::Int(value));
}

fn set_string(nbt: &mut Compound, key: &str, value: &str) {
    nbt.insert(key.into(), Tag::String(value.into()));
}
