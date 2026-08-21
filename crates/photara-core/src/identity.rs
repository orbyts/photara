use std::{fmt, str::FromStr};

use semver::Version;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum CanonicalIdError {
    #[error("identifier must not be empty")]
    Empty,
    #[error(
        "identifier {value:?} must be lowercase ASCII and may contain digits, '.', '-', or '_'"
    )]
    InvalidCharacters { value: String },
    #[error("identifier {value:?} must not start or end with punctuation")]
    EdgePunctuation { value: String },
    #[error("namespaced identifier {value:?} must contain at least one '.' separator")]
    MissingNamespace { value: String },
    #[error("identifier {value:?} contains an empty namespace segment")]
    EmptySegment { value: String },
}

fn validate(value: &str, namespaced: bool) -> Result<(), CanonicalIdError> {
    if value.is_empty() {
        return Err(CanonicalIdError::Empty);
    }
    if !value.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
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
    if value.split('.').any(str::is_empty) {
        return Err(CanonicalIdError::EmptySegment {
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
            /// Returns [`CanonicalIdError`] when the value is not canonical.
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
string_id!(NodeDefinitionId, true);
string_id!(ValueTypeId, true);
string_id!(SchemaId, true);
string_id!(CapabilityId, true);
string_id!(PortId, false);

/// The independently released semantic version of a node package.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct PackageVersion(Version);

impl PackageVersion {
    #[must_use]
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self(Version::new(major, minor, patch))
    }

    #[must_use]
    pub const fn as_semver(&self) -> &Version {
        &self.0
    }
}

impl fmt::Display for PackageVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for PackageVersion {
    type Err = semver::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse().map(Self)
    }
}

macro_rules! nonzero_version {
    ($name:ident, $description:literal) => {
        #[derive(
            Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
        )]
        #[serde(transparent)]
        pub struct $name(u32);

        impl $name {
            #[must_use]
            pub const fn first() -> Self {
                Self(1)
            }

            /// Creates a nonzero version.
            ///
            /// # Errors
            ///
            /// Returns [`VersionError`] when `value` is zero.
            pub const fn new(value: u32) -> Result<Self, VersionError> {
                if value == 0 {
                    return Err(VersionError::Zero { kind: $description });
                }
                Ok(Self(value))
            }

            #[must_use]
            pub const fn get(self) -> u32 {
                self.0
            }
        }
    };
}

nonzero_version!(NodeDefinitionVersion, "node definition");
nonzero_version!(ValueTypeVersion, "value type");
nonzero_version!(SchemaVersion, "schema");

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum VersionError {
    #[error("{kind} version must be greater than zero")]
    Zero { kind: &'static str },
}

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
uuid_id!(ConnectionId);
uuid_id!(CommandId);
uuid_id!(RequestId);
uuid_id!(EvaluationId);
uuid_id!(ProjectId);
uuid_id!(ProjectResourceId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespaced_ids_are_canonical() {
        assert!(NodePackageId::parse("photara.layout").is_ok());
        assert!(NodePackageId::parse("layout").is_err());
        assert!(NodePackageId::parse("Photara.Layout").is_err());
        assert!(NodePackageId::parse("photara..layout").is_err());
        assert!(NodePackageId::parse("photara.layout/v1").is_err());
    }

    #[test]
    fn versioned_identity_parts_round_trip_independently() {
        let package: PackageVersion = "2.4.0-beta.2+mac".parse().unwrap();
        let encoded = serde_json::to_string(&package).unwrap();
        let decoded: PackageVersion = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, package);
        assert_eq!(decoded.to_string(), "2.4.0-beta.2+mac");

        let definition = NodeDefinitionVersion::new(3).unwrap();
        let encoded = serde_json::to_string(&definition).unwrap();
        assert_eq!(
            serde_json::from_str::<NodeDefinitionVersion>(&encoded).unwrap(),
            definition
        );
    }
}
