# Current build policy (updated 2026-11-15)
Build and compile only through GitHub Actions. Do not run hybrid or local compilation. Earlier hybrid artifacts and steps below are historical; use GitHub Actions artifacts for current playtesting.
Latest successful GitHub build: run 43773403158; remote artifacts: /mnt/hdd2tb/arcforge-horizon-builds/arcforge-github-43773403158/.

## Current working checkpoint — Oil deposit survey alpha (GitHub build pending)
The working tree adds api/src/oil_deposit.rs, registers the Seismic Survey Tool, and reports deterministic ordinary-weight Overworld deposit results on block use. It does not yet extract oil or persist depleted volumes. Build only through GitHub Actions.
## Current working checkpoint — Superheater Array alpha (GitHub verified; gameplay pending)
Commit f1f8608 includes the Superheater Array and Horizon host-type fixes. GitHub Actions run 43773403158 passed the native crate check, Horizon content build/check, mod packaging, and Linux client build. The importable client ZIP is /mnt/hdd2tb/arcforge-horizon-builds/arcforge-github-43773403158/arcforge-playable-linux-x86_64-f1f8608b1d6ce52b67cbe38dbbb6ae5ff2dd1cc9/arcforge-linux-x86_64.zip (SHA-256 63654ada508bb66a4501b67aa34a627d10025e21312e6f3ea968a1b7b4e5c813). Embedded checksums and ZIP integrity verified. This proves build/package integrity, not in-game behavior. Next, import the client, craft Superheater Array Controller/Casing, form a 3x3x3 shell, use the Wrench to set facing and pressure mode, connect a thermal conduit, feed Steam, and confirm flow/status; then test High-Pressure and Superheated upgrades and save/reload. Build and compile only through GitHub Actions.

# Project handoff: Arcforge for Horizon

## Current working checkpoint — Electric Pump alpha
The remote checkout contains api/src/electric_pump.rs, block and capability registrations, source recipe/drop data, existing pump models, and an updated handbook entry. It uses 10 FE/t, fills 1,000 mB every 20 ticks, and has a 20,000 FE buffer plus a 16,000 mB tank. It pumps only infinite source water, requiring two horizontal neighboring source-water blocks, and does not remove those blocks. Finite-fluid pickup and ocean/beach Seawater conversion are deferred to unavailable pinned hooks. It passed GitHub Actions run [43769113792](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43769113792) for commit f56e5e41ed3ae81e895b93714281b143084c7f29; the Rust/content and Linux client jobs succeeded. In-game behavior has not been verified. Client ZIP: /mnt/hdd2tb/arcforge-horizon-builds/arcforge-github-43769113792/arcforge-playable-linux-x86_64-f56e5e41ed3ae81e895b93714281b143084c7f29/arcforge-linux-x86_64.zip (SHA-256 c5c2f72fb81bc82de337eb49a05677b5170aa6991b548862da2a600797179112); HMOD 88d689174a64eb2276a8cc2ca9fe2602fb953dd1b224d7788126563e880a077b; HNMOD ebd974da714a466565ece02edf0f9acfd759887b6eb1678a8c3228a90c74e233.

## Current working checkpoint — Industrial Water Intake alpha
The remote checkout now contains api/src/water_intake.rs, controller/casing registrations, survival recipes and loot, controller assets, and a right-click status readout. It implements a fixed 3×3×3 shell over at least 400 connected source-water blocks, pumps up to 8,000 mB/t at 0.25 FE/mB into a 128,000 mB tank, and leaves world water in place. The source count is capped at 400 and refreshed every 100 ticks. River/ocean rate bonus and loaded-chunk filtering are not represented because pinned Horizon hooks lack those queries. Hybrid build passed; gameplay has not been tested. Build ZIP: /mnt/hdd2tb/arcforge-horizon-builds/arcforge-hybrid-2026-11-15-water-intake/arcforge-linux-x86_64.zip (SHA-256 6b2b445fef553ae8ffe0aea752a3d951ae0d0a99ba78b2c9a7d594b0f91841b4). SHA manifests and unpacked files are beside it.
## Current working checkpoint — Condenser Array alpha on hybrid client
`api/src/steam_condenser.rs` adds a formed 3–7 block Condenser Array that converts Exhaust Steam to water 1:1, adds source-style cooling bonuses from adjacent water/ice, and reports state with the Wrench. The full Linux client and both Arcforge packages were rebuilt on rx580 with the hybrid workflow. Artifact: `/mnt/hdd2tb/arcforge-horizon-builds/arcforge-hybrid-2026-11-15-steam-turbine-rerun/arcforge-linux-x86_64.zip` (SHA-256 `319b87e1e17a250b558b2c98650e9a4c8391d611e5dd721b660e0ac1d9441221`). No GitHub build or in-game test was run.


**Snapshot date:** 2026-11-15
## Current snapshot — Steam Turbine and Condenser Array alpha

