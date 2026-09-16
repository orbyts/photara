//! Reviewed record mappings; never convert the complete package to `ProjectDocument`.
use super::{
    Arc, AuthoredCommand, BTreeMap, DefinitionResolver, GraphId, JsonLimits, MutationRequest,
    PackageError, PlanError, Value, ValueTypeRegistry, VerifiedClosure, decode, json, next,
    put_object,
};
use photara_core::{GraphCommand, GraphDocument, apply_graph_command};

pub(super) fn apply<R: DefinitionResolver>(
    base: &VerifiedClosure,
    request: &MutationRequest,
    authored: &mut Value,
    files: &mut BTreeMap<String, Arc<[u8]>>,
    definitions: &R,
    value_types: &ValueTypeRegistry,
) -> Result<bool, PlanError> {
    match &request.command {
        AuthoredCommand::ProjectMetadata { title, description } => {
            let normalized =
                photara_core::creation::project_name(title).map_err(|_| PlanError::Validation)?;
            if &normalized != title {
                return Err(PlanError::Validation);
            }
            if authored["title"] == *title && authored["description"] == *description {
                return Ok(false);
            }
            authored["title"] = json!(title);
            authored["description"] = json!(description);
            Ok(true)
        }
        AuthoredCommand::RenameGraph { graph_id, name } => edit_graph(
            base,
            *graph_id,
            authored,
            files,
            &request.updated_at,
            |graph| {
                if graph["name"] == *name {
                    return Ok(false);
                }
                graph["name"] = json!(name);
                graph["metadata_revision"] = json!(next(decode(&graph["metadata_revision"])?)?);
                Ok(true)
            },
        ),
        AuthoredCommand::Graph { envelope } => {
            if envelope.command_id.to_string() != request.operation_id.to_string() {
                return Err(PlanError::Validation);
            }
            supported(&envelope.command)?;
            edit_graph(
                base,
                envelope.graph_id,
                authored,
                files,
                &request.updated_at,
                |graph| {
                    let original: GraphDocument = decode(&graph["graph"])?;
                    // Unknown nested core fields can be dropped by typed decoding.
                    // Refuse that mapping instead of silently normalizing them away.
                    if photara_core::canonical_json(&original).map_err(|_| PlanError::Validation)?
                        != photara_core::canonical_json(&graph["graph"])
                            .map_err(|_| PlanError::Validation)?
                    {
                        return Err(PlanError::UnsupportedCommand);
                    }
                    let result = apply_graph_command(&original, envelope, definitions, value_types)
                        .map_err(|error| match error {
                            photara_core::GraphCommandError::RevisionConflict { .. } => {
                                PlanError::RevisionConflict
                            }
                            photara_core::GraphCommandError::RevisionExhausted => {
                                PlanError::RevisionExhausted
                            }
                            _ => PlanError::Validation,
                        })?;
                    let mut same_revision = result.graph.clone();
                    same_revision.revision = original.revision;
                    if same_revision == original {
                        return Ok(false);
                    }
                    graph["graph"] =
                        serde_json::to_value(result.graph).map_err(|_| PlanError::Validation)?;
                    Ok(true)
                },
            )
        }
    }
}
fn supported(command: &GraphCommand) -> Result<(), PlanError> {
    match command {
        GraphCommand::AddNode { .. } => Err(PlanError::UnsupportedCommand),
        GraphCommand::Batch { commands } => commands.iter().try_for_each(supported),
        _ => Ok(()),
    }
}
fn edit_graph(
    base: &VerifiedClosure,
    id: GraphId,
    authored: &mut Value,
    files: &mut BTreeMap<String, Arc<[u8]>>,
    updated_at: &str,
    edit: impl FnOnce(&mut Value) -> Result<bool, PlanError>,
) -> Result<bool, PlanError> {
    let graph = base
        .verified
        .graphs
        .iter()
        .find(|g| g.value["graph_id"] == json!(id))
        .ok_or(PlanError::Validation)?;
    let mut value = graph.value.clone();
    if !edit(&mut value)? {
        return Ok(false);
    }
    value["updated_at"] = json!(updated_at);
    let reference = put_object(files, &value, base.files.limits())?;
    let graphs = authored["graphs"]
        .as_array_mut()
        .ok_or(PlanError::Validation)?;
    let entry = graphs
        .iter_mut()
        .find(|g| g["graph_id"] == json!(id))
        .ok_or(PlanError::Validation)?;
    entry["document"] = json!(reference);
    Ok(true)
}

