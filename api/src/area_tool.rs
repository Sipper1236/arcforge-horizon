//! Steel Hammer and Excavator 3×3 mining behavior.
//!
//! The pinned Horizon API can break neighboring blocks with normal loot and a
//! supplied tool, but it does not expose a player-context break for each
//! neighbor. This alpha path therefore limits each sweep to the same block ID
//! as the center and applies one point of tool damage per additional block.

pub(crate) const STEEL_HAMMER: &str = "arcforge:steel_hammer";
pub(crate) const STEEL_EXCAVATOR: &str = "arcforge:steel_excavator";
pub(crate) const DURABILITY: u32 = 1_500;

#[derive(Clone, Copy)]
pub(crate) enum Axis {
    X,
    Y,
    Z,
}

pub(crate) fn is_area_tool(item: &str) -> bool {
    item == STEEL_HAMMER || item == STEEL_EXCAVATOR
}

/// Approximate the hit-face axis from the server player's last accepted view.
/// The destroy-block hook does not carry the hit face.
pub(crate) fn mining_axis(yaw: f32, pitch: f32) -> Axis {
    let yaw = yaw.to_radians();
    let pitch = pitch.to_radians();
    let x = -yaw.sin() * pitch.cos();
    let y = -pitch.sin();
    let z = yaw.cos() * pitch.cos();
    if x.abs() >= y.abs() && x.abs() >= z.abs() {
        Axis::X
    } else if y.abs() >= z.abs() {
        Axis::Y
    } else {
        Axis::Z
    }
}

/// Positions in the 3×3 plane normal to the player's view axis, in the same
/// deterministic order used by Arcforge's area tools.
pub(crate) fn targets(center: [i32; 3], axis: Axis) -> [[i32; 3]; 8] {
    let mut positions = [[0; 3]; 8];
    let mut index = 0;
    for a in -1..=1 {
        for b in -1..=1 {
            if a == 0 && b == 0 {
                continue;
            }
            positions[index] = match axis {
                Axis::X => [center[0], center[1] + a, center[2] + b],
                Axis::Y => [center[0] + a, center[1], center[2] + b],
                Axis::Z => [center[0] + a, center[1] + b, center[2]],
            };
            index += 1;
        }
    }
    positions
}

/// Horizon exposes neither the vanilla effective-tool query nor block
/// hardness in this hook. Keep this alpha behavior to common, breakable stone
/// and soil groups; identical IDs guarantee the neighbors match the center's
/// tool tier and hardness.
pub(crate) fn supports_center(item: &str, block: &str) -> bool {
    if item == STEEL_HAMMER {
        if block.starts_with("arcforge:") && block.ends_with("_ore") {
            return true;
        }
        if block.starts_with("minecraft:") && block.ends_with("_ore") {
            return true;
        }
        return matches!(
            block,
            "minecraft:stone"
                | "minecraft:cobblestone"
                | "minecraft:granite"
                | "minecraft:diorite"
                | "minecraft:andesite"
                | "minecraft:polished_granite"
                | "minecraft:polished_diorite"
                | "minecraft:polished_andesite"
                | "minecraft:deepslate"
                | "minecraft:cobbled_deepslate"
                | "minecraft:polished_deepslate"
                | "minecraft:tuff"
                | "minecraft:calcite"
                | "minecraft:dripstone_block"
                | "minecraft:netherrack"
                | "minecraft:blackstone"
                | "minecraft:basalt"
                | "minecraft:smooth_basalt"
                | "minecraft:end_stone"
                | "minecraft:sandstone"
                | "minecraft:red_sandstone"
        );
    }
    if item == STEEL_EXCAVATOR {
        return matches!(
            block,
            "minecraft:dirt"
                | "minecraft:coarse_dirt"
                | "minecraft:rooted_dirt"
                | "minecraft:grass_block"
                | "minecraft:podzol"
                | "minecraft:mycelium"
                | "minecraft:dirt_path"
                | "minecraft:farmland"
                | "minecraft:sand"
                | "minecraft:red_sand"
                | "minecraft:gravel"
                | "minecraft:clay"
                | "minecraft:snow"
                | "minecraft:snow_block"
                | "minecraft:powder_snow"
                | "minecraft:soul_sand"
                | "minecraft:soul_soil"
                | "minecraft:mud"
                | "minecraft:muddy_mangrove_roots"
        );
    }
    false
}
