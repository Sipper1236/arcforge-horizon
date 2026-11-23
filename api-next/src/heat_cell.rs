//! Current-API heat cells and the first heat-network consumer path.

use mod_api::{BlockEntityTickEvent, BlockEntityTickReply, Compound, ServerHost, Tag};

const WROUGHT: &str = "arcforge:wrought_heat_cell";
const TEMPERED: &str = "arcforge:tempered_heat_cell";
const HARDENED: &str = "arcforge:hardened_heat_cell";
const ARCFORGED: &str = "arcforge:arcforged_heat_cell";
const REDSTONE_MODE_KEY: &str = "redstone_mode";

const DIRECTIONS: [[i32; 3]; 6] = [
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, -1],
    [0, 0, 1],
    [-1, 0, 0],
    [1, 0, 0],
];

#[derive(Clone, Copy)]
struct Spec {
    tier_name: &'static str,
    capacity: u64,
    rate: u64,
    max_temperature: u32,
    leak_percent_per_minute: f64,
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

#[derive(Clone, Copy)]
pub(crate) struct HeatStore {
    capacity: u64,
    stored: u64,
    max_temperature: u32,
    source_temperature: u32,
}

impl HeatStore {
    pub(crate) fn new(capacity: u64, max_temperature: u32, stored: u64) -> Self {
        let capacity = capacity.max(1);
        Self {
            capacity,
            stored: stored.min(capacity),
            max_temperature: max_temperature.max(20),
            source_temperature: 20,
        }
    }

    pub(crate) fn stored_hu(self) -> u64 {
        self.stored
    }

    pub(crate) fn set_source_temperature(&mut self, temperature_c: u32) {
        self.source_temperature = temperature_c.max(20);
    }

    pub(crate) fn add_hu(&mut self, amount: u64) -> u64 {
        let accepted = amount.min(self.capacity - self.stored);
        self.stored += accepted;
        accepted
    }

    pub(crate) fn temperature_c(self) -> u32 {
        self.source_temperature.max(
            20 + ((self.stored as u128 * u128::from(self.max_temperature - 20))
                / u128::from(self.capacity)) as u32,
        )
    }

    pub(crate) fn remove_hu(&mut self, amount: u64) -> u64 {
        let removed = amount.min(self.stored);
        self.stored -= removed;
        removed
    }

