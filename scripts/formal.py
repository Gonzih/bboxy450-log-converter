# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
# License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

# Adapted from bboxy450-harness scripts/formal.py and bboxy450-term scripts/formal.py.
# Origin: https://github.com/bboxy450/bboxy450-harness/blob/main/scripts/formal.py
# Upstream policy: https://github.com/bboxy450/bboxy450-harness/blob/main/ATTRIBUTION.md
# Copyright 2026 Boxy (bboxy450) and contributors; LicenseRef-Respectful-Slop-1.0-Draft.
"""Finite native/reference gate; an empty product inventory never skips checker parity."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib
ROOT = Path(__file__).resolve().parents[1]

def artifact(pin, path):
    """Download only a pinned public artifact, with bounded IO and atomic install."""
    if not path.exists():
        path.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="download-", dir=path.parent) as own:
            staged = Path(own) / "artifact"
            subprocess.run(["curl", "-fsSL", "--retry", "2", "--max-time", "180",
                            "-o", str(staged), pin["url"]], check=True, timeout=560)
            verify_digest(staged, pin["sha256"])
            staged.replace(path)
    verify_digest(path, pin["sha256"])
    return path


def verify_digest(path, expected):
    actual = hashlib.sha256(path.read_bytes()).hexdigest()
    if actual != expected:
        raise ValueError(f"artifact checksum mismatch: {path}: {actual} != {expected}")


def tools(target):
    pins = json.loads((ROOT / "verification/tools.lock.json").read_text())
    cache = target / "tools"
    jar = artifact(pins["tlc"], cache / "tla2tools.jar")
    host = f"{platform.system()}-{platform.machine()}"
    if host not in pins["java"]:
        raise ValueError(f"no pinned unattended Java runtime for {host}; add a reviewed tools.lock.json entry")
    pin = pins["java"][host]
    archive = artifact(pin, cache / f"java-{host}.tar.gz")
    runtime = cache / f"java-{host}-{pin['version']}"
    if not runtime.exists():
        with tempfile.TemporaryDirectory(prefix="extract-", dir=cache) as own:
            with tarfile.open(archive) as tar:
                tar.extractall(own, filter="data")
            entries = list(Path(own).iterdir())
            if len(entries) != 1:
                raise ValueError("unexpected Java archive layout")
            entries[0].replace(runtime)
    java = runtime / ("Contents/Home/bin/java" if platform.system() == "Darwin" else "bin/java")
    if not java.is_file():
        raise ValueError(f"Java runtime missing executable: {java}")
    return java, jar, pins


def dot_graph(path, width):
    """Normalize TLC's labeled DOT graph to exact integer states and edge pairs."""
    nodes = {}
    edges = []
    for line in path.read_text().splitlines():
        edge = re.match(r'\s*(-?\d+)\s*->\s*(-?\d+)\s*\[label="A(\d+)"', line)
        if edge:
            edges.append(tuple(edge.groups()))
            continue
        node = re.match(r'\s*(-?\d+)\s*\[label="((?:\\.|[^"\\])*)"', line)
        if node:
            values = {int(i): int(v) for i, v in re.findall(r'v(\d+) = (-?\d+)', node[2])}
            if set(values) != set(range(width)):
                raise ValueError(f"unrecognized TLC state: {node[2]}")
            nodes[node[1]] = tuple(values[i] for i in range(width))
    if not nodes:
        raise ValueError(f"TLC did not emit states: {path}")
    transitions = {(nodes[a], int(action), nodes[b]) for a, b, action in edges}
    return set(nodes.values()), transitions


def native_graph(root, name):
    states = {tuple(map(int, line.split(','))) for line in (root / f"{name}.states").read_text().splitlines()}
    edges = set()
    for line in (root / f"{name}.edges").read_text().splitlines():
        if not line:
            continue
        a, action, b = line.split('\t')
        edges.add((tuple(json.loads(a)), int(action), tuple(json.loads(b))))
    return states, edges


def tlc(java, jar, root, name, graph):
    suffix = "Graph" if graph else ""
    # TLC unpacks standard modules into java.io.tmpdir. Each invocation owns
    # its directory so concurrent graph/property runs cannot rewrite one another.
    temporary = root / f"{name}{suffix}-java-tmp"
    temporary.mkdir(parents=True, exist_ok=True)
    command = [str(java), "-Djava.io.tmpdir="+str(temporary), "-XX:ActiveProcessorCount=2", "-Xmx256m", "-XX:+UseSerialGC", "-cp", str(jar),
               "tlc2.TLC", "-workers", "1", "-cleanup", "-config", f"{name}{suffix}.cfg",
               "-metadir", str(root / f"{name}{suffix}-metadata")]
    if graph:
        command += ["-dump", "dot,actionlabels", f"{name}.dot"]
    command += [name]
    try:
        result = subprocess.run(command, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                text=True, timeout=60)
    except subprocess.TimeoutExpired as error:
        (root / f"{name}{suffix}.log").write_bytes(error.stdout or b"")
        raise ValueError(f"TLC timed out: {name}; incomplete is not a pass") from error
    (root / f"{name}{suffix}.log").write_text(result.stdout)
    return result


