# Verification record

This file records observed checks only. Re-run checks after Rust or content changes; do not infer current results from an earlier build. For the completion criteria and current resume state, see [goal.md](../goal.md) and [handoff.md](../handoff.md).

## Arcforge Furnace Wrench status — remote compile checkpoint (2026-11-13)

Wrench use on the Arcforge Furnace port now reports formed/unformed state, temperature, remaining coke burn time, and—when metal and additive inputs form a valid recipe—progress, minimum temperature, and coke requirement. `cargo fmt` and the hybrid-remote-build locked `cargo check` passed against the pinned Horizon API; only the existing unused `EnergyStore::transfer_to` warning remains. GitHub run [43542601271](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43542601271) has passed its package/content job; the matching Linux client job is still building. No tests were run; in-world verification remains pending.

## Conduit and Solar block-state schemas — playable client verification (2026-11-14)

Commit `158d858` declares the side properties used by all four tiers of Energy, Thermal, Item, Fluid, and Pressurized Conduits, including `active` where their models select it. It also declares the Solar Array controller/casing/collector `formed`, `part`, `base`, and port properties with matching defaults. Remote `cargo fmt` and locked `cargo check` passed; only the existing unused `EnergyStore::transfer_to` warning remains. GitHub run [43540533024](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43540533024) passed both package and Linux-client jobs. The 2,145,355-byte package ZIP passed archive integrity and both internal SHA256SUMS; its outer SHA-256 is `4ef95526a63e7e3bb8080a3d3bc99fae9cf28a1e8d15567d2543f24313a5382e`. The 41,313,900-byte playable client ZIP passed archive integrity and all four internal SHA256SUMS; its outer SHA-256 is `d0a48b5afa55191a765d43a7af55a7b0a335b4828d805ef56da095d88087ad21`. No tests were run; corrected in-world conduit and Solar renders still need visual confirmation.

## Conduit handbook corrections — playable build verification (2026-11-14)

Commit `c4c5682` updates the Engineer's Handbook to describe the implemented Energy, Thermal, Item, Fluid, and Pressurized Conduit behaviors and clarify which faces the Wrench configures. GitHub run [43537740114](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43537740114) passed both package and Linux client jobs. The 41,305,273-byte playable ZIP passed archive integrity checks and all internal SHA256SUMS; outer SHA-256 is `62ae5e988e4872da9a626e0f2200c6ef217ef7be2540eff35a63db08e31a1f2c`. Internal hashes: `horizon-client` `26aaf8f1b067ab9b3a95adcdf68d42648861bb040d29059e83b2c81e69c79d87`, `native-mods.json` `ca72f4f81f97fcabd27364ad6e7d6e1f92f63407f18dbddb2b7c04a495726112`, `arcforge.hmod` `b33ed57407bd4c2d6a6eb971592b7bf5ba2065936506ba45eebc16e98afbb241`, and `arcforge.hnmod` `177a4ad33884d53620a9b1722d0d61ef898af1eb69500729270c591368d7fc94`. This proves build and package integrity; conduit gameplay still needs in-world verification.

## Single-processor Wrench status readouts — package verification (2026-11-14)

Wrench use on the Arc Crusher, Induction Furnace, Metal Press, and Sifter now shows stored FE and current recipe progress. The displayed duration accounts for machine Speed Upgrades and, for the Sifter, mesh speed. `cargo fmt --check`, `git diff --check`, remote locked `cargo check`, and both GitHub build jobs pass; only the pre-existing unused `EnergyStore::transfer_to` warning remains. No tests were run. The 2,144,302-byte package ZIP passed `unzip -t` and internal SHA256SUMS; `arcforge.hmod` is `a92250ac8a8da4c0d0312d18917133dbd4c8ca6621d29f8a9585c7857f5c76c8`, `arcforge.hnmod` is `177a4ad33884d53620a9b1722d0d61ef898af1eb69500729270c591368d7fc94`, and the outer ZIP is `b2e50e340951e70494e7e571a28dcd4cd4845eb54da6d8907c6a798964cca319`. The 41,304,754-byte playable client ZIP also passed `unzip -t` and internal SHA256SUMS; outer SHA-256 is `847613000cb4f9523846c81f73449e50e3a00aa01477ca295d4aec04046f893c`. Internal hashes: horizon-client `3112ec74adf53e6b509cb65f5838044bc2a84923becafa96331fd7f0bcc17e86`, native-mods.json `ca72f4f81f97fcabd27364ad6e7d6e1f92f63407f18dbddb2b7c04a495726112`, arcforge.hmod `a92250ac8a8da4c0d0312d18917133dbd4c8ca6621d29f8a9585c7857f5c76c8`, arcforge.hnmod `177a4ad33884d53620a9b1722d0d61ef898af1eb69500729270c591368d7fc94`. Commit `8608317` passed both jobs in run [43534354970](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43534354970); in-world feedback verification is pending.

