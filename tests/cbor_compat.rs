// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

use bboxy450_log_converter::ir::{cbor, *};
use ciborium::value::Value;

fn fixture(name: &str) -> Vec<u8> {
    let line = include_str!("fixtures/ir-v0.1.hex")
        .lines()
        .find(|s| s.starts_with(&format!("{name} ")))
        .unwrap();
    let hex = line.split_once(' ').unwrap().1;
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|v| u8::from_str_radix(std::str::from_utf8(v).unwrap(), 16).unwrap())
        .collect()
}
fn wire(bytes: &[u8]) -> Value {
    ciborium::de::from_reader(bytes).unwrap()
}
fn bytes(value: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    ciborium::ser::into_writer(value, &mut out).unwrap();
    out
}
fn entries(value: &mut Value) -> &mut Vec<(Value, Value)> {
    match value {
        Value::Map(entries) => entries,
        _ => panic!("map"),
    }
}
fn field(value: &mut Value, key: u64) -> &mut Value {
    &mut entries(value)
        .iter_mut()
        .find(|(k, _)| *k == Value::from(key))
        .unwrap()
        .1
}
fn item(value: &mut Value, index: usize) -> &mut Value {
    match value {
        Value::Array(items) => &mut items[index],
        _ => panic!("array"),
    }
}
fn future(value: &mut Value) {
    entries(value).push((
        Value::from(99),
        Value::Tag(
            1000,
            Box::new(Value::Map(vec![(
                Value::Text("future".into()),
                Value::Bytes(vec![0, 255, 128]),
            )])),
        ),
    ));
}
fn assert_preserved_event(value: Value) -> Event {
    let encoded = bytes(&value);
    let decoded = cbor::decode::<Event>(&encoded).unwrap();
    // The fixture was in canonical order before adding the highest key.
    assert_eq!(decoded.encode().unwrap(), encoded);
    decoded.into_value()
}

#[test]
fn unknown_fields_survive_at_each_event_record_depth() {
    let mut value = wire(&fixture("reasoning"));
    future(&mut value);
    future(field(&mut value, 6)); // Raw
    future(field(&mut value, 7)); // known Reasoning union
    future(field(field(&mut value, 7), 3)); // Sealed
    let event = assert_preserved_event(value);
    assert_eq!(event.raw.unwrap().bytes, vec![0, 255, 10]);
    match event.kind {
        EventKind::Reasoning {
            text,
            summary,
            sealed,
        } => {
            assert_eq!(text.as_deref(), Some("private thought"));
            assert_eq!(summary.as_deref(), Some("summary"));
            let sealed = sealed.unwrap();
            assert_eq!(sealed.provider, Provider::Claude);
            assert_eq!(sealed.bytes, vec![0, 255, 42]);
        }
        _ => panic!("reasoning was folded/lost"),
    }
    let mut value = wire(&fixture("message"));
    let blocks = field(field(&mut value, 7), 2);
    future(item(blocks, 0));
    future(field(item(blocks, 1), 2)); // BlobRef
    assert_preserved_event(value);
}

#[test]
fn unknown_session_source_git_and_lineage_keys_survive() {
    let mut value = wire(&fixture("session"));
    future(&mut value);
    future(field(&mut value, 1));
    future(field(&mut value, 4));
    future(item(field(&mut value, 2), 0));
    let encoded = bytes(&value);
    let decoded = cbor::decode::<Session>(&encoded).unwrap();
    assert_eq!(decoded.encode().unwrap(), encoded);
    assert_eq!(decoded.value().source.native_id, "synthetic");
}

#[test]
fn future_union_and_unit_tags_project_unknown_and_forward_losslessly() {
    let mut value = wire(&fixture("unknown"));
    *field(field(&mut value, 7), 0) = Value::from(1024);
    future(field(&mut value, 7));
    *field(&mut value, 5) = Value::from(1024);
    let event = assert_preserved_event(value);
    assert_eq!(event.kind, EventKind::Unknown);
    assert_eq!(event.fidelity, Fidelity::Unknown);
    let mut value = wire(&fixture("message"));
    let block = item(field(field(&mut value, 7), 2), 3);
    *field(block, 0) = Value::from(1024);
    future(block);
    let event = assert_preserved_event(value);
    match event.kind {
        EventKind::Message { blocks, .. } => assert_eq!(blocks.last(), Some(&Block::Unknown)),
        _ => panic!(),
    }
    let mut value = wire(&fixture("tool_unknown"));
    let input = field(field(&mut value, 7), 5);
    *field(input, 0) = Value::from(1024);
    future(input);
    let event = assert_preserved_event(value);
    match event.kind {
        EventKind::ToolCall { input, .. } => assert_eq!(input, ToolInput::Unknown),
        _ => panic!(),
    }
}

fn reverse_maps(value: &mut Value) {
    match value {
        Value::Map(entries) => {
            entries.reverse();
            for (k, v) in entries {
                reverse_maps(k);
                reverse_maps(v);
            }
        }
        Value::Array(items) => {
            for item in items {
                reverse_maps(item);
            }
        }
        Value::Tag(_, inner) => reverse_maps(inner),
        _ => {}
    }
}
#[test]
fn canonical_encoding_ignores_map_insertion_order_including_json_payloads() {
    for name in ["context", "tool_json", "message", "session"] {
        let original = fixture(name);
        let mut value = wire(&original);
        reverse_maps(&mut value);
        let changed = bytes(&value);
        let canonical = if name == "session" {
            cbor::decode::<Session>(&changed).unwrap().encode().unwrap()
        } else {
            cbor::decode::<Event>(&changed).unwrap().encode().unwrap()
        };
        assert_eq!(canonical, original, "{name}");
    }
}

