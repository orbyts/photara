use std::collections::{BTreeMap, HashMap};

use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::Row;
use storexa::Database;
use uuid::Uuid;

use crate::{
    PhotaraError, Result, cloud::ADOBE_LIGHTROOM_PROVIDER, config::PhotaraConfig,
    project::ProjectRecord,
};

const COLLECTION_NAMESPACE: &str = "photara-cloud-collection-v1";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CloudCollectionPlan {
    pub schema_version: u32,
    pub provider: String,
    pub account_label: String,
    pub project: String,
    pub inventory_snapshot_sha256: String,
    pub cloud_asset_count: usize,
    pub collection_count: usize,
    pub leaf_album_count: usize,
    pub album_membership_count: usize,
    pub nodes: Vec<CloudCollectionNode>,
    pub assets: Vec<CloudCollectionAsset>,
    #[serde(skip)]
    pub account_id: Uuid,
    #[serde(skip)]
    pub inventory_run_id: Uuid,
    #[serde(skip)]
    pub project_id: Uuid,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CloudCollectionNode {
    pub semantic_path: String,
    pub display_name: String,
    pub node_kind: String,
    pub remote_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_remote_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CloudCollectionAsset {
    pub asset_id: Uuid,
    pub remote_asset_id: String,
    pub remote_filename: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CloudCollectionSyncReport {
    pub schema_version: u32,
    pub provider: String,
    pub account_label: String,
    pub project: String,
    pub sync_run_id: Uuid,
    pub inventory_snapshot_sha256: String,
    pub collection_count: usize,
    pub leaf_album_count: usize,
    pub cloud_asset_count: usize,
    pub verified_membership_count: usize,
}

pub async fn plan(
    database: &Database,
    config: &PhotaraConfig,
    project: &ProjectRecord,
    account_label: &str,
) -> Result<CloudCollectionPlan> {
    let account = sqlx::query("SELECT id FROM cloud_accounts WHERE provider = $1 AND label = $2")
        .bind(ADOBE_LIGHTROOM_PROVIDER)
        .bind(account_label)
        .fetch_optional(database.pool())
        .await?
        .ok_or_else(|| {
            PhotaraError::Configuration(format!(
                "Adobe Lightroom Cloud account {account_label:?} was not found"
            ))
        })?;
    let account_id: Uuid = account.try_get("id")?;
    let inventory = sqlx::query(
        "SELECT id, snapshot_sha256 FROM cloud_provider_inventory_runs \
         WHERE account_id = $1 ORDER BY completed_at DESC, id DESC LIMIT 1",
    )
    .bind(account_id)
    .fetch_optional(database.pool())
    .await?
    .ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "Adobe Lightroom Cloud account {account_label:?} has no inventory"
        ))
    })?;
    let inventory_run_id: Uuid = inventory.try_get("id")?;
    let assets = sqlx::query(
        "SELECT asset.id AS asset_id, presence.remote_asset_id, inventory.file_name \
         FROM project_assets AS membership \
         JOIN assets AS asset ON asset.id = membership.asset_id \
         JOIN asset_cloud_presence AS presence ON presence.asset_id = asset.id \
           AND presence.account_id = $2 AND presence.status = 'present' \
         JOIN cloud_provider_inventory_assets AS inventory \
           ON inventory.run_id = $3 AND inventory.remote_asset_id = presence.remote_asset_id \
         WHERE membership.project_id = $1 AND inventory.file_name IS NOT NULL \
         ORDER BY inventory.file_name, asset.id",
    )
    .bind(project.id)
    .bind(account_id)
    .bind(inventory_run_id)
    .fetch_all(database.pool())
    .await?
    .into_iter()
    .map(|row| {
        Ok(CloudCollectionAsset {
            asset_id: row.try_get("asset_id")?,
            remote_asset_id: row.try_get("remote_asset_id")?,
            remote_filename: row.try_get("file_name")?,
        })
    })
    .collect::<std::result::Result<Vec<_>, sqlx::Error>>()?;
    if assets.is_empty() {
        return Err(PhotaraError::Configuration(format!(
            "project {:?} has no provider-verified Cloud assets",
            project.slug
        )));
    }

    let location = config.locations.get(&project.location).ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "project {:?} references missing location {:?}",
            project.slug, project.location
        ))
    })?;
    let scene = config.scenes.get(&project.scene).ok_or_else(|| {
        PhotaraError::Configuration(format!(
            "project {:?} references missing scene {:?}",
            project.slug, project.scene
        ))
    })?;
    let mut paths: BTreeMap<String, Vec<(String, String, String)>> = BTreeMap::new();
    paths.insert(
        format!("locations/{}/{}", project.location, project.slug),
        vec![
            ("locations".into(), "Locations".into(), "set".into()),
            (
                format!("locations/{}", project.location),
                location.display_name.clone(),
                "set".into(),
            ),
            (
                format!("locations/{}/{}", project.location, project.slug),
                project.display_name.clone(),
                "album".into(),
            ),
        ],
    );
    paths.insert(
        format!("scenes/{}/{}", project.scene, project.slug),
        vec![
            ("scenes".into(), "Scenes".into(), "set".into()),
            (
                format!("scenes/{}", project.scene),
                scene.display_name.clone(),
                "set".into(),
            ),
            (
                format!("scenes/{}/{}", project.scene, project.slug),
                project.display_name.clone(),
                "album".into(),
            ),
        ],
    );
    paths.insert(
        format!("projects/{}", project.slug),
        vec![
            ("projects".into(), "Projects".into(), "set".into()),
            (
                format!("projects/{}", project.slug),
                project.display_name.clone(),
                "album".into(),
            ),
        ],
    );
    for person_slug in &project.people {
        let person = config.people.get(person_slug).ok_or_else(|| {
            PhotaraError::Configuration(format!(
                "project {:?} references missing person {person_slug:?}",
                project.slug
            ))
        })?;
        paths.insert(
            format!("people/{person_slug}/{}", project.slug),
            vec![
                ("people".into(), "People".into(), "set".into()),
                (
                    format!("people/{person_slug}"),
                    person.display_name.clone(),
                    "set".into(),
                ),
                (
                    format!("people/{person_slug}/{}", project.slug),
                    project.display_name.clone(),
                    "album".into(),
                ),
            ],
        );
    }

    let mut unique = HashMap::<String, (String, String)>::new();
    for chain in paths.values() {
        for (path, name, kind) in chain {
            unique.insert(path.clone(), (name.clone(), kind.clone()));
        }
    }
    let mut semantic_paths = unique.keys().cloned().collect::<Vec<_>>();
    semantic_paths.sort_by_key(|path| (path.matches('/').count(), path.clone()));
    let nodes = semantic_paths
        .into_iter()
        .map(|semantic_path| {
            let (display_name, node_kind) = unique.remove(&semantic_path).expect("known path");
            let parent_path = semantic_path.rsplit_once('/').map(|(parent, _)| parent);
            CloudCollectionNode {
                remote_id: remote_id(account_id, &semantic_path),
                parent_remote_id: parent_path.map(|parent| remote_id(account_id, parent)),
                semantic_path,
                display_name,
                node_kind,
            }
        })
        .collect::<Vec<_>>();
    let leaf_album_count = nodes
        .iter()
        .filter(|node| node.node_kind == "album")
        .count();
    let album_membership_count = leaf_album_count * assets.len();
    Ok(CloudCollectionPlan {
        schema_version: 1,
        provider: ADOBE_LIGHTROOM_PROVIDER.into(),
        account_label: account_label.into(),
        project: project.slug.clone(),
        inventory_snapshot_sha256: inventory.try_get("snapshot_sha256")?,
        cloud_asset_count: assets.len(),
        collection_count: nodes.len(),
        leaf_album_count,
        album_membership_count,
        nodes,
        assets,
        account_id,
        inventory_run_id,
        project_id: project.id,
    })
}

