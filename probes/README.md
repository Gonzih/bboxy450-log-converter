<!--
Copyright 2026 Boxy (bboxy450) and contributors.
SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md
-->

# Native resume probes

Run `make test` (policy and witness controls plus all probes), or
`make test-probes`. Python 3.10+ and the standard library are sufficient. Each
client is optional: absence produces a labelled `SKIP` and exit 0. An installed
client that fails a probe makes the gate fail. No package install or remote
model service is needed.

The individual scripts also run directly with `python3 -B probes/<script>`:

| Script | Witness |
| --- | --- |
| `claude_resume.py` | Two completed turns, by id and by a path outside the client directory; broken UUID/parent chains; tolerated dangling parent |
| `claude_api_records.py` | The same history with explicitly zero `api-request*` records |
| `codex_rollout.py` | Hand-written rollout, no pre-existing database, and CLI-owned index backfill |
| `codex_importer.py` | Installed experimental protocol schema, completed native import, resume, unregistered source rejection, and malformed imported rollout |
| `codex_reasoning.py` | Readable reasoning summary with no encrypted content in the source and no nonempty encrypted content on the wire |

Every case emits JSON. `verdict_exit=0` means **CLI exit 0 and turn one present
in the mock's captured model input**; otherwise it is 1. Positive cases also
assert turn two. A script exits 0 only when its positive cases succeed and its
negative cases produce verdict 1. `status=PASS` on a negative record means that
the expected rejection was observed; it does not mean the candidate resumed.

Claude's negative source still contains both turns. Duplicate UUIDs leave
`parentUuid` references dangling: this damages the identity/parent chain and
the CLI loses the prior turns despite exiting 0. A single dangling parent with
unique UUIDs is tolerated and is recorded separately. Do not infer that Claude
validates all parent links. Codex's negative candidates retain the messages
but omit the leading `session_meta`; the CLI exits 1 before inference.

Every client invocation receives newly constructed environment variables,
throwaway `HOME`, `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, XDG and temporary directories,
dummy keys, and loopback model endpoints. Inherited credentials, seat
capabilities, proxies and config are excluded. Claude uses `--bare`, whose
installed help explicitly disables keychain reads. Codex uses a custom provider;
plugin synchronization and host skill discovery are disabled. External HTTP
proxy settings point at a closed loopback port. No authentication headers are
recorded. All native JSONL and databases are temporary synthetic artifacts;
the probe reads SQLite only with `mode=ro` and lets the client perform backfill
and import writes itself.

Client commands and RPC replies have a 25-second deadline. Asynchronous import
completion has a 10-second deadline; app-server shutdown gets 5 seconds.
Timeout kills the process group and **fails**, including in negative cases.
Temporary homes and mock servers are cleaned up. `probes/check.py` exercises
false-positive, timeout, absent-client, environment-isolation and source-policy
controls. These are CLI/process probes; converter Rust/formal targets belong
to the separate P1 bootstrap and are not present on the P0-only branch.

The importer protocol comes from `codex app-server generate-json-schema
--experimental`, not guessed method names. OpenAI documents the handshake and
version-matched schema generation at
https://learn.chatgpt.com/docs/app-server. An import id alone is insufficient:
the probe waits for a completed history with a successful target id, then
resumes that target against the mock.

The committed results and limits are in
`.bboxy/runs/p0-resume-probe-report.log`. These probes establish client replay
behavior for the recorded versions. A mock endpoint does not prove that a real
provider will accept every synthesized reasoning or tool item.
