# System plan: materials and world generation

## Target

Follow Arcforge's Ore Processing chapter after Getting Started and Alloys & Tiers. The source inventory includes Silver, Nickel, Tungsten, Fluorite, Bismuth, Arcite, Halite, Spodumene, Uraninite, and Galena in the Overworld, plus Iolite in the End. Include raw forms, storage blocks, dusts, mining tiers, Fortune/Silk Touch, and configuration behavior.

## Build order

1. Audit each upstream ore's block/item ids, generation config, biome/dimension rules, mining requirements, drops, and processing recipes.
2. Register blocks and items with their upstream assets and loot behavior.
3. Add data-driven generation for new chunks and keep generation deterministic from the world seed.
4. Add raw-to-dust and smelting/pressing paths using the existing crusher, sifter, press, and induction-furnace systems.
5. Add storage blocks, common tags, and recipe dependencies for alloys and later machines.
6. Verify disabled ores and existing worlds; configuration changes must not remove placed blocks or invalidate stored items.

## Current state

The configured and placed features for the pinned Overworld, Nether, and End ore set are registered with source-default placement data, and applicable drops, mining tags, and furnace conversions are present. The source Invar Dust crafting recipe now connects the registered Iron/Nickel Dust inputs to its existing crushing and smelting paths, with a local `c:dusts/iron` tag. Halite's flat-bed shape is approximated by a vanilla ore cluster; Arcforge's per-world generation toggles are unavailable in the pinned content schema. Other material IDs, raw/refined storage, and processing paths remain incomplete; see [implemented](../../status/implemented.md) and [not-built](../../status/not-built.md) for the exact boundary.

## Acceptance checks

- Each target ore appears only in its intended dimension/biomes and only in newly generated chunks.
- Mining level, Silk Touch, and Fortune match upstream behavior.
- Raw ores and blocks have complete processing and storage paths.
- Disabling generation does not break existing saves or recipes for already-owned items.
