# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
# License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md
# Copied from bboxy450-log-converter P0: https://github.com/bboxy450/bboxy450-log-converter/blob/7945ad92055b9f95b4339e51a213e9800a637448/probes/support.py
"""Synthetic-only CLI witnesses. No inherited credentials, config, or model traffic."""

import contextlib
import hashlib
import http.server
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import tempfile
import threading
import uuid

FIRST = "LC_SYNTHETIC_TURN_ONE_7e943b"
SECOND = "LC_SYNTHETIC_TURN_TWO_12aa09"
REPLY = "LC_MOCK_REPLY"
REASON = "LC_SYNTHETIC_REASONING_SUMMARY_29fc81"
STAMP = "2026-11-01T12:00:00.000Z"
TIMEOUT = 25


def emit(**fields):
    print(json.dumps(fields, sort_keys=True), flush=True)


def available(cli):
    path = shutil.which(cli)
    if path is None:
        emit(status="SKIP", cli=cli, reason=f"{cli} executable absent from PATH")
    return path


class Case:
    def __enter__(self):
        self.temp = tempfile.TemporaryDirectory(prefix="lc-probe-")
        self.root = Path(self.temp.name).resolve()
        self.cwd = self.root / "work"
        self.claude = self.root / ".claude"
        self.codex = self.root / ".codex"
        for path in (self.cwd, self.claude, self.codex, self.root / "tmp"):
            path.mkdir()
        # Construct from an allowlist: never copy the caller's auth, proxy, hooks,
        # seat capability, CODEX_HOME, or CLAUDE_CONFIG_DIR into a tested process.
        self.env = {
            "PATH": os.environ.get("PATH", "/usr/bin:/bin"),
            "HOME": str(self.root), "CLAUDE_CONFIG_DIR": str(self.claude),
            "CODEX_HOME": str(self.codex), "TMPDIR": str(self.root / "tmp"),
            "XDG_CONFIG_HOME": str(self.root / ".config"),
            "XDG_CACHE_HOME": str(self.root / ".cache"),
            "XDG_DATA_HOME": str(self.root / ".local/share"),
            "ANTHROPIC_API_KEY": "lc-probe-dummy-anthropic",
            "LC_MOCK_KEY": "lc-probe-dummy-responses",
            "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1",
            "DISABLE_AUTOUPDATER": "1", "CI": "1", "TERM": "dumb",
            "NO_COLOR": "1", "PYTHONDONTWRITEBYTECODE": "1",
            "HTTP_PROXY": "http://127.0.0.1:9", "HTTPS_PROXY": "http://127.0.0.1:9",
            "ALL_PROXY": "http://127.0.0.1:9", "NO_PROXY": "127.0.0.1,localhost,::1",
        }
        return self

    def __exit__(self, *args):
        self.temp.cleanup()

    def clean(self, value):
        return str(value).replace(str(self.root), "$CASE")

    def run(self, argv, timeout=TIMEOUT, output_limit=5000):
        proc = subprocess.Popen(argv, cwd=self.cwd, env=self.env,
                                stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE, start_new_session=True)
        timed_out = False
        try:
            stdout, stderr = proc.communicate(timeout=timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
            with contextlib.suppress(ProcessLookupError):
                os.killpg(proc.pid, signal.SIGKILL)
            stdout, stderr = proc.communicate(timeout=5)
        result = {
            "argv": [self.clean(a) for a in argv], "exit": proc.returncode,
            "timed_out": timed_out,
            "stdout": self.clean(stdout.decode(errors="replace"))[-output_limit:],
            "stderr": self.clean(stderr.decode(errors="replace"))[-output_limit:],
        }
        return result

    def config_codex(self, url):
        (self.codex / "config.toml").write_text(f'''model = "gpt-6.1-sol"
model_provider = "lc_mock"
approval_policy = "never"
sandbox_mode = "read-only"
check_for_update_on_startup = false
suppress_unstable_features_warning = true
[features]
plugins = false
remote_plugin = false
shell_snapshot = false
skip_host_skill_discovery = true
skill_mcp_dependency_install = false
[model_providers.lc_mock]
name = "Local synthetic probe"
base_url = "{url}/v1"
wire_api = "responses"
env_key = "LC_MOCK_KEY"
requires_openai_auth = false
supports_websockets = false
request_max_retries = 0
stream_max_retries = 0
stream_idle_timeout_ms = 5000
''')

    def codex_resume(self, sid):
        return ["codex", "exec", "--skip-git-repo-check", "--ignore-rules",
                "--json", "resume", sid, "LC_NEXT_TURN"]

    def claude_resume(self, target):
        # --bare explicitly disables keychain reads, hooks, memory and CLAUDE.md
        # discovery (installed CLI --help); -p skips the workspace trust dialog.
        return ["claude", "--bare", "-p", "--resume", str(target),
                "--model", "claude-sonnet-4-6", "--tools", "",
                "--strict-mcp-config", "--mcp-config", '{"mcpServers":{}}',
                "--setting-sources", "", "--permission-mode", "dontAsk",
                "--system-prompt-snapshot", "on", "--output-format", "json",
                "LC_NEXT_TURN"]


def write_jsonl(path, rows):
    path.parent.mkdir(parents=True, exist_ok=True)
    # Test fixtures obey the create-only contract too.
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
        for row in rows:
            stream.write(json.dumps(row) + "\n")


def claude_session(case, broken=False, external=False, dangling_only=False):
    sid = str(uuid.uuid4())
    parent = None
    rows = []
    for index, (role, text) in enumerate([
        ("user", FIRST), ("assistant", "LC_ASSISTANT_ONE"),
        ("user", SECOND), ("assistant", "LC_ASSISTANT_TWO"),
    ]):
        uid = str(uuid.uuid4())
        message = {"role": role, "content": [{"type": "text", "text": text}]}
        if role == "assistant":
            message.update(id=f"msg_probe_{index}", type="message",
                           model="claude-sonnet-4-6", stop_reason="end_turn",
                           stop_sequence=None, usage={"input_tokens": 10, "output_tokens": 3})
        rows.append({"type": role, "uuid": uid,
                     "parentUuid": str(uuid.uuid4()) if dangling_only and index == 2 else parent,
                     "sessionId": sid, "cwd": str(case.cwd), "gitBranch": "probe",
                     "version": "2.1.304", "timestamp": STAMP,
                     "isSidechain": False, "userType": "external", "message": message})
        parent = uid
    if broken:
        # Duplicate node identities break parentUuid references without removing
        # the first turn's bytes. Claude keeps only the last colliding node.
        # A dangling parent alone is tolerated in 2.1.304 (separately witnessed).
        for row in rows:
            row["uuid"] = rows[0]["uuid"]
    slug = re.sub(r"[^a-zA-Z0-9]", "-", str(case.cwd))
    path = ((case.root / "outside-client" / f"{sid}.jsonl") if external else
            case.claude / "projects" / slug / f"{sid}.jsonl")
    write_jsonl(path, rows)
    return sid, path


def codex_session(case, broken=False, reasoning=False):
    sid = str(uuid.uuid4())
    rows = [{"type": "session_meta", "payload": {
        "id": sid, "timestamp": STAMP, "cwd": str(case.cwd),
        "originator": "lc_probe", "cli_version": "0.164.0", "source": "cli",
        "model_provider": "lc_mock", "base_instructions": {"text": "Synthetic probe."},
    }}]
    for role, text in [("user", FIRST), ("assistant", "LC_ASSISTANT_ONE"),
                       ("user", SECOND), ("assistant", "LC_ASSISTANT_TWO")]:
        rows.append({"type": "response_item", "payload": {
            "type": "message", "role": role,
            "content": [{"type": "input_text" if role == "user" else "output_text", "text": text}],
        }})
    if reasoning:
        rows.insert(2, {"type": "response_item", "payload": {
            "type": "reasoning", "id": "rs_probe",
            "summary": [{"type": "summary_text", "text": REASON}],
        }})
    if broken:
        rows.pop(0)
    for row in rows:
        row["timestamp"] = STAMP
    path = case.codex / "sessions/2026/11/01" / f"rollout-2026-11-01T12-00-00-{sid}.jsonl"
    write_jsonl(path, rows)
    return sid, path


class Mock:
    def __enter__(self):
        self.requests = []
        owner = self

        class Handler(http.server.BaseHTTPRequestHandler):
            def log_message(self, *args):
                pass

            def do_GET(self):
                self.send_response(404)
                self.end_headers()

            def do_POST(self):
                body = json.loads(self.rfile.read(int(self.headers.get("Content-Length", 0))))
                # Never capture headers: only synthetic request bodies.
                owner.requests.append({"path": self.path, "body": body})
                if "count_tokens" in self.path:
                    data = json.dumps({"input_tokens": 10}).encode()
                    self.send_response(200)
                    self.send_header("Content-Length", str(len(data)))
                    self.end_headers()
                    self.wfile.write(data)
                    return
                if self.path.split("?")[0] not in ("/v1/messages", "/v1/responses"):
                    self.send_response(404)
                    self.end_headers()
                    return
                events = owner.anthropic(body) if "/messages" in self.path else owner.responses(body)
                data = "".join(f"event: {kind}\ndata: {json.dumps(value)}\n\n"
                               for kind, value in events).encode()
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream")
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                with contextlib.suppress(BrokenPipeError, ConnectionResetError):
                    self.wfile.write(data)

        self.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.server.daemon_threads = True
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.url = f"http://127.0.0.1:{self.server.server_port}"
        return self

    def __exit__(self, *args):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=2)

    @staticmethod
    def anthropic(body):
        message = {"id": "msg_lc_mock", "type": "message", "role": "assistant",
                   "model": body.get("model"), "content": [], "stop_reason": None,
                   "stop_sequence": None, "usage": {"input_tokens": 10, "output_tokens": 0}}
        return [("message_start", {"type": "message_start", "message": message}),
                ("content_block_start", {"type": "content_block_start", "index": 0,
                                         "content_block": {"type": "text", "text": ""}}),
                ("content_block_delta", {"type": "content_block_delta", "index": 0,
                                         "delta": {"type": "text_delta", "text": REPLY}}),
                ("content_block_stop", {"type": "content_block_stop", "index": 0}),
                ("message_delta", {"type": "message_delta", "delta": {
                    "stop_reason": "end_turn", "stop_sequence": None}, "usage": {"output_tokens": 4}}),
                ("message_stop", {"type": "message_stop"})]

    @staticmethod
    def responses(body):
        item = {"type": "message", "id": "msg_lc_mock", "role": "assistant",
                "status": "completed", "content": [{"type": "output_text", "text": REPLY,
                                                       "annotations": [], "logprobs": []}]}
        response = {"id": "resp_lc_mock", "object": "response", "created_at": 1793534400,
                    "model": body.get("model"), "status": "completed", "output": [item],
                    "usage": {"input_tokens": 10, "output_tokens": 4, "total_tokens": 14,
                              "input_tokens_details": {"cached_tokens": 0},
                              "output_tokens_details": {"reasoning_tokens": 0}}}
        return [("response.created", {"type": "response.created", "response": {
                    **response, "status": "in_progress", "output": []}}),
                ("response.output_item.done", {"type": "response.output_item.done",
                                               "output_index": 0, "item": item}),
                ("response.completed", {"type": "response.completed", "response": response})]

    def witness(self, result, name, expected=True, **extra):
        model = [r for r in self.requests if r["path"].split("?")[0] in
                 ("/v1/messages", "/v1/responses")]
        # Test actual model input, never tool descriptions, filesystem contents,
        # echoed CLI output, or count_tokens traffic.
        histories = [r["body"].get("messages", r["body"].get("input", [])) for r in model]
        first = any(FIRST in json.dumps(h) for h in histories)
        second = any(SECOND in json.dumps(h) for h in histories)
        verdict = result["exit"] == 0 and first
        evidence = {"name": name, "status": "PASS" if verdict == expected and
                    not result["timed_out"] else "FAIL", "expected_resume": expected,
                    "resume_verdict": verdict, "turn_one_in_request": first,
                    "turn_two_in_request": second, "request_count": len(model),
                    "input_sha256": [hashlib.sha256(json.dumps(h, sort_keys=True).encode()).hexdigest()
                                     for h in histories],
                    "captured_history": [[item for item in h if any(token in json.dumps(item)
                        for token in (FIRST, SECOND, REASON, "LC_NEXT_TURN"))] for h in histories],
                    "verdict_exit": 0 if verdict else 1, **result, **extra}
        emit(**evidence)
        assert not result["timed_out"], f"{name}: timeout is a failure, never a negative witness"
        assert verdict == expected, f"{name}: unexpected resume verdict"
        if expected:
            assert second, f"{name}: second turn lost"
        return evidence