The Condenser Array source, content, and native package build on rx580 alongside the earlier Steam Turbine implementation. `docs/status/verification.md` records exact checksums. The Condenser Array slice is committed locally on `main`; it has not been pushed to GitHub.

Project and builds are stored on the remote drive at `/mnt/hdd2tb/arcforge-horizon` and `/mnt/hdd2tb/arcforge-horizon-builds`; `/home/rayyan/Projects/arcforge-horizon*` are symlinks. GitHub Actions run [43738891125](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43738891125) passed at commit `9da8a49`, against Horizon Nightly 2026-11-13 revision `9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0`. Playable client path: `/mnt/hdd2tb/arcforge-horizon-builds/arcforge-run-43738891125/arcforge-playable-linux-x86_64-9da8a499837a3345520123ee872aaeb8ea50ad91/arcforge-linux-x86_64.zip` (SHA-256 `5227d66610ec600184bb38d978516e3617a95eed4a3d27b6854fa800e3c38170`). Imported locally as `imported-2026-11-15-4f43d47`; isolated test instance `arcforge-area-tools-test` points to it and contains the matching content pack/manifest. No launch or in-world test was performed. Next: test the Hammer/Excavator behavior in that instance when able, then continue the source feature inventory in [goal.md](goal.md).

Read-only source checkouts are also on that drive: Arcforge at `/mnt/hdd2tb/arcforge-upstream` commit `8843cb02cf9132f278f6a8e804b6d78837d3cbcb`, and Horizon at `/mnt/hdd2tb/horizon-reference` revision `9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0`. The latest API-boundary audit records why exact Halite beds and source-block Electric Pump pickup stay simplified/deferred under the current hooks; see [materials/worldgen](plan/systems/materials-and-worldgen.md), [steam/resources/industry](plan/systems/steam-resources-industry.md), and [parity notes](status/parity-notes.md).

**Project:** `/mnt/hdd2tb/arcforge-horizon` on `rayyan@100.89.43.102`

**Active goal:** see [goal.md](goal.md)  
**Status:** the Steam Turbine and Condenser Array sources, checked `.hmod`, `.hnmod`, and matching Linux client bundle build on rx580; both machines still need formation and gameplay verification.

Commit `5920fdd` adds an Arcforge Furnace Wrench status line for formation, temperature, burn time, and active recipe requirements/progress. Both GitHub jobs passed in run [43542601271](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43542601271); its package and playable client ZIPs and internal checksums are verified. In-world status verification remains pending. Commit `f15a514` adds the upstream behavior of clockwise port rotation when the structure is unformed, followed by an immediate formation recheck. Both GitHub jobs passed in run [43544310146](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43544310146), and the playable Linux client ZIP and checksums are verified in [verification.md](status/verification.md). In-world Wrench status and rotation checks remain pending.

Commit `ad227c0` adds Arc Crushing Array's source-default north-facing ports (top input, bottom output, south/back FE input) through the existing API. Commit `bd4be83` adds Wrench inspection on the Array center, reporting FE and all three lane progress values in the action bar. Its remote Rust check and both GitHub build jobs passed; the 41,302,382-byte playable Linux client ZIP and all internal checksums are verified in run [43532339226](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43532339226). Commit `8608317` extends Wrench FE/progress readouts to the Arc Crusher, Induction Furnace, Metal Press, and Sifter. Local format and remote compile checks and both GitHub build jobs pass; its 41,304,754-byte playable client ZIP and checksums are verified in [43534354970](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43534354970). In-world Array port, upgrade, and all readout checks remain pending. Saved per-casing Wrench port configuration remains unsupported by the pinned block-wide hooks. See [verification.md](status/verification.md) for package hashes and current build status.

Commit `158d858` adds explicit custom state schemas/defaults to the conduit tiers and Solar Array blocks after the earlier local smoke logged unsupported multipart properties. Both GitHub jobs passed in run [43540533024](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43540533024); the package and playable client archives and internal checksums are verified in [verification.md](status/verification.md). Corrected conduit and Solar rendering still needs an in-world check. The preceding menu commit is `83b8e22` (`feat: add Arcforge Furnace player menu`), built successfully in GitHub run [43547943707](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43547943707). The playable Linux ZIP and checksums are verified in [verification.md](status/verification.md). The previous code before this change was `f15a514` (`feat: rotate incomplete arcforge furnace port`), and its GitHub package/client build passed in run [43544310146](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43544310146); the playable Linux artifact and checksums are verified in [verification.md](status/verification.md).

**Earlier Furnace filter build (2026-11-14):** Commit `030998d` adds player-click validation for Arcforge Furnace slots 0–3. Both GitHub jobs in run [43610417779](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43610417779) passed; package and playable client hashes are recorded in [verification.md](status/verification.md). An initial CLI attempt paired the updated `.hmod` with an older native client and does not verify the hook. The matching client was installed into the existing imported test version with the old executable and manifest backed up in `/tmp/arcforge-version-backup-030998`.

