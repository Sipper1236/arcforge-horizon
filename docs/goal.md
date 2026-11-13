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

The client workflow no longer needs private secrets. Run [43339016409](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43339016409) passed package checks, fetched and verified the client plus 66 Linux libraries, and prepared Horizon. Native-build failed because the nested Horizon checkout's `CLAUDE.md` symlink was inside the Arcforge folder it scanned; CI now stages only the manifest and API sources into a clean path. Runtime diagnosis also found duplicate declarations that made Horizon reject all native Arcforge registrations; their custom states are now in Rust and the duplicate files are removed. The next GitHub run must build the current client, followed by client import and in-world checks. The overall Arcforge port remains incomplete. See [CI build setup](ci-build.md).

The project has a broad early gameplay prototype: ore worldgen and material recipes, source machine recipe catalogs, several processors, a formed Crushing Array, steelmaking and heat/power slices including the Geothermal Plant, Energy/Item/Fluid/Thermal Conduits, Energy Cells, four Heat Cell tiers, Fluid Tanks with full-bucket exchange, Resin Tap behavior, treated wood, and basic Steel equipment. Exact details and evidence are in [implemented](status/implemented.md), [parity notes](status/parity-notes.md), and [verification](status/verification.md).

### Latest work — Portable FE Batteries (2026-11-13)

Four portable FE Battery items now expose Horizon item-energy capabilities at the pinned Arcforge capacities and rates, with four source-shaped recipes and localized names. Charge is not copied by tier upgrades, and the Arcforge Energy Cell charging interaction is unavailable through the pinned hooks. Local `cargo check` passed with zero errors and the existing `EnergyStore::transfer_to` warning; Horizon built and checked all 400 recipes (SHA-256 `82db30b04bb7b40457e813fb2a717a7cd4065ddf0da964090bc7d15e938aefba`). The hybrid remote Rust build remains blocked by missing nested Plasma XML files on its worker; this is documented in [verification.md](status/verification.md). GitHub run [43364355794](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43364355794) passed both jobs for `1585e3d`. Runtime testing found and fixed the Cylinder gauge's invalid integer property in commit `f78b881`; run [43366816469](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43366816469) passed both jobs and its client bundle verified. The corrected client opened a disposable world without an account and registered/granted `arcforge:wrought_battery`. Battery energy transfer, charging, and persistence remain unverified.

The preceding cylinder slice is pushed as `dc3987d`. GitHub run [43360699934](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43360699934) completed successfully for both package and Linux native-client jobs. Its artifacts and checksums were downloaded and verified. This confirms CI built the cylinder slice, not the newer local battery changes or full gameplay behavior.

Recent slices fixed both pooled FE/HU conduit tick paths to use the current block entity passed by Horizon, added the missing source Invar Dust craft plus its Iron Dust common tag, wired FE/HU active glow, and implemented four Heat Cell tiers. Four Item Conduit tiers route bounded transfers between configured endpoints using Horizon's transactional item mover. Four Fluid Conduit tiers route one fluid type using Horizon's transactional mover and native sided capabilities for Auto faces. Item/fluid conduits lack buffers and animation; filtering is absent. Rust formatting and `cargo check --locked` pass. Horizon's content build/check passes with ore features converted to the pinned runtime schema. A disposable supported native build and imported-client smoke confirmed custom item IDs register when the matching `native-mods.json` accompanies the imported executable. GitHub run [43360699934](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43360699934) subsequently passed the package and Linux-client jobs for the cylinder slice. This smoke does not establish machine behavior, progression, saves, conduit visuals, or ore frequency. Continue the README inventory in coherent slices and keep those distinctions in the implementation docs.

## Build and play checkpoint (2026-11-12)

GitHub Actions run [43340862715](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43340862715) passed both package and Linux native-client builds. A disposable native build and imported-client smoke identified three practical issues: the artifact needs Horizon's generated `native-mods.json`; `raw_rubber` was declared twice; and ore feature JSON used an unsupported older shape. The working tree now packages the manifest, removes the duplicate, and converts all 15 ore features to Horizon 26.3's flattened `worldgen/feature/` schema. Local checks confirmed Arcforge items register and the converted placed features load.

The workflow/docs corrections and runtime source/data fixes still need a new GitHub Actions run. The smoke did not verify the ore distribution, crafting progression, machine operation, Heat Cell transfer or persistence, or multiplayer. Conduit and Solar multipart blockstate warnings remain. Continue with these as-built distinctions; do not call the full mod playable based only on registration and a world opening.
