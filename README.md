# Arcforge Horizon

A Rust-native port of Arcforge for Horizon. Arcforge is an MIT-licensed Minecraft technology mod; this project ports its gameplay and data to Horizon while tracking the provenance of upstream and third-party material. The current implementation is an early native gameplay prototype, not a complete port.

- Arcforge feature reference: <https://modrinth.com/mod/PtKWXzEG>
- Arcforge source and MIT license: <https://github.com/zagdrath/arcforge>
- Horizon native mods guide: <https://github.com/horizon-rust/horizon/blob/main/docs/native-mods.md>
- Horizon content mod guide: <https://github.com/horizon-rust/horizon/blob/main/docs/mods.md>

The detailed build plan and as-built inventory are in [`docs/README.md`](docs/README.md). The alpha's player progression will follow Arcforge's original order; implementation tasks may work on prerequisites early. The plan uses the pinned Horizon modding API without changing it.

GitHub Actions builds and uploads the checked content and native source packages, then builds a Linux x86_64 Horizon client on pushes and manual runs. The client artifact includes Horizon's native build manifest as well as the compiled executable; both are needed to activate Arcforge's native registrations after import. The CI job fetches Mojang's public Minecraft 26.3 build inputs and verifies their official hashes without publishing the client jar. A disposable runtime smoke test confirmed custom items register; machine progression still needs broader gameplay verification. See [`docs/ci-build.md`](docs/ci-build.md).

For a resumable project snapshot and immediate next actions, see [`docs/handoff.md`](docs/handoff.md) and [`docs/goal.md`](docs/goal.md).

The complete feature inventory is version-sensitive. The source audit was made against Arcforge commit `8843cb02cf9132f278f6a8e804b6d78837d3cbcb` (2026-10-28), with its published feature documentation checked on 2026-11-07. Treat that pinned snapshot as the parity target; update the inventory and implementation when the target changes. The matching MIT notice is in `LICENSE-Arcforge`.

## Porting principles

1. **License-aware port.** Arcforge's repository is MIT-licensed, so source code and other files covered by that license may be copied, adapted, and translated when that is the clearest route to parity. Preserve the copyright and MIT permission notice in this project and identify substantial ported/adapted files. Check each texture, model, sound, font, dependency, and bundled asset for its own provenance and license before reuse; the repository license alone does not establish ownership of third-party material. Keep Arcforge attribution and record any material exclusions.
2. **Horizon-native behavior.** Use `.hmod` content for data, recipes, tags, models, loot and worldgen where supported; use `.hnmod` Rust hooks for machine simulation and behavior. Keep client rendering optional where possible and keep authoritative inventory, energy, fluids, worldgen and machine state server-side.
3. **Composable systems.** Define shared stack, recipe, fluid, energy, heat, port, side-mode, upgrade, machine and multiblock abstractions before adding dozens of machines. All transfers must be bounded, deterministic, save-safe, and compatible with hoppers and other Horizon-native mods where Horizon exposes the seam.
4. **Incremental playable releases.** Each milestone should be independently usable, documented, and save-compatible. Build against the pinned Horizon modding API; simplify or defer features that the existing API cannot support reliably.
5. **Explicit compatibility.** Pin the Horizon source revision for the native bundle. Record required native-hook API versions, test LAN/client-server parity, and plan a migration for every saved-data schema change.

## Current state

The native crate in `api/` registers the Metal Press, Sifter, Arc Crusher, a formed three-lane Arc Crushing Array with 3×3×3 casing, Carbonizer, Induction Furnace, Infuser, Resin Tap, Firebox, Geothermal Plant, Fuel Burner, all four Fluid Tank tiers, Thermoelectric Plant, four Energy Conduit tiers, four Item Conduit tiers, four Fluid Conduit tiers, four thermal conduit tiers, four Energy Cell tiers, four Heat Cell tiers, four portable FE Battery items, a Wrench that rotates supported single-block machines and configures conduit faces, Fiberizer, Vulcanizer, Grain Dryer, the Arcforge Furnace port/bricks/walls, a separate Steelworks compatibility prototype, Combustion Plant, coke blocks, treated wood blocks, custom Creosote/Latex buckets, four tiered portable canisters, the tungsten heating coil, seven Steel tools, four Steel armor pieces, and processing materials. Its machine state is saved in Horizon block entities; FE and fluid handlers use Horizon's native capabilities.

Implemented machine slices:

