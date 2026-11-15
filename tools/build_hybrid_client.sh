#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

horizon_source="${HORIZON_SOURCE:-/mnt/hdd2tb/horizon-reference}"
version="nightly-2026-11-13-9f42064"
work="$root/target/hybrid-client-work"
launcher_home="$work/launcher"
game_home="$work/game"
minecraft="$work/minecraft"
bundle="$root/target/arcforge-linux-x86_64"
launcher="$horizon_source/launcher/target/debug/horizon-launcher"

export HORIZON_LAUNCHER_HOME="$launcher_home"
export HORIZON_HOME="$game_home"
export HORIZON_LAUNCHER_NO_SELF_UPDATE=1
export CARGO_INCREMENTAL=0
export RUSTC_WRAPPER=

test -f "$horizon_source/research-lock.json"
cargo fmt --check --manifest-path api/Cargo.toml
cargo build --locked --manifest-path api/Cargo.toml -j16

pushd content >/dev/null
cargo run --locked --manifest-path "$horizon_source/crates/cargo-horizon-mod/Cargo.toml" -- build
cargo run --locked --manifest-path "$horizon_source/crates/cargo-horizon-mod/Cargo.toml" -- check target/horizon-mod/arcforge.hmod
popd >/dev/null

cargo build --locked --manifest-path "$horizon_source/launcher/Cargo.toml" --bin horizon-launcher -j16
"$launcher" install "$version"
if ! "$launcher" instance list | grep -q arcforge-ci; then
    "$launcher" instance create arcforge-ci
fi
"$launcher" instance version arcforge-ci "$version"

python3 tools/fetch_mojang_inputs.py \
    --research-lock "$horizon_source/research-lock.json" \
    --destination "$minecraft"
"$launcher" prepare --minecraft "$minecraft" --version "$version"

mkdir -p "$work/native-build/arcforge/api"
cp horizon-mixin.toml "$work/native-build/arcforge/"
cp -a api/Cargo.toml api/Cargo.lock api/src "$work/native-build/arcforge/api/"
"$launcher" native-build "$work/native-build/arcforge" \
    --instance arcforge-ci \
    --version "$version" \
    --profile performance \
    --trust-native-code

revision="$(sed -n 's/^horizon_revision = "\(.*\)"$/\1/p' horizon-mixin.toml)"
revision_short="${revision:0:7}"
native_version_dir="$(find "$launcher_home/versions" -mindepth 1 -maxdepth 1 -type d -name "native-*${revision_short}*" -print -quit)"
test -n "$native_version_dir"
test -x "$native_version_dir/horizon-client"
test -s "$native_version_dir/native-mods.json"

mkdir -p "$bundle"
cp "$native_version_dir/horizon-client" "$bundle/"
cp "$native_version_dir/native-mods.json" "$bundle/"
cp content/target/horizon-mod/arcforge.hmod "$bundle/"
python3 tools/package_native_mod.py \
    --source api \
    --manifest horizon-mixin.toml \
    --license-file LICENSE-Arcforge \
    --notices-file THIRD_PARTY_NOTICES.md \
    --horizon-source "$horizon_source" \
    --output "$bundle/arcforge.hnmod"

cat > "$bundle/INSTALL.txt" <<EOF
Arcforge Horizon Linux x86_64 client

Built for Horizon Nightly 2026-11-13 ($revision), Minecraft 26.3.
Import with: Horizon.AppImage import-build ./horizon-client --version $version --trust-unsigned
The matching native-mods.json must be beside horizon-client in the imported version directory.
Install arcforge.hmod in the instance's horizon-mods/ folder and enable mod.arcforge.
arcforge.hnmod is the matching native source package. Horizon may ask you to trust native code.

Steam Turbine Array gameplay has not yet been verified in game.
EOF

(cd "$bundle" && sha256sum horizon-client native-mods.json arcforge.hmod arcforge.hnmod > SHA256SUMS)
python3 - "$bundle" <<'PY'
from pathlib import Path
from sys import argv
from zipfile import ZIP_DEFLATED, ZipFile

root = Path(argv[1])
with ZipFile(root.with_suffix('.zip'), 'w', ZIP_DEFLATED) as archive:
    for path in sorted(root.iterdir()):
        archive.write(path, path.name)
PY
echo "Built playable client bundle: $bundle.zip"
