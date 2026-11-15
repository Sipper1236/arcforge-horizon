//! Steel Hammer and Excavator 3×3 mining behavior.
//!
//! Steel Hammer and Excavator 3×3 mining behavior.
//!
//! Neighboring blocks use Horizon's player-break helper, so their drops,
//! enchantments, events and tool wear follow the normal player path. The
//! current API still requires an explicit block allowlist with estimated
//! hardness for the source tool-effectiveness and area comparison rules.

pub(crate) const STEEL_HAMMER: &str = "arcforge:steel_hammer";
pub(crate) const STEEL_EXCAVATOR: &str = "arcforge:steel_excavator";
pub(crate) const DURABILITY: u32 = 1_500;
thread_local! {
    static PLAYER_BREAK_ACTIVE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
pub(crate) fn player_break_is_active() -> bool {
    PLAYER_BREAK_ACTIVE.with(std::cell::Cell::get)
}
pub(crate) struct PlayerBreakGuard;
impl PlayerBreakGuard {
    pub(crate) fn enter() -> Option<Self> {
        PLAYER_BREAK_ACTIVE.with(|active| {
            if active.replace(true) {
                None
            } else {
                Some(Self)
            }
        })
    }
}
impl Drop for PlayerBreakGuard {
    fn drop(&mut self) {
        PLAYER_BREAK_ACTIVE.with(|active| active.set(false));
    }
}

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

/// Select the face axis from Horizon's exact ray hit point. The block-break
/// event carries only the position, but the pinned Host raycast helper retains
/// the intersection point.
pub(crate) fn axis_from_hit(center: [i32; 3], point: [f64; 3]) -> Axis {
    let local = [
        point[0] - f64::from(center[0]),
        point[1] - f64::from(center[1]),
        point[2] - f64::from(center[2]),
    ];
    let face_distance = local.map(|value| value.abs().min((1.0 - value).abs()));
    if face_distance[0] <= face_distance[1] && face_distance[0] <= face_distance[2] {
        Axis::X
    } else if face_distance[1] <= face_distance[2] {
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

/// Horizon does not expose the vanilla effective-tool or destroy-speed query
/// in this hook. Approximate them for the source's common stone/ore and soil
/// groups; the source compares neighbor hardness with the center block.
pub(crate) fn supports_center(item: &str, block: &str) -> bool {
    hardness(item, block).is_some()
}

/// The pinned API supplies no block-hardness query, so use source vanilla
/// hardness values for the blocks the early survival progression mines.
pub(crate) fn allows_target(item: &str, center: &str, target: &str) -> bool {
    let Some(center_hardness) = hardness(item, center) else {
        return false;
    };
    hardness(item, target).is_some_and(|target_hardness| target_hardness <= center_hardness)
}

fn hardness(item: &str, block: &str) -> Option<f32> {
    if item == STEEL_HAMMER {
        if block.starts_with("arcforge:") && block.ends_with("_ore") {
            // Arcite requires a diamond pickaxe; the Steel Hammer is iron-tier.
            if block.ends_with("arcite_ore") {
                return None;
            }
            return Some(if block.contains("deepslate") {
                4.5
            } else {
                3.0
            });
        }
        if block.starts_with("minecraft:") && block.ends_with("_ore") {
            return Some(if block.starts_with("minecraft:deepslate_") {
                4.5
            } else {
                3.0
            });
        }
        return match block {
            "minecraft:stone"
            | "minecraft:granite"
            | "minecraft:diorite"
            | "minecraft:andesite"
            | "minecraft:polished_granite"
            | "minecraft:polished_diorite"
            | "minecraft:polished_andesite"
            | "minecraft:tuff"
            | "minecraft:dripstone_block" => Some(1.5),
            "minecraft:cobblestone" | "minecraft:blackstone" => Some(2.0),
            "minecraft:deepslate"
            | "minecraft:cobbled_deepslate"
            | "minecraft:polished_deepslate"
            | "minecraft:end_stone" => Some(3.0),
            "minecraft:calcite" | "minecraft:sandstone" | "minecraft:red_sandstone" => Some(0.75),
            "minecraft:netherrack" => Some(0.4),
            "minecraft:basalt" | "minecraft:smooth_basalt" => Some(1.25),
            _ => None,
        };
    }
    if item == STEEL_EXCAVATOR {
        return match block {
            "minecraft:dirt"
            | "minecraft:coarse_dirt"
            | "minecraft:rooted_dirt"
            | "minecraft:podzol"
            | "minecraft:sand"
            | "minecraft:red_sand"
            | "minecraft:soul_sand"
            | "minecraft:soul_soil"
            | "minecraft:mud" => Some(0.5),
            "minecraft:grass_block"
            | "minecraft:mycelium"
            | "minecraft:farmland"
            | "minecraft:gravel"
            | "minecraft:clay" => Some(0.6),
            "minecraft:dirt_path" | "minecraft:muddy_mangrove_roots" => Some(0.65),
            "minecraft:snow" => Some(0.1),
            "minecraft:snow_block" => Some(0.2),
            "minecraft:powder_snow" => Some(0.25),
            _ => None,
        };
    }
    None
}
