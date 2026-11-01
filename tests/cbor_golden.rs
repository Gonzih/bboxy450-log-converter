// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

use bboxy450_log_converter::ir::{cbor, *};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn session() -> Session {
    serde_json::from_value(json!({
        "id":"00000000000000000000000001",
        "source":{"provider":"claude","native_id":"synthetic","cli_version":"1.0","parser_revision":1},
        "lineage":[{"relation":"fork_of","session":"00000000000000000000000002","native":["codex","parent"],"at":vec![2;16]}],
        "cwd":"/synthetic","git":{"branch":"main","commit":"abc","repo_url":"https://example.test/repo"},
        "created_at":"2026-01-01T00:00:00.000Z"
    })).unwrap()
}
fn event(kind: Value) -> Event {
    serde_json::from_value(json!({
        "seq":0,"id":EventId::native(Provider::Claude,"synthetic","uuid-0"),"parent":vec![2;16],"turn":"turn-0","at":"2026-01-01T00:00:00.000Z",
        "fidelity":"full","raw":{"provider":"claude","bytes":[0,255,10]},"kind":kind
    }))
    .unwrap()
}
fn kinds() -> Vec<(&'static str, Value)> {
    let blob = json!({"blake3":vec![3;32],"len":4});
    let sealed = json!({"provider":"claude","form":"thinking_signature","bytes":[0,255,42]});
    vec![
        ("turn_start", json!({"kind":"turn_start","origin":"user"})),
        ("turn_end", json!({"kind":"turn_end","outcome":"completed"})),
        (
            "message",
            json!({"kind":"message","role":"assistant","blocks":[
            {"kind":"text","text":"hi"},{"kind":"image","mime":"image/png","data":blob},
            {"kind":"data","mime":"application/octet-stream","data":blob},{"kind":"unknown"}]}),
        ),
        (
            "tool_json",
            json!({"kind":"tool_call","call_id":"call-0","name":"read","namespace":"test","tool_kind":"read","input":{"kind":"json","value":{"z":1,"a":[true,null,2.5]}}}),
        ),
        (
            "tool_text",
            json!({"kind":"tool_call","call_id":"call-0","name":"exec","namespace":null,"tool_kind":"execute","input":{"kind":"text","value":"echo hello"}}),
        ),
        (
            "tool_unknown",
            json!({"kind":"tool_call","call_id":"call-0","name":"future","namespace":null,"tool_kind":"unknown","input":{"kind":"unknown"}}),
        ),
        (
            "tool_result",
            json!({"kind":"tool_result","call_id":"call-0","is_error":true,"blocks":[{"kind":"text","text":"failed"}]}),
        ),
        (
            "reasoning",
            json!({"kind":"reasoning","text":"private thought","summary":"summary","sealed":sealed}),
        ),
        (
            "compaction",
            json!({"kind":"compaction","summary":[{"kind":"text","text":"digest"}],"replaced_through":vec![2;16],"replacement":[vec![1;16]],"sealed":sealed}),
        ),
        (
            "context",
            json!({"kind":"context","changes":{"model":"synthetic","permissions":{"mode":"test"},"budget":100}}),
        ),
        (
            "usage",
            json!({"kind":"usage","input":1,"output":2,"cache_read":3,"cache_write":4,"reasoning":5}),
        ),
        (
            "notice",
            json!({"kind":"notice","level":"warn","text":"synthetic warning"}),
        ),
        (
            "subagent",
            json!({"kind":"subagent","session":"00000000000000000000000002","relation":"subagent_of"}),
        ),
        (
            "native",
            json!({"kind":"native","provider":"codex","record_type":"future_record"}),
        ),
        ("unknown", json!({"kind":"unknown"})),
    ]
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn cases() -> BTreeMap<String, Vec<u8>> {
    let mut cases = BTreeMap::new();
    let session = session();
    let bytes = cbor::encode(&session).unwrap();
    assert_eq!(cbor::decode::<Session>(&bytes).unwrap().value(), &session);
    cases.insert("session".into(), bytes);
    let empty = Session {
        cwd: None,
        git: None,
        created_at: None,
        lineage: vec![],
        ..session
    };
    let bytes = cbor::encode(&empty).unwrap();
    assert_eq!(cbor::decode::<Session>(&bytes).unwrap().value(), &empty);
    cases.insert("session_minimal".into(), bytes);
    for (name, kind) in kinds() {
        let event = event(kind);
        let bytes = cbor::encode(&event).unwrap();
        assert_eq!(
            cbor::decode::<Event>(&bytes).unwrap().value(),
            &event,
            "{name}"
        );
        cases.insert(name.into(), bytes);
    }
    // Every unit enum variant, including explicit Unknown, has a stable ordinal.
    for (field, names) in [
        (
            "fidelity",
            vec![
                "full",
                "partial",
                "inferred",
                "summary_only",
                "opaque",
                "unknown",
            ],
        ),
        ("provider", vec!["claude", "codex", "bboxy", "unknown"]),
        (
            "relation",
            vec![
                "fork_of",
                "resume_of",
                "switched_from",
                "subagent_of",
                "compacted_from",
                "imported_from",
                "unknown",
            ],
        ),
        ("origin", vec!["user", "resume", "system", "unknown"]),
        (
            "outcome",
            vec!["completed", "interrupted", "failed", "unknown"],
        ),
        (
            "role",
            vec!["user", "assistant", "system", "developer", "unknown"],
        ),
        (
            "tool_kind",
            vec![
                "read", "edit", "delete", "move", "search", "execute", "think", "fetch", "other",
                "unknown",
            ],
        ),
        ("level", vec!["info", "warn", "error", "unknown"]),
        (
            "form",
            vec![
                "thinking_signature",
                "redacted_thinking",
                "encrypted_reasoning",
                "encrypted_compaction",
                "unknown",
            ],
        ),
    ] {
        for name in names {
            let mut e = serde_json::to_value(event(json!({"kind":"unknown"}))).unwrap();
            e["kind"] = match field {
                "origin" => json!({"kind":"turn_start","origin":name}),
                "outcome" => json!({"kind":"turn_end","outcome":name}),
                "role" => json!({"kind":"message","role":name,"blocks":[]}),
                "tool_kind" => {
                    json!({"kind":"tool_call","call_id":"c","name":"n","namespace":null,"tool_kind":name,"input":{"kind":"text","value":"x"}})
                }
                "level" => json!({"kind":"notice","level":name,"text":"x"}),
                "form" => {
                    json!({"kind":"reasoning","text":null,"summary":null,"sealed":{"provider":"codex","form":name,"bytes":[]}})
                }
                "provider" => json!({"kind":"native","provider":name,"record_type":"x"}),
                "relation" => {
                    json!({"kind":"subagent","session":"00000000000000000000000002","relation":name})
                }
                _ => {
                    e["fidelity"] = json!(name);
                    json!({"kind":"unknown"})
                }
            };
            let e: Event = serde_json::from_value(e).unwrap();
            let bytes = cbor::encode(&e).unwrap();
            assert_eq!(cbor::decode::<Event>(&bytes).unwrap().value(), &e);
            cases.insert(format!("{field}_{name}"), bytes);
        }
    }
    let minimal = Event {
        seq: u64::MAX,
        id: EventId([0; 16]),
        parent: None,
        turn: None,
        at: None,
        fidelity: Fidelity::Inferred,
        raw: None,
        kind: EventKind::Reasoning {
            text: None,
            summary: None,
            sealed: None,
        },
    };
    let bytes = cbor::encode(&minimal).unwrap();
    assert_eq!(cbor::decode::<Event>(&bytes).unwrap().value(), &minimal);
    cases.insert("event_minimal".into(), bytes);
    // A retained future field exercises the mixed-key order from RFC 8949 §4.2.1.
    use ciborium::value::Value as Cbor;
    let mut future: Cbor = ciborium::de::from_reader(cases["unknown"].as_slice()).unwrap();
    let keys = [
        Cbor::Text("a".into()),
        Cbor::from(-1),
        Cbor::from(100),
        Cbor::from(24),
        Cbor::from(23),
        Cbor::from(10),
    ];
    let Cbor::Map(fields) = &mut future else {
        panic!("event map")
    };
    fields.push((
        Cbor::from(99),
        Cbor::Map(keys.into_iter().map(|key| (key, Cbor::Null)).collect()),
    ));
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(&future, &mut bytes).unwrap();
    let decoded = cbor::decode::<Event>(&bytes).unwrap();
    assert_eq!(decoded.value().kind, EventKind::Unknown);
    cases.insert("future_mixed_keys".into(), decoded.encode().unwrap());
    cases
}
#[test]
fn every_variant_has_a_fixed_golden_and_round_trip() {
    let actual = cases()
        .into_iter()
        .map(|(k, v)| format!("{k} {}", hex(&v)))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let expected = include_str!("fixtures/ir-v0.1.hex")
        .lines()
        .filter(|s| !s.starts_with('#') && !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_eq!(
        actual, expected,
        "CBOR field/discriminant drift requires an intentional version review"
    );
}
