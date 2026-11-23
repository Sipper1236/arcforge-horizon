//! Current-API heat cells and the first heat-network consumer path.

use mod_api::{
    BlockEntityTickEvent, BlockEntityTickReply, Component, Compound, Host, ItemStack, ServerHost,
    Tag,
};

const WROUGHT: &str = "arcforge:wrought_heat_cell";
const TEMPERED: &str = "arcforge:tempered_heat_cell";
const HARDENED: &str = "arcforge:hardened_heat_cell";
const ARCFORGED: &str = "arcforge:arcforged_heat_cell";
const WROUGHT_CAPSULE: &str = "arcforge:wrought_thermal_capsule";
const TEMPERED_CAPSULE: &str = "arcforge:tempered_thermal_capsule";
const HARDENED_CAPSULE: &str = "arcforge:hardened_thermal_capsule";
const ARCFORGED_CAPSULE: &str = "arcforge:arcforged_thermal_capsule";
pub(crate) const INSULATION_UPGRADE: &str = "arcforge:insulation_upgrade";
const REDSTONE_MODE_KEY: &str = "redstone_mode";
const INPUT_SLOT: usize = 0;
const OUTPUT_SLOT: usize = 1;
const INSULATION_SLOT: usize = 2;
const MAX_INSULATION: u32 = 8;
const INPUT_TICK_KEY: &str = "HeatCellInputTick";
const INPUT_USED_KEY: &str = "HeatCellInputUsed";
const CAPSULE_LEAK_REMAINDER_KEY: &str = "horizon_heat_leak_remainder";

const DIRECTIONS: [[i32; 3]; 6] = [
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, -1],
    [0, 0, 1],
    [-1, 0, 0],
    [1, 0, 0],
];

struct HeatItemData;

impl Component for HeatItemData {
    type Value = Compound;

    fn id(&self) -> &str {
        "minecraft:custom_data"
    }

    fn decode(&self, value: &Tag) -> Option<Self::Value> {
        match value {
            Tag::Compound(data) => Some(data.clone()),
            _ => None,
        }
    }

    fn encode(&self, value: Self::Value) -> Tag {
        Tag::Compound(value)
    }
}

const HEAT_ITEM_DATA: HeatItemData = HeatItemData;

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

pub(crate) fn capsule_ids() -> [&'static str; 4] {
    [
        WROUGHT_CAPSULE,
        TEMPERED_CAPSULE,
        HARDENED_CAPSULE,
        ARCFORGED_CAPSULE,
    ]
}

pub(crate) fn is_capsule(id: &str) -> bool {
    capsule_ids().contains(&id)
}

fn capsule_spec(id: &str) -> Option<Spec> {
    match id {
        WROUGHT_CAPSULE => spec(WROUGHT),
        TEMPERED_CAPSULE => spec(TEMPERED),
        HARDENED_CAPSULE => spec(HARDENED),
        ARCFORGED_CAPSULE => spec(ARCFORGED),
        _ => None,
    }
}

fn item_heat(stack: &ItemStack, spec: Spec) -> u64 {
    let Some(data) = stack.get(&HEAT_ITEM_DATA) else {
        return 0;
    };
    match data.get(&"horizon_heat".into()) {
        Some(Tag::Int(amount)) => (*amount).max(0) as u64,
        _ => 0,
    }
    .min(spec.capacity)
}

fn set_item_heat(stack: &mut ItemStack, amount: u64) -> bool {
    let mut data = stack.get(&HEAT_ITEM_DATA).unwrap_or_default();
    let key = "horizon_heat".into();
    if amount == 0 {
        data.remove(&key);
        if data.is_empty() {
            stack.remove(&HEAT_ITEM_DATA);
            return true;
        }
    } else {
        data.insert(key, Tag::Int(amount.min(i32::MAX as u64) as i32));
    }
    stack.set(&HEAT_ITEM_DATA, data).is_ok()
}

fn item_leak_remainder(stack: &ItemStack) -> f64 {
    stack
        .get(&HEAT_ITEM_DATA)
        .and_then(|data| data.get(&CAPSULE_LEAK_REMAINDER_KEY.into()).cloned())
        .and_then(|tag| match tag {
            Tag::Double(bits) => Some(f64::from_bits(bits)),
            _ => None,
        })
        .filter(|remainder| remainder.is_finite() && (0.0..1.0).contains(remainder))
        .unwrap_or_default()
}

