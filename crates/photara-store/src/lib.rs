//! Backend-neutral persistence contracts and the minimum durable adapter.

mod local_assets;

pub use local_assets::{
    LocalAssetAdapterError, LocalProjectAssetAdapter, import_local_tiff_pair,
    refresh_local_representation_fingerprint,
};

use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use photara_core::{
    PackageRequirement, PortableDocumentError, ProjectDocument, ProjectId, ProjectRevision,
    ProjectValidationError,
};
use photara_node_sdk::{NodePackageManifest, NodePackageManifestError};
use thiserror::Error;

/// Authoritative repository for whole portable project aggregates.
///
/// Each create or replace call is the transactional unit for Stage 4A. The
/// embedded graph is never persisted as a second authoritative representation.
pub trait ProjectRepository {
    /// Creates a project that does not already exist.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::ProjectAlreadyExists`] when the project ID exists,
    /// or a validation/backend error when the write cannot complete.
    fn create_project(&mut self, project: ProjectDocument) -> Result<(), StoreError>;

    /// Loads a project by semantic ID, returning `None` when absent.
    ///
    /// # Errors
    ///
    /// Returns a validation/backend error when the read cannot complete.
    fn load_project(&self, id: ProjectId) -> Result<Option<ProjectDocument>, StoreError>;

    /// Atomically replaces a project after checking its expected revision.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::ProjectNotFound`],
    /// [`StoreError::ProjectRevisionConflict`], or a validation/backend error.
    fn replace_project(
        &mut self,
        project: ProjectDocument,
        expected_revision: ProjectRevision,
    ) -> Result<(), StoreError>;
}

/// Persistence for exact package-release registration metadata.
///
/// Registration is append-only in Stage 4A. Update, rollback, disable, and
/// uninstall semantics belong to Stage 4B.
pub trait PackageManifestRepository {
    /// Persists one exact validated package manifest.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::PackageAlreadyRegistered`] when that exact release
    /// already exists, or a validation/backend error.
    fn register_manifest(&mut self, manifest: NodePackageManifest) -> Result<(), StoreError>;

    /// Loads one exact package manifest, returning `None` when absent.
    ///
    /// # Errors
    ///
    /// Returns a validation/backend error when the read cannot complete.
    fn load_manifest(
        &self,
        requirement: &PackageRequirement,
    ) -> Result<Option<NodePackageManifest>, StoreError>;
}

/// Minimum non-durable adapter for tests and short-lived application services.
#[derive(Default)]
pub struct InMemoryStateStore {
    projects: BTreeMap<ProjectId, ProjectDocument>,
    manifests: BTreeMap<PackageRequirement, NodePackageManifest>,
}

impl ProjectRepository for InMemoryStateStore {
    fn create_project(&mut self, project: ProjectDocument) -> Result<(), StoreError> {
        project.validate()?;
        if self.projects.contains_key(&project.project_id) {
            return Err(StoreError::ProjectAlreadyExists(project.project_id));
        }
        self.projects.insert(project.project_id, project);
        Ok(())
    }

    fn load_project(&self, id: ProjectId) -> Result<Option<ProjectDocument>, StoreError> {
        Ok(self.projects.get(&id).cloned())
    }

    fn replace_project(
        &mut self,
        project: ProjectDocument,
        expected_revision: ProjectRevision,
    ) -> Result<(), StoreError> {
        validate_project_replacement(
            self.projects.get(&project.project_id),
            &project,
            expected_revision,
        )?;
        self.projects.insert(project.project_id, project);
        Ok(())
    }
}

impl PackageManifestRepository for InMemoryStateStore {
    fn register_manifest(&mut self, manifest: NodePackageManifest) -> Result<(), StoreError> {
        manifest.validate()?;
        let requirement = manifest.requirement();
        if self.manifests.contains_key(&requirement) {
            return Err(StoreError::PackageAlreadyRegistered(requirement));
        }
        self.manifests.insert(requirement, manifest);
        Ok(())
    }

