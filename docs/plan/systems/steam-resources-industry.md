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

**Steam Boiler Array alpha slice implemented in source (2026-11-15):** `api/src/steam_boiler.rs` adds a fixed 3×3×3 shell, controller and casing blocks, a two-slot water-bucket interface, 48,000 mB water and steam tanks, and heat input from supported Arcforge heat producers. At 100°C it consumes 8 HU/mB and outputs steam; at 500°C it outputs high-pressure steam at 12 HU/mB; at 900°C it outputs superheated steam at 16 HU/mB. Wrench use reports formation, temperature, water, and output gas/amount.

This is a first native slice, not Steam Power completion or source parity. Arcforge supports 3×3 through 7×7 shells with heights 3–16, selectable operating modes, source-specific capacity and efficiency behavior, and overpressure venting. The alpha currently supports only one 3×3×3 shell, a separate controller block added as Horizon scaffolding, one default orientation, and no vent or safety behavior. Its 48,000 mB per tank and 300,000 HU store are fixed implementation limits. Do not describe it as fully playable until a matching client has been installed and the shell, bucket input, heat transfer, steam output, and save/reload have been tested in-game.

Next in this system: obtain a matching-client in-game check for the boiler slice; then extend the source shell sizes, pressure controls and safety behavior where Horizon hooks permit, followed by the upstream turbine and closed-loop chain. The Electric Pump source-block pickup remains deferred because the current Horizon Host API lacks safe source-fluid pickup and removal semantics; do not change Horizon API.

## API-constrained alpha decision — Electric Pump (2026-11-15)

Arcforge's Electric Pump picks up a source-fluid block and removes it unless adjacent water sources make it infinite. The pinned Horizon mod `Host` exposes block reads, property changes, and a machine-destroy helper, but no source-fluid pickup operation with `BucketPickup` semantics; block destruction is not equivalent. Keep exact source-block pumping deferred rather than adding a Horizon API hook or creating fluid from ordinary blocks. Steam machines can use water supplied by buckets and existing fluid transfer while this boundary remains.

## Acceptance checks

Each industrial chain must be obtainable in survival, bounded by input/output capacity, persistent after save/reload, and covered by a scenario that checks its outputs and byproducts. Nuclear behavior additionally needs controlled shutdown and explicit safety tests.