**Latest source and runtime status (2026-11-14):** Commit `516bf9f` fixes and completes the Arc Crusher screen. GitHub run [43616251460](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43616251460) passed both package and Linux client jobs. The matching client rendered live FE/progress and completed a coal-to-carbon-dust recipe after 200 ticks, consuming 4,000 FE. Furnace slot insertion was also checked in that client: stone was refused at slot 300, iron entered slot 300, gold entered slots 301 and 302, and coke entered slot 303. The earlier stone report did not reproduce. A disposable 3×3×6 structure then formed and produced steel plus slag from iron and coke. The latest “it won’t go in” report did not name an item or slot; ask for both if it persists with a recipe ingredient. Current docs were stale before this update; see [verification.md](status/verification.md).

## Remote workspace

The working tree is stored at `sftp://rayyan@100.89.43.102/mnt/hdd2tb/arcforge-horizon`. This machine mounts `/mnt/hdd2tb` at `/home/rayyan/mnt/hdd2tb` with the enabled user service `arcforge-hdd2tb.service`; `/home/rayyan/Projects/arcforge-horizon` is a symlink to the remote checkout. GitHub build packages and playable client bundles are archived under `/mnt/hdd2tb/arcforge-horizon-builds/`, with a local compatibility symlink at `/home/rayyan/Projects/arcforge-horizon-builds`. Keep future project edits and build artifacts on that remote disk.

## Latest build artifacts — hybrid remote 2026-11-15
- Playable Linux client ZIP: `/mnt/hdd2tb/arcforge-horizon-builds/arcforge-hybrid-2026-11-15-steam-turbine-rerun/arcforge-linux-x86_64.zip` (SHA-256 `319b87e1e17a250b558b2c98650e9a4c8391d611e5dd721b660e0ac1d9441221`).
- The archive also contains standalone `arcforge.hmod` and `arcforge.hnmod` files, plus a source-only bundle ZIP and checksums. Horizon `native-mods.json` is included beside the client in the client ZIP.
- Import instructions are in `INSTALL.txt`. This verifies build and packaging, not in-game behavior.

## Previous GitHub build artifacts — 2026-11-15

GitHub Actions run [43658856875](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43658856875) passed package and Linux client build, assembly, and artifact upload for commit `0f45ffb`.

- Mod package ZIP: `/mnt/hdd2tb/arcforge-horizon-builds/arcforge-run-43658856875/arcforge-package.zip` (SHA-256 `f9bf29f98e9bebc780d61594a021af9c98190f0361cbc16bb6d70806a748da7b`).
- Playable artifact wrapper: `/mnt/hdd2tb/arcforge-horizon-builds/arcforge-run-43658856875/arcforge-playable-linux-x86_64.zip` (SHA-256 `b199b5d4c4d53033e6567a28f947a8ad610c0be27fe22496012afc086e869f77`). Its inner `arcforge-linux-x86_64.zip` SHA-256 is `06f8b62938fab34f3b78c42fafebb5a92a9358b4e8513d73a5885f5658c92515` and contains `horizon-client`, `native-mods.json`, `arcforge.hmod`, `arcforge.hnmod`, `INSTALL.txt`, and `SHA256SUMS`.

The archive was streamed to the remote build volume and verified against the GitHub artifact digest. `INSTALL.txt` contains Horizon import instructions. CI proves successful build and assembly, not in-world behavior. Commit `638969a` is comment-only; its exact-commit build is queued as run [43660398711](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43660398711).

## Previous build artifacts — liquid fuel buckets (2026-11-14)
GitHub Actions run [43654606595](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43654606595) passed both jobs. The package and Linux client ZIPs were downloaded directly into the remote build archive:

- `/mnt/hdd2tb/arcforge-horizon-builds/arcforge-run-43654606595/arcforge-playable-linux-x86_64-ca311120c37dac56515b0c0e77010e4948477afa.zip` — SHA-256 `6a979da8a6ff9bd37de89c0982ee1a8192f14869a6b16351afc01ea880080e33`
- `/mnt/hdd2tb/arcforge-horizon-builds/arcforge-run-43654606595/arcforge-ca311120c37dac56515b0c0e77010e4948477afa.zip` — SHA-256 `d02329c30343b101e61847fcc3d2f7461bd07658c68d808b1a27416e9d36cf6c`

The build confirms packaging and client assembly, not in-world behavior. Test the liquid fuel bucket interactions in Minecraft before treating them as runtime verified.

## 1. Resume here

