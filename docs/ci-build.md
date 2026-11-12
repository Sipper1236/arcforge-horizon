# GitHub Actions build

The workflow at `.github/workflows/build.yml` runs formatting, native-crate checks, content-pack build/check, and `.hnmod` packaging on pushes and pull requests. It uploads `arcforge.hmod`, `arcforge.hnmod`, checksums, and a combined ZIP.

On pushes and manual dispatches, a second job also prepares the pinned Horizon Nightly, compiles Arcforge into a Linux x86_64 Horizon client, and uploads an importable client bundle. Horizon requires the matching Minecraft 26.3 client install to prepare a game version. Its account-free launch option does not provide those files, and the workflow does not download game files for an unauthenticated account or publish them in this public repository.

## Provide the build input privately

The client job needs an archive hosted somewhere private and reachable by GitHub Actions. The archive must contain only these two paths, relative to its root:

```text
versions/26.3/26.3.jar
versions/26.3/26.3.json
```

Create it from a Minecraft 26.3 installation you own. For the official `.minecraft` folder on this machine, the command is:

```sh
tar -C "$HOME/.minecraft" -czf horizon-minecraft-26.3.tar.gz \
  versions/26.3/26.3.jar versions/26.3/26.3.json
sha256sum horizon-minecraft-26.3.tar.gz
```

Keep this archive private. Do not add it to the repository, a public release, or the workflow artifacts. Upload it to a private HTTPS object/file store that permits GitHub's runner to download it, then add these repository Actions secrets under **Settings → Secrets and variables → Actions**:

- `HORIZON_MINECRAFT_ARCHIVE_URL`: the private, time-limited download URL.
- `HORIZON_MINECRAFT_ARCHIVE_SHA256`: the SHA-256 printed above.

The workflow checks the archive checksum and accepts only the pinned version JSON and client jar paths. It then runs Horizon's normal `prepare --minecraft ...` command, which validates the files against Horizon's pinned game hashes and fetches the remaining preparation inputs. No Microsoft account token is needed when the owned client files are supplied.

Once both secrets are configured, push a source change or run **Actions → Build Arcforge for Horizon → Run workflow**. The resulting artifact is named `arcforge-playable-linux-x86_64-<commit>`. It contains the Linux client executable and the matching `.hmod` and `.hnmod`; it does not contain Minecraft's client assets. Import the client into the same Horizon Nightly listed in `INSTALL.txt`, then install `arcforge.hmod` in the instance.

## Current verification

Run [43329802649](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43329802649) on commit `6621649` passed formatting, native crate check, content build/check, and mod packaging. Its checked mod artifact is [available here](https://github.com/Sipper1236/arcforge-horizon/actions/runs/43329802649/artifacts/13993968359); I downloaded it and verified both checksums. The separate Linux client job stopped immediately because the required private archive secrets are not configured. A successful Linux client build, import, launch, and in-world gameplay check are still pending.
