#!/usr/bin/env python3
"""Fetch Horizon's pinned Minecraft inputs from Mojang for a Linux CI build."""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import time
import urllib.error
import urllib.request


USER_AGENT = "Arcforge-GitHub-Actions/1.0"


def download(url: str) -> bytes:
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    for attempt in range(3):
        try:
            with urllib.request.urlopen(request, timeout=90) as response:
                return response.read()
        except (urllib.error.URLError, TimeoutError):
            if attempt == 2:
                raise
            time.sleep(attempt + 1)
    raise AssertionError("unreachable")


def checked_download(url: str, sha1: str, size: int | None) -> bytes:
    content = download(url)
    actual = hashlib.sha1(content).hexdigest()
    if actual != sha1:
        raise ValueError(f"SHA-1 mismatch for {url}: expected {sha1}, got {actual}")
    if size is not None and len(content) != size:
        raise ValueError(f"size mismatch for {url}: expected {size}, got {len(content)}")
    return content


def linux_library(library: dict) -> bool:
    rules = library.get("rules")
    if not rules:
        return True
    allowed = False
    for rule in rules:
        os_rule = rule.get("os", {})
        if os_rule and os_rule.get("name") != "linux":
            continue
        if os_rule.get("arch") not in (None, "x86_64"):
            continue
        if os_rule.get("version"):
            raise ValueError(f"unsupported version-specific library rule: {rule}")
        if rule.get("features"):
            raise ValueError(f"unsupported feature-specific library rule: {rule}")
        allowed = rule["action"] == "allow"
    return allowed


def safe_library_path(relative: str) -> Path:
    path = PurePosixPath(relative)
    if path.is_absolute() or ".." in path.parts or not path.parts:
        raise ValueError(f"unsafe library path from Mojang metadata: {relative!r}")
    return Path(*path.parts)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--research-lock", type=Path, required=True)
    parser.add_argument("--destination", type=Path, required=True)
    args = parser.parse_args()

    lock = json.loads(args.research_lock.read_text())
    game = lock["minecraft"]
    version = game["version"]
    manifest_sha1 = game["manifest_sha1"]
    client_sha1 = game["client_sha1"]
    manifest_url = (
        "https://piston-meta.mojang.com/v1/packages/"
        f"{manifest_sha1}/{version}.json"
    )
    manifest_bytes = checked_download(manifest_url, manifest_sha1, None)
    manifest = json.loads(manifest_bytes)
    if manifest.get("id") != version:
        raise ValueError(f"Mojang returned {manifest.get('id')!r}, expected {version!r}")
    client = manifest["downloads"]["client"]
    if client.get("sha1") != client_sha1:
        raise ValueError("Mojang's client SHA-1 differs from Horizon's research lock")
    client_bytes = checked_download(client["url"], client_sha1, client.get("size"))

    version_dir = args.destination / "versions" / version
    version_dir.mkdir(parents=True, exist_ok=True)
    (version_dir / f"{version}.json").write_bytes(manifest_bytes)
    (version_dir / f"{version}.jar").write_bytes(client_bytes)

    artifacts = []
    for library in manifest.get("libraries", []):
        if not linux_library(library):
            continue
        artifact = library.get("downloads", {}).get("artifact")
        if artifact:
            destination = args.destination / "libraries" / safe_library_path(artifact["path"])
            artifacts.append((artifact, destination))

    def fetch_library(entry: tuple[dict, Path]) -> Path:
        artifact, destination = entry
        if destination.is_file() and destination.stat().st_size == artifact.get("size"):
            if hashlib.sha1(destination.read_bytes()).hexdigest() == artifact["sha1"]:
                return destination
        content = checked_download(artifact["url"], artifact["sha1"], artifact.get("size"))
        destination.parent.mkdir(parents=True, exist_ok=True)
        temporary = destination.with_suffix(destination.suffix + ".part")
        temporary.write_bytes(content)
        os.replace(temporary, destination)
        return destination

    with concurrent.futures.ThreadPoolExecutor(max_workers=12) as pool:
        for completed, path in enumerate(pool.map(fetch_library, artifacts), start=1):
            print(f"Verified library {completed}/{len(artifacts)}: {path.relative_to(args.destination)}")

    asset_index = manifest.get("assetIndex")
    if not asset_index:
        raise ValueError("Mojang's version manifest has no assetIndex")
    asset_index_sha1 = asset_index.get("sha1", "")
    if not re.fullmatch(r"[0-9a-f]{40}", asset_index_sha1):
        raise ValueError(f"invalid Mojang asset index SHA-1: {asset_index_sha1!r}")
    # The version manifest bytes were verified against the pinned research-lock SHA-1,
    # so this index hash is transitively authenticated even when Mojang republishes an index.
    asset_index_bytes = checked_download(
        asset_index["url"], asset_index_sha1, asset_index.get("size")
    )
    asset_index_dir = args.destination / "assets" / "indexes"
    asset_index_dir.mkdir(parents=True, exist_ok=True)
    (asset_index_dir / f"{asset_index['id']}.json").write_bytes(asset_index_bytes)
    asset_index_data = json.loads(asset_index_bytes)
    objects = list(asset_index_data.get("objects", {}).values())

    def fetch_asset(asset: dict) -> Path:
        digest = asset.get("hash", "")
        if not re.fullmatch(r"[0-9a-f]{40}", digest):
            raise ValueError(f"invalid Mojang asset hash: {digest!r}")
        destination = args.destination / "assets" / "objects" / digest[:2] / digest
        size = asset.get("size")
        if destination.is_file() and destination.stat().st_size == size:
            if hashlib.sha1(destination.read_bytes()).hexdigest() == digest:
                return destination
        url = f"https://resources.download.minecraft.net/{digest[:2]}/{digest}"
        content = checked_download(url, digest, size)
        destination.parent.mkdir(parents=True, exist_ok=True)
        temporary = destination.with_suffix(destination.suffix + ".part")
        temporary.write_bytes(content)
        os.replace(temporary, destination)
        return destination

    with concurrent.futures.ThreadPoolExecutor(max_workers=16) as pool:
        for completed, path in enumerate(pool.map(fetch_asset, objects), start=1):
            if completed == 1 or completed == len(objects) or completed % 500 == 0:
                print(f"Verified Minecraft asset {completed}/{len(objects)}: {path.name}")

    print(
        f"Fetched Minecraft {version}: verified client jar, version JSON, "
        f"{len(artifacts)} Linux library artifacts, and {len(objects)} assets from Mojang."
    )


if __name__ == "__main__":
    main()
