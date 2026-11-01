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
    // Literal frame lengths: domain 12, provider 6, session 9, key 6, role 0.
    let hash = blake3::hash(b"\x00\x00\x00\x0cbxl.event.v1\x00\x00\x00\x06claude\x00\x00\x00\x09synthetic\x00\x00\x00\x06uuid-0\x00\x00\x00\x00");
    let expected = [
        0xc9, 0xa0, 0xc0, 0x26, 0xc0, 0x23, 0xe3, 0x3f, 0x26, 0x8e, 0x8f, 0x01, 0x5f, 0xe9, 0x8b,
        0xf0,
    ];
    assert_eq!(&hash.as_bytes()[..16], &expected);
    assert_eq!(id.0, expected);
    assert_eq!(id, EventId::native(Provider::Claude, "synthetic", "uuid-0"));
    assert_ne!(id, EventId::native(Provider::Codex, "synthetic", "uuid-0"));
    assert_ne!(id, EventId::native(Provider::Claude, "another", "uuid-0"));
    assert_ne!(id, EventId::native(Provider::Claude, "synthetic", "uuid-1"));
    let start = EventId::inferred(Provider::Claude, "synthetic", "uuid-0", "turn-start");
    let end = EventId::inferred(Provider::Claude, "synthetic", "uuid-0", "turn-end");
    assert_eq!(
        start.0,
        [
            0x2d, 0xe1, 0x68, 0x09, 0x8b, 0x11, 0x52, 0x95, 0x81, 0x11, 0x55, 0x9c, 0xa5, 0xa8,
            0x6d, 0xd8
        ]
    );
    assert_eq!(
        end.0,
        [
            0xcb, 0xea, 0x0f, 0xfa, 0xee, 0xa0, 0xe6, 0x90, 0x76, 0x7d, 0x9b, 0x93, 0x1e, 0xe0,
            0x3c, 0x4d
        ]
    );
    assert_ne!(start, id);
    assert_ne!(end, start);
    assert_ne!(
        start,
        EventId::native(Provider::Claude, "synthetic", "uuid-0turn-start")
    );
    assert_ne!(
        start,
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
    let a = EventId::native(Provider::Claude, "s1", "23");
    let b = EventId::native(Provider::Claude, "s12", "3");
    assert_eq!(
        a.0,
        [
            0xe3, 0x4f, 0x33, 0x49, 0x41, 0x4f, 0x16, 0x2f, 0x5e, 0x50, 0x65, 0x5a, 0xd6, 0x45,
            0xdb, 0x33
        ]
    );
    assert_eq!(
        b.0,
        [
            0x4b, 0x79, 0x81, 0xa5, 0x04, 0x0f, 0x1f, 0x50, 0x4c, 0x42, 0xb1, 0x2d, 0x13, 0x3e,
            0x91, 0x89
        ]
    );
    assert_ne!(a, b);
    // Deliberately broken identity function: omit every length frame.
    let unframed = |session: &str, key: &str| {
        let joined = ["bxl.event.v1", "claude", session, key, ""].concat();
        blake3::hash(joined.as_bytes()).as_bytes()[..16].to_vec()
    };
    assert_eq!(unframed("s1", "23"), unframed("s12", "3"));
    assert_ne!(a.0.as_slice(), unframed("s1", "23"));
}

const MIXED_KEYS_GOLDEN: &[u8] = &[
    0xa6, // six entries, null values
    0x0a, 0xf6, // 10
    0x17, 0xf6, // 23
    0x18, 0x18, 0xf6, // 24
    0x18, 0x64, 0xf6, // 100
    0x20, 0xf6, // -1
    0x61, 0x61, 0xf6, // "a"
];
fn mixed_keys() -> Value {
    Value::Map(
        [
            Value::Text("a".into()),
            Value::from(-1),
            Value::from(100),
            Value::from(24),
            Value::from(23),
            Value::from(10),
        ]
        .into_iter()
        .map(|key| (key, Value::Null))
        .collect(),
    )
}
#[test]
fn rfc8949_core_order_is_bytewise_across_key_types() {
    let mut value = wire(&fixture("unknown"));
    entries(&mut value).push((Value::from(99), mixed_keys()));
    let forwarded = cbor::decode::<Event>(&bytes(&value))
        .unwrap()
        .encode()
        .unwrap();
    assert_eq!(bytes(field(&mut wire(&forwarded), 99)), MIXED_KEYS_GOLDEN);
}
#[test]
fn length_first_control_fails_the_mixed_key_golden() {
    use ciborium::value::CanonicalValue;
    let mut broken = mixed_keys();
    entries(&mut broken).sort_by_cached_key(|(key, _)| CanonicalValue::from(key.clone()));
    let broken_bytes = bytes(&broken);
    assert_eq!(
        broken_bytes,
        [
            0xa6, 0x0a, 0xf6, 0x17, 0xf6, 0x20, 0xf6, // 10,23,-1
            0x18, 0x18, 0xf6, 0x18, 0x64, 0xf6, 0x61, 0x61, 0xf6, // 24,100,"a"
        ]
    );
    assert_ne!(
        broken_bytes, MIXED_KEYS_GOLDEN,
        "length-first writer must fail the core-order golden"
    );
}

fn noncanonical(value: &Value, out: &mut Vec<u8>) {
    match value {
        Value::Map(fields) => {
            out.push(0xbf); // indefinite map, reversed field order
            for (key, value) in fields.iter().rev() {
                noncanonical(key, out);
                noncanonical(value, out);
            }
            out.push(0xff);
        }
        Value::Array(items) => {
            out.push(0x9f);
            for item in items {
                noncanonical(item, out);
            }
            out.push(0xff);
        }
        Value::Text(text) => {
            out.push(0x7f);
            out.extend(bytes(&Value::Text(text.clone())));
            out.push(0xff);
        }
        Value::Bytes(data) => {
            out.push(0x5f);
            out.extend(bytes(&Value::Bytes(data.clone())));
            out.push(0xff);
        }
        Value::Integer(integer) => {
            let number = i128::from(*integer);
            let argument = if number >= 0 {
                out.push(0x1b);
                number
            } else {
                out.push(0x3b);
                -1 - number
            };
            out.extend(u64::try_from(argument).unwrap().to_be_bytes());
        }
        Value::Float(number) => {
            out.push(0xfb);
            out.extend(number.to_bits().to_be_bytes());
        }
        Value::Tag(tag, inner) => {
            out.push(0xdb);
            out.extend(tag.to_be_bytes());
            noncanonical(inner, out);
        }
        _ => out.extend(bytes(value)),
    }
}
#[test]
fn reader_accepts_valid_noncanonical_lengths_floats_containers_and_key_order() {
    for name in ["tool_json", "message", "session", "future_mixed_keys"] {
        let canonical = fixture(name);
        let mut incoming = Vec::new();
        noncanonical(&wire(&canonical), &mut incoming);
        assert_ne!(incoming, canonical);
        let emitted = if name == "session" {
            cbor::decode::<Session>(&incoming)
                .unwrap()
                .encode()
                .unwrap()
        } else {
            cbor::decode::<Event>(&incoming).unwrap().encode().unwrap()
        };
        assert_eq!(
            emitted, canonical,
            "reader rejected or changed valid CBOR: {name}"
        );
    }
}
