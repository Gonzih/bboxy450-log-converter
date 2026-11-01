// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

//! Stable CBOR v0.1 key and discriminant table (lc-ir §7).
//! Record fields use the numbers below. Tagged unions reserve key 0 for their
//! integer discriminant; payload keys start at 1. Unit enums use their index,
//! with 255 reserved for Unknown. Never renumber or reuse a published key.
//! JSON-valued tool/context payloads retain their application's string keys.
use Schema::*;

#[derive(Clone, Copy, Debug)]
pub enum Schema {
    Scalar,
    Bytes,
    List(&'static Schema),
    Tuple(&'static [Schema]),
    Map(&'static Schema),
    Record(&'static [Field]),
    Tagged(&'static [Variant]),
    Unit(&'static [&'static str]),
}
#[derive(Clone, Copy, Debug)]
pub struct Field {
    pub key: u64,
    pub name: &'static str,
    pub schema: Schema,
}
#[derive(Clone, Copy, Debug)]
pub struct Variant {
    pub tag: u64,
    pub name: &'static str,
    pub fields: &'static [Field],
}

macro_rules! fields {
    ($( $key:literal => $name:literal : $schema:expr ),* $(,)?) => {
        &[$(Field { key: $key, name: $name, schema: $schema }),*]
    };
}
macro_rules! variant {
    ($tag:literal, $name:literal, $fields:expr) => {
        Variant {
            tag: $tag,
            name: $name,
            fields: $fields,
        }
    };
}

pub const PROVIDER: Schema = Unit(&["claude", "codex", "bboxy"]);
pub const RELATION: Schema = Unit(&[
    "fork_of",
    "resume_of",
    "switched_from",
    "subagent_of",
    "compacted_from",
    "imported_from",
]);
pub const FIDELITY: Schema = Unit(&["full", "partial", "inferred", "summary_only", "opaque"]);
pub const TURN_ORIGIN: Schema = Unit(&["user", "resume", "system"]);
pub const TURN_OUTCOME: Schema = Unit(&["completed", "interrupted", "failed"]);
pub const ROLE: Schema = Unit(&["user", "assistant", "system", "developer"]);
pub const TOOL_KIND: Schema = Unit(&[
    "read", "edit", "delete", "move", "search", "execute", "think", "fetch", "other",
]);
pub const LEVEL: Schema = Unit(&["info", "warn", "error"]);
pub const SEALED_FORM: Schema = Unit(&[
    "thinking_signature",
    "redacted_thinking",
    "encrypted_reasoning",
    "encrypted_compaction",
]);

pub const SOURCE: Schema = Record(fields![
    0 => "provider": PROVIDER, 1 => "native_id": Scalar,
    2 => "cli_version": Scalar, 3 => "parser_revision": Scalar,
]);
pub const GIT: Schema =
    Record(fields![0 => "branch": Scalar, 1 => "commit": Scalar, 2 => "repo_url": Scalar]);
pub const LINEAGE: Schema = Record(fields![
    0 => "relation": RELATION, 1 => "session": Scalar,
    2 => "native": Tuple(&[PROVIDER, Scalar]), 3 => "at": Bytes,
]);
pub const SESSION: Schema = Record(fields![
    0 => "id": Scalar, 1 => "source": SOURCE, 2 => "lineage": List(&LINEAGE),
    3 => "cwd": Scalar, 4 => "git": GIT, 5 => "created_at": Scalar,
]);
pub const BLOB_REF: Schema = Record(fields![0 => "blake3": Bytes, 1 => "len": Scalar]);
pub const RAW: Schema = Record(fields![0 => "provider": PROVIDER, 1 => "bytes": Bytes]);
pub const SEALED: Schema =
    Record(fields![0 => "provider": PROVIDER, 1 => "form": SEALED_FORM, 2 => "bytes": Bytes]);
pub const BLOCK: Schema = Tagged(&[
    variant!(0, "text", fields![1 => "text": Scalar]),
    variant!(
        1,
        "image",
        fields![1 => "mime": Scalar, 2 => "data": BLOB_REF]
    ),
    variant!(
        2,
        "data",
        fields![1 => "mime": Scalar, 2 => "data": BLOB_REF]
    ),
]);
pub const TOOL_INPUT: Schema = Tagged(&[
    variant!(0, "json", fields![1 => "value": Scalar]),
    variant!(1, "text", fields![1 => "value": Scalar]),
]);
pub const EVENT_KIND: Schema = Tagged(&[
    variant!(0, "turn_start", fields![1 => "origin": TURN_ORIGIN]),
    variant!(1, "turn_end", fields![1 => "outcome": TURN_OUTCOME]),
    variant!(
        2,
        "message",
        fields![1 => "role": ROLE, 2 => "blocks": List(&BLOCK)]
    ),
    variant!(
        3,
        "tool_call",
        fields![1 => "call_id": Scalar, 2 => "name": Scalar, 3 => "namespace": Scalar, 4 => "tool_kind": TOOL_KIND, 5 => "input": TOOL_INPUT]
    ),
    variant!(
        4,
        "tool_result",
        fields![1 => "call_id": Scalar, 2 => "is_error": Scalar, 3 => "blocks": List(&BLOCK)]
    ),
    variant!(
        5,
        "reasoning",
        fields![1 => "text": Scalar, 2 => "summary": Scalar, 3 => "sealed": SEALED]
    ),
    variant!(
        6,
        "compaction",
        fields![1 => "summary": List(&BLOCK), 2 => "replaced_through": Bytes, 3 => "replacement": List(&Bytes), 4 => "sealed": SEALED]
    ),
    variant!(7, "context", fields![1 => "changes": Map(&Scalar)]),
    variant!(
        8,
        "usage",
        fields![1 => "input": Scalar, 2 => "output": Scalar, 3 => "cache_read": Scalar, 4 => "cache_write": Scalar, 5 => "reasoning": Scalar]
    ),
    variant!(
        9,
        "notice",
        fields![1 => "level": LEVEL, 2 => "text": Scalar]
    ),
    variant!(
        10,
        "subagent",
        fields![1 => "session": Scalar, 2 => "relation": RELATION]
    ),
    variant!(
        11,
        "native",
        fields![1 => "provider": PROVIDER, 2 => "record_type": Scalar]
    ),
]);
pub const EVENT: Schema = Record(fields![
    0 => "seq": Scalar, 1 => "id": Bytes, 2 => "parent": Bytes,
    3 => "turn": Scalar, 4 => "at": Scalar, 5 => "fidelity": FIDELITY,
    6 => "raw": RAW, 7 => "kind": EVENT_KIND,
]);
