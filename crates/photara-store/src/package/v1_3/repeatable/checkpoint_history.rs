//! Deterministic first-turnover local history from independent journal/package
//! proofs. This prepares immutable bytes only; publication retains all barriers.
use super::{CheckpointIdentity, LocalArchive, check};
use crate::package::v1_3::repeatable::{VerifiedOriginal, journal::VerifiedSession, wire};
use crate::package::v1_3::{ReaderLimits, tree};
use crate::package::{PackageError, PackageUuid};
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// Exact immutable archive and lookup closure. Construction requires a verified,
/// drained old prefix; this object does not grant a lease or acknowledge Saved.
pub struct PreparedHistory<'a> {
    source: &'a VerifiedOriginal,
    archive: Value,
    lookup: Value,
    objects: BTreeMap<(String, u64), Value>,
    through: Value,
    mutation: Value,
    frame: Value,
    coordinate: Value,
    undo: Vec<Value>,
}
impl<'a> PreparedHistory<'a> {
    /// Compile first-adoption history using explicit existing tree geometry.
    /// # Errors
    /// Refuses incomplete/uncheckpointed journals, mismatched selected package,
    /// missing reciprocal local receipts, changed identities or bounded work.
    pub fn prepare(
        session: &VerifiedSession<'_>,
        current: &'a VerifiedOriginal,
        identity: &CheckpointIdentity,
        archive_id: PackageUuid,
        geometry: (usize, usize),
        limits: &ReaderLimits,
    ) -> Result<Self, PackageError> {
        let (journal, undo) = session.checkpoint_source(current)?;
        check(!archive_id.as_uuid().is_nil())?;
        let header = journal.header();
        for (field, value) in [
            ("project_id", identity.package.project.to_string()),
            ("library_id", identity.package.library.to_string()),
            (
                "bootstrap_sha256",
                identity.package.bootstrap_sha256.as_str().to_owned(),
            ),
            ("device_id", identity.device.to_string()),
            ("incarnation_id", identity.incarnation.to_string()),
        ] {
            check(header[field] == value)?;
        }
        let active = current
            .proof
            .roles
            .get("active")
            .ok_or(PackageError::Integrity)?;
        // Subsequent turnover needs the already-bound prior archive lookup, not
        // a fabricated flattened history. Keep that route fail closed for now.
        check(active.state["schema"]["version"] == 2)?;
        let mut local = BTreeMap::new();
        for (index, record) in journal.records().iter().enumerate() {
            let value = record.value();
            if value["kind"] == "Mutation" {
                let body = &value["body"];
                let operation = body["operation_receipt"]["operation_id"]
                    .as_str()
                    .ok_or(PackageError::Record)?;
                check(local.insert(operation.to_owned(), (index, body)).is_none())?;
            }
        }
        let mut entries = BTreeMap::new();
        for receipt in &active.receipts {
            let operation = receipt["operation_id"]
                .as_str()
                .ok_or(PackageError::Record)?;
            let evidence = if let Some((index, body)) = local.remove(operation) {
                check(body["operation_receipt"] == *receipt)?;
                json!({"mutation":journal.checkpoint_archive(index)?,"owner_sha256":wire::hash(&wire::encode(&body["owner"])),"action_sha256":wire::hash(&wire::encode(&body["action"]))})
            } else {
                Value::Null
            };
            let entry = json!({"operation_id":operation,"request_sha256":receipt["request_sha256"],"receipt":wire::reference(receipt),"local":evidence});
            check(entries.insert(operation.to_owned(), entry).is_none())?;
        }
        check(local.is_empty())?;
        let mut objects = BTreeMap::new();
        let lookup = build_tree(
            &entries.into_values().collect::<Vec<_>>(),
            identity,
            geometry,
            limits,
            &mut objects,
        )?;
        let archive = record(
            identity,
            "photara.session.checkpoint-archive",
            json!({"archive_id":archive_id.to_string(),"previous":null,"segments":[journal.checkpoint_segment()?]}),
        );
        LocalArchive::parse(&wire::encode(&archive), identity, limits.json)?;
        let last = journal.records().last().ok_or(PackageError::Integrity)?;
        let through = json!({"record_id":last.value()["record_id"],"checksum":last.checksum(),"sequence":last.value()["sequence"]});
        let mutation = if let Some(link) = session.through() {
            let row = journal
                .records()
                .iter()
                .find(|r| {
                    r.value()["record_id"] == link["record_id"]
                        && r.checksum() == link["record_checksum"]
                })
                .ok_or(PackageError::Integrity)?;
            json!({"record_id":row.value()["record_id"],"checksum":row.checksum(),"sequence":row.value()["sequence"]})
        } else {
            Value::Null
        };
        Ok(Self {
            source: current,
            archive,
            lookup,
            objects,
            through,
            mutation,
            frame: session.accepted_frame().unwrap_or(Value::Null),
            coordinate: session.coordinate(),
            undo,
        })
    }
    #[must_use]
    pub fn archive(&self) -> &Value {
        &self.archive
    }
    #[must_use]
    pub fn lookup(&self) -> &Value {
        &self.lookup
    }
    #[must_use]
    pub fn lookup_objects(&self) -> &BTreeMap<(String, u64), Value> {
        &self.objects
    }
    /// Construct local base only from an independently verified clean package
    /// whose active `AcceptanceBase` exactly matches the prepared coordinate.
    /// # Errors
    /// Refuses wrong identity, non-snapshot package, mismatched base/coordinates,
    /// nonempty suffix, or an unselected checkpoint commitment.
    pub fn bind_package(
        &self,
        checkpoint: &crate::package::v1_3::snapshot::AcceptanceBase,
        clean: &VerifiedOriginal,
        identity: &CheckpointIdentity,
        frames: crate::package::v1_3::FrameLimits,
        limits: &ReaderLimits,
        planner: crate::package::v1_3::repeatable::Limits,
    ) -> Result<super::LocalBase, PackageError> {
        crate::package::v1_3::reader::audit_checkpoint_adoption(
            self.source,
            checkpoint,
            &identity.package,
            frames,
            limits,
        )?;
        for key in [
            "project_id",
            "library_id",
            "bootstrap_sha256",
            "device_id",
            "incarnation_id",
        ] {
            let expected = record(identity, "identity", json!({}));
            check(self.archive[key] == expected[key])?;
        }
        let old = &self
            .source
            .proof
            .roles
            .get("active")
            .ok_or(PackageError::Integrity)?
            .state;
        check(self.source.scope == clean.scope && self.source.charge_unit == clean.charge_unit)?;
        let active = clean
            .proof
            .roles
            .get("active")
            .ok_or(PackageError::Integrity)?;
        let base = checkpoint.value();
        crate::package::v1_3::reader::snapshot_base_binding(
            clean,
            &crate::package::v1_3::selected::json_ref(&wire::reference(base))?,
            frames,
            limits,
        )?;
        check(
            active.state["schema"]["version"] == 3
                && active.state["authored"] == old["authored"]
                && active.state["history"] == old["history"]
                && active.state["accepted"] == base["accepted"]
                && active.state["authored_revision"] == base["authored_revision"]
                && active.state["authored"]["sha256"] == base["authored_sha256"]
                && active.state["journal_inclusion"] == base["journal_inclusion"]
                && active.state["authored_revision"] == self.coordinate["revision"]
                && active.semantic.contains(
                    &wire::key(&wire::reference(base)).map_err(|_| PackageError::Record)?,
                ),
        )?;
        let logical = crate::package::v1_3::repeatable::core::snapshot_closure(
            clean,
            identity.incarnation,
            frames,
            limits,
            planner,
        )
        .map_err(|_| PackageError::Integrity)?;
        check(
            serde_json::to_value(logical.coordinate()).map_err(|_| PackageError::Record)?
                == self.coordinate,
        )?;
        let value = record(
            identity,
            "photara.session.checkpoint-base",
            json!({"checkpoint":wire::reference(base),"package_head":clean.package.head,"package_commit":wire::reference(&clean.package.commit),"journal_id":self.archive["segments"][0]["journal_id"],"through":self.through,"mutation":self.mutation,"accepted_frame":self.frame,"coordinate":self.coordinate,"undo_roots":self.undo,"lookup":self.lookup,"archive":wire::reference(&self.archive)}),
        );
        super::LocalBase::parse(&wire::encode(&value), identity, limits.json)
    }
}
fn record(identity: &CheckpointIdentity, schema: &str, mut value: Value) -> Value {
    value["schema"] = json!({"id":schema,"version":1});
    value["project_id"] = json!(identity.package.project.to_string());
    value["library_id"] = json!(identity.package.library.to_string());
    value["bootstrap_sha256"] = json!(identity.package.bootstrap_sha256.as_str());
    value["device_id"] = json!(identity.device.to_string());
    value["incarnation_id"] = json!(identity.incarnation.to_string());
    value["extensions"] = json!({});
    value
}
fn build_tree(
    entries: &[Value],
    identity: &CheckpointIdentity,
    geometry: (usize, usize),
    limits: &ReaderLimits,
    objects: &mut BTreeMap<(String, u64), Value>,
) -> Result<Value, PackageError> {
    let (leaf, branch) = geometry;
    check(leaf > 0 && branch >= 2)?;
    if leaf > limits.tree.max_leaf_entries
        || branch > limits.tree.max_branch_children
        || entries.len() > limits.tree.max_entries
    {
        return Err(PackageError::Limit);
    }
    let insert = |value: Value,
                  objects: &mut BTreeMap<(String, u64), Value>|
     -> Result<Value, PackageError> {
        crate::package::parse_canonical_json(&wire::encode(&value), limits.json)?;
        let reference = wire::reference(&value);
        let key = wire::key(&reference).map_err(|_| PackageError::Record)?;
        objects.insert(key, value);
        if objects.len() > limits.tree.max_pages {
            return Err(PackageError::Limit);
        }
        Ok(reference)
    };
    let mut rows = Vec::new();
    let chunks = if entries.is_empty() {
        vec![entries]
    } else {
        entries.chunks(leaf).collect()
    };
    for chunk in chunks {
        let child = insert(
            record(
                identity,
                "photara.session.checkpoint-operation-lookup-leaf",
                json!({"count":chunk.len().to_string(),"entries":chunk}),
            ),
            objects,
        )?;
        rows.push(json!({"first":chunk.first().map(|v| &v["operation_id"]),"last":chunk.last().map(|v| &v["operation_id"]),"count":chunk.len().to_string(),"child":child}));
    }
    let mut depth = 0;
    while rows.len() > 1 {
        depth += 1;
        if depth > limits.tree.max_depth {
            return Err(PackageError::Limit);
        }
        let mut next = Vec::new();
        for chunk in rows.chunks(branch) {
            if chunk.len() == 1 {
                next.push(chunk[0].clone());
                continue;
            }
            let count = chunk.iter().try_fold(0u64, |sum, row| {
                sum.checked_add(tree::number(&row["count"])?)
                    .ok_or(PackageError::Limit)
            })?;
            let child = insert(
                record(
                    identity,
                    "photara.session.checkpoint-operation-lookup-branch",
                    json!({"count":count.to_string(),"children":chunk}),
                ),
                objects,
            )?;
            next.push(json!({"first":chunk.first().ok_or(PackageError::Record)?["first"],"last":chunk.last().ok_or(PackageError::Record)?["last"],"count":count.to_string(),"child":child}));
        }
        rows = next;
    }
    Ok(rows[0]["child"].clone())
}