1. Use the playable artifact from GitHub Actions run 43773403158; its remote copy is under /mnt/hdd2tb/arcforge-horizon-builds/arcforge-github-43773403158/. Import it into Horizon Nightly 2026-11-13 (`9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0`) with the included `INSTALL.txt`; install `arcforge.hmod` in the instance and enable `mod.arcforge`.
2. In a disposable world, craft the Steam Turbine Array Controller, build a 3×3×3 shell with casing corners, a hollow air core, and the controller centered on the front face. Use Pressure Glass or casing for the other shell blocks.
3. Feed steam from the Steam Boiler Array through a fluid conduit. Use the Wrench to check formation, steam flow, RPM, and FE; connect an energy conduit and extract Exhaust Steam from the controller.
4. Break and replace a shell block, rotate the controller with the Wrench, and save/reload. Record any issue, fix it without changing Horizon API/source, update verification docs, then continue through Arcforge’s progression.

**Observed local smoke:** On the pinned Nightly 2026-11-08 build, the imported client with matching manifest and content pack opened a disposable creative world. `/give arcforge:wrought_heat_cell 1` and `/give arcforge:steel_ingot 1` both succeeded and appeared in inventory. Runtime ore data parsed without skipped Arcforge placed features after conversion to `worldgen/feature/` and flattened target states. That earlier smoke logged unsupported conduit and Solar multipart properties; commit `158d858` declares those custom states, but corrected rendering is not yet verified. Machine behavior, ore distribution, and progression were not verified.

The GitHub artifact's first native executable was missing `native-mods.json`, so that exact ZIP starts without Arcforge registries if imported alone. The current workflow stages that manifest into the next artifact. The earlier native build also rejected duplicate `arcforge:raw_rubber`; the current source removes that duplicate. The `43340862715` artifact remains a historical build before these fixes.

Mojang's pinned 26.3 metadata and client/library downloads are verified by `tools/fetch_mojang_inputs.py` and remain ephemeral CI inputs. No private Actions secrets or user Microsoft token are required for CI build/prepare.

## 2. Fixed source targets and workspace

- Arcforge parity target: commit `8843cb02cf9132f278f6a8e804b6d78837d3cbcb`.
- Horizon API/build target: Nightly 2026-11-08, commit `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca`.
- Horizon checkout: `/home/rayyan/Projects/horizon-source` (clean before update; now detached at the exact official Nightly commit above).
- Arcforge reference checkout: `/tmp/arcforge-upstream` (read-only reference).
- Native Rust mod: `api/`, primarily `api/src/lib.rs`.
- Horizon content pack: `content/`.
- Git repository: `https://github.com/Sipper1236/arcforge-horizon`, branch `main`. Check `git status` before edits and commits.

## 3. What exists now

The native crate registers many core systems: Metal Press, Sifter, Arc Crusher, formed three-lane Arc Crushing Array, Carbonizer, Induction Furnace, Infuser, Resin Tap, Firebox, Firebox Array, Solar Thermal Array, Geothermal Plant, Fuel Burner, four Fluid Tank tiers, Thermoelectric Plant, four Energy Conduit tiers, four thermal conduit tiers, four Energy Cell tiers, four Heat Cell tiers, Wrench, Fiberizer, Vulcanizer, Grain Dryer, Arcforge Furnace and its bricks/port, a separate Steelworks prototype, Combustion Plant, coke and treated wood blocks, Creosote/Latex buckets, tiered portable canisters, tungsten heating coil, processing materials, seven Steel tools, and four Steel armor pieces.

Implemented behavior includes:

