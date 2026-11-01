// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

//! Incremental Codex JSONL reader. Checkpoints are complete native-record boundaries.
//! The caller owns the IR SessionId. Native ids and timestamps come from metadata.

use crate::ir::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, BufRead, Read, Seek, SeekFrom};

pub const PARSER_REVISION: u32 = 1;
pub const MAX_LINE_BYTES: usize = 64 * 1024 * 1024;

/// Save together with `Parser::session()`. Resume rereads only an incomplete tail.
/// A batch from `next_record` is atomic: checkpoint only after consuming the batch.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    pub offset: u64,
    pub line_ordinal: u64,
    pub next_seq: u64,
    pub previous: Option<EventId>,
    pub turn: Option<TurnId>,
    pub parse_errors: u64,
}

pub struct Parser<R> {
    reader: R,
    session: Session,
    cursor: Cursor,
    retain_raw: bool,
}

impl<R: BufRead + Seek> Parser<R> {
    pub fn new(reader: R, id: SessionId) -> io::Result<Self> {
        Self::resume(
            reader,
            Session {
                id,
                source: Source {
                    provider: Provider::Codex,
                    native_id: String::new(),
                    cli_version: None,
                    parser_revision: PARSER_REVISION,
                },
                lineage: vec![],
                cwd: None,
                git: None,
                created_at: None,
            },
            Cursor::default(),
        )
    }

    pub fn resume(mut reader: R, session: Session, cursor: Cursor) -> io::Result<Self> {
        if session.source.provider != Provider::Codex
            || session.source.parser_revision != PARSER_REVISION
        {
            return Err(invalid("checkpoint is not from this Codex parser revision"));
        }
        reader.seek(SeekFrom::Start(cursor.offset))?;
        Ok(Self {
            reader,
            session,
            cursor,
            retain_raw: true,
        })
    }

    pub fn without_raw(mut self) -> Self {
        self.retain_raw = false;
        self
    }
    pub fn into_parts(self) -> (R, Session, Cursor) {
        (self.reader, self.session, self.cursor)
    }

