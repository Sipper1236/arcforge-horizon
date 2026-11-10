# GitHub Actions build

The workflow at `.github/workflows/build.yml` runs on pushes, pull requests, and manual dispatch. It:

1. Checks Rust formatting and compiles the native crate against the pinned Horizon revision.
2. Builds and checks the content `.hmod` with the pinned Horizon content tooling.
3. Packages the native Rust source as Horizon's installable `.hnmod` format, with the Horizon, mixins, and game revisions pinned in its manifest.
4. Uploads both files individually and together in `arcforge-horizon-bundle.zip`, with checksums and install notes.

The workflow artifact is the complete mod distribution: `arcforge.hmod` provides content data and `arcforge.hnmod` provides native item registrations and gameplay code. Install both in Horizon. Horizon then compiles the native mod into its client for the installed Horizon version, OS, and architecture, after the player reviews and trusts the native source. The compiled client itself is instance-specific and is not a portable mod artifact, so CI does not distribute a prebuilt Horizon executable.

The first successful build from the expanded workflow will be linked here. Previous content-only workflow runs are recorded in [verification.md](status/verification.md).