fn set_item_heat_and_remainder(stack: &mut ItemStack, amount: u64, remainder: f64) -> bool {
    let mut data = stack.get(&HEAT_ITEM_DATA).unwrap_or_default();
    let heat_key = "horizon_heat".into();
    let remainder_key = CAPSULE_LEAK_REMAINDER_KEY.into();
    if amount == 0 {
        data.remove(&heat_key);
    } else {
        data.insert(heat_key, Tag::Int(amount.min(i32::MAX as u64) as i32));
    }
    if amount > 0 && remainder.is_finite() && remainder > 0.0 {
        data.insert(remainder_key, Tag::Double(remainder.to_bits()));
    } else {
        data.remove(&remainder_key);
    }
    if data.is_empty() {
        stack.remove(&HEAT_ITEM_DATA);
        true
    } else {
        stack.set(&HEAT_ITEM_DATA, data).is_ok()
    }
}

pub(crate) fn inventory_tick_capsule(
    host: &mut dyn ServerHost,
    event: &mut mod_api::InventoryTickEvent<'_>,
) {
    let item_id = event.stack.item().id(host).to_owned();
    let Some(spec) = capsule_spec(&item_id) else {
        return;
    };
    let Some(level) = host.level(host.dimension()) else {
        return;
    };
    if level.game_time(host).unwrap_or_default().rem_euclid(20) != 0 {
        return;
    }
    let held = item_heat(&event.stack, spec);
    if held == 0 {
        return;
    }

    // Carried capsules use the matching Heat Cell tier's per-second loss. Keep the
    // fractional HU in custom data so small capsules still lose heat over time.
    let per_second = held as f64 * spec.leak_percent_per_minute / 6_000.0;
    let total = per_second + item_leak_remainder(&event.stack);
    let lost = total.floor().min(held as f64) as u64;
    let remainder = if held <= lost {
        0.0
    } else {
        total - lost as f64
    };
    let place = mod_api::ItemPlace::PlayerSlot {
        player: event.player,
        slot: event.slot,
    };
    let _ = host.transaction(|host, tx| {
        if place.edit(host, tx, |stack| {
            let _ = set_item_heat_and_remainder(stack, held - lost, remainder);
        }) {
            Ok(())
        } else {
            Err(())
        }
    });
}