- **Metal Press:** loads all 25 upstream `arcforge:pressing` recipes and checks the die, ingredient, input count, result, and duration. Reusable plate, gear, rod, and pellet dies are supported. It uses 20 FE/t; the die slot is player-operated, excluded from hopper insertion, and preserved in the saved inventory.
- **Induction Furnace:** loads all 32 Arcforge ore/raw/dust smelting recipes from a mod-side catalog and retains common vanilla iron, copper, and gold ores/raw metals, ancient debris, sand, cobblestone, and clay balls as a fallback. It uses the source 20 FE/t, a 20,000 FE buffer, and half the recipe cooking time (100 ticks for standard recipes). Its source-shaped crafting path and tungsten heating coil recipe are present, with copper rods tagged for the coil. The pinned API does not expose the vanilla recipe manager to hooks, so unrelated vanilla and datapack furnace recipes are not queried; experience banking is not yet ported.
- **Sifter:** loads all four upstream `arcforge:sifting` recipes, rolls outputs independently, requires a mesh, and uses six output slots with output backpressure. The four mesh tiers retain upstream durability, speed, chance, and Unbreaking behavior. Its 20,000 FE buffer draws 20 FE/t before the mesh speed multiplier.
- **Arc Crusher and Arc Crushing Array:** load all 83 upstream `arcforge:crushing` recipes through Horizon's recipe helpers. The single Arc Crusher processes recipe durations, bonus chances, and ore-yield flags while respecting output capacity. The array forms a 3×3×3 cube from its casing; the center casing controls three independent lanes with separate result and bonus slots, doubled primary yield for `ore` recipes, and a shared 100,000 FE controller buffer drawing 16 FE/t per active lane. Lane priority rotates each tick so limited FE input cannot starve later lanes. Placement forms complete cubes, recovery ticks cover prebuilt structures, and breaking a casing unforms the remaining parts. The casing has its source-shaped recipe and block loot. Item access is center-only, facing is north-fixed, and array speed/energy upgrades and dedicated ports remain unfinished.
- **Carbonizer:** loads all four upstream `arcforge:carbonizing` recipes: carbon dust to coke (300 ticks); burnable logs to charcoal with 100 mB creosote (400 ticks); bio coal to coke with 250 mB creosote (600 ticks); and bio coal blocks to coke blocks with 2,250 mB creosote (5,400 ticks). Its saved 8,000 mB creosote tank is exposed through Horizon's fluid capability. Processing stops when item or fluid outputs cannot accept a batch.
- **Infuser:** has its source-shaped crafting recipe and loads all 10 upstream `arcforge:infusing` recipes and five input tags, and registers all five treated wood/plank outputs. It has a 20,000 FE buffer, an 8,000 mB creosote tank, uses 20 FE/t, and exchanges 1,000 mB custom creosote buckets with the tank. Fluid and resin alternatives retain their upstream 50/200 mB or 1/4 resin costs and 60/120-tick durations. Creosote can enter through Horizon fluid handlers or custom buckets. Treated logs, wood, and planks form the source input path; the complete building set is described below.
- **Treated Wood building set:** all 13 upstream logs, wood, planks, stairs, slab, fence, gate, door, trapdoor, pressure plate, and button blocks are registered with the upstream crafting recipes, loot, and treated-log tags. Crimson-family vanilla templates preserve the set’s nonflammable behavior; registered properties restore source hardness, blast resistance, brown map color, and oak wood sounds, while the button retains its source default sound/color. Infuser recipes provide treated logs, stripped forms, wood, and planks. The pinned Arcforge source defines no treated sign variants; farm/compost progression remains unimplemented.
- **Steel equipment:** the five basic tools, Hammer, Excavator and four armor pieces use Arcforge durability, mining/attack/defense/toughness values, steel repair tag, and equipment texture. All eleven upstream crafting recipes are present. The Hammer/Excavator 3×3 effect is deferred because existing hooks cannot inspect arbitrary block hardness and block-entity presence or charge extra tool durability for adjacent blocks; other advanced gear remains unimplemented.
- **Resin Tap:** follows the upstream 200-tick drip, natural-leaf and short trunk-gap checks, 3×3 leaf search, 1,000 mB Latex tank, 25 mB jungle-log output, 50% spruce and 15% other-log Pine Resin chances, and 16-resin cap. Its item output is available through the block inventory and a hopper below; empty-hand block clicks collect Pine Resin into the player inventory using the pinned API. Any amount that does not fit is dropped as a world item; if Horizon cannot spawn the drop, that amount is restored to the tap. Empty vanilla buckets fill into registered 1,000 mB Latex Buckets, which can be emptied back into the tap. The tank also drains through Horizon fluid handlers. Two Pine Resin and one kelp craft a slime ball with the source recipe. Tap configuration, custom tooltip, and drip particles remain absent.
- **Ore and mineral worldgen:** Silver, Nickel, Fluorite, Bismuth, Tungsten/Wolframite, Arcite, Spodumene, Uraninite, Galena, Halite, Nether Sulfur, End Iolite, Spodumene Pegmatite, and Heavy Mineral Sand now have configured features and biome placement based on Arcforge's pinned defaults. Drops, compatibility tags, mining-tool tags, and furnace recipes are included where applicable. Arcite ore/deepslate ore, raw Arcite blocks, and Arcite storage blocks emit their source light levels 5, 9, and 12. Horizon lacks Arcforge's custom serializers and per-world config conditions, so worldgen uses fixed defaults; Halite's custom flat bed is approximated by a vanilla ore cluster.
- **Ore reduction:** 32 vanilla furnace recipes and 31 blasting recipes convert registered Arcforge dusts, raw materials, and ores into crystals, ingots, glass, or vanilla products. Common dust tags supply their source recipe inputs. The source Invar Dust crafting recipe is now present, with `c:dusts/iron` mapped to Arcforge Iron Dust; two Iron Dust and one Nickel Dust produce three Invar Dust, which can then be crushed or smelted. Local copper rod, tungsten plate, redstone dust, sulfur dust, copper gear, iron nugget, and redstone block tags close current crafting references. The flour-to-bread and yellowcake smelting recipes remain unavailable because their item chains are not registered.
- **Combustion Plant:** burns coal, charcoal, coal blocks, Bio Coal, and Bio Coal blocks at twice vanilla furnace speed, producing 40 FE/t into a saved 50,000 FE buffer. It pauses fuel consumption while the buffer is full, pushes up to 200 FE/t into adjacent Arcforge FE consumers, and exposes the same bounded extraction through Horizon's energy capability. A Wrench cycles adjacent-transfer output faces through Auto, Input, Output, and Disabled; Horizon capability extraction remains all-side because its hook has no block position. Charcoal has the upstream 50% Wood Ash chance; ash is saved in an output slot and can be extracted below. The upstream-shaped crafting recipe and block loot are present. Configurable output/burn multipliers, flue gas, upgrades, and the custom burner screen are not ported.
- **Firebox and Thermoelectric Plant:** the Firebox burns the upstream combustion-fuel items at twice furnace speed and produces 80 HU/t into a 40,000 HU buffer capped at 1,100°C. Its 50% Wood Ash byproduct is saved in an output slot and can be extracted below. A burning Firebox transfers heat at up to 100 HU/t to adjacent consumers at its 1,100°C source temperature. The Thermoelectric Plant stores 20,000 HU (maximum 1,400°C), converts up to 102 HU/t into FE at the source's 0-to-115% temperature curve, and has a 50,000 FE buffer with a 200 FE/t output cap. The conversion pauses when FE is full. A Wrench cycles the Thermoelectric Plant's adjacent-transfer output faces through Auto, Input, Output, and Disabled; capability extraction remains all-side. The thermocouple and plant crafting recipes, plant loot, and block registration are included. Firebox face configuration, heat conduits, heat/energy upgrades, flue gas, and custom screens are absent.
- **Geothermal Plant:** an 8,000 mB lava tank accepts lava through its native input capability; lava buckets can be inserted through the two-slot inventory or used on the block. It drains 100 mB batches at 1 mB/t and makes 80 HU/mB into a 60,000 HU buffer capped at 1,400°C. Touching lava source blocks add 40 HU/t each and magma blocks add 16 HU/t each, rescanned every 20 ticks. Stored heat moves to supported adjacent consumers at up to 100 HU per call, so the plant can feed the Thermoelectric Plant without burning lava. The source-shaped recipe and loot are present. Upgrades, redstone and side modes, and a dedicated screen remain unfinished.
- **Solar Thermal Array:** its formed 2×2×4 collector tower generates buffered sun-driven heat. Wrench-configured faces can feed adjacent heat consumers or the four thermal conduit tiers, and enabled faces show a steel collar and heat-port plate on formed parts. The visual is a standard blockstate overlay; biome/snow fidelity, controls and screens remain incomplete.
- **Fuel Burner and Fluid Tanks:** all four Fluid Tank tiers store 16,000 / 64,000 / 256,000 / 1,024,000 mB through Horizon's fluid capability, with source-shaped crafting recipes and block loot. Full 1,000 mB water and lava buckets retain vanilla all-or-nothing exchange. Arcforge Creosote/Latex buckets and all four portable canister tiers support partial hand transfer in either direction. Canister capacities/rates are 16,000/1,000, 64,000/4,000, 256,000/16,000, and 1,024,000/64,000 mB. Transfers reject mismatched fluids, preserve unrelated custom data, and roll back tank changes if inventory exchange fails. Canisters transfer by right-clicking the tank rather than through Arcforge's automated tank slots. Horizon's generic item-fluid helpers only handle vanilla water/lava, so custom fluid exchange remains in this mod's server hook. Tank and canister tier upgrades use shaped recipes and do not preserve contents: the pinned crafting-result hook receives no ingredient stacks, so safe component copying is unavailable. The pinned fluid hook also lacks a per-tick tank throughput field. The Fuel Burner has an 8,000 mB input tank, a two-slot container for custom Creosote Buckets, a 40,000 HU buffer capped at 1,400°C, and transfers heat at up to 100 HU/t to adjacent supported consumers. It burns Creosote at the pinned source values of 120 HU/mB, 0.5 mB/t and 850°C, retaining fractional fuel between ticks. The pinned API does not expose Arcforge's fuel data maps; other fuels await their fluid registrations and recipe chains. Oxygen/flue-gas systems, side modes, output controls, upgrades, and custom screens remain absent.
- **Energy Conduits and Wrench:** Wrought, Tempered, Hardened, and Arcforged tiers provide network rates of 256, 1,024, 8,192, and 262,144 FE/t for mod-side transfers. Bidirectionally connected members share the lowest tier's rate and a member-scaled pool; one-way Output-to-Input links relay between separate pools. Producers feed networks through configured output faces, and pools distribute FE to supported Arcforge machines while respecting insertion limits and capacity. The imported active blockstate variants now glow across a pool while it stores FE and for ten ticks after the last transfer. Horizon's native energy capability remains per block and all-sided: external calls bypass conduit face modes and the shared rate. Their changes are reconciled before pool writes and may temporarily exceed normal capacity. Charge saved in the old 50,000 FE per-block buffers is retained; excess over normal pool capacity drains, then the pool capacity returns to its member-rate limit. A crafted Wrench cycles each conduit face through Auto, Input, Output, and Disabled; adjacent energy conduits can be disconnected and reconnected. On the Combustion Plant and Thermoelectric Plant it cycles per-face energy output settings; on supported single-block Arcforge machines it rotates horizontal facing clockwise. Multiblock controllers and the Resin Tap are excluded to preserve formed structures and the Tap's log attachment. The Wrought recipe and all loot tables are upstream-identical; higher tier recipes preserve source patterns and counts in vanilla crafting recipes but omit unsupported Arcforge component copying. Wrench port/dismantle modes, sneaking reverse-cycle, animated flow, colors/facades, and in-world network verification remain outstanding.
- **Item Conduits:** all four source tiers are registered and craftable. Connected runs move items directly between Wrench-configured Output and Input faces through Horizon's transactional `move_items` helper, with source limits of 8/16/32/64 items per operation and 20/10/5/2 ticks per operation, using the slowest tier in the run. A rejected transfer stays in its source inventory. Connected runs are cached and rebuilt after conduit placement, breaking, or Wrench changes. This first pass has no animated packets, conduit-held item buffer, filters, or automatic source/sink detection; faces must be configured. Tier upgrades use shaped crafting and do not preserve Wrench face settings. The network has not been exercised in-world.
- **Fluid Conduits:** all four source tiers are registered and craftable. Connected runs move one fluid type at a time through Horizon's transactional `move_fluids` helper, capped at 200/800/3,200/12,800 mB/t by the slowest tier. Wrench faces default to Auto, which detects sided fill/drain capability through Horizon's fluid-tank API; Input/Output/Disabled can be set manually. Runs are cached and invalidated on conduit topology and Wrench changes. This pass has no conduit-held fluid buffer, filters, or animated fill rendering. The network has not been exercised in-world. **Pressurized Conduits:** four tiers route the 22 known Arcforge gases separately from liquid networks at 400/1,600/6,400/25,600 mB/t. **Pressurized Cylinders and Gas Cartridges:** cylinders store 64,000/256,000/1,024,000/4,096,000 mB and accept only the pinned Arcforge gas IDs. Their two inventory slots transfer Gas Cartridges at 2,000/8,000/32,000/128,000 mB per tick; the five-level cylinder gauge follows stored fill. A charged cartridge can also be used directly on a Cylinder to insert gas, and an empty cartridge can extract it. Gas is preserved through ordinary block drops and placement. A disposable-world runtime check confirmed one Wrought cartridge transferring 32,000 mB oxygen into a Wrought Cylinder and the gauge reaching level 2. Source tier recipes are present, but tier-upgrade recipes cannot copy gas components in Horizon's current crafting hook. A disposable-world runtime check verified direct insertion/extraction and Cylinder gas persistence after save/reload. Commit `1e5d989` enlarges the five-stage gauge, uses the cutout render layer to keep adjacent blocks opaque, and adds a Wrench action-bar readout with exact gas amount/capacity; the visibility changes passed a matching-client check, and the user confirmed the result in-game. Cylinder menu access, side controls, comparator, gas tint, and dedicated UI remain incomplete or unverified. Pressurized Conduit routing, other Cylinder tiers/gases, and break/place retention remain unverified in-world. The pinned API cannot classify unknown third-party gases.
- **Thermal Conduits:** Wrought, Tempered, Hardened and Arcforged tiers move heat through mod-side saved pools. Bidirectionally connected members share the lowest tier's HU/t rate and member-scaled storage; one-way Output-to-Input links transfer between separate pools with both pool budgets enforced. The Wrench configures Auto/Input/Output/Disabled faces, and supported processors and the Thermoelectric Plant can receive heat. Horizon exposes no heat capability hook, so mod-side ticks bridge sources, pipes and supported sinks. Color/facade behavior and in-world verification remain unfinished.
- **Heat Cells:** Wrought, Tempered, Hardened, and Arcforged tiers store 50,000 / 200,000 / 800,000 / 3,200,000 HU and transfer up to 50 / 200 / 800 / 3,200 HU/t. They automatically accept and emit heat on all six faces, leak at their tier rate, display five fill levels, and preserve charge through ordinary block mining and placement. Four recipes and loot tables are included. This first slice has not been checked in-world; screens, capsule slots, insulation upgrades, redstone/side controls, and charge-preserving tier upgrades remain unimplemented.
- **Steel and alloy progression:** the Arcforge Furnace supports the source Steel, Wrought, Tempered, Hardened, Arcforged, and Graphite recipes with their source material counts, heat gates, times, coke, and slag behavior. The upstream Arcite/Tungsten Composite crafting recipe is included. Arcforged Alloy remains unobtainable in survival because its Carbon Fiber ingredient depends on Pitch from the unimplemented Refinery chain.
- **Energy Cells:** Wrought, Tempered, Hardened, and Arcforged blocks provide 500,000 / 4,000,000 / 32,000,000 / 256,000,000 FE of saved Horizon buffer capacity at source rates of 512 / 2,048 / 16,384 / 262,144 FE/t. Their block-item forms declare Horizon's native per-item energy buffers with matching capacities and transfer rates. Normal player mining carries stored FE into the dropped cell item, and placing that item restores the FE to its block entity. A per-tick charge snapshot also feeds loot callbacks for removal paths that do not expose the block entity; explosion drops still need in-world verification. Tier-upgrade recipes do not preserve charge. Input and output are enabled on every face because the pinned energy hook cannot vary faces by block orientation. Charged Arcforge Energy Cell/portable Battery items can be used on an Energy Cell to insert FE; empty items extract FE, bounded by both tier rates and capacity. The transfer synchronizes the saved item component and block entity and reports both levels in the action bar. Use one item per stack for hand transfer. The inventory GUI, automated item charging slots, redstone controls, comparator, and charge-state rendering remain unimplemented.
- **Fiberizer, Vulcanizer, and Grain Dryer:** the Fiberizer has its source-shaped crafting path and loads all three upstream `arcforge:fiberizing` recipes, uses a 20,000 FE buffer, consumes recipe FE/HU costs per tick, and observes the 800°C/1,000°C minimum temperatures. It pauses on insufficient power or heat and full output while retaining progress. The Vulcanizer follows the upstream 8,000 HU capacity, 400°C maximum, 140°C minimum, and batch values. The Grain Dryer processes all three upstream item recipes and the Latex fluid recipe using 4 HU/t, the 4,000 HU buffer, and the 60°C threshold. Heat currently moves by direct adjacency from the Firebox; thermal ports/conduits, oxygen and oxy-fuel, upgrades, and full machine screens are not implemented.
- **Arcforge Furnace and Steelworks compatibility prototype:** the Arcforge Furnace port checks the source 3×3×6 brick/wall/port pattern in all four horizontal facings, then runs the 12 upstream `arcforge:arcforge_smelting` recipes with four ingredient slots, coke used as both fuel and reagent, 800-tick coke burns, source temperature progression, recipe gates, outputs, and slag byproducts. A coke block provides nine loose coke. Formation is rechecked every 20 ticks because the pinned hooks do not notify the mod when a part changes. The standalone Steelworks block remains a separate compatibility prototype. Furnace bricks, walls, port, and coke block are registered with block drops; Furnace bricks, walls, and port have the upstream crafting recipes, and coke blocks can be crafted from or unpacked into coke.

