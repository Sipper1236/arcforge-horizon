//! Steam Boiler Array. Its shell and heat behavior follow the pinned Arcforge
//! source; the Horizon fluid capability has no entity position, so tank volume
//! remains the fixed 3x3x3 alpha capacity while heat scales with the shell.
use horizon_client::{
    containers::ModBlockEntity,
    mod_api::{Compound, Host, Tag},
};

pub(crate) const CONTROLLER: &str = "arcforge:steam_boiler_array_controller";
pub(crate) const CASING: &str = "arcforge:steam_boiler_array_casing";
pub(crate) const WATER: &str = "minecraft:water";
pub(crate) const STEAM: &str = "arcforge:steam";
pub(crate) const HIGH_PRESSURE_STEAM: &str = "arcforge:high_pressure_steam";
pub(crate) const SUPERHEATED_STEAM: &str = "arcforge:superheated_steam";

/// Horizon's fluid capability is declared by block id only, so retain the
/// current alpha tank size instead of advertising a capacity that pipes could
/// exceed after the shell changes.
pub(crate) const TANK_CAPACITY: u32 = 48_000;
const AMBIENT_C: u32 = 20;
const MAX_TEMPERATURE_C: u32 = 1_400;
const MAX_HEAT_AT_MAX_SIZE_HU_T: f64 = 50_400.0;
const MAX_SECTIONS: f64 = 7.0 * 7.0 * 16.0 / 9.0;
const HEAT_PER_SECTION_HU: f64 = 100_000.0;
const HEAT_COST_MULTIPLIER: f64 = 0.8;
const SIZE_EFFICIENCY: f64 = 0.2;
const HOT_SAVING: f64 = 0.2;
const BOILING_GRACE_TICKS: i32 = 20;
const RATE_SMOOTHING: f64 = 0.1;
const CLIMB_SHARE: f64 = 0.5;
const CLIMB_SHARE_GROWTH: f64 = 0.005;
const HEAT_IN_SMOOTHING: f64 = 0.1;
const CEILING_MARGIN_C: u32 = 10;

#[derive(Clone, Copy, Debug, Default)]
struct ShellSize {
    width: u32,
    depth: u32,
    height: u32,
}

impl ShellSize {
    fn sections(self) -> f64 {
        f64::from(self.width) * f64::from(self.depth) / 9.0 * f64::from(self.height)
    }

    fn heat_capacity(self) -> u64 {
        (HEAT_PER_SECTION_HU * self.sections())
            .round()
            .clamp(1.0, i32::MAX as f64) as u64
    }

    fn max_heat_per_tick(self) -> u64 {
        (MAX_HEAT_AT_MAX_SIZE_HU_T * self.sections() / MAX_SECTIONS)
            .round()
            .max(1.0) as u64
    }