## Crushing Array Wrench progress readout — build verification (2026-11-13)

Using the Wrench on the Array's center casing now reports stored FE and the progress of each of its three lanes in the action bar. Idle lanes are identified explicitly. `cargo fmt --check`, `git diff --check`, and the remote locked `cargo check` pass; only the pre-existing unused `EnergyStore::transfer_to` warning remains. No tests were run. Commit `bd4be83` passed both package and Linux client jobs in run [43532339226](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43532339226). The 2,143,713-byte bundle ZIP passed `unzip -t` and internal SHA256SUMS (`arcforge.hmod`: `a92250ac8a8da4c0d0312d18917133dbd4c8ca6621d29f8a9585c7857f5c76c8`; `arcforge.hnmod`: `0335517149d8b55b6e13d0fcce840e946128f7a216721b15138109a9ae7089aa`; outer ZIP: `c060dc9ef707c942cf55de0e2888a2ebf2d497a493b9e7812a56c52125355de6`). The 41,302,382-byte playable Linux client ZIP also passed `unzip -t` and SHA256SUMS; outer SHA-256: `8aeab8da409a5bb3f638fe8ad6765b446f1a80e2095219aae771c793bbafc93f`. Internal SHA256SUMS: horizon-client `d8aa64d68b58157504b04fc5a79c423dac6526b518cd5f60781edab631429e10`, native-mods.json `596b1b1f963f11cb0a5d93a471abba86902e0dd11c755796dae34d0985885bf1`, arcforge.hmod `a92250ac8a8da4c0d0312d18917133dbd4c8ca6621d29f8a9585c7857f5c76c8`, arcforge.hnmod `0335517149d8b55b6e13d0fcce840e946128f7a216721b15138109a9ae7089aa`. In-world display and port verification are pending.

## Crushing Array default ports — package verification (2026-11-13)

The Array's fixed north-facing default sided capabilities now match Arcforge's default `SideConfig`: top input slots 0–2, bottom output slots 3–8, and south/back energy input. Side item insertion continues to reject non-controller casings. The pinned API's block-wide container and energy hooks cannot inspect a particular casing's position and saved side settings, so Wrench-configurable face modes remain deferred. `remote env CARGO_INCREMENTAL=0 cargo check --locked --manifest-path api/Cargo.toml -j16` passed on `rx580`; the only warning was the pre-existing unused `EnergyStore::transfer_to` method. No tests were run. Commit `ad227c0` passed the package job in run [43530636592](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43530636592). Its 2,143,430-byte bundle ZIP passed `unzip -t` and internal SHA256SUMS; `arcforge.hmod` is `a92250ac8a8da4c0d0312d18917133dbd4c8ca6621d29f8a9585c7857f5c76c8`, `arcforge.hnmod` is `ab949170ae929d0772ee2f2bc03826bd3daab87f7a217ca5421699445f572afc`, and the outer ZIP is `215c498dcd43798824acc882057cced33af50bdd552b79de755e2e3219264f53`. The matching Linux client build and in-world item/energy automation checks are pending.

## Thermoelectric Efficiency Upgrade — package verification (2026-11-13)

Commit `6ace306` adds the item registration, source-shaped recipe, saved per-plant upgrade count, eight-card cap, +6.25% efficiency per card, and action-bar feedback. GitHub run [43514858076](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43514858076) passed both package and Linux native-client jobs. The 41,291,150-byte playable ZIP passed `unzip -t` and internal SHA256SUMS; outer SHA-256: `4d98ef3eb92caf45acce0d8e0afc8b7de384d03e58ffead409ec60c823f2d350`. Internal hashes: horizon-client `629cb56626f3ba4e4c9026e22c3f0c8680e3f585ddf2d03428311b572d3f6529`, native-mods.json `b66ceb37aa8e76b4e40c69afbd38dc90594c274e0cf8d11d5e497fd2bcab0194`, arcforge.hmod `16e3c380e3967532793ee412e8defd2795f370b692873eca07ff751d88c6491a`, arcforge.hnmod `f7639a9f9863196ed39f7418bf651df99d08e1e5e818e5e6ba461becfd9c1bc3`. In-world checks are still outstanding: craft/install the upgrade, verify the eight-card limit and output scaling, and save/reload the plant.

