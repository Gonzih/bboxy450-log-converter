# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
# License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

# Adapted from https://github.com/bboxy450/bboxy450-harness/blob/main/scripts/test.py
# Copyright 2026 Boxy (bboxy450) and contributors; LicenseRef-Respectful-Slop-1.0-Draft.
# Upstream policy: https://github.com/bboxy450/bboxy450-harness/blob/main/ATTRIBUTION.md
"""Canonical gate in a private HOME, closed stdin, with exact exit/hash evidence."""
import datetime
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
sys.dont_write_bytecode=True
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location("formal",ROOT/"scripts/formal.py")
formal=importlib.util.module_from_spec(spec);spec.loader.exec_module(formal)

def main():
    env=os.environ.copy()
    original=Path(env["HOME"])
    env["CARGO_HOME"]=env.get("CARGO_HOME",str(original/".cargo"))
    env["RUSTUP_HOME"]=env.get("RUSTUP_HOME",str(original/".rustup"))
    for name in ["BBOXY_CAPABILITY","UPDATE_CBOR_GOLDEN"]: env.pop(name,None)
    inputs=formal.input_hashes()
    rows=[]
    with tempfile.TemporaryDirectory(prefix="lc-tests-",dir="/tmp") as own:
        env.update(HOME=own,CODEX_HOME=own+"/codex",TMPDIR=own,GIT_TERMINAL_PROMPT="0")
        for command in [
            ["cargo","fmt","--check"],
            ["cargo","clippy","--offline","--locked","--all-targets","--","-D","warnings"],
            ["cargo","clippy","--offline","--locked","--all-targets","--all-features","--","-D","warnings"],
            ["cargo","test","--offline","--locked","--all-targets","--all-features"],
            ["python3","-I","scripts/checks.py","headers"],
            ["python3","-I","scripts/checks.py","secrets"],
            ["python3","-I","scripts/checks.py","deny"],
            ["python3","-I","scripts/test-checks.py"],
            ["make","test-formal"],
        ]:
            print("test: "+" ".join(command),flush=True)
            try: code=subprocess.run(command,cwd=ROOT,env=env,stdin=subprocess.DEVNULL,timeout=900).returncode
            except subprocess.TimeoutExpired: code=124
            rows.append({"command":command,"exit_code":code})
            if code: break
    code=rows[-1]["exit_code"]
    if inputs!=formal.input_hashes(): code=1
    record={"at":datetime.datetime.now(datetime.timezone.utc).isoformat(),"source":subprocess.check_output(["git","rev-parse","HEAD"],cwd=ROOT,text=True).strip(),"input_sha256":inputs,"private_home":True,"operator_input":False,"commands":rows,"exit_code":code}
    (ROOT/".bboxy/runs").mkdir(parents=True,exist_ok=True)
    (ROOT/".bboxy/runs/test-current.json").write_text(json.dumps(record,indent=2)+"\n")
    print("test: "+("PASS" if code==0 else "FAIL"),flush=True)
    return code
if __name__=="__main__": sys.exit(main())
