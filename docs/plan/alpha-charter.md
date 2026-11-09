# Alpha charter

## Purpose

Build a Horizon-native Arcforge port that gives players the same recognizable industrial progression as the pinned Arcforge release. The full target remains the survival feature inventory in the root README; an early alpha is a checkpoint on that path, not a replacement for it.

## Player experience

The intended experience includes discovering new ores, making power, building machines, making steel, and automating production, while staying close to Arcforge. The user is open to having the alpha follow Arcforge's original progression and recipe gates; use the pinned 20-chapter questline as the player-facing unlock reference. Do not move ores ahead of steel if the upstream progression gates them later. See [goal.md](../goal.md) and [handoff.md](../handoff.md) for the current project objective and session state.

The quest book and custom quests are optional polish. Machine progression and recipes must work without a quest mod.

## Fidelity rules

1. Reuse Arcforge source assets, recipes, tags, and data where their licenses permit and Horizon can consume them.
2. Translate machine behavior into Rust using Horizon's current native mod hooks.
3. Preserve upstream recipe inputs, outputs, durations, costs, and gates when supported.
4. Simplify only when the pinned Horizon modding API cannot express a behavior reliably. Record each change in the parity notes and keep the resulting progression understandable.
5. Keep the complete Arcforge feature inventory as the target, including features deferred from the first alpha.

## Alpha boundary

Deliver the gameplay in small releases that follow the source progression. Start with the opening industrial path, then add later Arcforge chapters in order. Each release should have a playable survival route, working machine behavior, and the verification described in the validation plan. The exact cut for a public alpha can be selected after the early progression is playable; it does not reduce the full-port target.

## Explicit exclusions from this plan

- No changes to Horizon's modding API, source, generated hook points, or upstream API proposals.
- No direct execution of the original Java mod jar.
- No removal of upstream features from the full-port target merely because they are not in the first alpha.
