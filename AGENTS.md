# bboxy450-log-converter — agent instructions

This repository parses Claude Code, Codex and Bboxy session logs into a common IR. The IR is stored in a streamable, append-only compressed format (bxl). The converter lowers the IR back into native sessions, and it powers cross-harness session switching in bboxy450-term.

The binding design lives in the bboxy450-harness knowledge base (https://github.com/bboxy450/bboxy450-harness, `.bboxy/`):
- `specs/log-converter.md` (#4965)
- interfaces `lc-ir.md` (#4960), `lc-bxl.md` (#4961), `lc-watch-sink.md` (#4962) and `session-switch.md` (#4963)
- decisions d4947 (sanitized demo bundles), d4949 (built from scratch; TLA+ with Rust TLC and Java parity), d4964 (own repo, git-pinned) and d4972 (native write exception)
- research #4951 and #4953

Tasks for this repository live in that knowledge base as well.

## Roles
| Role | Who | Does |
|---|---|---|
| Oracle | Claude | Briefs, verifies, merges, releases |
| Architect | Claude | Owns ICDs, invariants, verification matrix |
| Executor | Codex | Implements only from an assigned task, in its own worktree |

## Merge authorization
None recorded yet. Until the operator records it here, the Oracle does not merge. Executors open PRs.

## Rules for every change
- `make test` runs fully unattended, and includes formal checks: native Rust TLC with parity against Java TLC.
- Every source file carries the header from `ATTRIBUTION.md`.
- No dependency on, and no code copied from, ctx, casr, Harbor or session-bridge (d4949). A deny check enforces this.
- Every commit is signed.
- Never commit secrets, or unsanitized content from real session logs. Real-log content may be committed only after the d4947 sanitizer and an Oracle review.
