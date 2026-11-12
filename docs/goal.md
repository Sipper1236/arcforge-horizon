# Active project goal

## Objective

Port the complete survival gameplay scope of the pinned Arcforge release to Horizon as a Rust-native mod, using the pinned Horizon modding API without changing it. The parity target is Arcforge commit `8843cb02cf9132f278f6a8e804b6d78837d3cbcb`; the Horizon target is commit `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca` (Nightly 2026-11-08).

The current code is an early prototype, not a complete port. Continue implementing real gameplay from the scope in the root [README](../README.md), preserve Arcforge's recognizable mechanics and progression, and update the as-built and remaining-work documents whenever implementation changes.

## Player progression

The desired experience is to discover new ores, generate power, build machines, make steel, and automate production, while staying close to Arcforge. The alpha may follow Arcforge's original chapter order; use the pinned [progression map](plan/progression.md) for player unlocks and build prerequisites early when needed. Do not invent easier recipes that bypass source gates without documenting an explicit alpha simplification.

## Constraints

- Do not change Horizon's modding API, engine source, hooks, or generated bindings.
- Use only the interfaces in the pinned Horizon revision. When an interface cannot express a behavior reliably, implement a faithful mod-side alternative if possible; otherwise simplify or defer it and record the player-visible difference.
- Preserve upstream source behavior, recipe data, and assets when licensing and Horizon's content format permit. Keep attribution and third-party licensing records accurate.
- Keep machine simulation bounded, deterministic, save-safe, and server-authoritative within the existing hooks.
- Do not call a feature complete because its ID, recipe, or model exists. Distinguish source presence, build, load, behavior test, and parity completion.

## Completion criteria

The port is complete when every feature in the pinned README inventory is one of:

1. Implemented with behavior, recipes, assets, and survival access matching Arcforge, and validated in the game; or
2. Explicitly simplified/deferred with a precise reason, documented player impact, and a known limit.

The final quality pass must cover each progression chapter, clean installation, existing saves, save/reload, inventory and resource retention, output backpressure, supported automation, LAN/dedicated-server consistency, performance bounds, and source/content parity. API changes are outside this objective.

## Current milestone — build checkpoint 2026-11-09

The Pressurized Conduit slice now passes Rust formatting/type-check and is included in the checked 384-recipe content archive. GitHub Actions run [42679842491](https://github.com/Sipper1236/arcforge-horizon/actions/runs/42679842491) passed and uploaded that archive. Native registration and all new conduit behavior remain unverified in-game because the instance-specific Horizon bundle has not been built; Horizon requires the player's native-code trust decision. The larger Arcforge port remains incomplete.

The project has a broad early gameplay prototype: ore worldgen and material recipes, source machine recipe catalogs, several working processors, a formed Crushing Array, steelmaking and heat/power slices including the Geothermal Plant, Energy, Item, and Fluid Conduits, Energy Cells, Fluid Tanks with full-bucket exchange, Resin Tap behavior, treated wood, and basic Steel equipment. Exact details and evidence are in [implemented](status/implemented.md), [parity notes](status/parity-notes.md), and [verification](status/verification.md).

Recent slices fixed both pooled FE/HU conduit tick paths to use the current block entity passed by Horizon, added the missing source Invar Dust craft plus its Iron Dust common tag, and wired FE/HU active glow to stored resources and recent transfers. Four Item Conduit tiers route bounded transfers between configured endpoints using Horizon's transactional item mover. Four Fluid Conduit tiers route one fluid at a time with Horizon's transactional fluid mover and use native sided capabilities for Auto faces. Both first passes lack conduit buffers and animation; filtering is absent. The Rust crate now formats and passes `cargo check --locked`; the 384-recipe content pack builds and passes Horizon's checker, including the Pressurized Conduit resources. These checks do not establish runtime registration or in-world conduit behavior. The pinned native bundle still requires the player's native-code trust decision; current-source load and gameplay remain unverified. Solar Thermal Array heat-port overlays use standard blockstate multipart models and existing textures; the overlay remains uninspected in-world. Continue the README inventory in coherent slices. Do not treat compilation or content builds as evidence of in-world behavior.

## Build and play checkpoint (2026-11-12)

The 384-recipe content archive is checked and installed in the enabled disposable `arcforge-port-test` instance. The matching native source passes formatting and `cargo check --locked`, and GitHub Actions has a successful content-pack artifact build. The native game bundle is not built, so the current mod is not yet playable: its native registrations are not present in the instance. The player explicitly authorized the full-access native build. The attempt reached the pinned Horizon revision but failed fetching Cargo dependencies from crates.io after SSL resets/timeouts. GitHub Actions run [43094118580](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43094118580) built the `.hmod` and `.hnmod`, but inspection found the `.hnmod` crate path did not match its declared `api/` folder. That packaging fix is now in source and awaits a replacement Actions run. Retry the local pinned client build when dependency downloads work, then test item registration and a basic survival progression loop in a disposable world. Do not call the mod playable until those runtime checks pass.