    fn load_manifest(
        &self,
        requirement: &PackageRequirement,
    ) -> Result<Option<NodePackageManifest>, StoreError> {
        Ok(self.manifests.get(requirement).cloned())
    }
}

/// Directory-backed store for portable projects and exact package manifests.
///
/// It has no database, network service, or legacy-schema dependency. Project
/// JSON remains the only authoritative graph representation.
#[derive(Clone, Debug)]
pub struct FileSystemStateStore {
    root: PathBuf,
}

impl FileSystemStateStore {
    /// Opens or creates a clean Stage 4A store rooted at `path`.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Io`] when the store directories cannot be created.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, StoreError> {
        let store = Self { root: path.into() };
        create_dir_all(&store.projects_dir())?;
        create_dir_all(&store.packages_dir())?;
        Ok(store)
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn projects_dir(&self) -> PathBuf {
        self.root.join("projects")
    }

    fn packages_dir(&self) -> PathBuf {
        self.root.join("packages")
    }

    fn project_path(&self, id: ProjectId) -> PathBuf {
        self.projects_dir()
            .join(format!("{id}.photara-project.json"))
    }

    fn manifest_path(&self, requirement: &PackageRequirement) -> PathBuf {
        self.packages_dir()
            .join(requirement.package_id.as_str())
            .join(format!(
                "{}.photara-package.json",
                requirement.package_version
            ))
    }
}

impl ProjectRepository for FileSystemStateStore {
    fn create_project(&mut self, project: ProjectDocument) -> Result<(), StoreError> {
        project.validate()?;
        let path = self.project_path(project.project_id);
        let json = project.to_pretty_json()?;
        match atomic_create(&path, json.as_bytes())? {
            CreateOutcome::Created => Ok(()),
            CreateOutcome::AlreadyExists => {
                Err(StoreError::ProjectAlreadyExists(project.project_id))
            }
        }
    }

    fn load_project(&self, id: ProjectId) -> Result<Option<ProjectDocument>, StoreError> {
        let path = self.project_path(id);
        let Some(json) = read_optional(&path)? else {
            return Ok(None);
        };
        ProjectDocument::from_json(&json)
            .map(Some)
            .map_err(|source| StoreError::InvalidProjectFile {
                path,
                source: Box::new(source),
            })
    }

    fn replace_project(
        &mut self,
        project: ProjectDocument,
        expected_revision: ProjectRevision,
    ) -> Result<(), StoreError> {
        project.validate()?;
        let path = self.project_path(project.project_id);
        let _lock = FileLock::acquire(&path, project.project_id)?;
        let current = self.load_project(project.project_id)?;
        validate_project_replacement(current.as_ref(), &project, expected_revision)?;
        let json = project.to_pretty_json()?;
        atomic_replace(&path, json.as_bytes())
    }
}

impl PackageManifestRepository for FileSystemStateStore {
    fn register_manifest(&mut self, manifest: NodePackageManifest) -> Result<(), StoreError> {
        manifest.validate()?;
        let requirement = manifest.requirement();
        let path = self.manifest_path(&requirement);
        let json = serde_json::to_string_pretty(&manifest).map_err(|source| {
            StoreError::InvalidPackageManifestJson {
                path: path.clone(),
                source,
            }
        })?;
        match atomic_create(&path, json.as_bytes())? {
            CreateOutcome::Created => Ok(()),
            CreateOutcome::AlreadyExists => Err(StoreError::PackageAlreadyRegistered(requirement)),
        }
    }

    fn load_manifest(
        &self,
        requirement: &PackageRequirement,
    ) -> Result<Option<NodePackageManifest>, StoreError> {
        let path = self.manifest_path(requirement);
        let Some(json) = read_optional(&path)? else {
            return Ok(None);
        };
        let manifest: NodePackageManifest = serde_json::from_str(&json).map_err(|source| {
            StoreError::InvalidPackageManifestJson {
                path: path.clone(),
                source,
            }
        })?;
        manifest
            .validate()
            .map_err(|source| StoreError::InvalidPackageManifest {
                source: Box::new(source),
            })?;
        if manifest.requirement() != *requirement {
            return Err(StoreError::PackageManifestIdentityMismatch {
                expected: Box::new(requirement.clone()),
                actual: Box::new(manifest.requirement()),
            });
        }
        Ok(Some(manifest))
    }
}

