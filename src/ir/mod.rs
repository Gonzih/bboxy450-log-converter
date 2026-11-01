// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

//! The lc-ir v0.1 contract. Reasoning stays separate from spoken message blocks.
//! Inspired by ctx (fidelity, lineage and raw retention), casr (round trips),
//! and Harbor ATIF v1.8 (steps/tool calls); independently implemented.
//! Credits: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, path::PathBuf};

pub mod cbor;
pub mod keys;

/// IR-owned ULID, distinct from any native session identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(pub ulid::Ulid);

/// Native turn identifier, or the parser's deterministic inferred turn key.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TurnId(pub String);

/// RFC 3339 timestamp as recorded by the provider; never synthesized.
pub type Timestamp = String;

/// First 16 bytes of BLAKE3 over framed domain, provider, native_id,
/// native_key and role_tag. Every frame is a u32 BE byte length plus UTF-8 bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventId(pub [u8; 16]);

impl EventId {
    /// Native records use an empty role tag.
    /// # Panics
    /// Panics if a field exceeds the u32 framing limit (4 GiB minus one byte).
    pub fn native(provider: Provider, native_id: &str, native_key: &str) -> Self {
        Self::framed(provider, native_id, native_key, "")
    }

    /// Inferred turn boundaries use the unchanged anchor key and the separate
    /// role tag `turn-start` or `turn-end`.
    /// # Panics
    /// Panics if a field exceeds the u32 framing limit.
    pub fn inferred(provider: Provider, native_id: &str, anchor: &str, role_tag: &str) -> Self {
        Self::framed(provider, native_id, anchor, role_tag)
    }

    fn framed(provider: Provider, native_id: &str, native_key: &str, role_tag: &str) -> Self {
        let mut hash = blake3::Hasher::new();
        for field in [
            "bxl.event.v1",
            provider.as_str(),
            native_id,
            native_key,
            role_tag,
        ] {
            let len = u32::try_from(field.len()).expect("EventId field exceeds u32 length");
            hash.update(&len.to_be_bytes());
            hash.update(field.as_bytes());
        }
        Self(
            hash.finalize().as_bytes()[..16]
                .try_into()
                .expect("16-byte slice"),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub source: Source,
    pub lineage: Vec<Lineage>,
    pub cwd: Option<PathBuf>,
    pub git: Option<Git>,
    pub created_at: Option<Timestamp>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Git {
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub repo_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub provider: Provider,
    pub native_id: String,
    pub cli_version: Option<String>,
    pub parser_revision: u32,
}

macro_rules! unit_enum {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant,)+ #[serde(other)] Unknown }
    };
}

unit_enum!(Provider {
    Claude,
    Codex,
    Bboxy
});
impl Provider {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Bboxy => "bboxy",
            Self::Unknown => "unknown",
        }
    }
}
unit_enum!(Relation {
    ForkOf,
    ResumeOf,
    SwitchedFrom,
    SubagentOf,
    CompactedFrom,
    ImportedFrom
});
unit_enum!(Fidelity {
    Full,
    Partial,
    Inferred,
    SummaryOnly,
    Opaque
});
unit_enum!(TurnOrigin {
    User,
    Resume,
    System
});
unit_enum!(TurnOutcome {
    Completed,
    Interrupted,
    Failed
});
unit_enum!(Role {
    User,
    Assistant,
    System,
    Developer
});
// Protocol v0.4 §4 ToolKind, plus the required forward-compatible Unknown.
unit_enum!(ToolKind {
    Read,
    Edit,
    Delete,
    Move,
    Search,
    Execute,
    Think,
    Fetch,
    Other
});
unit_enum!(Level { Info, Warn, Error });
unit_enum!(SealedForm {
    ThinkingSignature,
    RedactedThinking,
    EncryptedReasoning,
    EncryptedCompaction
});

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lineage {
    pub relation: Relation,
    pub session: SessionId,
    pub native: Option<(Provider, String)>,
    pub at: Option<EventId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub seq: u64,
    pub id: EventId,
    pub parent: Option<EventId>,
    pub turn: Option<TurnId>,
    pub at: Option<Timestamp>,
    pub fidelity: Fidelity,
    pub raw: Option<Raw>,
    pub kind: EventKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventKind {
    TurnStart {
        origin: TurnOrigin,
    },
    TurnEnd {
        outcome: TurnOutcome,
    },
    Message {
        role: Role,
        blocks: Vec<Block>,
    },
    ToolCall {
        call_id: String,
        name: String,
        namespace: Option<String>,
        tool_kind: ToolKind,
        input: ToolInput,
    },
    ToolResult {
        call_id: String,
        is_error: bool,
        blocks: Vec<Block>,
    },
    Reasoning {
        text: Option<String>,
        summary: Option<String>,
        sealed: Option<Sealed>,
    },
    Compaction {
        summary: Option<Vec<Block>>,
        replaced_through: Option<EventId>,
        replacement: Vec<EventId>,
        sealed: Option<Sealed>,
    },
    Context {
        changes: BTreeMap<String, Value>,
    },
    Usage {
        input: u64,
        output: u64,
        cache_read: u64,
        cache_write: u64,
        reasoning: u64,
    },
    Notice {
        level: Level,
        text: String,
    },
    Subagent {
        session: SessionId,
        relation: Relation,
    },
    Native {
        provider: Provider,
        record_type: String,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ToolInput {
    Json(Value),
    Text(String),
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Block {
    Text {
        text: String,
    },
    Image {
        mime: String,
        data: BlobRef,
    },
    Data {
        mime: String,
        data: BlobRef,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobRef {
    pub blake3: [u8; 32],
    pub len: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sealed {
    pub provider: Provider,
    pub form: SealedForm,
    pub bytes: Vec<u8>,
}
/// One native line, without its trailing newline. Bytes need not be UTF-8/JSON.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Raw {
    pub provider: Provider,
    pub bytes: Vec<u8>,
}
