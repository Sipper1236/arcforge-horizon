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

The pinned Horizon revision already supports server-opened menus plus client screen render/input hooks, so the missing machine screens are unported functionality rather than a known API blocker. Do not change the modding API; build the first screen against these existing hooks and reuse that pattern for the other machines.

Portable FE Batteries are now registered as four single-stack Horizon item-energy items with source capacities/rates and four survival recipes. A mod-side hand interaction transfers energy between charged/empty portable batteries and Energy Cells at the item/cell rate limits; it is an alpha replacement for the source GUI slots and requires one item per stack. Energy Cell block items also have a one-item stack limit so stored FE cannot duplicate across placed blocks. Five Energy Cell charge-state variants are wired to saved charge. Commit `461fb88` passes the GitHub package job; transfer behavior, model updates, and persistence still need matching-client verification. Charge-preserving tier upgrades remain unavailable because the crafting hook cannot read ingredients. Lithium Cells and Battery Arrays remain unimplemented.

Pressurized Cylinder runtime evidence: the corrected Linux client drained one 32,000 mB Wrought Gas Cartridge of oxygen into a Wrought Cylinder in a disposable world; the cartridge emptied and the gauge reached level 2. The check used Horizon's server-side container data path because right-clicking the Cylinder did not open a player menu. Direct charged/empty cartridge hand interaction and 2,000 mB gas persistence through save/reload passed a disposable-world check. Commit `1e5d989` enlarges the five-stage gauge, assigns the cylinder model to the cutout layer, and adds exact Wrench inspection text; a matching-client visual check passed, and the user confirmed it in-game. Sided automation, other tiers/gases, and ordinary break/place retention still need checks; the player menu remains unavailable.

Recipe data exists for crushing (83), sifting (4), pressing (25), carbonizing (4), infusing (10), fiberizing (3), and Arcforge smelting (12). Runtime hooks cover these processors and a formed three-lane Crushing Array whose 3×3×3 casing cube is checked on formation and controller ticks. Its lane priority rotates each tick so limited FE cannot permanently starve later lanes. The Arc Crusher now has a dedicated native screen in source showing its three slots, FE/capacity, progress, and inventory; matching-client build/runtime verification is pending. The Wrench reports FE and recipe progress for the Arc Crusher, Induction Furnace, Metal Press, Sifter, and each Crushing Array lane. The array accepts source Speed/Energy Upgrades and exposes fixed north-facing source-default item/energy ports (top input, bottom output, south/back FE input); Wrench inspection on its center reports FE and progress for each lane. Per-casing Wrench configuration is unavailable through the pinned block-wide hooks. Four FE Conduit tiers relay energy with Wrench-configured face modes; four Item Conduit tiers move items between configured faces through the connected run at source tier rates; four Fluid Conduit tiers use Horizon sided tank capabilities and transactional fluid moves at the slowest tier rate; four Energy Cell tiers preserve charge on ordinary mining and placement, show five charge levels in source models, and support item hand-transfer; four Fluid Tank tiers exchange full vanilla buckets and partial Arcforge bucket/canister contents. Item and fluid moves use Horizon's transaction helpers directly, so the first pass omits packet animation and conduit-held buffers. Item faces require configuration; fluid Auto mode detects sided fill/drain capabilities. Filters remain absent. Canister capacity and tier transfer rates match upstream, with server-side hand interaction and four survival recipes; source automated tank slots are not represented. Tier crafting cannot copy fluid contents because the pinned crafting-result hook receives no ingredients. The array's in-world port automation and upgrade behavior, broader automation, content-preserving tank/cell upgrades, and remaining processor screens remain unfinished. No item/fluid conduit network or tank interaction has been behavior-tested in a world yet. Four Pressurized Conduit tiers now compile and their resources are packaged in the checked `.hmod`; their 22 gas IDs exactly match the pinned Arcforge data map. The Horizon API exposes no arbitrary gas-classification query, so unknown third-party gases cannot be identified. Routing remains unverified in-world.

## Acceptance checks

- Recipe matching and result counts match the pinned upstream files.
- Inputs, energy, heat, and fluids are consumed only when the operation can complete.
- Output backpressure never overwrites unlike stacks or loses byproducts.
- Hoppers/handlers cannot insert into output-only or tool-only slots.
- Machines and arrays retain all slots, progress, and resources across save/reload.
