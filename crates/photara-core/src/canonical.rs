use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest as _, Sha256};

/// A SHA-256 digest of the canonical JSON representation of semantic data.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct CanonicalDigest([u8; 32]);

impl CanonicalDigest {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for CanonicalDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Serializes semantic data as compact JSON with recursively sorted object keys.
///
/// # Errors
///
/// Returns a serialization error when `value` cannot be represented as JSON.
pub fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    let mut value = serde_json::to_value(value)?;
    canonicalize_value(&mut value);
    serde_json::to_vec(&value)
}

/// Digests the canonical JSON representation of semantic data with SHA-256.
///
/// # Errors
///
/// Returns a serialization error when `value` cannot be represented as JSON.
pub fn canonical_digest<T: Serialize>(value: &T) -> Result<CanonicalDigest, serde_json::Error> {
    let bytes = canonical_json(value)?;
    Ok(CanonicalDigest::from_bytes(Sha256::digest(bytes).into()))
}

fn canonicalize_value(value: &mut Value) {
    match value {
        Value::Array(values) => {
            for value in values {
                canonicalize_value(value);
            }
        }
        Value::Object(object) => {
            let old = std::mem::take(object);
            let mut entries: Vec<_> = old.into_iter().collect();
            entries.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));
            let mut canonical = Map::new();
            for (key, mut value) in entries {
                canonicalize_value(&mut value);
                canonical.insert(key, value);
            }
            *object = canonical;
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn object_order_does_not_change_canonical_bytes_or_digest() {
        let left = json!({"z": 2, "nested": {"b": true, "a": false}});
        let right: Value =
            serde_json::from_str(r#"{"nested":{"a":false,"b":true},"z":2}"#).unwrap();

        let canonical = br#"{"nested":{"a":false,"b":true},"z":2}"#;
        assert_eq!(canonical_json(&left).unwrap(), canonical);
        assert_eq!(canonical_json(&right).unwrap(), canonical);
        assert_eq!(
            canonical_digest(&left).unwrap(),
            canonical_digest(&right).unwrap()
        );
        assert_eq!(
            canonical_digest(&left).unwrap().to_string(),
            "eb8ed80b5dbca514493706fc7cb5c57061489812f7df7743c0f670ff4ded439d"
        );
    }
}