- Upstream machine recipe catalogs for 83 crushing, 25 pressing, four sifting, four carbonizing, ten infusing, three fiberizing, and twelve Arcforge Furnace smelting recipes. Induction Furnace loads 32 source smelting entries and a fixed set of vanilla fallbacks because its hook cannot query the vanilla/datapack recipe manager.
- A formed 3×3×3 Crushing Array with three lanes, output backpressure, per-lane processing, formation/recovery behavior, and rotating lane priority to avoid starving lanes when FE is scarce.
- A formed Firebox Array with a hollow 3–7 block casing/Pressure Glass structure, saved bounds and heat, solid coal/Bio Coal/Coal Coke fuels, source-scaled generation and storage, burn-temperature pauses, and supported-neighbor heat delivery. Liquid/gas fuels, oxygen, flue gas, ports, upgrades, and UI remain incomplete.
- A formed 2×2×4 Solar Thermal Array with source controller/casing/collector registrations, saved tower bounds and heat, a 40-tick structure scan, time/weather/alignment/sky-based generation, and Wrench-configurable heat-output faces on tower parts. Enabled faces transfer heat to supported adjacent consumers or thermal conduits and show a steel collar/heat-plate overlay on the formed block face. Biome output, snow distinction, controls, and UI remain incomplete; overlay appearance has not been inspected in-world.
- Source-default worldgen for the standard Overworld ore/mineral set, Nether Sulfur, End Iolite, Spodumene Pegmatite, and Heavy Mineral Sand, with applicable mining tags, drops, and furnace recipes. Halite's custom bed is approximated; worldgen switches remain fixed to the source defaults.
- Fuel Burner, Firebox, Combustion Plant, and Thermoelectric heat/FE slices. Arcforge Furnace processes the twelve source recipes. There is also a distinct Steelworks compatibility prototype.
- Geothermal Plant burns lava from an 8,000 mB tank at source rates and gains passive heat from adjacent lava source and magma blocks; its 60,000 HU buffer feeds supported consumers at 1,400°C. Vanilla bucket interactions require room for a full bucket, stage the empty bucket, and update the input-only tank atomically.
- Four FE conduit tiers and four thermal conduit tiers with Wrench-configurable face modes. Input receives, Output sends, Auto permits either direction, and Disabled disconnects. Both conduit families pool stored resources across bidirectionally connected members, use the lowest member tier's rate, and scale capacity by member count; one-way links transfer between separate pools. FE producers feed pools through configured output faces, and FE pools supply supported Arcforge machines; thermal pools retain the hottest carried source temperature and feed supported heat consumers. FE's native capability remains per block and can exceed the pooled rate through external callers, though changes are reconciled to prevent pooled writes from discarding that energy. Old FE charge in 50,000-per-block buffers is retained as temporary capacity during migration. Color/facade behavior, source-faithful dynamic thermal port nozzles, and other conduit families remain unfinished.
- Four Item Conduit tiers route items between configured faces with Horizon's transactional helper. Four Fluid Conduit tiers route one fluid at a time at the lowest tier's mB/t cap and use sided Horizon tank capabilities for Auto faces. Both compile but are not in-world verified. Item/fluid buffers, filters, and animated flow remain absent.
- Four Energy Cell tiers with source capacities/rates. Ordinary block mining carries FE into the dropped item, and placement restores FE while retaining custom data. A per-tick snapshot supports loot callbacks lacking block-entity access; explosion timing remains unverified. Tier-upgrade recipes do not copy charge.
- Four Heat Cell tiers store HU persistently, receive and output heat through all six adjacent faces, leak according to tier rates, expose five heat levels in blockstate, and preserve charge through block-item drops/placement. Recipes and loot are packaged. Screens, capsules, insulation, redstone/side settings, charge-preserving tier upgrades, and in-world verification remain.
- Four Fluid Tank tiers with capacities 16,000 / 64,000 / 256,000 / 1,024,000 mB, full vanilla water/lava bucket exchange, and partial Creosote/Latex bucket and tiered canister hand transfers at source rates. Failed inventory exchanges roll back in Rust logic. Automated canister slots are absent; tank/canister upgrade recipes lose contents because the crafting hook exposes no ingredients.
- Resin Tap's tree checks, Latex and Pine Resin production, player collection, hopper output, Latex bucket exchange, and source Pine Resin-to-slime-ball recipe.
- The treated wood set, basic Steel equipment, source-shaped crafting, loot/tags, and source assets where supported.

The root README has the fuller feature description. The source/data inventory and evidence level are in [implemented.md](status/implemented.md).

### Thermal conduit continuation (2026-11-08)

The four upstream thermal conduit tiers are registered with rates of 50, 200, 800, and 51,200 HU/t. Bidirectionally connected members share a HU pool: its rate is the lowest member tier, its capacity scales with member count, its input rate is budgeted once per game tick, and its output rate is separately budgeted once per tick. The hottest carried source temperature is synchronized across the pool. One-way Output-to-Input links relay through `accept_heat` into a separate destination pool, honoring both pools' budgets and face settings. Supported endpoints are Fiberizer, Vulcanizer, Grain Dryer, and Thermoelectric; Firebox, Firebox Array, Geothermal, Fuel Burner, and Wrench-enabled Solar faces can feed the pipes. The Wrench modes are Auto/Input/Output/Disabled. Source recipe patterns, ingredients, counts, base-tier tags, and drops are present; tier-upgrade recipes use vanilla shaped crafting because Horizon does not expose Arcforge's custom recipe type/component-copying hook. The native Horizon API was not changed. This is a mod-side scheduler and remains unverified in-world; source temperature edge cases, color data/dyed variants, and dynamic port models remain open.

### Conduit tick entity lookup correction (2026-11-08)

Both pooled schedulers now accept the current block entity as context while discovering/validating its network and collecting output endpoints. Horizon temporarily removes the ticking entity from ordinary world lookups, so the previous lookup-only entry path returned before running the scheduler. This source-level correction compiles against the pinned Horizon API; Rust tests, native bundle build, and in-world transfer behavior are still unverified. Exact checks are in [verification.md](status/verification.md).

| Check | Observed result | What it establishes |
|---|---|---|
| `cargo fmt --check --manifest-path api/Cargo.toml` | Passed after the FE/HU pooled schedulers and direction-preserving link updates. | Current Rust formatting is clean. |
| `cargo check --locked --manifest-path api/Cargo.toml` | Passed: 0 errors; existing dead-code warning for `EnergyStore::transfer_to`. | Both pooled network caches, per-tick budgets, and one-way conduit relays type-check against the pinned Horizon API. |
| Horizon content build/check | Not rerun; this slice changes Rust only. The prior Solar overlay archive remains 371 recipe JSON files, SHA-256 `5951d9ec9405d79370b7c012adf03e852cf13095186d834cfd29b66b36057271` (unsigned). | The content tree is unchanged; this older artifact does not verify Rust gameplay behavior. |
| Rust tests, native bundle, and in-world trial | Not run. | No behavior, native-client load, save/reload, or network-parity result is claimed. |

