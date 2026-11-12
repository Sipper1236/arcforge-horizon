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
    api_path = project["api"]["path"]
    api_parts = Path(api_path)
    if api_parts.is_absolute() or ".." in api_parts.parts or not api_parts.parts:
        raise SystemExit("[api].path must be a safe relative folder")
    horizon_revision = project["compatibility"]["horizon_revision"]
    if not re.fullmatch(r"[0-9a-f]{40}", horizon_revision):
        raise SystemExit("horizon-mixin.toml must pin a 40-character Horizon revision")

    native_mods = (args.horizon_source / "launcher/src/native_mods.rs").read_text()
    mixins_match = re.search(r'pub const MIXINS_REV: &str = "([0-9a-f]{40})";', native_mods)
    version_source = (args.horizon_source / "crates/horizon-core/src/version.rs").read_text()
    game_match = re.search(r'pub const VERSION_ID: &str = "([^"]+)";', version_source)
    if not mixins_match or not game_match:
        raise SystemExit("could not read pinned Horizon native-mod and game versions")

    if not (args.source / "Cargo.toml").is_file():
        raise SystemExit(f"native crate is missing Cargo.toml: {args.source}")
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
    entries.extend((prefix + api_path.rstrip("/") + "/" + name, path.read_bytes()) for name, path in files)
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

    with zipfile.ZipFile(args.output) as archive:
        names = archive.namelist()
        if len(names) != len(set(names)) or len(names) > MAX_FILES:
            raise SystemExit("generated native bundle has duplicate entries or exceeds Horizon's file limit")
        manifest = json.loads(archive.read("horizon-native.json"))
        mixin = tomllib.loads(archive.read(prefix + "horizon-mixin.toml").decode())
        expected = {
            "schema": 1,
            "id": mod["id"],
            "version": mod["version"],
            "game": game_match.group(1),
            "horizon": horizon_revision,
            "mixins": mixins_match.group(1),
        }
        if manifest != expected:
            raise SystemExit("generated native manifest does not match the pinned project")
        if (
            mixin["mod"]["id"] != manifest["id"]
            or mixin["mod"]["version"] != manifest["version"]
            or mixin["compatibility"]["horizon_revision"] != manifest["horizon"]
            or not archive.getinfo(prefix + api_path.rstrip("/") + "/Cargo.toml")
        ):
            raise SystemExit("generated native bundle layout does not match horizon-mixin.toml")
        unpacked = 0
        for name in names:
            if name == "horizon-native.json":
                continue
            rest = name.removeprefix(prefix)
            if rest == name or not allowed(rest):
                raise SystemExit(f"generated native bundle contains a disallowed path: {name}")
            unpacked += archive.getinfo(name).file_size
        if unpacked > MAX_UNPACKED_BYTES:
            raise SystemExit("generated native bundle exceeds Horizon's unpacked-size limit")

    print(
        f"Built {args.output}: {len(entries)} files, {unpacked_size} uncompressed bytes; "
        f"Horizon {horizon_revision}, mixins {mixins_match.group(1)}, game {game_match.group(1)}"
    )


if __name__ == "__main__":
    main()