fn validate_project_replacement(
    current: Option<&ProjectDocument>,
    replacement: &ProjectDocument,
    expected_revision: ProjectRevision,
) -> Result<(), StoreError> {
    replacement.validate()?;
    let current = current.ok_or(StoreError::ProjectNotFound(replacement.project_id))?;
    if current.revision != expected_revision {
        return Err(StoreError::ProjectRevisionConflict {
            expected: expected_revision,
            actual: current.revision,
        });
    }
    let required_revision = expected_revision
        .checked_next()
        .ok_or(StoreError::ProjectRevisionExhausted)?;
    if replacement.revision != required_revision {
        return Err(StoreError::InvalidProjectReplacementRevision {
            expected: required_revision,
            actual: replacement.revision,
        });
    }
    Ok(())
}

fn create_dir_all(path: &Path) -> Result<(), StoreError> {
    fs::create_dir_all(path).map_err(|source| StoreError::Io {
        operation: "create directory",
        path: path.to_path_buf(),
        source,
    })
}

fn read_optional(path: &Path) -> Result<Option<String>, StoreError> {
    match fs::read_to_string(path) {
        Ok(value) => Ok(Some(value)),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(StoreError::Io {
            operation: "read file",
            path: path.to_path_buf(),
            source,
        }),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CreateOutcome {
    Created,
    AlreadyExists,
}

fn atomic_create(path: &Path, bytes: &[u8]) -> Result<CreateOutcome, StoreError> {
    let parent = path
        .parent()
        .expect("store paths always have a parent directory");
    create_dir_all(parent)?;
    let temporary = temporary_path(path);
    write_synced(&temporary, bytes)?;
    let result = match fs::hard_link(&temporary, path) {
        Ok(()) => Ok(CreateOutcome::Created),
        Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
            Ok(CreateOutcome::AlreadyExists)
        }
        Err(source) => Err(StoreError::Io {
            operation: "publish new file",
            path: path.to_path_buf(),
            source,
        }),
    };
    remove_temporary(&temporary, result)
}

fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let temporary = temporary_path(path);
    write_synced(&temporary, bytes)?;
    match fs::rename(&temporary, path) {
        Ok(()) => Ok(()),
        Err(source) => remove_temporary(
            &temporary,
            Err(StoreError::Io {
                operation: "replace file",
                path: path.to_path_buf(),
                source,
            }),
        ),
    }
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|source| StoreError::Io {
            operation: "create temporary file",
            path: path.to_path_buf(),
            source,
        })?;
    file.write_all(bytes).map_err(|source| StoreError::Io {
        operation: "write temporary file",
        path: path.to_path_buf(),
        source,
    })?;
    file.sync_all().map_err(|source| StoreError::Io {
        operation: "sync temporary file",
        path: path.to_path_buf(),
        source,
    })
}

fn remove_temporary<T>(path: &Path, result: Result<T, StoreError>) -> Result<T, StoreError> {
    match fs::remove_file(path) {
        Ok(()) => result,
        Err(source) => match result {
            Ok(_) => Err(StoreError::Io {
                operation: "remove temporary file",
                path: path.to_path_buf(),
                source,
            }),
            Err(error) => Err(error),
        },
    }
}

fn temporary_path(path: &Path) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let name = path
        .file_name()
        .expect("store path always has a file name")
        .to_string_lossy();
    path.with_file_name(format!(".{name}.{}.{}.tmp", std::process::id(), nonce))
}

struct FileLock {
    path: PathBuf,
    _file: File,
}

