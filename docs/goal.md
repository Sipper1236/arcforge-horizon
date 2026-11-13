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

## Current milestone — Heat Cell alpha slice, 2026-11-12

Four Heat Cell tiers are implemented with bounded HU storage/transfer, leakage, blockstate heat levels, charge-preserving block drops/placement, and crafting/loot data. `cargo check --locked --manifest-path api/Cargo.toml` passed, and Horizon built and checked the 388-recipe content pack (SHA-256 `40c675764f93faf5953bff98c74605000f2e5e5183743e663ffcd3ba9913e790`). Commit `6621649` is pushed. GitHub run [43329802649](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43329802649) passed the complete mod-package job; its artifact checksums are verified. In-world behavior remains unverified. Screens, capsule slots, insulation, redstone/side controls, and charge-preserving upgrades remain absent.

The previous client job stopped because its workflow required private Minecraft archive secrets. CI now fetches and hash-checks the official Minecraft 26.3 build inputs from Mojang. Runtime diagnosis also found two duplicate content declarations that made Horizon reject all native Arcforge registrations; those redundant files have been removed. The next GitHub run must confirm the current client artifact, followed by client import and in-world checks. The overall Arcforge port remains incomplete. See [CI build setup](ci-build.md).

The project has a broad early gameplay prototype: ore worldgen and material recipes, source machine recipe catalogs, several processors, a formed Crushing Array, steelmaking and heat/power slices including the Geothermal Plant, Energy/Item/Fluid/Thermal Conduits, Energy Cells, four Heat Cell tiers, Fluid Tanks with full-bucket exchange, Resin Tap behavior, treated wood, and basic Steel equipment. Exact details and evidence are in [implemented](status/implemented.md), [parity notes](status/parity-notes.md), and [verification](status/verification.md).

Recent slices fixed both pooled FE/HU conduit tick paths to use the current block entity passed by Horizon, added the missing source Invar Dust craft plus its Iron Dust common tag, wired FE/HU active glow, and implemented four Heat Cell tiers. Four Item Conduit tiers route bounded transfers between configured endpoints using Horizon's transactional item mover. Four Fluid Conduit tiers route one fluid type using Horizon's transactional mover and native sided capabilities for Auto faces. Item/fluid conduits lack buffers and animation; filtering is absent. The Rust crate formats and passes `cargo check --locked`; the 388-recipe content pack builds and passes Horizon's checker. GitHub Actions has now rebuilt and uploaded the updated mod package with verified checksums. These checks do not establish runtime registration or gameplay. The pinned native bundle still needs a current-source build and gameplay check. Solar Thermal Array heat-port overlays use standard blockstate multipart models and existing textures; the overlay remains uninspected in-world. Continue the README inventory in coherent slices. Do not treat compilation or content builds as evidence of in-world behavior.

## Build and play checkpoint (2026-11-12)

The previous GitHub build [43329802649](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43329802649) produced the mod package but stopped before building the client because the workflow unnecessarily required a private game-input archive. The workflow now downloads and hash-checks Mojang's public 26.3 metadata and client jar for preparation, without including the jar in the output. A runtime log also showed native registration failed atomically because two block files duplicated blocks registered by Rust; both duplicate declarations are removed. The player authorized `--trust-native-code`. Next, run GitHub Actions on the updated commit, import its artifact, and verify Arcforge item registration and the Heat Cell charge/transfer/drop-reload loop. Do not call the mod playable until those runtime checks pass.
