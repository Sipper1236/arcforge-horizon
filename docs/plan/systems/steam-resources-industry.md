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

**Steam Boiler Array expanded source slice (2026-11-15):** `api/src/steam_boiler.rs` recognizes footprints 3×3 through 7×7 and heights 3–16, scales heat capacity and maximum heat rate by boiler sections, models the four pressure modes and steam grades, and implements boiling and pressure/temperature venting. Shell checks follow the pinned Arcforge rules: all corners are casing; for a footprint other than 3×3, all edges are casing; other boundary faces may be casing or unformed Pressure Glass; the core must be air. The Horizon alpha uses a separate controller centered on the front wall, a two-slot bucket interface, and Wrench status/controls. Pressure mode cycles when the Wrench hits the current-facing or vertical side; a different horizontal side changes facing.

The current Horizon fluid capability is declared by block ID and does not provide the target block-entity position. The boiler therefore retains a fixed 48,000 mB per fluid tank (the 3×3×3 source capacity) even as heat scales with the larger shell. It also retains the separate Horizon controller in place of Arcforge's casing master. The expanded source compiles through the rx580 hybrid build, but this is not client or in-game verification. Do not call Steam Power complete: confirm the shell, controls, bucket input, heat transfer, steam grades, venting, and save/reload in a matching client; then continue turbines, condensers, water intake and closed loops. The Electric Pump source-block pickup remains deferred because the current Horizon Host API lacks safe source-fluid pickup and removal semantics; do not change Horizon API.

## API-constrained alpha decision — Electric Pump (2026-11-15)

Arcforge's Electric Pump picks up a source-fluid block and removes it unless adjacent water sources make it infinite. The pinned Horizon mod `Host` exposes block reads, property changes, and a machine-destroy helper, but no source-fluid pickup operation with `BucketPickup` semantics; block destruction is not equivalent. Keep exact source-block pumping deferred rather than adding a Horizon API hook or creating fluid from ordinary blocks. Steam machines can use water supplied by buckets and existing fluid transfer while this boundary remains.

## Acceptance checks

Each industrial chain must be obtainable in survival, bounded by input/output capacity, persistent after save/reload, and covered by a scenario that checks its outputs and byproducts. Nuclear behavior additionally needs controlled shutdown and explicit safety tests.
