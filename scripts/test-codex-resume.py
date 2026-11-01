# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
# License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

"""Resume the Rust-lowered bytes through the P0 mock, with a missing-meta control."""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import sys
import traceback
import uuid

sys.dont_write_bytecode=True
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location("p0_mock", ROOT/"tests/support/p0_mock.py")
p0=importlib.util.module_from_spec(spec)
spec.loader.exec_module(p0)

def candidate(path, data):
    path.parent.mkdir(parents=True,exist_ok=True)
    with os.fdopen(os.open(path,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600),"wb") as stream:
        stream.write(data)

def run(path):
    if not p0.available("codex"):
        return
    data=path.read_bytes()
    sid=json.loads(data.splitlines()[0])["payload"]["id"]
    assert p0.FIRST.encode() in data and p0.SECOND.encode() in data
    with p0.Case() as case, p0.Mock() as mock:
        case.config_codex(mock.url)
        version=case.run(["codex","--version"])
        p0.emit(name="p4-codex-version",**version)
        assert version["exit"]==0 and not version["timed_out"]
        path=case.codex/"sessions/2026/11/01"/f"rollout-2026-11-01T12-00-00-{sid}.jsonl"
        candidate(path,data)
        assert not (case.codex/"state_5.sqlite").exists()
        mock.witness(case.run(case.codex_resume(sid)),"p4-lowered-codex-valid",native_bytes_unchanged=path.read_bytes().startswith(data))
        assert path.read_bytes().startswith(data),"CLI rewrote the lowered prefix"
        mock.requests.clear()
        bad_sid=str(uuid.uuid4())
        bad=path.with_name(f"rollout-2026-11-01T12-00-00-{bad_sid}.jsonl")
        candidate(bad,data.split(b"\n",1)[1])
        assert p0.FIRST.encode() in bad.read_bytes()
        result=case.run(case.codex_resume(bad_sid))
        mock.witness(result,"p4-lowered-codex-missing-meta",expected=False)
        assert result["exit"]!=0 and not mock.requests

def main():
    capture=io.StringIO()
    code=0
    with contextlib.redirect_stdout(capture):
        try: run(Path(sys.argv[1]))
        except Exception:
            traceback.print_exc(file=capture)
            code=1
    output=capture.getvalue()
    destination=Path(sys.argv[2]);destination.parent.mkdir(parents=True,exist_ok=True)
    header="\n".join(Path(__file__).read_text().splitlines()[:6])
    destination.write_text(header+"\n\n"+output+json.dumps({"script_exit":code})+"\n")
    print(output,end="")
    return code

if __name__=="__main__": sys.exit(main())
