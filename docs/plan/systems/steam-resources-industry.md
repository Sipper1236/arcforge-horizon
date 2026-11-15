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

These systems are not implemented in the current port. The existing 8,000 mB creosote tanks support limited Carbonizer/Infuser recipes; they do not constitute the broader steam, petroleum, refinery, or chemistry systems.

## API-constrained alpha decision — Electric Pump (2026-11-15)

Arcforge's Electric Pump picks up a source-fluid block and removes it unless adjacent water sources make it infinite. The pinned Horizon mod `Host` exposes block reads, property changes, and a machine-destroy helper, but no source-fluid pickup operation with `BucketPickup` semantics; block destruction is not equivalent. Keep exact source-block pumping deferred rather than adding a Horizon API hook or creating fluid from ordinary blocks. Steam machines can use water supplied by buckets and existing fluid transfer while this boundary remains.

## Acceptance checks

Each industrial chain must be obtainable in survival, bounded by input/output capacity, persistent after save/reload, and covered by a scenario that checks its outputs and byproducts. Nuclear behavior additionally needs controlled shutdown and explicit safety tests.
