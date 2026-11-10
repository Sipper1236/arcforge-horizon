#!/usr/bin/env python3
"""Package the native source folder as Horizon's installable .hnmod format."""

from __future__ import annotations

import argparse
import json
import re
import stat
import tomllib
import zipfile
from pathlib import Path


MAX_FILES = 512
MAX_UNPACKED_BYTES = 64 * 1024 * 1024


def allowed(relative: str) -> bool:
    parts = relative.split("/")
    name = parts[-1]
    in_data = len(parts) >= 2 and (
        parts[0] == "data" or (len(parts) >= 3 and parts[1] == "data")
    )
    text_file = len(parts) <= 2 and (
        name.startswith(("README", "LICENSE", "LICENCE", "COPYING", "NOTICE"))
        or name == ".gitignore"
    )
    return relative.endswith((".rs", ".toml")) or in_data or text_file


def source_files(root: Path) -> list[tuple[str, Path]]:
    result: list[tuple[str, Path]] = []
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root)
        if any(
            part in {"target", "node_modules"} or part.startswith(".")
            for part in relative.parts[:-1]
        ):
            continue
        if path.is_symlink():
            raise SystemExit(f"refusing symlink in native source: {path}")
        if path.is_file():
            name = relative.as_posix()
            if not any(part in {"target", "node_modules"} for part in relative.parts) and allowed(name):
                result.append((name, path))
    return result


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--license-file", type=Path, required=True)
    parser.add_argument("--notices-file", type=Path, required=True)
    parser.add_argument("--horizon-source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    project = tomllib.loads(args.manifest.read_text())
    mod = project["mod"]
    horizon_revision = project["compatibility"]["horizon_revision"]
    if not re.fullmatch(r"[0-9a-f]{40}", horizon_revision):
        raise SystemExit("horizon-mixin.toml must pin a 40-character Horizon revision")

    native_mods = (args.horizon_source / "launcher/src/native_mods.rs").read_text()
    mixins_match = re.search(r'pub const MIXINS_REV: &str = "([0-9a-f]{40})";', native_mods)
    version_source = (args.horizon_source / "crates/horizon-core/src/version.rs").read_text()
    game_match = re.search(r'pub const VERSION_ID: &str = "([^"]+)";', version_source)
    if not mixins_match or not game_match:
        raise SystemExit("could not read pinned Horizon native-mod and game versions")

    files = source_files(args.source)
    if not files:
        raise SystemExit("native source folder contains no packageable files")
    entries: list[tuple[str, bytes]] = [
        (
            "horizon-native.json",
            json.dumps(
                {
                    "schema": 1,
                    "id": mod["id"],
                    "version": mod["version"],
                    "game": game_match.group(1),
                    "horizon": horizon_revision,
                    "mixins": mixins_match.group(1),
                },
                indent=2,
            ).encode()
            + b"\n",
        )
    ]
    prefix = f"mixins/{mod['id']}/"
    entries.append((prefix + "horizon-mixin.toml", args.manifest.read_bytes()))
    entries.extend((prefix + name, path.read_bytes()) for name, path in files)
    entries.append((prefix + "api/LICENSE-Arcforge", args.license_file.read_bytes()))
    entries.append((prefix + "api/NOTICE-THIRD-PARTY.md", args.notices_file.read_bytes()))
    unpacked_size = sum(len(content) for _, content in entries)
    if len(entries) > MAX_FILES:
        raise SystemExit(f"native bundle has {len(entries)} files; Horizon allows {MAX_FILES}")
    if unpacked_size > MAX_UNPACKED_BYTES:
        raise SystemExit(
            f"native bundle expands to {unpacked_size} bytes; Horizon allows {MAX_UNPACKED_BYTES}"
        )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(args.output, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for name, content in entries:
            info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = (stat.S_IFREG | 0o644) << 16
            archive.writestr(info, content)

    print(
        f"Built {args.output}: {len(entries)} files, {unpacked_size} uncompressed bytes; "
        f"Horizon {horizon_revision}, mixins {mixins_match.group(1)}, game {game_match.group(1)}"
    )


if __name__ == "__main__":
    main()