    fn heat_cost_multiplier(self) -> f64 {
        HEAT_COST_MULTIPLIER * (self.sections() / 3.0).max(1.0).powf(-SIZE_EFFICIENCY)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pressure {
    Auto = 0,
    Steam = 1,
    HighPressure = 2,
    Superheated = 3,
}

impl Pressure {
    fn from_nbt(nbt: &Compound) -> Self {
        match get_i32(nbt, "BoilerPressure") {
            1 => Self::Steam,
            2 => Self::HighPressure,
            3 => Self::Superheated,
            _ => Self::Auto,
        }
    }

    fn grade(self) -> Option<SteamGrade> {
        match self {
            Self::Auto => None,
            Self::Steam => Some(SteamGrade::Steam),
            Self::HighPressure => Some(SteamGrade::HighPressure),
            Self::Superheated => Some(SteamGrade::Superheated),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Steam => "Steam",
            Self::HighPressure => "High-Pressure",
            Self::Superheated => "Superheated",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Auto => Self::Steam,
            Self::Steam => Self::HighPressure,
            Self::HighPressure => Self::Superheated,
            Self::Superheated => Self::Auto,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SteamGrade {
    Steam,
    HighPressure,
    Superheated,
}

impl SteamGrade {
    const ALL: [Self; 3] = [Self::Steam, Self::HighPressure, Self::Superheated];

    fn min_temperature(self) -> u32 {
        match self {
            Self::Steam => 100,
            Self::HighPressure => 500,
            Self::Superheated => 900,
        }
    }

    fn hu_per_mb(self) -> f64 {
        match self {
            Self::Steam => 10.0,
            Self::HighPressure => 15.0,
            Self::Superheated => 20.0,
        }
    }

    fn fluid(self) -> &'static str {
        match self {
            Self::Steam => STEAM,
            Self::HighPressure => HIGH_PRESSURE_STEAM,
            Self::Superheated => SUPERHEATED_STEAM,
        }
    }

    fn for_temperature(temperature: u32) -> Option<Self> {
        Self::ALL
            .into_iter()
            .rev()
            .find(|grade| temperature >= grade.min_temperature())
    }

    fn from_fluid(fluid: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|grade| grade.fluid() == fluid)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BoilerState {
    Heating = 0,
    Boiling = 1,
    NoWater = 2,
    Venting = 3,
}

impl BoilerState {
    fn from_nbt(nbt: &Compound) -> Self {
        match get_i32(nbt, "BoilerState") {
            1 => Self::Boiling,
            2 => Self::NoWater,
            3 => Self::Venting,
            _ => Self::Heating,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Heating => "heating",
            Self::Boiling => "boiling",
            Self::NoWater => "no water",
            Self::Venting => "venting",
        }
    }

    fn active(self) -> bool {
        matches!(self, Self::Boiling | Self::Venting)
    }
}

#[derive(Clone, Copy, Debug)]
struct TickResult {
    boiled: u32,
    state: BoilerState,
}

pub(crate) fn tick(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    entity.items.resize(2, None);
    let facing = host
        .block_property(pos, "facing")
        .unwrap_or_else(|| "north".to_owned());
    let shell = find_shell(host, pos, &facing);
    let formed = shell.is_some();
    let shell_size = shell.unwrap_or(ShellSize {
        width: 3,
        depth: 3,
        height: 3,
    });
    set_i32(&mut entity.nbt, "BoilerFormed", i32::from(formed));
    set_i32(&mut entity.nbt, "BoilerWidth", shell_size.width as i32);
    set_i32(&mut entity.nbt, "BoilerDepth", shell_size.depth as i32);
    set_i32(&mut entity.nbt, "BoilerHeight", shell_size.height as i32);
    set_block_bool(host, pos, "formed", formed);

    if let Some(bucket) = entity.items[0].as_ref()
        && bucket.item.as_ref() == "minecraft:water_bucket"
        && crate::fluid::can_insert(&entity.nbt, 0, WATER, 1_000, TANK_CAPACITY)
        && crate::can_add(entity.items[1].as_ref(), "minecraft:bucket", 1)
    {
        let amount = crate::fluid::insert(&mut entity.nbt, 0, WATER, 1_000, TANK_CAPACITY);
        if amount == 1_000 && crate::consume_one(&mut entity.items[0]) {
            crate::add_items(&mut entity.items[1], "minecraft:bucket", 1);
        }
    }

    let mut heat = heat_store(&entity.nbt, shell_size);
    let result = if formed {
        boil_tick(&mut entity.nbt, &mut heat, shell_size)
    } else {
        set_i32(&mut entity.nbt, "LastStoredHU", -1);
        set_i32(&mut entity.nbt, "BoiledLastTick", 0);
        TickResult {
            boiled: 0,
            state: BoilerState::Heating,
        }
    };
    set_i32(&mut entity.nbt, "HeatHU", heat.stored_hu().min(i32::MAX as u64) as i32);
    set_i32(&mut entity.nbt, "TemperatureC", heat.temperature_c() as i32);
    set_i32(&mut entity.nbt, "BoilerState", result.state as i32);
    set_i32(&mut entity.nbt, "BoiledLastTick", result.boiled as i32);
    set_block_bool(host, pos, "lit", result.state.active());
}

fn boil_tick(
    nbt: &mut Compound,
    heat: &mut crate::heat::HeatStore,
    shell: ShellSize,
) -> TickResult {
    let max_heat = shell.max_heat_per_tick();
    let mut sim = BoilerSim::from_nbt(nbt);
    let stored = heat.stored_hu();
    if sim.last_stored < 0 && sim.heat_in <= 0.0 && heat.temperature_c() >= 100 {
        sim.heat_in = max_heat as f64;
    }
    let incoming = if sim.last_stored < 0 {
        0
    } else {
        stored.saturating_sub(sim.last_stored as u64).min(u32::MAX as u64) as u32
    };
    if incoming > 0 {
        sim.heat_in += (incoming as f64 - sim.heat_in) * HEAT_IN_SMOOTHING;
        sim.climb_share = (sim.climb_share + CLIMB_SHARE_GROWTH).min(CLIMB_SHARE);
    }
    let boiled = sim.boil(nbt, heat, shell, incoming, max_heat);
    let vented_heat = sim.vent_heat(heat, shell, max_heat);
    let vented = vented_heat || sim.vented_steam;
    sim.last_stored = heat.stored_hu().min(i32::MAX as u64) as i32;
    sim.rate += (boiled - sim.rate) * RATE_SMOOTHING;
    sim.heat_used += (sim.last_heat_per_mb * boiled - sim.heat_used) * RATE_SMOOTHING;

    let state = if boiled > 0.0 || sim.lifting {
        sim.boiling_grace = BOILING_GRACE_TICKS;
        BoilerState::Boiling
    } else if sim.last_state != BoilerState::Boiling {
        sim.boiling_grace = 0;
        sim.last_state
    } else if sim.boiling_grace > 0 {
        sim.boiling_grace -= 1;
        BoilerState::Boiling
    } else {
        BoilerState::Heating
    };
    if vented {
        sim.vent_grace = BOILING_GRACE_TICKS;
    } else if sim.vent_grace > 0 {
        sim.vent_grace -= 1;
    }
    let state = if sim.vent_grace > 0 && state != BoilerState::NoWater {
        BoilerState::Venting
    } else {
        state
    };
    // Persist all fractional work so sub-mB and sub-HU work survives save/reload.
    sim.write(nbt);
    set_i32(nbt, "BoilerState", state as i32);
    set_i32(nbt, "BoiledLastTick", boiled.floor().max(0.0) as i32);
    TickResult {
        boiled: boiled.floor().max(0.0) as u32,
        state,
    }
}

struct BoilerSim {
    pressure: Pressure,
    pending_steam: f64,
    pending_heat: f64,
    lifted: f64,
    heat_in: f64,
    climb_share: f64,
    last_stored: i32,
    boiling_grace: i32,
    vent_grace: i32,
    last_state: BoilerState,
    vented_steam: bool,
    lifting: bool,
    rate: f64,
    heat_used: f64,
    last_heat_per_mb: f64,
}

impl BoilerSim {
    fn from_nbt(nbt: &Compound) -> Self {
        Self {
            pressure: Pressure::from_nbt(nbt),
            pending_steam: get_f64(nbt, "PendingSteam").max(0.0),
            pending_heat: get_f64(nbt, "PendingHeat").max(0.0),
            lifted: get_f64(nbt, "LiftedSteam").max(0.0),
            heat_in: get_f64(nbt, "HeatIn").max(0.0),
            climb_share: get_f64_or(nbt, "ClimbShare", CLIMB_SHARE).clamp(0.0, CLIMB_SHARE),
            last_stored: nbt
                .get(&"LastStoredHU".into())
                .and_then(|tag| match tag {
                    Tag::Int(value) => Some(*value),
                    _ => None,
                })
                .unwrap_or(-1),
            boiling_grace: get_i32(nbt, "BoilingGrace").clamp(0, BOILING_GRACE_TICKS),
            vent_grace: get_i32(nbt, "VentGrace").clamp(0, BOILING_GRACE_TICKS),
            last_state: match get_i32(nbt, "LastBoilerState") {
                1 => BoilerState::Boiling,
                2 => BoilerState::NoWater,
                3 => BoilerState::Venting,
                _ => BoilerState::Heating,
            },
            vented_steam: false,
            lifting: false,
            rate: get_f64(nbt, "BoilerRate").max(0.0),
            heat_used: get_f64(nbt, "BoilerHeatUsed").max(0.0),
            last_heat_per_mb: get_f64(nbt, "BoilerLastHeatPerMb").max(0.0),
        }
    }

    fn write(&self, nbt: &mut Compound) {
        set_i32(nbt, "BoilerPressure", self.pressure as i32);
        set_i32(nbt, "LastStoredHU", self.last_stored);
        set_i32(nbt, "BoilingGrace", self.boiling_grace);
        set_i32(nbt, "VentGrace", self.vent_grace);
        set_i32(nbt, "LastBoilerState", self.last_state as i32);
        set_f64(nbt, "PendingSteam", self.pending_steam);
        set_f64(nbt, "PendingHeat", self.pending_heat);
        set_f64(nbt, "LiftedSteam", self.lifted);
        set_f64(nbt, "HeatIn", self.heat_in);
        set_f64(nbt, "ClimbShare", self.climb_share);
        set_f64(nbt, "BoilerRate", self.rate);
        set_f64(nbt, "BoilerHeatUsed", self.heat_used);
        set_f64(nbt, "BoilerLastHeatPerMb", self.last_heat_per_mb);
    }

    fn top_celsius(&self) -> u32 {
        let max = Pressure::Superheated.grade().expect("superheated grade");
        let grade = self.pressure.grade().unwrap_or(max);
        match grade {
            SteamGrade::Steam => SteamGrade::HighPressure.min_temperature() - 1,
            SteamGrade::HighPressure => SteamGrade::Superheated.min_temperature() - 1,
            SteamGrade::Superheated => MAX_TEMPERATURE_C - CEILING_MARGIN_C,
        }
    }

    fn band_position(&self, grade: SteamGrade, temperature: u32) -> f64 {
        let top = match grade {
            SteamGrade::Steam => SteamGrade::HighPressure.min_temperature() - 1,
            SteamGrade::HighPressure => SteamGrade::Superheated.min_temperature() - 1,
            SteamGrade::Superheated => MAX_TEMPERATURE_C - CEILING_MARGIN_C,
        };
        if top <= grade.min_temperature() {
            1.0
        } else {
            (f64::from(temperature.saturating_sub(grade.min_temperature()))
                / f64::from(top - grade.min_temperature()))
            .clamp(0.0, 1.0)
        }
    }

    fn boil(
        &mut self,
        nbt: &mut Compound,
        heat: &mut crate::heat::HeatStore,
        shell: ShellSize,
        incoming: u32,
        max_heat: u64,
    ) -> f64 {
        self.last_state = BoilerState::Heating;
        self.vented_steam = false;
        self.lifting = false;
        let temperature = heat.temperature_c();
        let Some(grade) = SteamGrade::for_temperature(temperature) else {
            return 0.0;
        };
        let set_grade = self.pressure.grade();
        let at_set = set_grade.is_some_and(|set| grade as u8 >= set as u8);
        let target = if at_set { set_grade.unwrap() } else { grade };
        let mut making = self.grade_to_make(nbt, target);
        if crate::fluid::amount(nbt, 0) == 0 {
            self.last_state = BoilerState::NoWater;
            return 0.0;
        }
        self.last_state = BoilerState::Boiling;
        let top = self.top_celsius();
        let above_top = heat.stored_hu() as f64
            - min_stored_at(shell.heat_capacity(), top) as f64
            - self.pending_heat;
        let floor = set_grade.filter(|_| at_set).map_or(100, SteamGrade::min_temperature);
        let above_floor = heat.stored_hu() as f64
            - min_stored_at(shell.heat_capacity(), floor) as f64
            - self.pending_heat;
        let usable = if incoming == 0 {
            self.climb_share /= 2.0;
            above_top.max(self.heat_in)
        } else {
            above_top.max(if temperature < top {
                f64::from(incoming) * (1.0 - self.climb_share)
            } else {
                0.0
            })
        };
        let mut budget = (usable.min(above_floor).min(max_heat as f64)).max(0.0);
        let steam_held = crate::fluid::amount(nbt, 1);
        if making != target {
            let lift_per_mb = (target.hu_per_mb() - making.hu_per_mb()) * shell.heat_cost_multiplier();
            let spend = budget.min((f64::from(steam_held) - self.lifted).max(0.0) * lift_per_mb);
            if lift_per_mb > 0.0 {
                self.lifted += spend / lift_per_mb;
            }
            self.pending_heat += spend;
            budget -= spend;
            self.lifting = spend > 0.0;
            if self.lifted >= f64::from(steam_held) {
                retag_steam(nbt, target);
                making = target;
                self.lifted = 0.0;
            }
        } else {
            self.lifted = 0.0;
        }
        let hu_per_mb = making.hu_per_mb()
            * shell.heat_cost_multiplier()
            * (1.0 - HOT_SAVING * self.band_position(making, temperature));
        self.last_heat_per_mb = hu_per_mb;
        let possible = budget / hu_per_mb;
        let water = f64::from(crate::fluid::amount(nbt, 0));
        let boil = possible.min(water - self.pending_steam).max(0.0);
        self.pending_steam += boil;
        self.pending_heat += boil * hu_per_mb;
        let mb = self
            .pending_steam
            .floor()
            .max(0.0)
            .min(water) as u32;
        let hu = self
            .pending_heat
            .floor()
            .max(0.0)
            .min(heat.stored_hu() as f64) as u64;
        self.pending_steam -= f64::from(mb);
        self.pending_heat -= hu as f64;
        heat.remove_hu(hu);
        if mb > 0 {
            let consumed = crate::fluid::extract(nbt, 0, WATER, mb);
            let accepted = crate::fluid::insert(nbt, 1, making.fluid(), consumed, TANK_CAPACITY);
            self.vented_steam = accepted < consumed;
        }
        boil
    }

    fn grade_to_make(&mut self, nbt: &mut Compound, grade: SteamGrade) -> SteamGrade {
        let amount = crate::fluid::amount(nbt, 1);
        self.lifted = self.lifted.min(f64::from(amount));
        let stored = crate::fluid::name(nbt, 1).and_then(|fluid| SteamGrade::from_fluid(&fluid));
        let Some(stored) = stored else {
            return grade;
        };
        if (grade as u8) < (stored as u8) {
            retag_steam(nbt, grade);
            grade
        } else {
            stored
        }
    }

    fn vent_heat(
        &mut self,
        heat: &mut crate::heat::HeatStore,
        shell: ShellSize,
        max_heat: u64,
    ) -> bool {
        let top = self.top_celsius();
        if heat.temperature_c() <= top {
            return false;
        }
        let minimum = min_stored_at(shell.heat_capacity(), top.saturating_add(1));
        let excess = heat.stored_hu().saturating_sub(minimum.saturating_sub(1));
        heat.remove_hu(excess.min(max_heat));
        excess > 0
    }
}

fn min_stored_at(capacity: u64, celsius: u32) -> u64 {
    let span = u64::from(MAX_TEMPERATURE_C - AMBIENT_C);
    let degrees = u64::from(celsius.saturating_sub(AMBIENT_C));
    ((capacity as u128 * degrees as u128 + span as u128 - 1) / span as u128)
        .min(u64::MAX as u128) as u64
}

fn retag_steam(nbt: &mut Compound, grade: SteamGrade) {
    let Some(old) = crate::fluid::name(nbt, 1) else {
        return;
    };
    if old == grade.fluid() {
        return;
    }
    let amount = crate::fluid::amount(nbt, 1);
    let removed = crate::fluid::extract(nbt, 1, &old, amount);
    crate::fluid::insert(nbt, 1, grade.fluid(), removed, TANK_CAPACITY);
}

fn heat_store(nbt: &Compound, shell: ShellSize) -> crate::heat::HeatStore {
    crate::heat::HeatStore::new(
        shell.heat_capacity(),
        MAX_TEMPERATURE_C,
        get_i32(nbt, "HeatHU").max(0) as u64,
    )
}

fn saved_shell(nbt: &Compound) -> ShellSize {
    let get = |key| get_i32(nbt, key).clamp(3, 16) as u32;
    ShellSize {
        width: get("BoilerWidth").min(7),
        depth: get("BoilerDepth").min(7),
        height: get("BoilerHeight"),
    }
}

fn find_shell(host: &dyn Host, controller: [i32; 3], facing: &str) -> Option<ShellSize> {
    let (out, right) = match facing {
        "north" => ([0, 0, -1], [1, 0, 0]),
        "east" => ([1, 0, 0], [0, 0, 1]),
        "south" => ([0, 0, 1], [-1, 0, 0]),
        "west" => ([-1, 0, 0], [0, 0, -1]),
        _ => return None,
    };
    let inward = [-out[0], -out[1], -out[2]];
    let mut candidates = Vec::with_capacity(350);
    for width in 3..=7 {
        for depth in 3..=7 {
            for height in 3..=16 {
                candidates.push((width * depth * height, width, depth, height));
            }
        }
    }
    candidates.sort_unstable();
    for (_, width, depth, height) in candidates {
        let edges_must_be_casing = width != 3 || depth != 3;
        let center_x = (width - 1) / 2;
        let center_y = (height - 1) / 2;
        let mut valid = true;
        'volume: for x in 0..width {
            for y in 0..height {
                for z in 0..depth {
                    let dx = x as i32 - center_x as i32;
                    let dy = y as i32 - center_y as i32;
                    let pos = [
                        controller[0] + right[0] * dx + inward[0] * z as i32,
                        controller[1] + dy,
                        controller[2] + right[2] * dx + inward[2] * z as i32,
                    ];
                    let boundaries = usize::from(x == 0 || x + 1 == width)
                        + usize::from(y == 0 || y + 1 == height)
                        + usize::from(z == 0 || z + 1 == depth);
                    let block = host.block(pos);
                    let is_controller = x == center_x && y == center_y && z == 0;
                    if is_controller {
                        if pos != controller || block != CONTROLLER {
                            valid = false;
                            break 'volume;
                        }
                    } else if boundaries == 3
                        || (edges_must_be_casing && boundaries >= 2)
                    {
                        if block != CASING {
                            valid = false;
                            break 'volume;
                        }
                    } else if boundaries > 0 {
                        let formed_glass = block == "arcforge:pressure_glass"
                            && host.block_property(pos, "formed").as_deref() == Some("true");
                        if (block != CASING && block != "arcforge:pressure_glass") || formed_glass {
                            valid = false;
                            break 'volume;
                        }
                    } else if block != "minecraft:air"
                        && block != "minecraft:cave_air"
                        && block != "minecraft:void_air"
                    {
                        valid = false;
                        break 'volume;
                    }
                }
            }
        }
        if valid {
            return Some(ShellSize {
                width,
                depth,
                height,
            });
        }
    }
    None
}

fn set_block_bool(host: &mut dyn Host, pos: [i32; 3], property: &str, value: bool) {
    let state = if value { "true" } else { "false" };
    if host.block_property(pos, property).as_deref() != Some(state) {
        host.set_block_property(pos, property, state);
    }
}

pub(crate) fn cycle_pressure(host: &mut dyn Host, pos: [i32; 3]) {
    let mut changed = false;
    host.change_mod_block_entity(pos, &mut |entity| {
        if entity.id == CONTROLLER {
            let next = Pressure::from_nbt(&entity.nbt).next();
            set_i32(&mut entity.nbt, "BoilerPressure", next as i32);
            changed = true;
        }
    });
    if changed {
        host.block_entity_sync(pos);
    }
}

pub(crate) fn status(entity: &ModBlockEntity) -> String {
    let shell = saved_shell(&entity.nbt);
    let formed = get_i32(&entity.nbt, "BoilerFormed") != 0;
    let water = crate::fluid::amount(&entity.nbt, 0);
    let steam = crate::fluid::amount(&entity.nbt, 1);
    let heat = get_i32(&entity.nbt, "TemperatureC").max(AMBIENT_C as i32);
    let gas = crate::fluid::name(&entity.nbt, 1).unwrap_or_else(|| "none".to_owned());
    let state = BoilerState::from_nbt(&entity.nbt);
    let pressure = Pressure::from_nbt(&entity.nbt);
    let rate = get_f64(&entity.nbt, "BoilerRate").max(0.0);
    let size = format!("{}x{}x{}", shell.width, shell.depth, shell.height);
    format!(
        "Boiler {} {size} | {state} | {pressure} | {heat}C | Water {water}/{TANK_CAPACITY} | {gas} {steam}/{TANK_CAPACITY} | {rate:.0} mB/t",
        if formed { "formed" } else { "unformed" },
        state = state.name(),
        pressure = pressure.name(),
    )
}

pub(crate) fn accept_heat(
    host: &mut dyn Host,
    pos: [i32; 3],
    source: &mut crate::heat::HeatStore,
    rate: u64,
) -> u64 {
    let mut moved = 0;
    host.change_mod_block_entity(pos, &mut |entity| {
        if entity.id != CONTROLLER {
            return;
        }
        let shell = saved_shell(&entity.nbt);
        let mut sink = heat_store(&entity.nbt, shell);
        moved = source.transfer_to(&mut sink, rate);
        if moved > 0 {
            set_i32(&mut entity.nbt, "HeatHU", sink.stored_hu().min(i32::MAX as u64) as i32);
            set_i32(&mut entity.nbt, "TemperatureC", sink.temperature_c() as i32);
        }
    });
    if moved > 0 {
        host.block_entity_sync(pos);
    }
    moved
}

fn get_i32(nbt: &Compound, key: &str) -> i32 {
    match nbt.get(&key.into()) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn set_i32(nbt: &mut Compound, key: &str, value: i32) {
    nbt.insert(key.into(), Tag::Int(value));
}

fn get_f64(nbt: &Compound, key: &str) -> f64 {
    get_f64_or(nbt, key, 0.0)
}

fn get_f64_or(nbt: &Compound, key: &str, default: f64) -> f64 {
    match nbt.get(&key.into()) {
        Some(Tag::Double(bits)) => {
            let value = f64::from_bits(*bits);
            if value.is_finite() { value } else { default }
        }
        _ => default,
    }
}

fn set_f64(nbt: &mut Compound, key: &str, value: f64) {
    nbt.insert(key.into(), Tag::Double(if value.is_finite() {
        value.to_bits()
    } else {
        0.0_f64.to_bits()
    }));
}
