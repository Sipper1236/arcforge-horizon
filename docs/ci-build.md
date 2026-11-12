# GitHub Actions build

The workflow at `.github/workflows/build.yml` runs on pushes, pull requests, and manual dispatch. It:

1. Checks Rust formatting and compiles the native crate against the pinned Horizon revision.
2. Builds and checks the content `.hmod` with the pinned Horizon content tooling.
3. Packages the native Rust source as Horizon's installable `.hnmod` format, with the Horizon, mixins, and game revisions pinned in its manifest.
4. Uploads both files individually and together in `arcforge-horizon-bundle.zip`, with checksums and install notes.

The workflow artifact is the complete mod distribution: `arcforge.hmod` provides content data and `arcforge.hnmod` provides native item registrations and gameplay code. Install both in Horizon. Horizon then compiles the native mod into its client for the installed Horizon version, OS, and architecture, after the player reviews and trusts the native source. The compiled client itself is instance-specific and is not a portable mod artifact, so CI does not distribute a prebuilt Horizon executable.

Expanded run [43094118580](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43094118580) passed but its `.hnmod` misplaced the API crate relative to `[api] path = "api"`. The packer now preserves the required `mixins/arcforge/api/` layout and validates the archive before upload. The replacement commit is pending CI; do not use the old run’s native bundle. Previous content-only runs are recorded in [verification.md](status/verification.md).
