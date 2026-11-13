# GitHub Actions build

`.github/workflows/build.yml` builds and checks the Rust mod, builds and checks the Horizon `.hmod`, packages the native `.hnmod`, and uploads a combined mod bundle. On pushes to `main` and manual dispatches, it also builds a Linux x86_64 Horizon client containing the Arcforge native code.

The client job reads Horizon's pinned Minecraft hashes from its checked-out `research-lock.json`, then downloads the matching version JSON, client jar, and Linux library artifacts directly from Mojang. `tools/fetch_mojang_inputs.py` verifies each artifact's SHA-1 and listed size. These files stay on the ephemeral runner; they are not published or packaged in the artifact. Horizon can prepare the client without downloading the optional sound asset store. No Actions secrets or Microsoft account token are required. Horizon's offline launch option removes the account requirement at play time; it does not remove the need for pinned game files during preparation.

## Artifacts

- `arcforge-<commit>`: `.hmod`, `.hnmod`, checksums, and a combined install ZIP.
- `arcforge-playable-linux-x86_64-<commit>`: importable Horizon client executable plus the matching mod files and installation instructions.

The client artifact is for the Horizon Nightly commit pinned in the workflow. Import it into that Horizon version, then enable/install the bundled `arcforge.hmod` in the target instance. The executable contains the native Rust code; the `.hmod` supplies recipes, loot, tags, and other data.

## Re-run the build

Push a change under `api/`, `content/`, `tools/`, or the workflow, or use **Actions → Build Arcforge for Horizon → Run workflow**. The workflow runs the native client job only for pushes and manual dispatches; pull requests still build and upload the reviewable mod packages.

## Current verification

GitHub run [43339016409](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43339016409) passed preparation after fetching the pinned libraries, then native-build rejected the nested Horizon source checkout's `CLAUDE.md` symlink. The workflow now stages only the mod manifest and API source outside that checkout before invoking native-build. The same investigation found duplicate content declarations that caused Horizon to reject all native item/block registrations for Arcforge; those duplicate files were removed and their block states moved into Rust. A successful new client build, import, and in-world smoke check are still required before calling the mod playable.