def verdict(result):
    output = result.stdout
    if result.returncode == 0 and "Model checking completed. No error has been found." in output:
        return "pass"
    # Exit status and named diagnostics must agree; syntax/tool errors are never expected negatives.
    for name, code, diagnostic in [("safety", 12, "is violated"), ("deadlock", 11, "Deadlock reached"),
                                   ("liveness", 13, "Temporal properties were violated")]:
        if result.returncode == code and diagnostic in output:
            return name
    raise ValueError(f"TLC tool failure (exit {result.returncode}):\n{output[-4000:]}")



def inventory(root, names):
    """Independent corpus, every crate/source, and bounded paired contracts."""
    required = tomllib.loads((root / "verification/corpus.toml").read_text())["model"]
    expected = {r["name"]: r["expected"] for r in required}
    if not required or len(expected) != len(required) or set(expected) != names:
        raise ValueError("missing, duplicate or stale formal corpus")
    if any(r["expected"] not in {"pass","safety","deadlock","liveness"} or r["frontend"] not in {"checker","project","yarn"} for r in required):
        raise ValueError("invalid formal corpus verdict/frontend")
    manifest = tomllib.loads((root / "Cargo.toml").read_text())
    packages = {manifest["package"]["name"]} if "package" in manifest else set()
    for pattern in manifest.get("workspace",{}).get("members",[]):
        for directory in root.glob(pattern):
            packages.add(tomllib.loads((directory/"Cargo.toml").read_text())["package"]["name"])
    crates = tomllib.loads((root / "verification/coverage.toml").read_text())["crate"]
    if len(crates) != len(packages) or {r["name"] for r in crates} != packages:
        raise ValueError("crate inventory omission/duplication")
    for row in crates:
        if row["status"] not in {"implementation-tests","contract-model"} or not row["scope"].strip() or not row["implementation_only"]:
            raise ValueError("missing coverage scope or implementation-only limits")
        if row["status"] == "contract-model" and not row["models"]:
            raise ValueError("empty contract-model claim")
        if not set(row["models"]).issubset(names) or not row["sources"] or any(not (root/p).is_file() for p in row["sources"]):
            raise ValueError("missing model/source mapping")
    actual = {str(p.relative_to(root)) for p in root.rglob("*.rs") if p.parts[len(root.parts)] in {"src","crates"} and "target" not in p.parts}
    mapped = {p for r in crates for p in r["sources"]}
    if mapped != actual:
        raise ValueError("formal source inventory omission/stale entry: "+str(mapped ^ actual))
    contracts = tomllib.loads((root/"verification/contracts.toml").read_text())["contract"]
    project = {r["name"] for r in required if r["frontend"] == "project"}
    if len({r["name"] for r in contracts}) != len(contracts):
        raise ValueError("duplicate contract")
    covered = set()
    for row in contracts:
        family = {row["name"],*row["controls"]}
        if not row["controls"] or len(row["models"]) != len(family) or set(row["models"]) != family or not family.issubset(project):
            raise ValueError("missing executed contract pair")
        if expected[row["name"]] != "pass" or any(expected[n] == "pass" for n in row["controls"]):
            raise ValueError("broken control must fail")
        if covered & family:
            raise ValueError("duplicate model owner")
        covered |= family
        for field in ["assumptions","bounds","property"]:
            if not row.get(field,"").strip(): raise ValueError("missing contract "+field)
        for field in ["sources","implementation_tests"]:
            if not row.get(field) or any(not (root/p).is_file() for p in row[field]): raise ValueError("missing contract "+field)
        if row.get("checks") != ["make test-formal","make test"]: raise ValueError("missing canonical contract checks")
    if covered != project or {n for r in crates for n in r["models"]} != {r["name"] for r in contracts}:
        raise ValueError("project contract/source mapping omission")
    return required, crates, contracts


def input_hashes(root=ROOT):
    files = {"Cargo.toml","Cargo.lock","Makefile","ATTRIBUTION.md","deny.toml"}
    for base in ["src","tests","scripts","verification"]:
        files.update(str(p.relative_to(root)) for p in (root/base).rglob("*") if p.is_file() and "evidence" not in p.parts and "__pycache__" not in p.parts)
    return {p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in sorted(files)}