    pub(crate) fn transfer_to(&mut self, other: &mut Self, rate: u64) -> u64 {
        if rate == 0 || self.temperature_c() <= other.temperature_c() {
            return 0;
        }
        let limit = rate.min(self.stored).min(other.capacity - other.stored);
        let mut low = 0_u64;
        let mut high = limit;
        while low < high {
            let mid = low + (high - low + 1) / 2;
            let source_after = Self {
                stored: self.stored - mid,
                ..*self
            };
            let sink_after = Self {
                stored: other.stored + mid,
                ..*other
            };
            if source_after.temperature_c() >= sink_after.temperature_c() {
                low = mid;
            } else {
                high = mid - 1;
            }
        }
        self.stored -= low;
        other.stored += low;
        low
    }
}

pub(crate) fn ids() -> [&'static str; 4] {
    [WROUGHT, TEMPERED, HARDENED, ARCFORGED]
}

pub(crate) fn accept_heat(
    host: &mut dyn ServerHost,
    level: mod_api::LevelRef,
    pos: [i32; 3],
    source: &mut HeatStore,
    rate: u64,
) -> u64 {
    let block_id = level
        .block(host, pos)
        .map(|block| block.id(host).to_owned());
    let Some(block_id) = block_id else {
        return 0;
    };
    let Some(spec) = spec(&block_id) else {
        return 0;
    };
    let Some(entity) = host.block_entity(level, pos) else {
        return 0;
    };
    let Some(nbt) = entity.data(&*host) else {
        return 0;
    };

    let mut sink = HeatStore::new(
        spec.capacity,
        spec.max_temperature,
        get_i32(&nbt, "HeatHU").max(0) as u64,
    );
    let moved = source.transfer_to(&mut sink, rate.min(spec.rate));
    if moved > 0 {
        let mut updates = Compound::new();
        set_i32(
            &mut updates,
            "HeatHU",
            sink.stored_hu().min(i32::MAX as u64) as i32,
        );
        set_i32(
            &mut updates,
            "TemperatureC",
            sink.temperature_c().min(i32::MAX as u32) as i32,
        );
        let _ = entity.merge_data(host, updates);
        let heat_state = match sink.temperature_c() {
            1_000.. => "4",
            850.. => "3",
            650.. => "2",
            300.. => "1",
            _ => "0",
        };
        host.set_block_property(pos, "heat", heat_state);
    }
    moved
}

fn spec(id: &str) -> Option<Spec> {
    let spec = match id {
        WROUGHT => Spec {
            tier_name: "Wrought",
            capacity: 50_000,
            rate: 50,
            max_temperature: 1_100,
            leak_percent_per_minute: 10.0,
        },
        TEMPERED => Spec {
            tier_name: "Tempered",
            capacity: 200_000,
            rate: 200,
            max_temperature: 1_100,
            leak_percent_per_minute: 4.0,
        },
        HARDENED => Spec {
            tier_name: "Hardened",
            capacity: 800_000,
            rate: 800,
            max_temperature: 1_300,
            leak_percent_per_minute: 1.5,
        },
        ARCFORGED => Spec {
            tier_name: "Arcforged",
            capacity: 3_200_000,
            rate: 3_200,
            max_temperature: 1_400,
            leak_percent_per_minute: 0.5,
        },
        _ => return None,
    };
    Some(spec)
}

pub(crate) fn tick(host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>, id: &str) {
    *event.reply = BlockEntityTickReply::Unchanged;
    let Some(spec) = spec(id) else {
        return;
    };
    let Some(level) = host.level(host.dimension()) else {
        return;
    };

    let mut changed = false;
    let mut heat = HeatStore::new(
        spec.capacity,
        spec.max_temperature,
        get_i32(&event.entity.nbt, "HeatHU").max(0) as u64,
    );
    let tick = level.game_time(host).unwrap_or_default();
    let redstone_mode = RedstoneMode::from_value(get_i32(&event.entity.nbt, REDSTONE_MODE_KEY));
    let enabled = redstone_mode.can_run(level.has_neighbor_signal(host, event.pos));
    changed |= set_i32(
        &mut event.entity.nbt,
        REDSTONE_MODE_KEY,
        redstone_mode as i32,
    );
    if tick.rem_euclid(20) == 0 && heat.stored_hu() > 0 {
        let per_second = heat.stored_hu() as f64 * spec.leak_percent_per_minute / 6_000.0;
        let total = per_second + get_double(&event.entity.nbt, "HeatCellLeakRemainder");
        let lost = total.floor().min(i32::MAX as f64) as u64;
        let remainder = if heat.stored_hu() <= lost {
            0.0
        } else {
            total - lost as f64
        };
        changed |= set_double(&mut event.entity.nbt, "HeatCellLeakRemainder", remainder);
        heat.remove_hu(lost);
    }

    if enabled {
        let start = tick.rem_euclid(DIRECTIONS.len() as i64) as usize;
        let mut output_used = 0_u64;
        for index in 0..DIRECTIONS.len() {
            let budget = spec.rate.saturating_sub(output_used);
            if budget == 0 || heat.stored_hu() == 0 || heat.temperature_c() <= 20 {
                break;
            }
            let direction_index = (start + index) % DIRECTIONS.len();
            let offset = DIRECTIONS[direction_index];
            let neighbor = [
                event.pos[0] + offset[0],
                event.pos[1] + offset[1],
                event.pos[2] + offset[2],
            ];
            let before = heat.stored_hu();
            let moved = super::cracking_unit::accept_heat(host, level, neighbor, &mut heat, budget)
                + super::thermal::accept_heat(host, level, neighbor, event.pos, &mut heat, budget)
                + super::thermoelectric::accept_heat(
                    host,
                    level,
                    neighbor,
                    super::thermoelectric::opposite_face_index(direction_index),
                    &mut heat,
                    budget,
                );
            output_used = output_used.saturating_add(moved);
            changed |= before != heat.stored_hu();
        }
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
    let heat_state = match heat.temperature_c() {
        1_000.. => "4",
        850.. => "3",
        650.. => "2",
        300.. => "1",
        _ => "0",
    };
    changed |= host.set_block_property(event.pos, "heat", heat_state);

    let data = [
        (heat.stored_hu() / 10).min(i32::MAX as u64) as i32,
        (spec.capacity / 10).min(i32::MAX as u64) as i32,
        heat.temperature_c().min(i32::MAX as u32) as i32,
        spec.rate.min(i32::MAX as u64) as i32,
        (spec.leak_percent_per_minute * 100.0).round() as i32,
        redstone_mode as i32,
        if enabled { 1 } else { 0 },
        match spec.tier_name {
            "Wrought" => 0,
            "Tempered" => 1,
            "Hardened" => 2,
            _ => 3,
        },
    ];
    if event.entity.data.as_slice() != data.as_slice() {
        event.entity.data = data.into();
        changed = true;
    }

    if changed {
        *event.reply = BlockEntityTickReply::Changed;
    }
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

fn get_double(nbt: &Compound, name: &str) -> f64 {
    match nbt.get(&name.into()) {
        Some(Tag::Double(value)) => f64::from_bits(*value),
        _ => 0.0,
    }
}

fn set_double(nbt: &mut Compound, name: &str, value: f64) -> bool {
    if get_double(nbt, name) == value {
        return false;
    }
    nbt.insert(name.into(), Tag::Double(value.to_bits()));
    true
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
    let rate = data.get(3).copied().unwrap_or_default().max(0);
    let leak_hundredths = data.get(4).copied().unwrap_or_default().max(0);
    let redstone_mode = RedstoneMode::from_value(data.get(5).copied().unwrap_or_default());
    let enabled = data.get(6).copied().unwrap_or(1) != 0;
    let tier = match data.get(7).copied().unwrap_or_default() {
        0 => "Wrought",
        1 => "Tempered",
        2 => "Hardened",
        _ => "Arcforged",
    };
    let draw = &mut *event.draw;
    draw.text(
        &format!("{tier} Heat Cell"),
        [ox + 8.0, oy + 6.0],
        0xFFFFFFFF,
        true,
    );
    draw.text(
        &format!("Temperature: {temperature} C"),
        [ox + 8.0, oy + 19.0],
        0xFFBBBBBB,
        true,
    );
    draw.fill([ox + 8.0, oy + 33.0, 160.0, 8.0], 0xFF343434);
    if capacity > 0 {
        let width = (160.0 * heat.min(capacity) as f32 / capacity as f32).round();
        if width > 0.0 {
            draw.fill([ox + 8.0, oy + 33.0, width, 8.0], 0xFFE37B32);
        }
    }
    draw.text(
        &format!("Heat: {heat} / {capacity} HU"),
        [ox + 8.0, oy + 45.0],
        0xFFFFFFFF,
        true,
    );
    let output_status = if enabled {
        format!("Output: up to {rate} HU/t")
    } else {
        "Heat output paused by redstone".to_owned()
    };
    draw.text(&output_status, [ox + 8.0, oy + 59.0], 0xFFFFFFFF, true);
    draw.text(
        &format!("Leak: {:.2}% per minute", leak_hundredths as f32 / 100.0),
        [ox + 8.0, oy + 72.0],
        0xFFBBBBBB,
        true,
    );
    draw.text("Redstone:", [ox + 8.0, oy + 94.0], 0xFFBBBBBB, true);
    for (mode, x, width) in [
        (RedstoneMode::Ignore, 62.0, 35.0),
        (RedstoneMode::High, 99.0, 28.0),
        (RedstoneMode::Low, 129.0, 27.0),
    ] {
        draw.fill(
            [ox + x, oy + 91.0, width, 12.0],
            if mode == redstone_mode {
                0xFF526A7A
            } else {
                0xFF343434
            },
        );
        draw.text(mode.label(), [ox + x + 3.0, oy + 93.0], 0xFFFFFFFF, true);
    }
}

pub(crate) fn screen_click(at: [f32; 2], right: bool) -> Option<i32> {
    if right || !(91.0..103.0).contains(&at[1]) {
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
        lines.push("Redstone pauses output; heat leakage continues".to_owned());
    } else if (33.0..41.0).contains(&at[1]) && (8.0..168.0).contains(&at[0]) {
        lines.push("Stored heat HU / capacity".to_owned());
    } else if (70.0..82.0).contains(&at[1]) && (8.0..168.0).contains(&at[0]) {
        lines.push("Heat lost each minute; leakage continues while paused".to_owned());
    }
}
