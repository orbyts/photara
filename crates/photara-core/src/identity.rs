use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum CanonicalIdError {
    #[error("identifier must not be empty")]
    Empty,
    #[error(
        "identifier {value:?} must be lowercase ASCII and may contain digits, '.', '-', '_', or '/'"
    )]
    InvalidCharacters { value: String },
    #[error("identifier {value:?} must not start or end with punctuation")]
    EdgePunctuation { value: String },
    #[error("namespaced identifier {value:?} must contain at least one '.' separator")]
    MissingNamespace { value: String },
}

fn validate(value: &str, namespaced: bool) -> Result<(), CanonicalIdError> {
    if value.is_empty() {
        return Err(CanonicalIdError::Empty);
    }
    if !value.bytes().all(|byte| {
        byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(byte, b'.' | b'-' | b'_' | b'/')
    }) {
        return Err(CanonicalIdError::InvalidCharacters {
            value: value.to_owned(),
        });
    }
    if !value
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphanumeric)
        || !value
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
    {
        return Err(CanonicalIdError::EdgePunctuation {
            value: value.to_owned(),
        });
    }
    if namespaced && !value.contains('.') {
        return Err(CanonicalIdError::MissingNamespace {
            value: value.to_owned(),
        });
    }
    Ok(())
}

macro_rules! string_id {
    ($name:ident, $namespaced:literal) => {
        #[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Parses and validates a canonical identifier.
            ///
            /// # Errors
            ///
            /// Returns [`CanonicalIdError`] when the value is empty, contains
            /// unsupported characters, has punctuation at an edge, or lacks
            /// the namespace required by this identifier type.
            pub fn parse(value: impl Into<String>) -> Result<Self, CanonicalIdError> {
                let value = value.into();
                validate(&value, $namespaced)?;
                Ok(Self(value))
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }

        impl FromStr for $name {
            type Err = CanonicalIdError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse(value)
            }
        }
    };
}

string_id!(NodePackageId, true);
string_id!(NodeTypeId, true);
string_id!(ValueTypeId, true);
string_id!(SchemaId, true);
string_id!(CapabilityId, true);
string_id!(PortId, false);

macro_rules! uuid_id {
    ($name:ident) => {
        #[derive(
            Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
        )]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            #[must_use]
            pub const fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }

            #[must_use]
            pub const fn as_uuid(self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

uuid_id!(GraphId);
uuid_id!(NodeInstanceId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespaced_ids_are_canonical() {
        assert!(NodePackageId::parse("photara.layout").is_ok());
        assert!(NodePackageId::parse("layout").is_err());
        assert!(NodePackageId::parse("Photara.Layout").is_err());
        assert!(NodePackageId::parse("photara.layout.").is_err());
    }
}
