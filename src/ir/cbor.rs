// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

//! Deterministic, integer-key CBOR. The decoded document retains the complete
//! wire value, including unknown fields/variants at any depth. Read through
//! `Decoded::value` and forward with `Decoded::encode` to preserve extensions.
//! `into_value` explicitly leaves that lossless envelope behind.
use super::{
    Event, Session,
    keys::{self, Field, Schema},
};
use ciborium::value::{CanonicalValue, Value};
use serde::{Serialize, de::DeserializeOwned};
use std::{fmt, io::Cursor};

#[derive(Debug)]
pub struct Error(String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
fn fail(message: impl Into<String>) -> Error {
    Error(message.into())
}

/// Types with a fixed v0.1 integer-key schema.
pub trait CborRecord: Serialize + DeserializeOwned {
    const SCHEMA: Schema;
}
impl CborRecord for Event {
    const SCHEMA: Schema = keys::EVENT;
}
impl CborRecord for Session {
    const SCHEMA: Schema = keys::SESSION;
}

#[derive(Clone, Debug)]
pub struct Decoded<T> {
    value: T,
    wire: Value,
}
impl<T> Decoded<T> {
    pub fn value(&self) -> &T {
        &self.value
    }
    /// Extract only the known IR projection; future CBOR fields are discarded.
    pub fn into_value(self) -> T {
        self.value
    }
    /// Canonical forwarding preserves all unknown fields and discriminants.
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        canonical_bytes(self.wire.clone())
    }
}

pub fn encode<T: CborRecord>(value: &T) -> Result<Vec<u8>, Error> {
    let named = Value::serialized(value).map_err(|e| fail(e.to_string()))?;
    canonical_bytes(transcode(named, T::SCHEMA, false)?)
}

pub fn decode<T: CborRecord>(bytes: &[u8]) -> Result<Decoded<T>, Error> {
    let mut reader = Cursor::new(bytes);
    let mut wire: Value =
        ciborium::de::from_reader(&mut reader).map_err(|e| fail(e.to_string()))?;
    if reader.position() != bytes.len() as u64 {
        return Err(fail("trailing CBOR bytes"));
    }
    canonicalize(&mut wire)?; // Duplicate keys are ambiguous, even in unknown subtrees.
    let named = transcode(wire.clone(), T::SCHEMA, true)?;
    let value = named.deserialized().map_err(|e| fail(e.to_string()))?;
    Ok(Decoded { value, wire })
}

fn uint(value: &Value) -> Result<u64, Error> {
    match value {
        Value::Integer(i) => u64::try_from(*i).map_err(|_| fail("expected unsigned key/tag")),
        _ => Err(fail("expected integer key/tag")),
    }
}
fn map(value: Value) -> Result<Vec<(Value, Value)>, Error> {
    match value {
        Value::Map(v) => Ok(v),
        _ => Err(fail("expected map")),
    }
}
fn array(value: Value) -> Result<Vec<Value>, Error> {
    match value {
        Value::Array(v) => Ok(v),
        _ => Err(fail("expected array")),
    }
}
fn record(entries: Vec<(Value, Value)>, fields: &[Field], decoding: bool) -> Result<Value, Error> {
    let mut out = Vec::new();
    for (key, value) in entries {
        let field = if decoding {
            let key = uint(&key)?;
            fields.iter().find(|f| f.key == key)
        } else {
            fields.iter().find(|f| key.as_text() == Some(f.name))
        };
        if let Some(f) = field {
            out.push((
                if decoding {
                    Value::Text(f.name.into())
                } else {
                    Value::from(f.key)
                },
                transcode(value, f.schema, decoding)?,
            ));
        } else if !decoding {
            return Err(fail("serde field missing from integer-key table"));
        }
    }
    Ok(Value::Map(out))
}

