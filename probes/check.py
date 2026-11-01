# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
# License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md
"""Check source policy and exercise false-positive and timeout controls for the gate."""

import ast
import contextlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
from unittest.mock import patch

from support import Case, FIRST, SECOND, Mock, available, emit


def main():
    root = Path(__file__).resolve().parent.parent
    origin = (root / "ATTRIBUTION.md").read_text().split("```", 2)[1].strip()
    header = "\n".join("#" + line[2:] for line in origin.splitlines())
    sources = list((root / "probes").glob("*.py")) + list((root / "probes").glob("*.sh")) + [root / "Makefile"]
    local = {path.stem for path in sources}
    for path in sources:
        content = path.read_text()
        assert header in content[:1000], f"{path}: attribution header missing"
        if path.suffix == ".py":
            tree = ast.parse(content, filename=str(path))
            for node in ast.walk(tree):
                imports = ([alias.name for alias in node.names] if isinstance(node, ast.Import)
                           else [node.module] if isinstance(node, ast.ImportFrom) else [])
                for module in imports:
                    assert module.split(".")[0] in sys.stdlib_module_names | local, f"unexpected dependency: {module}"
    result = subprocess.run(["/bin/sh", "-n", str(root / "probes/resume.sh")],
                            stdin=subprocess.DEVNULL, timeout=5, check=False)
    assert result.returncode == 0

    fake = {"exit": 0, "timed_out": False, "stdout": FIRST, "stderr": "", "argv": []}
    history = [{"role": "user", "content": FIRST}, {"role": "user", "content": SECOND}]
    model_request = {"path": "/v1/messages?beta=true", "body": {"messages": history}}
    with contextlib.redirect_stdout(io.StringIO()):
        mock = Mock()
        mock.requests = [model_request]
        assert mock.witness(fake, "positive")["resume_verdict"]
        # Exit zero without prior input, prior input with exit nonzero, and a
        # token-count request are all inadequate evidence of successful resume.
        for request, code in [(None, 0), (model_request, 1),
                              ({"path": "/v1/messages/count_tokens", "body": {"messages": history}}, 0)]:
            mock.requests = [request] if request else []
            candidate = {**fake, "exit": code}
            assert not mock.witness(candidate, "negative", expected=False)["resume_verdict"]
            try:
                mock.witness(candidate, "broken-control", expected=True)
            except AssertionError:
                pass
            else:
                raise AssertionError("a false-positive control escaped the gate")

        with patch.dict(os.environ, {"ANTHROPIC_API_KEY": "must-not-propagate",
                                     "CODEX_API_KEY": "must-not-propagate",
                                     "BBOXY_CAPABILITY": "must-not-propagate"}):
            with Case() as case:
                assert all("must-not-propagate" not in value for value in case.env.values())
                assert "BBOXY_CAPABILITY" not in case.env and "CODEX_API_KEY" not in case.env
                assert case.env["HOME"] == str(case.root)
                result = case.run([sys.executable, "-c", "import time; time.sleep(30)"], timeout=0.1)
                assert result["timed_out"] and result["exit"] != 0
                mock.requests = []
                try:
                    mock.witness(result, "timeout-control", expected=False)
                except AssertionError:
                    pass
                else:
                    raise AssertionError("timeout incorrectly counted as an expected rejection")
        with patch.dict(os.environ, {"PATH": ""}):
            assert available("claude") is None and available("codex") is None

    emit(name="probe-checks", status="PASS", source_files=len(sources),
         controls=["attribution", "stdlib-only", "shell-syntax", "exit-and-history",
                   "ignore-token-count", "drop-inherited-auth", "timeout-is-failure", "absent-cli-skip"])


if __name__ == "__main__":
    main()