## Combustion Energy Upgrade — local verification (2026-11-13)

Added the source-shaped Energy Upgrade recipe and a saved eight-card install path on the Combustion Plant. Each upgrade increases FE generated per fuel by 12.5%, up to +100%, while preserving the source burn duration. `cargo fmt` and `RUSTC_WRAPPER= cargo check --locked --manifest-path api/Cargo.toml` pass; the only warning is the existing unused `EnergyStore::transfer_to` method. Both affected JSON files parse and `git diff --check` passes. No tests were run. GitHub packaging, the client build, and in-world crafting/output/save-reload checks are pending. FE-cost reduction on consuming machines remains outside this slice.

## Speed Upgrade — build verification (2026-11-13)

The source-shaped recipe, item registration/localization, and saved eight-card installation cover the currently ported source-compatible power and heat machines: Combustion Plant, Arc Crusher, Arc Crushing Array, Induction Furnace, Metal Press, Infuser, Fiberizer, Sifter, Firebox, Geothermal Plant, Fuel Burner, Thermoelectric Plant, Vulcanizer, and Grain Dryer. Speed follows `2^(n/2)`; processing durations use the source rounded time divisor, while per-tick fuel, FE, and HU consumption/output scale with speed. Local `cargo fmt`, `RUSTC_WRAPPER= cargo check --locked --manifest-path api/Cargo.toml`, recipe JSON parsing, and `git diff --check` passed; only the pre-existing unused `EnergyStore::transfer_to` warning remains. No tests were run. GitHub run [43525901170](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43525901170), commit `0fb3e4f`, passed both package and Linux client jobs. Its downloaded 2,143,402-byte mod-package ZIP passed `unzip -t` and SHA256SUMS (`arcforge.hmod`: `a92250ac8a8da4c0d0312d18917133dbd4c8ca6621d29f8a9585c7857f5c76c8`; `arcforge.hnmod`: `2da4cf392a142e27feed7c0344dcc9a85e9f2921f6e4cfca948b5219ff810e05`; outer ZIP: `4e2a37de9aa6709d11a38cce8b859dcc696e204f8a40f299143425136aa7b0cf`). The 41,301,190-byte playable ZIP also passed `unzip -t` and all internal hashes: `horizon-client` (`62804a4fccde33d07c608bb730580d22c9486f748082607adcee69d0bc095676`), `native-mods.json` (`34dd6f17faff6c8e89e7c0c71e455ceda62ca070824ae8262a027981642b0611`), `arcforge.hmod` (`a92250ac8a8da4c0d0312d18917133dbd4c8ca6621d29f8a9585c7857f5c76c8`), and `arcforge.hnmod` (`2da4cf392a142e27feed7c0344dcc9a85e9f2921f6e4cfca948b5219ff810e05`); outer ZIP SHA-256: `8f266ee2f8fadecbe71caabe98361cc8a66903c9b47b15f710d82b788efa3382`. This verifies package integrity and successful client compilation; Speed/Energy Upgrade behavior, limits, and save/reload still need in-world checks.

## Energy Upgrade consumer expansion — build verification (2026-11-13)

The saved eight-card Energy Upgrade now applies to the source-supported Arc Crusher, Arc Crushing Array, Induction Furnace, Metal Press, Infuser, Fiberizer, and Sifter. Consumer FE costs follow `ceil(base × 0.8^n)`; the Sifter applies this after its mesh speed rate. `cargo fmt --check`, `RUSTC_WRAPPER= cargo check --locked --manifest-path api/Cargo.toml`, and `git diff --check` pass. No tests were run. Run [43521767841](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43521767841) passed both jobs for the earlier Combustion-only implementation. Expanded commit `e70ad41` passed both package and Linux client jobs in run [43522478937](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43522478937), but its consumer installation callback incorrectly allowed only Combustion. Commit `0fb3e4f` corrects the callback; its package and matching Linux client ZIPs passed build and checksum verification in run [43525901170](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43525901170). In-world Energy/Speed Upgrade behavior, caps, and persistence remain pending.

## Shared Heat Upgrade — pending verification (2026-11-13)

