//! Pinned Arcforge Flare Stack: a gas sink that burns accepted gases safely.

use horizon_client::{
    containers::ModBlockEntity,
    mod_api::{
        BlockEntityDraw, BlockEntityRenderBoundingBoxEvent, BlockEntitySubmitEvent, Compound,
        CubeFace, Host, ModFluidTanks, Tag,
    },
};

pub(crate) const BLOCK: &str = "arcforge:flare_stack";

const TANK: usize = 0;
const TANK_CAPACITY: u32 = 4_000;
const BURN_RATE: u32 = 1_000;
const FLAME_SCALE: i32 = 1_000;
const FLAME_DECAY: i32 = 20;
const FLAME_SPRITE: &str = "arcforge:block/flare_stack/flame";
const PIPE_SPRITE: &str = "arcforge:block/flare_stack/side";
const FLANGE_SPRITE: &str = "arcforge:block/flare_stack/top";

fn nbt_i32(nbt: &Compound, name: &str) -> i32 {
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

pub(crate) fn accepts(tank: usize, fluid: &str) -> bool {
    tank == TANK
        && crate::fluid_conduit::is_arcforge_gas(fluid)
        && !matches!(
            fluid,
            "arcforge:uranium_hexafluoride"
                | "arcforge:depleted_uranium_hexafluoride"
                | "arcforge:nitrogen_dioxide"
        )
}

pub(crate) fn fluid_tanks() -> ModFluidTanks {
    ModFluidTanks {
        tanks: vec![TANK_CAPACITY as i32],
        // The source Flare Stack takes gas from every face except the top.
        faces: [vec![0], vec![], vec![0], vec![0], vec![0], vec![0]],
        inputs: vec![0],
        outputs: vec![],
    }
}

pub(crate) fn tick(host: &mut dyn Host, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let amount = crate::fluid::amount(&entity.nbt, TANK);
    let fluid = crate::fluid::name(&entity.nbt, TANK);
    let burnt = if amount > 0 && fluid.as_deref().is_some_and(|fluid| accepts(TANK, fluid)) {
        let fluid = fluid.as_deref().unwrap_or_default();
        crate::fluid::extract(&mut entity.nbt, TANK, fluid, amount.min(BURN_RATE))
    } else {
        0
    };

    // The source grows the flame to four times the fraction of burn rate and
    // lets it fade by 0.02 each tick after the gas pulse ends.
    let target = (burnt.saturating_mul(4).min(BURN_RATE) as i32)
        .min(FLAME_SCALE)
        .max(0);
    let previous = nbt_i32(&entity.nbt, "FlareStackFlame").clamp(0, FLAME_SCALE);
    let flame = if target > previous {
        target
    } else {
        previous.saturating_sub(FLAME_DECAY).max(target)
    };
    set_i32(&mut entity.nbt, "FlareStackFlame", flame);
    set_lit(host, pos, flame > 0);
}

fn draw_box(
    host: &dyn Host,
    draw: &mut BlockEntityDraw,
    from: [f32; 3],
    to: [f32; 3],
    sprite: &str,
) {
    let face = CubeFace { sprite, uv: None };
    let _ = host.draw_cube(draw, from, to, [Some(face); 6], 0x00FF_FFFF);
}

pub(crate) fn render_bounds(event: &mut BlockEntityRenderBoundingBoxEvent<'_>) {
    event
        .bounds
        .copy_from_slice(&[-1.0, 0.0, -1.0, 2.0, 10.0, 2.0]);
}

fn flame_quad(host: &dyn Host, draw: &mut BlockEntityDraw, corners: [[f32; 3]; 4]) {
    let _ = host.draw_quad(
        draw,
        FLAME_SPRITE,
        corners,
        [0.0, 0.0, 16.0, 32.0],
        0x00FF_FFFF,
    );
    let reversed = [corners[3], corners[2], corners[1], corners[0]];
    let _ = host.draw_quad(
        draw,
        FLAME_SPRITE,
        reversed,
        [0.0, 0.0, 16.0, 32.0],
        0x00FF_FFFF,
    );
}

pub(crate) fn draw_model(host: &dyn Host, event: &mut BlockEntitySubmitEvent<'_>) {
    let draw = &mut *event.draw;

    // The pipe runs four blocks above the base block, with its flanges and
    // service platform at the same heights as the pinned FlareStackMesh.
    draw_box(
        host,
        draw,
        [1.5, 16.0, 1.5],
        [14.5, 17.0, 14.5],
        FLANGE_SPRITE,
    );
    draw_box(
        host,
        draw,
        [5.0, 17.0, 5.0],
        [11.0, 74.0, 11.0],
        PIPE_SPRITE,
    );
    for y in [33.0, 51.0] {
        draw_box(
            host,
            draw,
            [4.2, y, 4.2],
            [11.8, y + 1.5, 11.8],
            FLANGE_SPRITE,
        );
    }
    draw_box(
        host,
        draw,
        [4.0, 74.0, 4.0],
        [12.0, 76.0, 12.0],
        PIPE_SPRITE,
    );
    draw_box(
        host,
        draw,
        [3.2, 76.0, 3.2],
        [12.8, 80.0, 12.8],
        FLANGE_SPRITE,
    );
    draw_box(
        host,
        draw,
        [4.0, 76.0, 4.0],
        [12.0, 78.0, 12.0],
        PIPE_SPRITE,
    );

    // A railed work platform with its ladder and pilot line.
    draw_box(
        host,
        draw,
        [1.0, 60.5, 1.0],
        [15.0, 62.0, 15.0],
        FLANGE_SPRITE,
    );
    for (from, to) in [
        ([1.0, 62.0, 1.0], [2.0, 69.0, 2.0]),
        ([14.0, 62.0, 1.0], [15.0, 69.0, 2.0]),
        ([1.0, 62.0, 14.0], [2.0, 69.0, 15.0]),
        ([14.0, 62.0, 14.0], [15.0, 69.0, 15.0]),
    ] {
        draw_box(host, draw, from, to, PIPE_SPRITE);
    }
    draw_box(
        host,
        draw,
        [1.0, 69.0, 1.0],
        [15.0, 70.0, 2.0],
        FLANGE_SPRITE,
    );
    draw_box(
        host,
        draw,
        [1.0, 69.0, 2.0],
        [2.0, 70.0, 14.0],
        FLANGE_SPRITE,
    );
    draw_box(
        host,
        draw,
        [14.0, 69.0, 2.0],
        [15.0, 70.0, 15.0],
        FLANGE_SPRITE,
    );
    draw_box(
        host,
        draw,
        [2.0, 69.0, 14.0],
        [4.0, 70.0, 15.0],
        FLANGE_SPRITE,
    );
    for x in [4.5, 13.5] {
        draw_box(
            host,
            draw,
            [x - 0.4, 16.0, 13.6],
            [x + 0.4, 70.0, 14.4],
            PIPE_SPRITE,
        );
    }
    for y in (19..60).step_by(4) {
        draw_box(
            host,
            draw,
            [4.5, y as f32 - 0.3, 13.7],
            [13.5, y as f32 + 0.3, 14.3],
            FLANGE_SPRITE,
        );
    }
    draw_box(
        host,
        draw,
        [7.7, 18.0, 4.0],
        [8.3, 79.0, 4.6],
        FLANGE_SPRITE,
    );

    let flame = nbt_i32(&event.entity.nbt, "FlareStackFlame").clamp(0, FLAME_SCALE) as f32
        / FLAME_SCALE as f32;
    if flame > 0.0 {
        let light = draw.light;
        draw.light = 0x0000_F0F0;
        let height = (16.0 + 40.0 * flame) / 16.0;
        let half = height / 4.0;
        let y = 78.0 / 16.0;
        let cx = 8.0 / 16.0;
        let cz = 8.0 / 16.0;
        flame_quad(
            host,
            draw,
            [
                [cx - half, y, cz],
                [cx + half, y, cz],
                [cx + half, y + height, cz],
                [cx - half, y + height, cz],
            ],
        );
        flame_quad(
            host,
            draw,
            [
                [cx, y, cz - half],
                [cx, y, cz + half],
                [cx, y + height, cz + half],
                [cx, y + height, cz - half],
            ],
        );
        draw.light = light;
    }
}