Twenty-five early crafting recipes make the Wrench, Carbonizer, prototype Steelworks, Combustion Plant, Metal Press, reusable dies, Sifter, four mesh tiers, steel storage block, Arc Crusher, furnace bricks, Firebox, Wrought Fluid Tank, Fuel Burner, Resin Tap, Grain Dryer, and Vulcanizer obtainable in survival; two Pine Resin and kelp also craft a slime ball. Four more source-shaped recipes craft the tungsten heating coil, Induction Furnace, Infuser, and Fiberizer; copper rods now have their common item tag. Nine further recipes craft the treated wood building set, whose 13 blocks have upstream loot and treated-log tags. Two additional recipes craft the Arcforge Furnace walls and port; two more pack coke into blocks or unpack it back into nine items; two craft the thermocouple and Thermoelectric Plant; one crafts the Geothermal Plant; four craft/upgrade the Energy Conduit tiers, four craft thermal conduit tiers, four craft/upgrade Energy Cells, four craft Heat Cells, three upgrade Fluid Tank tiers, and four craft/upgrade each of the Item Conduit and Fluid Conduit tiers. Four more recipes craft/upgrade portable canisters; tank and canister tier recipes do not preserve fluid contents because the crafting hook does not expose ingredient stacks. The Arcite/Tungsten Composite is craftable with the upstream shapeless recipe. Ten raw material types, including Rock Salt, and eleven refined materials plus Rubber can be packed into and unpacked from 9-item storage blocks using upstream recipes. Rubber and Rubber Gasket are registered. Arcite ore and deepslate ore emit level 5 light; its amethyst-template storage blocks emit level 9 (Raw Arcite) and 12 (Arcite). Common item tags cover steel forms and the ingots, dusts, and plates needed by the current furnace and press recipes. Shared integer `HeatStore` and `EnergyStore` types provide bounded storage and rate-limited transfer; active Firebox and Geothermal Plant heat now flows at its source temperature. The Combustion Plant, Thermoelectric Plant, energy conduits, Energy Cells, and Heat Cells expose saved resource buffers.

