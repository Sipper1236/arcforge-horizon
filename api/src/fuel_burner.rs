//! Source Arcforge Fuel Burner fuel profiles.
//!
//! The pinned source uses a fluid data map for these values. The current
//! Horizon content API does not expose that data-map type, so this table keeps
//! the pinned defaults in the mod. Refinery configuration overrides therefore
//! remain fixed to their source defaults.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fuel {
    pub heat_hu_per_mb: u32,
    pub burn_millimb_per_tick: u32,
    pub burn_temperature_c: u32,
}

const LIQUID_FUEL_BUCKETS: &[(&str, &str)] = &[
    ("arcforge:creosote", "arcforge:creosote_bucket"),
    ("arcforge:naphtha", "arcforge:naphtha_bucket"),
    ("arcforge:light_oil", "arcforge:light_oil_bucket"),
    ("arcforge:heavy_oil", "arcforge:heavy_oil_bucket"),
    ("arcforge:ethanol", "arcforge:ethanol_bucket"),
    ("arcforge:biodiesel", "arcforge:biodiesel_bucket"),
    ("arcforge:kerosene", "arcforge:kerosene_bucket"),
    ("arcforge:diesel", "arcforge:diesel_bucket"),
    ("arcforge:gasoline", "arcforge:gasoline_bucket"),
    ("arcforge:sour_kerosene", "arcforge:sour_kerosene_bucket"),
    ("arcforge:sour_diesel", "arcforge:sour_diesel_bucket"),
];

pub fn liquid_fuel_buckets() -> &'static [(&'static str, &'static str)] {
    LIQUID_FUEL_BUCKETS
}

pub fn fluid_in_bucket(bucket: &str) -> Option<&'static str> {
    LIQUID_FUEL_BUCKETS
        .iter()
        .find_map(|(fluid, item)| (*item == bucket).then_some(*fluid))
}

pub fn bucket_for_fluid(fluid: &str) -> Option<&'static str> {
    LIQUID_FUEL_BUCKETS
        .iter()
        .find_map(|(name, item)| (*name == fluid).then_some(*item))
}

pub fn fuel(fluid: &str) -> Option<Fuel> {
    let profile = match fluid {
        "arcforge:creosote" => (120, 500, 850),
        "arcforge:naphtha" => (400, 500, 1_200),
        "arcforge:light_oil" => (250, 500, 1_000),
        "arcforge:heavy_oil" => (200, 250, 750),
        "arcforge:hydrogen" => (60, 1_000, 1_400),
        "arcforge:ethanol" => (300, 500, 900),
        "arcforge:biodiesel" => (320, 500, 1_000),
        "arcforge:biogas" => (45, 1_000, 1_100),
        "arcforge:syngas" => (50, 1_000, 1_150),
        "arcforge:kerosene" => (380, 500, 1_150),
        "arcforge:diesel" => (420, 500, 1_150),
        "arcforge:refinery_gas" => (300, 500, 1_200),
        "arcforge:gasoline" => (450, 500, 1_250),
        "arcforge:sour_kerosene" => (285, 500, 1_050),
        "arcforge:sour_diesel" => (315, 500, 1_050),
        "arcforge:natural_gas" => (55, 1_000, 1_250),
        _ => return None,
    };
    Some(Fuel {
        heat_hu_per_mb: profile.0,
        burn_millimb_per_tick: profile.1,
        burn_temperature_c: profile.2,
    })
}