The source-shaped recipe and local implementation install up to eight Heat Upgrades on Firebox, Geothermal Plant, Fuel Burner, Thermoelectric Plant, Fiberizer, Vulcanizer, and Grain Dryer. The first three heat producers get +12.5% output per upgrade; the processors reduce HU cost by 0.8^n; Thermoelectric applies the source curve improvement capped at 100% before its separate Thermoelectric upgrades. `cargo check` and `cargo fmt --check` pass locally. GitHub run [43518862820](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43518862820), commit `3f3f350`, passed the complete mod-package job. The 2,141,555-byte package ZIP passed `unzip -t`; internal `SHA256SUMS` verified `arcforge.hmod` (`3523658` bytes) and `arcforge.hnmod` (`95532` bytes). Outer ZIP SHA-256: `d5ec5b68e257a5da357d8258d686fecc18d7c2b8356aef71f4a02dd4189c2c18`. Run [43518862820](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43518862820) completed both jobs successfully. The 41,294,198-byte Linux client ZIP passed `unzip -t`; its internal SHA256SUMS verified `horizon-client` (`d377c61a5176119d10c0a917b3ebaf265c82a35b250e7a52ac32f4bf08046e35`), `native-mods.json` (`0c7b896479d92fc36bbc65228660e8437f4a79f6a9c9ebad860764aa777e11e0`), `arcforge.hmod` (`f94d9bc7ce4494523bbc580cb313b923e9d2aebc2236221683c47a342161f2d8`), and `arcforge.hnmod` (`bb1a23f8a5e853cc9a31925a3095a9dc1a923f39403478b892d46da62ed1ddeb`). Outer ZIP SHA-256: `7af2dda8a34bce476b3172093faefbff4e7e7b34918c9b0f00a8470e42a610cc`. This verifies build and archive integrity, not Heat Upgrade behavior, which still needs in-world testing. Check crafting, each machine's effect, ninth-card rejection, and save/reload after importing a matching client.

## Energy Cell feature package builds (2026-11-13)

| Check | Result | What it establishes |
|---|---|---|
| GitHub Actions [43503798037](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43503798037), commit `2864301` | Both package and Linux native-client jobs passed; CI assembled and uploaded the playable-client bundle. | The portable battery hand-transfer source and content build, and the pinned Horizon client integration succeeds. This commit predates the Energy Cell charge models and block-item stack limit; no matching-client runtime behavior is established. |
| GitHub Actions [43505199774](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43505199774), commit `2b16b81` | Both package and Linux native-client jobs passed; CI assembled and uploaded the playable-client bundle. | The five-state Energy Cell block registration compiles into the pinned client. This commit predates the one-item Energy Cell block-item limit. Charge rendering and FE transfer have not been checked in-world. |
| GitHub Actions [43506438439](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43506438439), commit `461fb88` | Both package and Linux native-client jobs passed. Downloaded the 41,288,319-byte playable client ZIP; `unzip -t` passed and SHA256SUMS verified `horizon-client` (`dda7630d…`), `native-mods.json` (`644111a3…`), `arcforge.hmod` (`92984c97…`), and `arcforge.hnmod` (`1211a4e8…`). The ZIP SHA-256 is `34290a097b5e6c085c21deb29d019e89552b3db8c1cfd0b0cd113c8102566121`. | The current source, including Energy Cell one-item block stacks and five charge states, is packaged into the pinned Linux Horizon client. This verifies build and bundle integrity, not in-game Energy Cell behavior. |
| Energy Cell charge visuals and battery hand transfer in-game | Not yet checked on a matching client. | Model rendering, block-state changes, FE transfer, and save/reload remain unverified. |
| Energy Cell Wrench FE readout — GitHub run [43510577279](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43510577279), commit `2513e2d` | Both package and Linux native-client jobs passed. Downloaded the 41,288,224-byte client ZIP; archive integrity and SHA256SUMS passed. Internal hashes: `horizon-client` `aef057b0…`, `native-mods.json` `d9fe0a13…`, `arcforge.hmod` `92984c97…`, `arcforge.hnmod` `56d4851d…`. ZIP SHA-256: `09240cd0193f78a64cb8dcfb721d16614358fc216a2b30c96c0873b3ba0cc389`. | The Wrench FE readout is built into the pinned client. Numeric display and all Energy Cell behavior remain unverified in-world. |

## Portable FE Battery local build (2026-11-13)

