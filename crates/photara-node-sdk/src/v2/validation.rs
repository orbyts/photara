use super::types::{
    AccessMode, AuthoredSchema, CacheScope, CapabilityRequirement, ComponentAction,
    ComponentBinding, ContractRegistry, DefinitionCoordinate, Determinism, ExecutionContract,
    FallbackReason, HostAvailability, HostComponentId, IdRemapContract, ManifestInspection,
    ManifestSpec, NodeDefinitionV2, NodeFallbackDescriptor, NodePackageManifestV2, PortCardinality,
    PortDirection, RequiredFeature, SelectorScope,
};
use photara_core::contracts::{
    access::ProjectAction,
    resource::ResourceRight,
    schema::{
        ContractError, LocalName, Result, SchemaRef, ValueFamily, ValueShape, ValueTypeRef,
        Version, decode_strict, display_text, ordered_unique, unique, validate_family,
    },
};
use std::collections::BTreeSet;

pub const MAX_MANIFEST_BYTES: usize = 1_048_576;
pub const CATEGORIES: &[&str] = &[
    "photara.category.sources.filesystem",
    "photara.category.sources.application-catalog",
    "photara.category.sources.cloud",
    "photara.category.selection",
    "photara.category.filtering",
    "photara.category.context.reference",
    "photara.category.context.query",
    "photara.category.metadata.enrichment",
    "photara.category.metadata.classification",
    "photara.category.layout.composition",
    "photara.category.processing.application",
    "photara.category.processing.ai",
    "photara.category.inspection.review",
    "photara.category.output.metadata",
    "photara.category.output.files",
    "photara.category.output.application-update",
    "photara.category.delivery.cloud",
    "photara.category.publishing.social",
    "photara.category.publishing.web",
    "photara.category.automation.control",
];
impl TryFrom<ManifestSpec> for NodePackageManifestV2 {
    type Error = ContractError;
    fn try_from(m: ManifestSpec) -> Result<Self> {
        if m.manifest_schema_version != 2 || m.contract_version != Version::FIRST {
            return Err(ContractError::Unsupported);
        }
        display_text(&m.display_name, 1024)?;
        ordered_unique(&m.required_features)?;
        if m.definitions.is_empty() || m.definitions.len() > 256 {
            return Err(ContractError::Limit);
        }
        for required in [
            RequiredFeature::TypedPorts,
            RequiredFeature::ContextDeclarations,
            RequiredFeature::Inspector,
        ] {
            if !m.required_features.contains(&required) {
                return Err(ContractError::Missing);
            }
        }
        let mut coords = BTreeSet::new();
        for d in &m.definitions {
            if d.coordinate.package_id != m.package_id
                || d.coordinate.package_version != m.package_version
            {
                return Err(ContractError::Scope);
            }
            if !coords.insert(&d.coordinate) {
                return Err(ContractError::Duplicate);
            }
            d.validate()?;
            if d.work_surface.is_some()
                && !m.required_features.contains(&RequiredFeature::WorkSurface)
            {
                return Err(ContractError::Missing);
            }
            if d.ports.iter().any(|p| p.family == ValueFamily::AssetSet)
                && !m.required_features.contains(&RequiredFeature::AssetSetV2)
            {
                return Err(ContractError::Missing);
            }
            if d.capabilities.iter().any(|c| {
                matches!(
                    c,
                    CapabilityRequirement::Resource { .. } | CapabilityRequirement::Place { .. }
                )
            }) && !m
                .required_features
                .contains(&RequiredFeature::ResourceDescriptors)
            {
                return Err(ContractError::Missing);
            }
        }
        if photara_core::canonical_json(&m)
            .map_err(|_| ContractError::Serialization)?
            .len()
            > MAX_MANIFEST_BYTES
        {
            return Err(ContractError::Limit);
        }
        Ok(Self(m))
    }
}
impl NodePackageManifestV2 {
    #[must_use]
    pub const fn spec(&self) -> &ManifestSpec {
        &self.0
    }
    /// # Errors
    /// Rejects malformed, duplicate-key, unknown, oversized or inconsistent contracts.
    pub fn from_json(bytes: &[u8]) -> Result<Self> {
        decode_strict(bytes, MAX_MANIFEST_BYTES)
    }
    /// # Errors
    /// Rejects serialization failures; uses unchanged canonical-json.v1.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        photara_core::canonical_json(self).map_err(|_| ContractError::Serialization)
    }
    /// # Errors
    /// Rejects unregistered schemas/value families or inconsistent registry claims.
    pub fn validate_registered(&self, registry: &ContractRegistry) -> Result<()> {
        for d in &self.0.definitions {
            d.validate_registered(registry)?;
        }
        Ok(())
    }
}
impl DefinitionCoordinate {
    fn validate(&self) -> Result<()> {
        if !self
            .definition_id
            .as_str()
            .starts_with(&format!("{}.", self.package_id))
        {
            return Err(ContractError::Scope);
        }
        Ok(())
    }
}
impl AuthoredSchema {
    fn validate(&self) -> Result<()> {
        if self.fields.len() > 1024 {
            return Err(ContractError::Limit);
        }
        let mut names = BTreeSet::new();
        for field in &self.fields {
            if !names.insert(&field.name) {
                return Err(ContractError::Duplicate);
            }
            field.shape.validate_variable_shape()?;
        }
        Ok(())
    }
}
impl NodeDefinitionV2 {
    /// Validates every independent required contract axis, without granting rights.
    /// # Errors
    /// Rejects inconsistent ports, capabilities, cache, schemas, UI or migration axes.
    pub fn validate(&self) -> Result<()> {
        self.coordinate.validate()?;
        self.configuration_schema.validate()?;
        if let Some(state) = &self.state_schema {
            state.validate()?;
        }
        if self.versions.contract_version != Version::FIRST
            || self.versions.context_version != Version::FIRST
            || self.versions.expression_version != Version::FIRST
            || self.versions.evaluation_key_version.get() != 2
            || self.context.contract_version != self.versions.context_version
            || self.context.expression_version != self.versions.expression_version
            || self.cache.evaluation_key_version != self.versions.evaluation_key_version
        {
            return Err(ContractError::Unsupported);
        }
        if self.ports.len() > 256 || self.capabilities.len() > 256 {
            return Err(ContractError::Limit);
        }
        let mut ports = BTreeSet::new();
        for p in &self.ports {
            if !ports.insert(&p.id) {
                return Err(ContractError::Duplicate);
            }
            validate_family(&p.value_type, p.family)?;
            if p.direction == PortDirection::Output && p.cardinality != PortCardinality::One {
                return Err(ContractError::Union);
            }
            if matches!(
                p.family,
                ValueFamily::SecretRef
                    | ValueFamily::LiveHandle
                    | ValueFamily::WorkflowOutputReference
            ) {
                return Err(ContractError::Privacy);
            }
            if p.family == ValueFamily::AssetSet
                && (p.value_type.id.as_str() != "photara.asset-set"
                    || p.value_type.version.get() != 2)
            {
                return Err(ContractError::Unsupported);
            }
        }
        let mut slots = BTreeSet::new();
        for cap in &self.capabilities {
            if !slots.insert(cap.slot()) {
                return Err(ContractError::Duplicate);
            }
            self.validate_capability(cap)?;
        }
        self.validate_execution()?;
        self.validate_cache()?;
        self.validate_presentation()?;
        if self.platforms.is_empty() || self.platforms.len() > 3 {
            return Err(ContractError::Missing);
        }
        let mut platforms = Vec::new();
        for p in &self.platforms {
            if platforms.contains(&p.platform) {
                return Err(ContractError::Duplicate);
            }
            platforms.push(p.platform);
            if p.runtimes.is_empty() {
                return Err(ContractError::Missing);
            }
            ordered_unique(&p.runtimes)?;
            ordered_unique(&p.tools)?;
        }
        let mut facts = BTreeSet::new();
        for fact in &self.context.captured_facts {
            if !facts.insert(&fact.id) {
                return Err(ContractError::Duplicate);
            }
            if !slots.contains(&fact.source_slot) {
                return Err(ContractError::Missing);
            }
            if matches!(
                self.capability(&fact.source_slot),
                Some(CapabilityRequirement::Credential { .. })
            ) {
                return Err(ContractError::Privacy);
            }
        }
        if let Determinism::Captured { facts: expected } = &self.determinism {
            ordered_unique(expected)?;
            if expected.is_empty() || expected.iter().collect::<BTreeSet<_>>() != facts {
                return Err(ContractError::Missing);
            }
        }
        self.validate_migrations()
    }
    fn validate_migrations(&self) -> Result<()> {
        let mut migrations = BTreeSet::new();
        for m in &self.migrations {
            m.from.validate()?;
            m.to.validate()?;
            if !migrations.insert(&m.migration_id) {
                return Err(ContractError::Duplicate);
            }
            if !m.deterministic
                || m.from == m.to
                || m.to != self.coordinate
                || m.to_config != self.configuration_schema.schema
                || m.to_state.as_ref() != self.state_schema.as_ref().map(|s| &s.schema)
            {
                return Err(ContractError::Union);
            }
            if m.from.package_id != m.to.package_id || m.from.definition_id != m.to.definition_id {
                return Err(ContractError::Scope);
            }
        }
        Ok(())
    }
    fn capability(&self, slot: &LocalName) -> Option<&CapabilityRequirement> {
        self.capabilities.iter().find(|c| c.slot() == slot)
    }
    fn validate_capability(&self, cap: &CapabilityRequirement) -> Result<()> {
        let pure = matches!(self.execution, ExecutionContract::Pure);
        let effect = matches!(self.execution, ExecutionContract::Effect { .. });
        match cap {
            CapabilityRequirement::Selector {
                scope,
                access,
                actions,
                fields,
                max_items,
                ..
            } => {
                if *max_items == 0 || *max_items > 10_000 || fields.is_empty() {
                    return Err(ContractError::Limit);
                }
                ordered_unique(fields)?;
                if !actions.allows(ProjectAction::Read) || (pure && *access == AccessMode::Live) {
                    return Err(ContractError::Union);
                }
                if *access == AccessMode::Live && !effect && actions.bits() & 0xfc != 0 {
                    return Err(ContractError::Union);
                }
                if let SelectorScope::InputPort { port_id } = scope
                    && (*access != AccessMode::Captured
                        || !self
                            .ports
                            .iter()
                            .any(|p| p.id == *port_id && p.direction == PortDirection::Input))
                {
                    return Err(ContractError::Scope);
                }
            }
            CapabilityRequirement::Resource {
                access,
                rights,
                max_items,
                operations,
                storage_class,
                ..
            } => {
                if *max_items == 0 || *max_items > 10_000 {
                    return Err(ContractError::Limit);
                }
                validate_resource_capability(pure, effect, *access, rights, operations)?;
                if *storage_class == photara_core::contracts::resource::StorageClass::ManagedProject
                    && rights.writes()
                {
                    return Err(ContractError::Union);
                }
            }
            CapabilityRequirement::Place {
                access,
                rights,
                operations,
                ..
            } => validate_resource_capability(pure, effect, *access, rights, operations)?,
            CapabilityRequirement::Credential { operations, .. } => {
                if pure || operations.is_empty() {
                    return Err(ContractError::Union);
                }
                ordered_unique(operations)?;
            }
        }
        Ok(())
    }
    fn validate_execution(&self) -> Result<()> {
        let operations = match &self.execution {
            ExecutionContract::Effect { operations } => {
                if operations.is_empty() || operations.len() > 256 {
                    return Err(ContractError::Missing);
                }
                operations
            }
            _ => return self.validate_non_effect_operations(),
        };
        let mut ids = BTreeSet::new();
        for op in operations {
            if !ids.insert(&op.id) {
                return Err(ContractError::Duplicate);
            }
            match self.capability(&op.target_slot) {
                Some(
                    CapabilityRequirement::Resource {
                        access: AccessMode::Live,
                        rights,
                        operations,
                        ..
                    }
                    | CapabilityRequirement::Place {
                        access: AccessMode::Live,
                        rights,
                        operations,
                        ..
                    },
                ) => {
                    if !operations.contains(&op.id)
                        || op.rights.is_empty()
                        || !rights.permits(&op.rights)
                    {
                        return Err(ContractError::Union);
                    }
                }
                _ => return Err(ContractError::Missing),
            }
            if op.rights.contains(ResourceRight::Replace) && !op.replacement_supported {
                return Err(ContractError::Union);
            }
            if let Some(slot) = &op.credential_slot
                && !matches!(self.capability(slot),Some(CapabilityRequirement::Credential {operations,..}) if operations.contains(&op.id))
            {
                return Err(ContractError::Missing);
            }
        }
        for cap in &self.capabilities {
            let named = match cap {
                CapabilityRequirement::Resource { operations, .. }
                | CapabilityRequirement::Place { operations, .. }
                | CapabilityRequirement::Credential { operations, .. } => operations,
                CapabilityRequirement::Selector { .. } => continue,
            };
            if named.iter().any(|id| !ids.contains(id)) {
                return Err(ContractError::Missing);
            }
        }
        Ok(())
    }
    fn validate_non_effect_operations(&self) -> Result<()> {
        for cap in &self.capabilities {
            // Read adapters may name read-only provider operations; there is no effect dispatch.
            if matches!(self.execution, ExecutionContract::Pure)
                && matches!(cap,CapabilityRequirement::Resource {operations,..}|CapabilityRequirement::Place {operations,..} if !operations.is_empty())
            {
                return Err(ContractError::Union);
            }
        }
        Ok(())
    }
    fn validate_cache(&self) -> Result<()> {
        if self.cache.scope != CacheScope::None {
            if matches!(self.execution, ExecutionContract::Effect { .. })
                || self.determinism == Determinism::NonDeterministic
            {
                return Err(ContractError::Union);
            }
            if matches!(self.execution, ExecutionContract::Read)
                && !matches!(self.determinism, Determinism::Captured { .. })
            {
                return Err(ContractError::Missing);
            }
            if matches!(self.determinism, Determinism::Captured { .. })
                && !self.cache.verified_capture_required
            {
                return Err(ContractError::Missing);
            }
            if self
                .capabilities
                .iter()
                .any(|c| matches!(c, CapabilityRequirement::Credential { .. }))
            {
                return Err(ContractError::Privacy);
            }
            if self.cache.scope == CacheScope::Project
                && self
                    .capabilities
                    .iter()
                    .any(|c| matches!(c, CapabilityRequirement::Place { .. }))
            {
                return Err(ContractError::Privacy);
            }
        }
        Ok(())
    }
    fn validate_presentation(&self) -> Result<()> {
        let p = &self.presentation;
        display_text(&p.brand, 1024)?;
        display_text(&p.display_name, 1024)?;
        if p.taxonomy_version != Version::FIRST
            || !CATEGORIES.contains(&p.primary_category.as_str())
        {
            return Err(ContractError::Unsupported);
        }
        if p.search_terms.is_empty() || p.search_terms.len() > 256 {
            return Err(ContractError::Missing);
        }
        unique(&p.search_terms)?;
        for term in &p.search_terms {
            display_text(term, 256)?;
        }
        ordered_unique(&p.provider_tags)?;
        ordered_unique(&p.capability_tags)?;
        if self.inspector.contribution.contract_version != Version::FIRST {
            return Err(ContractError::Unsupported);
        }
        unique(&self.inspector.sections)?;
        if self.inspector.sections.len() != 6 {
            return Err(ContractError::Missing);
        }
        if let Some(work) = &self.work_surface {
            if work.contribution.contract_version != Version::FIRST {
                return Err(ContractError::Unsupported);
            }
            if work.components.len() > 256 {
                return Err(ContractError::Limit);
            }
            let mut ids = BTreeSet::new();
            for c in &work.components {
                if !ids.insert(&c.instance_id) {
                    return Err(ContractError::Duplicate);
                }
                if c.contract_version != Version::FIRST {
                    return Err(ContractError::Unsupported);
                }
                unique(&c.actions)?;
                match (&c.component_id, &c.input) {
                    (HostComponentId::Assets, ComponentBinding::AssetPort { port_id }) => {
                        if !self.ports.iter().any(|p| {
                            p.id == *port_id
                                && p.direction == PortDirection::Input
                                && p.family == ValueFamily::AssetSet
                                && p.value_type.version.get() == 2
                        }) || c.actions.contains(&ComponentAction::CreateLibraryRecord)
                        {
                            return Err(ContractError::Scope);
                        }
                    }
                    (HostComponentId::Assets, _) | (_, ComponentBinding::AssetPort { .. }) => {
                        return Err(ContractError::Union);
                    }
                    (
                        _,
                        ComponentBinding::LibraryQuery { selector_slot }
                        | ComponentBinding::AssignmentSubset { selector_slot },
                    ) => match self.capability(selector_slot) {
                        Some(CapabilityRequirement::Selector { scope, actions, .. }) => {
                            if matches!(c.input, ComponentBinding::LibraryQuery { .. })
                                && *scope != SelectorScope::Library
                            {
                                return Err(ContractError::Scope);
                            }
                            if c.actions.contains(&ComponentAction::CreateLibraryRecord)
                                && (!actions.allows(ProjectAction::Edit)
                                    || *scope != SelectorScope::Library)
                            {
                                return Err(ContractError::Scope);
                            }
                        }
                        _ => return Err(ContractError::Missing),
                    },
                }
            }
        }
        Ok(())
    }
}
fn validate_resource_capability(
    pure: bool,
    effect: bool,
    access: AccessMode,
    rights: &photara_core::contracts::resource::ResourceRights,
    operations: &[LocalName],
) -> Result<()> {
    if rights.is_empty()
        || (pure && access == AccessMode::Live)
        || (rights.writes() && (!effect || access != AccessMode::Live))
    {
        return Err(ContractError::Union);
    }
    ordered_unique(operations)?;
    Ok(())
}
impl ContractRegistry {
    /// # Errors
    /// Rejects duplicate registered schema coordinates.
    pub fn register_schema(&mut self, schema: SchemaRef) -> Result<()> {
        if self.schemas.insert(schema) {
            Ok(())
        } else {
            Err(ContractError::Duplicate)
        }
    }
    /// # Errors
    /// Rejects unregistered schemas, family mismatch and duplicate exact versions.
    pub fn register_value(
        &mut self,
        value_type: ValueTypeRef,
        schema: SchemaRef,
        family: ValueFamily,
    ) -> Result<()> {
        validate_family(&value_type, family)?;
        if !self.schemas.contains(&schema) {
            return Err(ContractError::Missing);
        }
        if self.values.contains_key(&value_type) {
            return Err(ContractError::Duplicate);
        }
        self.values.insert(value_type, (schema, family));
        Ok(())
    }
    fn schema(&self, s: &SchemaRef) -> Result<()> {
        if self.schemas.contains(s) {
            Ok(())
        } else {
            Err(ContractError::Missing)
        }
    }
    fn value(&self, v: &ValueTypeRef) -> Result<&(SchemaRef, ValueFamily)> {
        self.values.get(v).ok_or(ContractError::Missing)
    }
    fn shape(&self, s: &ValueShape) -> Result<()> {
        match s {
            ValueShape::Leaf { value_type, family } => {
                if self.value(value_type)?.1 != *family {
                    return Err(ContractError::Privacy);
                }
            }
            ValueShape::List { item, .. } => self.shape(item)?,
            ValueShape::Record { fields } => {
                for s in fields.values() {
                    self.shape(s)?;
                }
            }
        }
        Ok(())
    }
}
impl NodeDefinitionV2 {
    fn validate_registered(&self, r: &ContractRegistry) -> Result<()> {
        r.schema(&self.configuration_schema.schema)?;
        for field in &self.configuration_schema.fields {
            r.shape(&field.shape)?;
        }
        if let Some(s) = &self.state_schema {
            r.schema(&s.schema)?;
            for field in &s.fields {
                r.shape(&field.shape)?;
            }
        }
        r.schema(&self.inspector.schema)?;
        for p in &self.ports {
            if r.value(&p.value_type)? != &(p.schema.clone(), p.family) {
                return Err(ContractError::Union);
            }
        }
        for c in &self.capabilities {
            if let CapabilityRequirement::Selector { value_type, .. } = c {
                r.value(value_type)?;
            }
        }
        for f in &self.context.captured_facts {
            r.value(&f.value_type)?;
        }
        if let ExecutionContract::Effect { operations } = &self.execution {
            for op in operations {
                r.schema(&op.request_schema)?;
                r.schema(&op.receipt_schema)?;
            }
        }
        if let Some(w) = &self.work_surface {
            for c in &w.components {
                r.schema(&c.result_schema)?;
            }
        }
        for m in &self.migrations {
            for s in [&m.from_config, &m.to_config, &m.diagnostics_schema] {
                r.schema(s)?;
            }
            for s in [&m.from_state, &m.to_state].into_iter().flatten() {
                r.schema(s)?;
            }
            if let IdRemapContract::Typed { schema } = &m.id_remap {
                r.schema(schema)?;
            }
        }
        Ok(())
    }
    /// Chooses a safe host fallback from independent trust/runtime/skin facts.
    /// # Errors
    /// Rejects an invalid definition before evaluating host availability.
    pub fn availability(
        &self,
        registry: &ContractRegistry,
        host: &HostAvailability,
    ) -> Result<NodeFallbackDescriptor> {
        self.validate()?;
        let reason = if self.validate_registered(registry).is_err() {
            FallbackReason::MissingSchema
        } else if host.installed_implementation.is_none() {
            FallbackReason::MissingRuntime
        } else if !host.trusted {
            FallbackReason::Untrusted
        } else if host.installed_implementation != Some(self.versions.implementation_digest) {
            FallbackReason::IncompatibleRuntime
        } else if let Some(platform) = self.platforms.iter().find(|p| p.platform == host.platform) {
            if platform.runtimes.iter().all(|r| host.runtimes.contains(r))
                && platform.tools.iter().all(|t| host.tools.contains(t))
            {
                FallbackReason::Ready
            } else {
                FallbackReason::IncompatibleRuntime
            }
        } else {
            FallbackReason::UnsupportedPlatform
        };
        Ok(fallback(reason, host.native_skin_available))
    }
}
fn fallback(reason: FallbackReason, skin: bool) -> NodeFallbackDescriptor {
    let ready = reason == FallbackReason::Ready;
    NodeFallbackDescriptor {
        reason,
        preserve_exact_bytes: true,
        host_owned_inspector: true,
        portable_evaluation: ready,
        semantic_edits: ready,
        node_presentation_code: ready && skin,
        migration_execution: false,
    }
}
/// Inspects exact manifest bytes without executing presentation or migration code.
/// # Errors
/// Rejects excessive input; malformed/unknown bounded bytes are retained for backup.
pub fn inspect_manifest(bytes: &[u8]) -> Result<ManifestInspection> {
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(ContractError::Limit);
    }
    let value: Result<serde_json::Value> = decode_strict(bytes, MAX_MANIFEST_BYTES);
    let mut manifest = None;
    let reason = match value {
        Ok(v)
            if v.get("manifest_schema_version")
                .and_then(serde_json::Value::as_u64)
                == Some(1) =>
        {
            match serde_json::from_value::<crate::NodePackageManifest>(v) {
                Ok(m) if m.validate().is_ok() => FallbackReason::LegacyV1,
                _ => FallbackReason::Malformed,
            }
        }
        Ok(v)
            if v.get("manifest_schema_version")
                .and_then(serde_json::Value::as_u64)
                == Some(2) =>
        {
            match NodePackageManifestV2::from_json(bytes) {
                Ok(m) => {
                    manifest = Some(m);
                    FallbackReason::MissingRuntime
                }
                Err(_) => FallbackReason::UnsupportedContract,
            }
        }
        Ok(_) => FallbackReason::UnsupportedContract,
        Err(_) => FallbackReason::Malformed,
    };
    Ok(ManifestInspection {
        original: bytes.to_vec(),
        manifest,
        fallback: fallback(reason, false),
    })
}

/// Checks an exact v2 port connection without invoking either node.
/// # Errors
/// Rejects direction/cardinality, unregistered types, schemas or family mismatches.
pub fn validate_port_connection_v2(
    registry: &ContractRegistry,
    output: &super::PortContract,
    input: &super::PortContract,
) -> Result<()> {
    validate_family(&output.value_type, output.family)?;
    if matches!(
        output.family,
        ValueFamily::SecretRef | ValueFamily::LiveHandle | ValueFamily::WorkflowOutputReference
    ) {
        return Err(ContractError::Privacy);
    }
    if output.family == ValueFamily::AssetSet
        && (output.value_type.id.as_str() != "photara.asset-set"
            || output.value_type.version.get() != 2)
    {
        return Err(ContractError::Unsupported);
    }
    if output.direction != PortDirection::Output
        || input.direction != PortDirection::Input
        || output.cardinality != PortCardinality::One
    {
        return Err(ContractError::Union);
    }
    if output.value_type != input.value_type
        || output.schema != input.schema
        || output.family != input.family
    {
        return Err(ContractError::Union);
    }
    if registry.value(&output.value_type)? != &(output.schema.clone(), output.family) {
        return Err(ContractError::Union);
    }
    Ok(())
}
