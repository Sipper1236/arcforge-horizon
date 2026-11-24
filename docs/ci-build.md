# Build and compile policy (updated 2026-11-24)
All Arcforge builds and compile verification run through GitHub Actions. Do not invoke the hybrid remote build script or compile locally. Previous hybrid results below are historical records only.

## Current build policy

Latest build-verified client: commit `07b59a7`, run [45325717893](https://github.com/Sipper1236/arcforge-horizon/actions/runs/45325717893). Both the package and Linux client jobs passed. The workflow resolves an available Minecraft 26.3 Nightly for game resources, then uses the Horizon revision pinned in `horizon-mixin.toml` for native compilation. Client ZIP: `/mnt/hdd2tb/arcforge-horizon-builds/arcforge-run-45325717893/arcforge-playable-linux-x86_64-07b59a77bc3fa3adc961b73f88e1e5ccc2ffe4ba/arcforge-linux-x86_64.zip` (47,864,473 bytes; SHA-256 `f2adc4375592e41913fb7a2d17cc9a1e43c572902caf9f47b23da0ad180efe2e`). HMOD SHA-256 `8e88545a63e91e4748ac29a99faa2a2e67b51e6ca5640407fa532c358b3f6eaa`; HNMOD SHA-256 `8e498f96903ec773f62adb5616a236818c2e3cc393b48230aa1ff3d40cdc77b8`; package bundle ZIP SHA-256 `775176063e113cc21948db007b965c81f5718b588555419cbab5d633389020b6`. ZIP integrity and package/client embedded checksums passed. Build success does not establish in-game behavior.

Build only with GitHub Actions. GitHub runners use their normal parallelism; the half-thread cap applies only to local compiling. No local compilation or tests were run for this slice. The workflow checks Rust and content, packages the mod, then builds the pinned Linux client on pushes and manual dispatches.
## Historical hybrid remote build (inactive)
`tools/build_hybrid_client.sh` builds the Rust crate, checks the `.hmod`, packages the `.hnmod`, prepares the pinned Horizon Nightly, and creates an importable Linux client ZIP. Run it from the project root through the hybrid runner:

```sh
remote bash tools/build_hybrid_client.sh
```

It uses the remote Horizon checkout at `/mnt/hdd2tb/horizon-reference`, downloads the Minecraft 26.3 client inputs and checksums from Mojang, and writes the ZIP to `target/arcforge-linux-x86_64.zip` in the remote build workspace. The script disables `RUSTC_WRAPPER` because the distributed cache worker has failed on an incomplete `libc` source mirror. Archive the resulting ZIP and checksums on `/mnt/hdd2tb/arcforge-horizon-builds/`; this route is historical and inactive; use GitHub Actions only. If the local SFTP view is unavailable and the wrapper cannot sync, invoke the same script on the canonical remote checkout through `ssh rx580 'cd /mnt/hdd2tb/arcforge-horizon && bash tools/build_hybrid_client.sh'`. The bundle is build-verified but still needs in-game testing.

## Historical hybrid artifact
`/mnt/hdd2tb/arcforge-horizon-builds/arcforge-hybrid-2026-11-15-steam-turbine-rerun/arcforge-linux-x86_64.zip` — includes the Condenser Array alpha, Horizon `9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0`, Minecraft 26.3, SHA-256 `319b87e1e17a250b558b2c98650e9a4c8391d611e5dd721b660e0ac1d9441221`. The hybrid route is inactive under the current build instruction.

# GitHub Actions build

`.github/workflows/build.yml` builds and checks the Rust mod, builds and checks the Horizon `.hmod`, packages the native `.hnmod`, and uploads a combined mod bundle. On pushes that match the workflow path filters and on manual dispatches, it also builds a Linux x86_64 Horizon client containing the Arcforge native code.

The client job keeps the Horizon source checkout pinned at `9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0` and resolves an available published Nightly as its Minecraft 26.3 resource base. It reads the pinned Minecraft hashes from `research-lock.json`, then downloads the matching version JSON, client jar, and Linux library artifacts directly from Mojang. `tools/fetch_mojang_inputs.py` verifies each artifact's SHA-1 and listed size. These files stay on the ephemeral runner; they are not published or packaged in the artifact. After preparation, the workflow points the CI instance at the pinned source tag so Horizon's native build uses the same revision as the mod. Horizon can prepare the client without downloading the optional sound asset store. No Actions secrets or Microsoft account token are required. Horizon's offline launch option removes the account requirement at play time; it does not remove the need for pinned game files during preparation.

## Artifacts

- `arcforge-<commit>`: `.hmod`, `.hnmod`, checksums, and a combined install ZIP.
- `arcforge-playable-linux-x86_64-<commit>`: Horizon client executable, the native build's required `native-mods.json`, matching mod files, checksums, and installation instructions.

The native mod is compiled against the Horizon source revision pinned in `horizon-mixin.toml`; the workflow uses an available Nightly as its Minecraft 26.3 resource base. Import the client artifact into an installed Horizon version for Minecraft 26.3 and copy the included `native-mods.json` beside `horizon-client` in the imported version folder. Horizon uses that file to activate the compiled native registrations; importing only the executable starts vanilla registries. Then enable `arcforge.hmod` in the target instance for recipes, loot, tags, and other data. The `.hnmod` is the matching native source bundle.

## Re-run the build

Push a change under `api/`, `content/`, `tools/`, or the workflow, or use **Actions → Build Arcforge for Horizon → Run workflow**. The workflow runs the native client job only for pushes and manual dispatches; pull requests still build and upload the reviewable mod packages.

## Historical GitHub client record

Before the hybrid build, the latest GitHub playable client was commit `b6fedb2` in run [43639909488](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43639909488); it predates the Steam Turbine Array and is historical only.


Historical note: the initial native client artifact omitted Horizon's generated

The Pressurized Cylinder/Gas Cartridge slice is commit `dc3987d`. Run [43360699934](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43360699934) passed both the mod-package and Linux native-client jobs. The downloadable artifacts were retrieved and their `SHA256SUMS` verified; the playable-client ZIP is 41,182,118 bytes (outer SHA-256 `2d81a80c632b9053077671e3e6ec8d28068c9870ca6b0e9ec23d5f3cfe16952d`). Commit `1585e3d` added portable FE Batteries; run [43364355794](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43364355794) passed both jobs and its 39.4 MiB Linux client bundle's internal checksums verified. A disposable launch found and reproduced the Cylinder gauge's unsupported integer state property. Fix `f78b881` changed it to an enum supported by the pinned schema; run [43366816469](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43366816469) passed both jobs. Its 41,184,952-byte client bundle (outer SHA-256 `b56bc2bf2c822dd5059e935eb98c544df8d1b6ca45bf109dd296ab60eef65105`) passed internal checksums. The corrected client opened a disposable world and registered/gave the Wrought Battery; energy transfer and persistence still need gameplay validation.
