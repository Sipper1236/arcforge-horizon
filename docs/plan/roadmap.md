# Delivery roadmap

This roadmap keeps the full feature target intact and organizes implementation around playable progression. The player-facing unlock order is in [progression.md](progression.md); implementation slices may build dependencies ahead of their unlock.

## Gate 0 — source and packaging baseline

Pin the Arcforge and Horizon revisions, retain license notices, validate `.hmod` resources, build the native mod, and load it in a disposable world. Record current state and test commands. **Current (2026-11-12):** Arcforge remains pinned at `8843cb02cf9132f278f6a8e804b6d78837d3cbcb`; Horizon is pinned to official Nightly 2026-11-08 `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca`. The 384-recipe `.hmod` build/check and Horizon-pinned Cargo check pass, including the Pressurized Conduit edits. Four Item and four Fluid Conduit tiers have source recipes and transactional network transfers; fluid Auto faces use Horizon sided tank capabilities. Pressurized Conduits now have four compiled tiers, source-matched known-gas IDs, and packaged recipes/loot/tags; runtime behavior remains unverified. Rust tests have not been run. The current 384-recipe `.hmod` is installed and enabled in the disposable profile; the previous archive is backed up beside it. GitHub Actions run [43094118580](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43094118580) packaged both files, but the native bundle omitted the declared `api/` crate folder; a packaging fix is awaiting CI revalidation. A trust-authorized local native build for this pin has not completed: Cargo dependency fetches from crates.io timed out after SSL resets. Direct `horizon-ctl` previously opened an offline creative world from existing files, but no current-source item or machine behavior has been verified in-world.

## Gate 1 — Getting Started

Complete the opening recipes and machine chain: Wrench and starter power, Arcforge furnace bricks, Carbonizer, Coal Coke and creosote, formed Arcforge Furnace, steel and slag, Wrought Alloy, Metal Press/dies, Firebox, Thermoelectric Plant, and the first Wrought networks and Energy Cells. The craftable Wrench can configure Energy Conduit faces and per-face adjacent energy output on Combustion and Thermoelectric Plants; all four FE tiers relay adjacent FE at their source tier rates. The Furnace runs the 12 source recipes, and the Firebox-to-Thermoelectric path simulates source temperature and backpressure. Wrench-configured plant output modes affect adjacent pushes; capability extraction remains all-side. Full Wrench modes, Furnace controls, generator upgrades, pooled network behavior, and in-world behavior tests remain incomplete.

**Acceptance:** a new world can produce steel using the upstream recipe path, transfer power/heat through the supported mod-side implementation, and preserve machine contents/progress after save and reload.

## Gate 2 — Alloys & Tiers

The Furnace and its source Steel, Wrought, Tempered, Hardened, Arcforged Alloy, and Graphite recipes are implemented; the Arcite/Tungsten Composite recipe is also present. Arcforged Alloy is not yet survival-obtainable because its Carbon Fiber ingredient depends on Pitch from the unimplemented Refinery chain. Complete the remaining alloy dependencies and storage/network tier upgrades. Resolve dependencies such as Carbon Fiber through the source progression rather than adding an easier shortcut.

**Acceptance:** each tier's materials are obtainable through its intended recipe chain; upgrading storage preserves contents.

## Gate 3 — Ore Processing

Ore/raw generation, source mining tags/drops, Sifter/mesh tiers, Arc Crusher recipes, and 32 smelting plus 31 blasting conversion recipes are implemented. The Induction Furnace uses those 32 smelting recipes at source time and power costs, alongside its common vanilla fallback. Complete remaining dust/ore reduction coverage, Fortune/Silk Touch behavior audits, arbitrary vanilla/datapack Induction Furnace lookup, and the Crushing/Induction/Pressing arrays.

**Acceptance:** new ores are discoverable in new chunks and can be processed through the same progression gates as Arcforge; disabled/configured cases do not corrupt existing worlds.

## Gates 4–7 — Heat, logistics, gear, automation

Add heat producers/consumers, conduits and cells; then storage and per-face transfer; then tools/equipment; then machine arrays and automation. The four Item Conduit tiers now move bounded item counts per network operation between Wrench-configured source and destination faces; port packet travel, filters, conduit buffering, and automatic source/sink detection. Ordinary Energy Cell block-item charge preservation and bucket exchange for the four Fluid Tank tiers are implemented in source; verify them in-world before relying on them. Preserve recipe and chapter order, and keep all transfers bounded and save-safe.

**Acceptance:** the supported generator-to-consumer and resource-transfer loops run, stop at capacity, and work with the game's existing automation where available. Save/reload and in-world behavior are not yet verified for the current implementation.

## Gates 8–15 — Steam, engines, oil, refinery, chemistry, salt, farming, bio-industry

Build water/steam loops, turbines, deposit-based extraction, refining, chemical processing, lithium/battery materials, and agricultural/biofuel chains in the order of the source chapters.

**Acceptance:** each chain has an obtainable input, data-driven recipes, bounded output/byproduct handling, and survives save/reload.

## Gates 16–20 — Nuclear and endgame

Complete nuclear materials and fuel before reactors; implement power, safety, fuel cycling, and advanced cores in source order. Keep hazards and shutdown behavior observable and server-authoritative.

**Acceptance:** the entire fuel cycle is obtainable and reactors remain bounded, controllable, and safe across restarts and multiplayer.

## Final parity gate

Audit each feature in the root README against the pinned source. Mark it implemented, intentionally simplified for the current API, or deferred with a reason. Test clean install, all progression gates, automation, save migration, LAN/dedicated server behavior, and performance before calling the full port complete.
