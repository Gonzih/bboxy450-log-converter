// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

//! Streaming same-provider replay. Cross-provider synthesis belongs to P6.
use crate::ir::{Event, EventId, EventKind, Fidelity, Provider, Session};
use std::{
    borrow::Borrow,
    io::{self, Write},
};

/// Input must be in dense seq order. No client file or database is opened here.
/// Retained records are written verbatim; only their removed LF is restored.
/// Unsupported non-raw events fail rather than disappearing silently.
pub fn write<W, I, E>(session: &Session, events: I, mut output: W) -> io::Result<()>
where
    W: Write,
    I: IntoIterator<Item = E>,
    E: Borrow<Event>,
{
    if session.source.provider != Provider::Codex {
        return Err(invalid("not a Codex source session"));
    }
    let mut first = true;
    let mut derived: Vec<EventId> = vec![];
    for (seq, event) in events.into_iter().enumerate() {
        let event = event.borrow();
        if event.seq != seq as u64 {
            return Err(invalid("events must be in dense seq order from zero"));
        }
        let Some(raw) = &event.raw else {
            if event.fidelity == Fidelity::Inferred && derived.contains(&event.id) {
                continue;
            }
            return Err(invalid("non-raw event needs the P6 synthesis path"));
        };
        if raw.provider != Provider::Codex || raw.bytes.contains(&b'\n') {
            return Err(invalid("not one retained Codex native line"));
        }
        if first {
            let value: serde_json::Value = serde_json::from_slice(&raw.bytes)
                .map_err(|_| invalid("first record is not session_meta"))?;
            if value["type"] != "session_meta"
                || value["payload"]["id"]
                    .as_str()
                    .or_else(|| value["payload"]["session_id"].as_str())
                    != Some(&session.source.native_id)
            {
                return Err(invalid("first record must be this session's session_meta"));
            }
            first = false;
        }
        derived = match &event.kind {
            EventKind::Compaction { replacement, .. } => replacement.clone(),
            _ => vec![],
        };
        output.write_all(&raw.bytes)?;
        output.write_all(b"\n")?;
    }
    if first {
        return Err(invalid("session_meta is required"));
    }
    Ok(())
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
