//! Current-API heat cells and the first heat-network consumer path.

use mod_api::{BlockEntityTickEvent, BlockEntityTickReply, Compound, ServerHost, Tag};

const WROUGHT: &str = "arcforge:wrought_heat_cell";
const TEMPERED: &str = "arcforge:tempered_heat_cell";
const HARDENED: &str = "arcforge:hardened_heat_cell";
const ARCFORGED: &str = "arcforge:arcforged_heat_cell";

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
    capacity: u64,
    rate: u64,
    max_temperature: u32,
    leak_percent_per_minute: f64,
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
            capacity: 50_000,
            rate: 50,
            max_temperature: 1_100,
            leak_percent_per_minute: 10.0,
        },
        TEMPERED => Spec {
            capacity: 200_000,
            rate: 200,
            max_temperature: 1_100,
            leak_percent_per_minute: 4.0,
        },
        HARDENED => Spec {
            capacity: 800_000,
            rate: 800,
            max_temperature: 1_300,
            leak_percent_per_minute: 1.5,
        },
        ARCFORGED => Spec {
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
