# bboxy450-log-converter

Common IR for Claude Code, Codex and Bboxy session logs. The library and CLI
surface are bootstrapped; native parsing, lowering and bxl persistence follow in
later phases. The CLI lists the planned commands with `--help` and exits 2 for
commands whose implementation has not landed.

```sh
cargo run --offline --locked -- --help
make test
make test-formal
```

`make test` uses a private HOME/CODEX_HOME and closed stdin. It checks formatting,
strict clippy with default/all features, all Rust targets, source headers,
credential shapes, denied dependencies/source provenance, broken controls and
native Rust TLC/Java TLC exact graph and verdict parity. Dependencies must be
fetched first (`cargo fetch --locked`); pinned public Java/TLC artifacts are
provisioned automatically in `target/formal/tools`, without a global install.

The IR follows [lc-ir v0.1](https://github.com/bboxy450/bboxy450-harness/blob/main/.bboxy/interfaces/lc-ir.md).
Its stable CBOR table is [src/ir/keys.rs](src/ir/keys.rs): record keys and enum
discriminants are integers; IDs/hashes/raw/sealed data use byte strings where
appropriate; application JSON maps retain their own keys. Encoding uses definite
lengths, shortest primitive representations and recursive bytewise map key
ordering under [RFC 8949 §4.2.1](https://www.rfc-editor.org/rfc/rfc8949.html#section-4.2.1).
Fixed goldens cover every variant and enum value, including mixed key types.

Event IDs take the first 16 BLAKE3 bytes of five framed UTF-8 fields: domain tag
`bxl.event.v1`, provider, native session ID, native record key and role tag.
Every field, including the domain and role, has a u32 big-endian byte length.
Native records use an empty role tag; inferred boundaries use the anchor's key
with a separate `turn-start` or `turn-end` role. The finalized contract is
[harness d7cc01e](https://github.com/bboxy450/bboxy450-harness/blob/d7cc01e/.bboxy/interfaces/lc-ir.md).

Readers accept valid noncanonical record encodings, including indefinite-length
containers and nonpreferred primitive widths, and validate the IR schema.
Writers always emit the canonical form; the mixed-key golden uses
`{10,23,24,100,-1,"a"}` in that bytewise order.

```rust
use bboxy450_log_converter::ir::{Event, cbor};
# fn example(bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
let decoded = cbor::decode::<Event>(bytes)?;
let event = decoded.value(); // known IR view; reasoning is separate from Message
let forwarded = decoded.encode()?; // preserves unknown fields/tags at every depth
# Ok(())
# }
```

`Decoded` keeps an immutable known projection and the complete CBOR value so
future fields are retained without adding transport fields to the binding IR.
`into_value()` explicitly discards future fields; encoding that extracted value
creates a new v0.1 record. Duplicate map keys and malformed/trailing records fail.
No real session logs are included; every fixture is synthetic.

[Verification inventory](verification/README.md) records P1's implementation-only
coverage, the checker bootstrap corpus and planned product contracts. The binding
specs and task records remain in the harness KB identified in [AGENTS.md](AGENTS.md).

Origin, license and accurate reuse credit: [ATTRIBUTION.md](ATTRIBUTION.md).