pub async fn record_sync(
    database: &Database,
    plan: &CloudCollectionPlan,
) -> Result<CloudCollectionSyncReport> {
    let run_id = Uuid::new_v4();
    let mut transaction = database.begin().await?;
    sqlx::query(
        "INSERT INTO cloud_collection_sync_runs \
           (id, account_id, project_id, inventory_run_id, state, plan) \
         VALUES ($1, $2, $3, $4, 'complete', $5)",
    )
    .bind(run_id)
    .bind(plan.account_id)
    .bind(plan.project_id)
    .bind(plan.inventory_run_id)
    .bind(serde_json::to_value(plan)?)
    .execute(&mut *transaction)
    .await?;
    for node in &plan.nodes {
        let id = parse_remote_id(&node.remote_id)?;
        let parent_id = node
            .parent_remote_id
            .as_deref()
            .map(parse_remote_id)
            .transpose()?;
        sqlx::query(
            "INSERT INTO cloud_collection_nodes \
               (id, account_id, semantic_path, display_name, node_kind, parent_id, \
                remote_id, last_sync_run_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
             ON CONFLICT (account_id, semantic_path) DO UPDATE SET \
               display_name = EXCLUDED.display_name, node_kind = EXCLUDED.node_kind, \
               parent_id = EXCLUDED.parent_id, remote_id = EXCLUDED.remote_id, \
               last_sync_run_id = EXCLUDED.last_sync_run_id, updated_at = now()",
        )
        .bind(id)
        .bind(plan.account_id)
        .bind(&node.semantic_path)
        .bind(&node.display_name)
        .bind(&node.node_kind)
        .bind(parent_id)
        .bind(&node.remote_id)
        .bind(run_id)
        .execute(&mut *transaction)
        .await?;
        if node.node_kind == "album" {
            for asset in &plan.assets {
                sqlx::query(
                    "INSERT INTO cloud_collection_memberships \
                       (collection_id, asset_id, remote_asset_id, last_sync_run_id) \
                     VALUES ($1, $2, $3, $4) \
                     ON CONFLICT (collection_id, asset_id) DO UPDATE SET \
                       remote_asset_id = EXCLUDED.remote_asset_id, \
                       last_sync_run_id = EXCLUDED.last_sync_run_id, \
                       last_verified_at = now()",
                )
                .bind(id)
                .bind(asset.asset_id)
                .bind(&asset.remote_asset_id)
                .bind(run_id)
                .execute(&mut *transaction)
                .await?;
            }
            let asset_ids = plan
                .assets
                .iter()
                .map(|asset| asset.asset_id)
                .collect::<Vec<_>>();
            sqlx::query(
                "DELETE FROM cloud_collection_memberships \
                 WHERE collection_id = $1 AND NOT (asset_id = ANY($2))",
            )
            .bind(id)
            .bind(&asset_ids)
            .execute(&mut *transaction)
            .await?;
        }
    }
    transaction.commit().await?;
    Ok(CloudCollectionSyncReport {
        schema_version: 1,
        provider: plan.provider.clone(),
        account_label: plan.account_label.clone(),
        project: plan.project.clone(),
        sync_run_id: run_id,
        inventory_snapshot_sha256: plan.inventory_snapshot_sha256.clone(),
        collection_count: plan.collection_count,
        leaf_album_count: plan.leaf_album_count,
        cloud_asset_count: plan.cloud_asset_count,
        verified_membership_count: plan.album_membership_count,
    })
}

fn remote_id(account_id: Uuid, semantic_path: &str) -> String {
    let digest =
        Sha256::digest(format!("{COLLECTION_NAMESPACE}:{account_id}:{semantic_path}").as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes).simple().to_string()
}

fn parse_remote_id(value: &str) -> Result<Uuid> {
    Uuid::parse_str(value).map_err(|_| {
        PhotaraError::Configuration(format!("invalid deterministic collection ID {value:?}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_collection_ids_are_stable_and_path_scoped() {
        let account = Uuid::nil();
        assert_eq!(
            remote_id(account, "projects"),
            remote_id(account, "projects")
        );
        assert_ne!(
            remote_id(account, "projects/red-meridian"),
            remote_id(account, "people/trinity-woodward/red-meridian")
        );
        assert_eq!(remote_id(account, "projects").len(), 32);
    }
}
