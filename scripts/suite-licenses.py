#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
# Adapted from bboxy450-term scripts/license-packages.py and scripts/source-notices.py:
# https://github.com/bboxy450/bboxy450-term/tree/main/scripts
# Canonical shared copy: https://github.com/bboxy450/bboxy450-term/blob/main/scripts/suite-licenses.py
"""Portable, offline policy gate for bboxy450 repositories; never compiles Rust."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tomllib

POLICY = "Respectful Slop License 1.0-Draft"
SPDX = "LicenseRef-Respectful-Slop-1.0-Draft"
LICENSE_SHA256 = "07c15c9bc6b7c6ca2b8ef48c57421f4317cacc3ba6b27272dbbbc1c99013daaa"
DOCUMENTS = ("LICENSE", "NOTICE", "ATTRIBUTION.md")
SUFFIXES = {".rs", ".py", ".sh", ".wgsl", ".js", ".mjs", ".cjs", ".ts", ".tsx", ".css", ".html", ".c", ".h", ".cc", ".cpp", ".m", ".mm"}
IGNORED = {".git", ".bboxy", "target", "node_modules", ".venv", "__pycache__"}


def regular(root, name):
    path = root / name
    if path.is_symlink() or not path.is_file() or not path.resolve().is_relative_to(root):
        raise ValueError(f"{name}: required regular file is missing or escapes the repository")
    value = path.read_bytes()
    if not value:
        raise ValueError(f"{name}: required file is empty")
    return value


def digest(value):
    return hashlib.sha256(value).hexdigest()


def sources(root, scopes):
    found = set()
    for scope in scopes:
        path = root / scope
        if path.is_symlink() or not path.resolve().is_relative_to(root):
            raise ValueError(f"{scope}: source scope escapes the repository")
        if path.is_file():
            found.add(path)
        elif path.is_dir():
            for base, dirs, files in os.walk(path, followlinks=False):
                dirs[:] = [d for d in dirs if d not in IGNORED]
                for name in files:
                    candidate = Path(base) / name
                    if candidate.suffix in SUFFIXES:
                        found.add(candidate)
    return sorted(found)


def packages(root):
    manifest = root / "Cargo.toml"
    if not manifest.exists():
        return []
    data = tomllib.loads(regular(root, "Cargo.toml").decode())
    manifests = {manifest} if "package" in data else set()
    for member in data.get("workspace", {}).get("members", []):
        matches = list(root.glob(member))
        if not matches:
            raise ValueError(f"{member}: workspace member is missing")
        manifests.update(p / "Cargo.toml" for p in matches)
    result = []
    for path in sorted(manifests):
        relative = str(path.relative_to(root))
        value = tomllib.loads(regular(root, relative).decode())
        package = value["package"]
        if package.get("license-file") != "LICENSE" or "license" in package:
            raise ValueError(f"{relative}: use explicit local license-file = LICENSE")
        directory = path.parent
        for name in DOCUMENTS:
            content = regular(root, str((directory / name).relative_to(root)))
            if name == "LICENSE" and digest(content) != LICENSE_SHA256:
                raise ValueError(f"{relative}: package LICENSE differs from the suite license")
        result.append((package["name"], path))
    return result


def check(root, package_lists=False, selected=None):
    root = Path(root).resolve()
    config = json.loads(regular(root, "licensing.json"))
    if config.get("version") != 1 or not config.get("project", "").startswith("bboxy450-"):
        raise ValueError("licensing.json: invalid suite policy profile")
    if digest(regular(root, "LICENSE")) != LICENSE_SHA256:
        raise ValueError("LICENSE differs from the agreed suite license")
    source_project = config.get("source_project", config["project"])
    origin = f"https://github.com/bboxy450/{source_project}"
    for name in ("NOTICE", "ATTRIBUTION.md"):
        text = " ".join(regular(root, name).decode().split())
        if POLICY not in text or origin not in text:
            raise ValueError(f"{name}: controlling policy or canonical source credit is missing")
    for name, expected in config.get("preserved", {}).items():
        if digest(regular(root, name)) != expected:
            raise ValueError(f"{name}: preserved upstream/prior license bytes changed")
    shared = config.get("shared", {})
    for name, expected in shared.items():
        if digest(regular(root, name)) != expected:
            raise ValueError(f"{name}: shared license checker differs from its canonical copy")
    count = 0
    for path in sources(root, config["source_roots"]):
        name = str(path.relative_to(root))
        text = regular(root, name).decode()
        project = "bboxy450-term" if name in shared else source_project
        url = f"https://github.com/bboxy450/{project}"
        prefix = "\n".join(text.splitlines()[:14])
        required = ("Copyright", SPDX, url, url + "/blob/main/LICENSE", url + "/blob/main/ATTRIBUTION.md", "Credit copied/adapted/translated reuse accurately")
        if not all(piece in prefix for piece in required):
            raise ValueError(f"{name}: source-attached license/credit notice is missing")
        count += 1
    members = packages(root)
    if selected and selected not in {name for name, _ in members}:
        raise ValueError(f"{selected}: unknown package")
    listed = 0
    if package_lists:
        for name, manifest in members:
            if selected and name != selected:
                continue
            result = subprocess.run(["cargo", "package", "--locked", "--offline", "--allow-dirty", "--list", "--manifest-path", str(manifest), "--package", name], cwd=root, capture_output=True, text=True, timeout=60)
            if result.returncode:
                raise ValueError(f"{name}: cargo package --list failed: {result.stderr.strip()}")
            files = set(result.stdout.splitlines())
            missing = set(DOCUMENTS).difference(files)
            if missing or any(p.startswith(".bboxy/") for p in files):
                raise ValueError(f"{name}: package omits notices {sorted(missing)} or includes private .bboxy state")
            listed += 1
    return {"project": config["project"], "source_files": count, "packages": len(members), "package_lists": listed, "license_sha256": LICENSE_SHA256}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--packages", action="store_true")
    parser.add_argument("--package")
    args = parser.parse_args()
    try:
        print(json.dumps(check(args.root, args.packages, args.package), sort_keys=True))
        return 0
    except (ValueError, OSError, KeyError, TypeError, subprocess.TimeoutExpired) as error:
        print(f"suite-licenses: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
