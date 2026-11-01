#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
# License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md
"""Discover and exercise Codex's own importer, including completed import history."""

import hashlib
import json
from pathlib import Path
import sqlite3
import time
import uuid

from rpc import Rpc
from support import Case, FIRST, Mock, available, claude_session, emit, write_jsonl


def completed_import(rpc, params):
    response = rpc.call("externalAgentConfig/import", params)
    assert "error" not in response, response
    iid = response["result"]["importId"]
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        response = rpc.call("externalAgentConfig/import/readHistories", None)
        assert "error" not in response, response
        matches = [h for h in response["result"]["data"] if h["importId"] == iid]
        if matches:
            return matches[0]
        time.sleep(0.05)
    raise AssertionError("importId was returned but import did not finish within 10 seconds")


def main():
    if not available("codex"):
        return
    with Case() as case:
        version = case.run(["codex", "--version"])
        emit(name="codex-version", **version)
        assert version["exit"] == 0 and not version["timed_out"]
        schema = case.root / "schema"
        result = case.run(["codex", "app-server", "generate-json-schema", "--experimental", "--out", str(schema)])
        assert result["exit"] == 0 and not result["timed_out"], result
        request = json.loads((schema / "ClientRequest.json").read_text())
        methods = [o["properties"]["method"]["enum"][0] for o in request["oneOf"]
                   if "externalAgentConfig/" in str(o)]
        assert "externalAgentConfig/detect" in methods and "externalAgentConfig/import" in methods
        emit(name="importer-schema", methods=methods,
             schema_sha256=hashlib.sha256((schema / "ClientRequest.json").read_bytes()).hexdigest(), **result)
        help_result = case.run(["codex", "--help"], output_limit=16000)
        assert help_result["exit"] == 0 and not help_result["timed_out"]
        commands = help_result["stdout"].split("Commands:")[-1].split("Arguments:")[0].strip()
        emit(name="codex-advertised-commands", commands=commands, exit=help_result["exit"])

    with Case() as case, Mock() as mock:
        case.config_codex(mock.url)
        # Separate the source home from the destination CODEX_HOME. This is the
        # production-relevant route: stage under a disposable HOME, let Codex
        # write its own target store. CLAUDE_CONFIG_DIR is ignored by this importer.
        staging = case.root / "import-home"
        staging.mkdir()
        case.env["HOME"] = str(staging)
        case.claude = staging / ".claude"
        _, source = claude_session(case)
        before = source.read_bytes()
        assert not (case.codex / "state_5.sqlite").exists()
        with Rpc(case) as rpc:
            detected = rpc.call("externalAgentConfig/detect", {
                "includeHome": True, "cwds": [str(case.cwd)], "maxSessions": 10,
                "maxSessionAgeDays": 36500})
            assert "error" not in detected, detected
            items = [i for i in detected["result"]["items"] if i["itemType"] == "SESSIONS"]
            assert len(items) == 1 and len(items[0]["details"]["sessions"]) == 1
            assert items[0]["details"]["sessions"][0]["path"] == str(source)
            history = completed_import(rpc, {"migrationItems": items, "providerId": "lc_probe"})
            assert not history["failures"] and len(history["successes"]) == 1, history
            sid = history["successes"][0]["target"]
            emit(name="importer-completed", source_home="$CASE/import-home",
                 target_codex_home="$CASE/.codex", source_sha256=hashlib.sha256(before).hexdigest(),
                 history=json.loads(case.clean(json.dumps(history))))

            # An arbitrary direct file is not a supported source entry point.
            _, external = claude_session(case, external=True)
            direct = completed_import(rpc, {"migrationItems": [{
                "itemType": "SESSIONS", "description": "Synthetic unregistered source",
                "details": {"sessions": [{"path": str(external), "cwd": str(case.cwd)}]},
            }]})
            assert not direct["successes"] and direct["failures"][0]["subErrorType"] == "session_not_detected"
            emit(name="importer-direct-file-rejected", history=json.loads(case.clean(json.dumps(direct))))

        assert source.read_bytes() == before
        result = case.run(case.codex_resume(sid))
        mock.witness(result, "codex-imported-claude-valid", source_unchanged=source.read_bytes() == before)
        assert source.read_bytes() == before, "import/resume modified the source"

        with sqlite3.connect(f"file:{case.codex}/state_5.sqlite?mode=ro", uri=True) as db:
            target = Path(db.execute("SELECT rollout_path FROM threads WHERE id = ?", (sid,)).fetchone()[0])
        rows = [json.loads(line) for line in target.read_text().splitlines()]
        assert rows[0]["type"] == "session_meta"
        # Copy the imported history to a fresh malformed candidate: never alter
        # the importer's valid artifact or write its database ourselves.
        bad_sid = str(uuid.uuid4())
        bad = case.codex / "sessions/2026/11/01" / f"rollout-2026-11-01T12-00-00-{bad_sid}.jsonl"
        write_jsonl(bad, rows[1:])
        assert FIRST in bad.read_text(), "negative must still contain turn one's bytes"
        mock.requests.clear()
        result = case.run(case.codex_resume(bad_sid))
        mock.witness(result, "codex-imported-claude-missing-meta", expected=False,
                     source_first_turn_present=True)
        assert result["exit"] != 0 and not mock.requests


if __name__ == "__main__":
    main()
