//! Bounded metadata operations over supplied immutable `AssetSet` evidence, never extraction.
use super::expression::Function;
use super::value::{Shape, Type, TypedReference, Value};
use super::{ErrorCode, Result, VALUE_LIMIT, bytes};
use crate::contracts::{
    asset_set::AssetSetSnapshot,
    ids::{AssetId, AssetRepresentationId, ContentRevisionId, ProjectId},
    schema::{LocalName, ObjectKind, ObjectRef, QualifiedName},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryBinding {
    pub input_port: LocalName,
    pub field: QualifiedName,
    pub selector: QualifiedName,
    pub value_type: Type,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
pub enum Observation {
    Value(Box<Value>),
    Missing,
    Unavailable,
    Ambiguous,
    Conflict,
    StaleFingerprint,
    Forbidden,
    Revoked,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataMember {
    pub asset_id: AssetId,
    pub representation_id: Option<AssetRepresentationId>,
    pub content_revision_id: Option<ContentRevisionId>,
    pub observations: Vec<ObjectRef>,
    pub outcome: Observation,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataSpec {
    pub input: AssetSetSnapshot,
    pub field: QualifiedName,
    pub selector: QualifiedName,
    pub value_type: Type,
    pub members: Vec<MetadataMember>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "MetadataSpec", into = "MetadataSpec")]
pub struct MetadataCapture(MetadataSpec);
impl TryFrom<MetadataSpec> for MetadataCapture {
    type Error = super::ContextError;
    fn try_from(s: MetadataSpec) -> Result<Self> {
        s.value_type.validate(true)?;
        if s.members.len() > 10_000 || s.members.windows(2).any(|w| w[0].asset_id >= w[1].asset_id)
        {
            return Err(ErrorCode::Conflict.into());
        }
        if s.members.len() != s.input.members().len() {
            return Err(ErrorCode::Incomplete.into());
        }
        for row in &s.members {
            let member = s
                .input
                .members()
                .iter()
                .find(|a| a.asset_id == row.asset_id)
                .ok_or(ErrorCode::ScopeMismatch)?;
            if row.representation_id.is_some() != row.content_revision_id.is_some() {
                return Err(ErrorCode::InvalidCoordinate.into());
            }
            if let (Some(rep), Some(revision)) = (row.representation_id, row.content_revision_id)
                && !member
                    .representations
                    .iter()
                    .any(|r| r.representation_id == rep && r.content_revision_id == revision)
            {
                return Err(ErrorCode::StaleFingerprint.into());
            }
            if row.observations.len() > 256 {
                return Err(ErrorCode::LimitExceeded.into());
            }
            let keys = row
                .observations
                .iter()
                .map(|r| bytes(r, VALUE_LIMIT))
                .collect::<Result<Vec<_>>>()?;
            if keys.windows(2).any(|w| w[0] >= w[1])
                || row.observations.iter().any(|r| r.kind != ObjectKind::Json)
            {
                return Err(ErrorCode::InvalidCoordinate.into());
            }
            if let Observation::Value(v) = &row.outcome {
                if row.representation_id.is_none() || row.observations.is_empty() {
                    return Err(ErrorCode::Incomplete.into());
                }
                s.value_type.check(v)?;
            }
        }
        bytes(&s, VALUE_LIMIT)?;
        Ok(Self(s))
    }
}
impl From<MetadataCapture> for MetadataSpec {
    fn from(v: MetadataCapture) -> Self {
        v.0
    }
}
impl MetadataCapture {
    #[must_use]
    pub fn spec(&self) -> &MetadataSpec {
        &self.0
    }
    pub fn evaluate(&self, function: Function) -> Result<Value> {
        let s = &self.0;
        match function {
            Function::MetadataValues => Ok(Value::List(
                s.members
                    .iter()
                    .map(|m| row_value(s.input.project_id(), m))
                    .collect(),
            )),
            Function::MetadataCommon => {
                let mut common = None;
                for m in &s.members {
                    let v = observation_value(&m.outcome)?;
                    if common.is_some_and(|old| old != v) {
                        return Err(ErrorCode::Conflict.into());
                    }
                    common = Some(v);
                }
                common.cloned().ok_or_else(|| ErrorCode::Unavailable.into())
            }
            Function::MetadataDistinct => {
                let mut groups: BTreeMap<Vec<u8>, (Value, Vec<Value>)> = BTreeMap::new();
                let mut issues = Vec::new();
                for m in &s.members {
                    if let Observation::Value(v) = &m.outcome {
                        let group = groups
                            .entry(bytes(v, VALUE_LIMIT)?)
                            .or_insert_with(|| (*v.clone(), vec![]));
                        group.1.push(asset(s.input.project_id(), m.asset_id));
                    } else {
                        issues.push(row_value(s.input.project_id(), m));
                    }
                }
                let values = groups
                    .into_values()
                    .map(|(v, assets)| record([("value", v), ("assets", Value::List(assets))]))
                    .collect();
                Ok(record([
                    ("values", Value::List(values)),
                    ("issues", Value::List(issues)),
                ]))
            }
            _ => Err(ErrorCode::UnsupportedVersion.into()),
        }
    }
}
fn name(s: &str) -> LocalName {
    LocalName::parse(s).expect("static field")
}
fn record<const N: usize>(fields: [(&str, Value); N]) -> Value {
    Value::Record(fields.into_iter().map(|(k, v)| (name(k), v)).collect())
}
fn asset(project_id: ProjectId, asset_id: AssetId) -> Value {
    Value::Reference(TypedReference::Asset {
        project_id,
        asset_id,
    })
}
fn status(o: &Observation) -> &'static str {
    match o {
        Observation::Value(_) => "value",
        Observation::Missing => "missing",
        Observation::Unavailable => "unavailable",
        Observation::Ambiguous => "ambiguous",
        Observation::Conflict => "conflict",
        Observation::StaleFingerprint => "stale_fingerprint",
        Observation::Forbidden => "forbidden",
        Observation::Revoked => "revoked",
    }
}
fn row_value(p: ProjectId, m: &MetadataMember) -> Value {
    record([
        ("asset", asset(p, m.asset_id)),
        ("status", Value::Enum(name(status(&m.outcome)))),
        (
            "value",
            Value::Optional(if let Observation::Value(v) = &m.outcome {
                Some(v.clone())
            } else {
                None
            }),
        ),
    ])
}
fn observation_value(o: &Observation) -> Result<&Value> {
    match o {
        Observation::Value(v) => Ok(v),
        Observation::Missing | Observation::Unavailable => Err(ErrorCode::Unavailable.into()),
        Observation::Ambiguous => Err(ErrorCode::Ambiguous.into()),
        Observation::Conflict => Err(ErrorCode::Conflict.into()),
        Observation::StaleFingerprint => Err(ErrorCode::StaleFingerprint.into()),
        Observation::Forbidden => Err(ErrorCode::Forbidden.into()),
        Observation::Revoked => Err(ErrorCode::Revoked.into()),
    }
}
pub fn result_type(function: Function, t: &Type) -> Result<Type> {
    t.validate(true)?;
    let reference = Type::builtin(Shape::Reference);
    let row = Type::builtin(Shape::Record {
        fields: [
            (name("asset"), reference.clone()),
            (
                name("status"),
                Type::builtin(Shape::Enum {
                    members: [
                        "ambiguous",
                        "conflict",
                        "forbidden",
                        "missing",
                        "revoked",
                        "stale_fingerprint",
                        "unavailable",
                        "value",
                    ]
                    .map(name)
                    .into(),
                }),
            ),
            (
                name("value"),
                Type::builtin(Shape::Optional {
                    element: Box::new(t.clone()),
                }),
            ),
        ]
        .into(),
    });
    let list = |element: Type| {
        Type::builtin(Shape::List {
            element: Box::new(element),
            max_items: 10_000,
        })
    };
    Ok(match function {
        Function::MetadataCommon => t.clone(),
        Function::MetadataValues => list(row),
        Function::MetadataDistinct => Type::builtin(Shape::Record {
            fields: [
                (
                    name("values"),
                    list(Type::builtin(Shape::Record {
                        fields: [
                            (name("value"), t.clone()),
                            (name("assets"), list(reference)),
                        ]
                        .into(),
                    })),
                ),
                (name("issues"), list(row)),
            ]
            .into(),
        }),
        _ => return Err(ErrorCode::UnsupportedVersion.into()),
    })
}
