# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
# License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md
"""Run every independent probe even if another fails; emit machine-readable exit evidence."""

import os
from pathlib import Path
import subprocess
import sys

from support import emit


def main():
    failures = 0
    scripts = ("claude_resume.py", "claude_api_records.py", "codex_rollout.py",
               "codex_importer.py", "codex_reasoning.py")
    for script in scripts:
        # CLI probes themselves supply all homes and dummy keys. The outer
        # Python process receives no inherited credentials or seat capability.
        result = subprocess.run([sys.executable, "-B", str(Path(__file__).parent / script)],
            env={"PATH": os.environ.get("PATH", "/usr/bin:/bin"), "PYTHONDONTWRITEBYTECODE": "1"},
            stdin=subprocess.DEVNULL, check=False)
        emit(name=script, script_exit=result.returncode)
        failures += result.returncode != 0
    emit(name="test-probes", exit=1 if failures else 0, failed_scripts=failures)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