fn transcode(value: Value, schema: Schema, decoding: bool) -> Result<Value, Error> {
    if value == Value::Null {
        return Ok(value);
    }
    match schema {
        Schema::Scalar => Ok(value),
        Schema::Bytes => {
            if decoding {
                match value {
                    Value::Bytes(bytes) => {
                        Ok(Value::Array(bytes.into_iter().map(Value::from).collect()))
                    }
                    _ => Err(fail("expected byte string")),
                }
            } else {
                Ok(Value::Bytes(
                    array(value)?
                        .iter()
                        .map(|v| u8::try_from(uint(v)?).map_err(|_| fail("invalid byte")))
                        .collect::<Result<_, _>>()?,
                ))
            }
        }
        Schema::List(element) => Ok(Value::Array(
            array(value)?
                .into_iter()
                .map(|v| transcode(v, *element, decoding))
                .collect::<Result<_, _>>()?,
        )),
        Schema::Tuple(elements) => {
            let values = array(value)?;
            if values.len() != elements.len() {
                return Err(fail("wrong tuple length"));
            }
            Ok(Value::Array(
                values
                    .into_iter()
                    .zip(elements)
                    .map(|(v, s)| transcode(v, *s, decoding))
                    .collect::<Result<_, _>>()?,
            ))
        }
        Schema::Map(element) => Ok(Value::Map(
            map(value)?
                .into_iter()
                .map(|(k, v)| {
                    if !matches!(k, Value::Text(_)) {
                        return Err(fail("expected string payload key"));
                    }
                    Ok((k, transcode(v, *element, decoding)?))
                })
                .collect::<Result<_, _>>()?,
        )),
        Schema::Record(fields) => record(map(value)?, fields, decoding),
        Schema::Unit(names) => {
            if decoding {
                Ok(Value::Text(
                    names
                        .get(usize::try_from(uint(&value)?).unwrap_or(usize::MAX))
                        .unwrap_or(&"unknown")
                        .to_string(),
                ))
            } else {
                let name = value.as_text().ok_or_else(|| fail("expected enum name"))?;
                Ok(Value::from(
                    names
                        .iter()
                        .position(|n| *n == name)
                        .map_or(255, |n| n as u64),
                ))
            }
        }
        Schema::Tagged(variants) => {
            let mut entries = map(value)?;
            let index = entries
                .iter()
                .position(|(k, _)| {
                    if decoding {
                        *k == Value::from(0)
                    } else {
                        k.as_text() == Some("kind")
                    }
                })
                .ok_or_else(|| fail("missing enum tag"))?;
            let (_, tag) = entries.remove(index);
            let variant = if decoding {
                let tag = uint(&tag)?;
                // Even an unknown union uses unsigned integer field keys.
                for (key, _) in &entries {
                    uint(key)?;
                }
                variants.iter().find(|v| v.tag == tag)
            } else {
                variants.iter().find(|v| tag.as_text() == Some(v.name))
            };
            let mut out = if let Some(v) = variant {
                map(record(entries, v.fields, decoding)?)?
            } else {
                Vec::new()
            };
            out.push(if decoding {
                (
                    Value::Text("kind".into()),
                    Value::Text(variant.map_or("unknown", |v| v.name).into()),
                )
            } else {
                (Value::from(0), Value::from(variant.map_or(255, |v| v.tag)))
            });
            Ok(Value::Map(out))
        }
    }
}

fn canonical_bytes(mut value: Value) -> Result<Vec<u8>, Error> {
    canonicalize(&mut value)?;
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(&value, &mut bytes).map_err(|e| fail(e.to_string()))?;
    Ok(bytes)
}
fn canonicalize(value: &mut Value) -> Result<(), Error> {
    match value {
        Value::Map(entries) => {
            for (k, v) in entries.iter_mut() {
                canonicalize(k)?;
                canonicalize(v)?;
            }
            entries.sort_by_cached_key(|(k, _)| CanonicalValue::from(k.clone()));
            if entries.windows(2).any(|w| {
                CanonicalValue::from(w[0].0.clone()) == CanonicalValue::from(w[1].0.clone())
            }) {
                return Err(fail("duplicate CBOR map key"));
            }
        }
        Value::Array(values) => {
            for value in values {
                canonicalize(value)?;
            }
        }
        Value::Tag(_, inner) => canonicalize(inner)?,
        _ => {}
    }
    Ok(())
}