### FE conduit pooling continuation (2026-11-08)

The four FE conduit tiers now discover bidirectionally connected groups, pool their saved energy, use the lowest member tier's rate in mod-side transfers, and scale normal capacity by member count. Separate source-input and sink-output budgets apply once per pool per game tick. Producers feed a conduit through its configured Output/Auto face; a network leader distributes pooled FE across supported Arcforge consumers and directed Output-to-Input conduit links. The static Horizon energy capability remains per block; direct insert/extract calls are reconciled before pooled writes but cannot share a network rate budget, so they may exceed normal capacity. Existing 50,000 FE saved buffers can also exceed source capacity, so discovery preserves old charge as temporary capacity and lets it drain rather than deleting it on migration. Horizon API and engine hooks remain unchanged.

| Check | Result | What it establishes |
|---|---|---|
| `cargo fmt --check --manifest-path api/Cargo.toml` | Passed after the FE pool integration. | Rust formatting is clean. |
| `cargo check --locked --manifest-path api/Cargo.toml` | Passed: 0 errors; existing `EnergyStore::transfer_to` warning remains. | The energy network cache, shared mod-side budgets, and producer/consumer adapters compile against the pinned API. |
| Horizon content build/check | Not rerun at the time because this slice changed Rust only. | That historical checkpoint did not change content; the then-latest packaged artifact had 371 recipes. Current content checks are recorded in the Item Conduit section of `status/verification.md`. |
| Rust tests, native bundle, and in-world rate/direction/migration trial | Not run. | Runtime transfer ordering, legacy buffer migration, save/reload, and multiplayer behavior remain unverified. |

## 4. Verification record

The newest checks are the Heat Cell alpha slice dated 2026-11-12 at the top of [verification.md](status/verification.md); older feature-specific evidence follows it:

- Previous Solar overlay bundle: 371 recipe JSON files; Horizon build and checker passed with SHA-256 `5951d9ec9405d79370b7c012adf03e852cf13095186d834cfd29b66b36057271` before the Invar Dust recipe was added.
- Previous content bundle: 372 recipe JSON files after the Invar Dust progression addition; Horizon build/check passed with SHA-256 `22f5c584a702b6829fc3cada5307dbd732bc397847b9a55765b606220d9c68a9` (unsigned). The current 380-recipe archive and Item/Fluid Conduit source checks are recorded in the latest [verification.md](status/verification.md) sections.
- Earlier static audits covered smelting/blasting, Induction entries, common tags and item IDs, eleven loot tables, and steel equipment recipes as described in that file. The new Geothermal loot table has not had a separate source-parity audit.
- `cargo fmt --manifest-path api/Cargo.toml` and `cargo check --locked --manifest-path api/Cargo.toml`: passed after the Geothermal Plant change; one existing dead-code warning remains for `EnergyStore::transfer_to`.
- Historical pre-Nightly-update baseline: `cargo test --locked --manifest-path api/Cargo.toml` passed 85 tests across two suites; see newer results below for the current count.
- Horizon native build passed for the pinned client; disposable instance selected native bundle `native-66f3981-6165e0307f55`.
- No Horizon API changes were made.

### Nightly 2026-11-08 update