The `content/` directory contains the `.hmod` resource pack: early progression recipes, material tags, labels, machine recipe data, ore/mineral worldgen, and the complete 6,039-file upstream `assets/arcforge/` tree from the pinned MIT-licensed snapshot. Arc Crusher, Carbonizer, Combustion Plant, Firebox, Fiberizer, Induction Furnace, Infuser, Metal Press, Resin Tap, Sifter, furnace bricks, and treated wood use upstream models and textures. Other registered content still uses vanilla templates, and most upstream assets await matching registrations and behavior. Horizon does not yet support Arcforge-specific item components for conduit colors, jetpack plating, wrench modes, and filled settings cards, so 26 upstream item definitions use static fallback art.

This remains an early prototype, not a full Arcforge port. Custom screens, full survival progression and material recipes, most logistics features, most formed multiblocks, save migration, and LAN/dedicated-server parity remain incomplete. Firebox heat and energy capability extraction still use simplified all-side behavior; Combustion Plant and Thermoelectric Plant now have per-face energy output settings. FE and HU conduit pools and their active glow are implemented mod-side but remain unverified in-world. Upgrades and flue-gas output remain absent.

## Full feature inventory

The target is functional parity with Arcforge's survival gameplay, not a byte-for-byte recreation of its implementation or presentation.

