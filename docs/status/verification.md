# Verification record

This file records observed checks only. Re-run checks after Rust or content changes; do not infer current results from an earlier build. For the completion criteria and current resume state, see [goal.md](../goal.md) and [handoff.md](../handoff.md).

## Conduit tick entity lookup correction (2026-11-08)

Horizon removes the block entity currently ticking from ordinary `Host::mod_block_entity` lookups and passes it separately to the tick hook. Both pooled conduit schedulers now use that supplied entity during network discovery, cache validation, pool reconciliation, and output endpoint collection. This lets the tick path proceed without changing Horizon's API. This corrects a source-level early return; it does not establish that network transfers behave correctly in a running world.

| Check | Result | What it establishes |
|---|---|---|
| `cargo fmt --manifest-path api/Cargo.toml` | Passed. | Formatting was applied to the updated scheduler paths. |
| `cargo check --locked --manifest-path api/Cargo.toml` | Passed: 0 errors; existing unused `EnergyStore::transfer_to` warning remains. | Both schedulers compile with the current tick entity supplied through lookup-dependent helpers. |
| Rust tests, native bundle, in-world transfer trial | Not run. | Runtime FE/HU delivery, directed links, persistence, and multiplayer behavior remain unverified. |

## Invar Dust recipe path (2026-11-08)

Added Arcforge's shapeless Invar Dust recipe: two `c:dusts/iron` plus one `c:dusts/nickel` produce three `arcforge:invar_dust`. The item, Nickel Dust tag, crusher recipe, and smelting routes were already registered. Added `c:dusts/iron` pointing to Arcforge Iron Dust so this recipe works in a standalone Horizon pack while remaining compatible with other tag providers. The recipe's ingredients and output match the pinned source; the source file is under `crafting/` and is retained at the corresponding content path.

| Check | Result | What it establishes |
|---|---|---|
| Recipe tree count | 372 JSON files. | One source crafting recipe was added to the prior 371-recipe content tree. |
| Horizon `cargo-horizon-mod build` in `content/` | Passed; built `target/horizon-mod/arcforge.hmod`. | The new recipe and tag package with the existing content. |
| Horizon `cargo-horizon-mod check target/horizon-mod/arcforge.hmod` | Passed; 3,500,079 bytes; SHA-256 `22f5c584a702b6829fc3cada5307dbd732bc397847b9a55765b606220d9c68a9` (unsigned). | The updated pack passes the pinned Horizon content checker. This does not prove in-game recipe discovery or crafting. |
| Rust tests or in-world crafting | Not run. | The source recipe data is present and packaged; crafting behavior remains unverified in a world. |

## Horizon Nightly update (2026-11-08)

