#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
# Canonical shared copy: https://github.com/bboxy450/bboxy450-term/blob/main/scripts/test-suite-licenses.py
"""Unattended negative controls for the suite's portable license policy."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import sys
from unittest.mock import patch
from types import SimpleNamespace

spec = importlib.util.spec_from_file_location("suite_licenses", Path(__file__).with_name("suite-licenses.py"))
policy = importlib.util.module_from_spec(spec)
sys.dont_write_bytecode = True
spec.loader.exec_module(policy)


class LicensePolicy(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="bboxy-suite-licenses-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        canonical = Path(__file__).resolve().parents[1]
        (self.root / "LICENSE").write_bytes((canonical / "LICENSE").read_bytes())
        for name in ("NOTICE", "ATTRIBUTION.md"):
            (self.root / name).write_text(policy.POLICY + "\nhttps://github.com/bboxy450/bboxy450-term\n")
        (self.root / "licensing.json").write_text(json.dumps({"version": 1, "project": "bboxy450-term", "source_roots": ["src"]}))
        (self.root / "src").mkdir()
        header = "\n".join(Path(__file__).read_text().splitlines()[1:7]) + "\n"
        (self.root / "src/check.py").write_text(header + "answer = 42\n")

    def fails(self, text):
        with self.assertRaisesRegex(ValueError, text):
            policy.check(self.root)

    def test_valid_fixture_and_sources(self):
        self.assertEqual(policy.check(self.root)["source_files"], 1)

    def test_changed_license_body_is_rejected(self):
        with (self.root / "LICENSE").open("a") as f:
            f.write("changed terms\n")
        self.fails("LICENSE differs")

    def test_missing_notice_is_rejected(self):
        (self.root / "NOTICE").unlink()
        self.fails("NOTICE")

    def test_symlinked_license_is_rejected(self):
        (self.root / "LICENSE").rename(self.root / "original")
        (self.root / "LICENSE").symlink_to("original")
        self.fails("regular file")

    def test_erased_source_credit_is_rejected(self):
        (self.root / "src/check.py").write_text("answer = 42\n")
        self.fails("source-attached")

    def test_blanket_cargo_spdx_override_is_rejected(self):
        (self.root / "Cargo.toml").write_text('[package]\nname="bboxy450-fixture"\nlicense="MIT"\n')
        self.fails("license-file")

    def test_changed_preserved_license_is_rejected(self):
        path = self.root / "LICENSE-MIT"
        path.write_text("original upstream terms")
        config = json.loads((self.root / "licensing.json").read_text())
        config["preserved"] = {"LICENSE-MIT": policy.digest(path.read_bytes())}
        (self.root / "licensing.json").write_text(json.dumps(config))
        policy.check(self.root)
        path.write_text("replacement upstream terms")
        self.fails("preserved upstream")

    def test_changed_shared_checker_is_rejected(self):
        config = json.loads((self.root / "licensing.json").read_text())
        config["shared"] = {"src/check.py": policy.digest((self.root / "src/check.py").read_bytes())}
        (self.root / "licensing.json").write_text(json.dumps(config))
        with (self.root / "src/check.py").open("a") as f:
            f.write("answer = 43\n")
        self.fails("shared license checker")

    def test_package_missing_notice_is_rejected(self):
        (self.root / "Cargo.toml").write_text('[package]\nname="bboxy450-fixture"\nlicense-file="LICENSE"\n')
        output = SimpleNamespace(returncode=0, stdout="LICENSE\nATTRIBUTION.md\n", stderr="")
        with patch.object(policy.subprocess, "run", return_value=output):
            with self.assertRaisesRegex(ValueError, "package omits notices"):
                policy.check(self.root, package_lists=True)

    def test_package_private_state_is_rejected(self):
        (self.root / "Cargo.toml").write_text('[package]\nname="bboxy450-fixture"\nlicense-file="LICENSE"\n')
        output = SimpleNamespace(returncode=0, stdout="LICENSE\nNOTICE\nATTRIBUTION.md\n.bboxy/private.md\n", stderr="")
        with patch.object(policy.subprocess, "run", return_value=output):
            with self.assertRaisesRegex(ValueError, "private .bboxy"):
                policy.check(self.root, package_lists=True)


if __name__ == "__main__":
    unittest.main()