### Progression and materials

- Arcforge creative tabs, machine/item/block catalog, tags, common material tags and configurable recipes.
- Steel, wrought/tempered/hardened/arcforged alloys, plates, gears, rods, dies, machine frames and tiered components.
- Ten Overworld ores plus End Iolite: Silver, Nickel, Tungsten, Fluorite, Bismuth, Arcite, Halite, Spodumene, Uraninite, Galena and Iolite; raw forms, storage blocks, ore generation, mining requirements, Fortune/Silk Touch behavior and config toggles.
- Steel tools, armor, hammer, excavator, Foundry Suit, Long Fall Boots and armored variants.
- Treated wood building set, Arcforge furnace bricks, alloy/metal storage blocks, pressure glass, asphalt and other industrial blocks.

### Power, heat and steel

- Power and heat generation: Combustion Plant, Firebox, Fuel Burner, Geothermal Plant, Thermoelectric Plant, Firebox Array and Solar Thermal Array.
- Heat buffers and temperatures, hot-to-cold flow, heat loss, port faces, thermodynamic conduits, Heat Cells, oxygen-assisted oxy-fuel, fuel data maps and configuration.
- Carbonizer single/multiblock coke production; Arcforge Furnace multiblock with coke consumption, 1,200°C+ recipe gates, multiple additives and slag byproducts.
- Alloy and steel progression, graphite, carbon dust, coke and steel recipes.

### Processing and logistics

- Arc Crusher and Crushing Array; Sifter and mesh tiers; Induction Furnace/Array; Metal Press/Pressing Array; Fiberizer; Infuser.
- FE, heat, fluid and pressurized conduits in four tiers; covers/facades, filters, upgrades, visual flow and active-state indicators.
- Partial transfer with registered Arcforge Creosote/Latex buckets and all four portable canister tiers is implemented through hand interaction. Automated fluid-canister slots/general item-fluid transfer and content-preserving tank upgrades remain. Pressurized Cylinders and four portable Gas Cartridge tiers are implemented in source with saved gas, tiered transfer, and drop/place retention; Wrought cartridge insertion/extraction and gas persistence after save/reload passed a disposable-world check. Commit `1e5d989` enlarges the gauge, uses the cutout render layer, and adds a Wrench amount readout; the matching-client visual check passed, and the user confirmed it in-game. Player menu access, side controls, comparator/tint, and gas-preserving tier upgrades remain. Heat Cells now provide four tiers of automatic heat storage and transfer; their screens, capsules, insulation upgrades, controls, and charge-preserving tier upgrades remain. Lithium Cells, Battery Arrays, and Energy Cell GUIs, charge-preserving tier upgrades, comparators, and redstone control are also in scope.
- Four portable FE Battery items now use Horizon's item-energy capability with Arcforge's source capacities and transfer rates. They have the four source crafting patterns (tier upgrades are ordinary shaped recipes and lose charge because the crafting hook cannot copy item data). Charged or empty batteries can be used directly on an Energy Cell for bounded hand transfer; this existing-hook path is in source and awaits a matching-client build and runtime check. Lithium Cells and Battery Arrays remain in scope.
- Wrench port/dismantle modes, per-face port modes, general machine-side configuration, multiblock diagnostics, item/fluid/energy transfer, machine links and wired links. Basic clockwise facing rotation for supported single-block machines and energy output face settings on the Combustion and Thermoelectric Plants are implemented.
- Speed, Energy, Heat and Insulation upgrades; machine side settings; settings/security cards, crates, vaults and storage upgrades.

