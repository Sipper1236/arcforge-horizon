# Arcforge progression order

The alpha follows the chapter order in Arcforge's pinned `docs/QUESTLINE.md`. Quest content itself remains optional; the chapter gates are used to preserve the survival progression.

| Order | Arcforge chapter | Progression role |
|---:|---|---|
| 1 | Getting Started | Wrench, first power, furnace bricks, Carbonizer, Coal Coke, Arcforge Furnace, first steel, Metal Press, early heat-to-power, and Wrought-tier networks. |
| 2 | Alloys & Tiers | Wrought, Tempered, Hardened, and Arcforged alloys; Graphite; tier upgrades. |
| 3 | Ore Processing | New ore veins and raw forms, dusts, crushing, sifting, induction smelting, arrays, and machine parts. |
| 4 | Heat & Power | Geothermal and fuel burners, heat conduits/cells, Firebox Array, solar heat, oxy-fuel, and thermal upgrades. |
| 5 | Logistics & Storage | Conduits, tanks, cells, crates, vaults, reservoirs, meters, machine links, and storage upgrades. |
| 6 | Tools & Gear | Steel tools and armor, Foundry Suit, Jetpacks, Arc Drill/Saw, and modules. |
| 7 | Automation | Machine arrays and item/fluid/energy/heat automation. |
| 8 | Steam Power | Water intake, pumps, boilers, steam grades, and closed water/steam handling. |
| 9 | Turbines & Engines | Turbine power, generators, and high-output engine chains. |
| 10 | Oil & Gas | Deposits, surveys, wells, derricks, and extraction. |
| 11 | Distillation & Refinery | Crude processing, fractions, fuels, lubricants, and refinery byproducts. |
| 12 | Chemistry & Plastics | Chemical processing, gases, catalysts, plastics, and chemical byproducts. |
| 13 | Salt & Lithium | Brine, salt, lithium extraction, and battery materials. |
| 14 | Farming | Crops and agricultural inputs used by industrial chains. |
| 15 | Bio-Industry | Fermentation, seed oil, biofuel, and biomass processing. |
| 16 | Nuclear Fuel | Uranium/related material processing and fuel fabrication. |
| 17 | Nuclear Power | Reactor construction and power generation. |
| 18 | Reactor Safety | Controls, cooling, heat limits, hazards, and safe shutdown. |
| 19 | Fuel Cycle | Used-fuel processing and recovery loops. |
| 20 | Advanced Cores | Endgame materials, cores, and final advanced machines. |

## Implementation notes

- Player unlocks should follow this order even when implementation work temporarily builds a later prerequisite first.
- The first chapter already depends on several systems currently absent from this port. Keep those gaps visible rather than silently replacing the recipes.
- The source questline contains detailed item and trigger gates. Consult the pinned upstream chapter before implementing each gate; do not copy quests into alpha as a machine dependency.
- If the pinned Horizon API forces a simplification, keep it inside the mod and record the changed recipe or interaction in the system plan and status docs.
