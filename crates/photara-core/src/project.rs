use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    str::FromStr,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};
use serde_json::Value;
use thiserror::Error;

use crate::{
    AssetContextError, CanonicalDigest, ConnectionId, GraphDocument, NodeInstanceId, NodePackageId,
    PackageVersion, ProjectAssetContext, ProjectId, ProjectResourceId, SchemaVersion,
    canonical_digest,
};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ProjectRevision(u64);

impl ProjectRevision {
    #[must_use]
    pub const fn initial() -> Self {
        Self(0)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    #[must_use]
    pub const fn checked_next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct PackageRequirement {
    pub package_id: NodePackageId,
    pub package_version: PackageVersion,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProjectMetadata {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NodeGraphMetadata {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A normalized path resolved only relative to an explicit project root.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProjectRelativePath(String);

impl ProjectRelativePath {
    /// Normalizes a portable project-relative path.
    ///
    /// Backslashes become `/`, redundant separators and `.` are removed, and
    /// parent traversal, absolute roots, drive/scheme prefixes, NULs, and empty
    /// paths are rejected.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectRelativePathError`] when the path is not safely relative
    /// to a project root.
    pub fn parse(value: impl Into<String>) -> Result<Self, ProjectRelativePathError> {
        let original = value.into();
        if original.is_empty() {
            return Err(ProjectRelativePathError::Empty);
        }
        if original.contains('\0') {
            return Err(ProjectRelativePathError::Nul { value: original });
        }
        let portable = original.replace('\\', "/");
        if portable.starts_with('/')
            || portable.contains(':')
            || portable
                .as_bytes()
                .get(1)
                .is_some_and(|separator| *separator == b':')
        {
            return Err(ProjectRelativePathError::Absolute { value: original });
        }
        let mut segments = Vec::new();
        for segment in portable.split('/') {
            match segment {
                "" | "." => {}
                ".." => return Err(ProjectRelativePathError::ParentTraversal { value: original }),
                _ => segments.push(segment),
            }
        }
        if segments.is_empty() {
            return Err(ProjectRelativePathError::Empty);
        }
        Ok(Self(segments.join("/")))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProjectRelativePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for ProjectRelativePath {
    type Err = ProjectRelativePathError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl Serialize for ProjectRelativePath {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ProjectRelativePath {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(D::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ProjectRelativePathError {
    #[error("project-relative path must not be empty")]
    Empty,
    #[error(
        "project-relative path {value:?} must not be absolute, drive-qualified, or scheme-like"
    )]
    Absolute { value: String },
    #[error("project-relative path {value:?} must not traverse to a parent")]
    ParentTraversal { value: String },
    #[error("project-relative path {value:?} must not contain NUL")]
    Nul { value: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProjectResourceRef {
    pub id: ProjectResourceId,
    pub relative_path: ProjectRelativePath,
}

/// Portable authoritative project semantics.
///
/// Runtime/evaluation records, caches, secrets, credential material, host
/// paths, and workspace UI state are intentionally absent.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ProjectDocument {
    pub schema_version: SchemaVersion,
    pub project_id: ProjectId,
    pub revision: ProjectRevision,
    pub metadata: ProjectMetadata,
    pub required_packages: Vec<PackageRequirement>,
    pub graph: GraphDocument,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<ProjectResourceRef>,
    #[serde(default, skip_serializing_if = "ProjectAssetContext::is_empty")]
    pub asset_context: ProjectAssetContext,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

/// A standalone, trivially shareable node graph using the same graph payload
/// and exact package/definition pins as a project.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NodeGraphDocument {
    pub schema_version: SchemaVersion,
    pub metadata: NodeGraphMetadata,
    pub required_packages: Vec<PackageRequirement>,
    pub graph: GraphDocument,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

impl ProjectDocument {
    /// Creates a portable project around an existing graph.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectValidationError`] when metadata or graph structure is
    /// invalid.
    pub fn new(
        project_id: ProjectId,
        title: impl Into<String>,
        graph: GraphDocument,
    ) -> Result<Self, ProjectValidationError> {
        let document = Self {
            schema_version: SchemaVersion::first(),
            project_id,
            revision: ProjectRevision::initial(),
            metadata: ProjectMetadata {
                title: title.into(),
                description: None,
            },
            required_packages: package_requirements(&graph),
            graph,
            resources: Vec::new(),
            asset_context: ProjectAssetContext::default(),
            extensions: BTreeMap::new(),
        };
        document.validate()?;
        Ok(document)
    }

    /// Validates portable structure without requiring installed node packages.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectValidationError`] for unsupported schema versions,
    /// malformed metadata, inconsistent requirements, or invalid graph links.
    pub fn validate(&self) -> Result<(), ProjectValidationError> {
        validate_document_schema(self.schema_version)?;
        validate_name("project title", &self.metadata.title)?;
        validate_requirements(&self.required_packages, &self.graph)?;
        validate_graph_structure(&self.graph)?;
        validate_resources(&self.resources)?;
        let resource_ids = self.resources.iter().map(|resource| resource.id).collect();
        self.asset_context.validate(&resource_ids)?;
        validate_extension_keys(
            &self.extensions,
            &[
                "schema_version",
                "project_id",
                "revision",
                "metadata",
                "required_packages",
                "graph",
                "resources",
                "asset_context",
            ],
        )
    }

    /// Parses and validates a project document from JSON.
    ///
    /// # Errors
    ///
    /// Returns [`PortableDocumentError`] for invalid JSON or project structure.
    pub fn from_json(json: &str) -> Result<Self, PortableDocumentError> {
        let document: Self = serde_json::from_str(json)?;
        document.validate()?;
        Ok(document)
    }

    /// Produces human-inspectable JSON. Whitespace is not semantically
    /// significant; use [`Self::digest`] for canonical identity.
    ///
    /// # Errors
    ///
    /// Returns [`PortableDocumentError`] if validation or serialization fails.
    pub fn to_pretty_json(&self) -> Result<String, PortableDocumentError> {
        self.validate()?;
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Computes the canonical semantic digest of the project document.
    ///
    /// # Errors
    ///
    /// Returns [`PortableDocumentError`] if validation or serialization fails.
    pub fn digest(&self) -> Result<CanonicalDigest, PortableDocumentError> {
        self.validate()?;
        Ok(canonical_digest(self)?)
    }

    /// Copies the authored node graph into a standalone shareable document.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectValidationError`] when the source graph is invalid.
    pub fn export_node_graph(
        &self,
        name: impl Into<String>,
    ) -> Result<NodeGraphDocument, ProjectValidationError> {
        let document = NodeGraphDocument {
            schema_version: SchemaVersion::first(),
            metadata: NodeGraphMetadata {
                name: name.into(),
                description: None,
            },
            required_packages: self.required_packages.clone(),
            graph: self.graph.clone(),
            extensions: BTreeMap::new(),
        };
        document.validate()?;
        Ok(document)
    }
}

impl NodeGraphDocument {
    /// Creates a standalone shareable node-graph document.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectValidationError`] when metadata or graph structure is
    /// invalid.
    pub fn new(
        name: impl Into<String>,
        graph: GraphDocument,
    ) -> Result<Self, ProjectValidationError> {
        let document = Self {
            schema_version: SchemaVersion::first(),
            metadata: NodeGraphMetadata {
                name: name.into(),
                description: None,
            },
            required_packages: package_requirements(&graph),
            graph,
            extensions: BTreeMap::new(),
        };
        document.validate()?;
        Ok(document)
    }

    /// Validates shareable graph structure without requiring packages to be
    /// installed on the current machine.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectValidationError`] for unsupported schema versions,
    /// malformed metadata, inconsistent requirements, or invalid graph links.
    pub fn validate(&self) -> Result<(), ProjectValidationError> {
        validate_document_schema(self.schema_version)?;
        validate_name("node graph name", &self.metadata.name)?;
        validate_requirements(&self.required_packages, &self.graph)?;
        validate_graph_structure(&self.graph)?;
        validate_extension_keys(
            &self.extensions,
            &["schema_version", "metadata", "required_packages", "graph"],
        )
    }

    /// Parses and validates a standalone node graph from JSON.
    ///
    /// # Errors
    ///
    /// Returns [`PortableDocumentError`] for invalid JSON or graph structure.
    pub fn from_json(json: &str) -> Result<Self, PortableDocumentError> {
        let document: Self = serde_json::from_str(json)?;
        document.validate()?;
        Ok(document)
    }

    /// Produces human-inspectable standalone node-graph JSON.
    ///
    /// # Errors
    ///
    /// Returns [`PortableDocumentError`] if validation or serialization fails.
    pub fn to_pretty_json(&self) -> Result<String, PortableDocumentError> {
        self.validate()?;
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Computes the canonical semantic digest of the standalone node graph.
    ///
    /// # Errors
    ///
    /// Returns [`PortableDocumentError`] if validation or serialization fails.
    pub fn digest(&self) -> Result<CanonicalDigest, PortableDocumentError> {
        self.validate()?;
        Ok(canonical_digest(self)?)
    }
}

fn package_requirements(graph: &GraphDocument) -> Vec<PackageRequirement> {
    graph
        .nodes
        .iter()
        .map(|node| PackageRequirement {
            package_id: node.definition.package_id.clone(),
            package_version: node.definition.package_version.clone(),
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn validate_document_schema(version: SchemaVersion) -> Result<(), ProjectValidationError> {
    if version != SchemaVersion::first() {
        return Err(ProjectValidationError::UnsupportedDocumentSchema(version));
    }
    Ok(())
}

fn validate_name(kind: &'static str, value: &str) -> Result<(), ProjectValidationError> {
    if value.trim().is_empty() {
        return Err(ProjectValidationError::EmptyName { kind });
    }
    Ok(())
}

fn validate_requirements(
    requirements: &[PackageRequirement],
    graph: &GraphDocument,
) -> Result<(), ProjectValidationError> {
    let mut unique = BTreeSet::new();
    for requirement in requirements {
        if !unique.insert(requirement) {
            return Err(ProjectValidationError::DuplicatePackageRequirement(
                requirement.clone(),
            ));
        }
    }
    for node in &graph.nodes {
        let required = PackageRequirement {
            package_id: node.definition.package_id.clone(),
            package_version: node.definition.package_version.clone(),
        };
        if !unique.contains(&required) {
            return Err(ProjectValidationError::MissingPackageRequirement {
                node_id: node.id,
                requirement: required,
            });
        }
    }
    Ok(())
}

fn validate_graph_structure(graph: &GraphDocument) -> Result<(), ProjectValidationError> {
    if graph.schema_version != SchemaVersion::first() {
        return Err(ProjectValidationError::UnsupportedGraphSchema(
            graph.schema_version,
        ));
    }
    validate_extension_keys(
        &graph.extensions,
        &["schema_version", "id", "revision", "nodes", "connections"],
    )?;
    let mut nodes = BTreeSet::new();
    for node in &graph.nodes {
        if !nodes.insert(node.id) {
            return Err(ProjectValidationError::DuplicateNode(node.id));
        }
        validate_extension_keys(
            &node.extensions,
            &["id", "definition", "configuration", "authored_state"],
        )?;
    }
    let mut connections = BTreeSet::new();
    let mut endpoints = BTreeSet::new();
    for connection in &graph.connections {
        if !connections.insert(connection.id) {
            return Err(ProjectValidationError::DuplicateConnection(connection.id));
        }
        if !endpoints.insert((&connection.output, &connection.input)) {
            return Err(ProjectValidationError::DuplicateConnectionEndpoints {
                output: connection.output.clone(),
                input: connection.input.clone(),
            });
        }
        validate_extension_keys(&connection.extensions, &["id", "output", "input"])?;
        for node_id in [connection.output.node_id, connection.input.node_id] {
            if !nodes.contains(&node_id) {
                return Err(ProjectValidationError::ConnectionReferencesMissingNode {
                    connection_id: connection.id,
                    node_id,
                });
            }
        }
    }
    validate_acyclic(graph, &nodes)
}

fn validate_acyclic(
    graph: &GraphDocument,
    nodes: &BTreeSet<NodeInstanceId>,
) -> Result<(), ProjectValidationError> {
    let mut incoming: BTreeMap<_, usize> = nodes.iter().map(|node| (*node, 0)).collect();
    let mut outgoing: BTreeMap<NodeInstanceId, BTreeSet<NodeInstanceId>> = BTreeMap::new();
    for connection in &graph.connections {
        if outgoing
            .entry(connection.output.node_id)
            .or_default()
            .insert(connection.input.node_id)
        {
            *incoming
                .get_mut(&connection.input.node_id)
                .expect("connection nodes were validated") += 1;
        }
    }
    let mut ready: BTreeSet<_> = incoming
        .iter()
        .filter_map(|(node, count)| (*count == 0).then_some(*node))
        .collect();
    let mut visited = 0;
    while let Some(node) = ready.pop_first() {
        visited += 1;
        if let Some(targets) = outgoing.get(&node) {
            for target in targets {
                let count = incoming.get_mut(target).expect("target node exists");
                *count -= 1;
                if *count == 0 {
                    ready.insert(*target);
                }
            }
        }
    }
    if visited != nodes.len() {
        return Err(ProjectValidationError::GraphContainsCycle);
    }
    Ok(())
}

fn validate_resources(resources: &[ProjectResourceRef]) -> Result<(), ProjectValidationError> {
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for resource in resources {
        if !ids.insert(resource.id) {
            return Err(ProjectValidationError::DuplicateResourceId(resource.id));
        }
        if !paths.insert(&resource.relative_path) {
            return Err(ProjectValidationError::DuplicateResourcePath(
                resource.relative_path.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_extension_keys(
    extensions: &BTreeMap<String, Value>,
    reserved: &[&str],
) -> Result<(), ProjectValidationError> {
    const EXCLUDED: &[&str] = &[
        "runtime",
        "evaluation",
        "evaluations",
        "progress",
        "cancellation",
        "availability",
        "materialization",
        "proxy",
        "proxies",
        "thumbnail",
        "preview",
        "cache",
        "caches",
        "credentials",
        "secrets",
        "environment",
        "workspace",
        "panels",
        "window_geometry",
        "gallery_selection",
    ];
    if let Some(key) = extensions
        .keys()
        .find(|key| reserved.contains(&key.as_str()))
    {
        return Err(ProjectValidationError::ReservedExtensionKey(key.clone()));
    }
    if let Some(key) = extensions
        .keys()
        .find(|key| EXCLUDED.contains(&key.as_str()))
    {
        return Err(ProjectValidationError::ExcludedPortableField(key.clone()));
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum PortableDocumentError {
    #[error("invalid portable document JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Validation(#[from] ProjectValidationError),
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ProjectValidationError {
    #[error("unsupported project/node-graph document schema version {0:?}")]
    UnsupportedDocumentSchema(SchemaVersion),
    #[error("unsupported embedded graph schema version {0:?}")]
    UnsupportedGraphSchema(SchemaVersion),
    #[error("{kind} must not be empty")]
    EmptyName { kind: &'static str },
    #[error("duplicate package requirement {0:?}")]
    DuplicatePackageRequirement(PackageRequirement),
    #[error("node {node_id} is missing exact package requirement {requirement:?}")]
    MissingPackageRequirement {
        node_id: NodeInstanceId,
        requirement: PackageRequirement,
    },
    #[error("duplicate node {0}")]
    DuplicateNode(NodeInstanceId),
    #[error("duplicate connection {0}")]
    DuplicateConnection(ConnectionId),
    #[error("connection {connection_id} references missing node {node_id}")]
    ConnectionReferencesMissingNode {
        connection_id: ConnectionId,
        node_id: NodeInstanceId,
    },
    #[error("duplicate connection endpoints {output:?} -> {input:?}")]
    DuplicateConnectionEndpoints {
        output: crate::PortEndpoint,
        input: crate::PortEndpoint,
    },
    #[error("portable graph contains a cycle")]
    GraphContainsCycle,
    #[error("duplicate project resource ID {0}")]
    DuplicateResourceId(ProjectResourceId),
    #[error("duplicate project resource path {0}")]
    DuplicateResourcePath(ProjectRelativePath),
    #[error("invalid project asset context: {0}")]
    InvalidAssetContext(Box<AssetContextError>),
    #[error("extension key {0:?} collides with a defined document field")]
    ReservedExtensionKey(String),
    #[error("portable document must not contain excluded state field {0:?}")]
    ExcludedPortableField(String),
}

impl From<AssetContextError> for ProjectValidationError {
    fn from(error: AssetContextError) -> Self {
        Self::InvalidAssetContext(Box::new(error))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::*;
    use crate::{
        AssetId, AssetRepresentationId, Connection, GraphId, NodeDefinitionId, NodeDefinitionRef,
        NodeDefinitionVersion, NodeInstance, PortEndpoint, PortId, ProjectAsset,
        RepresentationBinding, RepresentationDescriptor, RepresentationFingerprint,
        RepresentationRoleId, SchemaId, SchemaRef, SchemaValue,
    };

    fn schema(id: &str) -> SchemaRef {
        SchemaRef {
            id: SchemaId::parse(id).unwrap(),
            version: SchemaVersion::first(),
        }
    }

    fn node(id: NodeInstanceId, definition: &str, package: &str) -> NodeInstance {
        NodeInstance {
            id,
            definition: NodeDefinitionRef {
                package_id: NodePackageId::parse(package).unwrap(),
                package_version: PackageVersion::new(1, 2, 3),
                definition_id: NodeDefinitionId::parse(definition).unwrap(),
                definition_version: NodeDefinitionVersion::new(4).unwrap(),
            },
            configuration: SchemaValue {
                schema: schema(&format!("{definition}.config")),
                value: json!({"amount": 7, "future-config": {"mode": "new"}}),
            },
            authored_state: Some(SchemaValue {
                schema: schema(&format!("{definition}.state")),
                value: json!({"curve": [0, 0.5, 1], "future-state": true}),
            }),
            extensions: BTreeMap::from([("future-node-field".to_owned(), json!({"x": 1}))]),
        }
    }

    fn project() -> ProjectDocument {
        let first = NodeInstanceId::new();
        let second = NodeInstanceId::new();
        let mut graph = GraphDocument::new(GraphId::new());
        graph.nodes = vec![
            node(first, "example.generate.value", "example.generate"),
            node(second, "example.transform.value", "example.transform"),
        ];
        graph.connections.push(Connection {
            id: ConnectionId::new(),
            output: PortEndpoint {
                node_id: first,
                port_id: PortId::parse("value").unwrap(),
            },
            input: PortEndpoint {
                node_id: second,
                port_id: PortId::parse("input").unwrap(),
            },
            extensions: BTreeMap::from([("future-connection-field".to_owned(), json!(9))]),
        });
        let mut project = ProjectDocument::new(ProjectId::new(), "Portable Test", graph).unwrap();
        let resource_id = ProjectResourceId::new();
        project.resources.push(ProjectResourceRef {
            id: resource_id,
            relative_path: ProjectRelativePath::parse("assets/./session//image.tif").unwrap(),
        });
        project.asset_context.assets.push(ProjectAsset {
            id: AssetId::new(),
            display_name: "Portable Asset".to_owned(),
            representations: vec![RepresentationDescriptor {
                id: AssetRepresentationId::new(),
                role: RepresentationRoleId::parse("example.rendition.original").unwrap(),
                fingerprint: RepresentationFingerprint::sha256([9; 32]),
                capabilities: BTreeSet::new(),
                binding: RepresentationBinding::ProjectResource { resource_id },
                extensions: BTreeMap::from([(
                    "future-representation-field".to_owned(),
                    json!({"kept": true}),
                )]),
            }],
            extensions: BTreeMap::new(),
        });
        project
            .extensions
            .insert("future-project-field".to_owned(), json!({"enabled": true}));
        project
    }

    #[test]
    fn media_agnostic_project_round_trips_without_semantic_loss() {
        let project = project();
        let json = project.to_pretty_json().unwrap();
        let decoded = ProjectDocument::from_json(&json).unwrap();
        assert_eq!(decoded, project);
        assert_eq!(
            decoded.resources[0].relative_path.as_str(),
            "assets/session/image.tif"
        );
        assert_eq!(
            decoded.graph.nodes[0].definition.definition_version.get(),
            4
        );
        assert_eq!(
            decoded.graph.nodes[0].authored_state,
            project.graph.nodes[0].authored_state
        );
    }

    #[test]
    fn canonical_digest_ignores_human_json_whitespace() {
        let project = project();
        let pretty = project.to_pretty_json().unwrap();
        let compact =
            serde_json::to_string(&serde_json::from_str::<Value>(&pretty).unwrap()).unwrap();
        assert_eq!(
            ProjectDocument::from_json(&pretty)
                .unwrap()
                .digest()
                .unwrap(),
            ProjectDocument::from_json(&compact)
                .unwrap()
                .digest()
                .unwrap()
        );
    }

    #[test]
    fn standalone_node_graph_is_separate_and_trivially_round_trips() {
        let project = project();
        let export = project.export_node_graph("Shared Graph").unwrap();
        let json = export.to_pretty_json().unwrap();
        let decoded = NodeGraphDocument::from_json(&json).unwrap();
        assert_eq!(decoded, export);
        assert_eq!(decoded.graph, project.graph);
        assert!(!json.contains("project_id"));
        assert!(!json.contains("resources"));
        assert!(!json.contains("asset_context"));
    }

    #[test]
    fn portable_schema_excludes_runtime_cache_secret_and_workspace_state() {
        let json = serde_json::to_value(project()).unwrap();
        let fields = json.as_object().unwrap();
        assert_eq!(
            fields.keys().map(String::as_str).collect::<BTreeSet<_>>(),
            BTreeSet::from([
                "future-project-field",
                "graph",
                "asset_context",
                "metadata",
                "project_id",
                "required_packages",
                "resources",
                "revision",
                "schema_version",
            ])
        );
        for excluded in [
            "runtime",
            "evaluations",
            "cache",
            "credentials",
            "environment",
            "workspace",
            "panels",
            "window_geometry",
        ] {
            assert!(!fields.contains_key(excluded));
        }
    }

    #[test]
    fn relative_paths_reject_machine_specific_or_escaping_locations() {
        assert!(ProjectRelativePath::parse("/Users/me/image.tif").is_err());
        assert!(ProjectRelativePath::parse("C:\\Users\\me\\image.tif").is_err());
        assert!(ProjectRelativePath::parse("assets/../secret.txt").is_err());
        let windows = ProjectRelativePath::parse("assets\\session\\image.tif").unwrap();
        assert_eq!(windows.as_str(), "assets/session/image.tif");
    }

    #[test]
    fn excluded_state_extensions_and_graph_cycles_are_rejected() {
        let mut with_cache = project();
        with_cache
            .extensions
            .insert("cache".to_owned(), json!({"derived": true}));
        assert!(matches!(
            with_cache.validate(),
            Err(ProjectValidationError::ExcludedPortableField(field)) if field == "cache"
        ));

        let mut cyclic = project();
        let first = cyclic.graph.nodes[0].id;
        let second = cyclic.graph.nodes[1].id;
        cyclic.graph.connections.push(Connection {
            id: ConnectionId::new(),
            output: PortEndpoint {
                node_id: second,
                port_id: PortId::parse("value").unwrap(),
            },
            input: PortEndpoint {
                node_id: first,
                port_id: PortId::parse("input").unwrap(),
            },
            extensions: BTreeMap::new(),
        });
        assert_eq!(
            cyclic.validate(),
            Err(ProjectValidationError::GraphContainsCycle)
        );
    }
}
