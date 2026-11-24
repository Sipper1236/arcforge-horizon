# System plan: steam, resources, and industry

## Target

Continue in Arcforge's chapter order through Steam Power, Turbines & Engines, Oil & Gas, Distillation & Refinery, Chemistry & Plastics, Salt & Lithium, Farming, Bio-Industry, and the four nuclear/endgame chapters.

## Build order

1. Add water intake, pumps, boilers, steam grades, turbines, superheaters, condensers, and closed loops.
2. Add deterministic world oil deposits, survey/extraction tools, wells, derricks, and depletion.
3. Add distillation and refinery recipes, fluids/gases, catalysts, byproducts, and fuel users.
4. Add chemistry, salt/brine/lithium, batteries, farming, and biofuel chains in their source order.
5. Complete nuclear ore/material processing and fuel before reactors; then add generation, safety, fuel cycle, and advanced cores.

## Design rules

- Use data-driven recipes and worldgen formats accepted by the pinned content mod.
- Persist deposit state, tanks, pressure/temperature, byproducts, and machine progress.
- Bound all scans, transfer rates, network sizes, and per-tick work.
- Keep cycles from creating free energy or unbounded products.
- If a mechanic cannot be expressed with the current mod hooks, simplify or defer it for alpha and record the deviation. Do not change Horizon's modding API.

