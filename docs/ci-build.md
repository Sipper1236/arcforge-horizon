# GitHub Actions build

`.github/workflows/build.yml` builds and checks the Rust mod, builds and checks the Horizon `.hmod`, packages the native `.hnmod`, and uploads a combined mod bundle. On pushes to `main` and manual dispatches, it also builds a Linux x86_64 Horizon client containing the Arcforge native code.

The client job reads Horizon's pinned Minecraft hashes from its checked-out `research-lock.json`, then downloads the matching version JSON, client jar, and Linux library artifacts directly from Mojang. `tools/fetch_mojang_inputs.py` verifies each artifact's SHA-1 and listed size. These files stay on the ephemeral runner; they are not published or packaged in the artifact. Horizon can prepare the client without downloading the optional sound asset store. No Actions secrets or Microsoft account token are required. Horizon's offline launch option removes the account requirement at play time; it does not remove the need for pinned game files during preparation.

## Artifacts

- `arcforge-<commit>`: `.hmod`, `.hnmod`, checksums, and a combined install ZIP.
- `arcforge-playable-linux-x86_64-<commit>`: Horizon client executable, the native build's required `native-mods.json`, matching mod files, checksums, and installation instructions.

The client artifact is for the Horizon Nightly commit pinned in the workflow. Import it into that Horizon version and copy the included `native-mods.json` beside `horizon-client` in the imported version folder. Horizon uses that file to activate the compiled native registrations; importing only the executable starts vanilla registries. Then enable `arcforge.hmod` in the target instance for recipes, loot, tags, and other data. The `.hnmod` is the matching native source bundle.

## Re-run the build

Push a change under `api/`, `content/`, `tools/`, or the workflow, or use **Actions → Build Arcforge for Horizon → Run workflow**. The workflow runs the native client job only for pushes and manual dispatches; pull requests still build and upload the reviewable mod packages.

## Current verification

Latest complete playable client: commit `2e8947c` in GitHub run [43632964856](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43632964856), with both package and Linux client jobs passed. Its 41,331,163-byte playable ZIP passed archive integrity and internal SHA256SUMS; outer SHA-256 is `e3d7c79d7324690e736ec70a947c3cc97c33cc7891626969b0aaed6cda17e4d2`. It adds an available-coke versus startup/recipe requirement to the Furnace screen. The matching-client screen check remains pending. The downloaded artifact is at `/tmp/arcforge-run-43632964856-client/arcforge-playable-linux-x86_64-2e8947cbcb8a88ac3cefb7b0f5a4bec6258bbc94/arcforge-linux-x86_64.zip`. See [verification.md](status/verification.md) for internal hashes and earlier verification records.


The initial native client artifact omitted Horizon's generated `native-mods.json`, so importing its executable alone did not activate Arcforge's registries. The workflow now packages that file too. Runtime verification also found a duplicate `arcforge:raw_rubber` item registration; it is now declared once through the shared catalog. Ore feature JSON now uses Horizon 26.3's flattened `worldgen/feature/` format, so ore generation can be read by the pinned build. The imported executable plus its manifest and content pack passed an isolated item-registration smoke check; full progression and machine behavior still need in-world play verification.

The Pressurized Cylinder/Gas Cartridge slice is commit `dc3987d`. Run [43360699934](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43360699934) passed both the mod-package and Linux native-client jobs. The downloadable artifacts were retrieved and their `SHA256SUMS` verified; the playable-client ZIP is 41,182,118 bytes (outer SHA-256 `2d81a80c632b9053077671e3e6ec8d28068c9870ca6b0e9ec23d5f3cfe16952d`). Commit `1585e3d` added portable FE Batteries; run [43364355794](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43364355794) passed both jobs and its 39.4 MiB Linux client bundle's internal checksums verified. A disposable launch found and reproduced the Cylinder gauge's unsupported integer state property. Fix `f78b881` changed it to an enum supported by the pinned schema; run [43366816469](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43366816469) passed both jobs. Its 41,184,952-byte client bundle (outer SHA-256 `b56bc2bf2c822dd5059e935eb98c544df8d1b6ca45bf109dd296ab60eef65105`) passed internal checksums. The corrected client opened a disposable world and registered/gave the Wrought Battery; energy transfer and persistence still need gameplay validation.
