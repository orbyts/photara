//! Context apply journal. Local variable writes and receipts commit together;
//! Project intents retain package authority until the separately gated publisher.
use super::local::local_authority;
use super::*;
use photara_core::context::{
    expression::Coordinate,
    proposal::{
        ApplyEvidence, ApplyPlan, ApplyReceipt, Authority, ReceiptOutcome, ReceiptSpec,
        VariableChangeProposal,
    },
    variable::{Binding, ValueOrigin, VariableAggregate},
};
use photara_core::contracts::{
    LocalPrincipalId, OperationId, ReceiptId, VariableValueId,
    dto::{RevisionCoordinate, ScopeRef},
};

impl LocalLibraryStore {
    /// Applies one explicitly accepted proposal with current authorization and CAS.
    /// An exact retry returns its retained receipt; changed bytes never reuse an ID.
    /// # Errors
    /// Rejects invalid source evidence, changed requests, Project authority and stale CAS.
    pub async fn apply_library_proposal(
        &self,
        actor: LocalPrincipalId,
        proposal: &VariableChangeProposal,
        evidence: &ApplyEvidence,
        at: photara_core::context::value::Timestamp,
    ) -> Result<ApplyReceipt> {
        let plan =
            ApplyPlan::prepare(vec![proposal.clone()], evidence).map_err(|_| Error::Invalid)?;
        let Authority::Library { library_id } = plan.authority() else {
            return Err(Error::Unsupported);
        };
        let library = LibraryId::try_from(library_id.uuid())?;
        let mut tx = self.write().await?;
        local_authority(&mut tx, library, actor).await?;
        let spec = proposal.spec();
        let bytes = canonical(proposal)?.into_bytes();
        if let Some(receipt) = existing_apply(&mut tx, library, spec.operation_id, &bytes).await? {
            return Ok(receipt);
        }
        let values = super::context::read_variables(&mut tx, library).await?;
        let Coordinate::Variable {
            scope: ScopeRef::Library {
                library_id: target_library,
            },
            variable_id,
        } = spec.target.coordinate
        else {
            return Err(Error::Invalid);
        };
        if target_library != library_id {
            return Err(Error::Invalid);
        }
        let old = values
            .iter()
            .find(|v| v.spec().variable_id == variable_id)
            .ok_or(Error::Invalid)?;
        let before = old.spec();
        if before.revision != spec.expected_aggregate_revision
            || before.owner_revision != spec.expected_owner_revision
            || before.ty != spec.literal.ty
            || spec.sensitivity < before.sensitivity
            || spec.portability < before.portability
        {
            return Err(Error::Conflict);
        }
        let mut next = before.clone();
        next.current = Some(Binding::Literal(Box::new(spec.literal.clone())));
        next.revision = before.revision.next().map_err(|_| Error::Limit)?;
        let RevisionCoordinate::Local { revision } = before.owner_revision else {
            return Err(Error::Unsupported);
        };
        next.owner_revision = RevisionCoordinate::Local {
            revision: revision.next().map_err(|_| Error::Limit)?,
        };
        next.value_id =
            Some(before.value_id.unwrap_or(
                VariableValueId::from_uuid(Uuid::new_v4()).map_err(|_| Error::Invalid)?,
            ));
        next.origin = ValueOrigin::NodeProposal;
        next.updated_at = at.clone();
        next.sensitivity = spec.sensitivity;
        next.portability = spec.portability;
        let next = VariableAggregate::try_from(next).map_err(|_| Error::Invalid)?;
        insert_intent(
            &mut tx,
            library,
            proposal,
            &bytes,
            "prepared",
            None,
            super::context::timestamp_millis(&at)?,
        )
        .await?;
        super::context::put_variable(&mut tx, library, &next, Some(before.revision)).await?;
        let receipt = ApplyReceipt::try_from(ReceiptSpec {
            receipt_id: ReceiptId::from_uuid(Uuid::new_v4()).map_err(|_| Error::Invalid)?,
            operation_id: spec.operation_id,
            request_digest: proposal.request_digest().map_err(|_| Error::Invalid)?,
            authority: plan.authority(),
            outcome: ReceiptOutcome::Applied,
            resulting_revisions: vec![RevisionCoordinate::Local {
                revision: next.spec().revision,
            }],
            prior_receipts: vec![],
            observed_at: at,
            evidence: vec![spec.snapshot.clone()],
        })
        .map_err(|_| Error::Invalid)?;
        let result = canonical(&receipt)?.into_bytes();
        let observed = super::context::timestamp_millis(&receipt.spec().observed_at)?;
        sqlx::query(
            "INSERT INTO context_apply_receipts VALUES(?,?,?,'local-applied',?,?,NULL,NULL,?)",
        )
        .bind(receipt.spec().receipt_id.uuid().as_bytes().to_vec())
        .bind(library.bytes())
        .bind(spec.operation_id.uuid().as_bytes().to_vec())
        .bind(&result)
        .bind(sha(&result).to_vec())
        .bind(observed)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE context_apply_intents SET state='settled',local_revision=local_revision+1,updated_at=? WHERE operation_id=? AND state='prepared' AND local_revision=1").bind(observed).bind(spec.operation_id.uuid().as_bytes().to_vec()).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(receipt)
    }
    /// Persists a Project-authority intent without changing or publishing a package.
    /// The verified proposal carries the exact expected package commit coordinate.
    /// # Errors
    /// Rejects denied context access, missing commit pins and changed-request retries.
    pub async fn prepare_project_proposal(
        &self,
        actor: LocalPrincipalId,
        library: LibraryId,
        proposal: &VariableChangeProposal,
        evidence: &ApplyEvidence,
        at: Timestamp,
    ) -> Result<()> {
        let plan =
            ApplyPlan::prepare(vec![proposal.clone()], evidence).map_err(|_| Error::Invalid)?;
        let Authority::Project { project_id } = plan.authority() else {
            return Err(Error::Invalid);
        };
        let project = ProjectId::try_from(project_id.uuid())?;
        let mut tx = self.write().await?;
        super::local::project_authority(
            &mut tx,
            library,
            project,
            actor,
            photara_core::contracts::access::ProjectAction::ManageContext,
        )
        .await?;
        let RevisionCoordinate::Package {
            commit_id,
            commit_sha256,
        } = proposal.spec().expected_owner_revision
        else {
            return Err(Error::Invalid);
        };
        let bytes = canonical(proposal)?.into_bytes();
        let old=sqlx::query("SELECT request_canonical,state FROM context_apply_intents WHERE operation_id=? AND library_id=?").bind(proposal.spec().operation_id.uuid().as_bytes().to_vec()).bind(library.bytes()).fetch_optional(&mut *tx).await?;
        if let Some(old) = old {
            if old.try_get::<Vec<u8>, _>("request_canonical")? != bytes {
                return Err(Error::Conflict);
            }
            return Ok(());
        }
        insert_intent(
            &mut tx,
            library,
            proposal,
            &bytes,
            "awaiting-publication",
            Some((project, commit_id.uuid(), digest_bytes(commit_sha256)?)),
            at.get(),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
    /// Retained requests are recovery evidence, never permission to redispatch an
    /// unknown operation. Package publication remains a separate authority.
    /// # Errors
    /// Returns denied scope or corrupted canonical evidence.
    pub async fn pending_context_applies(
        &self,
        actor: LocalPrincipalId,
        library: LibraryId,
    ) -> Result<Vec<PendingContextApply>> {
        let mut tx = self.read().await?;
        local_authority(&mut tx, library, actor).await?;
        let rows=sqlx::query("SELECT operation_id,state,request_canonical,request_sha256 FROM context_apply_intents WHERE library_id=? AND state<>'settled' ORDER BY created_at,operation_id LIMIT 1001").bind(library.bytes()).fetch_all(&mut *tx).await?;
        if rows.len() > 1000 {
            return Err(Error::Limit);
        }
        rows.into_iter()
            .map(|row| {
                let bytes: Vec<u8> = row.try_get("request_canonical")?;
                if row.try_get::<Vec<u8>, _>("request_sha256")? != sha(&bytes) {
                    return Err(Error::Corrupt);
                }
                let proposal: VariableChangeProposal =
                    serde_json::from_slice(&bytes).map_err(|_| Error::Corrupt)?;
                if canonical(&proposal)?.as_bytes() != bytes
                    || row.try_get::<Vec<u8>, _>("operation_id")?
                        != proposal.spec().operation_id.uuid().as_bytes()
                {
                    return Err(Error::Corrupt);
                }
                let state = match row.try_get::<String, _>("state")?.as_str() {
                    "prepared" => ContextApplyState::Prepared,
                    "awaiting-publication" => ContextApplyState::AwaitingPublication,
                    "unknown" => ContextApplyState::Unknown,
                    _ => return Err(Error::Corrupt),
                };
                Ok(PendingContextApply { proposal, state })
            })
            .collect()
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextApplyState {
    Prepared,
    AwaitingPublication,
    Unknown,
}
#[derive(Clone, Debug)]
pub struct PendingContextApply {
    pub proposal: VariableChangeProposal,
    pub state: ContextApplyState,
}
async fn existing_apply(
    conn: &mut SqliteConnection,
    library: LibraryId,
    operation: OperationId,
    bytes: &[u8],
) -> Result<Option<ApplyReceipt>> {
    let row=sqlx::query("SELECT i.request_canonical,r.result_canonical,r.result_sha256 FROM context_apply_intents i LEFT JOIN context_apply_receipts r ON r.library_id=i.library_id AND r.operation_id=i.operation_id AND r.observation_kind='local-applied' WHERE i.operation_id=? AND i.library_id=?").bind(operation.uuid().as_bytes().to_vec()).bind(library.bytes()).fetch_optional(&mut *conn).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    if row.try_get::<Vec<u8>, _>("request_canonical")? != bytes {
        return Err(Error::Conflict);
    }
    let result: Vec<u8> = row
        .try_get::<Option<Vec<u8>>, _>("result_canonical")?
        .ok_or(Error::Conflict)?;
    if row.try_get::<Vec<u8>, _>("result_sha256")? != sha(&result) {
        return Err(Error::Corrupt);
    }
    let receipt: ApplyReceipt = serde_json::from_slice(&result).map_err(|_| Error::Corrupt)?;
    if canonical(&receipt)?.as_bytes() != result || receipt.spec().operation_id != operation {
        return Err(Error::Corrupt);
    }
    Ok(Some(receipt))
}
async fn insert_intent(
    conn: &mut SqliteConnection,
    library: LibraryId,
    proposal: &VariableChangeProposal,
    bytes: &[u8],
    state: &str,
    package: Option<(ProjectId, Uuid, Vec<u8>)>,
    at: i64,
) -> Result<()> {
    let s = proposal.spec();
    let proposal_id = Uuid::parse_str(&String::from(s.proposal_id)).map_err(|_| Error::Invalid)?;
    sqlx::query("INSERT INTO context_apply_intents VALUES(?,?,?,?,?,?,?,?,?,?,?,?,1,1,?,?)")
        .bind(s.operation_id.uuid().as_bytes().to_vec())
        .bind(library.bytes())
        .bind(package.as_ref().map(|p| p.0.bytes()))
        .bind(proposal_id.as_bytes().to_vec())
        .bind(bytes)
        .bind(sha(bytes).to_vec())
        .bind(s.run_id.uuid().as_bytes().to_vec())
        .bind(s.attempt_id.uuid().as_bytes().to_vec())
        .bind(if package.is_some() {
            "project"
        } else {
            "library"
        })
        .bind(package.as_ref().map(|p| p.1.as_bytes().to_vec()))
        .bind(package.as_ref().map(|p| p.2.clone()))
        .bind(state)
        .bind(at)
        .bind(at)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
fn digest_bytes(digest: photara_core::contracts::schema::Digest) -> Result<Vec<u8>> {
    let s = digest.to_string();
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| Error::Invalid))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use photara_core::context::proposal::{
        ProposalSourceEvidence, ProposalSpec, RunOutcome, TargetEvidence,
    };
    use photara_core::contracts::schema::{DecimalU64, Digest, ObjectKind, ObjectRef};
    fn id<T: std::str::FromStr>() -> T
    where
        T::Err: std::fmt::Debug,
    {
        Uuid::new_v4().to_string().parse().unwrap()
    }
    #[tokio::test]
    #[allow(clippy::too_many_lines)] // One end-to-end transaction/restart scenario.
    async fn proposal_atomic_apply_retry_collision_and_reopen() {
        let dir = tempfile::tempdir_in("/private/tmp").unwrap();
        let path = dir.path().join("state.sqlite");
        let at = Timestamp::try_from(1_789_142_400_000).unwrap();
        let (store, identity) = LocalLibraryStore::open_app_state(&path, at).await.unwrap();
        let v = super::super::context::tests::variable(identity.library_id);
        store
            .put_library_variable(identity.principal_id, &v, None)
            .await
            .unwrap();
        let Binding::Literal(literal) = v.spec().current.clone().unwrap() else {
            panic!()
        };
        let p = VariableChangeProposal::try_from(ProposalSpec {
            proposal_id: Uuid::new_v4().to_string().try_into().unwrap(),
            operation_id: id(),
            target: v.dependency(),
            expected_aggregate_revision: v.spec().revision,
            expected_owner_revision: v.spec().owner_revision.clone(),
            literal: *literal,
            sensitivity: v.spec().sensitivity,
            portability: v.spec().portability,
            project_id: id(),
            run_id: id(),
            attempt_id: id(),
            snapshot_id: id(),
            snapshot: ObjectRef {
                kind: ObjectKind::Json,
                sha256: Digest::of_bytes(b"snapshot"),
                byte_length: DecimalU64::new(8),
            },
            snapshot_digest: Digest::of_bytes(b"snapshot-context"),
            output_digest: Digest::of_bytes(b"output"),
        })
        .unwrap();
        let s = p.spec();
        let evidence = ApplyEvidence {
            run_id: s.run_id,
            outcome: RunOutcome::Succeeded,
            explicit_acceptance: true,
            targets: vec![TargetEvidence {
                target: s.target.clone(),
                current_aggregate_revision: s.expected_aggregate_revision,
                current_owner_revision: s.expected_owner_revision.clone(),
                expected_type: s.literal.ty.clone(),
                sensitivity: s.sensitivity,
                portability: s.portability,
                declared: true,
                currently_authorized: true,
            }],
            sources: vec![ProposalSourceEvidence {
                run_id: s.run_id,
                attempt_id: s.attempt_id,
                snapshot_id: s.snapshot_id,
                snapshot: s.snapshot.clone(),
                snapshot_digest: s.snapshot_digest,
                output_digest: s.output_digest,
                sensitivity: s.sensitivity,
                portability: s.portability,
            }],
        };
        let receipt = store
            .apply_library_proposal(
                identity.principal_id,
                &p,
                &evidence,
                v.spec().updated_at.clone(),
            )
            .await
            .unwrap();
        assert_eq!(
            store
                .apply_library_proposal(
                    identity.principal_id,
                    &p,
                    &evidence,
                    v.spec().updated_at.clone()
                )
                .await
                .unwrap(),
            receipt
        );
        store.verify_integrity().await.unwrap();
        store.close().await;
        let (store, reopened) = LocalLibraryStore::open_app_state(&path, at).await.unwrap();
        assert_eq!(identity, reopened);
        assert_eq!(
            store
                .apply_library_proposal(
                    identity.principal_id,
                    &p,
                    &evidence,
                    v.spec().updated_at.clone()
                )
                .await
                .unwrap(),
            receipt
        );
        assert_eq!(
            store
                .library_variables(identity.principal_id, identity.library_id)
                .await
                .unwrap()[0]
                .spec()
                .revision
                .get(),
            2
        );
        let mut changed = p.spec().clone();
        changed.operation_id = id();
        let stale = VariableChangeProposal::try_from(changed).unwrap();
        assert!(matches!(
            store
                .apply_library_proposal(
                    identity.principal_id,
                    &stale,
                    &evidence,
                    v.spec().updated_at.clone()
                )
                .await,
            Err(Error::Conflict)
        ));
        assert!(
            store
                .pending_context_applies(identity.principal_id, identity.library_id)
                .await
                .unwrap()
                .is_empty()
        );
        store.close().await;
    }
}