| Check | Result | What it establishes |
|---|---|---|
| `cargo fmt --manifest-path api/Cargo.toml` | Passed. | Rust formatting completed. |
| Hybrid remote `remote env CARGO_INCREMENTAL=0 cargo check --locked --manifest-path api/Cargo.toml -j16` | Failed on the distributed worker: `wayland-protocols-plasma` could not open nested Plasma protocol XML files; the job was stopped after stalling. The remote wrapper also reported the known missing `bc` utility in its transfer summary. | This remote attempt does not verify the Rust crate. |
| Local fallback `CARGO_INCREMENTAL=0 cargo check --locked --manifest-path api/Cargo.toml -j6` | Passed: 0 errors; one existing dead-code warning for `EnergyStore::transfer_to`. | Current Rust source, including portable battery registrations, type-checks against the pinned Horizon API. |
| Horizon content `cargo-horizon-mod build` and `check target/horizon-mod/arcforge.hmod` | Passed: 400 recipes, 3,522,365 bytes, SHA-256 `82db30b04bb7b40457e813fb2a717a7cd4065ddf0da964090bc7d15e938aefba` (unsigned). | Updated recipe, locale, and content pack pass the pinned Horizon checker. |
| GitHub Actions [43364355794](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43364355794) | For commit `1585e3dd12aa2237366c36431eb2e9b5871e3680`, formatting, Rust check, content build/check, `.hnmod` packaging, and Linux native-client build/upload all passed. Downloaded 39.4 MiB client ZIP; internal checksums for `horizon-client`, `native-mods.json`, `arcforge.hmod`, and `arcforge.hnmod` passed. | CI produced a verified client bundle, but launching it revealed the separate Cylinder state declaration bug below. |
| In-world battery behavior | Startup and item registration passed in the disposable world described below; energy transfer and persistence were not tested. | The new client can load the mod and register a battery item, but charging and saved charge are still unproven. |

## Cylinder gauge property launch crash (2026-11-13)

| Check | Result | What it establishes |
|---|---|---|
| Launch CI client from run [43364355794](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43364355794) with disposable `arcforge-battery-test` and `/home/rayyan/.local/share/horizon-launcher/versions/imported-2026-11-13-ec7a296/horizon-ctl launch --version imported-2026-11-13-ec7a296 --world "Arcforge Battery Smoke" --seed 42 --creative --wait` | Reproduced startup panic before the control socket/world: `valid Pressurized Cylinder gauge block state: unknown variant int, expected boolean or enum`. The manifest and `.hmod` were verified and installed in the disposable profile. | The new client artifact contains active native code; Cylinder state declaration prevented the registry from loading. |
| Pinned Horizon model inspection | `StateProperty` accepts only `boolean` and `enum`; cylinder models already name `level=0` through `level=4`. | The bug is the unsupported integer declaration in `api/src/lib.rs`, not the five gauge variants or the uploaded manifest. |
| Fix, local Rust check, and CI | Commit `f78b881` declares gauge values as enum `0` through `4`. Local `cargo check` passed, and GitHub run [43366816469](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43366816469) passed both package and Linux client jobs; downloaded bundle checksums passed. | The declaration matches the pinned schema and the corrected client was built. |
| Corrected client runtime smoke | The executable placed in the disposable imported version directory matched the corrected CI bundle (`horizon-client` SHA-256 `f63f30eb2288cdae07b461fa26de4dce1dd686b884d7d51b405bfbadc4b223b0`). With `HORIZON_HOME` set to disposable `arcforge-battery-test`, `horizon-ctl launch --version imported-2026-11-13-ec7a296 --world "Arcforge Battery Smoke" --seed 42 --creative --wait` opened the world without the earlier registry panic. The offline player received `arcforge:wrought_battery`; events reported an item gain and inventory showed count 1. `save` and `quit` both returned `ok:true`. | The fixed client loads the mod in-world, and the portable battery ID is registered and grantable without a Microsoft account. Battery capability values, charging, and persistence across save/reload remain unverified. |

## Wrought Cylinder cartridge transfer runtime smoke (2026-11-13)

