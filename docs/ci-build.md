# GitHub Actions build

The workflow at `.github/workflows/build.yml` runs on pushes, pull requests, and manual dispatch. It checks Rust formatting, type-checks the native mod crate against the pinned Horizon revision, builds and checks the content `.hmod` with Horizon's pinned content tooling, and uploads the checked `.hmod` as a workflow artifact.

The successful first run is [42678494547](https://github.com/Sipper1236/arcforge-horizon/actions/runs/42678494547); it uploaded [the checked content artifact](https://github.com/Sipper1236/arcforge-horizon/actions/runs/42678494547/artifacts/13701378349). The artifact is the content pack only. The native Rust source also compiles in CI, but Horizon integrates it into a client bundle for a particular installed game instance. That native build is performed by Horizon's launcher and requires the player to review and explicitly trust the source; CI does not make that choice or produce a playable instance bundle.

Download `arcforge-content-<commit-sha>` from the successful Actions run to get the `.hmod`. To play the native mod, build the Rust source with Horizon's native-build command for the target instance after reviewing the source and accepting Horizon's native-code warning.
