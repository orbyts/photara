//! Whole canonical batches and actor-bound, authenticated opaque cursors.
use crate::{Actor, Request, Result, Scope, Service, ServiceError, canonical, hash};
use photara_core::contracts::access::ProjectAction;
use photara_core::contracts::{
    AccountId, CommitId, DeviceId, LibraryId, OperationId, ProjectId, StorageLocationId,
};
use serde::{Deserialize, Serialize};
use sqlx::Row as _;
use storexa::Transaction;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StorageProjection {
    pub library: LibraryId,
    pub root: StorageLocationId,
    pub revision: i64,
    pub display_name: String,
    pub purpose: String,
    pub retired: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CatalogProjection {
    pub library: LibraryId,
    pub project: ProjectId,
    pub observation: Uuid,
    pub locator: Uuid,
    pub commit: CommitId,
    pub commit_sha256: [u8; 32],
    pub package_revision: u64,
    pub title: String,
    pub asset_count: i64,
    pub graph_count: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    content = "record",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
pub enum ContentRoot {
    Storage(StorageProjection),
    Catalog(CatalogProjection),
}
impl ContentRoot {
    pub(crate) fn validate(&self, scope: Scope) -> Result<()> {
        match self {
            Self::Storage(s) => {
                if scope != (Scope::Library { library: s.library })
                    || s.revision < 1
                    || s.display_name.trim().is_empty()
                    || s.display_name.len() > 512
                    || s.purpose.is_empty()
                    || s.purpose.len() > 128
                {
                    return Err(ServiceError::Invalid);
                }
            }
            Self::Catalog(c) => {
                if scope
                    != (Scope::Project {
                        library: c.library,
                        project: c.project,
                    })
                    || c.observation.is_nil()
                    || c.locator.is_nil()
                    || c.commit_sha256 == [0; 32]
                    || c.package_revision == 0
                    || c.title.len() > 4096
                    || c.asset_count < 0
                    || c.graph_count < 0
                {
                    return Err(ServiceError::Invalid);
                }
            }
        }
        canonical(self)?;
        Ok(())
    }
    pub(crate) fn coordinates(&self) -> (&'static str, Uuid, i64) {
        match self {
            Self::Storage(s) => ("storage-root", s.root.uuid(), s.revision),
            Self::Catalog(c) => ("package-observation", c.observation, 1),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishContent {
    pub operation: OperationId,
    pub expected_revision: Option<i64>,
    pub root: ContentRoot,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Cursor {
    payload: Vec<u8>,
    signature: [u8; 32],
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Position {
    scope: Scope,
    account: AccountId,
    device: DeviceId,
    generation: i64,
    stream: Uuid,
    epoch: Uuid,
    sequence: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContentReceipt {
    pub operation: OperationId,
    pub cursor: Cursor,
    pub root_sha256: [u8; 32],
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Batch {
    pub scope: Scope,
    pub stream: Uuid,
    pub epoch: Uuid,
    pub sequence: i64,
    pub operation: OperationId,
    pub roots: Vec<ContentRoot>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedPage {
    pub batches: Vec<Batch>,
    pub cursor: Cursor,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub roots: Vec<ContentRoot>,
    pub cursor: Cursor,
    pub canonical_sha256: [u8; 32],
}
impl Service {
    fn cursor(&self, p: &Position) -> Result<Cursor> {
        let payload = canonical(p)?;
        let signature = self.sign("cursor-v1", &payload)?;
        Ok(Cursor { payload, signature })
    }
    pub(crate) fn position(&self, actor: &Actor, r: Request, c: &Cursor) -> Result<Position> {
        if c.payload.len() > 4096
            || !crate::secret_eq(&self.sign("cursor-v1", &c.payload)?, &c.signature)
        {
            return Err(ServiceError::Invalid);
        }
        let p: Position = serde_json::from_slice(&c.payload).map_err(|_| ServiceError::Invalid)?;
        if canonical(&p)? != c.payload
            || p.scope != r.scope
            || p.account != actor.account
            || p.device != r.device
            || p.generation != r.generation
            || p.sequence < 0
        {
            return Err(ServiceError::Forbidden);
        }
        Ok(p)
    }
    /// The current content codecs cover storage projections and package observations.
    /// # Errors
    /// Rejects cross-scope content, stale bases, changed requests and missing current write rights.
    pub async fn publish_content(
        &self,
        actor: &Actor,
        r: Request,
        c: &PublishContent,
    ) -> Result<ContentReceipt> {
        c.root.validate(r.scope)?;
        let action = match c.root {
            ContentRoot::Storage(_) => ProjectAction::ManageStorage,
            ContentRoot::Catalog(_) => ProjectAction::Edit,
        };
        let mut tx = self.begin(actor, r, action, &[]).await?;
        let body = (r.scope, "publish-content", c);
        let request = canonical(&body)?;
        if let Some(row)=sqlx::query("SELECT actor_account_id,device_id,request_canonical,request_sha256,response_canonical,response_sha256,outcome FROM photara_private.scoped_command_receipts WHERE library_id=$1 AND operation_id=$2").bind(r.scope.library().uuid()).bind(c.operation.uuid()).fetch_optional(&mut *tx).await? {
            if row.try_get::<Uuid,_>("actor_account_id")?!=actor.account.uuid()||row.try_get::<Uuid,_>("device_id")?!=r.device.uuid()||row.try_get::<Vec<u8>,_>("request_canonical")?!=request{return Err(ServiceError::Conflict);}
            if row.try_get::<Vec<u8>,_>("request_sha256")?!=hash(&request)||row.try_get::<String,_>("outcome")?!="accepted"{return Err(ServiceError::Integrity);}
            let bytes:Vec<u8>=row.try_get("response_canonical")?;let response:ContentReceipt=serde_json::from_slice(&bytes).map_err(|_|ServiceError::Integrity)?;
            if row.try_get::<Vec<u8>,_>("response_sha256")?!=hash(&bytes)||canonical(&response)?!=bytes{return Err(ServiceError::Integrity);}
            // Receipt bytes retain their original cursor. It cannot bootstrap a later generation.
            tx.commit().await?;return Ok(response);
        }
        match &c.root {
            ContentRoot::Storage(s) => {
                let label = photara_library::gen2::normalize_term(&s.display_name)
                    .map_err(|_| ServiceError::Invalid)?;
                if let Some(expected) = c.expected_revision {
                    if expected < 1
                        || s.revision != expected.checked_add(1).ok_or(ServiceError::Conflict)?
                    {
                        return Err(ServiceError::Conflict);
                    }
                    let n=sqlx::query("UPDATE photara.storage_roots SET display_name=$3,label_key=$4,purpose=$5,revision=revision+1,state=$6,updated_at=CURRENT_TIMESTAMP,retired_at=CASE WHEN $7 THEN CURRENT_TIMESTAMP ELSE NULL END WHERE library_id=$1 AND storage_root_id=$2 AND revision=$8").bind(s.library.uuid()).bind(s.root.uuid()).bind(&s.display_name).bind(label).bind(&s.purpose).bind(if s.retired{"tombstoned"}else{"active"}).bind(s.retired).bind(expected).execute(&mut *tx).await?.rows_affected();
                    if n != 1 {
                        return Err(ServiceError::Conflict);
                    }
                } else {
                    if s.revision != 1 || s.retired {
                        return Err(ServiceError::Invalid);
                    }
                    sqlx::query("INSERT INTO photara.storage_roots VALUES($1,$2,$3,$4,$5,1,'active',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,NULL)").bind(s.root.uuid()).bind(s.library.uuid()).bind(&s.display_name).bind(label).bind(&s.purpose).execute(&mut *tx).await?;
                }
            }
            ContentRoot::Catalog(p) => {
                if c.expected_revision.is_some() {
                    return Err(ServiceError::Invalid);
                }
                let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM photara.project_locators WHERE library_id=$1 AND project_id=$2 AND locator_id=$3 AND state='active')").bind(p.library.uuid()).bind(p.project.uuid()).bind(p.locator).fetch_one(&mut *tx).await?;
                if !exists {
                    crate::runtime::authorize(&mut tx, r.scope, ProjectAction::ManageStorage)
                        .await?;
                    sqlx::query("INSERT INTO photara.project_locators VALUES($1,$2,$3,NULL,NULL,'active',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,NULL)").bind(p.locator).bind(p.library.uuid()).bind(p.project.uuid()).execute(&mut *tx).await?;
                }
                let bytes = canonical(p)?;
                sqlx::query("INSERT INTO photara.package_observations(observation_id,library_id,project_id,locator_id,commit_id,commit_sha256,package_revision,index_schema,title,project_lifecycle,asset_count,graph_count,reported_by_account_id,reported_by_device_id,observed_at,received_at,projection_canonical,projection_sha256) VALUES($1,$2,$3,$4,$5,$6,$7::text::numeric,1,$8,'active',$9,$10,$11,$12,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,$13,$14)").bind(p.observation).bind(p.library.uuid()).bind(p.project.uuid()).bind(p.locator).bind(p.commit.uuid()).bind(p.commit_sha256.to_vec()).bind(p.package_revision.to_string()).bind(&p.title).bind(p.asset_count).bind(p.graph_count).bind(actor.account.uuid()).bind(r.device.uuid()).bind(&bytes).bind(hash(&bytes).to_vec()).execute(&mut *tx).await?;
            }
        }
        let mut p = stream(&mut tx, actor, r).await?;
        p.sequence = p.sequence.checked_add(1).ok_or(ServiceError::Conflict)?;
        let batch = Batch {
            scope: r.scope,
            stream: p.stream,
            epoch: p.epoch,
            sequence: p.sequence,
            operation: c.operation,
            roots: vec![c.root.clone()],
        };
        let bytes = canonical(&batch)?;
        let receipt = ContentReceipt {
            operation: c.operation,
            cursor: self.cursor(&p)?,
            root_sha256: hash(&canonical(&c.root)?),
        };
        let response = canonical(&receipt)?;
        sqlx::query("INSERT INTO photara_private.scoped_command_receipts(library_id,operation_id,actor_account_id,device_id,scope_kind,project_id,command_kind,request_canonical,request_sha256,outcome,response_canonical,response_sha256,accepted_stream_id,accepted_epoch,accepted_sequence,completed_at) VALUES($1,$2,$3,$4,$5,$6,'publish-content',$7,$8,'accepted',$9,$10,$11,$12,$13,CURRENT_TIMESTAMP)").bind(r.scope.library().uuid()).bind(c.operation.uuid()).bind(actor.account.uuid()).bind(r.device.uuid()).bind(r.scope.kind()).bind(r.scope.project().map(ProjectId::uuid)).bind(&request).bind(hash(&request).to_vec()).bind(&response).bind(hash(&response).to_vec()).bind(p.stream).bind(p.epoch).bind(p.sequence).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO photara_private.scoped_change_batches VALUES($1,$2,$3,$4,$5,$6,1,$7,$8,CURRENT_TIMESTAMP)").bind(p.stream).bind(p.epoch).bind(p.sequence).bind(r.scope.library().uuid()).bind(r.scope.project().map(ProjectId::uuid)).bind(c.operation.uuid()).bind(&bytes).bind(hash(&bytes).to_vec()).execute(&mut *tx).await?;
        let (kind, id, revision) = c.root.coordinates();
        let root = canonical(&c.root)?;
        sqlx::query("INSERT INTO photara_private.scoped_changes VALUES($1,$2,$3,0,$4,$5,$6,$7,$8,'upsert',1,$9,$10,$11)").bind(p.stream).bind(p.epoch).bind(p.sequence).bind(r.scope.library().uuid()).bind(r.scope.project().map(ProjectId::uuid)).bind(kind).bind(id).bind(revision).bind(serde_json::to_value(&c.root).map_err(|_|ServiceError::Invalid)?).bind(&root).bind(hash(&root).to_vec()).execute(&mut *tx).await?;
        let n=sqlx::query("UPDATE photara_private.scoped_streams SET last_sequence=$2 WHERE stream_id=$1 AND last_sequence=$3").bind(p.stream).bind(p.sequence).bind(p.sequence-1).execute(&mut *tx).await?.rows_affected();
        if n != 1 {
            return Err(ServiceError::Conflict);
        }
        tx.commit().await?;
        Ok(receipt)
    }
    /// Returns the complete current projection for the explicitly supported content codecs.
    /// # Errors
    /// Rejects access loss, unsupported stored codecs, corruption or snapshot limits.
    pub async fn snapshot(&self, actor: &Actor, r: Request) -> Result<Snapshot> {
        let mut tx = self.begin(actor, r, ProjectAction::Read, &[]).await?;
        let p = stream(&mut tx, actor, r).await?;
        let mut roots = Vec::new();
        if let Some(project) = r.scope.project() {
            let bounded:bool=sqlx::query_scalar("SELECT count(*)<=10000 AND coalesce(sum(octet_length(projection_canonical)),0)<=16777216 FROM photara.package_observations WHERE library_id=$1 AND project_id=$2").bind(r.scope.library().uuid()).bind(project.uuid()).fetch_one(&mut *tx).await?;
            if !bounded {
                return Err(ServiceError::Invalid);
            }
            let rows=sqlx::query("SELECT projection_canonical,projection_sha256 FROM photara.package_observations WHERE library_id=$1 AND project_id=$2 ORDER BY observation_id LIMIT 10001").bind(r.scope.library().uuid()).bind(project.uuid()).fetch_all(&mut *tx).await?;
            for row in rows {
                let bytes: Vec<u8> = row.try_get("projection_canonical")?;
                if row.try_get::<Vec<u8>, _>("projection_sha256")? != hash(&bytes) {
                    return Err(ServiceError::Integrity);
                }
                let v: CatalogProjection =
                    serde_json::from_slice(&bytes).map_err(|_| ServiceError::Unsupported)?;
                if canonical(&v)? != bytes {
                    return Err(ServiceError::Integrity);
                }
                roots.push(ContentRoot::Catalog(v));
            }
        } else {
            let bounded:bool=sqlx::query_scalar("SELECT count(*)<=10000 AND coalesce(sum((octet_length(display_name)::bigint+octet_length(purpose))*6+512),0)<=16777216 FROM photara.storage_roots WHERE library_id=$1").bind(r.scope.library().uuid()).fetch_one(&mut *tx).await?;
            if !bounded {
                return Err(ServiceError::Invalid);
            }
            let rows=sqlx::query("SELECT storage_root_id,revision,display_name,purpose,state FROM photara.storage_roots WHERE library_id=$1 ORDER BY storage_root_id LIMIT 10001").bind(r.scope.library().uuid()).fetch_all(&mut *tx).await?;
            for row in rows {
                roots.push(ContentRoot::Storage(StorageProjection {
                    library: r.scope.library(),
                    root: StorageLocationId::from_uuid(row.try_get("storage_root_id")?)
                        .map_err(|_| ServiceError::Integrity)?,
                    revision: row.try_get("revision")?,
                    display_name: row.try_get("display_name")?,
                    purpose: row.try_get("purpose")?,
                    retired: row.try_get::<String, _>("state")? == "tombstoned",
                }));
            }
        }
        if roots.len() > 10_000 {
            return Err(ServiceError::Invalid);
        }
        for root in &roots {
            root.validate(r.scope)?;
        }
        let bytes = photara_core::canonical_json(&roots).map_err(|_| ServiceError::Integrity)?;
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(ServiceError::Invalid);
        }
        let cursor = self.cursor(&p)?;
        tx.commit().await?;
        Ok(Snapshot {
            roots,
            cursor,
            canonical_sha256: hash(&bytes),
        })
    }
    /// # Errors
    /// Requires a current authenticated cursor; whole batches are never per-user filtered.
    pub async fn feed(
        &self,
        actor: &Actor,
        r: Request,
        c: &Cursor,
        limit: u16,
    ) -> Result<FeedPage> {
        if limit == 0 || limit > 100 {
            return Err(ServiceError::Invalid);
        }
        let mut p = self.position(actor, r, c)?;
        let mut tx = self.begin(actor, r, ProjectAction::Read, &[]).await?;
        let current = stream(&mut tx, actor, r).await?;
        if p.stream != current.stream || p.epoch != current.epoch || p.sequence > current.sequence {
            return Err(ServiceError::Conflict);
        }
        let rows=sqlx::query("SELECT sequence,batch_canonical,batch_sha256 FROM (SELECT sequence,batch_canonical,batch_sha256,sum(octet_length(batch_canonical)) OVER(ORDER BY sequence) AS page_bytes FROM photara_private.scoped_change_batches WHERE stream_id=$1 AND epoch=$2 AND sequence>$3 ORDER BY sequence LIMIT $4) page WHERE page_bytes<=16777216 ORDER BY sequence").bind(p.stream).bind(p.epoch).bind(p.sequence).bind(i64::from(limit)).fetch_all(&mut *tx).await?;
        let mut batches = Vec::new();
        let mut total = 0;
        for row in rows {
            let bytes: Vec<u8> = row.try_get("batch_canonical")?;
            total += bytes.len();
            if total > 16 * 1024 * 1024 {
                break;
            }
            if row.try_get::<Vec<u8>, _>("batch_sha256")? != hash(&bytes) {
                return Err(ServiceError::Integrity);
            }
            let batch: Batch =
                serde_json::from_slice(&bytes).map_err(|_| ServiceError::Unsupported)?;
            if canonical(&batch)? != bytes
                || batch.scope != r.scope
                || batch.stream != p.stream
                || batch.epoch != p.epoch
                || batch.sequence != p.sequence + 1
                || batch.sequence != row.try_get::<i64, _>("sequence")?
                || batch.roots.is_empty()
                || batch.roots.len() > 1000
            {
                return Err(ServiceError::Integrity);
            }
            let children=sqlx::query("SELECT ordinal,entity_kind,entity_id,entity_revision,record_schema,post_state_canonical,post_state_sha256 FROM photara_private.scoped_changes WHERE stream_id=$1 AND epoch=$2 AND sequence=$3 ORDER BY ordinal").bind(p.stream).bind(p.epoch).bind(batch.sequence).fetch_all(&mut *tx).await?;
            if children.len() != batch.roots.len() {
                return Err(ServiceError::Integrity);
            }
            for (n, (root, child)) in batch.roots.iter().zip(children).enumerate() {
                root.validate(r.scope)?;
                let bytes = canonical(root)?;
                let (kind, id, revision) = root.coordinates();
                if child.try_get::<i32, _>("ordinal")?
                    != i32::try_from(n).map_err(|_| ServiceError::Integrity)?
                    || child.try_get::<String, _>("entity_kind")? != kind
                    || child.try_get::<Uuid, _>("entity_id")? != id
                    || child.try_get::<i64, _>("entity_revision")? != revision
                    || child.try_get::<i32, _>("record_schema")? != 1
                    || child.try_get::<Vec<u8>, _>("post_state_canonical")? != bytes
                    || child.try_get::<Vec<u8>, _>("post_state_sha256")? != hash(&bytes)
                {
                    return Err(ServiceError::Integrity);
                }
            }
            p.sequence = batch.sequence;
            batches.push(batch);
        }
        tx.commit().await?;
        Ok(FeedPage {
            batches,
            cursor: self.cursor(&p)?,
        })
    }
    /// A snapshot/served-feed cursor is the only accepted acknowledgement authority.
    /// # Errors
    /// Rejects forged cursors, old generations, out-of-range or regressing acknowledgements.
    pub async fn acknowledge(&self, actor: &Actor, r: Request, c: &Cursor) -> Result<()> {
        let p = self.position(actor, r, c)?;
        let mut tx = self.begin(actor, r, ProjectAction::Read, &[]).await?;
        let current = stream(&mut tx, actor, r).await?;
        if p.stream != current.stream || p.epoch != current.epoch || p.sequence > current.sequence {
            return Err(ServiceError::Conflict);
        }
        sqlx::query("INSERT INTO photara_private.scoped_sync_clients VALUES($1,$2,$3,$4,$5,$6,CURRENT_TIMESTAMP) ON CONFLICT(stream_id,account_id,device_id) DO UPDATE SET authorization_generation=EXCLUDED.authorization_generation,epoch=EXCLUDED.epoch,acknowledged_sequence=EXCLUDED.acknowledged_sequence,last_seen_at=EXCLUDED.last_seen_at").bind(p.stream).bind(actor.account.uuid()).bind(r.device.uuid()).bind(r.generation).bind(p.epoch).bind(p.sequence).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
}
async fn stream(tx: &mut Transaction, actor: &Actor, r: Request) -> Result<Position> {
    let row=sqlx::query("SELECT stream_id,epoch,last_sequence FROM photara_private.scoped_streams WHERE library_id=$1 AND scope_kind=$2 AND project_id IS NOT DISTINCT FROM $3").bind(r.scope.library().uuid()).bind(r.scope.kind()).bind(r.scope.project().map(ProjectId::uuid)).fetch_optional(&mut **tx).await?.ok_or(ServiceError::Forbidden)?;
    Ok(Position {
        scope: r.scope,
        account: actor.account,
        device: r.device,
        generation: r.generation,
        stream: row.try_get("stream_id")?,
        epoch: row.try_get("epoch")?,
        sequence: row.try_get("last_sequence")?,
    })
}