| Check | Result | What it establishes |
|---|---|---|
| Corrected GitHub client in disposable `arcforge-battery-test` profile and `Arcforge Battery Smoke` world | Passed. The test used `HORIZON_HOME=/home/rayyan/.local/share/horizon/instances/arcforge-battery-test`, the imported corrected client whose SHA-256 is recorded above, and offline player `Player04792`. The server placed `arcforge:wrought_pressurized_cylinder[level=0]` at `-7 70 2`. A Wrought Gas Cartridge was given 32,000 mB of `arcforge:oxygen`; its saved item component was checked before insertion. The item was inserted into the Cylinder's top slot using server-side block data, then the world advanced. | Confirms runtime registration and operation of one tier's cartridge-to-Cylinder gas transfer, including the existing Horizon container/tick hooks, in an isolated disposable world. |
| Final block data and state | `/data get block -7 70 2` reported an empty cartridge stack in slot 0 and `fluids: [{Amount: 32000, Fluid: "arcforge:oxygen"}]`; `horizon-ctl block -7 70 2` reported `level=2`. `save` and `quit` returned `ok:true`. | The Wrought cartridge drained into the Wrought Cylinder and the five-stage gauge updated to the expected 32,000/64,000 mB level. This does not verify reload persistence, the reverse transfer slot, other tiers/gases, sided automation, or normal player menu access. The test used the server command/data path because the Cylinder does not expose a player-facing menu in this Horizon build. |

## Cartridge-on-Cylinder hand interaction compile check (2026-11-13)

The native mod now handles an Arcforge Gas Cartridge used on a Cylinder through the existing `use_item_on` and fluid-tank hooks. A charged cartridge inserts up to its tier rate; an empty cartridge extracts up to its tier rate. Unsupported gases are rejected, item capacity is bounded, and the held stack is replaced transactionally with fluid rollback if replacement fails. This gives normal players access to Cylinder transfer without relying on a block menu.

| Check | Result | What it establishes |
|---|---|---|
| `cargo fmt --manifest-path api/Cargo.toml` | Passed. | The new handler is formatted. |
| Hybrid remote `remote env CARGO_INCREMENTAL=0 cargo check --locked --manifest-path api/Cargo.toml -j16` | Passed on `rx580` in 5m 24s with 0 errors and the existing dead-code warning for `EnergyStore::transfer_to`. The known `bc: command not found` appeared only in remote-cli's transfer summary after Cargo finished. | Current native source compiles against the pinned Horizon hooks. This does not verify the hand interaction in-game; GitHub client build and a runtime check are pending. No tests were run. |

## Corrected portable battery client artifact (2026-11-13)

GitHub run [43366816469](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43366816469) passed the Rust crate check, 400-recipe Horizon content build/check, `.hnmod` packaging, and Linux native client integration for commit `f78b881c252064bd5b01e7ceb52e1c08aa867e90`. Artifact `arcforge-playable-linux-x86_64-f78b881c252064bd5b01e7ceb52e1c08aa867e90` is 41,184,952 bytes; outer ZIP SHA-256 is `b56bc2bf2c822dd5059e935eb98c544df8d1b6ca45bf109dd296ab60eef65105`. Internal `SHA256SUMS` verifies the executable, manifest, and both mod bundles.

## GitHub native client and disposable gameplay smoke check (2026-11-12)

GitHub run [43340862715](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43340862715) built the checked `.hmod` and `.hnmod` plus a Linux x86_64 Horizon client with Arcforge's native Rust source. The first published client ZIP omitted Horizon's generated `native-mods.json`; importing the executable alone therefore started with vanilla registries. The workflow now packages this required manifest beside the executable and includes it in `SHA256SUMS`. The player must copy it beside `horizon-client` in the imported version folder after `import-build`.

A disposable local native build found a second atomic registry error: `arcforge:raw_rubber` was declared by both the shared item catalog and an extra item entry. It is now declared once. The same runtime check found that this Horizon pin expects ore features in flattened `worldgen/feature/` JSON, with target block IDs as namespaced strings. All 15 features were converted from the older nested `configured_feature` representation.

