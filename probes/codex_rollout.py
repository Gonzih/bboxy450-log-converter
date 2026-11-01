#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
# License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md
"""Resume hand-written rollouts from a home with no SQLite database; inspect backfill read-only."""

import json
import sqlite3

from support import Case, Mock, REASON, available, codex_session, emit


def main(reasoning=False):
    if not available("codex"):
        return
    for broken in (False, True):
        with Case() as case, Mock() as mock:
            case.config_codex(mock.url)
            sid, path = codex_session(case, broken=broken, reasoning=reasoning)
            database = case.codex / "state_5.sqlite"
            assert not database.exists(), "probe must begin without a database"
            if reasoning:
                records = [json.loads(line) for line in path.read_text().splitlines()]
                item = next(r["payload"] for r in records if r["payload"].get("type") == "reasoning")
                assert "encrypted_content" not in item
            result = case.run(case.codex_resume(sid))
            indexed = []
            if database.exists():
                with sqlite3.connect(f"file:{database}?mode=ro", uri=True) as db:
                    indexed = db.execute("SELECT id, rollout_path FROM threads WHERE id = ?", (sid,)).fetchall()
            if not broken:
                assert indexed and indexed[0] == (sid, str(path)), "CLI did not backfill the expected rollout"
            reasoning_items = [item for request in mock.requests for item in request["body"].get("input", [])
                               if item.get("type") == "reasoning"]
            if reasoning and not broken:
                assert any(REASON in json.dumps(item.get("summary")) and
                           not item.get("encrypted_content") for item in reasoning_items)
            mock.witness(result,
                f"codex-{'reasoning-summary' if reasoning else 'filename-fallback'}-{'missing-meta' if broken else 'valid'}",
                expected=not broken, database_existed_before=False,
                backfilled_rows=[[sid, case.clean(p)] for sid, p in indexed],
                reasoning_items=reasoning_items)
            if broken:
                assert result["exit"] != 0 and not mock.requests, "missing metadata must fail before inference"


if __name__ == "__main__":
    main()
