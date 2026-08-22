use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    AssetId, CommandId, ProjectAsset, ProjectDocument, ProjectId, ProjectResourceRef,
    ProjectRevision,
};

/// Revision-checked semantic command over the portable project aggregate.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ProjectCommandEnvelope {
    pub command_id: CommandId,
    pub project_id: ProjectId,
    pub expected_revision: ProjectRevision,
    pub command: ProjectCommand,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ProjectCommand {
    AddAsset {
        asset: ProjectAsset,
        resources: Vec<ProjectResourceRef>,
    },
    /// Inserts or replaces semantic assets by identity without interpreting
    /// provider-specific node state or runtime locators.
    UpsertAssets { assets: Vec<ProjectAsset> },
    /// Atomically removes a provider's prior membership and publishes its new
    /// semantic assets. Provider ownership remains outside Core.
    ReconcileAssets {
        remove_asset_ids: Vec<AssetId>,
        upsert_assets: Vec<ProjectAsset>,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ProjectCommandResult {
    pub command_id: CommandId,
    pub project: ProjectDocument,
}

/// Applies one semantic command to an immutable project snapshot.
///
/// The durable project revision is not advanced until the application saves
/// the resulting aggregate through compare-and-swap persistence.
///
/// # Errors
///
/// Returns an identity, revision, or portable-project validation error.
pub fn apply_project_command(
    project: &ProjectDocument,
    envelope: &ProjectCommandEnvelope,
) -> Result<ProjectCommandResult, ProjectCommandError> {
    if project.project_id != envelope.project_id {
        return Err(ProjectCommandError::ProjectMismatch {
            expected: project.project_id,
            actual: envelope.project_id,
        });
    }
    if project.revision != envelope.expected_revision {
        return Err(ProjectCommandError::RevisionConflict {
            expected: envelope.expected_revision,
            actual: project.revision,
        });
    }
    let mut updated = project.clone();
    match &envelope.command {
        ProjectCommand::AddAsset { asset, resources } => {
            updated.resources.extend(resources.iter().cloned());
            updated.asset_context.assets.push(asset.clone());
        }
        ProjectCommand::UpsertAssets { assets } => {
            for asset in assets {
                if let Some(existing) = updated
                    .asset_context
                    .assets
                    .iter_mut()
                    .find(|existing| existing.id == asset.id)
                {
                    existing.clone_from(asset);
                } else {
                    updated.asset_context.assets.push(asset.clone());
                }
            }
        }
        ProjectCommand::ReconcileAssets {
            remove_asset_ids,
            upsert_assets,
        } => {
            updated
                .asset_context
                .assets
                .retain(|asset| !remove_asset_ids.contains(&asset.id));
            for asset in upsert_assets {
                if let Some(existing) = updated
                    .asset_context
                    .assets
                    .iter_mut()
                    .find(|existing| existing.id == asset.id)
                {
                    existing.clone_from(asset);
                } else {
                    updated.asset_context.assets.push(asset.clone());
                }
            }
        }
    }
    updated
        .validate()
        .map_err(|error| ProjectCommandError::InvalidProject {
            message: error.to_string(),
        })?;
    Ok(ProjectCommandResult {
        command_id: envelope.command_id,
        project: updated,
    })
}

#[derive(Clone, Debug, Deserialize, Eq, Error, PartialEq, Serialize)]
#[serde(tag = "code", content = "details", rename_all = "kebab-case")]
pub enum ProjectCommandError {
    #[error("command targets project {actual}, but snapshot is project {expected}")]
    ProjectMismatch {
        expected: ProjectId,
        actual: ProjectId,
    },
    #[error("project revision conflict: expected {expected:?}, actual {actual:?}")]
    RevisionConflict {
        expected: ProjectRevision,
        actual: ProjectRevision,
    },
    #[error("project command would produce an invalid document: {message}")]
    InvalidProject { message: String },
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::{
        AssetId, GraphDocument, GraphId, ProjectAssetContext, ProjectRelativePath,
        ProjectResourceId,
    };

    use super::*;

    #[test]
    fn asset_addition_is_validated_and_does_not_mutate_the_input() {
        let project = ProjectDocument::new(
            ProjectId::new(),
            "Project command",
            GraphDocument::new(GraphId::new()),
        )
        .unwrap();
        let asset = ProjectAsset {
            id: AssetId::new(),
            display_name: "Photograph".to_owned(),
            representations: Vec::new(),
            extensions: BTreeMap::new(),
        };
        let result = apply_project_command(
            &project,
            &ProjectCommandEnvelope {
                command_id: CommandId::new(),
                project_id: project.project_id,
                expected_revision: project.revision,
                command: ProjectCommand::AddAsset {
                    asset: asset.clone(),
                    resources: vec![ProjectResourceRef {
                        id: ProjectResourceId::new(),
                        relative_path: ProjectRelativePath::parse("representations/photo.tiff")
                            .unwrap(),
                    }],
                },
            },
        )
        .unwrap();
        assert_eq!(project.asset_context, ProjectAssetContext::default());
        assert_eq!(result.project.asset_context.assets, vec![asset]);
        assert_eq!(result.project.revision, project.revision);
    }

    #[test]
    fn source_reconciliation_removes_only_prior_membership() {
        let mut project = ProjectDocument::new(
            ProjectId::new(),
            "Reconciliation",
            GraphDocument::new(GraphId::new()),
        )
        .unwrap();
        let removed = ProjectAsset {
            id: AssetId::new(),
            display_name: "Old provider asset".to_owned(),
            representations: Vec::new(),
            extensions: BTreeMap::new(),
        };
        let retained = ProjectAsset {
            id: AssetId::new(),
            display_name: "Unrelated imported asset".to_owned(),
            representations: Vec::new(),
            extensions: BTreeMap::new(),
        };
        let replacement = ProjectAsset {
            id: AssetId::new(),
            display_name: "New provider asset".to_owned(),
            representations: Vec::new(),
            extensions: BTreeMap::new(),
        };
        project.asset_context.assets = vec![removed.clone(), retained.clone()];

        let result = apply_project_command(
            &project,
            &ProjectCommandEnvelope {
                command_id: CommandId::new(),
                project_id: project.project_id,
                expected_revision: project.revision,
                command: ProjectCommand::ReconcileAssets {
                    remove_asset_ids: vec![removed.id],
                    upsert_assets: vec![replacement.clone()],
                },
            },
        )
        .unwrap();

        assert_eq!(
            result.project.asset_context.assets,
            vec![retained, replacement]
        );
    }
}