| Check | Result | What it establishes |
|---|---|---|
| `cargo fmt --check --manifest-path api/Cargo.toml` | Passed. | Rust formatting is clean. |
| `cargo check --locked --manifest-path api/Cargo.toml` | Passed: 0 errors; one existing unused `EnergyStore::transfer_to` warning remains. | Native registrations compile against the pinned API. |
| Horizon `cargo-horizon-mod build` and `check target/horizon-mod/arcforge.hmod` | Passed after worldgen conversion; 3,515,870 bytes, SHA-256 `e710c4199b9d0289c6b110cab7d0f163f3b61b67c4c4b49c1325a7a580630183` (unsigned). | The updated content bundle passes Horizon's checker. |
| Horizon launcher `native-build` on a disposable instance | Passed: `native-fdc87ae-d2eeca097da2`; after the one-line registry fix, incremental rebuild `native-fdc87ae-6b53986d7fb7` passed in 48 seconds. | The full mod source builds through the supported launcher flow. |
| Imported-client smoke with matching `native-mods.json` and `.hmod` | Passed: the world opened; `/give arcforge:wrought_heat_cell 1` and `/give arcforge:steel_ingot 1` succeeded; the inventory contained both. The mod content pack loaded. | The manifest is required for imported-client native registration, and custom item/block IDs now register. |
| Ore worldgen startup | Passed after schema conversion: no invalid feature fields, missing block identifiers, or skipped Arcforge placed features were logged. | Horizon accepts the converted ore features. Deposit frequency and actual in-world ore distribution were not sampled. |
| Broader machine/recipe/heat/energy/fluid behavior | Not verified. | A running world and registered IDs do not establish functional progression or machine parity. |

Horizon still logs unsupported multipart properties for conduit and Solar Thermal Array blockstate definitions. The current smoke check therefore does not establish their visual appearance. The local smoke artifacts and world are isolated under `/tmp/arcforge-smoke-*`; the launcher build cache is under `~/.local/share/arcforge-smoke-launcher`.

## Previous Heat Cells alpha build (2026-11-12)

Added Wrought, Tempered, Hardened, and Arcforged Heat Cells with tier capacities, transfer caps, maximum temperatures, passive leakage, five-level heat visuals, charge-preserving block drops/placement, source-shaped tier recipes, and explosion-aware loot tables. The alpha uses automatic six-side connections; screens, capsules, insulation upgrades, redstone/side controls, and charge-preserving tier upgrades are not implemented. No gameplay world was used for verification.

| Check | Result | What it establishes |
|---|---|---|
| `cargo fmt --manifest-path api/Cargo.toml` | Passed. | Rust sources are formatted. |
| `cargo check --locked --manifest-path api/Cargo.toml` | Passed: 0 errors; the existing unused `EnergyStore::transfer_to` warning remains. | Heat Cell hooks and the native crate compile against the pinned API. |
| Horizon `cargo-horizon-mod build` in `content/` | Passed; 388 recipe JSON files are present. | Heat Cell recipes and loot package with the existing content. |
| Horizon `cargo-horizon-mod check target/horizon-mod/arcforge.hmod` | Passed; 3,517,204 bytes; SHA-256 `40c675764f93faf5953bff98c74605000f2e5e5183743e663ffcd3ba9913e790` (unsigned). | The content bundle passes Horizon's checker. This does not prove runtime item registration or crafting. |
| GitHub Actions run [43329802649](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43329802649) on commit `6621649` | Mod package job passed: formatting, crate check, content build/check, and `.hnmod` packaging. Artifact [13993968359](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43329802649/artifacts/13993968359) downloaded; SHA-256 values verified. The separate client job failed at the former private-input preflight. | Historical run: GitHub produced the source/data mod package but no playable client. The current workflow has removed that preflight and fetches pinned official inputs from Mojang. |
| Rust tests, native client build, and in-world Heat Cell trial | Not run. | Runtime transfer/loss rates, state persistence, item charge round-trip, and visuals remain unverified. |

Commit `6621649` is the prior baseline. The registry and workflow fixes above are pending GitHub verification. The native client and in-world behavior remain outstanding.

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

## Remaining gameplay checks

1. In a new world, sample generated ore distribution and test representative mining drops.
2. Craft the opening machines and verify a simple recipe/power/heat loop.
3. Test Heat Cell transfer and persistence, output backpressure, hopper/handler access, and LAN parity before marking those slices behavior-tested.

## Pressurized Cylinder and Gas Cartridge slice (2026-11-12)

Four Cylinder tiers accept the 22 known Arcforge gas IDs. Their top inventory slot drains a cartridge into the cylinder, while the bottom slot fills a cartridge, bounded by cartridge tier rate and capacity. Four cartridge tiers use capacities of 32,000 / 128,000 / 512,000 / 2,048,000 mB and rates of 2,000 / 8,000 / 32,000 / 128,000 mB per tick. The cylinders store 64,000 / 256,000 / 1,024,000 / 4,096,000 mB, expose a five-level gauge state, and cache gas for ordinary drop and placement hooks. The pinned crafting preparation hook cannot copy cartridge data during tier upgrades, so the source upgrade shapes are regular crafting recipes and lose contents.

