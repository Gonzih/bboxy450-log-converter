# Formal and implementation verification

P1 implements immutable IR data and CBOR bytes. Under d4949, these get golden,
round-trip and malformed-input tests. Event identity tests include a literal
length-framed hash vector and the s1/23 versus s12/3 collision control. A mixed-key
CBOR vector distinguishes RFC 8949 §4.2.1 bytewise order from length-first order.
There are **zero product stateful models**
at this phase. `coverage.toml` inventories the crate and every production Rust
source with its implementation-only evidence; omission fails the gate. No Yarn
runtime/topology exists in P1, so there is no applicable Yarn projection yet.

`make test-formal` always runs native `bboxy450-tlc =0.1.1`, exports TLA+ from its
finite action DSL, and compares full reachable state and action-labeled edge sets
plus independent safety/deadlock/liveness verdicts with pinned Java TLC 1.7.4.
The private Temurin 21 runtime and jar have SHA-256 pins in `tools.lock.json`.
Nothing starts Java in the product runtime. No global installation or operator
input is required. The canonical `make test` includes this parity gate.

The bootstrap checker corpus is infrastructure coverage, not a product proof:

| Case | Bounds and initial state | Actions/owners | Property and fairness | Expected |
|---|---|---|---|---|
| GateCounter | count 0..2, initially 0 | one owner increments to 2 | count ≤ 2; terminal state allowed; no fairness | pass |
| BrokenGateSafety | same | same | count ≤ 1, deliberately false | safety |
| BrokenGateDeadlock | same | same | terminal state incorrectly disallowed | deadlock |
| GateFairResponse | done/tick 0..1, initially 0/0 | serve and independent tick | eventually done, WF(serve) | pass |
| BrokenGateLiveness | same | same | eventually done without fairness; tick can starve serve | liveness |

`tests/formal/models.rs` registers future product contracts.
`corpus.toml` independently states every expected verdict. `contracts.toml` starts
empty and requires product models to have bounded assumptions, positive and
broken controls, source mappings and actual API/process witness paths. Inventory,
manifest omissions/duplicates, incomplete exploration, invalid semantics, timeouts,
tool failures, bad hashes, graph mismatches or verdict disagreements fail. There
is no inference of arbitrary Rust correctness or complete TLC compatibility.

| Product contract | Status | Planned bounds and controls |
|---|---|---|
| LcAppendLog | deferred to P2; no writer exists | 3 frames × 2 events, 2 readers, 1 rotation; early yield, early index, overwritten seek-table controls |
| LcWatcher | deferred to P7; no watcher exists | partial/truncated/replaced lines, crash/restart, ≤4 batches; cursor-before-ack, partial-line, ignored-inode controls |
| LcSessionSwitch | deferred to P8 in term; no handoff exists | quiesce/stop/convert/launch/rollback A→B→A; open-turn, early-launch, source-write and queued-input controls |

The binding model scope/assumptions belong to the harness ICDs until those phases
land. Parsing, lowering, JSON/CBOR/zstd bytes, cryptography, filesystem durability,
real process behavior and runtime deadlines need their named implementation
witnesses. P1 has `tests/cbor_golden.rs`, `tests/cbor_compat.rs`, `tests/cli.rs` and
`scripts/test-checks.py`; later-phase witnesses are not claimed.

The gate writes `target/formal/evidence.json` and archives model bytes, native
states/edges, Java logs and receipts under `verification/evidence/<digest>/`.
Receipts bind HEAD, source/model/tool hashes, inventories and verdicts. After a
clean commit or rebase run `make test` again and `make check-formal-evidence`;
a receipt from a previous HEAD fails. Generated evidence stays local and survives
`target` cleanup. `.bboxy/runs/test-current.json` lists exact gate exit statuses.

The dependency/provenance gate rejects direct, transitive, renamed, dev and
platform dependencies on the d4949 projects, plus declared copied/adapted/
translated origins in source. Broken controls exercise these rejections.
Undeclared copied code still requires source review; the checker does not claim
to solve plagiarism detection. Code here was written from the binding ICD;
no implementation from the denied projects was read or copied.

The parity driver and exporter are adapted from bboxy450-harness/bboxy450-term
with adjacent origin/license notices. Product policy:
https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

P2 integration note: keep the `Decoded<Event>`/`Decoded<Session>` envelope through
bxl read/forward paths so nested future fields survive. A typed editing API that
retains extensions is deferred; `into_value()` currently discards them explicitly.
