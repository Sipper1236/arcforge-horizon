//! Arcforge Air Separator: uses FE to separate ambient air into Nitrogen and Oxygen.
use horizon_client::{
    containers::{ModBlockEntity, ModContainer},
    mod_api::{Compound, Host, ModEnergyBuffer, ModFluidTanks, Tag, serde_json::Value},
};

pub(crate) const BLOCK: &str = "arcforge:air_separator";

const NITROGEN_TANK: usize = 0;
const OXYGEN_TANK: usize = 1;
const GAS_CAPACITY: u32 = 8_000;
const ENERGY_CAPACITY: u64 = 40_000;
const MAX_ENERGY_INPUT: u64 = 400;
const DEFAULT_ENERGY_PER_TICK: u64 = 80;
const SPEED_SLOT: usize = 0;
const ENERGY_SLOT: usize = 1;
const SPEED_UPGRADE: &str = "arcforge:speed_upgrade";
const ENERGY_UPGRADE: &str = "arcforge:energy_upgrade";

struct Product {
    fluid: String,
    amount: u32,
}

struct Recipe {
    nitrogen: Product,
    oxygen: Product,
    ticks: u32,
    energy_per_tick: u32,
}

fn recipe_from_json(json: &Value) -> Option<Recipe> {
    let amount = |value: &Value| {
        value
            .as_u64()
            .map(|amount| amount.clamp(1, u64::from(u32::MAX)) as u32)
    };
    let product = |value: &Value| {
        Some(Product {
            fluid: value.get("id")?.as_str()?.to_owned(),
            amount: amount(value.get("amount")?)?,
        })
    };
    let primary = product(json.get("primary")?)?;
    let secondary = product(json.get("secondary")?)?;
    Some(Recipe {
        nitrogen: primary,
        oxygen: secondary,
        ticks: json
            .get("time")
            .and_then(Value::as_u64)
            .unwrap_or(40)
            .clamp(1, u64::from(u32::MAX)) as u32,
        energy_per_tick: json
            .get("energy_per_tick")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_ENERGY_PER_TICK)
            .min(u64::from(u32::MAX)) as u32,
    })
}

fn find_recipe(host: &dyn Host) -> Option<Recipe> {
    host.recipes_of_type("arcforge:air_separating")
        .iter()
        .filter(|recipe| {
            !recipe
                .json
                .get("excluded_dimensions")
                .and_then(Value::as_array)
                .is_some_and(|dimensions| {
                    dimensions
                        .iter()
                        .any(|dimension| dimension.as_str() == Some(host.dimension()))
                })
        })
        .find_map(|recipe| recipe_from_json(&recipe.json))
}