The official [Horizon releases page](https://github.com/horizon-rust/horizon/releases) identifies Nightly 2026-11-08 as full commit `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca` (tag `nightly`). This is the game build, not the separately published launcher releases. The Horizon checkout was clean before update and is now detached at that exact commit. Review of the intervening source changes found content support for explicit item-to-block placement aliases and the `worldgen/feature/` folder name while retaining the old alias; native API generation/dispatch and content hot-switch support also changed. The mod's current hooks compile against the new generated API; no Horizon source or API files were edited. These reviewed changes do not yet eliminate the documented tank/cell or GUI limitations.

| Check | Observed result | What it establishes |
|---|---|---|
| `cargo check --manifest-path api/Cargo.toml` | Passed against exact Horizon revision `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca`, with the existing unused `EnergyStore::transfer_to` warning. Cargo updated `api/Cargo.lock` to this revision and refreshed lockfile dependency resolutions. | The mod's Rust source and current hook use compile against the new Horizon dependency. |
| `cargo fmt --check --manifest-path api/Cargo.toml` | Passed. | Current Rust formatting is clean. |
| `cargo test --locked --manifest-path api/Cargo.toml` | Passed: 85 tests, 0 failed; doc tests 0. | Existing deterministic simulation/data tests pass with the updated API. |
| Horizon `cargo-horizon-mod build` and `check target/horizon-mod/arcforge.hmod` | Passed; 354 recipes; SHA-256 `a7c629dd7148f4ef7c620135610499627c42c7620fe52eebf22727b80f857968` (unsigned). | Content still builds and passes the checker from the updated Horizon checkout. |
| `horizon-launcher native-build /home/rayyan/Projects/arcforge-horizon --trust-native-code` | Not run: shell returned `horizon-launcher: command not found` (exit 127). | No native game bundle build is claimed for the new Nightly yet. |

At this checkpoint, the native bundle had not been built for the updated Nightly. The later direct `horizon-ctl` check below established that an offline creative world can start from existing files; it did not establish that Arcforge items load.

## Historical thermal conduit integration (before pooled scheduler, 2026-11-08)

Four thermal conduit tiers now carry 50/200/800/51,200 HU/t with one tier-rate buffered per block. Saved HU and hottest source temperature relay between adjacent conduits and supported heat consumers; Firebox, Firebox Array, Geothermal, Fuel Burner, and enabled Solar tower faces can feed them. The Wrench cycles each pipe face through Auto/Input/Output/Disabled. Recipes, tier tags, loot, blockstate assets, and runtime registration are present. Transfer remains a mod-side per-segment simulation because Horizon has no thermal capability hook; unlike Arcforge, the implementation does not pool rates network-wide. Upgrade recipes use vanilla shaped crafting because the custom Arcforge tier-upgrade/copy-components behavior is unsupported.

| Check | Result | What it establishes |
|---|---|---|
| `cargo fmt --check --manifest-path api/Cargo.toml` | Passed after integration. | Rust formatting is clean. |
| `cargo check --locked --manifest-path api/Cargo.toml` | Passed: 0 errors; one existing dead-code warning for `EnergyStore::transfer_to`. | The mod-side conduits and producer/consumer adapters compile against the pinned API. |
| Horizon `cargo-horizon-mod build` and `check target/horizon-mod/arcforge.hmod` | Passed; 3,498,816 bytes, 371 recipes; SHA-256 `612aae849a8c18c676a2a9528baa64cab7ac856fd660a64d6f4d7b953ccc7152` (unsigned). | The content bundle packages and passes Horizon's checker. |
| Rust tests, native bundle, in-world trial | Not run. | Runtime behavior, structure placement, save/reload, and multiplayer are not verified. |

## Pooled thermal conduit scheduler (2026-11-08)

Thermal conduit members now share one mod-side pool when every internal face pair permits heat in both directions. The pool rate is the lowest member tier's rate and capacity is member count times that rate, capped to fit saved block-entity integers. Input and output have separate per-game-tick rate budgets. Output-to-Input one-way links stay between separate pools and transfer through the destination's normal input path, so a shared pool does not carry heat against a configured one-way face. Pool HU and hottest carried source temperature are synchronized to member block entities. This implementation has compile evidence only.

| Check | Result | What it establishes |
|---|---|---|
| `cargo fmt --check --manifest-path api/Cargo.toml` | Passed after formatting the scheduler changes. | Rust formatting is clean. |
| `cargo check --locked --manifest-path api/Cargo.toml` | Passed: 0 errors; the existing `EnergyStore::transfer_to` dead-code warning remains. | The network cache, shared input/output budgets, and directed bridge relay compile against the pinned Horizon API. |
| Horizon content build/check | Not rerun because this slice changed Rust only. The last verified Solar overlay archive has 371 recipes and SHA-256 `5951d9ec9405d79370b7c012adf03e852cf13095186d834cfd29b66b36057271`. | No new content artifact is claimed. |
| Rust tests, native bundle, in-world rate/direction/save-load trial | Not run. | Correctness under gameplay ticking, chunk unload/reload, persistence, and multiplayer remains unverified. |

The scheduler pools only bidirectionally connected groups; directed links cross group boundaries and are metered by both source and destination pools. This mod-side topology and endpoint behavior still needs comparison against Arcforge in a running world.

## Pooled FE conduit scheduler (2026-11-08)

FE conduit networks now share stored energy over bidirectionally connected members, set the rate to the lowest member tier, and scale capacity by member count. Producer-to-pipe input and pool-to-consumer output have separate once-per-tick budgets. Directed Output-to-Input links feed a different pool through its normal input path. Supported Arcforge machines receive FE within their registered insertion rates and capacities. Since the previous alpha stored up to 50,000 FE in each block, network discovery retains any existing saved charge above source-sized capacity as temporary capacity; as the excess drains, the pool returns to its normal member-rate capacity. Horizon's static per-block energy capability is reconciled before pooled writes, but direct callers can bypass the aggregate rate and may raise the pool above its normal capacity.

| Check | Result | What it establishes |
|---|---|---|
| `cargo fmt --check --manifest-path api/Cargo.toml` | Passed after formatting the FE scheduler changes. | Rust formatting is clean. |
| `cargo check --locked --manifest-path api/Cargo.toml` | Passed: 0 errors; the existing `EnergyStore::transfer_to` dead-code warning remains. | The FE network cache, shared input/output budgets, directed conduit links, and producer/consumer adapters compile against the pinned Horizon API. |
| Horizon content build/check | Not rerun because this slice changed Rust only. The last verified Solar overlay archive has 371 recipes and SHA-256 `5951d9ec9405d79370b7c012adf03e852cf13095186d834cfd29b66b36057271`. | No new content artifact is claimed. |
| Rust tests, native bundle, in-world rate/direction/migration trial | Not run. | Runtime transfer ordering, old-save charge retention, save/reload, and multiplayer behavior remain unverified. |

Capability-driven FE source/sink discovery, transactional multi-machine sharing, and Arcforge's endpoint backoff behavior are not implemented; the pool uses the current mod's registered Arcforge machine adapters. Horizon's native conduit energy buffer remains a static per-block capability that can bypass the shared rate. The scheduler reconciles direct capability changes before its pooled writes to avoid overwriting the resulting FE, but this does not make external calls network-rate-limited. The network scheduler has compile evidence only.

## Solar heat-port overlay continuation (2026-11-08)

Wrench-enabled Solar output faces now map to `port_<face>` blockstate properties on formed controller, casing, and collector blocks. Saved controller settings are synchronized when the tower forms; teardown clears visual properties. Standard multipart selectors add a steel collar and heat-port plate from the existing upstream textures, and internal tower faces are hidden. This is a static per-block overlay; Arcforge's dynamic model computes the collar depth against the formed multiblock geometry, which this port does not reproduce.

| Check | Result | What it establishes |
|---|---|---|
| `cargo fmt --check --manifest-path api/Cargo.toml` | Passed. | Rust formatting is clean. |
| `cargo check --locked --manifest-path api/Cargo.toml` | Passed: 0 errors; existing dead-code warning for `EnergyStore::transfer_to`. | The port-property synchronization and Wrench path type-check against the pinned Horizon API. |
| Horizon `cargo-horizon-mod build` and `check target/horizon-mod/arcforge.hmod` | Passed; 3,499,535 bytes, 371 recipe JSON files; SHA-256 `5951d9ec9405d79370b7c012adf03e852cf13095186d834cfd29b66b36057271` (unsigned). | The new blockstates, model, and texture references package and pass the content checker. This does not prove an in-world render. |
| Rust tests, native bundle, in-world visual/behavior trial | Not run. | No runtime or rendering behavior is claimed. |

## Fluid Tank item-transfer slice (2026-11-08)

Upstream review of `FluidTankBlockEntity.processContainer` and `FluidTankBlock.useItemOn` confirms item fluid handlers are used in both directions, allow partial `ResourceHandlerUtil.move` transfers, and only commit the transaction if the resulting container fits the output slot. The updated Horizon `native_item_fluids.rs` contract declares a capacity via the native item-fluid hook and stores mod-container contents in `minecraft:custom_data/horizon_fluid`; its generic fluid set is only water/lava. Arcforge's registered Creosote/Latex items declare 1,000 mB capacity, so this slice implements bounded partial right-click transfer for those two existing items in the mod's server-side block-use hook, leaving vanilla buckets on their standard full-bucket path. The mod refuses an item whose stored fluid disagrees with its Arcforge bucket type, refuses mixing at the tank, preserves other custom data, and restores tank fluid when item exchange fails.

| Check | Result | What it establishes |
|---|---|---|
| `cargo fmt --check --manifest-path api/Cargo.toml` | Passed. | Formatting is clean. |
| `cargo check --locked --manifest-path api/Cargo.toml` | Passed against Horizon `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca`; existing unused `EnergyStore::transfer_to` warning. | New block-use hook code compiles against the selected Horizon API. |
| `cargo test --locked --manifest-path api/Cargo.toml` | Passed: 88 tests, 0 failed. New tests cover partial tank→container and container→tank exchanges, fluid mismatch/mixing refusal, preservation of unrelated item custom data, and rollback when the result cannot enter the player's inventory. | Deterministic mod-side exchange logic is unit-tested; does not prove live Horizon inventory interactions. |
| Horizon content build/check | Passed after this slice; 354 recipes; same archive SHA-256 `a7c629dd7148f4ef7c620135610499627c42c7620fe52eebf22727b80f857968`. | The Rust-only gameplay edit did not change content output; the bundle still builds and passes the checker. |
| Horizon native bundle build | Still not run: `horizon-launcher` is unavailable in PATH. | The latest Nightly native bundle cannot yet be claimed built. |

No in-world test was run. Runtime use, inventory rollback under actual Horizon UI semantics, save/reload of partially filled buckets, and LAN/dedicated parity remain unverified.

## Portable canister transfer slice (2026-11-08)

The four Arcforge canister tiers now declare their source tank capacities (16,000 / 64,000 / 256,000 / 1,024,000 mB) and transfer limits (1,000 / 4,000 / 16,000 / 64,000 mB per interaction). Their server-side right-click path uses the existing `UseItemOn` and fluid hooks, exchanges `horizon_fluid` custom data while preserving unrelated data, and caps each transfer by tier rate. Four shaped crafting recipes provide survival access. The pinned `native_prepare_item_craft` wrapper passes player, name, optional table position, and a mutable prepared result only; ingredients are not available, so safe state-copying tier upgrades cannot be implemented there. Upstream's automated tank inventory slots are not present; hand exchange is the supported mod-side alternative.

| Check | Result | What it establishes |
|---|---|---|
| `cargo fmt --check --manifest-path api/Cargo.toml` | Passed. | Rust formatting is clean. |
| `cargo check --locked --manifest-path api/Cargo.toml` | Passed against Horizon `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca`; existing unused `EnergyStore::transfer_to` warning. | Mod code compiles against the pinned hooks. |
| `cargo test --locked --manifest-path api/Cargo.toml` | Historical result: 89 tests passed before the carried-stack replacement correction. The suite has not been executed since the correction. | The current selected-slot replacement path has compile evidence from the prior `cargo check --locked --tests`, but no post-correction test execution is recorded. |
| Horizon `cargo-horizon-mod build` and `check target/horizon-mod/arcforge.hmod` | Passed; 358 recipe JSON files; archive SHA-256 `74bea320e9974fbaa340735607277d807e79179bb16535952c8f7d452ac8469c` (unsigned). | New recipes package and pass Horizon's content checker. |
| Horizon native bundle build | Not available: `horizon-launcher` is absent from PATH. | No native bundle build is claimed. |

No in-world test was run. Hand interaction, inventory rollback semantics in Horizon, canister item persistence, save/reload, hopper automation, and LAN/dedicated behavior remain unverified. The tier-upgrade recipes intentionally lose fluid contents because the prepared-crafting hook cannot inspect ingredients.

## Geothermal Plant slice (2026-11-08)

The source-shaped recipe and loot table were added with the native Geothermal Plant. The implementation accepts lava only, burns the upstream 100 mB batch at 1 mB/t for 80 HU/mB, stores up to 60,000 HU at 1,400°C, and adds 40 HU/t per touching lava source or 16 HU/t per touching magma block. Surroundings are scanned every 20 ticks. Heat moves to currently supported adjacent consumers at up to 100 HU per call. This preserves the source's base generation loop; upgrades, redstone/side controls, its dedicated screen, and thermal conduit integration remain absent.

| Check | Result | What it establishes |
|---|---|---|
| `cargo fmt --check --manifest-path api/Cargo.toml` | Passed. | Rust formatting is clean after this slice. |
| `cargo check --locked --manifest-path api/Cargo.toml` | Passed against Horizon `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca`; one existing dead-code warning for `EnergyStore::transfer_to`. | The native Geothermal Plant implementation compiles against the pinned API. |
| `cargo run --locked --manifest-path /home/rayyan/Projects/horizon-source/crates/cargo-horizon-mod/Cargo.toml -- build` in `content/` | Passed; 359 recipe JSON files. | Content packages after adding the Geothermal Plant recipe. |
| `cargo run --locked --manifest-path /home/rayyan/Projects/horizon-source/crates/cargo-horizon-mod/Cargo.toml -- check target/horizon-mod/arcforge.hmod` in `content/` | Passed; unsigned archive SHA-256 `e08b316db1191164e407b50c0c92c6919f2ad38af3b464c0c4093fe4e23aa3f3`. | Horizon's content checker accepts the bundle. |
| `cargo test` | Not run. | No current unit-test execution or in-world behavior evidence is claimed for this slice. The earlier 89-test result predates the carried-stack correction and remains historical. |

## Gameplay correctness follow-ups (2026-11-08)

Source-level defects found during review were corrected. Vanilla bucket insertion now checks that the complete 1,000 mB fits before changing state. The Geothermal Plant stages the empty bucket and updates its saved tank state in one mod-block-entity transaction, so rollback does not depend on an output face. FE conduit face modes now use Input as receive-only, Output as send-only, Auto as bidirectional, and Disabled as disconnected for both conduit-to-machine and conduit-to-conduit transfers. Crushing Array lane priority rotates each tick so early lanes cannot monopolize scarce FE.

| Check | Result | What it establishes |
|---|---|---|
| `cargo fmt --check --manifest-path api/Cargo.toml` | Passed. | Rust formatting is clean after the corrections. |
| `cargo check --locked --manifest-path api/Cargo.toml` | Passed against Horizon `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca`; one existing dead-code warning for `EnergyStore::transfer_to`. | The corrected mod source compiles against the pinned hooks. |
| Rust tests | Not run. | No test execution is claimed for this change. |
| Horizon content build/check | Not rerun; this change only affects Rust hooks. | The existing 359-recipe content bundle is unchanged. |

There is still no in-world verification of bucket interaction or conduit directionality. Horizon's native bundle build remains unavailable because `horizon-launcher` is not in this shell's PATH.

## Firebox Array and conduit controls (2026-11-08)

The Firebox Array now has its source-shaped controller/casing/Pressure Glass registrations and recipes, bounded 3–7 block multiblock formation, saved bounds, casing teardown/recovery, scaled solid-fuel heat, and direct output to supported adjacent consumers. Liquid/gas fuel, oxygen, flue gas, ports, and the dedicated screen are still unfinished. The Wrench now cycles connected conduit faces independently through all four modes, preserving the opposite face's setting.

| Check | Result | What it establishes |
|---|---|---|
| `cargo fmt --manifest-path api/Cargo.toml` | Passed after these source changes. | The edited Rust source parses and is formatted. It does not establish type correctness. |
| `cargo check`, Rust tests | Not run after this slice. | No compile or test result is claimed for the current source tree. |
| Horizon content build/check | Not run after the three Firebox Array recipes were added. | At the time of that slice, the content tree had 362 recipe JSON files. The last verified archive was the earlier 359-recipe artifact with SHA-256 `e08b316db1191164e407b50c0c92c6919f2ad38af3b464c0c4093fe4e23aa3f3`; the current tree has since grown to 367 recipes. |
| Native bundle or in-world test | Not run. | Runtime registration, multiblock formation, fuel behavior, inventory interaction, and heat delivery have not been verified in Horizon. |

## Solar Thermal Array core slice (2026-11-08)

The Solar Thermal Array helper is wired into the native crate. The controller, casing, collector, receiver tube, and trough mirror are registered; the controller checks the 2×2×4 tower every 40 ticks, models sun/weather/alignment/sky visibility, stores up to 16,000 HU, and transfers heat through Wrench-configurable faces on tower parts to supported adjacent consumers. Breaking a formed component clears the remaining formed state. Biome-specific output, snow detection, source port blocks/visuals, thermal conduit output, the dedicated screen, and gameplay validation remain incomplete. The five Solar crafting recipes are present, bringing the current recipe tree to 367 JSON files.

| Check | Result | What it establishes |
|---|---|---|
| `cargo fmt --check --manifest-path api/Cargo.toml` | Passed after the Wrench heat-port integration. | Rust formatting is clean. |
| `cargo check --locked --manifest-path api/Cargo.toml` | Passed: 0 errors; one existing dead-code warning for `EnergyStore::transfer_to`. | The integrated Solar structure, Wrench settings, and heat transfer type-check against the pinned Horizon API. |
| Rust tests | Not run. | No test execution is claimed. |
| Horizon `cargo-horizon-mod build` and `check target/horizon-mod/arcforge.hmod` | Passed; 3,495,275 bytes, 367 recipe JSON files; SHA-256 `d1528f21500985e92a010b71391ed2c1911b67c8c85f8628f2dd22c237c887d8` (unsigned). | The current content archive packages and passes Horizon's checker, including the corrected Solar handbook instructions. |
| Native bundle or in-world test | Not run. | Runtime registrations, structure behavior, solar output, and heat transfer remain unverified in Horizon. |

## Historical pre-Nightly-update baseline (2026-11-08)

| Check | Observed result | What it establishes |
|---|---|---|
| `cargo run --locked --manifest-path /home/rayyan/Projects/horizon-source/crates/cargo-horizon-mod/Cargo.toml -- build` in `content/` | Passed after adding the Pine Resin slime-ball recipe. | The current 354-recipe content bundle packages successfully. |
| `cargo run --locked --manifest-path /home/rayyan/Projects/horizon-source/crates/cargo-horizon-mod/Cargo.toml -- check target/horizon-mod/arcforge.hmod` | Passed; SHA-256 `a7c629dd7148f4ef7c620135610499627c42c7620fe52eebf22727b80f857968`. | The current archive and manifest pass the pinned Horizon content checker. |
| Static source/content audit for smelting, blasting and dust tags | Passed: 63 vanilla smelting/blasting recipes match source data; all 32 Induction entries match source ingredients, results, cooking times and experience; every Arcforge item id is registered and every referenced common tag exists. | The supported recipes match pinned upstream data semantically; the composite recipe and 13 Arcforge dust tags match source definitions; every Arcforge item id is registered and every referenced common item tag has local data. |
| Static source/content audit for 11 loot tables | Passed: Arc Crusher, Carbonizer, Fiberizer, Firebox, Induction Furnace, Infuser, Metal Press, Sifter, Steel Block, Vulcanizer, and Raw Arcite Block match pinned source byte-for-byte. | The imported loot data preserves upstream drop conditions and modifiers. |
| Static source/content audit for steel equipment | Passed: all eleven steel crafting recipes match pinned source byte-for-byte; the wooden-rod compatibility tag resolves to vanilla sticks. | Survival recipes preserve Arcforge patterns, ingredients, and output counts. |
| Pine Resin slime-ball recipe registration | Passed: the shapeless recipe uses two `arcforge:pine_resin` and one kelp to craft one slime ball; the Rust content-registration test confirms every Arcforge recipe item is registered. | Resin Tap output now has the source crafting use in the early progression path. |
| Horizon nightly `66f3981` with `--dev-mod` | An earlier content bundle loaded and presented a frame. | Resource loading/rendering was observed; this was before the Resin Tap update and does not exercise native hooks or machine behavior. |
| `cargo fmt --check --manifest-path api/Cargo.toml` | Passed on the current source. | Rust formatting is clean. |
| `cargo check --locked --manifest-path api/Cargo.toml` | Passed on the current source with one existing dead-code warning. | The native Rust crate compiles against the pinned Horizon dependencies. |
| `cargo test --locked --manifest-path api/Cargo.toml` | Passed: 85 tests across two suites, including Crushing Array geometry, survival crafting registration, Wrench behavior, machine registrations, Steel equipment stats, Fluid Tank bucket round trips/partial-insertion rollback, Energy Cell item-buffer capacities/rates, charge-data preservation/clamping, one-shot cached-charge consumption, and empty-charge cache replacement. | Rust simulation/data tests pass. Energy Cell cache/item helpers are unit-tested; player-mining, explosion, and placement hook sequencing is not covered by a mock or gameplay world. Fluid Tank interactions have mock-host coverage. |
| Horizon `native-build` for pinned nightly `66f3981` | Passed in 26 seconds (performance, incremental); installed and selected `native-66f3981-6165e0307f55` for the disposable `Arcforge Port Test` instance. | The updated native mod compiled and linked into the full Horizon client using the project's single direct `horizon-client` dependency; no Horizon API/source changes were made. Horizon previously refused to start without a signed-in Microsoft account that owns Minecraft: Java Edition, so the rebuilt hooks have not been exercised in-world. |

The Horizon client logged that optional `libflite.so.1` narration could not load during an earlier render; it still presented a frame. A prior launch attempt for `Arcforge Port Test` returned `Sign in with a Microsoft account that owns Minecraft: Java Edition to play`; the offline-account command also requires an owning account. Thus no gameplay world could be opened for this verification. No machine was placed or exercised in a gameplay scenario. The native `.hnmod` build passed, but its hooks have not yet been exercised in-game. No save/reload, hopper, LAN, dedicated-server, or migration test has been recorded.

## Required next checks

1. Launch the disposable instance with its native build and open a disposable world with the mod enabled.
2. Place machines and verify the opening recipe/power/heat loop.
3. Test save/reload, output backpressure, hopper/handler access, and LAN parity before marking the slice behavior-tested.

## FE/HU conduit active glow (2026-11-08)

Both pooled schedulers now set the imported `active` blockstate across all network members while the pool contains FE/HU and for up to ten game ticks after the last transfer, matching Arcforge's idle glow interval. Rust formatting and `cargo check --locked --manifest-path api/Cargo.toml` passed after the change; the existing unused `EnergyStore::transfer_to` warning remains. No Rust tests, Horizon content rebuild, native bundle build, or in-world visual check was run for this change. The code compiles, but the rendered glow and tick behavior remain unverified in a Minecraft world.


## Current install/play readiness snapshot (2026-11-08)

The user reported that no Arcforge items appear in the game. The disposable `arcforge-port-test` instance has `horizon-mods/arcforge.hmod` installed and `horizon-mods.txt` enables `mod.arcforge:true`; its archive is still the older 380-recipe build (SHA-256 `6fd8cc039fc035b0145108daa01a2931a93bed6ebd845649124008ee93622d80`); the new checked 384-recipe archive has not been copied into the instance. The `.hmod` is content only: registered items and blocks come from the native Rust bundle. That profile still points at stale `native-66f3981-6165e0307f55`, built against Horizon `66f3981f522b24c250b21523a70080965b940424`, before current source edits and the pinned Horizon `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca`. A direct `/give @s arcforge:arc_crusher` against that build returned `Unknown item 'arcforge:arc_crusher'`; this confirms the reported symptom. Current source registers machine blocks with `item: true`, and materials through the native item-registration hook. The defect is that this current source has not been built into the selected game bundle.

The stale build also logged duplicate `arcforge:carbon_fiber` registration. The duplicate shared-catalog entry has been removed from source; only the Fiberizer material registration remains. Rust formatting/type-check passed after the fix. The previous slice added four Item Conduit tiers, Wrench face handling, tier-rate-limited routing, recipes, and loot. The current slice added four Fluid Conduit tiers, transactional network routing, sided-capability Auto faces, Wrench overrides, recipes, tags, and loot. The current source compiled with `cargo check --locked`; the `.hmod` built and passed Horizon's checker; no Rust tests were run. The updated archive was copied into the disposable profile. Running `Horizon.AppImage native-build` without the trust flag exits with: `Native mods run with full access to this computer. Read their source, then pass --trust-native-code.` The player must review and trust the source personally; the agent must not supply that consent. After that action, rebuild with `/home/rayyan/Applications/Horizon.AppImage native-build /home/rayyan/Projects/arcforge-horizon --instance arcforge-port-test --trust-native-code`, then restart the instance so both the native bundle and `.hmod` load together. The direct control binary is `/home/rayyan/.local/share/horizon-launcher/versions/nightly-2026-11-05-66f3981/horizon-ctl`; it is not on `PATH`, and no game is currently attached to its socket. The account-owner gate on Horizon's MCP/CLI Play path does not apply to the GUI developer offline setting or direct `horizon-ctl` launch.

## Item Conduit first pass (2026-11-08)

Four Item Conduit tiers now register with source blockstates, loot tables, and source-pattern crafting recipes. The mod discovers connected Item Conduit blocks, uses the slowest tier's operation limit and interval (8/20, 16/10, 32/5, 64/2 items/ticks), and attempts direct transfers from configured Output faces to configured Input faces through Horizon's `move_items` transaction helper. Wrench face state is saved in each conduit block entity. Connected topology is cached; placement, breaking, Wrench edits, and changed local edges invalidate the cache. The current simplification has no traveling packet animation, conduit-held item buffer, item filters, or automatic detection of generic inventory input/output capability; players configure endpoints manually. Tier upgrades use vanilla shaped crafting, so Wrench face settings are not copied. An item transfer commits only through Horizon's source/destination helper, which leaves refused items at the source.

| Check | Observed result | What it establishes |
|---|---|---|
| `cargo fmt --manifest-path api/Cargo.toml` | Passed. | Rust formatting is clean. |
| `cargo check --locked --manifest-path api/Cargo.toml` | Passed: 0 errors; existing unused `EnergyStore::transfer_to` warning. | Native mod source and the item transfer API compile against the pinned Horizon dependency. |
| Horizon content `build` and `check target/horizon-mod/arcforge.hmod` | Passed; 380 recipe JSON files; archive SHA-256 `6fd8cc039fc035b0145108daa01a2931a93bed6ebd845649124008ee93622d80` (unsigned). | Item and Fluid Conduit recipes, tags, and loot package and pass Horizon's content checker. |
| Copy into disposable instance | Passed; source and installed `.hmod` SHA-256 match; `mod.arcforge:true` remains enabled. | The current content archive is present in `arcforge-port-test`; this does not register native items. |
| Rust tests, native bundle build, and in-world Item Conduit scenario | Not run. | No runtime registration, item transfer, rate, endpoint-mode, persistence, or visual behavior is claimed. |


## Fluid Conduit first pass (2026-11-08)

Four Fluid Conduit tiers are registered and have their source recipes, loot tables, and item tags. Connected networks use Horizon's transactional `move_fluids` helper, cap total routing at the lowest tier's 200/800/3,200/12,800 mB/t, and keep one selected fluid type until its accessible sources run dry. Auto faces query Horizon's sided fluid-tank capability for drain/fill roles; Wrench overrides cycle Auto, Input, Output, and Disabled. Topology is cached and invalidated on conduit placement/breaking, endpoint placement, and Wrench changes. The pass has no conduit buffer, filters, gas handling, or animated fill renderer. No Horizon API or engine changes were made.

| Check | Observed result | What it establishes |
|---|---|---|
| `cargo fmt --manifest-path api/Cargo.toml` | Passed. | New Rust source is formatted. |
| `cargo check --locked --manifest-path api/Cargo.toml` | Passed: 0 errors; existing unused `EnergyStore::transfer_to` warning. | Fluid conduit code compiles against the pinned API, including sided fluid reads and transactional moves. |
| Horizon content `build` and `check target/horizon-mod/arcforge.hmod` | Passed; 380 recipe JSON files; archive SHA-256 `6fd8cc039fc035b0145108daa01a2931a93bed6ebd845649124008ee93622d80` (unsigned). | New conduit recipes, tags, and loot are packaged and accepted by Horizon's content checker. |
| Copy into disposable instance | Passed; source/installed `.hmod` hashes match and `mod.arcforge:true` remains enabled. | The updated content archive is installed; this alone does not load native registrations. |
| Native bundle rebuild and in-world fluid scenario | Not run. Horizon requires the player to review the native source and supply `--trust-native-code`. | Runtime registration, Auto roles, tier rates, backpressure, persistence, and rendering are not yet verified. |

## Pressurized Conduit and GitHub Actions build (2026-11-09)

Added four Pressurized Conduit tiers to the existing fluid conduit router. Gas conduits use distinct network kinds at 400/1,600/6,400/25,600 mB/t; liquid networks reject the 22 known Arcforge gas IDs. The explicit list matches all 22 keys in the pinned Arcforge `data_maps/fluid/gas_properties.json`. Horizon's pinned API does not expose gas density or arbitrary fluid-tag classification, so unknown third-party gas IDs remain unsupported. No Horizon API changes were made.

| Check | Observed result | What it establishes |
|---|---|---|
| `cargo fmt --manifest-path api/Cargo.toml` | Passed. | Rust source is formatted. |
| `cargo check --locked --manifest-path api/Cargo.toml` | Passed: 0 errors; one existing unused `EnergyStore::transfer_to` warning. | Pressurized Conduit changes compile against the pinned Horizon API. |
| Horizon content build and check | Passed; 384 recipe JSON files; archive is 3,514,071 bytes with SHA-256 `1320a906595b3075ae77dafcab1c8a23f87004386f592267611131bacff287e0` (unsigned). | The conduit recipes, tags, loot, and gas tag are packaged and accepted. |
| GitHub Actions run [42679842491](https://github.com/Sipper1236/arcforge-horizon/actions/runs/42679842491) | Passed on 2026-11-09: formatting, native crate check, content build/check, and artifact upload. | CI reproduces the source check and produces the content archive. |
| Uploaded content artifact | [Download `arcforge-content-ac5c6804af5f9dffc70bf25e9b6b69d893d51cfa`](https://github.com/Sipper1236/arcforge-horizon/actions/runs/42679842491/artifacts/13701787845); artifact SHA-256 `f4c4696649d330115ac46e65c51ef661fb0a3daf059dd907e0f9bb4dcbf4fe0f`. | The artifact is a ZIP containing the checked `.hmod`; it is not the native client bundle. |
| Native bundle and in-world test | Not run. | Horizon requires the player to review and choose `--trust-native-code`; native registration, routing, pressure behavior, persistence, and gameplay remain unverified. |

The workflow and public source are at [Sipper1236/arcforge-horizon](https://github.com/Sipper1236/arcforge-horizon). The Actions artifact is only the content `.hmod`. The native crate was type-checked in CI, but Horizon builds native code into an instance-specific client bundle; the player's trust decision is still required before that build. The disposable profile still has the earlier 380-recipe `.hmod` and stale native bundle `native-66f3981-6165e0307f55`. Rust tests were not run.

## Current playable-build checkpoint (2026-11-10)

| Check | Observed result | What it establishes |
|---|---|---|
| Current checked `.hmod` copied to `arcforge-port-test` | Passed; installed archive SHA-256 `1320a906595b3075ae77dafcab1c8a23f87004386f592267611131bacff287e0`, matching the current checked archive; `mod.arcforge:true` remains enabled. Previous archive backed up at `~/.local/share/horizon/instances/arcforge-port-test/arcforge.hmod.before-384`. | Current content data is staged in the disposable profile. It does not provide native item or block registrations. |
| Horizon `native-build` without trust option | Refused: `Native mods run with full access to this computer. Read their source, then pass --trust-native-code.` | No current native bundle was built and no consent was given on the player's behalf. |
| Disposable instance native bundle | Still `native-66f3981-6165e0307f55`, older than the pinned Horizon Nightly 2026-11-08 source and current native mod source. | Current source has not loaded in-game; the mod is not presently playable. |
| Next native build, after the player personally reviews and trusts the source | `/home/rayyan/Applications/Horizon.AppImage native-build /home/rayyan/Projects/arcforge-horizon --instance arcforge-port-test --trust-native-code` | Horizon's explicit trust gate is the remaining build prerequisite. Afterward restart the instance and verify item registration and a basic progression path in a disposable world. |

Source for review: [native crate](../../api/src/lib.rs), [Horizon pin](../../horizon-mixin.toml), and the full public repository at [Sipper1236/arcforge-horizon](https://github.com/Sipper1236/arcforge-horizon). No Rust tests or in-world runtime checks have been run for the current state. The user asked to make a playable build; their request does not itself waive Horizon's specific requirement that the player review and trust the full-access native code.

## GitHub build expansion and pinned client build (2026-11-12)

The player explicitly authorized Horizon's `--trust-native-code` option. The initial native build accidentally used the test instance's older `66f3981` Nightly and correctly failed because the mod targets `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca`. A retry explicitly selecting the pinned `nightly-2026-11-08-fdc87ae` downloaded and unpacked the correct Horizon source, but failed before compilation: Cargo could not fetch the `gif` crate from crates.io after repeated SSL resets and transfer timeouts. No native client bundle was produced or installed.

The workflow now builds and uploads the complete mod distribution: the checked content `.hmod`, the native source `.hnmod` with pinned Horizon/mixins/game metadata, and a combined install ZIP with checksums and instructions. Run [43319234022](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43319234022) passed and uploaded those artifacts. Its separate Linux client job failed before native compilation because Horizon requires its Nightly to be prepared from Minecraft 26.3 files. The corrected `.hnmod` nests the crate under the declared `mixins/arcforge/api/` path, includes the mixin manifest and license/notice files, and is checked before upload; its previously verified 15-entry archive SHA-256 is `987700a3e60fd32ea0bbe6e5ebc990dbc5878d71fb71f54f2a8844af45817bc3`. The workflow now supports a private, checksum-verified Minecraft archive and Horizon preparation, but the required Actions secrets are not configured yet. See the latest checkpoint below.

## GitHub client-build checkpoint (2026-11-12)

| Check | Observed result | What it establishes |
|---|---|---|
| GitHub Actions run [43322958666](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43322958666), `build` job | Passed formatting, native crate check, content build/check, `.hnmod` packaging, and artifact upload. Downloaded artifact [13992230701](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43322958666/artifacts/13992230701); `SHA256SUMS` verified after download. | The checked source/data distribution builds in CI. `arcforge.hmod` SHA-256 is `1320a906595b3075ae77dafcab1c8a23f87004386f592267611131bacff287e0`; `arcforge.hnmod` is `987700a3e60fd32ea0bbe6e5ebc990dbc5878d71fb71f54f2a8844af45817bc3`. |
| Same run, `build-linux-client` job | Stopped at the early preflight because the two private Minecraft archive secrets are not configured. No launcher compile or native client build ran in this run. | No importable native client was produced. The previous run [43319234022](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43319234022) reached `native-build` and confirmed Horizon reports “version is not prepared yet” without game files. |
| Local Minecraft 26.3 input | `26.3.jar` SHA-1 matches Horizon's pinned `research-lock.json`; local `26.3.json` matches one of the explicitly accepted republished manifests. The two files are packaged at `/tmp/arcforge-ci-input/horizon-minecraft-26.3.tar.gz`, SHA-256 `f7dc9f49c5d8f9a9651865c21093035361c5df13d0d9f05e05f6c2325502fdba`. | A pin-compatible input exists locally and has the archive layout expected by CI. It has not been uploaded to private storage or sent to GitHub Actions. |
| Workflow and setup guide | The workflow expects `HORIZON_MINECRAFT_ARCHIVE_URL` and `HORIZON_MINECRAFT_ARCHIVE_SHA256`, checks the archive hash and exact file layout, then runs Horizon's normal prepare and native-build commands. `gh secret list` showed no configured secrets. | The code path is ready, but the owner must place the archive in private HTTPS storage and configure both repository secrets before the client build can run. |

The account-free launch option does not prepare missing game files. Keep the client input in private storage; do not commit it or publish it as a workflow artifact. After configuring both secrets and rerunning Actions, inspect the client bundle, import it into the matching Horizon Nightly, launch an instance, and verify Arcforge registrations and progression in-world before calling the mod playable.
