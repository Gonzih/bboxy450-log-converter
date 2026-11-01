// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

use bboxy450_log_converter::{
    ir::*,
    lower::codex::write,
    parse::codex::{Cursor as Checkpoint, Parser},
};
use serde_json::json;
use std::io::{BufReader, Cursor};
#[path = "support/codex_fixture.rs"]
mod fixture;
fn id() -> SessionId {
    SessionId(ulid::Ulid::from(4u128))
}
fn parse(bytes: &[u8]) -> (Session, Vec<Event>, Checkpoint) {
    let mut parser = Parser::new(BufReader::with_capacity(7, Cursor::new(bytes)), id()).unwrap();
    let mut events = vec![];
    while let Some(batch) = parser.next_record().unwrap() {
        events.extend(batch);
    }
    (parser.session().clone(), events, parser.cursor().clone())
}
#[test]
fn codex_roundtrip_raw_and_ir_are_identical() {
    let bytes = fixture::comprehensive();
    let (session, events, cursor) = parse(&bytes);
    let mut lowered = vec![];
    write(&session, &events, &mut lowered).unwrap();
    assert_eq!(lowered, bytes);
    assert_eq!(parse(&lowered), (session.clone(), events.clone(), cursor));
    assert_eq!(session.source.native_id, fixture::SID);
    assert_eq!(session.cwd.unwrap().to_str(), Some("/synthetic"));
    for (seq, event) in events.iter().enumerate() {
        assert_eq!(event.seq, seq as u64);
    }
    assert_eq!(events[0].parent, None);
    for pair in events.windows(2) {
        assert_eq!(pair[1].parent, Some(pair[0].id));
    }
    assert_eq!(
        events
            .iter()
            .map(|e| e.id)
            .collect::<std::collections::HashSet<_>>()
            .len(),
        events.len()
    );
}
#[test]
fn codex_maps_the_full_fixture() {
    let (_, e, _) = parse(&fixture::comprehensive());
    assert!(e.iter().any(|e| matches!(
        &e.kind,
        EventKind::Message {
            role: Role::System,
            ..
        }
    )));
    assert!(e.iter().any(|e| matches!(
        &e.kind,
        EventKind::Message {
            role: Role::Developer,
            ..
        }
    )));
    assert!(
        e.iter().any(
            |e| matches!(&e.kind,EventKind::Context{changes} if changes["model"]=="synthetic")
        )
    );
    assert!(e.iter().any(|e|matches!(&e.kind,EventKind::Message{blocks,..} if blocks.iter().any(|b|matches!(b,Block::Image{data,..} if data.len==3 && data.blake3==*blake3::hash(b"abc").as_bytes())))));
    let reason = e
        .iter()
        .find(|e| {
            matches!(
                &e.kind,
                EventKind::Reasoning {
                    sealed: Some(_),
                    ..
                }
            )
        })
        .unwrap();
    assert!(
        matches!(&reason.kind,EventKind::Reasoning{text:Some(t),summary:Some(s),sealed:Some(z)} if t=="readable reasoning" && s=="readable summary" && z.provider==Provider::Codex && z.form==SealedForm::EncryptedReasoning && z.bytes==b"LC_SYNTHETIC_SEALED_REASONING")
    );
    assert_eq!(
        reason.id,
        EventId::native(Provider::Codex, fixture::SID, "rs_synthetic")
    );
    let call = e
        .iter()
        .find(|e| matches!(&e.kind,EventKind::ToolCall{call_id,..} if call_id=="call_fn"))
        .unwrap();
    let output = e
        .iter()
        .find(|e| matches!(&e.kind,EventKind::ToolResult{call_id,..} if call_id=="call_fn"))
        .unwrap();
    assert_eq!(
        call.id,
        EventId::native(Provider::Codex, fixture::SID, "call_fn")
    );
    assert_ne!(call.id, output.id); // An output's call_id refers to its call.
    assert!(
        matches!(&call.kind,EventKind::ToolCall{input:ToolInput::Json(v),namespace:Some(n),tool_kind:ToolKind::Execute,..} if v["cmd"]=="synthetic" && n=="functions")
    );
    assert!(e.iter().any(|e|matches!(&e.kind,EventKind::ToolCall{input:ToolInput::Text(t),..} if t=="synthetic JavaScript text")));
    assert!(
        e.iter()
            .any(|e| matches!(&e.kind, EventKind::ToolResult { is_error: true, .. }))
    );
    assert!(e.iter().any(|e| matches!(
        e.kind,
        EventKind::Usage {
            input: 11,
            output: 12,
            cache_read: 13,
            cache_write: 14,
            reasoning: 15
        }
    )));
    assert_eq!(
        e.iter()
            .filter(|e| matches!(e.kind, EventKind::Unknown))
            .count(),
        2
    );
    for event in &e {
        if let EventKind::Compaction {
            replacement,
            sealed: Some(sealed),
            ..
        } = &event.kind
        {
            assert_eq!(sealed.provider, Provider::Codex);
            assert_eq!(sealed.form, SealedForm::EncryptedCompaction);
            assert_eq!(event.fidelity, Fidelity::Opaque);
            for id in replacement {
                assert!(
                    e.iter().any(|p| p.id == *id
                        && p.fidelity == Fidelity::Inferred
                        && p.raw.is_none())
                );
            }
        }
    }
    let starts = e
        .iter()
        .filter(|e| matches!(e.kind, EventKind::TurnStart { .. }))
        .collect::<Vec<_>>();
    assert_eq!(starts[0].turn, Some(TurnId("turn-one".into())));
    assert_eq!(starts[1].turn, Some(TurnId("turn-two".into())));
    assert!(e.iter().any(|e| matches!(
        e.kind,
        EventKind::TurnEnd {
            outcome: TurnOutcome::Interrupted
        }
    )));
}
#[test]
fn codex_determinism_includes_cbor() {
    let bytes = fixture::comprehensive();
    let a = parse(&bytes);
    let b = parse(&bytes);
    assert_eq!(a, b);
    assert_eq!(
        a.1.iter()
            .map(|e| cbor::encode(e).unwrap())
            .collect::<Vec<_>>(),
        b.1.iter()
            .map(|e| cbor::encode(e).unwrap())
            .collect::<Vec<_>>()
    );
}
#[test]
fn codex_checkpoint_at_every_byte_boundary_tolerates_truncated_utf8_and_json() {
    let bytes = fixture::comprehensive();
    let expected = parse(&bytes);
    for cut in 0..bytes.len() {
        let (session, mut prefix, cursor) = parse(&bytes[..cut]);
        let through = bytes[..cut]
            .iter()
            .rposition(|b| *b == b'\n')
            .map_or(0, |i| i + 1);
        assert_eq!(cursor.offset, through as u64);
        // Serialize the checkpoint as P7 will, then reconstruct a fresh reader.
        let restored = serde_json::from_slice(&serde_json::to_vec(&cursor).unwrap()).unwrap();
        let mut resumed = Parser::resume(Cursor::new(&bytes), session, restored).unwrap();
        while let Some(batch) = resumed.next_record().unwrap() {
            prefix.extend(batch);
        }
        assert_eq!(
            (resumed.session().clone(), prefix, resumed.cursor().clone()),
            expected,
            "cut {cut}"
        );
    }
}
#[test]
fn codex_partial_tail_does_not_advance_an_open_reader() {
    let bytes = fixture::resumable("/synthetic");
    let mut parser = Parser::new(Cursor::new(bytes[..bytes.len() - 1].to_vec()), id()).unwrap();
    while parser.next_record().unwrap().is_some() {}
    let before = parser.cursor().clone();
    assert!(parser.next_record().unwrap().is_none());
    assert_eq!(parser.cursor(), &before);
    let (mut reader, session, cursor) = parser.into_parts();
    reader.get_mut().push(b'\n');
    let mut parser = Parser::resume(reader, session, cursor).unwrap();
    assert!(parser.next_record().unwrap().is_some());
    assert_eq!(parser.cursor().offset, bytes.len() as u64);
}
#[test]
fn codex_malformed_and_unknown_lines_are_retained_without_panics() {
    let mut bytes = fixture::encode(vec![fixture::header("/synthetic")]);
    bytes.extend_from_slice(b"not json\n\xff\n42\n{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"arguments\":\"{\"}}\n");
    let (session, events, cursor) = parse(&bytes);
    assert_eq!(cursor.parse_errors, 4);
    assert!(matches!(
        events[1].kind,
        EventKind::Notice {
            level: Level::Warn,
            ..
        }
    ));
    assert!(matches!(events[3].kind, EventKind::Unknown));
    let mut lowered = vec![];
    write(&session, &events, &mut lowered).unwrap();
    assert_eq!(lowered, bytes);
    for byte in 0..=255 {
        let mut candidate = fixture::encode(vec![fixture::header("/synthetic")]);
        candidate.extend([byte, b'\n']);
        let _ = parse(&candidate);
    }
}
#[test]
fn codex_no_raw_preserves_sealed_and_readable_reasoning() {
    let mut parser = Parser::new(Cursor::new(fixture::comprehensive()), id())
        .unwrap()
        .without_raw();
    let mut sealed = 0;
    while let Some(events) = parser.next_record().unwrap() {
        for event in events {
            assert!(event.raw.is_none());
            if let EventKind::Reasoning {
                summary: Some(s),
                sealed: Some(z),
                ..
            } = event.kind
            {
                assert_eq!(s, "readable summary");
                assert!(!z.bytes.is_empty());
                sealed += 1;
            }
        }
    }
    assert_eq!(sealed, 1);
}
#[test]
fn codex_lowering_refuses_missing_meta_nonraw_foreign_and_reordering() {
    let (session, mut events, _) = parse(&fixture::comprehensive());
    assert!(write(&session, &events[1..], vec![]).is_err());
    events.swap(1, 2);
    assert!(write(&session, &events, vec![]).is_err());
    events.swap(1, 2);
    events[1].raw = None;
    assert!(write(&session, &events, vec![]).is_err());
    let mut foreign = session.clone();
    foreign.source.provider = Provider::Claude;
    assert!(write(&foreign, &events, vec![]).is_err());
    assert!(Parser::resume(Cursor::new(vec![]), foreign, Checkpoint::default()).is_err());
}
#[test]
fn codex_reasoning_summary_without_encryption_and_total_usage_fallback() {
    let bytes = fixture::encode(vec![
        fixture::header("/synthetic"),
        json!({"type":"response_item","payload":{"type":"reasoning","summary":[{"text":"summary only"}]}}),
        json!({"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":5}}}}),
    ]);
    let (_, events, _) = parse(&bytes);
    assert!(
        matches!(&events[1].kind,EventKind::Reasoning{summary:Some(s),sealed:None,..} if s=="summary only")
    );
    assert!(matches!(
        events[2].kind,
        EventKind::Usage {
            input: 5,
            output: 0,
            ..
        }
    ));
}
