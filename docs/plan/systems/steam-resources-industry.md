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
**Cracking Unit alpha (legacy production package; gameplay pending):** the source 3×3×5 unit implements catalytic and hydrogen cracking, four product tanks, 50-batch catalyst wear, speed upgrades, thermal input, and Wrench mode switching. The current branch adds an in-screen selector, tooltips, and live input/product/heat/progress readouts. Fixed tank and box sizes, absent automatic output routing, regenerator geometry, and source side-tab layout remain alpha limits. Nine regression tests cover source recipe values, all four facings, input tank restrictions, output backpressure, catalyst bounds, speed timing, and screen behavior; the matching gameplay remains unverified.

**Current-API staging:** `api-next/` registers and parses Cracking recipes, saves tank operations, processes batches, wears catalysts, applies up to eight Speed Upgrades, consumes stored heat, and exposes menu data. Run [45103760092](https://github.com/Sipper1236/arcforge-horizon/actions/runs/45103760092) passed staged compilation/content/package checks for the eight-item slot cap. The latest Wrench hook toggles modes on the top face and selects facing on horizontal faces, with action-bar feedback; it awaits CI. Heat delivery, a current-API screen, remaining mod content, current-nightly production packaging, and gameplay verification remain outstanding.

**Oil survey and Pumpjack structure update (GitHub verified; gameplay pending):** Commit af81fc5 implements Arcforge's 3×5 Pumpjack structure and craftable Well Head stand-in. GitHub Actions run 43783919669 built it; gameplay remains unverified. Seed-based regional deposits share a world-saved remaining-volume store. The Seismic Survey Tool reports depletion; the single-block Pumpjack draws power and outputs source-type crude, using Arcforge’s 60 FE/t, 30 mB/t full-field rate, 25% minimum share, and 40 mB/s depleted-field trickle. The source 3×5 structure, derrick/wellhead drilling, biome richness, oil caverns, and gas remain unimplemented. GitHub Actions run [43780024722](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43780024722) passed package and Linux client builds; gameplay testing is pending.
**Superheater Array alpha (2026-11-15, GitHub verified):** api/src/superheater.rs adds a 3–7 block cubic casing shell with a separate controller, persisted heat and steam tanks, Steam/High-Pressure/Superheated grades, Auto and fixed grade modes, and thermal-conduit heat input. It follows Arcforge defaults: 200,000 HU, 2,000 HU/t, 16,000 mB and 1,000 mB/t per 27-block cube, 1,400°C maximum, and a 0.6 heat-cost multiplier. Wrench interaction sets facing or cycles pressure mode; the action bar reports shell, mode, temperature, tanks, and flow. The native fluid capability has fixed maximum-size tanks and all-side ports; internal processing scales to the formed cube size. GitHub Actions run 43773403158 passed the native crate check, content build/check, packaging, and Linux client build. This alpha has not been gameplay-tested yet.

**Electric Pump alpha (2026-11-15):** api/src/electric_pump.rs registers the source-default 10 FE/t, 1,000 mB per 20 ticks pump with a 20,000 FE buffer and 16,000 mB water tank. It extracts only from infinite water sources and exposes output to fluid conduits. Finite extraction and seawater conversion remain API-limited; GitHub Actions run 43769113792 passed Rust/content and Linux client builds; no game test has been run.

**Industrial Water Intake alpha (2026-11-15):** api/src/water_intake.rs adds the fixed 3×3×3 shell, horizontal-face controller, bounded 400-source-water scan, FE-powered pumping (8,000 mB/t at 0.25 FE/mB), and 128,000 mB output tank. Water is not consumed. The pinned Host API has no biome or loaded-chunk query, so the river/ocean multiplier and unloaded-chunk filtering are deferred; test the installed alpha in game.
**Condenser Array alpha (2026-11-15):** `api/src/steam_condenser.rs` forms a 3–7 block box around a separate controller, accepts Exhaust Steam, and returns water 1:1. Its open-air rate is 400 mB/t per 27 blocks, touching water/ice adds the source bonuses, and the 800 mB/t per-cube cap is retained. It has fixed 203,259 mB tanks (the maximum source box capacity) and no biome-temperature multiplier because the pinned Host hooks do not expose biome climate here. Recipes, drops, item models, localization, and handbook instructions are present. It compiled and packaged in the rx580 hybrid client build; formation, cooling, fluid transfer, and save/reload remain unverified. The separate controller and fixed maximum-sized tanks are alpha simplifications.

**Steam Turbine Array alpha (2026-11-15):** `api/src/steam_turbine.rs` forms 3–7 by 3–7 cross-sections with 3–27 length, consumes steam grades and lubricant, generates FE with RPM ramping and source-style bonuses, and outputs Exhaust Steam. The controller recipe, model/state mapping, item model, and loot table are in place. The pinned Horizon callbacks provide only block IDs for fluid and energy declarations, so capacity and connection faces are fixed for all turbines. The source and importable Linux client package compile on rx580; survival use and piping remain unverified.


**Steam Boiler Array expanded source slice (2026-11-15):** `api/src/steam_boiler.rs` recognizes footprints 3×3 through 7×7 and heights 3–16, scales heat capacity and maximum heat rate by boiler sections, models the four pressure modes and steam grades, and implements boiling and pressure/temperature venting. Shell checks follow the pinned Arcforge rules: all corners are casing; for a footprint other than 3×3, all edges are casing; other boundary faces may be casing or unformed Pressure Glass; the core must be air. The Horizon alpha uses a separate controller centered on the front wall, a two-slot bucket interface, and Wrench status/controls. Pressure mode cycles when the Wrench hits the current-facing or vertical side; a different horizontal side changes facing.

The boiler-to-turbine-to-condenser fluid path has compatible Steam, Exhaust Steam, and Water tank declarations and can be connected with the existing fluid conduits; this circuit still needs in-game verification.
The current Horizon fluid capability is declared by block ID and does not provide the target block-entity position. The boiler therefore retains a fixed 48,000 mB per fluid tank (the 3×3×3 source capacity) even as heat scales with the larger shell. It also retains the separate Horizon controller in place of Arcforge's casing master. The expanded source compiles through the rx580 hybrid build, but this is not client or in-game verification. Do not call Steam Power complete: confirm the shell, controls, bucket input, heat transfer, steam grades, venting, and save/reload in a matching client; then verify turbine/condenser behavior and continue water intake and closed loops. The Electric Pump source-block pickup remains deferred because the current Horizon Host API lacks safe source-fluid pickup and removal semantics; do not change Horizon API.

## API-constrained alpha decision — Electric Pump (2026-11-15)

The alpha operates only over an infinite water source: it requires at least two horizontal neighboring source-water blocks and leaves that pool in place, matching Arcforge's default infinite-water case. Finite-fluid source extraction remains deferred because the pinned Host exposes no source-fluid pickup operation; destroying a block is not equivalent. The alpha also omits the ocean/beach Seawater conversion because no biome query is exposed. No Horizon API hooks were changed.

## Acceptance checks

Each industrial chain must be obtainable in survival, bounded by input/output capacity, persistent after save/reload, and covered by a scenario that checks its outputs and byproducts. Nuclear behavior additionally needs controlled shutdown and explicit safety tests.
