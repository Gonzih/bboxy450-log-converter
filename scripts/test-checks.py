# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
# License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

"""Broken controls: dependency aliases/transitives, declared copied code, secret and formal omissions."""
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
sys.dont_write_bytecode=True
ROOT=Path(__file__).resolve().parents[1]
def module(name):
    spec=importlib.util.spec_from_file_location(name,ROOT/"scripts"/(name+".py"))
    result=importlib.util.module_from_spec(spec);spec.loader.exec_module(result);return result
checks=module("checks")
formal=module("formal")

class BrokenControls(unittest.TestCase):
    def test_deny_direct_renamed_target_and_transitive_packages(self):
        for package in ["ctx","ctxrs","casr","cross_agent_session_resumer","harbor","session_bridge","claude-code-log"]:
            with self.subTest(package=package),self.assertRaisesRegex(ValueError,"denied dependency"):
                checks.dependency_records({"packages":[{"name":"ordinary"},{"name":package}]})
        checks.dependency_records({"packages":[{"name":"serde"},{"name":"context"}]})
        with tempfile.TemporaryDirectory() as own:
            path=Path(own)/"Cargo.toml"
            for section in ["dependencies","dev-dependencies","target.'cfg(unix)'.build-dependencies"]:
                path.write_text('['+section+']\ninnocent = { package = "casr", version = "1" }\n')
                with self.assertRaisesRegex(ValueError,"manifest dependency"): checks.manifest_dependencies(path)
        with self.assertRaisesRegex(ValueError,"denied dependency"):
            checks.dependency_records({"packages":[{"name":"innocent","source":"git+https://github.com/ctxrs/ctx?rev=1"}]})

    def test_copied_code_origin_is_rejected_but_inspiration_is_allowed(self):
        with tempfile.TemporaryDirectory() as own:
            path=Path(own)/"source.rs"
            for mode in ["Copied","Adapted","Translated"]:
                path.write_text("// "+mode+" from ctx: https://github.com/ctxrs/ctx\nfn copied() {}\n")
                with self.assertRaisesRegex(ValueError,"copied origin"): checks.copied_source(path)
            path.write_text("// Inspired by ctx; no implementation copied.\nfn original() {}\n")
            checks.copied_source(path)

    def test_secret_control_reports_only_the_path(self):
        with tempfile.TemporaryDirectory() as own:
            path=Path(own)/"planted.txt"
            path.write_text("sk-"+"A"*32)
            with self.assertRaisesRegex(ValueError,"value suppressed"): checks.scan_secrets([path])

    def inventory_root(self,own):
        root=Path(own)
        for base in ["src","verification"]:
            shutil.copytree(ROOT/base,root/base,ignore=shutil.ignore_patterns("evidence"))
        shutil.copyfile(ROOT/"Cargo.toml",root/"Cargo.toml")
        return root

    def test_inventory_omitted_crate_source_and_corpus_are_rejected(self):
        names={r["name"] for r in formal.tomllib.loads((ROOT/"verification/corpus.toml").read_text())["model"]}
        with tempfile.TemporaryDirectory() as own:
            root=self.inventory_root(own)
            formal.inventory(root,names)
            with self.assertRaisesRegex(ValueError,"corpus"): formal.inventory(root,names-{"BrokenGateSafety"})
            added=root/"src/forgotten.rs";added.write_text("// omitted source\n")
            with self.assertRaisesRegex(ValueError,"source inventory"): formal.inventory(root,names)
            added.unlink()
            (root/"verification/coverage.toml").write_text("crate=[]\n")
            with self.assertRaisesRegex(ValueError,"crate inventory"): formal.inventory(root,names)

    def test_product_model_cannot_bypass_pair_and_source_witness_inventory(self):
        names={r["name"] for r in formal.tomllib.loads((ROOT/"verification/corpus.toml").read_text())["model"]}
        with tempfile.TemporaryDirectory() as own:
            root=self.inventory_root(own)
            with (root/"verification/corpus.toml").open("a") as out:
                out.write('\n[[model]]\nname="LcFuture"\nexpected="pass"\nfrontend="project"\n')
            with self.assertRaisesRegex(ValueError,"mapping omission"): formal.inventory(root,names|{"LcFuture"})
            (root/"verification/contracts.toml").write_text('[[contract]]\nname="LcFuture"\ncontrols=[]\nmodels=["LcFuture"]\n')
            with self.assertRaisesRegex(ValueError,"contract pair"): formal.inventory(root,names|{"LcFuture"})

    def test_syntax_errors_timeouts_and_incomplete_runs_never_count_as_negative_verdicts(self):
        for code,text in [(1,"parse error"),(12,"parse error"),(0,"still running"),(124,"timeout")]:
            with self.assertRaisesRegex(ValueError,"tool failure"): formal.verdict(subprocess.CompletedProcess([],code,text))
        self.assertEqual(formal.verdict(subprocess.CompletedProcess([],12,"Invariant Bound is violated")),"safety")

    def test_unrecognized_tlc_graph_state_is_rejected(self):
        with tempfile.TemporaryDirectory() as own:
            path=Path(own)/"bad.dot"
            path.write_text('1 [label="wrong = 0"];\n')
            with self.assertRaisesRegex(ValueError,"unrecognized TLC state"): formal.dot_graph(path,1)

    def test_wrong_artifact_hash_is_rejected_without_installing(self):
        with tempfile.TemporaryDirectory() as own:
            path=Path(own)/"artifact";path.write_bytes(b"wrong")
            with self.assertRaisesRegex(ValueError,"checksum mismatch"): formal.artifact({"sha256":"0"*64},path)

if __name__=="__main__": unittest.main()
