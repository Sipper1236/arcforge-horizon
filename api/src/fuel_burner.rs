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
