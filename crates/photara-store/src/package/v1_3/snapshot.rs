//! Snapshot-recovery records. Structural/audit evidence is not custody or Saved.
use super::super::{
    JsonLimits, ObjectRef, PackageError, PackageUuid, Sha256Hex, parse_canonical_json,
};
use super::operations::{OperationAudit, validate_receipt};
use super::selected::{hash_value, json_ref, nonnil, schema, sha};
use super::tree::{fields, number};
use super::{
    LogicalKey, LogicalRecords, LogicalTree, LogicalTreeKind, Membership, SelectionIdentity,
    TreeLimits,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub const FEATURE: &str = "photara.snapshot-recovery.v1";

/// An authenticated checkpoint coordinate. Historical roots are lookup edges,
/// not a claim that their complete history was read on this open.
pub struct AcceptanceBase {
    value: Value,
}
impl AcceptanceBase {
    /// # Errors
    /// Refuses noncanonical bytes, unknown fields, wrong identity/schema or
    /// inconsistent accepted/authored/journal coordinates.
    pub fn parse(
        bytes: &[u8],
        identity: &SelectionIdentity,
        limits: JsonLimits,
    ) -> Result<Self, PackageError> {
        Self::from_value(parse_canonical_json(bytes, limits)?, identity)
    }
    pub(super) fn from_value(
        value: Value,
        identity: &SelectionIdentity,
    ) -> Result<Self, PackageError> {
        schema(
            &value,
            "photara.package.acceptance-base",
            1,
            &[
                "library_id",
                "bootstrap_sha256",
                "checkpoint_id",
                "source_commit",
                "source_state",
                "authored_revision",
                "authored_sha256",
                "accepted",
                "journal_inclusion",
                "receipts_by_id",
                "receipts_by_ordinal",
                "receipt_count",
            ],
            identity.project,
        )?;
        exact_common(&value, identity)?;
        nonnil(&value["checkpoint_id"])?;
        fields(&value["source_commit"], &["commit_id", "commit_sha256"])?;
        nonnil(&value["source_commit"]["commit_id"])?;
        sha(&value["source_commit"]["commit_sha256"])?;
        for field in ["source_state", "receipts_by_id", "receipts_by_ordinal"] {
            json_ref(&value[field])?;
        }
        number(&value["authored_revision"])?;
        sha(&value["authored_sha256"])?;
        let count = accepted(&value["accepted"])?;
        if number(&value["receipt_count"])? != count {
            return Err(PackageError::Integrity);
        }
        inclusion(
            &value["journal_inclusion"],
            count,
            &value["authored_revision"],
            &value["authored_sha256"],
        )?;
        if count == 0 && value["accepted"]["prefix_sha256"] != empty_prefix(identity)?.as_str() {
            return Err(PackageError::Integrity);
        }
        Ok(Self { value })
    }
    /// Validates base commitments against the complete old operation-prefix audit.
    /// Other old package closure/accounting and live custody checks remain separate;
    /// this content comparison alone cannot authorize or publish first adoption.
    /// # Errors
    /// Refuses altered source commitments, indexes, coordinates or old history.
    pub fn audit_adoption(
        &self,
        source: &impl LogicalRecords,
        commit: &Value,
        state: &Value,
        identity: &SelectionIdentity,
        limits: TreeLimits,
    ) -> Result<(), PackageError> {
        exact_common(&self.value, identity)?;
        exact_common(state, identity)?;
        if commit["project_id"] != identity.project.to_string()
            || commit["bootstrap_sha256"] != identity.bootstrap_sha256.as_str()
            || commit["root_set"]["active"]
                != serde_json::to_value(object_ref(state)?).map_err(|_| PackageError::Record)?
            || state["schema"] != json!({"id":"photara.package.state-root","version":2})
            || self.value["source_state"]
                != serde_json::to_value(object_ref(state)?).map_err(|_| PackageError::Record)?
            || self.value["source_commit"]
                != json!({"commit_id":commit["commit_id"],"commit_sha256":hash_value(commit)?})
            || self.value["authored_revision"] != state["authored_revision"]
            || self.value["authored_sha256"] != state["authored"]["sha256"]
            || self.value["accepted"] != state["accepted"]
            || self.value["journal_inclusion"] != state["journal_inclusion"]
        {
            return Err(PackageError::Integrity);
        }
        super::audit_operations(source, state, identity, limits)?;
        let index = source.resolve(&json_ref(&state["operation_index"])?, Membership::Semantic)?;
        if self.value["receipts_by_id"] != index["by_id"]
            || self.value["receipts_by_ordinal"] != index["by_ordinal"]
        {
            return Err(PackageError::Integrity);
        }
        Ok(())
    }
    #[must_use]
    pub fn value(&self) -> &Value {
        &self.value
    }

    /// Authenticates one exact historical receipt through both reciprocal index
    /// paths. Unvisited archive bytes are intentionally not audited.
    /// # Errors
    /// Refuses malformed paths, missing reciprocal entry, wrong receipt binding,
    /// out-of-base ordinal, or a reused ID with a different request digest.
    pub fn lookup(
        &self,
        archive: &impl LogicalRecords,
        identity: &SelectionIdentity,
        operation: PackageUuid,
        request: Option<&Sha256Hex>,
        limits: TreeLimits,
    ) -> Result<Option<Value>, PackageError> {
        exact_common(&self.value, identity)?;
        let tree = LogicalTree::new(
            archive,
            identity.project,
            LogicalTreeKind::OperationId,
            limits,
        );
        let Some(entry) = tree.lookup(
            &json_ref(&self.value["receipts_by_id"])?,
            &LogicalKey::Id(operation),
        )?
        else {
            return Ok(None);
        };
        entry_fields(&entry)?;
        let ordinal = number(&entry["acceptance_ordinal"])?;
        if ordinal == 0
            || ordinal > number(&self.value["receipt_count"])?
            || nonnil(&entry["operation_id"])? != operation
        {
            return Err(PackageError::Integrity);
        }
        let reciprocal = LogicalTree::new(
            archive,
            identity.project,
            LogicalTreeKind::OperationOrdinal,
            limits,
        )
        .lookup(
            &json_ref(&self.value["receipts_by_ordinal"])?,
            &LogicalKey::Ordinal(ordinal),
        )?;
        if reciprocal.as_ref() != Some(&entry) {
            return Err(PackageError::Integrity);
        }
        let receipt = resolve_receipt(archive, &entry, identity)?;
        if request.is_some_and(|expected| receipt["request_sha256"] != expected.as_str()) {
            return Err(PackageError::Integrity);
        }
        Ok(Some(receipt))
    }
}
fn object_ref(value: &Value) -> Result<ObjectRef, PackageError> {
    let bytes = photara_core::canonical_json(value).map_err(|_| PackageError::Record)?;
    Ok(ObjectRef {
        kind: super::super::ObjectKind::Json,
        sha256: super::super::digest(&bytes),
        byte_length: super::super::DecimalU64::parse(&bytes.len().to_string())?,
    })
}
fn exact_common(value: &Value, identity: &SelectionIdentity) -> Result<(), PackageError> {
    if value["project_id"] != identity.project.to_string()
        || value["library_id"] != identity.library.to_string()
        || value["bootstrap_sha256"] != identity.bootstrap_sha256.as_str()
        || value["extensions"] != json!({})
    {
        return Err(PackageError::Integrity);
    }
    Ok(())
}
fn accepted(value: &Value) -> Result<u64, PackageError> {
    fields(value, &["through_ordinal", "prefix_sha256"])?;
    sha(&value["prefix_sha256"])?;
    number(&value["through_ordinal"])
}
fn empty_prefix(identity: &SelectionIdentity) -> Result<Sha256Hex, PackageError> {
    hash_value(
        &json!({"domain":"photara.package.accepted-prefix.v1","project_id":identity.project,"library_id":identity.library,"bootstrap_sha256":identity.bootstrap_sha256,"through_ordinal":"0"}),
    )
}
fn inclusion(
    value: &Value,
    count: u64,
    revision: &Value,
    digest: &Value,
) -> Result<(), PackageError> {
    if count == 0 {
        return if value.is_null() {
            Ok(())
        } else {
            Err(PackageError::Integrity)
        };
    }
    fields(
        value,
        &[
            "journal_id",
            "through_sequence",
            "prefix_sha256",
            "resulting_authored_revision",
            "resulting_authored_sha256",
        ],
    )?;
    nonnil(&value["journal_id"])?;
    if number(&value["through_sequence"])? == 0
        || value["resulting_authored_revision"] != *revision
        || value["resulting_authored_sha256"] != *digest
    {
        return Err(PackageError::Integrity);
    }
    sha(&value["prefix_sha256"])?;
    Ok(())
}
fn entry_fields(entry: &Value) -> Result<(), PackageError> {
    fields(
        entry,
        &[
            "operation_id",
            "request_sha256",
            "acceptance_ordinal",
            "receipt",
        ],
    )?;
    nonnil(&entry["operation_id"])?;
    sha(&entry["request_sha256"])?;
    number(&entry["acceptance_ordinal"])?;
    json_ref(&entry["receipt"])?;
    Ok(())
}
fn resolve_receipt(
    provider: &impl LogicalRecords,
    entry: &Value,
    identity: &SelectionIdentity,
) -> Result<Value, PackageError> {
    let value = provider.resolve(&json_ref(&entry["receipt"])?, Membership::Semantic)?;
    validate_receipt(&value, identity)?;
    if ["operation_id", "request_sha256", "acceptance_ordinal"]
        .iter()
        .any(|key| value[key] != entry[key])
    {
        return Err(PackageError::Integrity);
    }
    Ok(value)
}

/// Audits all post-base receipts, while historical absence/duplicate checks use
/// authenticated scoped archive paths. Does not certify base adoption/custody.
/// # Errors
/// Refuses identity/version, reciprocal coverage, discontinuity, archive ID reuse,
/// prefix/coordinate mismatch or caller traversal budget exhaustion.
#[expect(
    clippy::too_many_lines,
    reason = "Explicit snapshot suffix proof and hash domains"
)]
pub fn audit_snapshot_operations(
    current: &impl LogicalRecords,
    archive: &impl LogicalRecords,
    state: &Value,
    identity: &SelectionIdentity,
    limits: TreeLimits,
) -> Result<OperationAudit, PackageError> {
    if state["schema"] != json!({"id":"photara.package.state-root","version":3}) {
        return Err(PackageError::UnsupportedVersion);
    }
    exact_common(state, identity)?;
    let reference = json_ref(&state["operation_index"])?;
    let index = current.resolve(&reference, Membership::Semantic)?;
    schema(
        &index,
        "photara.package.operation-index",
        3,
        &[
            "library_id",
            "bootstrap_sha256",
            "base",
            "accepted",
            "by_id",
            "by_ordinal",
        ],
        identity.project,
    )?;
    exact_common(&index, identity)?;
    let base_ref = json_ref(&index["base"])?;
    let base =
        AcceptanceBase::from_value(current.resolve(&base_ref, Membership::Semantic)?, identity)?;
    let start = accepted(&base.value["accepted"])?;
    let end = accepted(&index["accepted"])?;
    if end < start || index["accepted"] != state["accepted"] {
        return Err(PackageError::Integrity);
    }
    let by_id = LogicalTree::new(
        current,
        identity.project,
        LogicalTreeKind::OperationId,
        limits,
    )
    .audit(&json_ref(&index["by_id"])?)?;
    let by_ordinal = LogicalTree::new(
        current,
        identity.project,
        LogicalTreeKind::OperationOrdinal,
        limits,
    )
    .audit(&json_ref(&index["by_ordinal"])?)?;
    if end - start != by_id.entries.len() as u64 || by_id.entries.len() != by_ordinal.entries.len()
    {
        return Err(PackageError::Integrity);
    }
    let mut objects = BTreeSet::from([reference, base_ref]);
    objects.extend(by_id.nodes);
    objects.extend(by_ordinal.nodes);
    let mut mapped = BTreeMap::new();
    for entry in &by_id.entries {
        entry_fields(entry)?;
        if mapped
            .insert(nonnil(&entry["operation_id"])?, entry)
            .is_some()
        {
            return Err(PackageError::Integrity);
        }
    }
    let mut prefix = sha(&base.value["accepted"]["prefix_sha256"])?;
    let mut coordinate =
        json!({"revision":base.value["authored_revision"],"digest":base.value["authored_sha256"]});
    let mut journal = base.value["journal_inclusion"].clone();
    let mut sequence = if journal.is_null() {
        0
    } else {
        number(&journal["through_sequence"])?
    };
    let mut receipts = Vec::new();
    for (offset, entry) in by_ordinal.entries.iter().enumerate() {
        entry_fields(entry)?;
        let ordinal = start
            .checked_add(offset as u64)
            .and_then(|n| n.checked_add(1))
            .ok_or(PackageError::Limit)?;
        let operation = nonnil(&entry["operation_id"])?;
        if mapped.get(&operation) != Some(&entry)
            || number(&entry["acceptance_ordinal"])? != ordinal
            || base
                .lookup(archive, identity, operation, None, limits)?
                .is_some()
        {
            return Err(PackageError::Integrity);
        }
        let receipt = resolve_receipt(current, entry, identity)?;
        if receipt["before"] != coordinate || number(&receipt["journal_sequence"])? <= sequence {
            return Err(PackageError::Integrity);
        }
        if journal.is_null() {
            journal = json!({"journal_id":receipt["journal_id"],"prefix_sha256":hash_value(&json!({"domain":"photara.package.journal-prefix.v1","journal_id":receipt["journal_id"],"through_sequence":"0"}))?});
        }
        if journal["journal_id"] != receipt["journal_id"] {
            return Err(PackageError::Integrity);
        }
        let receipt_ref = json_ref(&entry["receipt"])?;
        prefix = hash_value(
            &json!({"domain":"photara.package.accepted-prefix-link.v1","previous_sha256":prefix,"acceptance_ordinal":receipt["acceptance_ordinal"],"operation_id":receipt["operation_id"],"request_sha256":receipt["request_sha256"],"receipt_sha256":receipt_ref.sha256}),
        )?;
        let frame = json!({"schema":{"id":"photara.package.accepted-journal-frame","version":1},"journal_id":receipt["journal_id"],"sequence":receipt["journal_sequence"],"operation_id":receipt["operation_id"],"request_sha256":receipt["request_sha256"],"receipt_sha256":receipt_ref.sha256});
        journal["prefix_sha256"] = serde_json::to_value(hash_value(&json!({"domain":"photara.package.journal-prefix-link.v1","previous_sha256":journal["prefix_sha256"],"frame_sha256":hash_value(&frame)?}))?).map_err(|_| PackageError::Record)?;
        journal["through_sequence"] = receipt["journal_sequence"].clone();
        journal["resulting_authored_revision"] = receipt["after"]["revision"].clone();
        journal["resulting_authored_sha256"] = receipt["after"]["digest"].clone();
        coordinate = receipt["after"].clone();
        sequence = number(&receipt["journal_sequence"])?;
        objects.insert(receipt_ref);
        receipts.push(receipt);
    }
    if index["accepted"]["prefix_sha256"] != prefix.as_str()
        || coordinate["revision"] != state["authored_revision"]
        || coordinate["digest"] != state["authored"]["sha256"]
        || journal != state["journal_inclusion"]
    {
        return Err(PackageError::Integrity);
    }
    inclusion(
        &journal,
        end,
        &state["authored_revision"],
        &state["authored"]["sha256"],
    )?;
    Ok(OperationAudit { receipts, objects })
}

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod tests;