    pub fn session(&self) -> &Session {
        &self.session
    }
    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }

    /// Return one complete native record and any derived compaction messages.
    /// EOF or a non-newline-terminated tail returns None without advancing cursor.
    /// CR in CRLF is retained in Raw so the writer reproduces the exact bytes.
    pub fn next_record(&mut self) -> io::Result<Option<Vec<Event>>> {
        let mut line = Vec::new();
        let count = self
            .reader
            .by_ref()
            .take((MAX_LINE_BYTES + 2) as u64)
            .read_until(b'\n', &mut line)?;
        if count == 0 {
            return Ok(None);
        }
        if count > MAX_LINE_BYTES + 1 {
            self.reader.seek(SeekFrom::Start(self.cursor.offset))?;
            return Err(invalid("native line exceeds 64 MiB"));
        }
        if line.last() != Some(&b'\n') {
            self.reader.seek(SeekFrom::Start(self.cursor.offset))?;
            return Ok(None);
        }
        line.pop();
        let row = serde_json::from_slice::<Value>(&line);
        let ordinal = self.cursor.line_ordinal;
        let mut derived = vec![];
        let (kind, fidelity, at, key) = match row {
            Ok(row) => {
                let key = native_key(&row, ordinal);
                let at = row
                    .get("timestamp")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let mapped = self.map(&row, &key, &mut derived);
                match mapped {
                    Some((kind, fidelity)) => (kind, fidelity, at, key),
                    None => {
                        self.cursor.parse_errors += 1;
                        (EventKind::Unknown, Fidelity::Opaque, at, key)
                    }
                }
            }
            Err(error) => {
                self.cursor.parse_errors += 1;
                (
                    EventKind::Notice {
                        level: Level::Warn,
                        text: format!("Codex JSON parse error at line {ordinal}: {error}"),
                    },
                    Fidelity::Partial,
                    None,
                    ordinal.to_string(),
                )
            }
        };
        let id = EventId::native(Provider::Codex, &self.session.source.native_id, &key);
        if matches!(kind, EventKind::TurnStart { .. }) {
            self.cursor.turn = Some(TurnId(row_turn_id(&line).unwrap_or_else(|| key.clone())));
        }
        let raw = self.retain_raw.then_some(Raw {
            provider: Provider::Codex,
            bytes: line,
        });
        let mut events = vec![self.event(
            id,
            at.clone(),
            if self.retain_raw {
                fidelity
            } else {
                Fidelity::Partial
            },
            raw,
            kind,
        )];
        for (id, kind) in derived {
            events.push(self.event(id, at.clone(), Fidelity::Inferred, None, kind));
        }
        if matches!(events[0].kind, EventKind::TurnEnd { .. }) {
            self.cursor.turn = None;
        }
        self.cursor.offset = self
            .cursor
            .offset
            .checked_add(count as u64)
            .ok_or_else(|| invalid("offset overflow"))?;
        self.cursor.line_ordinal += 1;
        Ok(Some(events))
    }

    fn event(
        &mut self,
        id: EventId,
        at: Option<Timestamp>,
        fidelity: Fidelity,
        raw: Option<Raw>,
        kind: EventKind,
    ) -> Event {
        let event = Event {
            seq: self.cursor.next_seq,
            id,
            parent: self.cursor.previous,
            turn: self.cursor.turn.clone(),
            at,
            fidelity,
            raw,
            kind,
        };
        self.cursor.next_seq += 1;
        self.cursor.previous = Some(id);
        event
    }

    fn map(
        &mut self,
        row: &Value,
        key: &str,
        derived: &mut Vec<(EventId, EventKind)>,
    ) -> Option<(EventKind, Fidelity)> {
        let payload = row.get("payload")?;
        let kind = match row.get("type")?.as_str()? {
            "session_meta" if self.cursor.line_ordinal == 0 => {
                self.session.source.native_id = payload
                    .get("id")
                    .or_else(|| payload.get("session_id"))?
                    .as_str()?
                    .to_owned();
                self.session.source.cli_version = text(payload, "cli_version");
                self.session.cwd = text(payload, "cwd").map(Into::into);
                self.session.created_at =
                    text(payload, "timestamp").or_else(|| text(row, "timestamp"));
                self.session.git = payload.get("git").map(|g| Git {
                    branch: text(g, "branch"),
                    commit: text(g, "commit_hash"),
                    repo_url: text(g, "repository_url"),
                });
                let instructions = payload
                    .get("base_instructions")
                    .and_then(|v| v.as_str().or_else(|| v.get("text")?.as_str()));
                match instructions {
                    Some(s) => EventKind::Message {
                        role: Role::System,
                        blocks: vec![Block::Text { text: s.to_owned() }],
                    },
                    None => EventKind::Native {
                        provider: Provider::Codex,
                        record_type: "session_meta".into(),
                    },
                }
            }
            "turn_context" => EventKind::Context {
                changes: payload.as_object()?.clone().into_iter().collect(),
            },
            "response_item" => {
                return response(payload).map(|kind| {
                    let fidelity = if matches!(
                        kind,
                        EventKind::Reasoning {
                            sealed: Some(_),
                            ..
                        } | EventKind::Compaction { .. }
                    ) {
                        Fidelity::Opaque
                    } else {
                        Fidelity::Partial
                    };
                    (kind, fidelity)
                });
            }
            "event_msg" => event_message(payload)?,
            "compacted" => {
                let history = payload.get("replacement_history").and_then(Value::as_array);
                let mut replacement = vec![];
                if let Some(history) = history {
                    for (index, item) in history.iter().enumerate() {
                        if item.get("type").and_then(Value::as_str) == Some("message") {
                            let id = EventId::inferred(
                                Provider::Codex,
                                &self.session.source.native_id,
                                key,
                                &format!("#replacement-{index}"),
                            );
                            derived.push((id, response(item).unwrap_or(EventKind::Unknown)));
                            replacement.push(id);
                        }
                    }
                }
                let sealed = payload
                    .get("compaction")
                    .and_then(|v| seal(v, SealedForm::EncryptedCompaction))
                    .or_else(|| seal(payload, SealedForm::EncryptedCompaction))
                    .or_else(|| {
                        history?.iter().find_map(|v| {
                            if v.get("type")?.as_str()? == "compaction" {
                                seal(v, SealedForm::EncryptedCompaction)
                            } else {
                                None
                            }
                        })
                    });
                return Some((
                    EventKind::Compaction {
                        summary: text(payload, "message").map(|s| vec![Block::Text { text: s }]),
                        replaced_through: None,
                        replacement,
                        sealed,
                    },
                    Fidelity::Opaque,
                ));
            }
            "token_usage_record" | "world_state" | "inter_agent_communication_metadata" => {
                return Some((
                    EventKind::Native {
                        provider: Provider::Codex,
                        record_type: row["type"].as_str()?.into(),
                    },
                    Fidelity::Opaque,
                ));
            }
            _ => return None,
        };
        Some((kind, Fidelity::Partial))
    }
}

