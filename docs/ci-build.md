# GitHub Actions build

`.github/workflows/build.yml` builds and checks the Rust mod, builds and checks the Horizon `.hmod`, packages the native `.hnmod`, and uploads a combined mod bundle. On pushes to `main` and manual dispatches, it also builds a Linux x86_64 Horizon client containing the Arcforge native code.

The client job downloads Minecraft 26.3's version JSON and client jar directly from Mojang's official `piston-meta` and `piston-data` endpoints. It verifies the official version JSON and client SHA-1 pins before using those files as temporary build inputs. The workflow does not publish the Minecraft jar or package it in the artifact. No Actions secrets or Microsoft account token are required. Horizon's offline launch option removes the account requirement at play time; it does not remove the need for version data during preparation.

## Artifacts

- `arcforge-<commit>`: `.hmod`, `.hnmod`, checksums, and a combined install ZIP.
- `arcforge-playable-linux-x86_64-<commit>`: importable Horizon client executable plus the matching mod files and installation instructions.

The client artifact is for the Horizon Nightly commit pinned in the workflow. Import it into that Horizon version, then enable/install the bundled `arcforge.hmod` in the target instance. The executable contains the native Rust code; the `.hmod` supplies recipes, loot, tags, and other data.

## Re-run the build

Push a change under `api/`, `content/`, `tools/`, or the workflow, or use **Actions → Build Arcforge for Horizon → Run workflow**. The workflow runs the native client job only for pushes and manual dispatches; pull requests still build and upload the reviewable mod packages.

## Current verification

The previous workflow run [43329802649](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43329802649) built the mod packages but did not reach client compilation because it expected a private Minecraft archive. The workflow now uses SHA-1-pinned Mojang inputs instead. The same investigation found two content block declaration files duplicated native block declarations, which caused Horizon to reject all native item/block registrations for Arcforge; those redundant declarations were removed. A successful current GitHub run, import, and in-world smoke check are still required before calling the mod playable.