#[test]
fn malformed_truncated_duplicate_and_trailing_data_fail() {
    let encoded = fixture("reasoning");
    for end in 0..encoded.len() {
        assert!(
            cbor::decode::<Event>(&encoded[..end]).is_err(),
            "offset {end}"
        );
    }
    let mut trailing = encoded.clone();
    trailing.push(0);
    assert!(cbor::decode::<Event>(&trailing).is_err());
    let mut value = wire(&encoded);
    let duplicate = entries(&mut value)[0].clone();
    entries(&mut value).push(duplicate);
    assert!(cbor::decode::<Event>(&bytes(&value)).is_err());
    let mut value = wire(&encoded);
    *field(&mut value, 1) = Value::Bytes(vec![1; 15]);
    assert!(cbor::decode::<Event>(&bytes(&value)).is_err());
    let mut value = wire(&encoded);
    entries(&mut value).push((Value::Text("future".into()), Value::Null));
    assert!(cbor::decode::<Event>(&bytes(&value)).is_err());
    let mut value = wire(&encoded);
    let repeated = Value::Map(vec![
        (Value::from(1), Value::Null),
        (Value::from(1), Value::Null),
    ]);
    entries(&mut value).push((Value::from(99), repeated));
    assert!(
        cbor::decode::<Event>(&bytes(&value)).is_err(),
        "duplicate key inside an unknown field"
    );
}

#[test]
fn event_identity_obeys_the_exact_native_and_inferred_key_formula() {
    let id = EventId::native(Provider::Claude, "synthetic", "uuid-0");
    // Literal framing assembled independently: domain, then 6/9/6-byte fields.
    let hash = blake3::hash(
        b"bxl.event.v1\x00\x00\x00\x06claude\x00\x00\x00\x09synthetic\x00\x00\x00\x06uuid-0",
    );
    let expected = [
        0x7c, 0x9b, 0x6c, 0x9f, 0x48, 0xea, 0xbd, 0x07, 0x3d, 0x1e, 0x48, 0xcc, 0xd3, 0xf0, 0x20,
        0x57,
    ];
    assert_eq!(&hash.as_bytes()[..16], &expected);
    assert_eq!(id.0, expected);
    assert_eq!(id, EventId::native(Provider::Claude, "synthetic", "uuid-0"));
    assert_ne!(id, EventId::native(Provider::Codex, "synthetic", "uuid-0"));
    assert_ne!(id, EventId::native(Provider::Claude, "another", "uuid-0"));
    assert_ne!(id, EventId::native(Provider::Claude, "synthetic", "uuid-1"));
    assert_eq!(
        EventId::inferred(Provider::Claude, "synthetic", "uuid-0", "#turn-start"),
        EventId::native(Provider::Claude, "synthetic", "uuid-0#turn-start")
    );
    // Native file order and Claude DAG order remain independent.
    let e = Event {
        seq: 3,
        id,
        parent: Some(EventId([9; 16])),
        turn: None,
        at: None,
        fidelity: Fidelity::Partial,
        raw: None,
        kind: EventKind::Context {
            changes: Default::default(),
        },
    };
    assert_eq!(
        cbor::decode::<Event>(&cbor::encode(&e).unwrap())
            .unwrap()
            .value(),
        &e
    );
}

#[test]
fn event_identity_length_framing_separates_adjacent_native_fields() {
    assert_ne!(
        EventId::native(Provider::Claude, "s1", "23"),
        EventId::native(Provider::Claude, "s12", "3")
    );
}

#[test]
fn rfc8949_core_order_is_bytewise_across_key_types() {
    // RFC 8949 §4.2.1 example, deliberately inserted in reverse order.
    let mut value = wire(&fixture("unknown"));
    let keys = [
        Value::from(10),
        Value::from(100),
        Value::from(-1),
        Value::Text("z".into()),
        Value::Text("aa".into()),
        Value::Array(vec![Value::from(100)]),
        Value::Array(vec![Value::from(-1)]),
        Value::Bool(false),
    ];
    entries(&mut value).push((
        Value::from(99),
        Value::Map(
            keys.into_iter()
                .rev()
                .map(|key| (key, Value::Null))
                .collect(),
        ),
    ));
    let forwarded = cbor::decode::<Event>(&bytes(&value))
        .unwrap()
        .encode()
        .unwrap();
    let mut canonical = wire(&forwarded);
    let expected = [
        0xa8, // 8 entries, each with a null value
        0x0a, 0xf6, // 10
        0x18, 0x64, 0xf6, // 100
        0x20, 0xf6, // -1
        0x61, 0x7a, 0xf6, // "z"
        0x62, 0x61, 0x61, 0xf6, // "aa"
        0x81, 0x18, 0x64, 0xf6, // [100]
        0x81, 0x20, 0xf6, // [-1]
        0xf4, 0xf6, // false
    ];
    assert_eq!(bytes(field(&mut canonical, 99)), expected);
}