impl FileLock {
    fn acquire(target: &Path, project_id: ProjectId) -> Result<Self, StoreError> {
        let path = sibling_with_suffix(target, ".lock");
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(|source| {
                if source.kind() == io::ErrorKind::AlreadyExists {
                    StoreError::ProjectWriteLocked(project_id)
                } else {
                    StoreError::Io {
                        operation: "acquire project write lock",
                        path: path.clone(),
                        source,
                    }
                }
            })?;
        Ok(Self { path, _file: file })
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn sibling_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let name = path
        .file_name()
        .expect("store path always has a file name")
        .to_string_lossy();
    path.with_file_name(format!("{name}{suffix}"))
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("project {0} already exists")]
    ProjectAlreadyExists(ProjectId),
    #[error("project {0} does not exist")]
    ProjectNotFound(ProjectId),
    #[error("project revision conflict: expected {expected:?}, actual {actual:?}")]
    ProjectRevisionConflict {
        expected: ProjectRevision,
        actual: ProjectRevision,
    },
    #[error("project revision space is exhausted")]
    ProjectRevisionExhausted,
    #[error("replacement project revision must be {expected:?}, got {actual:?}")]
    InvalidProjectReplacementRevision {
        expected: ProjectRevision,
        actual: ProjectRevision,
    },
    #[error("project {0} is already being written")]
    ProjectWriteLocked(ProjectId),
    #[error("package release {0:?} is already registered")]
    PackageAlreadyRegistered(PackageRequirement),
    #[error("persisted package identity is {actual:?}, expected {expected:?}")]
    PackageManifestIdentityMismatch {
        expected: Box<PackageRequirement>,
        actual: Box<PackageRequirement>,
    },
    #[error("invalid project: {0}")]
    InvalidProject(Box<ProjectValidationError>),
    #[error("invalid project file {path}: {source}")]
    InvalidProjectFile {
        path: PathBuf,
        source: Box<PortableDocumentError>,
    },
    #[error("project document could not be serialized: {source}")]
    ProjectSerialization { source: Box<PortableDocumentError> },
    #[error("invalid package manifest: {source}")]
    InvalidPackageManifest {
        source: Box<NodePackageManifestError>,
    },
    #[error("invalid package manifest JSON {path}: {source}")]
    InvalidPackageManifestJson {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("could not {operation} at {path}: {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        source: io::Error,
    },
}

impl From<NodePackageManifestError> for StoreError {
    fn from(source: NodePackageManifestError) -> Self {
        Self::InvalidPackageManifest {
            source: Box::new(source),
        }
    }
}

impl From<ProjectValidationError> for StoreError {
    fn from(source: ProjectValidationError) -> Self {
        Self::InvalidProject(Box::new(source))
    }
}

impl From<PortableDocumentError> for StoreError {
    fn from(source: PortableDocumentError) -> Self {
        Self::ProjectSerialization {
            source: Box::new(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use photara_core::{GraphDocument, GraphId, ProjectId};

    use super::*;

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!("photara-stage-4a-store-{}", ProjectId::new())))
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn brand_new_store_saves_reopens_and_revision_checks_a_portable_project() {
        let root = TestRoot::new();
        assert!(!root.0.exists());
        let id = ProjectId::new();
        let graph = GraphDocument::new(GraphId::new());
        let mut project = ProjectDocument::new(id, "Clean Project", graph).unwrap();
        project.extensions.insert(
            "future-project-field".to_owned(),
            serde_json::json!({"kept": true}),
        );

        let mut store = FileSystemStateStore::open(&root.0).unwrap();
        store.create_project(project.clone()).unwrap();
        drop(store);

        let mut reopened_store = FileSystemStateStore::open(&root.0).unwrap();
        assert_eq!(
            reopened_store.load_project(id).unwrap(),
            Some(project.clone())
        );

        let stale = project.clone();
        project.revision = project.revision.checked_next().unwrap();
        project.metadata.description = Some("persisted replacement".to_owned());
        reopened_store
            .replace_project(project.clone(), ProjectRevision::initial())
            .unwrap();
        assert_eq!(reopened_store.load_project(id).unwrap(), Some(project));
        assert!(matches!(
            reopened_store.replace_project(stale, ProjectRevision::initial()),
            Err(StoreError::ProjectRevisionConflict { .. })
        ));

        assert!(root.0.join("projects").is_dir());
        assert!(root.0.join("packages").is_dir());
    }
}