- Official releases page names `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca` as the game Nightly; the separately updated launcher is not the game-build pin.
- Historical Nightly-update checks passed against the new pin; the pre-item-transfer suite had 85 passing tests. The content build/check at that point covered 354 recipes (SHA-256 `a7c629dd7148f4ef7c620135610499627c42c7620fe52eebf22727b80f857968`). Later canister-slice results are recorded below.
- Native bundle build is pending because this shell cannot resolve `horizon-launcher` (`command not found`). No native build or gameplay load is claimed for the new pin yet.
- Intervening content/API generation changes were inspected; current mod hooks compile without changes. Content gained a `place_block` alias, and the current mod has not adopted it because no mapped item registration needed it in this slice. Full machine hook delta review remains in progress.
- Fluid Tank item-transfer slice: Arcforge source uses item fluid handlers in both directions and permits partial movement. The updated Horizon native item-fluid contract declares item capacity and stores `horizon_fluid` in custom data, but its generic fluid helper only knows water/lava. The mod now uses its existing server-side block-use and fluid hooks to exchange partial amounts with the already-registered Creosote/Latex bucket items (1,000 mB max), rejects mismatched item fluids and tank mixing, preserves unrelated custom data, and rolls back on inventory exchange failure. `cargo fmt --check`, `cargo check --locked`, and `cargo test --locked` passed; suite is now 88 tests, including result-inventory rollback. Updated details are in the README, `status/implemented.md`, `status/not-built.md`, `status/parity-notes.md`, `status/verification.md`, and `plan/systems/processing-logistics.md`. Native bundle is still blocked by missing `horizon-launcher`; no gameplay test was run.
- Portable canister continuation: pinned hooks allow a bounded server-side hand interaction and item capacity declaration. All four canister tiers transfer at upstream capacity/rates (16,000/1,000 through 1,024,000/64,000 mB), and four survival recipes were added. This is a documented alpha simplification from upstream's automated tank input/output slots. The crafting-prepared hook only exposes the prepared result, player/name, and table position; it does not expose ingredients, so the tank/canister tier recipes cannot copy stored fluid safely. The earlier 89-test pass predates the carried-stack replacement correction; the suite has not been rerun since that correction. The follow-up `cargo check --locked --tests` compiled the then-current tests but did not execute them. Content build/check passes with 358 recipe files; SHA-256 `74bea320e9974fbaa340735607277d807e79179bb16535952c8f7d452ac8469c`. No in-world test was run.
- Geothermal Plant continuation: source-shaped crafting and loot data are included. The native block accepts only lava, drains 100 mB at a time at 1 mB/t for 80 HU/mB, gathers 40 HU/t per adjacent lava source and 16 HU/t per adjacent magma block, and transfers heat to supported adjacent consumers. Base heat capacity and maximum temperature match upstream at 60,000 HU and 1,400°C; scan interval is 20 ticks. `cargo fmt` and `cargo check --locked` passed (one pre-existing dead-code warning); no Rust tests were run. Horizon content build/check passes with 359 recipe files; SHA-256 `e08b316db1191164e407b50c0c92c6919f2ad38af3b464c0c4093fe4e23aa3f3`. There was no native bundle or in-world test.
- Follow-up corrections: full vanilla buckets now require 1,000 mB of free tank capacity before insertion, preventing a partial-insert duplication path for input-only tanks. Conduit direction checks now respect Input as receive-only and Output as send-only for both machine/conduit and conduit/conduit transfers. `cargo fmt --check` and `cargo check --locked` passed; no tests or content build were run because this Rust-only change leaves content unchanged. No in-world check is available.
- Crushing Array fairness: saved lane priority now rotates the first lane considered each controller tick, preventing lane 0 from consuming every available 16 FE when total input cannot advance all three lanes. `cargo fmt --check` and `cargo check --locked` passed; tests were not run and no gameplay timing check was possible.
- Firebox Array slice: the controller, casing, and Pressure Glass are registered with source-shaped recipes and loot. The controller scans for a 3–7 block hollow box, saves its bounds, unforms the remaining walls when a casing or glass pane breaks, and burns the existing Firebox fuels plus Coal Coke/blocks. Heat buffer and maximum HU/t scale with box volume; solid fuels pause at their source temperatures. Heat is sent to supported adjacent consumers at the array throughput. Full Horizon API type-check, Rust tests, content build/check, native bundle build, and in-world behavior remain unverified after this slice. `cargo fmt --manifest-path api/Cargo.toml` passed after the current source edits. The connected-conduit Wrench path now cycles each face independently through Auto/Input/Output/Disabled.
- Solar Thermal Array slice: the controller, casing, collector block, receiver tube, and trough mirror are registered. The controller checks a 2×2×4 tower every 40 ticks, computes time/weather/alignment/sky output, stores up to 16,000 HU, and transfers heat through Wrench-configurable faces on tower parts to supported adjacent consumers. Breaking a component unforms the remaining tower. Biome output, snow distinction, source port blocks/visuals, thermal conduit output, and dedicated screen remain incomplete. `cargo fmt --check` and `cargo check --locked` passed; the Horizon content build/check passed with 367 recipes. Rust tests, native bundle build, and in-world trial have not been run for this slice.

These checks show source/data validity and compilation only. The mod has not been exercised in a gameplay world. Horizon blocked world launch because the available account does not have an authenticated Microsoft account that owns Minecraft: Java Edition. No machine placement, save/reload, hopper, LAN, dedicated-server, or migration scenario has been recorded. Do not describe current hooks as behavior-tested.

## 5. Outstanding implementation scope

The full inventory remains the root README's feature scope, not just the items listed here. Major gaps include:

