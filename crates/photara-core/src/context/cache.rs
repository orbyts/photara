//! Version-2 key material and current-authorization lookup checks; no cache storage.
use super::expression::Dependency;
use super::snapshot::{
    Audience, ContextSnapshot, DeviceContextSnapshot, Replayability, Sensitivity, Versions,
};
use super::value::TypedValue;
use super::{ErrorCode, Result, SNAPSHOT_LIMIT, digest};
use crate::contracts::{
    access::Principal,
    dto::AuthorizationGeneration,
    ids::{GraphId, NodeInstanceId, ProjectId},
    schema::{Digest, LocalName, QualifiedName, Version},
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorizationBoundary {
    pub project_id: ProjectId,
    pub principal: Principal,
    pub generation: AuthorizationGeneration,
    pub audience: Audience,
    pub sensitivity_ceiling: Sensitivity,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefinitionPin {
    pub package_id: QualifiedName,
    pub package_version: crate::PackageVersion,
    pub definition_id: QualifiedName,
    pub definition_version: Version,
    pub implementation_digest: Digest,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputValue {
    pub port_id: LocalName,
    pub values: Vec<TypedValue>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CacheEligibility {
    Pure,
    CapturedRead,
    NonDeterministic,
    Effect,
    SecretTainted,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CacheLocation {
    Device,
    Project,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeKeySpec {
    pub key_version: Version,
    pub contract_version: Version,
    pub versions: Versions,
    pub node_id: NodeInstanceId,
    pub graph_id: GraphId,
    pub definition: DefinitionPin,
    pub configuration: TypedValue,
    pub authored_state: Option<TypedValue>,
    pub inputs: Vec<InputValue>,
    pub dependencies: Vec<Dependency>,
    pub environment_digest: Digest,
    pub authorization: AuthorizationBoundary,
    pub eligibility: CacheEligibility,
    pub location: CacheLocation,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NodeCacheKey {
    digest: Digest,
    authorization: AuthorizationBoundary,
    location: CacheLocation,
    device_digest: Option<Digest>,
}
impl NodeCacheKey {
    pub fn build(
        spec: &NodeKeySpec,
        snapshot: &ContextSnapshot,
        device: Option<&DeviceContextSnapshot>,
    ) -> Result<Self> {
        if spec.key_version.get() != 2 || spec.contract_version != Version::FIRST {
            return Err(ErrorCode::UnsupportedVersion.into());
        }
        spec.versions.validate()?;
        if spec.versions != snapshot.spec().versions
            || spec.authorization.project_id != snapshot.spec().project_id
        {
            return Err(ErrorCode::ScopeMismatch.into());
        }
        snapshot.validate_audience(&spec.authorization.audience)?;
        if !matches!(
            spec.eligibility,
            CacheEligibility::Pure | CacheEligibility::CapturedRead
        ) || snapshot.replayability() == Replayability::Blocked
        {
            return Err(ErrorCode::Forbidden.into());
        }
        if spec
            .definition
            .definition_id
            .as_str()
            .strip_prefix(spec.definition.package_id.as_str())
            .is_none_or(|s| !s.starts_with('.'))
        {
            return Err(ErrorCode::InvalidCoordinate.into());
        }
        spec.configuration.validate(true)?;
        if let Some(s) = &spec.authored_state {
            s.validate(true)?;
        }
        if spec.inputs.len() > 256 || spec.inputs.windows(2).any(|w| w[0].port_id >= w[1].port_id) {
            return Err(ErrorCode::Conflict.into());
        }
        for input in &spec.inputs {
            if input.values.len() > 1 {
                return Err(ErrorCode::LimitExceeded.into());
            }
            for value in &input.values {
                value.validate(false)?;
            }
        }
        let portable = portable_dependencies(spec, snapshot, device)?;
        let facts = snapshot.subset(&portable)?;
        for fact in &facts {
            fact.resolved()?;
            if fact.sensitivity > spec.authorization.sensitivity_ceiling {
                return Err(ErrorCode::Forbidden.into());
            }
        }
        let device_digest = device.map(DeviceContextSnapshot::digest);
        if snapshot.spec().device_required && device.is_none() {
            return Err(ErrorCode::Incomplete.into());
        }
        if device.is_some() && spec.location == CacheLocation::Project {
            return Err(ErrorCode::Privacy.into());
        }
        let digest = digest(
            &serde_json::json!({"domain":"photara.node-cache.v2","spec":spec,"context_digest":snapshot.subset_digest(&portable)?,"device_digest":device_digest}),
            SNAPSHOT_LIMIT,
        )?;
        Ok(Self {
            digest,
            authorization: spec.authorization.clone(),
            location: spec.location,
            device_digest,
        })
    }
    #[must_use]
    pub const fn digest(&self) -> Digest {
        self.digest
    }
    pub fn authorize_lookup(
        &self,
        current: &AuthorizationBoundary,
        current_access: bool,
        current_device_digest: Option<Digest>,
    ) -> Result<()> {
        if !current_access {
            return Err(ErrorCode::Revoked.into());
        }
        if current != &self.authorization {
            return Err(ErrorCode::Forbidden.into());
        }
        if self.device_digest != current_device_digest {
            return Err(ErrorCode::Stale.into());
        }
        Ok(())
    }
}
pub fn graph_cache_key(
    graph_id: GraphId,
    node_keys: &[(NodeInstanceId, Digest)],
    snapshot: &ContextSnapshot,
    environment: Digest,
    device: Option<&DeviceContextSnapshot>,
    authorization: &AuthorizationBoundary,
) -> Result<Digest> {
    if node_keys.len() > 10_000 || node_keys.windows(2).any(|w| w[0].0 >= w[1].0) {
        return Err(ErrorCode::Conflict.into());
    }
    if authorization.project_id != snapshot.spec().project_id {
        return Err(ErrorCode::ScopeMismatch.into());
    }
    snapshot.validate_audience(&authorization.audience)?;
    if snapshot.spec().device_required && device.is_none() {
        return Err(ErrorCode::Incomplete.into());
    }
    digest(
        &serde_json::json!({"domain":"photara.graph-cache.v2","graph_id":graph_id,"nodes":node_keys,"context":snapshot.content_digest(),"environment":environment,"device":device.map(DeviceContextSnapshot::digest),"authorization":authorization,"versions":snapshot.spec().versions}),
        SNAPSHOT_LIMIT,
    )
}

fn portable_dependencies(
    spec: &NodeKeySpec,
    snapshot: &ContextSnapshot,
    device: Option<&DeviceContextSnapshot>,
) -> Result<Vec<Dependency>> {
    let owner = crate::contracts::dto::ScopeRef::Node {
        project_id: spec.authorization.project_id,
        graph_id: spec.graph_id,
        node_id: spec.node_id,
    };
    for dep in &spec.dependencies {
        use super::expression::Coordinate;
        let allowed = match dep.coordinate {
            Coordinate::Variable { scope, .. } => {
                super::expression::scope_allows(owner, scope, snapshot.spec().owning_library_id)
            }
            Coordinate::Input {
                project_id,
                graph_id,
                node_id,
                ..
            }
            | Coordinate::MetadataQuery {
                project_id,
                graph_id,
                node_id,
                ..
            }
            | Coordinate::AssetMetadata {
                project_id,
                graph_id,
                node_id,
                ..
            } => {
                owner
                    == crate::contracts::dto::ScopeRef::Node {
                        project_id,
                        graph_id,
                        node_id,
                    }
            }
            Coordinate::HostPlace { place } => {
                device.ok_or(ErrorCode::Incomplete)?.resolve(place)?;
                true
            }
            _ => true,
        };
        if !allowed {
            return Err(ErrorCode::ScopeMismatch.into());
        }
    }
    Ok(spec
        .dependencies
        .iter()
        .filter(|d| {
            !matches!(
                d.coordinate,
                super::expression::Coordinate::HostPlace { .. }
            )
        })
        .cloned()
        .collect::<Vec<_>>())
}
