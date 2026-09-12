//! Closed, validated portable scalar coordinates. Errors never echo input data.
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{collections::BTreeSet, fmt, str::FromStr};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, ContractError>;
#[derive(Clone, Copy, Debug, Eq, PartialEq, Error, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContractError {
    #[error("invalid-identity")]
    Identity,
    #[error("invalid-coordinate")]
    Coordinate,
    #[error("invalid-version")]
    Version,
    #[error("limit-exceeded")]
    Limit,
    #[error("duplicate-coordinate")]
    Duplicate,
    #[error("invalid-order")]
    Order,
    #[error("scope-mismatch")]
    Scope,
    #[error("invalid-union")]
    Union,
    #[error("invalid-action-mask")]
    Mask,
    #[error("unsupported-contract")]
    Unsupported,
    #[error("privacy-boundary")]
    Privacy,
    #[error("digest-mismatch")]
    Digest,
    #[error("required-field-missing")]
    Missing,
    #[error("noncanonical-input")]
    Canonical,
    #[error("serialization-failed")]
    Serialization,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct QualifiedName(String);
impl QualifiedName {
    /// Parses an exact bounded namespaced ASCII machine coordinate.
    /// # Errors
    /// Rejects wildcards, empty segments, noncanonical spelling and overlong IDs.
    pub fn parse(s: impl Into<String>) -> Result<Self> {
        Self::try_from(s.into())
    }
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for QualifiedName {
    type Error = ContractError;
    fn try_from(s: String) -> Result<Self> {
        if s.len() > 256
            || !s.contains('.')
            || s.split('.').any(|part| {
                part.is_empty()
                    || !part.as_bytes()[0].is_ascii_lowercase()
                    || !part.as_bytes()[part.len() - 1].is_ascii_alphanumeric()
                    || !part.bytes().all(|c| {
                        c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'_'
                    })
            })
        {
            return Err(ContractError::Coordinate);
        }
        Ok(Self(s))
    }
}
impl From<QualifiedName> for String {
    fn from(v: QualifiedName) -> Self {
        v.0
    }
}
impl fmt::Display for QualifiedName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct LocalName(String);
impl LocalName {
    /// Parses a lowercase `snake_case` slot, field or port identifier.
    /// # Errors
    /// Rejects empty, noncanonical or overlong names.
    pub fn parse(s: impl Into<String>) -> Result<Self> {
        Self::try_from(s.into())
    }
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for LocalName {
    type Error = ContractError;
    fn try_from(s: String) -> Result<Self> {
        if s.len() > 64
            || !s.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
            || !s
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
        {
            return Err(ContractError::Coordinate);
        }
        Ok(Self(s))
    }
}
impl From<LocalName> for String {
    fn from(v: LocalName) -> Self {
        v.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct Version(u32);
impl Version {
    pub const FIRST: Self = Self(1);
    /// # Errors
    /// Zero versions are invalid.
    pub const fn new(n: u32) -> Result<Self> {
        if n == 0 {
            Err(ContractError::Version)
        } else {
            Ok(Self(n))
        }
    }
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}
impl TryFrom<u32> for Version {
    type Error = ContractError;
    fn try_from(n: u32) -> Result<Self> {
        Self::new(n)
    }
}
impl From<Version> for u32 {
    fn from(v: Version) -> Self {
        v.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "i64", into = "i64")]
pub struct LocalRevision(i64);
impl LocalRevision {
    pub const INITIAL: Self = Self(1);
    /// # Errors
    /// Rejects zero and negative revisions.
    pub const fn new(n: i64) -> Result<Self> {
        if n > 0 {
            Ok(Self(n))
        } else {
            Err(ContractError::Version)
        }
    }
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
    /// # Errors
    /// Fails at the i64 boundary without wrapping.
    pub fn next(self) -> Result<Self> {
        Self::new(self.0.checked_add(1).ok_or(ContractError::Limit)?)
    }
}
impl TryFrom<i64> for LocalRevision {
    type Error = ContractError;
    fn try_from(n: i64) -> Result<Self> {
        Self::new(n)
    }
}
impl From<LocalRevision> for i64 {
    fn from(v: LocalRevision) -> Self {
        v.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DecimalU64(u64);
impl DecimalU64 {
    #[must_use]
    pub const fn new(n: u64) -> Self {
        Self(n)
    }
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}
impl TryFrom<String> for DecimalU64 {
    type Error = ContractError;
    fn try_from(s: String) -> Result<Self> {
        let n = s.parse::<u64>().map_err(|_| ContractError::Coordinate)?;
        if n.to_string() != s {
            return Err(ContractError::Canonical);
        }
        Ok(Self(n))
    }
}
impl From<DecimalU64> for String {
    fn from(v: DecimalU64) -> Self {
        v.0.to_string()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Digest([u8; 32]);
impl Digest {
    #[must_use]
    pub const fn from_bytes(b: [u8; 32]) -> Self {
        Self(b)
    }
    #[must_use]
    pub const fn bytes(&self) -> &[u8; 32] {
        &self.0
    }
    #[must_use]
    pub fn of_bytes(b: &[u8]) -> Self {
        Self(Sha256::digest(b).into())
    }
    /// Uses the unchanged Core canonical-json.v1 codec.
    /// # Errors
    /// Rejects values the codec cannot encode.
    pub fn canonical<T: Serialize>(v: &T) -> Result<Self> {
        Ok(Self::of_bytes(
            &crate::canonical_json(v).map_err(|_| ContractError::Serialization)?,
        ))
    }
}
impl FromStr for Digest {
    type Err = ContractError;
    fn from_str(s: &str) -> Result<Self> {
        if s.len() != 64
            || !s
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(ContractError::Digest);
        }
        let mut b = [0; 32];
        for (i, byte) in b.iter_mut().enumerate() {
            *byte =
                u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).map_err(|_| ContractError::Digest)?;
        }
        Ok(Self(b))
    }
}
impl TryFrom<String> for Digest {
    type Error = ContractError;
    fn try_from(s: String) -> Result<Self> {
        s.parse()
    }
}
impl From<Digest> for String {
    fn from(d: Digest) -> Self {
        d.to_string()
    }
}
impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaRef {
    pub id: QualifiedName,
    pub version: Version,
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValueTypeRef {
    pub id: QualifiedName,
    pub version: Version,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ObjectKind {
    Json,
    Blob,
}
/// Pure S2 reference shape; does not read or verify a package object.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectRef {
    pub kind: ObjectKind,
    pub sha256: Digest,
    pub byte_length: DecimalU64,
}

/// Strict order rather than sorting during validation preserves semantic bytes.
/// # Errors
/// Rejects duplicate or out-of-order coordinates.
pub fn ordered_unique<T: Ord>(items: &[T]) -> Result<()> {
    if items.windows(2).any(|p| p[0] >= p[1]) {
        Err(ContractError::Order)
    } else {
        Ok(())
    }
}
/// # Errors
/// Rejects duplicate items without changing their order.
pub fn unique<T: Ord>(items: &[T]) -> Result<()> {
    if items.iter().collect::<BTreeSet<_>>().len() == items.len() {
        Ok(())
    } else {
        Err(ContractError::Duplicate)
    }
}
/// # Errors
/// Rejects empty, control-bearing or excessive display text without echoing it.
pub fn display_text(s: &str, max: usize) -> Result<()> {
    if s.trim().is_empty() || s.len() > max || s.chars().any(char::is_control) {
        Err(ContractError::Coordinate)
    } else {
        Ok(())
    }
}

/// Registry-backed semantic families; schema shape alone never authorizes dataflow.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ValueFamily {
    Configuration,
    LibraryReference,
    ResourceDescriptor,
    AssetSet,
    MetadataSet,
    MetadataPatch,
    ArtifactSet,
    GroupSet,
    EffectReceipt,
    LiveHandle,
    SecretRef,
    WorkflowOutputReference,
}
impl ValueFamily {
    #[must_use]
    pub const fn variable_allowed(self) -> bool {
        matches!(
            self,
            Self::Configuration | Self::LibraryReference | Self::ResourceDescriptor
        )
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "shape", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ValueShape {
    Leaf {
        value_type: ValueTypeRef,
        family: ValueFamily,
    },
    List {
        item: Box<ValueShape>,
        max_items: u32,
    },
    Record {
        fields: std::collections::BTreeMap<LocalName, ValueShape>,
    },
}
impl ValueShape {
    /// Pure admission check only; this does not implement D18 variables or values.
    /// # Errors
    /// Rejects nested workflow/live/secret families, false reserved-family claims
    /// and excessive schema depth/node/container bounds.
    pub fn validate_variable_shape(&self) -> Result<()> {
        fn walk(s: &ValueShape, depth: usize, nodes: &mut usize) -> Result<()> {
            *nodes += 1;
            if depth > 32 || *nodes > 1024 {
                return Err(ContractError::Limit);
            }
            match s {
                ValueShape::Leaf { value_type, family } => {
                    validate_family(value_type, *family)?;
                    if !family.variable_allowed() {
                        return Err(ContractError::Privacy);
                    }
                }
                ValueShape::List { item, max_items } => {
                    if *max_items > 10_000 {
                        return Err(ContractError::Limit);
                    }
                    walk(item, depth + 1, nodes)?;
                }
                ValueShape::Record { fields } => {
                    for child in fields.values() {
                        walk(child, depth + 1, nodes)?;
                    }
                }
            }
            Ok(())
        }
        walk(self, 0, &mut 0)
    }
}
/// Checks reserved built-in families; custom claims additionally need a registry.
/// # Errors
/// Rejects false family declarations for built-in workflow/resource identities.
pub fn validate_family(value_type: &ValueTypeRef, family: ValueFamily) -> Result<()> {
    let reserved = match value_type.id.as_str() {
        "photara.asset-set" => Some(ValueFamily::AssetSet),
        "photara.metadata-set" => Some(ValueFamily::MetadataSet),
        "photara.metadata-patch" => Some(ValueFamily::MetadataPatch),
        "photara.artifact-set" => Some(ValueFamily::ArtifactSet),
        "photara.group-set" => Some(ValueFamily::GroupSet),
        "photara.effect-receipt" => Some(ValueFamily::EffectReceipt),
        "photara.resource-descriptor" => Some(ValueFamily::ResourceDescriptor),
        "photara.secret-ref" => Some(ValueFamily::SecretRef),
        "photara.materialized-handle" => Some(ValueFamily::LiveHandle),
        "photara.workflow-output-reference" => Some(ValueFamily::WorkflowOutputReference),
        _ => None,
    };
    if reserved.is_some_and(|expected| expected != family) {
        Err(ContractError::Privacy)
    } else {
        Ok(())
    }
}

/// Parses bounded JSON with duplicate-key rejection before typed decoding.
/// # Errors
/// Returns redacted errors for malformed/duplicate/oversized or invalid input.
pub fn decode_strict<T: serde::de::DeserializeOwned>(bytes: &[u8], max_bytes: usize) -> Result<T> {
    use serde::de::{MapAccess, SeqAccess, Visitor};
    struct Strict(serde_json::Value);
    impl<'de> Deserialize<'de> for Strict {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
            struct V;
            impl<'de> Visitor<'de> for V {
                type Value = Strict;
                fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    f.write_str("bounded unique-key JSON")
                }
                fn visit_bool<E: serde::de::Error>(
                    self,
                    v: bool,
                ) -> std::result::Result<Strict, E> {
                    Ok(Strict(v.into()))
                }
                fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Strict, E> {
                    Ok(Strict(v.into()))
                }
                fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Strict, E> {
                    Ok(Strict(v.into()))
                }
                fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<Strict, E> {
                    serde_json::Number::from_f64(v)
                        .map(|n| Strict(n.into()))
                        .ok_or_else(|| E::custom("invalid-number"))
                }
                fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<Strict, E> {
                    Ok(Strict(v.into()))
                }
                fn visit_string<E: serde::de::Error>(
                    self,
                    v: String,
                ) -> std::result::Result<Strict, E> {
                    Ok(Strict(v.into()))
                }
                fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Strict, E> {
                    Ok(Strict(serde_json::Value::Null))
                }
                fn visit_seq<A: SeqAccess<'de>>(
                    self,
                    mut a: A,
                ) -> std::result::Result<Strict, A::Error> {
                    let mut v = Vec::new();
                    while let Some(Strict(x)) = a.next_element()? {
                        v.push(x);
                    }
                    Ok(Strict(v.into()))
                }
                fn visit_map<A: MapAccess<'de>>(
                    self,
                    mut a: A,
                ) -> std::result::Result<Strict, A::Error> {
                    let mut m = serde_json::Map::new();
                    while let Some((k, Strict(v))) = a.next_entry::<String, Strict>()? {
                        if m.insert(k, v).is_some() {
                            return Err(serde::de::Error::custom("duplicate-key"));
                        }
                    }
                    Ok(Strict(m.into()))
                }
            }
            d.deserialize_any(V)
        }
    }
    if bytes.len() > max_bytes {
        return Err(ContractError::Limit);
    }
    let Strict(value) = serde_json::from_slice(bytes).map_err(|_| ContractError::Canonical)?;
    serde_json::from_value(value).map_err(|_| ContractError::Canonical)
}

/// Canonical MIME type/subtype, without header parameters or executable URLs.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct MediaType(String);
impl MediaType {
    /// # Errors
    /// Rejects noncanonical or parameter-bearing MIME coordinates.
    pub fn parse(value: impl Into<String>) -> Result<Self> {
        Self::try_from(value.into())
    }
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for MediaType {
    type Error = ContractError;
    fn try_from(s: String) -> Result<Self> {
        if s.len() > 255 {
            return Err(ContractError::Limit);
        }
        let parts: Vec<_> = s.split('/').collect();
        if parts.len() != 2
            || parts.iter().any(|p| {
                p.is_empty()
                    || !p.as_bytes()[0].is_ascii_alphanumeric()
                    || !p.bytes().all(|c| {
                        c.is_ascii_lowercase() || c.is_ascii_digit() || b"!#$&^_.+-".contains(&c)
                    })
            })
        {
            return Err(ContractError::Coordinate);
        }
        Ok(Self(s))
    }
}
impl From<MediaType> for String {
    fn from(v: MediaType) -> Self {
        v.0
    }
}