### Steam, world resources and refining

- Electric Pump, Industrial Water Intake, Steam Boiler Array, Steam Turbine Array, Superheater Array, Condenser Array and closed water/steam loops.
- Oil deposits, Seismic Survey Tool, Oil Derrick, Well Head, Pumpjack and shared deposit depletion.
- Distillation Array and fuels/products; Cracking Unit, Catalytic Reformer, Coker, Lubricant Blender, Hydrotreater, Sulfur Recovery Unit, Steam Cracker, Diesel Generator and Flare Stack.
- Fluids and gases: water, brine, creosote, crude/sour crude, refinery fractions, steam grades, exhaust steam, hydrogen, oxygen, syngas, acid, chlorine, plastics and byproducts.
- Data-driven fuel, recipe and worldgen definitions; migration-safe world configuration.

### Chemistry, agriculture and advanced power

- Electrolyzer, Chemical Reactor and array, Carbon Reclaimer, Gasifier, Fischer-Tropsch Reactor, Thermal Evaporator Array, biodiesel chain, lithium/brine/battery chain and ore leaching.
- Fermenter, Seed Oil Press, biofuel crops and relevant crop/soil behavior.
- Nuclear progression: Uraninite/Galena processing, fuel fabrication, reactor components, fission reactor(s), fuel cycle, radiation/heat/safety behavior, and associated power generation. Confirm the exact currently shipped nuclear feature set against upstream before starting this phase.
- Diamond Press, turbine upgrades, higher tier machines and endgame materials.

### Player-facing features and integrations

- Jetpacks and armored jetpacks (gas tanks, modes, fuel display, thrust, sounds/particles and server-authoritative flight); Arc Drill/Saw with charge, modules, area/vein/felling behavior; long-fall equipment.
- Wrench port/dismantle interactions, general tank item automation and cylinder side configuration, machine screens, tooltips, HUDs, sounds, particles, gas tint, comparator output and creative inventory. Pressurized Cylinder cartridge-slot transfer is implemented, but its dedicated screen, side modes, and comparator remain unfinished. Basic Wrench face configuration for FE conduits and energy output faces on the Combustion and Thermoelectric Plants, plus clockwise facing rotation for supported single-block machines, are implemented; rotation does not yet change sided item/energy behavior.
- Optional questline parity (the 20-chapter/280-quest Hibiscus content and custom triggers) only after the core recipe/progression graph is stable; never make quests a dependency for machines.
- Publicly described APIs/integration surfaces, interoperability tags, and optional integrations with other Horizon-native content mods.

## Architecture to establish before broad content work

### Content layer (`content/`)

Use a Horizon content mod for namespace-owned item/block declarations, recipes, tags, loot, models, textures, creative tabs and data-driven worldgen whenever the current `.hmod` schema can express the behavior. Keep machine and multiblock state changes in Rust. Give every survival item and block a recipe/loot path and a deliberate creative-tab entry.

### Native layer (`api/`)

Split the native crate into focused modules: `registry`, `machine`, `recipe`, `inventory`, `fluid`, `energy`, `heat`, `ports`, `upgrade`, `multiblock`, `worldgen`, `player`, `network`, `screen`, `render`, and `migration`. Keep hook implementations thin; put deterministic state transitions in pure domain functions and call Horizon through a narrow adapter.

Use only the pinned Horizon modding API: native blocks/items, block entities/containers/menus/screens, fluid and energy handlers, world and server tick hooks, recipes, item panels, interactions, sounds, particles and render hooks that exist at the pinned revision. Verify the generated API before relying on a hook. Do not change Horizon's modding API or plan upstream hook additions during the alpha. When an Arcforge feature cannot be implemented reliably with the existing hooks, record a mod-side simplification or defer it with a clear parity note; do not emulate the Java runtime wholesale.

### Core data model

- `MachineKind`, `MachineState`, `MachineRecipe`, item/fluid ingredients and outputs, byproducts, duration, FE/t, HU/t, minimum temperature and optional conditions.
- `PortMode` and per-face policy for item, fluid, gas, energy, heat, oxygen, steam and exhaust; explicit priority and conflict rules.
- Integer fixed-point energy/heat/temperature/flow math with documented units and overflow-safe operations.
- Persistent versioned machine state; transient client display data separated from authoritative state.
- A multiblock definition/formed-instance model with deterministic validation, controller identity, orientation, bounds, ports, teardown and recovery after chunk unload or block break.
- A migration registry keyed by save schema version, preserving inventory, tanks, FE/HU, configuration, upgrades and names when blocks are upgraded, moved or unformed.

## Delivery roadmap

The implementation phases are ordered by dependency. The alpha's player unlock order follows Arcforge's original 20-chapter progression, documented in [`docs/plan/progression.md`](docs/plan/progression.md). Each phase exit criterion is a release gate; large scopes should ship as multiple small slices inside each phase.

### Phase 0 — reference audit and project baseline

