# Pinned Horizon integration boundary

## Fixed target

The native mod targets Horizon commit `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca` (Nightly 2026-11-08), as recorded in `horizon-mixin.toml` and `api/Cargo.toml`. The content mod uses the Horizon content manifest API version `0.2`.

## No API changes

This project plan does not include changes to Horizon's modding API, engine source, hook points, or generated bindings. Do not make upstream API proposals part of the alpha plan.

## Use the existing surfaces

- Put supported data and assets in `.hmod`: registered blocks/items, recipes, tags, models, textures, loot, structures, and world-generation data that the pinned content format accepts.
- Put behavior in the Rust native mod: machine ticks, inventory/energy/fluid handling, player interactions, and screens/rendering through hooks present at the pinned revision.
- Keep simulation deterministic, bounded, and server-authoritative within those hooks.
- Use existing mod-side abstractions where they simplify repeated machine behavior without replacing Horizon's runtime.

## Unsupported behavior rule

For a feature the pinned API does not support:

1. Check whether the current content format or native hooks can implement it within the mod.
2. If a reliable mod-side implementation preserves the gameplay, use it and test it.
3. If not, simplify or defer the feature for alpha and document the exact difference from Arcforge.
4. Revisit only after the mod reaches alpha; do not make an API change a prerequisite for current progress.

The current README already records some known gaps, including complete heat logistics, custom item components, full screens, and formed arrays. New limitations belong in the relevant system plan and [not-built inventory](../status/not-built.md).
