//! Closed typed payloads; secrets and live leases have no variant in this algebra.
use super::{AST_DEPTH, AST_NODES, ErrorCode, Result, VALUE_LIMIT, bytes};
use crate::contracts::{
    asset_set::AssetSetSnapshotDescriptor,
    ids::{AssetId, LibraryId, LocationId, OrganizationId, PersonId, ProjectId},
    resource::{RelativeComponents, ResourceDescriptor},
    schema::{
        LocalName, QualifiedName, SchemaRef, ValueFamily, ValueTypeRef, Version, validate_family,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Integer(i64);
impl Integer {
    #[must_use]
    pub const fn new(n: i64) -> Self {
        Self(n)
    }
    #[must_use]
    pub const fn get(&self) -> i64 {
        self.0
    }
}
impl TryFrom<String> for Integer {
    type Error = super::ContextError;
    fn try_from(s: String) -> Result<Self> {
        let n = s.parse::<i64>().map_err(|_| ErrorCode::Overflow)?;
        if n.to_string() != s {
            return Err(ErrorCode::InvalidCoordinate.into());
        }
        Ok(Self(n))
    }
}
impl From<Integer> for String {
    fn from(v: Integer) -> Self {
        v.0.to_string()
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decimal {
    pub coefficient: String,
    pub scale: u8,
}
impl Decimal {
    pub fn validate(&self) -> Result<()> {
        let n = self
            .coefficient
            .parse::<i128>()
            .map_err(|_| ErrorCode::Overflow)?;
        if n.to_string() != self.coefficient
            || self.coefficient.trim_start_matches('-').len() > 38
            || self.scale > 18
        {
            return Err(ErrorCode::InvalidCoordinate.into());
        }
        Ok(())
    }
    pub(crate) fn formatted(&self) -> String {
        let negative = self.coefficient.starts_with('-');
        let mut digits = self.coefficient.trim_start_matches('-').to_owned();
        while digits.len() <= usize::from(self.scale) {
            digits.insert(0, '0');
        }
        if self.scale > 0 {
            digits.insert(digits.len() - usize::from(self.scale), '.');
        }
        if negative {
            digits.insert(0, '-');
        }
        digits
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Timestamp(String);
impl TryFrom<String> for Timestamp {
    type Error = super::ContextError;
    fn try_from(source: String) -> Result<Self> {
        let bytes = source.as_bytes();
        if bytes.len() != 24
            || bytes[4] != b'-'
            || bytes[7] != b'-'
            || bytes[10] != b'T'
            || bytes[13] != b':'
            || bytes[16] != b':'
            || bytes[19] != b'.'
            || bytes[23] != b'Z'
        {
            return Err(ErrorCode::InvalidCoordinate.into());
        }
        for (i, c) in bytes.iter().enumerate() {
            if ![4, 7, 10, 13, 16, 19, 23].contains(&i) && !c.is_ascii_digit() {
                return Err(ErrorCode::InvalidCoordinate.into());
            }
        }
        let component = |a: usize, z: usize| {
            source[a..z]
                .parse::<u32>()
                .map_err(|_| ErrorCode::InvalidCoordinate)
        };
        let year = component(0, 4)?;
        let month = component(5, 7)?;
        let day = component(8, 10)?;
        let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let days = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if leap {
                    29
                } else {
                    28
                }
            }
            _ => 0,
        };
        if year == 0
            || day == 0
            || day > days
            || component(11, 13)? > 23
            || component(14, 16)? > 59
            || component(17, 19)? > 59
        {
            return Err(ErrorCode::InvalidCoordinate.into());
        }
        Ok(Self(source))
    }
}
impl From<Timestamp> for String {
    fn from(v: Timestamp) -> Self {
        v.0
    }
}
impl Timestamp {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum TypedReference {
    Person {
        library_id: LibraryId,
        person_id: PersonId,
    },
    Organization {
        library_id: LibraryId,
        organization_id: OrganizationId,
    },
    Location {
        library_id: LibraryId,
        location_id: LocationId,
    },
    Asset {
        project_id: ProjectId,
        asset_id: AssetId,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceValue {
    pub descriptor: ResourceDescriptor,
    pub components: Vec<String>,
}
impl ResourceValue {
    pub fn validate(&self) -> Result<()> {
        if !self.components.is_empty() {
            RelativeComponents::new(self.components.clone())?;
        }
        if matches!(
            self.descriptor,
            ResourceDescriptor::Managed { .. } | ResourceDescriptor::ProjectArtifacts { .. }
        ) && !self.components.is_empty()
        {
            return Err(ErrorCode::Forbidden.into());
        }
        if matches!(self.descriptor, ResourceDescriptor::ProjectRoot { .. })
            && self.components.first().is_some_and(|p| {
                matches!(
                    p.to_ascii_lowercase().as_str(),
                    "head" | "objects" | "commits" | "locks" | "manifest" | "manifests"
                )
            })
        {
            return Err(ErrorCode::Forbidden.into());
        }
        Ok(())
    }
    pub fn join(&self, components: &[String]) -> Result<Self> {
        let mut v = self.clone();
        v.components.extend_from_slice(components);
        v.validate()?;
        Ok(v)
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
pub enum Value {
    Bool(bool),
    String(String),
    Integer(Integer),
    Decimal(Decimal),
    Timestamp(Timestamp),
    Null,
    Enum(LocalName),
    Rational {
        numerator: Integer,
        denominator: Integer,
        unit: QualifiedName,
    },
    List(Vec<Value>),
    Record(BTreeMap<LocalName, Value>),
    Optional(Option<Box<Value>>),
    Reference(TypedReference),
    Resource(Box<ResourceValue>),
    SlotResource(SlotResourceValue),
    AssetSet(AssetSetSnapshotDescriptor),
    Metadata(Box<super::metadata::MetadataCapture>),
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Shape {
    Bool,
    String {
        max_bytes: usize,
    },
    Integer,
    Decimal {
        scale: u8,
    },
    Timestamp,
    Null,
    Enum {
        members: Vec<LocalName>,
    },
    Rational {
        unit: QualifiedName,
    },
    List {
        element: Box<Type>,
        max_items: usize,
    },
    Record {
        fields: BTreeMap<LocalName, Type>,
    },
    Optional {
        element: Box<Type>,
    },
    Reference,
    Resource,
    AssetSet,
    Metadata {
        element: Box<Type>,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Type {
    pub value_type: ValueTypeRef,
    pub schema: SchemaRef,
    pub family: ValueFamily,
    pub shape: Shape,
}
impl Type {
    /// Constructs an intrinsic type descriptor.
    /// # Panics
    /// Only if a compile-time intrinsic type coordinate is invalid.
    #[must_use]
    pub fn builtin(shape: Shape) -> Self {
        let (id, family) = match &shape {
            Shape::Bool => ("photara.bool", ValueFamily::Configuration),
            Shape::String { .. } => ("photara.string", ValueFamily::Configuration),
            Shape::Integer => ("photara.i64", ValueFamily::Configuration),
            Shape::Decimal { .. } => ("photara.decimal", ValueFamily::Configuration),
            Shape::Timestamp => ("photara.timestamp", ValueFamily::Configuration),
            Shape::Null => ("photara.null", ValueFamily::Configuration),
            Shape::Enum { .. } => ("photara.enum", ValueFamily::Configuration),
            Shape::Rational { .. } => ("photara.rational", ValueFamily::Configuration),
            Shape::List { .. } => ("photara.list", ValueFamily::Configuration),
            Shape::Record { .. } => ("photara.record", ValueFamily::Configuration),
            Shape::Optional { .. } => ("photara.optional", ValueFamily::Configuration),
            Shape::Reference => ("photara.library-reference", ValueFamily::LibraryReference),
            Shape::Resource => (
                "photara.resource-descriptor",
                ValueFamily::ResourceDescriptor,
            ),
            Shape::AssetSet => ("photara.asset-set", ValueFamily::AssetSet),
            Shape::Metadata { .. } => ("photara.metadata-set", ValueFamily::MetadataSet),
        };
        let id = QualifiedName::parse(id).expect("fixed qualified type");
        Self {
            value_type: ValueTypeRef {
                id: id.clone(),
                version: if matches!(shape, Shape::AssetSet) {
                    Version::new(2).expect("v2")
                } else {
                    Version::FIRST
                },
            },
            schema: SchemaRef {
                id,
                version: Version::FIRST,
            },
            family,
            shape,
        }
    }
    #[must_use]
    pub fn string() -> Self {
        Self::builtin(Shape::String {
            max_bytes: VALUE_LIMIT,
        })
    }
    pub fn validate(&self, variable: bool) -> Result<()> {
        let mut nodes = 0;
        self.walk(variable, 0, &mut nodes)
    }
    fn walk(&self, variable: bool, depth: usize, nodes: &mut usize) -> Result<()> {
        *nodes += 1;
        if depth >= AST_DEPTH || *nodes > AST_NODES {
            return Err(ErrorCode::LimitExceeded.into());
        }
        validate_family(&self.value_type, self.family)?;
        let builtin = Self::builtin(self.shape.clone());
        if self.value_type.id.as_str().starts_with("photara.")
            && self.value_type != builtin.value_type
        {
            return Err(ErrorCode::UnsupportedVersion.into());
        }
        if self.schema.id.as_str().starts_with("photara.") && self.schema != builtin.schema {
            return Err(ErrorCode::UnsupportedVersion.into());
        }
        if variable && !self.family.variable_allowed() {
            return Err(ErrorCode::Privacy.into());
        }
        let required = match self.shape {
            Shape::AssetSet => ValueFamily::AssetSet,
            Shape::Metadata { .. } => ValueFamily::MetadataSet,
            Shape::Resource => ValueFamily::ResourceDescriptor,
            Shape::Reference => ValueFamily::LibraryReference,
            _ => ValueFamily::Configuration,
        };
        if self.family != required {
            return Err(ErrorCode::TypeMismatch.into());
        }
        match &self.shape {
            Shape::String { max_bytes } if *max_bytes > VALUE_LIMIT => {
                return Err(ErrorCode::LimitExceeded.into());
            }
            Shape::Decimal { scale } if *scale > 18 => return Err(ErrorCode::LimitExceeded.into()),
            Shape::Enum { members } => {
                if members.is_empty() || members.len() > 1024 {
                    return Err(ErrorCode::LimitExceeded.into());
                }
                crate::contracts::schema::ordered_unique(members)?;
            }
            Shape::List { element, max_items } => {
                if *max_items > 10_000 {
                    return Err(ErrorCode::LimitExceeded.into());
                }
                element.walk(variable, depth + 1, nodes)?;
            }
            Shape::Optional { element } => element.walk(variable, depth + 1, nodes)?,
            Shape::Metadata { element } => element.walk(true, depth + 1, nodes)?,
            Shape::Record { fields } => {
                for t in fields.values() {
                    t.walk(variable, depth + 1, nodes)?;
                }
            }
            Shape::AssetSet if self.value_type != Self::builtin(Shape::AssetSet).value_type => {
                return Err(ErrorCode::UnsupportedVersion.into());
            }
            _ => {}
        }
        Ok(())
    }
    pub fn check(&self, value: &Value) -> Result<()> {
        self.validate(false)?;
        self.check_inner(value)?;
        bytes(value, VALUE_LIMIT)?;
        Ok(())
    }
    fn check_inner(&self, value: &Value) -> Result<()> {
        match (&self.shape, value) {
            (Shape::Bool, Value::Bool(_))
            | (Shape::Integer, Value::Integer(_))
            | (Shape::Timestamp, Value::Timestamp(_))
            | (Shape::Null, Value::Null)
            | (Shape::Reference, Value::Reference(_))
            | (Shape::Optional { .. }, Value::Optional(None)) => {}
            (Shape::String { max_bytes }, Value::String(s)) if s.len() <= *max_bytes => {}
            (Shape::Decimal { scale }, Value::Decimal(d)) if d.scale == *scale => d.validate()?,
            (Shape::Enum { members }, Value::Enum(n)) if members.contains(n) => {}
            (
                Shape::Rational { unit },
                Value::Rational {
                    denominator,
                    unit: u,
                    ..
                },
            ) if u == unit && denominator.get() > 0 => {}
            (Shape::List { element, max_items }, Value::List(v)) if v.len() <= *max_items => {
                for v in v {
                    element.check_inner(v)?;
                }
            }
            (Shape::Record { fields }, Value::Record(values)) if fields.len() == values.len() => {
                for (k, t) in fields {
                    t.check_inner(values.get(k).ok_or(ErrorCode::TypeMismatch)?)?;
                }
            }
            (Shape::Optional { element }, Value::Optional(Some(v))) => element.check_inner(v)?,
            (Shape::Resource, Value::Resource(r)) => r.validate()?,
            (Shape::Resource, Value::SlotResource(r)) => r.validate()?,
            (Shape::AssetSet, Value::AssetSet(s)) => s.validate()?,
            (Shape::Metadata { element }, Value::Metadata(m))
                if element.as_ref() == &m.spec().value_type =>
            {
                super::metadata::MetadataCapture::try_from(m.spec().clone())?;
            }
            _ => return Err(ErrorCode::TypeMismatch.into()),
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypedValue {
    pub ty: Type,
    pub value: Value,
}
impl TypedValue {
    pub fn validate(&self, variable: bool) -> Result<()> {
        self.ty.validate(variable)?;
        self.ty.check(&self.value)
    }
}

/// Host-owned exact type/schema registry. Registration never grants a read or effect.
#[derive(Clone, Debug, Default)]
pub struct TypeRegistry {
    types: BTreeMap<(ValueTypeRef, SchemaRef), Type>,
}
impl TypeRegistry {
    pub fn register(&mut self, ty: Type) -> Result<()> {
        ty.validate(false)?;
        let key = (ty.value_type.clone(), ty.schema.clone());
        if self.types.contains_key(&key) {
            return Err(ErrorCode::Conflict.into());
        }
        self.types.insert(key, ty);
        Ok(())
    }
    pub fn validate_type(&self, ty: &Type) -> Result<()> {
        ty.validate(false)?;
        if self.types.get(&(ty.value_type.clone(), ty.schema.clone())) != Some(ty) {
            return Err(ErrorCode::UnsupportedVersion.into());
        }
        Ok(())
    }
    pub fn validate_value(&self, value: &TypedValue) -> Result<()> {
        self.validate_type(&value.ty)?;
        value.validate(false)
    }
}

/// Logical slot root and narrowed relative address, pinned to its captured target.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlotResourceValue {
    pub capture: crate::contracts::resource::SlotCapture,
    pub components: Vec<String>,
}
impl SlotResourceValue {
    pub fn validate(&self) -> Result<()> {
        if !self.components.is_empty() {
            RelativeComponents::new(self.components.clone())?;
        }
        Ok(())
    }
    pub fn join(&self, components: &[String]) -> Result<Self> {
        let mut next = self.clone();
        next.components.extend_from_slice(components);
        next.validate()?;
        Ok(next)
    }
}