- Record the exact Arcforge upstream commit/release and make a feature matrix mapping each feature to upstream docs/data/code location, Horizon mechanism, dependencies, and parity decision.
- Inspect all upstream survival items, blocks, recipes, tags, worldgen, configs, upgrade paths, interactions and save migrations. Mark uncertain/omitted features explicitly.
- Establish project layout, license/attribution policy, pinned Horizon revision, formatting/lint policy, asset naming, recipe conventions and release metadata.
- Record whether each feature fits the pinned Horizon API, has a reliable mod-side implementation, or must be simplified/deferred for alpha. Do not propose Horizon modding API changes.

**Exit:** a versioned scope inventory and a buildable empty content/native bundle that loads in a disposable world.

### Phase 1 — machine framework and steelmaking vertical slice

- Replace the prototype's coal/iron-only tick with a reusable recipe engine and explicit item/fuel slots, saved progress, temperature, FE/HU and byproduct handling.
- Implement steel ingot, slag, coke, carbon dust, machine bricks, core alloys, plates/rods/gears and recipes.
- Implement Carbonizer as a single block first; then Arcforge Furnace as a 3×3×6 multiblock with formation, 1,200°C threshold, coke reagent/fuel, four inputs, additives and slag.
- Add screens, progress/temperature indicators, sided item IO, hopper automation, recipe discoverability and basic sounds/visual state.

**Exit:** a new survival world can progress from vanilla iron/coal to steel, coke and first alloy; save/reload and automation preserve exact contents and progress.

### Phase 2 — foundational FE and heat simulation

- FE conduits pool mod-side storage and throughput across bidirectionally connected members, with directed links between pools. The FE and thermal schedulers use the current block entity passed by Horizon during its tick, since ordinary world lookups omit that entity; this code path compiles but still needs in-world verification. Compare ordering, native per-block capability behavior, and legacy charge migration against Arcforge. Energy Cells preserve charge through ordinary player mining and placement and use a per-tick snapshot for other loot callbacks, while same-tick explosion accuracy, tier-upgrade recipes, and orientation-aware faces remain incomplete.
- Implement HU stores, temperature mapping, heat transfer only hot-to-cold, transfer limits, thermal equilibrium/rounding rules and Heat Cells. Thermal conduits now pool HU over bidirectionally connected members, and four automatic Heat Cell tiers store, leak, and transfer HU; their runtime behavior and remaining source controls still need verification/implementation.
- Implement Combustion Plant, Firebox, Geothermal Plant and Thermoelectric Plant; add fuel maps, lava contact heat, output limits and pause-when-full behavior. Core Geothermal Plant lava and passive-contact heat production is now implemented; its controls/upgrades and source heat network remain open.
- Add Heat/Energy upgrades, meters, comparator outputs, redstone modes and server-authoritative screen state.

**Exit:** generator → FE consumer and firebox → heat consumer loops work, persist, respect face modes, stop at capacity and behave identically in integrated and LAN play.

### Phase 3 — recipes and single-block processing

- Generalize recipe categories, matching, fluid/energy/heat costs, byproducts, catalysts, minimum temperature, config multipliers and reload behavior.
- Port Arc Crusher, Sifter, Induction Furnace, Metal Press, Fiberizer and Infuser.
- Port the remaining steel/alloy/tool/armor/building progression and all required recipe/tag data.
- Add machine tiers, upgrade slots, speed/energy/heat upgrades, dies and output backpressure.

**Exit:** every early and midgame progression recipe is machine-complete and recipe data can be extended without Rust changes.

### Phase 4 — shared logistics and machine controls

- Implement four tiers of energy/heat/fluid/item/pressurized conduits, face modes, transfer rate/capacity, filtering, facades, rendering and active-flow state. FE/HU pooling and the stored/just-moved active glow are now implemented; other conduit families and animated flow remain open.
- Extend Fluid Tank item transfer beyond implemented partial bucket and canister hand exchange to automated tank slots and broader item-fluid support; add content-preserving upgrades, Pressurized Cylinder side controls, gas tint/comparator and component-preserving upgrades, Heat Cell screens/capsules/insulation and charge-preserving upgrades, Lithium Cells and Battery Arrays, charge-preserving Energy Cell tier upgrades, gauge/comparator behavior, redstone controls, and orientation-aware sides. Ordinary Energy Cell mining and placement now preserve stored FE through the block item.
- Finish Wrench modes and side configuration UI, then add machine links/wired links, safe transfer simulation and access/security cards as scoped by Horizon support.
- Add automated machine-to-machine transfer and compatibility tests with vanilla hoppers and Horizon-native handlers.

**Exit:** machines can be wired into persistent, configurable item/fluid/gas/FE/HU networks with no duplication, loss, or extraction bypass.

### Phase 5 — multiblock foundation and arrays

- Generalize formation checks, controller orientation, bounds, casing/window predicates, port registration, teardown, chunk-boundary behavior and saved multiblock identity.
- Port Firebox Array, Arc Crushing Array, Induction Furnace Array, Metal Pressing Array, Chemical Reactor Array and Battery Array.
- Port array lane scheduling, parallel outputs, scaled buffers/capacities/rates and formation diagnostics.
- Add multiblock visuals incrementally; keep a legible fallback if custom animated rendering is unavailable.

**Exit:** all initial arrays form/revert safely after block break, world reload and chunk unload, and preserve inventories/fluids/energy.

### Phase 6 — steam and renewable heat

- Port Industrial Water Intake, Steam Boiler Array, Steam Turbine Array, Superheater Array, Condenser Array and Electric Pump.
- Add three steam grades, water/exhaust loops, overpressure/venting, turbine spin-up, lubricant bonus, temperature/pressure modes and solar thermal tracking.
- Add relevant fluid/gas ports, environmental cooling, closed-loop examples and screen readouts.

**Exit:** a complete water → steam → FE → exhaust → water power plant runs and recovers from missing input/output/capacity without voiding contents.

### Phase 7 — ores, geology and material expansion