fn row_turn_id(line: &[u8]) -> Option<String> {
    let row: Value = serde_json::from_slice(line).ok()?;
    text(row.get("payload")?, "turn_id")
}

// Only an item's own identity is used. An output's call_id is a foreign key to
// its call, not its own id; using it again would collide with the call event.
fn native_key(row: &Value, ordinal: u64) -> String {
    let item = &row["payload"];
    if row["type"] == "response_item" {
        let id = match item["type"].as_str() {
            Some("function_call" | "custom_tool_call") => item.get("call_id"),
            _ => item.get("id"),
        };
        if let Some(id) = id.and_then(Value::as_str) {
            return id.to_owned();
        }
    }
    row.get("ordinal")
        .and_then(|n| {
            n.as_str()
                .map(str::to_owned)
                .or_else(|| n.as_u64().map(|n| n.to_string()))
        })
        .unwrap_or_else(|| ordinal.to_string())
}

fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(str::to_owned)
}

fn seal(value: &Value, form: SealedForm) -> Option<Sealed> {
    // lc-ir Provider::Codex denotes OpenAI-native reasoning; OpenAI is not a
    // separate IR provider. The opaque wire string is retained without decoding.
    Some(Sealed {
        provider: Provider::Codex,
        form,
        bytes: value
            .get("encrypted_content")?
            .as_str()?
            .as_bytes()
            .to_vec(),
    })
}