def evidence_receipt(target, root=ROOT, require_clean=True):
    data = json.loads((target/"evidence.json").read_text())
    sha = subprocess.check_output(["git","rev-parse","HEAD"],cwd=root,text=True).strip()
    if data["source"] != sha or data["input_sha256"] != input_hashes(root):
        raise ValueError("stale formal source/hash receipt")
    if require_clean and (data["dirty"] or source_dirty(root)):
        raise ValueError("formal receipt requires clean source")
    required, crates, contracts = inventory(root,{m["name"] for m in data["models"]})
    if data["crates"] != crates or data["contracts"] != contracts or len(data["models"]) != len(required):
        raise ValueError("stale formal inventory receipt")
    pins = json.loads((root/"verification/tools.lock.json").read_text())
    if data["tlc"] != pins["tlc"] or data["java"] not in pins["java"].values() or data["complete_tlc_replacement"] is not False:
        raise ValueError("invalid tool pins or completeness claim")
    expected = {r["name"]:r["expected"] for r in required}
    for row in data["models"]:
        if (row["expected"] != expected[row["name"]] or row["graph_equal"] is not True or row["states"] < 1 or row["edges"] < 0
                or row["tlc_exit"] != {"pass":0,"safety":12,"deadlock":11,"liveness":13}[row["expected"]]):
            raise ValueError("incomplete/mismatched parity evidence")
        for suffix, digest in row["artifacts"].items():
            if hashlib.sha256((target/"models"/(row["name"]+suffix)).read_bytes()).hexdigest() != digest:
                raise ValueError("stale model/graph/verdict bytes")
    return data


def source_dirty(root=ROOT):
    return bool(subprocess.check_output(["git","status","--porcelain","--","src","tests","scripts","verification/coverage.toml","verification/corpus.toml","verification/contracts.toml","verification/tools.lock.json","Cargo.toml","Cargo.lock","Makefile","deny.toml"],cwd=root).strip())


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target",type=Path,default=ROOT/"target/formal")
    parser.add_argument("--native",action="store_true")
    parser.add_argument("--verify-evidence",action="store_true")
    args=parser.parse_args()
    target=args.target.resolve()
    if args.verify_evidence:
        receipt=evidence_receipt(target)
        print("formal: current-source receipt PASS; "+str(len(receipt["models"]))+" models")
        return
    root=target/"models"
    rows=[line.split("\t") for line in (root/"manifest.tsv").read_text().splitlines()]
    if not rows or any(len(r)!=4 for r in rows) or len({r[0] for r in rows}) != len(rows):
        raise ValueError("empty/duplicate/malformed native manifest")
    required,crates,contracts=inventory(ROOT,{r[0] for r in rows})
    if {r[0]:r[1] for r in rows} != {r["name"]:r["expected"] for r in required}:
        raise ValueError("native verdict differs from independent corpus")
    if any(not re.fullmatch(r"[A-Za-z][A-Za-z0-9_]*",n) or int(c)<1 or int(w)<1 for n,e,c,w in rows):
        raise ValueError("invalid native model manifest")
    if args.native:
        print(f"formal: native {len(rows)} checker/contract cases; {len(crates)} crates; {len(contracts)} product contracts")
        return
    java,jar,pins=tools(target)
    inputs=input_hashes()
    def check_row(row):
        name,expected,count,width=row
        graph=tlc(java,jar,root,name,True)
        if verdict(graph)!="pass": raise ValueError("incomplete graph generation: "+name)
        java_states,java_edges=dot_graph(root/(name+".dot"),int(width))
        states,edges=native_graph(root,name)
        if java_states != states or java_edges != edges or len(states)!=int(count):
            raise ValueError("native/Java full state/transition mismatch: "+name)
        checked=tlc(java,jar,root,name,False)
        if verdict(checked)!=expected: raise ValueError("native/Java verdict mismatch: "+name)
        artifacts={p.name[len(name):]:hashlib.sha256(p.read_bytes()).hexdigest() for p in root.glob(name+".*") if p.is_file()}
        print(f"formal: {name}: {expected}, {len(states)} states, {len(edges)} edges match",flush=True)
        return {"name":name,"expected":expected,"states":len(states),"edges":len(edges),"graph_equal":True,"tlc_exit":checked.returncode,"artifacts":artifacts}
    with ThreadPoolExecutor(max_workers=4) as pool:
        models=list(pool.map(check_row,rows))
    if inputs!=input_hashes(): raise ValueError("source changed during parity")
    data={"source":subprocess.check_output(["git","rev-parse","HEAD"],cwd=ROOT,text=True).strip(),
          "input_sha256":inputs,"dirty":source_dirty(),"tlc":pins["tlc"],"java":pins["java"][f"{platform.system()}-{platform.machine()}"],
          "crates":crates,"contracts":contracts,"models":models,"complete_tlc_replacement":False}
    raw=json.dumps(data,indent=2)+"\n"
    (target/"evidence.json").write_text(raw)
    archive=ROOT/"verification/evidence"/hashlib.sha256(raw.encode()).hexdigest()
    archive.mkdir(parents=True,exist_ok=True)
    shutil.copyfile(target/"evidence.json",archive/"evidence.json")
    shutil.copytree(root,archive/"models",dirs_exist_ok=True,ignore=shutil.ignore_patterns("*-metadata","*-java-tmp"))
    evidence_receipt(archive,require_clean=False)
    print(f"formal: PASS; {len(models)} exact graph/verdict pairs; {len(contracts)} product contracts; retained {archive.relative_to(ROOT)}")

if __name__=="__main__":
    try: main()
    except (ValueError,OSError,KeyError,subprocess.SubprocessError) as error:
        print(f"formal: FAILED: {error}",file=sys.stderr)
        sys.exit(1)