fn insulation_count(host: &dyn Host, items: &[Option<ItemStack>]) -> u32 {
    items
        .get(INSULATION_SLOT)
        .and_then(Option::as_ref)
        .filter(|stack| stack.item().id(host) == INSULATION_UPGRADE)
        .map_or(0, |stack| stack.count().min(MAX_INSULATION))
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
    let tick = level.game_time(host).unwrap_or_default();
    let used = if get_i64(&nbt, INPUT_TICK_KEY) == tick {
        get_i32(&nbt, INPUT_USED_KEY).max(0) as u64
    } else {
        0
    };
    let moved = source.transfer_to(&mut sink, rate.min(spec.rate.saturating_sub(used)));
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
        set_i64(&mut updates, INPUT_TICK_KEY, tick);
        set_i32(
            &mut updates,
            INPUT_USED_KEY,
            used.saturating_add(moved).min(i32::MAX as u64) as i32,
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
    let insulation = insulation_count(host, &event.entity.items);
    let leak_percent_per_minute = spec.leak_percent_per_minute * 0.8_f64.powi(insulation as i32);
    changed |= set_i32(
        &mut event.entity.nbt,
        REDSTONE_MODE_KEY,
        redstone_mode as i32,
    );
    if tick.rem_euclid(20) == 0 && heat.stored_hu() > 0 {
        let per_second = heat.stored_hu() as f64 * leak_percent_per_minute / 6_000.0;
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

    let mut input_used = if get_i64(&event.entity.nbt, INPUT_TICK_KEY) == tick {
        get_i32(&event.entity.nbt, INPUT_USED_KEY).max(0) as u64
    } else {
        0
    };
    let input_stack = event
        .entity
        .items
        .get(INPUT_SLOT)
        .and_then(Option::as_ref)
        .cloned();
    if let Some(stack) = input_stack {
        let item_id = stack.item().id(host).to_owned();
        if let Some(capsule_spec) = capsule_spec(&item_id) {
            let held = item_heat(&stack, capsule_spec);
            let mut capsule =
                HeatStore::new(capsule_spec.capacity, capsule_spec.max_temperature, held);
            let mut cell_after = heat;
            let budget = capsule_spec
                .rate
                .min(spec.rate.saturating_sub(input_used))
                .min(held);
            let moved = capsule.transfer_to(&mut cell_after, budget);
            if moved > 0 {
                let mut updated = stack;
                if set_item_heat(&mut updated, capsule.stored_hu()) {
                    event.entity.items[INPUT_SLOT] = Some(updated);
                    heat = cell_after;
                    input_used = input_used.saturating_add(moved);
                    changed = true;
                    changed |= set_i64(&mut event.entity.nbt, INPUT_TICK_KEY, tick);
                    changed |= set_i32(
                        &mut event.entity.nbt,
                        INPUT_USED_KEY,
                        input_used.min(i32::MAX as u64) as i32,
                    );
                }
            }
        }
    }

    let mut output_used = 0_u64;
    if enabled {
        let output_stack = event
            .entity
            .items
            .get(OUTPUT_SLOT)
            .and_then(Option::as_ref)
            .cloned();
        if let Some(stack) = output_stack {
            let item_id = stack.item().id(host).to_owned();
            if let Some(capsule_spec) = capsule_spec(&item_id) {
                let held = item_heat(&stack, capsule_spec);
                let mut capsule =
                    HeatStore::new(capsule_spec.capacity, capsule_spec.max_temperature, held);
                let mut cell_after = heat;
                let room = capsule_spec.capacity.saturating_sub(held);
                let budget = capsule_spec
                    .rate
                    .min(spec.rate.saturating_sub(output_used))
                    .min(room);
                let moved = cell_after.transfer_to(&mut capsule, budget);
                if moved > 0 {
                    let mut updated = stack;
                    if set_item_heat(&mut updated, capsule.stored_hu()) {
                        event.entity.items[OUTPUT_SLOT] = Some(updated);
                        heat = cell_after;
                        output_used = output_used.saturating_add(moved);
                        changed = true;
                    }
                }
            }
        }

        let start = tick.rem_euclid(DIRECTIONS.len() as i64) as usize;
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
            let mut moved =
                super::cracking_unit::accept_heat(host, level, neighbor, &mut heat, budget);
            let remaining = budget.saturating_sub(moved);
            moved +=
                super::thermal::accept_heat(host, level, neighbor, event.pos, &mut heat, remaining);
            let remaining = budget.saturating_sub(moved);
            moved += super::thermoelectric::accept_heat(
                host,
                level,
                neighbor,
                super::thermoelectric::opposite_face_index(direction_index),
                &mut heat,
                remaining,
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
        (leak_percent_per_minute * 100.0).round() as i32,
        redstone_mode as i32,
        if enabled { 1 } else { 0 },
        match spec.tier_name {
            "Wrought" => 0,
            "Tempered" => 1,
            "Hardened" => 2,
            _ => 3,
        },
        insulation as i32,
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

fn get_i64(nbt: &Compound, name: &str) -> i64 {
    match nbt.get(&name.into()) {
        Some(Tag::Long(value)) => *value,
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

fn set_i64(nbt: &mut Compound, name: &str, value: i64) -> bool {
    if get_i64(nbt, name) == value {
        return false;
    }
    nbt.insert(name.into(), Tag::Long(value));
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
        height: 254.0,
        slots: vec![
            Some([16.0, 110.0]),
            Some([16.0, 140.0]),
            Some([136.0, 126.0]),
        ],
        inventory_y: 168.0,
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
    let insulation = data.get(8).copied().unwrap_or_default().clamp(0, 8);
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
    draw.text(
        &format!("Insulation: {insulation} / 8"),
        [ox + 8.0, oy + 81.0],
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
    draw.text("Capsule in", [ox + 39.0, oy + 113.0], 0xFFBBBBBB, true);
    draw.text("Capsule out", [ox + 39.0, oy + 143.0], 0xFFBBBBBB, true);
    draw.text("Insulation", [ox + 106.0, oy + 132.0], 0xFFBBBBBB, true);
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
