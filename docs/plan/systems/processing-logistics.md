# System plan: processing and logistics

## Target

Arcforge's early processors are the Arc Crusher, Sifter, Induction Furnace, Metal Press, Fiberizer, and Infuser. Later progression adds arrays, tiered item/fluid/energy/heat/pressurized conduits, storage, machine configuration, upgrades, and automation.

## Build order

1. Finish single-block processors with upstream recipe data, slot rules, costs, timing, output limits, and save behavior.
2. Make ingredients and outputs obtainable; register all referenced items/blocks and tags.
3. Add player-facing machine screens and discoverability using the pinned mod hooks.
4. Add side configuration, rotation, and automation. Test vanilla hoppers and Horizon handlers where supported.
5. Add storage items and tiered conduits before arrays that depend on them.
6. Add multiblock arrays after single machines, recipes, and transfers are stable.

## Current state

Recipe data exists for crushing (83), sifting (4), pressing (25), carbonizing (4), infusing (10), fiberizing (3), and Arcforge smelting (12). Runtime hooks cover these processors and a formed three-lane Crushing Array whose 3×3×3 casing cube is checked on formation and controller ticks. Its lane priority rotates each tick so limited FE cannot permanently starve later lanes. Four FE Conduit tiers relay energy with Wrench-configured face modes; four Item Conduit tiers move items between configured faces through the connected run at source tier rates; four Fluid Conduit tiers use Horizon sided tank capabilities and transactional fluid moves at the slowest tier rate; four Energy Cell tiers preserve charge on ordinary mining and placement; four Fluid Tank tiers exchange full vanilla buckets and partial Arcforge bucket/canister contents. Item and fluid moves use Horizon's transaction helpers directly, so the first pass omits packet animation and conduit-held buffers. Item faces require configuration; fluid Auto mode detects sided fill/drain capabilities. Filters remain absent. Canister capacity and tier transfer rates match upstream, with server-side hand interaction and four survival recipes; source automated tank slots are not represented. Tier crafting cannot copy fluid contents because the pinned crafting-result hook receives no ingredients. The array has no speed/energy upgrades or dedicated ports; only the center exposes inventory access. Broader automation, content-preserving tank/cell upgrades, and custom screens remain unfinished. No item/fluid conduit network or tank interaction has been behavior-tested in a world yet. Pressurized Conduit source edits and resource files have been started, but are not compiled, packaged, or verified. The Horizon API exposes no gas classification query, so current source work uses an explicit Arcforge gas list and cannot classify arbitrary third-party gases.

## Acceptance checks

- Recipe matching and result counts match the pinned upstream files.
- Inputs, energy, heat, and fluids are consumed only when the operation can complete.
- Output backpressure never overwrites unlike stacks or loses byproducts.
- Hoppers/handlers cannot insert into output-only or tool-only slots.
- Machines and arrays retain all slots, progress, and resources across save/reload.