/// Bound recursive typed input before canonical encoding or Core's recursive Batch.
/// This is admission, not parsing an unchecked wire protocol.
pub(super) fn admit(request: &MutationRequest, limits: JsonLimits) -> Result<(), PlanError> {
    if let AuthoredCommand::Graph { envelope } = &request.command {
        let mut pending = vec![(&envelope.command, 0usize)];
        let mut count = 0usize;
        while let Some((command, depth)) = pending.pop() {
            count = count.checked_add(1).ok_or(PackageError::Limit)?;
            if depth > limits.max_depth.saturating_sub(8) || count > limits.max_array_elements {
                return Err(PackageError::Limit.into());
            }
            match command {
                GraphCommand::Batch { commands } => {
                    if commands.len() > limits.max_array_elements {
                        return Err(PackageError::Limit.into());
                    }
                    pending.extend(commands.iter().map(|c| (c, depth + 1)));
                }
                GraphCommand::SetConfiguration { configuration, .. } => {
                    bounded_value(&configuration.value, limits, depth + 8)?;
                }
                GraphCommand::SetAuthoredState {
                    authored_state: Some(state),
                    ..
                } => bounded_value(&state.value, limits, depth + 8)?,
                GraphCommand::AddNode { .. } => return Err(PlanError::UnsupportedCommand),
                _ => {}
            }
        }
    }
    serde_json::to_writer(
        Budget {
            remaining: limits.max_bytes,
        },
        request,
    )
    .map_err(|_| PackageError::Limit)?;
    Ok(())
}
fn bounded_value(value: &Value, limits: JsonLimits, depth: usize) -> Result<(), PlanError> {
    let mut pending = vec![(value, depth)];
    while let Some((value, depth)) = pending.pop() {
        if depth > limits.max_depth {
            return Err(PackageError::Limit.into());
        }
        match value {
            Value::Array(values) => {
                if values.len() > limits.max_array_elements {
                    return Err(PackageError::Limit.into());
                }
                pending.extend(values.iter().map(|v| (v, depth + 1)));
            }
            Value::Object(values) => {
                if values.len() > limits.max_members {
                    return Err(PackageError::Limit.into());
                }
                pending.extend(values.values().map(|v| (v, depth + 1)));
            }
            _ => {}
        }
    }
    Ok(())
}

struct Budget {
    remaining: usize,
}
impl std::io::Write for Budget {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.remaining = self
            .remaining
            .checked_sub(bytes.len())
            .ok_or_else(|| std::io::Error::other("request budget"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Reader compatibility alone is not permission to edit unknown node schemas.
/// Require every current node value schema to match its validated manifest pin.
pub(super) fn check_schemas(package: &super::v1_1::ValidatedPackageV1_1) -> Result<(), PlanError> {
    for graph in &package.graphs {
        let nodes = graph.value["graph"]["nodes"]
            .as_array()
            .ok_or(PlanError::Validation)?;
        let contracts = graph.value["node_contracts"]
            .as_array()
            .ok_or(PlanError::Validation)?;
        for node in nodes {
            let contract = contracts
                .iter()
                .find(|c| c["node_id"] == node["id"])
                .ok_or(PlanError::UnsupportedCommand)?;
            let reference: super::ObjectRef =
                decode(&contract["manifest"]).map_err(|_| PlanError::UnsupportedCommand)?;
            let manifest = &package
                .objects
                .get(&reference.sha256)
                .ok_or(PlanError::UnsupportedCommand)?
                .value["manifest"];
            let definitions = manifest["definitions"]
                .as_array()
                .ok_or(PlanError::UnsupportedCommand)?;
            let definition = definitions
                .iter()
                .find(|d| {
                    d["coordinate"]["definition_id"] == node["definition"]["definition_id"]
                        && d["coordinate"]["definition_version"]
                            == node["definition"]["definition_version"]
                })
                .ok_or(PlanError::UnsupportedCommand)?;
            if node["configuration"]["schema"] != definition["configuration_schema"]["schema"] {
                return Err(PlanError::UnsupportedCommand);
            }
            if let Some(state) = node.get("authored_state").filter(|v| !v.is_null())
                && (definition["state_schema"].is_null()
                    || state["schema"] != definition["state_schema"]["schema"])
            {
                return Err(PlanError::UnsupportedCommand);
            }
        }
    }
    Ok(())
}