## Current state
**Lubricant Blender alpha (GitHub verified; gameplay pending):** commit `07b59a7` adds the source Heavy Oil recipe (100 mB → 90 mB Lubricant, 40 ticks, 20 FE/t), bounded 16,000/8,000 mB tanks, 50,000 FE storage with 1,000 FE/t input, Speed/Energy Upgrade slots, Wrench status, and a screen. Run [45325717893](https://github.com/Sipper1236/arcforge-horizon/actions/runs/45325717893) passed the package and Linux client jobs. All capability faces are exposed; automatic source-style 500 mB/t output and configurable side modes remain unported. Gameplay verification is pending.

**Hydrotreater alpha (GitHub Actions verified; gameplay pending):** commits `8711736` and `8cbd807` add source recipes for sour Kerosene and Diesel, using Hydrogen and producing clean fuel plus Hydrogen Sulfide. It implements source 40-tick processing at 300°C, 20 HU/t, and 20 FE/t, two bounded inputs and outputs, FE and heat storage, upgrades, Wrench status, crafting/loot, and a screen. Run [45331449362](https://github.com/Sipper1236/arcforge-horizon/actions/runs/45331449362) passed both jobs. All-side tank and energy capabilities, automatic output ejection, configurable side modes, and gameplay remain unverified.

**Sulfur Recovery Unit alpha (GitHub Actions pending; gameplay pending):** `api/src/sulfur_recovery.rs` implements the source 100 mB Hydrogen Sulfide + 50 mB Oxygen → 100 mB Steam + Sulfur Dust process over 10 ticks at 15 HU/t and 250°C, with bounded tanks, a Speed Upgrade slot, sulfur output, Wrench status, crafting/loot, and a screen. The source recipe, craft, and loot files are copied into Horizon content. This slice has not yet been built by Actions. Fluid capability access is all-sided; automatic source-style ejection/routing and configurable side modes remain unimplemented.
**Oil survey and Pumpjack structure update (GitHub verified; gameplay pending):** Commit af81fc5 implements Arcforge's 3×5 Pumpjack structure and craftable Well Head stand-in. GitHub Actions run 43783919669 built it; gameplay remains unverified. Seed-based regional deposits share a world-saved remaining-volume store. The Seismic Survey Tool reports depletion; the single-block Pumpjack draws power and outputs source-type crude, using Arcforge’s 60 FE/t, 30 mB/t full-field rate, 25% minimum share, and 40 mB/s depleted-field trickle. The source 3×5 structure, derrick/wellhead drilling, biome richness, oil caverns, and gas remain unimplemented. GitHub Actions run [43780024722](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43780024722) passed package and Linux client builds; gameplay testing is pending.
**Superheater Array alpha (2026-11-15, GitHub verified):** api/src/superheater.rs adds a 3–7 block cubic casing shell with a separate controller, persisted heat and steam tanks, Steam/High-Pressure/Superheated grades, Auto and fixed grade modes, and thermal-conduit heat input. It follows Arcforge defaults: 200,000 HU, 2,000 HU/t, 16,000 mB and 1,000 mB/t per 27-block cube, 1,400°C maximum, and a 0.6 heat-cost multiplier. Wrench interaction sets facing or cycles pressure mode; the action bar reports shell, mode, temperature, tanks, and flow. The native fluid capability has fixed maximum-size tanks and all-side ports; internal processing scales to the formed cube size. GitHub Actions run 43773403158 passed the native crate check, content build/check, packaging, and Linux client build. This alpha has not been gameplay-tested yet.

**Electric Pump alpha (2026-11-15):** api/src/electric_pump.rs registers the source-default 10 FE/t, 1,000 mB per 20 ticks pump with a 20,000 FE buffer and 16,000 mB water tank. It extracts only from infinite water sources and exposes output to fluid conduits. Finite extraction and seawater conversion remain API-limited; GitHub Actions run 43769113792 passed Rust/content and Linux client builds; no game test has been run.

**Industrial Water Intake alpha (2026-11-15):** api/src/water_intake.rs adds the fixed 3×3×3 shell, horizontal-face controller, bounded 400-source-water scan, FE-powered pumping (8,000 mB/t at 0.25 FE/mB), and 128,000 mB output tank. Water is not consumed. The pinned Host API has no biome or loaded-chunk query, so the river/ocean multiplier and unloaded-chunk filtering are deferred; test the installed alpha in game.
**Condenser Array alpha (2026-11-15):** `api/src/steam_condenser.rs` forms a 3–7 block box around a separate controller, accepts Exhaust Steam, and returns water 1:1. Its open-air rate is 400 mB/t per 27 blocks, touching water/ice adds the source bonuses, and the 800 mB/t per-cube cap is retained. It has fixed 203,259 mB tanks (the maximum source box capacity) and no biome-temperature multiplier because the pinned Host hooks do not expose biome climate here. Recipes, drops, item models, localization, and handbook instructions are present. It compiled and packaged in the rx580 hybrid client build; formation, cooling, fluid transfer, and save/reload remain unverified. The separate controller and fixed maximum-sized tanks are alpha simplifications.

**Steam Turbine Array alpha (2026-11-15; routing update GitHub verified 2026-11-24):** `api/src/steam_turbine.rs` forms 3–7 by 3–7 cross-sections with 3–27 length, consumes steam grades and lubricant, generates FE with RPM ramping and source-style bonuses, and outputs Exhaust Steam. The controller recipe, model/state mapping, item model, and loot table are in place. The latest slice routes generated FE to adjacent supported targets using Wrench-configured faces and the turbine's tier rate. Horizon's native energy capability remains a block-ID declaration and advertises all faces; the mod-side route applies face preferences. GitHub Actions run [45311824901](https://github.com/Sipper1236/arcforge-horizon/actions/runs/45311824901) passed both jobs. Survival formation, steam/exhaust routing, FE output, and persistence remain unverified.


**Steam Boiler Array expanded source slice (2026-11-15; scan update GitHub verified 2026-11-24):** `api/src/steam_boiler.rs` recognizes footprints 3×3 through 7×7 and heights 3–16, scales heat capacity and maximum heat rate by boiler sections, models the four pressure modes and steam grades, and implements boiling and pressure/temperature venting. Structure scans are throttled to every 20 ticks, with immediate checks on initialization and facing changes. Shell checks follow the pinned Arcforge rules: all corners are casing; for a footprint other than 3×3, all edges are casing; other boundary faces may be casing or unformed Pressure Glass; the core must be air. The Horizon alpha uses a separate controller centered on the front wall, a two-slot bucket interface, and Wrench status/controls. Pressure mode cycles when the Wrench hits the current-facing or vertical side; a different horizontal side changes facing.

The boiler-to-turbine-to-condenser fluid path has compatible Steam, Exhaust Steam, and Water tank declarations and can be connected with the existing fluid conduits; this circuit still needs in-game verification.
The current Horizon fluid capability is declared by block ID and does not provide the target block-entity position. The boiler therefore retains a fixed 48,000 mB per fluid tank (the 3×3×3 source capacity) even as heat scales with the larger shell. It also retains the separate Horizon controller in place of Arcforge's casing master. The Steam Boiler scan and Steam Turbine routing updates passed the package and Linux client jobs in run [45311824901](https://github.com/Sipper1236/arcforge-horizon/actions/runs/45311824901); runtime behavior remains unverified. Do not call Steam Power complete: confirm the shell, controls, bucket input, heat transfer, steam grades, venting, FE output, and save/reload in a matching client; then verify the turbine/condenser loop and continue water intake and closed loops. The Electric Pump source-block pickup remains deferred because the current Horizon Host API lacks safe source-fluid pickup and removal semantics; do not change Horizon API.

## API-constrained alpha decision — Electric Pump (2026-11-15)

The alpha operates only over an infinite water source: it requires at least two horizontal neighboring source-water blocks and leaves that pool in place, matching Arcforge's default infinite-water case. Finite-fluid source extraction remains deferred because the pinned Host exposes no source-fluid pickup operation; destroying a block is not equivalent. The alpha also omits the ocean/beach Seawater conversion because no biome query is exposed. No Horizon API hooks were changed.

## Acceptance checks

Each industrial chain must be obtainable in survival, bounded by input/output capacity, persistent after save/reload, and covered by a scenario that checks its outputs and byproducts. Nuclear behavior additionally needs controlled shutdown and explicit safety tests.