fn get_i32(nbt: &Compound, name: &str) -> i32 {
    let Tag::String(key) = Tag::String(name.into()) else {
        unreachable!()
    };
    match nbt.get(&key) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn set_i32(nbt: &mut Compound, name: &str, value: i32) {
    let Tag::String(key) = Tag::String(name.into()) else {
        unreachable!()
    };
    nbt.insert(key, Tag::Int(value));
}

fn set_lit(host: &mut dyn Host, pos: [i32; 3], lit: bool) {
    let value = if lit { "true" } else { "false" };
    if host.block_property(pos, "lit").as_deref() != Some(value) {
        host.set_block_property(pos, "lit", value);
    }
}

fn upgrades(entity: &ModBlockEntity, slot: usize, item: &str) -> u32 {
    entity
        .items
        .get(slot)
        .and_then(Option::as_ref)
        .filter(|stack| stack.item.as_ref() == item)
        .map_or(0, |stack| stack.count.min(8))
}

fn progress(entity: &ModBlockEntity) -> u32 {
    get_i32(&entity.nbt, "AirSeparatorProgress").max(0) as u32
}

fn duration(recipe: &Recipe, entity: &ModBlockEntity) -> u32 {
    crate::speed_upgrade_time(recipe.ticks, upgrades(entity, SPEED_SLOT, SPEED_UPGRADE)).max(1)
}

fn products_fit(entity: &ModBlockEntity, recipe: &Recipe) -> bool {
    crate::fluid::can_insert(
        &entity.nbt,
        NITROGEN_TANK,
        &recipe.nitrogen.fluid,
        recipe.nitrogen.amount,
        GAS_CAPACITY,
    ) && crate::fluid::can_insert(
        &entity.nbt,
        OXYGEN_TANK,
        &recipe.oxygen.fluid,
        recipe.oxygen.amount,
        GAS_CAPACITY,
    )
}

fn sync_screen(entity: &mut ModBlockEntity, energy_rate: u32, total: u32, status: i32) {
    entity.data.resize(6, 0);
    entity.data[0] = get_i32(&entity.nbt, "energy").max(0);
    entity.data[1] = progress(entity).min(i32::MAX as u32) as i32;
    entity.data[2] = total.min(i32::MAX as u32) as i32;
    entity.data[3] = crate::fluid::amount(&entity.nbt, NITROGEN_TANK).min(i32::MAX as u32) as i32;
    entity.data[4] = crate::fluid::amount(&entity.nbt, OXYGEN_TANK).min(i32::MAX as u32) as i32;
    entity.data[5] = status;
    set_i32(
        &mut entity.nbt,
        "AirSeparatorEnergyRate",
        energy_rate.min(i32::MAX as u32) as i32,
    );
}

fn set_status(entity: &mut ModBlockEntity, energy_rate: u32, total: u32, status: i32) {
    set_i32(&mut entity.nbt, "AirSeparatorStatus", status);
    sync_screen(entity, energy_rate, total, status);
}

pub(crate) fn tick(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    entity.items.resize(2, None);
    let stored_energy = (get_i32(&entity.nbt, "energy").max(0) as u64).min(ENERGY_CAPACITY);
    set_i32(&mut entity.nbt, "energy", stored_energy as i32);

    let Some(recipe) = find_recipe(host) else {
        set_i32(&mut entity.nbt, "AirSeparatorProgress", 0);
        set_status(
            entity,
            0,
            0,
            if host.dimension() == "minecraft:the_end" {
                1
            } else {
                2
            },
        );
        set_lit(host, pos, false);
        return;
    };

    let total = duration(&recipe, entity);
    set_i32(
        &mut entity.nbt,
        "AirSeparatorTotal",
        total.min(i32::MAX as u32) as i32,
    );
    if !products_fit(entity, &recipe) {
        set_status(entity, 0, total, 3);
        set_lit(host, pos, false);
        return;
    }

    let speed = crate::speed_upgrade_multiplier(upgrades(entity, SPEED_SLOT, SPEED_UPGRADE));
    let energy_cost = crate::scaled_upgrade_cost(
        u64::from(recipe.energy_per_tick),
        speed,
        upgrades(entity, ENERGY_SLOT, ENERGY_UPGRADE),
    );
    let mut energy = crate::energy::EnergyStore::new(ENERGY_CAPACITY, stored_energy);
    if energy.stored_fe() < energy_cost {
        set_status(entity, 0, total, 4);
        set_lit(host, pos, false);
        return;
    }

    energy.extract(energy_cost, energy_cost);
    set_i32(&mut entity.nbt, "energy", energy.stored_fe() as i32);
    let next = progress(entity).saturating_add(1);
    set_i32(
        &mut entity.nbt,
        "AirSeparatorProgress",
        next.min(i32::MAX as u32) as i32,
    );
    set_status(
        entity,
        energy_cost.min(u64::from(u32::MAX)) as u32,
        total,
        0,
    );
    set_lit(host, pos, true);

    if next >= total {
        crate::fluid::insert(
            &mut entity.nbt,
            NITROGEN_TANK,
            &recipe.nitrogen.fluid,
            recipe.nitrogen.amount,
            GAS_CAPACITY,
        );
        crate::fluid::insert(
            &mut entity.nbt,
            OXYGEN_TANK,
            &recipe.oxygen.fluid,
            recipe.oxygen.amount,
            GAS_CAPACITY,
        );
        set_i32(&mut entity.nbt, "AirSeparatorProgress", 0);
        set_status(entity, 0, total, 0);
        set_lit(host, pos, false);
    }
}

pub(crate) fn accepts(_tank: usize, _fluid: &str) -> bool {
    false
}

pub(crate) fn fluid_tanks() -> ModFluidTanks {
    ModFluidTanks {
        tanks: vec![GAS_CAPACITY as i32, GAS_CAPACITY as i32],
        // Fixed to the source default facing (north): Nitrogen leaves west and Oxygen east.
        faces: [
            vec![],
            vec![],
            vec![],
            vec![],
            vec![NITROGEN_TANK],
            vec![OXYGEN_TANK],
        ],
        inputs: vec![],
        outputs: vec![NITROGEN_TANK, OXYGEN_TANK],
    }
}

pub(crate) fn energy_buffer() -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: ENERGY_CAPACITY as i32,
        max_insert: MAX_ENERGY_INPUT as i32,
        max_extract: 0,
        // Fixed to the source default facing (north): FE enters through the back (south).
        insert_faces: [false, false, false, true, false, false],
        extract_faces: [false; 6],
    }
}

