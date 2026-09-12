//! Strict, bounded input parsing before constructing `serde_json::Value`.

use std::{collections::BTreeSet, fmt};

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

use super::PackageError;

/// Resource ceilings applied before and during JSON decoding.
#[derive(Clone, Copy, Debug)]
pub struct JsonLimits {
    pub max_bytes: usize,
    pub max_depth: usize,
    pub max_members: usize,
    pub max_array_elements: usize,
}

impl Default for JsonLimits {
    fn default() -> Self {
        Self {
            max_bytes: 16 * 1024 * 1024,
            max_depth: 64,
            max_members: 4096,
            max_array_elements: 100_000,
        }
    }
}

/// Parses bounded UTF-8 JSON, rejecting duplicates before constructing a Value.
///
/// Integer lexemes outside i64-negative/u64-positive range are rejected before
/// `serde_json` could coerce them to a lossy floating-point value. Floats retain
/// the existing Core codec's finite `serde_json` semantics, including negative zero.
///
/// # Errors
/// Returns a redacted JSON/limit error for malformed or unsupported input.
pub fn parse_json(bytes: &[u8], limits: JsonLimits) -> Result<Value, PackageError> {
    if bytes.len() > limits.max_bytes {
        return Err(PackageError::Limit);
    }
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) || std::str::from_utf8(bytes).is_err() {
        return Err(PackageError::Json);
    }
    validate_integer_lexemes(bytes)?;
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let value = Seed { limits, depth: 0 }
        .deserialize(&mut decoder)
        .map_err(|_| PackageError::Json)?;
    decoder.end().map_err(|_| PackageError::Json)?;
    Ok(value)
}

/// Parses a strict canonical body without accepting whitespace/reformatting.
///
/// # Errors
/// Returns an input error or `NonCanonical` when its bytes differ from Core.
pub fn parse_canonical_json(bytes: &[u8], limits: JsonLimits) -> Result<Value, PackageError> {
    let value = parse_json(bytes, limits)?;
    if photara_core::canonical_json(&value).map_err(|_| PackageError::Json)? != bytes {
        return Err(PackageError::NonCanonical);
    }
    Ok(value)
}

fn validate_integer_lexemes(bytes: &[u8]) -> Result<(), PackageError> {
    let mut position = 0;
    while position < bytes.len() {
        if bytes[position] == b'"' {
            position += 1;
            while position < bytes.len() {
                match bytes[position] {
                    b'\\' => position += 2,
                    b'"' => {
                        position += 1;
                        break;
                    }
                    _ => position += 1,
                }
            }
        } else if bytes[position] == b'-' || bytes[position].is_ascii_digit() {
            let start = position;
            while position < bytes.len()
                && matches!(
                    bytes[position],
                    b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E'
                )
            {
                position += 1;
            }
            let token = &bytes[start..position];
            if !token.iter().any(|b| matches!(b, b'.' | b'e' | b'E')) {
                let token = std::str::from_utf8(token).map_err(|_| PackageError::Json)?;
                let valid = if token.starts_with('-') {
                    token.parse::<i64>().is_ok()
                } else {
                    token.parse::<u64>().is_ok()
                };
                if !valid {
                    return Err(PackageError::Json);
                }
            }
        } else {
            position += 1;
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Seed {
    limits: JsonLimits,
    depth: usize,
}

impl<'de> DeserializeSeed<'de> for Seed {
    type Value = Value;
    fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        if self.depth > self.limits.max_depth {
            return Err(de::Error::custom("json depth limit"));
        }
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Seed {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("bounded JSON")
    }
    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Value, E> {
        Ok(Value::Bool(v))
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Value, E> {
        Ok(Value::Number(v.into()))
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Value, E> {
        Ok(Value::Number(v.into()))
    }
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Value, E> {
        Number::from_f64(v)
            .map(Value::Number)
            .ok_or_else(|| E::custom("nonfinite number"))
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Value, E> {
        Ok(Value::String(v.to_owned()))
    }
    fn visit_string<E: de::Error>(self, v: String) -> Result<Value, E> {
        Ok(Value::String(v))
    }
    fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_none<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Value, A::Error> {
        let mut values = Vec::new();
        let next = Self {
            depth: self.depth + 1,
            ..self
        };
        while let Some(value) = access.next_element_seed(next)? {
            if values.len() >= self.limits.max_array_elements {
                return Err(de::Error::custom("array limit"));
            }
            values.push(value);
        }
        Ok(Value::Array(values))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Value, A::Error> {
        let mut values = Map::new();
        let mut keys = BTreeSet::new();
        let next = Self {
            depth: self.depth + 1,
            ..self
        };
        while let Some(key) = access.next_key::<String>()? {
            if values.len() >= self.limits.max_members || !keys.insert(key.clone()) {
                return Err(de::Error::custom("duplicate key or object limit"));
            }
            let value = access.next_value_seed(next)?;
            values.insert(key, value);
        }
        Ok(Value::Object(values))
    }
}
