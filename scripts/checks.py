# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
# License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

"""Unattended source headers, credential shapes and d4949 dependency/provenance gates."""
from pathlib import Path
import json
import re
import subprocess
import sys
import tomllib
ROOT=Path(__file__).resolve().parents[1]
HEADER=(ROOT/"ATTRIBUTION.md").read_text().split("```\n")[1].split("```")[0].strip().splitlines()

def source_paths(root=ROOT):
    return sorted(p for base in ["src","tests","scripts","verification"] for p in (root/base).rglob("*") if p.is_file() and "evidence" not in p.parts and "__pycache__" not in p.parts)

def headers():
    paths=source_paths()+[ROOT/"Cargo.toml",ROOT/"deny.toml",ROOT/"Makefile"]
    for path in paths:
        if path.suffix not in {".rs",".py",".sh",".toml",".hex"} and path.name!="Makefile": continue
        lines=path.read_text().splitlines()
        if lines and lines[0].startswith("#!/"): lines=lines[1:]
        expected=HEADER if path.suffix==".rs" else [s.replace("//","#",1) for s in HEADER]
        if lines[:len(expected)]!=expected: raise ValueError("source header differs: "+str(path.relative_to(ROOT)))
    print("headers: PASS")

def scan_secrets(paths):
    patterns=[re.compile(r"sk-(?:or-v1-)?[A-Za-z0-9_-]{20,}"),re.compile(r"(?i)(?:api_key|key|token)\s*[:=]\s*[\x22\x27]?[0-9a-f]{32,}"),re.compile(r"-----BEGIN (?:OPENSSH |RSA |EC )?PRIVATE KEY-----"),re.compile(r"(?:ghp_|github_pat_)[A-Za-z0-9_]{30,}")]
    for path in paths:
        text=path.read_bytes().decode("latin-1")
        if any(p.search(text) for p in patterns): raise ValueError("credential shape in "+path.name+" (value suppressed)")

def secrets():
    # Include new source before staging; never scan other workers' real log inputs.
    tracked=subprocess.check_output(["git","ls-files","-z"],cwd=ROOT).decode().split("\0")
    paths={ROOT/p for p in tracked if p and (ROOT/p).is_file()}
    paths.update(source_paths())
    paths.update(ROOT/p for p in ["Cargo.toml","Cargo.lock","Makefile","deny.toml"])
    scan_secrets(sorted(paths))
    print("secrets: PASS")

def normalize(name): return name.lower().replace("_","-")
def policy(): return tomllib.loads((ROOT/"deny.toml").read_text())
def denied(name,source=""):
    rules=policy()
    return normalize(name) in {normalize(n) for n in rules["banned_packages"]} or any(n.lower() in source.lower() for n in rules["banned_origins"])

def dependency_records(data):
    for package in data["packages"]:
        if denied(package["name"],package.get("source") or ""): raise ValueError("d4949 denied dependency: "+package["name"])

def manifest_dependencies(path):
    data=tomllib.loads(path.read_text())
    def visit(value):
        if not isinstance(value,dict): return
        for key,entry in value.items():
            if key in {"dependencies","dev-dependencies","build-dependencies"}:
                for name,dep in entry.items():
                    package=dep.get("package",name) if isinstance(dep,dict) else name
                    source=dep.get("git","") if isinstance(dep,dict) else ""
                    if denied(package,source): raise ValueError("d4949 denied manifest dependency: "+package)
            else: visit(entry)
    visit(data)

def copied_source(path):
    text=path.read_text()
    # Reject declared copied/adapted/translated origins, including a wrapped URL.
    # Unattributed plagiarism is a source-review obligation, not a solved detector.
    lines=text.splitlines()
    for index,line in enumerate(lines):
        if re.search(r"(?i)(?:copied|adapted|translated) from",line):
            origin=" ".join(lines[index:index+3]).lower()
            if any(n.lower() in origin for n in policy()["banned_origins"]) or any(re.search(r"(?<![a-z0-9])"+re.escape(n)+r"(?![a-z0-9])",origin) for n in policy()["banned_packages"]):
                raise ValueError("d4949 copied origin: "+path.name)

def deny():
    for manifest in [ROOT/"Cargo.toml",*list((ROOT/"src").rglob("Cargo.toml"))]: manifest_dependencies(manifest)
    data=json.loads(subprocess.check_output(["cargo","metadata","--offline","--locked","--format-version","1"],cwd=ROOT))
    dependency_records(data)
    for path in source_paths():
        if path.suffix in {".rs",".py",".sh"}: copied_source(path)
    print(f"deny: PASS; {len(data['packages'])} direct/transitive/dev packages, source provenance checked")

if __name__=="__main__":
    try: {"headers":headers,"secrets":secrets,"deny":deny}[sys.argv[1]]()
    except (ValueError,KeyError,OSError,subprocess.SubprocessError) as error:
        print(f"checks: FAILED: {error}",file=sys.stderr); sys.exit(1)