pub(crate) fn container() -> ModContainer {
    ModContainer {
        slots: 2,
        faces: [
            vec![SPEED_SLOT, ENERGY_SLOT],
            vec![SPEED_SLOT, ENERGY_SLOT],
            vec![SPEED_SLOT, ENERGY_SLOT],
            vec![SPEED_SLOT, ENERGY_SLOT],
            vec![SPEED_SLOT, ENERGY_SLOT],
            vec![SPEED_SLOT, ENERGY_SLOT],
        ],
        inputs: vec![SPEED_SLOT, ENERGY_SLOT],
        outputs: vec![],
        menu: Some("arcforge:machine_2_slot".into()),
    }
}

pub(crate) fn item_allowed(slot: usize, item: &str, count: u32) -> bool {
    count <= 8
        && ((slot == SPEED_SLOT && item == SPEED_UPGRADE)
            || (slot == ENERGY_SLOT && item == ENERGY_UPGRADE))
}

pub(crate) fn draw_screen(
    host: &dyn Host,
    event: &mut horizon_client::mod_api::ScreenDrawEvent<'_>,
) {
    let [ox, oy] = event.origin;
    let data = event.data;
    let batch = &mut *event.batch;
    macro_rules! text {
        ($label:expr, $x:expr, $y:expr, $color:expr $(,)?) => {
            host.text(batch, $label, [ox + $x, oy + $y], $color, true)
        };
    }
    macro_rules! fill {
        ($rect:expr, $color:expr) => {
            host.fill(
                batch,
                [ox + ($rect)[0], oy + ($rect)[1], ($rect)[2], ($rect)[3]],
                $color,
            )
        };
    }

    text!("Air Separator", 8.0, 6.0, 0xFFFFFFFF);
    text!("Speed", 38.0, 17.0, 0xFFBBBBBB);
    text!("Energy", 122.0, 17.0, 0xFFBBBBBB);
    text!("Nitrogen", 8.0, 44.0, 0xFFBBBBBB);
    text!("Oxygen", 90.0, 44.0, 0xFFBBBBBB);

    let energy = data.first().copied().unwrap_or_default().max(0);
    let progress = data.get(1).copied().unwrap_or_default().max(0);
    let duration = data.get(2).copied().unwrap_or_default().max(0);
    let nitrogen = data.get(3).copied().unwrap_or_default().max(0);
    let oxygen = data.get(4).copied().unwrap_or_default().max(0);
    let state = match data.get(5).copied().unwrap_or_default() {
        1 => "No air in this dimension",
        2 => "No air separation recipe",
        3 => "Gas tank full",
        4 => "Waiting for FE",
        _ if duration > 0 => "Separating air",
        _ => "Idle",
    };
    text!(
        &format!("{nitrogen}/{GAS_CAPACITY} mB"),
        8.0,
        58.0,
        0xFFE0E0E0
    );
    text!(
        &format!("{oxygen}/{GAS_CAPACITY} mB"),
        90.0,
        58.0,
        0xFFE0E0E0
    );
    text!(
        &format!("FE {energy}/{ENERGY_CAPACITY}"),
        8.0,
        73.0,
        0xFFE0E0E0
    );
    text!(state, 8.0, 88.0, 0xFFBBBBBB);
    if duration > 0 {
        text!(
            &format!("Progress {progress}/{duration} t"),
            8.0,
            100.0,
            0xFFE0E0E0,
        );
        fill!([8.0, 111.0, 160.0, 6.0], 0xFF383838);
        let fraction = (progress as f32 / duration as f32).clamp(0.0, 1.0);
        fill!([8.0, 111.0, 160.0 * fraction, 6.0], 0xFF62B64F);
    }
    text!("Inventory", 8.0, 130.0, 0xFFFFFFFF);
}
