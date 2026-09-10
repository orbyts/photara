//! Portable project assignments are owned by the Project Document, not `SQLite`.
use super::{
    Arc, BridgeError, CommandId, PhotaraProject, ProjectCommand, ProjectCommandEnvelope,
    ProjectSessionState, Uuid, apply_project_command,
};
use crate::library::{library_error, library_uuid};
use crate::{BridgeLibraryKind, BridgeLibraryRecordDto, PhotaraLibrary};
use photara_library::{ProjectLibraryAssignment, ProjectLibraryContext, ProjectLibraryReference};

const CONTEXT_KEY: &str = "photara.library.v1";

#[derive(Clone, Debug, uniffi::Record)]
pub struct BridgeProjectAssignmentDto {
    pub assignment_id: String,
    pub record_id: String,
    pub owner_id: String,
    pub record_revision: u64,
    pub display_name: String,
    pub kind: BridgeLibraryKind,
    pub relationship: String,
    pub date: String,
    pub notes: String,
}
#[derive(Clone, Debug, uniffi::Record)]
pub struct BridgeProjectInfoDto {
    pub revision: u64,
    pub assignments: Vec<BridgeProjectAssignmentDto>,
}
fn context(state: &ProjectSessionState) -> Result<ProjectLibraryContext, BridgeError> {
    state
        .project
        .extensions
        .get(CONTEXT_KEY)
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map(Option::unwrap_or_default)
        .map_err(library_error)
}
fn dto(value: &ProjectLibraryContext) -> BridgeProjectInfoDto {
    BridgeProjectInfoDto {
        revision: value.revision,
        assignments: value
            .assignments
            .iter()
            .map(|a| {
                let kind = match a.reference.kind {
                    photara_library::LibraryRecordKind::Person => BridgeLibraryKind::Person,
                    photara_library::LibraryRecordKind::Client => BridgeLibraryKind::Client,
                    photara_library::LibraryRecordKind::Location => BridgeLibraryKind::Location,
                    photara_library::LibraryRecordKind::Scene => BridgeLibraryKind::Scene,
                };
                BridgeProjectAssignmentDto {
                    assignment_id: a.assignment_id.to_string(),
                    record_id: a.reference.record_id.to_string(),
                    owner_id: a.reference.owner_id.clone(),
                    record_revision: a.reference.record_revision_snapshot,
                    display_name: a.reference.display_name_snapshot.clone(),
                    kind,
                    relationship: a.reference.relationship.clone(),
                    date: a.date.clone(),
                    notes: a.notes.clone(),
                }
            })
            .collect(),
    }
}
impl PhotaraProject {
    fn update_library_context(
        &self,
        expected: u64,
        edit: impl FnOnce(&mut ProjectLibraryContext) -> Result<(), BridgeError>,
    ) -> Result<BridgeProjectInfoDto, BridgeError> {
        let mut state = self.lock_state()?;
        let mut value = context(&state)?;
        if value.revision != expected {
            return Err(library_error(
                "Project Info changed. Reload before editing.",
            ));
        }
        edit(&mut value)?;
        value.revision = value
            .revision
            .checked_add(1)
            .ok_or_else(|| library_error("Project Info revision exhausted"))?;
        let before = state.project.extensions.get(CONTEXT_KEY).cloned();
        let result = apply_project_command(
            &state.project,
            &ProjectCommandEnvelope {
                command_id: CommandId::new(),
                project_id: state.project.project_id,
                expected_revision: state.project.revision,
                command: ProjectCommand::SetExtension {
                    key: CONTEXT_KEY.into(),
                    expected_value: before.clone(),
                    value: Some(serde_json::to_value(&value).map_err(library_error)?),
                },
            },
        )
        .map_err(library_error)?;
        state.library_undo.push(before);
        state.project = result.project;
        state.dirty = true;
        Ok(dto(&value))
    }
}
#[uniffi::export]
impl PhotaraProject {
    /// Reads portable project assignments without consulting the Library.
    ///
    /// # Errors
    /// Returns a stale revision, unavailable record, invalid document or session error.
    pub fn library_context(&self) -> Result<BridgeProjectInfoDto, BridgeError> {
        let state = self.lock_state()?;
        Ok(dto(&context(&state)?))
    }
    /// Assigns a live owner-scoped Library record with a historical snapshot.
    ///
    /// # Errors
    /// Returns a stale revision, unavailable record, invalid document or session error.
    #[allow(clippy::needless_pass_by_value)] // UniFFI exports owned arguments.
    pub fn assign_library_record(
        &self,
        library: Arc<PhotaraLibrary>,
        record_id: String,
        expected_revision: u64,
        relationship: String,
    ) -> Result<BridgeProjectInfoDto, BridgeError> {
        let record = library.record(&record_id)?;
        let display = BridgeLibraryRecordDto::from(&record);
        self.update_library_context(expected_revision, |context| {
            let relation = relationship.trim();
            if relation.is_empty() {
                return Err(library_error("Assignment role is required"));
            }
            // Scenes intentionally allow repeated occurrences. Other assignments are idempotent.
            if display.kind != BridgeLibraryKind::Scene
                && context.assignments.iter().any(|a| {
                    a.reference.record_id == record.header().record_id
                        && a.reference.owner_id == display.owner_id
                        && a.reference.relationship == relation
                })
            {
                return Err(library_error("This record already has that project role"));
            }
            context.assignments.push(ProjectLibraryAssignment {
                assignment_id: Uuid::new_v4(),
                reference: ProjectLibraryReference {
                    owner_id: display.owner_id,
                    record_id: record.header().record_id,
                    kind: record.kind(),
                    relationship: relation.into(),
                    display_name_snapshot: display.display_name,
                    record_revision_snapshot: display.revision,
                },
                date: String::new(),
                notes: String::new(),
            });
            Ok(())
        })
    }
    /// Edits or removes one unique project assignment.
    ///
    /// # Errors
    /// Returns a stale revision, unavailable record, invalid document or session error.
    #[allow(clippy::needless_pass_by_value)] // UniFFI exports owned arguments.
    pub fn edit_library_assignment(
        &self,
        assignment_id: String,
        expected_revision: u64,
        date: String,
        notes: String,
        remove: bool,
    ) -> Result<BridgeProjectInfoDto, BridgeError> {
        let id = library_uuid(&assignment_id)?;
        self.update_library_context(expected_revision, |context| {
            let assignment = context
                .assignments
                .iter_mut()
                .find(|a| a.assignment_id == id)
                .ok_or_else(|| library_error("Assignment unavailable"))?;
            assignment.date = date;
            assignment.notes = notes;
            if remove {
                context.assignments.retain(|a| a.assignment_id != id);
            }
            Ok(())
        })
    }
    /// Restores the previous assignment value at a new edit revision.
    ///
    /// # Errors
    /// Returns a stale revision, unavailable record, invalid document or session error.
    pub fn undo_library_assignment(
        &self,
        expected_revision: u64,
    ) -> Result<BridgeProjectInfoDto, BridgeError> {
        let mut state = self.lock_state()?;
        let current = context(&state)?;
        if current.revision != expected_revision {
            return Err(library_error("Project Info changed. Reload before undo."));
        }
        let before = state
            .library_undo
            .last()
            .ok_or_else(|| library_error("No Project Info edit to undo"))?
            .clone();
        let mut restored: ProjectLibraryContext = before
            .map(serde_json::from_value)
            .transpose()
            .map_err(library_error)?
            .unwrap_or_default();
        restored.revision = current
            .revision
            .checked_add(1)
            .ok_or_else(|| library_error("Project Info revision exhausted"))?;
        let result = apply_project_command(
            &state.project,
            &ProjectCommandEnvelope {
                command_id: CommandId::new(),
                project_id: state.project.project_id,
                expected_revision: state.project.revision,
                command: ProjectCommand::SetExtension {
                    key: CONTEXT_KEY.into(),
                    expected_value: state.project.extensions.get(CONTEXT_KEY).cloned(),
                    value: Some(serde_json::to_value(&restored).map_err(library_error)?),
                },
            },
        )
        .map_err(library_error)?;
        state.project = result.project;
        state.library_undo.pop();
        state.dirty = true;
        Ok(dto(&restored))
    }
}
