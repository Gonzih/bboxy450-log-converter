# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
# License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md
"""Bounded stdio app-server client using the installed binary's JSON-RPC schema."""

import json
import os
import queue
import signal
import subprocess
import threading
import time

from support import TIMEOUT, emit


class Rpc:
    def __init__(self, case):
        self.case = case
        self.messages = queue.Queue()
        self.transcript = []
        self.next_id = 0

    def __enter__(self):
        self.stderr = (self.case.root / "app-server.stderr").open("w+")
        self.proc = subprocess.Popen(["codex", "app-server", "--listen", "stdio://"],
            cwd=self.case.cwd, env=self.case.env, stdin=subprocess.PIPE,
            stdout=subprocess.PIPE, stderr=self.stderr, text=True, start_new_session=True)

        def read():
            for line in self.proc.stdout:
                self.messages.put(json.loads(line))
            self.messages.put(None)

        self.reader = threading.Thread(target=read, daemon=True)
        self.reader.start()
        try:
            self.call("initialize", {"clientInfo": {"name": "lc_probe", "version": "0.0.0"},
                                     "capabilities": {"experimentalApi": True}})
            self.send({"method": "initialized"})
        except BaseException:
            self.__exit__(None, None, None)
            raise
        return self

    def send(self, message):
        self.proc.stdin.write(json.dumps(message) + "\n")
        self.proc.stdin.flush()

    def call(self, method, params):
        self.next_id += 1
        rid = self.next_id
        self.send({"id": rid, "method": method, "params": params})
        deadline = time.monotonic() + TIMEOUT
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError(f"app-server deadline waiting for {method}")
            message = self.messages.get(timeout=remaining)
            if message is None:
                raise AssertionError(f"app-server EOF waiting for {method}")
            if message.get("id") == rid:
                self.transcript.append({"method": method, "reply": message})
                return message
            if "id" in message and "method" in message:
                self.send({"id": message["id"], "error": {
                    "code": -32601, "message": "Probe denies all server requests"}})

    def __exit__(self, *args):
        self.proc.stdin.close()
        timed_out = False
        try:
            self.proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            timed_out = True
            os.killpg(self.proc.pid, signal.SIGKILL)
            self.proc.wait(timeout=5)
        self.reader.join(timeout=2)
        self.proc.stdout.close()
        self.stderr.seek(0)
        emit(name="app-server-lifecycle", exit=self.proc.returncode,
             timed_out=timed_out, stderr=self.case.clean(self.stderr.read())[-3000:])
        self.stderr.close()
        assert not timed_out, "app-server failed to exit after stdin EOF"
        assert self.proc.returncode == 0, "app-server failed"