fn response(item: &Value) -> Option<EventKind> {
    Some(match item.get("type")?.as_str()? {
        "message" => EventKind::Message {
            role: match item.get("role")?.as_str()? {
                "user" => Role::User,
                "assistant" => Role::Assistant,
                "developer" => Role::Developer,
                "system" => Role::System,
                _ => Role::Unknown,
            },
            blocks: blocks(item.get("content")?)?,
        },
        "reasoning" => EventKind::Reasoning {
            text: item.get("content").and_then(Value::as_array).map(|a| {
                a.iter()
                    .filter_map(|v| v.get("text")?.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            }),
            summary: item.get("summary").and_then(Value::as_array).map(|a| {
                a.iter()
                    .filter_map(|v| v.get("text")?.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            }),
            sealed: seal(item, SealedForm::EncryptedReasoning),
        },
        "compaction" => EventKind::Compaction {
            summary: None,
            replaced_through: None,
            replacement: vec![],
            sealed: seal(item, SealedForm::EncryptedCompaction),
        },
        "function_call" | "custom_tool_call" => {
            let name = text(item, "name")?;
            let input = if item["type"] == "function_call" {
                ToolInput::Json(serde_json::from_str(item.get("arguments")?.as_str()?).ok()?)
            } else {
                ToolInput::Text(text(item, "input")?)
            };
            EventKind::ToolCall {
                call_id: text(item, "call_id")?,
                tool_kind: tool_kind(&name),
                name,
                namespace: text(item, "namespace"),
                input,
            }
        }
        "function_call_output" | "custom_tool_call_output" => EventKind::ToolResult {
            call_id: text(item, "call_id")?,
            is_error: item
                .get("is_error")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            blocks: blocks(item.get("output")?)?,
        },
        _ => return None,
    })
}

fn blocks(value: &Value) -> Option<Vec<Block>> {
    if let Some(text) = value.as_str() {
        return Some(vec![Block::Text {
            text: text.to_owned(),
        }]);
    }
    value
        .as_array()?
        .iter()
        .map(|v| {
            Some(match v.get("type")?.as_str()? {
                "input_text" | "output_text" | "text" => Block::Text {
                    text: text(v, "text")?,
                },
                "input_image" => {
                    // Only inline bodies have a content hash here. Remote images are
                    // not fetched; raw retains their URI and Block::Unknown marks loss.
                    let image = v.get("image_url").and_then(Value::as_str).unwrap_or("");
                    if let Some((mime, body)) = image
                        .strip_prefix("data:")
                        .and_then(|s| s.split_once(";base64,"))
                    {
                        let body = STANDARD.decode(body).ok()?;
                        Block::Image {
                            mime: mime.into(),
                            data: BlobRef {
                                blake3: *blake3::hash(&body).as_bytes(),
                                len: body.len() as u64,
                            },
                        }
                    } else {
                        Block::Unknown
                    }
                }
                _ => Block::Unknown,
            })
        })
        .collect()
}

fn event_message(payload: &Value) -> Option<EventKind> {
    Some(match payload.get("type")?.as_str()? {
        "task_started" | "turn_started" => EventKind::TurnStart {
            origin: TurnOrigin::User,
        },
        "task_complete" | "task_completed" | "turn_complete" | "turn_completed" => {
            EventKind::TurnEnd {
                outcome: TurnOutcome::Completed,
            }
        }
        "turn_aborted" => EventKind::TurnEnd {
            outcome: TurnOutcome::Interrupted,
        },
        "user_message" | "agent_message" => EventKind::Message {
            role: if payload["type"] == "user_message" {
                Role::User
            } else {
                Role::Assistant
            },
            blocks: vec![Block::Text {
                text: text(payload, "message")?,
            }],
        },
        "agent_reasoning" => EventKind::Reasoning {
            text: text(payload, "text"),
            summary: None,
            sealed: None,
        },
        "error" | "warning" => EventKind::Notice {
            level: if payload["type"] == "error" {
                Level::Error
            } else {
                Level::Warn
            },
            text: text(payload, "message")?,
        },
        "token_count" => {
            let info = payload.get("info")?;
            let usage = info
                .get("last_token_usage")
                .filter(|v| v.is_object())
                .or_else(|| info.get("total_token_usage"))?;
            let number = |keys: &[&str]| -> Option<u64> {
                for key in keys {
                    if let Some(n) = usage.get(key) {
                        return n.as_u64();
                    }
                }
                Some(0)
            };
            EventKind::Usage {
                input: number(&["input_tokens"])?,
                output: number(&["output_tokens"])?,
                cache_read: number(&["cached_input_tokens", "cache_read_input_tokens"])?,
                cache_write: number(&["cache_write_input_tokens", "cache_creation_input_tokens"])?,
                reasoning: number(&["reasoning_output_tokens"])?,
            }
        }
        _ => return None,
    })
}

fn tool_kind(name: &str) -> ToolKind {
    match name {
        "exec" | "exec_command" | "shell" => ToolKind::Execute,
        "apply_patch" => ToolKind::Edit,
        _ => ToolKind::Other,
    }
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
