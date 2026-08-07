use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, Row};
use storexa::Database;
use uuid::Uuid;

use crate::{PhotaraError, Result, config::PhotaraConfig, config::validate_slug};

#[derive(Clone, Debug)]
pub struct NewProject {
    pub slug: String,
    pub display_name: String,
    pub scene: String,
    pub location: String,
    pub people: Vec<String>,
    pub origin: ProjectOrigin,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectOrigin {
    Native,
    Proetus,
    Adopted,
}

impl ProjectOrigin {
    fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Proetus => "proetus",
            Self::Adopted => "adopted",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProjectManifest {
    pub schema_version: u32,
    pub slug: String,
    pub display_name: String,
    pub scene: String,
    pub location: String,
    pub people: Vec<String>,
    pub origin: String,
    pub status: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectRecord {
    pub id: Uuid,
    pub slug: String,
    pub display_name: String,
    pub scene: String,
    pub location: String,
    pub people: Vec<String>,
    pub origin: String,
    pub status: String,
}

pub async fn initialize(
    database: &Database,
    config: &PhotaraConfig,
    project: NewProject,
) -> Result<ProjectRecord> {
    let manifest = validate_and_manifest(config, project)?;
    let mut transaction = database.begin().await?;
    let record = insert_or_verify(&mut transaction, config, &manifest).await?;
    materialize_directory(&config.settings.projects_root, &manifest, false)?;
    record_event(
        &mut transaction,
        record.id,
        "project.initialized",
        format!("project:{}:initialized", record.id),
        serde_json::to_value(&manifest)?,
    )
    .await?;
    transaction.commit().await?;
    Ok(record)
}

pub async fn reconfigure(
    database: &Database,
    config: &PhotaraConfig,
    project: NewProject,
) -> Result<ProjectRecord> {
    let manifest = validate_and_manifest(config, project)?;
    let mut transaction = database.begin().await?;
    let existing = find_on(&mut transaction, &manifest.slug)
        .await?
        .ok_or_else(|| {
            PhotaraError::Configuration(format!("project {:?} was not found", manifest.slug))
        })?;
    update(&mut transaction, config, existing.id, &manifest).await?;
    materialize_directory(&config.settings.projects_root, &manifest, true)?;
    let event_key = format!(
        "project:{}:configured:{}",
        existing.id,
        serde_json::to_string(&manifest)?
    );
    record_event(
        &mut transaction,
        existing.id,
        "project.configured",
        event_key,
        serde_json::to_value(&manifest)?,
    )
    .await?;
    transaction.commit().await?;
    find(database, &manifest.slug)
        .await?
        .ok_or_else(|| PhotaraError::Configuration("project update did not persist".into()))
}

pub async fn find(database: &Database, slug: &str) -> Result<Option<ProjectRecord>> {
    let mut connection = database.acquire().await?;
    find_on(&mut connection, slug).await
}

async fn find_on(connection: &mut PgConnection, slug: &str) -> Result<Option<ProjectRecord>> {
    let row = sqlx::query(
        "SELECT id, slug, display_name, scene_slug, location_slug, origin, status \
         FROM projects WHERE slug = $1",
    )
    .bind(slug)
    .fetch_optional(&mut *connection)
    .await?;

    match row {
        Some(row) => {
            let id: Uuid = row.try_get("id")?;
            let people = people(connection, id).await?;
            Ok(Some(ProjectRecord {
                id,
                slug: row.try_get("slug")?,
                display_name: row.try_get("display_name")?,
                scene: row.try_get("scene_slug")?,
                location: row.try_get("location_slug")?,
                origin: row.try_get("origin")?,
                status: row.try_get("status")?,
                people,
            }))
        }
        None => Ok(None),
    }
}

fn validate_and_manifest(config: &PhotaraConfig, project: NewProject) -> Result<ProjectManifest> {
    validate_slug(&project.slug).map_err(|message| {
        PhotaraError::Configuration(format!("invalid project slug: {message}"))
    })?;
    if project.display_name.trim().is_empty() {
        return Err(PhotaraError::Configuration(
            "project display name must not be empty".into(),
        ));
    }
    if !config.scenes.contains_key(&project.scene) {
        return Err(PhotaraError::Configuration(format!(
            "scene {:?} does not exist in scenes.yml",
            project.scene
        )));
    }
    if !config.locations.contains_key(&project.location) {
        return Err(PhotaraError::Configuration(format!(
            "location {:?} does not exist in locations.yml",
            project.location
        )));
    }
    let unique_people: BTreeSet<_> = project.people.into_iter().collect();
    for person in &unique_people {
        if !config.people.contains_key(person) {
            return Err(PhotaraError::Configuration(format!(
                "person {person:?} does not exist in people.yml"
            )));
        }
    }

    Ok(ProjectManifest {
        schema_version: 1,
        slug: project.slug,
        display_name: project.display_name,
        scene: project.scene,
        location: project.location,
        people: unique_people.into_iter().collect(),
        origin: project.origin.as_str().into(),
        status: "active".into(),
    })
}

async fn insert_or_verify(
    connection: &mut PgConnection,
    config: &PhotaraConfig,
    manifest: &ProjectManifest,
) -> Result<ProjectRecord> {
    let scene = serde_json::to_value(&config.scenes[&manifest.scene])?;
    let location = serde_json::to_value(&config.locations[&manifest.location])?;
    let id = Uuid::new_v4();

    let inserted = sqlx::query(
        "INSERT INTO projects \
         (id, slug, display_name, scene_slug, scene_snapshot, location_slug, \
          location_snapshot, origin, status) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         ON CONFLICT (slug) DO NOTHING",
    )
    .bind(id)
    .bind(&manifest.slug)
    .bind(&manifest.display_name)
    .bind(&manifest.scene)
    .bind(scene)
    .bind(&manifest.location)
    .bind(location)
    .bind(&manifest.origin)
    .bind(&manifest.status)
    .execute(&mut *connection)
    .await?
    .rows_affected()
        == 1;

    let record = find_on(connection, &manifest.slug)
        .await?
        .ok_or_else(|| PhotaraError::Configuration("project insert did not persist".into()))?;
    if !inserted {
        ensure_project_matches(&record, manifest)?;
        return Ok(record);
    }
    ensure_project_core_matches(&record, manifest)?;

    for person in &manifest.people {
        let snapshot = serde_json::to_value(&config.people[person])?;
        sqlx::query(
            "INSERT INTO project_people (project_id, person_slug, person_snapshot) \
             VALUES ($1, $2, $3) ON CONFLICT (project_id, person_slug) DO NOTHING",
        )
        .bind(record.id)
        .bind(person)
        .bind(snapshot)
        .execute(&mut *connection)
        .await?;
    }

    let record = find_on(connection, &manifest.slug).await?.unwrap();
    ensure_project_matches(&record, manifest)?;
    Ok(record)
}

async fn update(
    connection: &mut PgConnection,
    config: &PhotaraConfig,
    project_id: Uuid,
    manifest: &ProjectManifest,
) -> Result<()> {
    let scene = serde_json::to_value(&config.scenes[&manifest.scene])?;
    let location = serde_json::to_value(&config.locations[&manifest.location])?;
    sqlx::query(
        "UPDATE projects SET display_name = $2, scene_slug = $3, scene_snapshot = $4, \
         location_slug = $5, location_snapshot = $6, origin = $7, status = $8, \
         updated_at = now() WHERE id = $1",
    )
    .bind(project_id)
    .bind(&manifest.display_name)
    .bind(&manifest.scene)
    .bind(scene)
    .bind(&manifest.location)
    .bind(location)
    .bind(&manifest.origin)
    .bind(&manifest.status)
    .execute(&mut *connection)
    .await?;

    sqlx::query("DELETE FROM project_people WHERE project_id = $1")
        .bind(project_id)
        .execute(&mut *connection)
        .await?;
    for person in &manifest.people {
        sqlx::query(
            "INSERT INTO project_people (project_id, person_slug, person_snapshot) \
             VALUES ($1, $2, $3)",
        )
        .bind(project_id)
        .bind(person)
        .bind(serde_json::to_value(&config.people[person])?)
        .execute(&mut *connection)
        .await?;
    }
    Ok(())
}

fn ensure_project_matches(record: &ProjectRecord, manifest: &ProjectManifest) -> Result<()> {
    ensure_project_core_matches(record, manifest)?;
    if record.people != manifest.people {
        return Err(project_conflict(&manifest.slug));
    }
    Ok(())
}

fn ensure_project_core_matches(record: &ProjectRecord, manifest: &ProjectManifest) -> Result<()> {
    if record.slug != manifest.slug
        || record.display_name != manifest.display_name
        || record.scene != manifest.scene
        || record.location != manifest.location
        || record.origin != manifest.origin
        || record.status != manifest.status
    {
        return Err(project_conflict(&manifest.slug));
    }
    Ok(())
}

fn project_conflict(slug: &str) -> PhotaraError {
    PhotaraError::Configuration(format!(
        "project {slug:?} already exists with different configuration"
    ))
}

async fn people(connection: &mut PgConnection, project_id: Uuid) -> Result<Vec<String>> {
    let rows = sqlx::query(
        "SELECT person_slug FROM project_people WHERE project_id = $1 ORDER BY person_slug",
    )
    .bind(project_id)
    .fetch_all(&mut *connection)
    .await?;
    rows.into_iter()
        .map(|row| row.try_get("person_slug").map_err(Into::into))
        .collect()
}

fn materialize_directory(
    root: &Path,
    manifest: &ProjectManifest,
    replace: bool,
) -> Result<PathBuf> {
    if !root.is_dir() {
        return Err(PhotaraError::Configuration(format!(
            "projects_root {} is not an available directory",
            root.display()
        )));
    }
    let project_root = root.join(&manifest.slug);
    for relative in [
        "layouts/instagram",
        "layouts/website",
        "layouts/shared",
        "manifests",
        "masters/flattened",
        "workspace/exports",
        "workspace/previews",
        "workspace/tmp",
    ] {
        let path = project_root.join(relative);
        fs::create_dir_all(&path)
            .map_err(|source| PhotaraError::filesystem("create directory", path, source))?;
    }

    let manifest_path = project_root.join("project.json");
    let contents = format!("{}\n", serde_json::to_string_pretty(manifest)?);
    match fs::read_to_string(&manifest_path) {
        Ok(existing) if existing == contents => return Ok(project_root),
        Ok(_) if !replace => {
            return Err(PhotaraError::Configuration(format!(
                "{} already exists with different contents",
                manifest_path.display()
            )));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(PhotaraError::filesystem("read file", manifest_path, source));
        }
    }

    let temporary = project_root.join(".project.json.tmp");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|source| PhotaraError::filesystem("create file", &temporary, source))?;
    file.write_all(contents.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|source| PhotaraError::filesystem("write file", &temporary, source))?;
    fs::rename(&temporary, &manifest_path)
        .map_err(|source| PhotaraError::filesystem("rename file", manifest_path, source))?;
    Ok(project_root)
}

async fn record_event(
    connection: &mut PgConnection,
    project_id: Uuid,
    event_type: &str,
    idempotency_key: String,
    payload: serde_json::Value,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO workflow_events (project_id, event_type, payload, idempotency_key) \
         VALUES ($1, $2, $3, $4) ON CONFLICT (idempotency_key) DO NOTHING",
    )
    .bind(project_id)
    .bind(event_type)
    .bind(payload)
    .bind(idempotency_key)
    .execute(&mut *connection)
    .await?;
    Ok(())
}
