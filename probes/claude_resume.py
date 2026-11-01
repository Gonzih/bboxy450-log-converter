#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
# License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md
"""Probe id and external-path resume, plus a broken UUID/parent chain."""

import hashlib
import json

from support import Case, FIRST, Mock, available, claude_session, emit


def main(api_only=False):
    if not available("claude"):
        return
    with Case() as case:
        result = case.run(["claude", "--version"])
        emit(name="claude-version", **result)
        assert result["exit"] == 0 and not result["timed_out"]
    targets = ("without-api-request",) if api_only else ("id", "external-path")
    for target in targets:
        for broken in (False, True):
            with Case() as case, Mock() as mock:
                case.env["ANTHROPIC_BASE_URL"] = mock.url
                sid, path = claude_session(case, broken=broken, external=target == "external-path")
                source = path.read_bytes()
                assert FIRST.encode() in source
                rows = [json.loads(line) for line in source.splitlines()]
                assert not any(row["type"].startswith("api-request") for row in rows)
                if broken:
                    ids = {row["uuid"] for row in rows}
                    assert any(row["parentUuid"] not in ids for row in rows if row["parentUuid"])
                    assert len(ids) < len(rows)
                result = case.run(case.claude_resume(path if target == "external-path" else sid))
                mock.witness(result, f"claude-{target}-{'broken-chain' if broken else 'valid'}",
                    expected=not broken, api_request_records=0,
                    source_first_turn_present=True,
                    source_sha256=hashlib.sha256(source).hexdigest(),
                    source_unchanged=source == path.read_bytes())
    if not api_only:
        # Keep the surprising, weaker-control result explicit. Never claim the
        # CLI validates arbitrary parentUuid chains merely because our control fails.
        with Case() as case, Mock() as mock:
            case.env["ANTHROPIC_BASE_URL"] = mock.url
            sid, _ = claude_session(case, dangling_only=True)
            mock.witness(case.run(case.claude_resume(sid)), "claude-dangling-parent-tolerated")


if __name__ == "__main__":
    main()
