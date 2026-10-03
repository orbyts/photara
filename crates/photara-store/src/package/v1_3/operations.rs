//! Exact accepted-prefix and reciprocal receipt-index verification.
use super::super::{ObjectRef, PackageError};
use super::selected::{hash_value, json_ref, nonnil, schema, sha};
use super::tree::{fields, number};
use super::{
    LogicalRecords, LogicalTree, LogicalTreeKind, Membership, SelectionIdentity, TreeLimits,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// Full operation-index evidence only; neither host authorization nor durability.
pub struct OperationAudit {
    receipts: Vec<Value>,
    objects: BTreeSet<ObjectRef>,
}
impl OperationAudit {
    #[must_use]
    pub fn receipts(&self) -> &[Value] {
        &self.receipts
    }
    #[must_use]
    pub fn objects(&self) -> &BTreeSet<ObjectRef> {
        &self.objects
    }
}
/// Checks both complete indexes, exact historical receipt bytes, accepted/journal
/// prefixes and authored continuity. Limits are caller budgets, not the fixture's
/// former 32-operation ceiling. Historical provenance never grants current access.
/// # Errors
/// Refuses unsupported receipt/provenance forms, mismatched reciprocal indexes,
/// gaps, identity or prefix failures. Does not replay commands or certify Graph closure.
#[expect(
    clippy::too_many_lines,
    reason = "Keep exact frozen field and identity checks in auditable protocol order"
)]
pub fn audit_operations(
    provider: &impl LogicalRecords,
    state: &Value,
    identity: &SelectionIdentity,
    limits: TreeLimits,
) -> Result<OperationAudit, PackageError> {
    let reference = json_ref(&state["operation_index"])?;
    let index = provider.resolve(&reference, Membership::Semantic)?;
    schema(
        &index,
        "photara.package.operation-index",
        2,
        &[
            "library_id",
            "bootstrap_sha256",
            "accepted",
            "by_id",
            "by_ordinal",
        ],
        identity.project,
    )?;
    if index["library_id"] != identity.library.to_string()
        || index["bootstrap_sha256"] != identity.bootstrap_sha256.as_str()
        || index["accepted"] != state["accepted"]
    {
        return Err(PackageError::Integrity);
    }
    fields(&index["accepted"], &["through_ordinal", "prefix_sha256"])?;
    let count = number(&index["accepted"]["through_ordinal"])?;
    sha(&index["accepted"]["prefix_sha256"])?;
    let by_id = LogicalTree::new(
        provider,
        identity.project,
        LogicalTreeKind::OperationId,
        limits,
    )
    .audit(&json_ref(&index["by_id"])?)?;
    let by_ordinal = LogicalTree::new(
        provider,
        identity.project,
        LogicalTreeKind::OperationOrdinal,
        limits,
    )
    .audit(&json_ref(&index["by_ordinal"])?)?;
    if count != by_id.entries.len() as u64 || count != by_ordinal.entries.len() as u64 {
        return Err(PackageError::Integrity);
    }
    let mut objects = by_id.nodes;
    objects.extend(by_ordinal.nodes);
    objects.insert(reference);
    let mut mapped = BTreeMap::new();
    for entry in &by_id.entries {
        fields(
            entry,
            &[
                "operation_id",
                "request_sha256",
                "acceptance_ordinal",
                "receipt",
            ],
        )?;
        mapped.insert(nonnil(&entry["operation_id"])?, entry);
    }
    let mut prefix = hash_value(
        &json!({"domain":"photara.package.accepted-prefix.v1","project_id":identity.project,"library_id":identity.library,"bootstrap_sha256":identity.bootstrap_sha256,"through_ordinal":"0"}),
    )?;
    let mut receipts: Vec<Value> = Vec::new();
    let mut sequence = 0;
    for (ordinal, entry) in by_ordinal.entries.iter().enumerate() {
        fields(
            entry,
            &[
                "operation_id",
                "request_sha256",
                "acceptance_ordinal",
                "receipt",
            ],
        )?;
        if mapped.get(&nonnil(&entry["operation_id"])?) != Some(&entry)
            || number(&entry["acceptance_ordinal"])? != ordinal as u64 + 1
        {
            return Err(PackageError::Integrity);
        }
        let receipt_ref = json_ref(&entry["receipt"])?;
        let receipt = provider.resolve(&receipt_ref, Membership::Semantic)?;
        validate_receipt(&receipt, identity)?;
        if ["operation_id", "request_sha256", "acceptance_ordinal"]
            .iter()
            .any(|k| receipt[k] != entry[k])
            || number(&receipt["journal_sequence"])? <= sequence
        {
            return Err(PackageError::Integrity);
        }
        sequence = number(&receipt["journal_sequence"])?;
        if receipts
            .last()
            .is_some_and(|p| p["after"] != receipt["before"])
        {
            return Err(PackageError::Integrity);
        }
        prefix = hash_value(
            &json!({"domain":"photara.package.accepted-prefix-link.v1","previous_sha256":prefix,"acceptance_ordinal":receipt["acceptance_ordinal"],"operation_id":receipt["operation_id"],"request_sha256":receipt["request_sha256"],"receipt_sha256":receipt_ref.sha256}),
        )?;
        objects.insert(receipt_ref);
        receipts.push(receipt);
    }
    if index["accepted"]["prefix_sha256"] != prefix.as_str() {
        return Err(PackageError::Integrity);
    }
    if let Some(last) = receipts.last() {
        if last["after"]["digest"] != state["authored"]["sha256"]
            || last["after"]["revision"] != state["authored_revision"]
        {
            return Err(PackageError::Integrity);
        }
        journal(&receipts, state)?;
    } else if !state["journal_inclusion"].is_null() {
        return Err(PackageError::Integrity);
    }
    Ok(OperationAudit { receipts, objects })
}
fn validate_receipt(value: &Value, identity: &SelectionIdentity) -> Result<(), PackageError> {
    schema(
        value,
        "photara.package.operation-receipt",
        1,
        &[
            "library_id",
            "bootstrap_sha256",
            "operation_id",
            "request_sha256",
            "acceptance_ordinal",
            "journal_id",
            "journal_sequence",
            "outcome",
            "before",
            "after",
            "undo_group_id",
            "provenance",
        ],
        identity.project,
    )?;
    if value["library_id"] != identity.library.to_string()
        || value["bootstrap_sha256"] != identity.bootstrap_sha256.as_str()
        || value["outcome"] != "accepted"
        || !value["undo_group_id"].is_null()
    {
        return Err(PackageError::Record);
    }
    nonnil(&value["operation_id"])?;
    nonnil(&value["journal_id"])?;
    sha(&value["request_sha256"])?;
    number(&value["acceptance_ordinal"])?;
    number(&value["journal_sequence"])?;
    for coordinate in [&value["before"], &value["after"]] {
        fields(coordinate, &["revision", "digest"])?;
        number(&coordinate["revision"])?;
        sha(&coordinate["digest"])?;
    }
    let p = &value["provenance"];
    fields(
        p,
        &[
            "principal",
            "actor",
            "grantor",
            "grant_ref",
            "effective_scope",
            "policy_decision_sha256",
        ],
    )?;
    for principal in [&p["principal"], &p["grantor"]] {
        fields(principal, &["kind", "account_id"])?;
        if principal["kind"] != "account" {
            return Err(PackageError::UnsupportedVersion);
        }
        nonnil(&principal["account_id"])?;
    }
    fields(&p["actor"], &["kind", "actor_id"])?;
    nonnil(&p["actor"]["actor_id"])?;
    fields(&p["grant_ref"], &["kind", "reference_id"])?;
    nonnil(&p["grant_ref"]["reference_id"])?;
    fields(&p["effective_scope"], &["project_id", "actions"])?;
    // These are the exact reviewed persisted provenance variants. Future actors
    // or action-mask encodings need explicit dispatch, not a permissive alias.
    if p["actor"]["kind"] != "photara.gui"
        || p["grant_ref"]["kind"] != "photara.project-grant"
        || p["effective_scope"]["project_id"] != identity.project.to_string()
        || p["effective_scope"]["actions"] != 71
    {
        return Err(PackageError::UnsupportedVersion);
    }
    sha(&p["policy_decision_sha256"])?;
    Ok(())
}
fn journal(receipts: &[Value], state: &Value) -> Result<(), PackageError> {
    let journal = &state["journal_inclusion"];
    fields(
        journal,
        &[
            "journal_id",
            "through_sequence",
            "prefix_sha256",
            "resulting_authored_revision",
            "resulting_authored_sha256",
        ],
    )?;
    nonnil(&journal["journal_id"])?;
    let mut prefix = hash_value(
        &json!({"domain":"photara.package.journal-prefix.v1","journal_id":journal["journal_id"],"through_sequence":"0"}),
    )?;
    for receipt in receipts {
        if receipt["journal_id"] != journal["journal_id"] {
            return Err(PackageError::Integrity);
        }
        let frame = json!({"schema":{"id":"photara.package.accepted-journal-frame","version":1},"journal_id":receipt["journal_id"],"sequence":receipt["journal_sequence"],"operation_id":receipt["operation_id"],"request_sha256":receipt["request_sha256"],"receipt_sha256":hash_value(receipt)?});
        prefix = hash_value(
            &json!({"domain":"photara.package.journal-prefix-link.v1","previous_sha256":prefix,"frame_sha256":hash_value(&frame)?}),
        )?;
    }
    let last = receipts.last().ok_or(PackageError::Record)?;
    if journal["through_sequence"] != last["journal_sequence"]
        || journal["prefix_sha256"] != prefix.as_str()
        || journal["resulting_authored_revision"] != last["after"]["revision"]
        || journal["resulting_authored_sha256"] != last["after"]["digest"]
    {
        return Err(PackageError::Integrity);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::{DecimalU64, ObjectKind, digest};
    struct Records(BTreeMap<ObjectRef, Value>);
    fn reference(value: &Value) -> ObjectRef {
        let bytes = photara_core::canonical_json(value).unwrap();
        ObjectRef {
            kind: ObjectKind::Json,
            byte_length: DecimalU64::parse(&bytes.len().to_string()).unwrap(),
            sha256: digest(&bytes),
        }
    }
    impl LogicalRecords for Records {
        fn resolve(
            &self,
            object: &ObjectRef,
            membership: Membership,
        ) -> Result<Value, PackageError> {
            if membership != Membership::Semantic {
                return Err(PackageError::Record);
            }
            self.0.get(object).cloned().ok_or(PackageError::Integrity)
        }
    }
    #[test]
    fn original_graph_receipts_prefixes_and_reciprocal_indexes_are_preserved() {
        let corpus: Value = serde_json::from_str(include_str!(
            "../../../../../docs/architecture/proposals/ps2/integrated/linked.json"
        ))
        .unwrap();
        let records = Records(
            corpus["records"]
                .as_object()
                .unwrap()
                .values()
                .map(|row| {
                    let value: Value =
                        serde_json::from_str(row["canonical"].as_str().unwrap()).unwrap();
                    (reference(&value), value)
                })
                .collect(),
        );
        let limits = TreeLimits {
            max_depth: 16,
            max_pages: 512,
            max_entries: 4096,
            max_leaf_entries: 64,
            max_branch_children: 8,
        };
        let mut found = 0;
        for state in records
            .0
            .values()
            .filter(|v| v["schema"]["id"] == "photara.package.state-root")
        {
            let identity = SelectionIdentity {
                project: nonnil(&state["project_id"]).unwrap(),
                library: nonnil(&state["library_id"]).unwrap(),
                bootstrap_sha256: sha(&state["bootstrap_sha256"]).unwrap(),
            };
            let proof = audit_operations(&records, state, &identity, limits).unwrap();
            assert!(!proof.receipts().is_empty());
            let mut wrong = state.clone();
            wrong["journal_inclusion"]["prefix_sha256"] = json!("0".repeat(64));
            assert!(audit_operations(&records, &wrong, &identity, limits).is_err());
            let mut wrong = state.clone();
            wrong["authored_revision"] = json!("999");
            assert!(audit_operations(&records, &wrong, &identity, limits).is_err());
            found += 1;
        }
        assert!(found > 0);
    }
}