- Complete ore processing, storage forms, alloy progression, and the survival recipe paths for the implemented ore/mineral worldgen.
- Match Halite's custom flat-bed geometry; Horizon's vanilla ore feature currently approximates its frequency, size, and height range.
- Add Invar, nickel/silver/tungsten/fluorite/bismuth/Arcite materials, storage blocks and dependent machines/upgrades.
- Validate new chunk generation, existing-world behavior, dimension restrictions and disabled-ore configurations.

**Exit:** all material progression is obtainable in survival on new chunks, and config changes never invalidate existing saved blocks/items.

### Phase 8 — oil extraction and refinery chain

- Implement deterministic regional oil deposits, survey tool, derrick drilling, well heads, pumpjacks and shared deposit depletion.
- Port Distillation Array and crude/creosote recipes; then Cracking Unit, Reformer, Coker, Blender, Hydrotreater, Sulfur Recovery, Steam Cracker, Diesel Generator and Flare Stack.
- Add fluids/gases, recipe maps, catalysts, byproducts, sour variants, gas/item transfer, flare behavior and oil-rich biome tags.

**Exit:** a complete oil deposit → crude → refinery products → fuel/power chain runs, survives restart and handles drained deposits and full tanks deterministically.

### Phase 9 — chemistry, agriculture and batteries

- Port Electrolyzer, Chemical Reactor, Carbon Reclaimer, Gasifier and Fischer-Tropsch Reactor with energy-balance floor and heat windows.
- Port Thermal Evaporator Array, salt/brine, chlor-alkali, lithium extraction, battery materials and Battery Array.
- Port Fermenter, oil crops, Seed Oil Press, biodiesel and agriculture progression.
- Add recipe categories, multi-tank handling, waste/byproduct rules and safety/vent controls.

**Exit:** chemical and agricultural loops are fully automatable, bounded by input/output capacities and cannot create net energy through recycling.

### Phase 10 — nuclear and endgame processing

- Audit upstream's exact nuclear gameplay and dependencies first; model reactor structure, fuel, heat, cooling, control, outputs, hazards and failure outcomes.
- Port nuclear ores/materials and complete fuel-cycle chain before reactor blocks become survival-accessible.
- Port Diamond Press, high-tier alloys/materials, Arcforged networks, modules and remaining endgame machines.
- Add clear safeguards, controllable shutdown, instrumentation and server limits for expensive simulations.

**Exit:** the complete endgame chain is obtainable, observable and stable across saves, chunk boundaries and multiplayer.

### Phase 11 — tools, equipment and player systems

- Basic steel tools and armor are implemented; add the Steel Hammer/Excavator area behavior and protection/break-event rules.
- Port Foundry Suit heat/fire protection and lava shield; Long Fall Boots and armored upgrade preservation.
- Port Jetpacks, gas fuel, three flight modes, chest/accessory compatibility where possible, fuel HUD, exhaust effects and server authority.
- Port Arc Drill/Saw energy storage, charging, modules, area/vein/felling rules, tooltips and hand animations.

**Exit:** multiplayer tests confirm client input cannot spoof movement or block breaking, and all equipment state is retained through crafting, charging and upgrades.

### Phase 12 — quests, polish and compatibility

- Recreate the optional quest progression and triggers from the stable content graph without copying the upstream quest assets verbatim.
- Finish custom block/item visuals, GUIs, particles, sounds, creative tabs, manuals and configurable balance settings.
- Add integration docs, interoperability tags and optional adapters for supported Horizon-native mods.
- Run full new-world, existing-world, LAN/dedicated-server, upgrade/migration, automation, performance and content-completeness passes; publish known limitations and matched Arcforge version.

**Exit:** every feature in the audited inventory is implemented or explicitly marked unsupported with a reason; clean install and upgrade paths are documented.

## Cross-cutting quality gates

- **State safety:** all inventory/fluid/FE/HU movements are atomic or simulate-then-commit; outputs never overwrite unlike stacks; full outputs stop processing; every saved entity has schema version and bounded values.
- **Multiplayer:** server owns simulation and permissions; clients display synchronized state only; joining clients receive matching content; no client-only inventory or world mutations.
- **World safety:** deterministic generation from world seed, bounded scans, no unbounded per-tick neighbor searches, no force-load assumptions, and safe behavior at chunk borders.
- **Performance:** amortize multiblock validation; schedule network transfer; avoid allocations/large recipe scans per tick; cap all player-controlled dimensions, inventories, arrays and scans.
- **Balance:** port published Arcforge rates and recipes as a starting target, record intentional deviations, then tune only with explicit progression review.
- **Migration:** test prior-version saves with filled inventories, tanks, energy, heat, upgrades, orientation and formed/unformed structures. Never silently discard state.
- **Packaging:** verify `.hmod` schemas and registries, `.hnmod` trust/install flow, clean build at the pinned revision, optional resource assets, and archive checksums.

## Build the current prototype

This native mod is pinned to Horizon Nightly 2026-11-08, commit `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca`. Review native code before building; native mods run with the user's permissions.

```sh
/home/rayyan/Applications/Horizon.AppImage native-build \
  /home/rayyan/Projects/arcforge-horizon \
  --instance arcforge-port-test \
  --trust-native-code
```

Review the source before running this command: `--trust-native-code` grants the compiled mod the user's permissions. The build selects the native bundle for the disposable `arcforge-port-test` instance. Keep the build target and gameplay trial world disposable until the save migration and client/server behavior gates above are in place.

## Latest verification

Recorded 2026-11-12. Horizon remains pinned to Nightly 2026-11-08 commit `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca`. Local Rust format/check, content build/check, and input downloads pass. GitHub run [43339016409](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43339016409) prepared Horizon from Mojang's verified client and all 66 Linux libraries, then failed when native-build found a symlink in the nested Horizon source checkout. CI now stages only the mod manifest and API source in a clean directory. A new client run and in-world runtime check are pending. No Horizon source/API changes were made.