- Complete recipe obtainability and material progression, especially missing machine materials, dust/ore reduction paths, alloy ingredients, and farm/compost recipes.
- Arcforge Furnace controls/screen and full input handling; upgrades and richer heat controls.
- Arcforge FE/HU network source parity, capability-driven Auto side behavior, animated FE/HU/item conduit flow, colors/facades, pressurized conduits, machine links, ports, Heat/Lithium cells, and redstone controls. FE/HU pooling and active glow are implemented mod-side for bidirectional components with separate directed links. Item conduits transfer between configured endpoints; fluid conduits use native sided tank detection and transactional transfers. Packet animation, buffers, filters, gas handling, and in-world verification remain absent. All are unverified in-world.
- Energy Cell charge-preserving tier upgrades, GUI/charging slots, orientation-aware behavior and comparator/redstone support. Explosion pickup path needs in-world testing.
- Fluid Tank automated canister slots/general item-fluid transfer, content-preserving tank/canister tier upgrades, and in-world verification of bucket/canister interactions.
- Solar Thermal Array biome/snow fidelity where available, player-facing controls/status, dedicated screen, and in-world verification of heat-port overlays and behavior.
- Induction Furnace XP banking/arbitrary vanilla recipes, Induction/Pressing arrays, processor screens and fuller input/output automation.
- Steam and closed water loops; oil deposits, surveys and extraction; refinery and fuel chains; chemistry, salt/brine/lithium, battery, agriculture and biofuel chains; exact nuclear progression and safety.
- Hammer/Excavator area behavior, Foundry Suit, Long Fall Boots, Jetpacks, Arc Drill/Saw, custom screens, HUD, audio/particles, manuals, and optional questline.
- Save migration, existing-world compatibility, solo/LAN/dedicated parity, performance, and broad source-to-port acceptance checks.

See [not-built.md](status/not-built.md) for the organized backlog and [parity-notes.md](status/parity-notes.md) for current simplifications versus unfinished work.

## 6. Suggested next actions

1. Download the checked `.hmod` artifact if the player wants the content pack. Before installing/running native code, have the player review `api/` and choose whether to accept Horizon's native-code trust prompt. Then build the pinned native bundle for `arcforge-port-test`, restart the disposable profile, and verify item registration before testing Item/Fluid/Pressurized routing and tier rates, sided Auto behavior, pooled FE/HU rates, face direction, thermal transfer, chunk loading, legacy charge migration, and save/reload in-world.
2. Read the relevant upstream Arcforge class/data and current Horizon hook definitions in the pinned checkouts. Reuse/adapt MIT-licensed source where that is the clearest route; retain notices and verify asset-specific licensing.
3. Implement the feature only in `api/` and/or `content/`. Do not touch Horizon source, hook declarations, generated bindings, or API proposals.
4. Add Rust tests only for the feature's deterministic logic/data boundaries. Update root README, implemented/not-built/parity/verification records and the affected system plan in the same work slice.
5. Run the checks listed in [verification.md](status/verification.md) after source/content changes. If no in-world test can be run, state that limitation explicitly and do not upgrade the evidence classification.
6. Keep the user updated during sustained work, and report concrete changed files, behavior, check results, and remaining blockers.

Next: the portable FE Battery hand-transfer slice (`2864301`) and Energy Cell charge-model slice (`2b16b81`) are committed and pushed; their GitHub runs are in progress. Use the newest successful artifact for the next disposable-client review. Battery and Energy Cell state persistence, charge visuals, and player transfer still need runtime verification. Wider in-world checks remain for ore distribution, crafting progression, heat/fluid/energy transfer and persistence, conduit visuals, and machine processing. The port remains incomplete. Direct `horizon-ctl` launch works offline with existing game files; use a disposable world for further checks.

## 7. Documentation map

- [goal.md](goal.md): objective, constraints, and completion bar.
- [../README.md](../README.md): main project overview, implementation snapshot, feature inventory, architecture, phases, and recorded build summary.
- [README.md](README.md): documentation index and status terminology.
- [plan/alpha-charter.md](plan/alpha-charter.md): user-facing alpha principles and scope.
- [plan/progression.md](plan/progression.md): Arcforge chapter order.
- [plan/roadmap.md](plan/roadmap.md): implementation gates and acceptance criteria.
- [plan/integration-boundary.md](plan/integration-boundary.md): pinned API boundary, explicitly no API changes.
- [plan/systems/](plan/systems/): per-system targets, build order, current state, and acceptance checks.
- [status/implemented.md](status/implemented.md): what exists in source/data.
- [status/not-built.md](status/not-built.md): remaining work.
- [status/parity-notes.md](status/parity-notes.md): parity gaps and classifications.
- [status/verification.md](status/verification.md): observed commands/results and missing in-world evidence.
- [assets-and-licensing.md](assets-and-licensing.md) and [`../THIRD_PARTY_NOTICES.md`](../THIRD_PARTY_NOTICES.md): provenance and license notes.

## 8. Documentation maintenance rules

- When code or data changes, update the as-built inventory and the relevant system plan in the same task.
- Update `not-built.md` and `parity-notes.md` when a remaining item changes state or a limitation is discovered.
- Append verification results only after running the exact command/scenario. Keep older results labeled as historical if their bundle/source differs.
- Keep README counts, recipe inventory, asset counts, and pinned revisions consistent with the detailed docs.
- Separate “implemented in source,” “builds,” “loads,” “behavior-tested,” and “parity-complete.”
- Never imply that a failed attempt to launch a game is an in-world test.
