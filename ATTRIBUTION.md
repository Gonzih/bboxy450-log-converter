# Respectful Slopping — Source Attribution and Reuse

Project: **bboxy450-log-converter**, agent session-log parser, streamable compressed IR and cross-harness lowering.
Canonical source: https://github.com/bboxy450/bboxy450-log-converter
Copyright 2026 Boxy (bboxy450) and contributors, for their respective contributions.

Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md
License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE

**Slop respectfully. Keep the credit attached.**

## Terms

Project-owned contributions are under the Respectful Slop License 1.0-Draft
(`LicenseRef-Respectful-Slop-1.0-Draft`, see `LICENSE`). When you reuse material from this
project, credit it accurately as one of the following:

- **Copied from**
- **Adapted from**
- **Translated from**
- **Inspired by**

Each credit names **bboxy450-log-converter**, this canonical URL, and the affected file, symbol or block.
Independent inspiration credit is a request, not a condition.

## Keep credit beside the material

Every source file starts with this header:

```
// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md
```

A distinctive implementation adapted from elsewhere carries its own `// Adapted from <project>: <url>` line beside it.

## Lineage and credits

- **Inspired by Gonzih/kittens** (https://github.com/Gonzih/kittens), the predecessor Rust harness by the same authors. Ideas carried over:
  - the turn-end guarantee
  - tool-call pairing
  - the cancellation tree
  - `!cmd` key profiles
  - ACP over one event stream
- **Inspired by** these projects. The design survey copied no code from them; evidence is in `.bboxy/research/sota-harnesses.md`.
  - openai/codex
  - xai-org/grok-build
  - badlogic/pi-mono
  - oh-my-pi
  - sst/opencode
  - block/goose
  - Aider
  - the Zed Agent Client Protocol
  - the Model Context Protocol
- **Built on** the bboxy450-term toolkit crates (https://github.com/bboxy450/bboxy450-term), used as dependencies under their own terms.

## Inspired by (ideas only; no code copied, no dependency — decision d4949)

- **ctx** — https://github.com/ctxrs/ctx (Apache-2.0): per-event fidelity enum, session lineage, raw native record retention.
- **cross_agent_session_resumer (casr)** — https://github.com/Dicklesworthstone/cross_agent_session_resumer (MIT): provider read/write/resume shape and the read(write(read(x))) fidelity check.
- **Harbor ATIF v1.8** — https://github.com/laude-institute/harbor (Apache-2.0): trajectory step and tool-call shape; ATIF export is our own implementation.
- **claude-code-log** — https://github.com/daaain/claude-code-log (MIT): taxonomy of Claude Code entry types.

## Shared suite license verification

The license text is byte-identical to bboxy450-term's Respectful Slop License
1.0-Draft. `make test-licenses` checks the entire license, local Cargo metadata,
source-attached credit, preserved prior terms and deliberate broken controls.
`make check-license-packages` checks Cargo's real package file lists where present.

Copied from bboxy450-term, with original source notices retained:
- https://github.com/bboxy450/bboxy450-term/blob/main/scripts/suite-licenses.py
- https://github.com/bboxy450/bboxy450-term/blob/main/scripts/test-suite-licenses.py

The accompanying `licensing.json` records source scope, preserved-file hashes
and the exact shared checker copies. These checks do not replace upstream terms.