| Check | Observed result | What it establishes |
|---|---|---|
| `cargo fmt --manifest-path api/Cargo.toml` | Passed. | Rust formatting is clean. |
| Hybrid remote Rust check | Attempted through `remote`; a distributed sccache worker failed to open files inside `wayland-protocols-plasma`. | Remote compilation did not establish a successful check; the failure is in the remote worker's incomplete dependency files. |
| Local fallback `RUSTC_WRAPPER=sccache CARGO_INCREMENTAL=0 cargo check --locked --manifest-path api/Cargo.toml -j6` | Passed: 0 errors; one existing unused `EnergyStore::transfer_to` warning. | Current native source type-checks against Horizon `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca`. |
| Horizon content build and check | Passed; 396 recipe JSON files; 3,520,889-byte archive; SHA-256 `795bc42c2285ceb9c4363d0476440855c33eede11aea798379d60fd62a49f5bd` (unsigned). | The eight shaped recipes, names, and four cylinder loot tables package and pass Horizon's content checker. |
| Rust tests, native bundle, in-world test | Not run for this slice. | Cylinder registration, slot access, gas transfer, fill-state rendering, drops, placement, save/reload, and multiplayer behavior remain unverified. |

The remote CLI also printed `bc: command not found` in its transfer summary; the Rust build error came from the distributed worker's missing dependency files. Local sccache fallback succeeded. No Horizon API changes were made.

## FE/HU conduit active glow (2026-11-08)

Both pooled schedulers now set the imported `active` blockstate across all network members while the pool contains FE/HU and for up to ten game ticks after the last transfer, matching Arcforge's idle glow interval. Rust formatting and `cargo check --locked --manifest-path api/Cargo.toml` passed after the change; the existing unused `EnergyStore::transfer_to` warning remains. No Rust tests, Horizon content rebuild, native bundle build, or in-world visual check was run for this change. The code compiles, but the rendered glow and tick behavior remain unverified in a Minecraft world.


## Historical install/play diagnosis before the current native build (2026-11-08)

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

## Cylinder visibility follow-up (2026-11-13)

The previous disposable-world check verified 2,000 mB Wrought oxygen hand insertion and extraction, then confirmed the Cylinder still held 2,000 mB after save/reload. To address difficulty reading its level, the five-stage gauge display was doubled in size on all 25 tier/state models. The model now explicitly uses `minecraft:cutout` for its alpha valve texture, preventing the full model from entering the translucent terrain pass. Wrench inspection now reports gas name and exact amount/capacity in the action bar, and successful cartridge transfers also show this readout.

| Check | Observed result | What it establishes |
|---|---|---|
| `cargo fmt --manifest-path api/Cargo.toml` | Passed. | Rust formatting is clean. |
| Local `cargo check --locked --manifest-path api/Cargo.toml -j6` | Passed with zero errors and the existing unused `EnergyStore::transfer_to` warning. Remote SSH timed out and local sccache startup failed, so this used the direct local compiler fallback. | The Wrench and transfer readout code type-checks against the pinned API. |
| Cylinder model JSON edits | 25 models parsed successfully; `git diff --check` passed. | All tiers and five gauge stages carry the larger display and cutout render type. |
| Matching-client render and Wrench readout | GitHub run [43488089381](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43488089381) passed both jobs; all client bundle hashes verified. In the disposable world, Wrench inspection displayed `Cylinder: arcforge:oxygen, 8000/64000 mB (gauge 0/4)`. The user confirmed the larger gauge and opaque adjacent blocks in-game. | The user-visible gauge, readout, and neighboring-block render issue are confirmed fixed on the matching client. |

No Horizon API or engine changes were made. No Rust tests were added or run.

## Portable Energy Item hand transfer (2026-11-13)

A mod-side `use_item_on` path now transfers FE between the four portable Battery tiers or Energy Cell items and the four Energy Cell blocks. Charged items insert, empty items extract; every move is bounded by the item and block capacities and transfer rates. The block entity's saved `energy` and the item's `horizon_energy` custom data are updated with rollback if the carried-stack exchange fails. The action bar reports both stored amounts and the moved FE. To preserve per-item component semantics without changing stack counts, this interaction requires a single item in the held stack. This adapts Arcforge's separate Energy Cell GUI input/output slots to a direct hand interaction within the pinned hooks.

The change uses only existing Horizon hooks. It has not yet passed the next GitHub Actions build or a matching-client runtime/persistence check. No Rust tests were added or run.
