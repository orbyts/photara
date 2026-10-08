//! Core-derived local accepted prefix. No native authority or acknowledgement.
use super::super::{Limits, PlannerInputs, VerifiedOriginal, core};
use super::{
    BTreeMap, BTreeSet, Journal, JournalAppend, JournalLimits, JournalRecord,
    JournalRecordIdentity, JsonLimits, PackageError, RepeatablePlan, Sha256Hex, Value,
    VerifiedJournal, check, digest, fields, id, intent_body, json, n, parse_canonical_json, plan,
    reference, wire,
};
use crate::package::planning::{
    self, AuthoredCommand, CheckpointIds, MutationRequest, PackageNamingPolicy, PlanOutcome,
    PlanRequest, VerifiedClosure, WriteId,
};
use photara_core::{NodeDefinitionRegistry, ValueTypeRegistry, creation::PackageExtension};
type Objects = BTreeMap<wire::Key, Value>;

fn checked<T>(result: wire::Result<T>) -> Result<T, PackageError> {
    result.map_err(|_| PackageError::Integrity)
}
pub(crate) fn owner(value: &Value) -> Result<(), PackageError> {
    fields(
        value,
        &["attachment_id", "attachment_generation", "principal"],
    )?;
    id(&value["attachment_id"])?;
    check(n(&value["attachment_generation"])? > 0)?;
    let principal = &value["principal"];
    match principal["kind"].as_str() {
        Some("account") => {
            fields(principal, &["kind", "account_id"])?;
            id(&principal["account_id"])?;
        }
        Some("local-controller") => {
            fields(principal, &["kind", "principal_id"])?;
            id(&principal["principal_id"])?;
        }
        _ => return Err(PackageError::UnsupportedVersion),
    }
    Ok(())
}
pub(crate) fn link(value: &Value) -> Result<(), PackageError> {
    fields(value, &["record_id", "record_checksum"])?;
    id(&value["record_id"])?;
    digest(&value["record_checksum"])
}
fn target(value: &Value) -> Result<(), PackageError> {
    fields(value, &["kind", "operation_id", "mutation"])?;
    check(value["kind"] == "operation")?;
    id(&value["operation_id"])?;
    link(&value["mutation"])
}
pub(crate) fn coordinate(value: &Value) -> Result<(), PackageError> {
    fields(value, &["revision", "authored_digest", "graphs"])?;
    n(&value["revision"])?;
    digest(&value["authored_digest"])?;
    for (graph, coordinate) in value["graphs"].as_object().ok_or(PackageError::Record)? {
        id(&json!(graph))?;
        fields(
            coordinate,
            &[
                "revision",
                "semantic_digest",
                "payload_digest",
                "envelope_digest",
            ],
        )?;
        n(&coordinate["revision"])?;
        for key in ["semantic_digest", "payload_digest", "envelope_digest"] {
            digest(&coordinate[key])?;
        }
    }
    Ok(())
}
fn refs(values: &Value) -> Result<(), PackageError> {
    let mut prior = None;
    for row in values.as_array().ok_or(PackageError::Record)? {
        fields(row, &["reference", "value"])?;
        check(row["value"].is_object() && row["reference"] == reference(&row["value"]))?;
        let key = checked(wire::key(&row["reference"]))?;
        check(prior.as_ref().is_none_or(|p| p < &key))?;
        prior = Some(key);
    }
    Ok(())
}
fn head(value: &Value, project: &Value) -> Result<(), PackageError> {
    fields(
        value,
        &["schema", "project_id", "commit_id", "commit_sha256"],
    )?;
    check(
        value["schema"] == json!({"id":"photara.package.head","version":1})
            && &value["project_id"] == project,
    )?;
    id(&value["commit_id"])?;
    digest(&value["commit_sha256"])
}
fn intent_receipt(body: &Value, header: &Value) -> Result<(), PackageError> {
    let intent = &body["semantic_intent"];
    fields(
        intent,
        &[
            "bootstrap_sha256",
            "boundary",
            "command",
            "domain",
            "expected",
            "library_id",
            "operation_id",
            "project_id",
            "undo_group_id",
            "updated_at",
            "version",
        ],
    )?;
    check(
        intent["domain"] == "photara.package.operation-intent.v1"
            && intent["version"] == 1
            && intent["boundary"] == "single"
            && intent["undo_group_id"].is_null(),
    )?;
    coordinate(&intent["expected"])?;
    for key in ["project_id", "library_id", "bootstrap_sha256"] {
        check(intent[key] == header[key])?;
    }
    let identity = crate::package::v1_3::SelectionIdentity {
        project: id(&header["project_id"])?,
        library: id(&header["library_id"])?,
        bootstrap_sha256: Sha256Hex::parse(
            header["bootstrap_sha256"]
                .as_str()
                .ok_or(PackageError::Record)?,
        )?,
    };
    let receipt = &body["operation_receipt"];
    crate::package::v1_3::operations::validate_receipt(receipt, &identity)?;
    check(
        receipt["operation_id"] == intent["operation_id"]
            && receipt["request_sha256"] == wire::hash(&wire::encode(intent))
            && receipt["journal_id"] == header["journal_id"]
            && receipt["undo_group_id"].is_null()
            && body["accepted_frame"] == plan::journal_frame(receipt),
    )
}
#[expect(
    clippy::too_many_lines,
    reason = "Exact four-kind schema dispatch kept together"
)]
pub(super) fn shape(record: &Value, header: &Value) -> Result<(), PackageError> {
    let body = &record["body"];
    owner(&body["owner"])?;
    match record["kind"].as_str() {
        Some("Mutation") => {
            fields(
                body,
                &[
                    "semantic_intent",
                    "operation_receipt",
                    "accepted_frame",
                    "owner",
                    "base_head",
                    "result",
                    "patch",
                    "action",
                    "redo_invalidated",
                ],
            )?;
            intent_receipt(body, header)?;
            head(&body["base_head"], &header["project_id"])?;
            coordinate(&body["result"])?;
            let patch = &body["patch"];
            fields(patch, &["before", "after", "removed", "added"])?;
            checked(wire::key(&patch["before"]))?;
            checked(wire::key(&patch["after"]))?;
            refs(&patch["removed"])?;
            refs(&patch["added"])?;
            let action = &body["action"];
            match action["kind"].as_str() {
                Some("edit") => fields(action, &["kind"])?,
                Some("undo") => {
                    fields(action, &["kind", "target"])?;
                    target(&action["target"])?;
                }
                Some("redo") => {
                    fields(action, &["kind", "target", "undo"])?;
                    target(&action["target"])?;
                    target(&action["undo"])?;
                }
                _ => return Err(PackageError::UnsupportedVersion),
            }
            let mut prior = None;
            for value in body["redo_invalidated"]
                .as_array()
                .ok_or(PackageError::Record)?
            {
                let next = id(value)?;
                check(prior.is_none_or(|p| p < next))?;
                prior = Some(next);
            }
            Ok(())
        }
        Some("UndoBoundary") => {
            fields(body, &["owner", "target", "reason"])?;
            target(&body["target"])?;
            check(body["reason"] == "single-operation")
        }
        Some("SessionBarrier") => {
            fields(
                body,
                &["owner", "reason", "through", "accepted_frame", "target"],
            )?;
            check(matches!(
                body["reason"].as_str(),
                Some("flush" | "save-now" | "close" | "switch" | "sleep" | "terminate")
            ))?;
            check(body["through"].is_null() == body["accepted_frame"].is_null())?;
            if !body["through"].is_null() {
                link(&body["through"])?;
            }
            coordinate(&body["target"])
        }
        Some("RecoveryDecision") => {
            fields(body, &["owner", "verified_through", "observed", "outcome"])?;
            if !body["verified_through"].is_null() {
                link(&body["verified_through"])?;
            }
            if !body["observed"].is_null() {
                fields(&body["observed"], &["head", "commit"])?;
                head(&body["observed"]["head"], &header["project_id"])?;
                check(
                    body["observed"]["head"]["commit_sha256"]
                        == wire::hash(&wire::encode(&body["observed"]["commit"])),
                )?;
            }
            let outcome = &body["outcome"];
            match outcome["kind"].as_str() {
                Some("prefix-replay") => {
                    fields(outcome, &["kind", "through_mutation", "result"])?;
                    if !outcome["through_mutation"].is_null() {
                        link(&outcome["through_mutation"])?;
                    }
                    coordinate(&outcome["result"])
                }
                Some("checkpoint-reconciled") => {
                    fields(outcome, &["kind", "intent", "receipt"])?;
                    link(&outcome["intent"])?;
                    link(&outcome["receipt"])
                }
                Some("conflict-preserved") => {
                    fields(outcome, &["kind", "preserved_branch_id", "reason"])?;
                    id(&outcome["preserved_branch_id"])?;
                    check(matches!(
                        outcome["reason"].as_str(),
                        Some(
                            "unrelated-head" | "rollback" | "missing-package" | "identity-changed"
                        )
                    ))
                }
                _ => Err(PackageError::UnsupportedVersion),
            }
        }
        _ => Err(PackageError::UnsupportedVersion),
    }
}
fn record_link(record: &JournalRecord) -> Value {
    json!({"record_id":record.value["record_id"],"record_checksum":record.checksum()})
}
fn operation_target(record: &JournalRecord) -> Value {
    json!({"kind":"operation","operation_id":record.value["body"]["operation_receipt"]["operation_id"],"mutation":record_link(record)})
}
fn objects(logical: &VerifiedClosure) -> Result<(Value, Objects), PackageError> {
    let files = logical.files();
    let head = parse_canonical_json(files.get("HEAD.json")?, files.limits().json)?;
    let commit = parse_canonical_json(
        files.get(&format!(
            "commits/{}.json",
            head["commit_id"].as_str().ok_or(PackageError::Record)?
        ))?,
        files.limits().json,
    )?;
    let root = commit["authored"].clone();
    let mut pending = vec![root.clone()];
    let mut objects = Objects::new();
    while let Some(reference) = pending.pop() {
        let key = checked(wire::key(&reference))?;
        if objects.contains_key(&key) {
            continue;
        }
        let value = parse_canonical_json(
            files.get(&format!("objects/json/sha256/{}.json", key.0))?,
            files.limits().json,
        )?;
        let mut edges = Vec::new();
        crate::package::v1_1::references(&value, &mut edges)?;
        pending.extend(
            edges
                .into_iter()
                .map(|r| serde_json::to_value(r).expect("reference")),
        );
        objects.insert(key, value);
    }
    Ok((root, objects))
}
fn difference(left: &Objects, right: &Objects) -> Vec<Value> {
    left.iter()
        .filter(|(key, _)| !right.contains_key(*key))
        .map(|(_, value)| json!({"reference":reference(value),"value":value}))
        .collect()
}
struct Budget(usize);
impl std::io::Write for Budget {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self
            .0
            .checked_sub(bytes.len())
            .ok_or_else(|| std::io::Error::other("JSON budget"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn bound(value: &Value, limits: JsonLimits) -> Result<(), PackageError> {
    let mut pending = vec![(value, 0usize)];
    let mut nodes = 0usize;
    while let Some((value, depth)) = pending.pop() {
        nodes = nodes.checked_add(1).ok_or(PackageError::Limit)?;
        if depth > limits.max_depth || nodes > limits.max_bytes {
            return Err(PackageError::Limit);
        }
        match value {
            Value::Array(values) => {
                if values.len() > limits.max_array_elements
                    || values.len()
                        > limits
                            .max_bytes
                            .saturating_sub(nodes)
                            .saturating_sub(pending.len())
                {
                    return Err(PackageError::Limit);
                }
                pending.extend(values.iter().map(|v| (v, depth + 1)));
            }
            Value::Object(values) => {
                if values.len() > limits.max_members
                    || values.len()
                        > limits
                            .max_bytes
                            .saturating_sub(nodes)
                            .saturating_sub(pending.len())
                {
                    return Err(PackageError::Limit);
                }
                pending.extend(values.values().map(|v| (v, depth + 1)));
            }
            _ => {}
        }
    }
    serde_json::to_writer(Budget(limits.max_bytes), value).map_err(|_| PackageError::Limit)
}
fn replay(
    logical: &VerifiedClosure,
    intent: &Value,
    receipt: Option<&Value>,
) -> Result<VerifiedClosure, PackageError> {
    bound(intent, logical.files().limits().json)?;
    check(
        serde_json::to_value(logical.coordinate()).map_err(|_| PackageError::Record)?
            == intent["expected"],
    )?;
    fields(&intent["command"], &["kind", "envelope"])?;
    check(intent["command"]["kind"] == "graph")?;
    let envelope: photara_core::GraphCommandEnvelope =
        serde_json::from_value(intent["command"]["envelope"].clone())
            .map_err(|_| PackageError::Record)?;
    // No typed decoder may discard unknown command bytes.
    check(
        serde_json::to_value(&envelope).map_err(|_| PackageError::Record)?
            == intent["command"]["envelope"],
    )?;
    let request = MutationRequest {
        version: 1,
        operation_id: serde_json::from_value(intent["operation_id"].clone())
            .map_err(|_| PackageError::Record)?,
        expected: logical.coordinate().clone(),
        command: AuthoredCommand::Graph {
            envelope: Box::new(envelope),
        },
        updated_at: intent["updated_at"]
            .as_str()
            .ok_or(PackageError::Record)?
            .into(),
    };
    let extension = PackageExtension::try_from("ps3preparation".to_owned())
        .map_err(|_| PackageError::Record)?;
    let naming = PackageNamingPolicy {
        write_extension: extension.clone(),
        legacy_read_extensions: vec![],
    };
    let result = planning::plan(
        logical,
        PlanRequest {
            mutation: &request,
            expected_head: logical.token(),
            ids: CheckpointIds {
                write_id: WriteId::parse(
                    intent["operation_id"]
                        .as_str()
                        .ok_or(PackageError::Record)?,
                )
                .map_err(|_| PackageError::Record)?,
                commit_id: serde_json::from_value(intent["operation_id"].clone())
                    .map_err(|_| PackageError::Record)?,
            },
            naming: &naming,
            observed_extension: &extension,
        },
        &NodeDefinitionRegistry::default(),
        &ValueTypeRegistry::default(),
    )
    .map_err(|_| PackageError::Integrity)?;
    let (candidate, prepared) = match &result {
        PlanOutcome::Checkpoint(plan) => (plan.candidate(), plan.receipt()),
        PlanOutcome::Unchanged(receipt) => (logical, receipt),
    };
    if let Some(receipt) = receipt {
        for (expected, key) in [(prepared.before(), "before"), (prepared.after(), "after")] {
            let value = serde_json::to_value(expected).map_err(|_| PackageError::Record)?;
            check(
                receipt[key]
                    == json!({"revision":value["revision"],"digest":value["authored_digest"]}),
            )?;
        }
    }
    // Reopen the complete Core candidate; no caller patch bytes influence it.
    VerifiedClosure::verify(
        candidate.files().clone(),
        planning::IncarnationId::parse(
            intent["operation_id"]
                .as_str()
                .ok_or(PackageError::Record)?,
        )
        .map_err(|_| PackageError::Record)?,
    )
    .map_err(|_| PackageError::Integrity)
}
struct MutationEvidence {
    record: JournalRecord,
    before: Objects,
    after: Objects,
    accepted: Value,
    journal_inclusion: Value,
    undone_by: Option<String>,
    invalidated: bool,
}
/// Fully replayed local prefix; provenance and ownership remain historical data.
/// A separately qualified coordinator must authorize and barrier any append.
pub struct VerifiedSession<'a> {
    journal: &'a Journal,
    logical: VerifiedClosure,
    head: Value,
    accepted: Value,
    journal_inclusion: Value,
    ordinal: u64,
    sequence: u64,
    mutations: BTreeMap<String, MutationEvidence>,
    historical: BTreeSet<String>,
    order: Vec<String>,
    checkpointed: usize,
    completed: Option<Value>,
    checkpoint_target: Value,
}
impl<'a> VerifiedJournal<'a> {
    /// Replays local records from an independently verified header base and
    /// independently verified checkpoint history. Does not confer durability.
    /// # Errors
    /// Refuses false patches, unsupported inverses, discontinuous inclusion,
    /// forged lifecycle evidence or a different base.
    #[expect(
        clippy::too_many_lines,
        reason = "Ordered local and independent checkpoint prefix validation kept together"
    )]
    pub fn verify_session(
        &self,
        base: &VerifiedOriginal,
        inputs: &PlannerInputs,
        limits: Limits,
    ) -> Result<VerifiedSession<'a>, PackageError> {
        check(
            base.package.head == self.journal.header["base_head"]
                && base.package.commit["package_revision"]
                    == self.journal.header["base_package_revision"],
        )?;
        let (logical, _) = checked(core::original_closure(
            &base.package,
            &base.proof,
            inputs,
            limits,
        ))?;
        let active = base
            .proof
            .roles
            .get("active")
            .ok_or(PackageError::Integrity)?;
        let checkpoint_target =
            json!({"coordinate":logical.coordinate(),"through":null,"accepted_frame":null});
        let mut session = VerifiedSession {
            journal: self.journal,
            logical,
            head: base.package.head.clone(),
            accepted: active.state["accepted"].clone(),
            journal_inclusion: active.state["journal_inclusion"].clone(),
            ordinal: n(&active.state["accepted"]["through_ordinal"])?,
            sequence: n(&active.state["journal_inclusion"]["through_sequence"])?,
            mutations: BTreeMap::new(),
            historical: active
                .receipts
                .iter()
                .map(|r| {
                    r["operation_id"]
                        .as_str()
                        .map(str::to_owned)
                        .ok_or(PackageError::Record)
                })
                .collect::<Result<_, _>>()?,
            order: Vec::new(),
            checkpointed: 0,
            completed: None,
            checkpoint_target,
        };
        for (index, record) in self.journal.records.iter().enumerate() {
            match record.value["kind"].as_str() {
                Some("Mutation") => session.apply_mutation(record)?,
                Some("CheckpointReceipt") => {
                    let intent_id = record.value["body"]["intent_record_id"]
                        .as_str()
                        .ok_or(PackageError::Record)?;
                    let plan = self.plans.get(intent_id).ok_or(PackageError::Integrity)?;
                    let operation = plan.prepared.receipt["operation_id"]
                        .as_str()
                        .ok_or(PackageError::Record)?;
                    let state_ref = &plan.stages[6].commit["root_set"]["active"];
                    // StateRoots are packed; the authenticated finalizer carries
                    // the exact selected active value, not a loose StateRoot.
                    let selected_state = plan.stages[6]
                        .loose
                        .values()
                        .map(|bytes| parse_canonical_json(bytes, limits.semantic.json))
                        .collect::<Result<Vec<_>, _>>()?
                        .into_iter()
                        .find_map(|value| {
                            let state = &value["target_projection"]["active_state"];
                            (value["schema"]["id"] == "photara.storage.finalization-control"
                                && value["original"] == reference(&plan.original)
                                && reference(state) == *state_ref)
                                .then(|| state.clone())
                        })
                        .ok_or(PackageError::Integrity)?;
                    if let Some(mutation) = session.mutations.get(operation) {
                        check(
                            selected_state["accepted"] == mutation.accepted
                                && selected_state["journal_inclusion"]
                                    == mutation.journal_inclusion,
                        )?;
                        check(
                            session
                                .order
                                .get(session.checkpointed)
                                .is_some_and(|id| id == operation)
                                && mutation.record.value["body"]["operation_receipt"]
                                    == plan.prepared.receipt
                                && mutation.record.value["body"]["semantic_intent"]
                                    == intent_body(plan)["semantic_intent"],
                        )?;
                        session.checkpoint_target = json!({"coordinate":mutation.record.value["body"]["result"],"through":record_link(&mutation.record),"accepted_frame":mutation.record.value["body"]["accepted_frame"]});
                        session.checkpointed += 1;
                    } else {
                        // Existing checkpoint-only streams advance the inherited
                        // coordinate without inventing local Mutation ownership.
                        check(session.order.is_empty())?;
                        check(session.historical.insert(operation.to_owned()))?;
                        let body = intent_body(plan);
                        session.logical = replay(
                            &session.logical,
                            &body["semantic_intent"],
                            Some(&body["operation_receipt"]),
                        )?;
                        session.ordinal = n(&plan.prepared.receipt["acceptance_ordinal"])?;
                        session.sequence = n(&plan.prepared.receipt["journal_sequence"])?;
                        session.advance_prefix(&plan.prepared.receipt);
                        check(
                            selected_state["accepted"] == session.accepted
                                && selected_state["journal_inclusion"] == session.journal_inclusion,
                        )?;
                        session.checkpoint_target = json!({"coordinate":session.coordinate(),"through":null,"accepted_frame":null});
                    }
                    session.head = plan.stages[6].head.clone();
                    session.completed = Some(record_link(record));
                }
                Some("CheckpointIntent") => {
                    // Initial checkpoint-only history preserves its older logical
                    // sequence variants; PS3 preludes after a Mutation use the
                    // exact contiguous local sequence.
                    if !session.order.is_empty() {
                        session.validate_checkpoint_intent(&record.value["body"], false)?;
                    }
                }
                _ => session.validate_aux(record, index)?,
            }
        }
        Ok(session)
    }
}
impl VerifiedSession<'_> {
    #[must_use]
    /// # Panics
    /// Only if the internally verified coordinate ceases to be JSON serializable.
    pub fn coordinate(&self) -> Value {
        serde_json::to_value(self.logical.coordinate()).expect("coordinate")
    }
    #[must_use]
    pub fn through(&self) -> Option<Value> {
        self.order
            .last()
            .map(|id| record_link(&self.mutations[id].record))
    }
    #[must_use]
    pub fn accepted_frame(&self) -> Option<Value> {
        self.order
            .last()
            .map(|id| self.mutations[id].record.value["body"]["accepted_frame"].clone())
    }
    #[must_use]
    pub fn checkpoint_target(&self) -> &Value {
        &self.checkpoint_target
    }
    #[must_use]
    pub fn completed_checkpoint(&self) -> Option<&Value> {
        self.completed.as_ref()
    }
    #[must_use]
    pub fn base_head(&self) -> &Value {
        &self.head
    }
    #[must_use]
    pub fn mutation(&self, operation: &str) -> Option<&Value> {
        self.mutations
            .get(operation)
            .map(|m| &m.record.value["body"])
    }
    #[must_use]
    pub fn undo_targets(&self, owner: &Value) -> Vec<Value> {
        self.order
            .iter()
            .rev()
            .filter_map(|id| {
                let m = &self.mutations[id];
                (m.record.value["body"]["owner"] == *owner
                    && m.record.value["body"]["action"]["kind"] == "edit"
                    && m.undone_by.is_none()
                    && !m.invalidated)
                    .then(|| operation_target(&m.record))
            })
            .collect()
    }
    #[must_use]
    pub fn redo_targets(&self, owner: &Value) -> Vec<Value> {
        self.order.iter().rev().filter_map(|id| { let m=&self.mutations[id]; (m.record.value["body"]["owner"]==*owner && !m.invalidated).then(||m.undone_by.as_ref().map(|undo|json!({"target":operation_target(&m.record),"undo":operation_target(&self.mutations[undo].record)}))).flatten() }).collect()
    }
    fn resolve_target(
        &self,
        target: &Value,
        owner: &Value,
    ) -> Result<&MutationEvidence, PackageError> {
        let operation = target["operation_id"]
            .as_str()
            .ok_or(PackageError::Record)?;
        let mutation = self
            .mutations
            .get(operation)
            .ok_or(PackageError::Integrity)?;
        check(
            operation_target(&mutation.record) == *target
                && mutation.record.value["body"]["owner"] == *owner,
        )?;
        Ok(mutation)
    }
    fn invalidated(&self, owner: &Value, action: &Value) -> Vec<Value> {
        if action["kind"] != "edit" {
            return vec![];
        }
        self.mutations
            .iter()
            .filter(|(_, m)| {
                m.record.value["body"]["owner"] == *owner && m.undone_by.is_some() && !m.invalidated
            })
            .map(|(id, _)| json!(id))
            .collect()
    }
    fn validate_action(
        &self,
        intent: &Value,
        owner: &Value,
        action: &Value,
    ) -> Result<(), PackageError> {
        if action["kind"] == "edit" {
            return Ok(());
        }
        let target = self.resolve_target(&action["target"], owner)?;
        check(target.record.value["body"]["action"]["kind"] == "edit" && !target.invalidated)?;
        let redo = action["kind"] == "redo";
        if redo {
            let undo = self.resolve_target(&action["undo"], owner)?;
            check(
                target.undone_by.as_deref()
                    == undo.record.value["body"]["operation_receipt"]["operation_id"].as_str()
                    && undo.record.value["body"]["action"]["kind"] == "undo"
                    && undo.record.value["body"]["action"]["target"] == action["target"],
            )?;
        } else {
            check(target.undone_by.is_none())?;
        }
        let (_, current) = objects(&self.logical)?;
        let expected = inverse(
            &target.record.value["body"]["semantic_intent"]["command"],
            &target.before,
            &target.after,
            &current,
            redo,
        )?;
        check(
            intent["command"]["kind"] == expected["kind"]
                && intent["command"]["envelope"]["graph_id"] == expected["envelope"]["graph_id"]
                && intent["command"]["envelope"]["command"] == expected["envelope"]["command"],
        )
    }
    /// Derives supported undo/redo command values. Caller supplies a fresh
    /// operation/command ID, current expected coordinate and timestamp afterward.
    /// # Errors
    /// Refuses wrong owners, stale affected values and unexpressible inverses.
    pub fn inverse_command(&self, owner: &Value, action: &Value) -> Result<Value, PackageError> {
        let target = self.resolve_target(&action["target"], owner)?;
        let (_, current) = objects(&self.logical)?;
        inverse(
            &target.record.value["body"]["semantic_intent"]["command"],
            &target.before,
            &target.after,
            &current,
            action["kind"] == "redo",
        )
    }
    /// Next logical receipt coordinates; outer record sequences are independent.
    /// # Errors
    /// Refuses exhausted portable ordinals or logical sequences.
    pub fn next_receipt_coordinates(&self) -> Result<(u64, u64), PackageError> {
        Ok((
            self.ordinal.checked_add(1).ok_or(PackageError::Limit)?,
            self.sequence.checked_add(1).ok_or(PackageError::Limit)?,
        ))
    }
    /// Returns authenticated current graph envelopes for disposable editor views.
    /// # Errors
    /// Refuses incomplete schema closure.
    pub fn graph_objects(&self) -> Result<Vec<Value>, PackageError> {
        let (_, objects) = objects(&self.logical)?;
        Ok(objects
            .into_values()
            .filter(|v| v["graph_id"].is_string() && v["graph"].is_object())
            .collect())
    }
    /// Computes the complete Core result before receipt metadata is assigned.
    /// # Errors
    /// Refuses stale coordinates, unsupported commands and invalid authored state.
    pub fn preview(&self, intent: &Value) -> Result<Value, PackageError> {
        let next = replay(&self.logical, intent, None)?;
        serde_json::to_value(next.coordinate()).map_err(|_| PackageError::Record)
    }
    fn mutation_body(
        &self,
        intent: &Value,
        receipt: &Value,
        owner: &Value,
        action: &Value,
    ) -> Result<(Value, VerifiedClosure, Objects, Objects), PackageError> {
        check(
            !self.historical.contains(
                intent["operation_id"]
                    .as_str()
                    .ok_or(PackageError::Record)?,
            ),
        )?;
        let preliminary = json!({"semantic_intent":intent,"operation_receipt":receipt,"accepted_frame":plan::journal_frame(receipt)});
        intent_receipt(&preliminary, &self.journal.header)?;
        check(
            n(&receipt["acceptance_ordinal"])?
                == self.ordinal.checked_add(1).ok_or(PackageError::Limit)?
                && n(&receipt["journal_sequence"])?
                    == self.sequence.checked_add(1).ok_or(PackageError::Limit)?,
        )?;
        check(owner["principal"] == receipt["provenance"]["principal"])?;
        self.validate_action(intent, owner, action)?;
        let next = replay(&self.logical, intent, Some(receipt))?;
        let (before_root, before) = objects(&self.logical)?;
        let (after_root, after) = objects(&next)?;
        let result = serde_json::to_value(next.coordinate()).map_err(|_| PackageError::Record)?;
        let patch = json!({"before":before_root,"after":after_root,"removed":difference(&before,&after),"added":difference(&after,&before)});
        Ok((
            json!({"semantic_intent":intent,"operation_receipt":receipt,"accepted_frame":plan::journal_frame(receipt),"owner":owner,"base_head":self.head,"result":result,"patch":patch,"action":action,"redo_invalidated":self.invalidated(owner,action)}),
            next,
            before,
            after,
        ))
    }
    fn advance_prefix(&mut self, receipt: &Value) {
        self.accepted = json!({"through_ordinal":receipt["acceptance_ordinal"],"prefix_sha256":wire::hash(&wire::encode(&json!({"domain":"photara.package.accepted-prefix-link.v1","previous_sha256":self.accepted["prefix_sha256"],"acceptance_ordinal":receipt["acceptance_ordinal"],"operation_id":receipt["operation_id"],"request_sha256":receipt["request_sha256"],"receipt_sha256":reference(receipt)["sha256"]})))});
        self.journal_inclusion = json!({"journal_id":receipt["journal_id"],"through_sequence":receipt["journal_sequence"],"prefix_sha256":wire::hash(&wire::encode(&json!({"domain":"photara.package.journal-prefix-link.v1","previous_sha256":self.journal_inclusion["prefix_sha256"],"frame_sha256":wire::hash(&wire::encode(&plan::journal_frame(receipt)))}))),"resulting_authored_revision":receipt["after"]["revision"],"resulting_authored_sha256":receipt["after"]["digest"]});
    }
    fn apply_mutation(&mut self, record: &JournalRecord) -> Result<(), PackageError> {
        let body = &record.value["body"];
        let operation = body["operation_receipt"]["operation_id"]
            .as_str()
            .ok_or(PackageError::Record)?
            .to_owned();
        check(!self.mutations.contains_key(&operation))?;
        let (expected, next, before, after) = self.mutation_body(
            &body["semantic_intent"],
            &body["operation_receipt"],
            &body["owner"],
            &body["action"],
        )?;
        check(expected == *body)?;
        for id in body["redo_invalidated"]
            .as_array()
            .ok_or(PackageError::Record)?
        {
            self.mutations
                .get_mut(id.as_str().ok_or(PackageError::Record)?)
                .ok_or(PackageError::Integrity)?
                .invalidated = true;
        }
        if matches!(body["action"]["kind"].as_str(), Some("undo" | "redo")) {
            let target = body["action"]["target"]["operation_id"]
                .as_str()
                .ok_or(PackageError::Record)?;
            self.mutations
                .get_mut(target)
                .ok_or(PackageError::Integrity)?
                .undone_by = if body["action"]["kind"] == "undo" {
                Some(operation.clone())
            } else {
                None
            };
        }
        self.logical = next;
        self.ordinal = n(&body["operation_receipt"]["acceptance_ordinal"])?;
        self.sequence = n(&body["operation_receipt"]["journal_sequence"])?;
        self.advance_prefix(&body["operation_receipt"]);
        self.order.push(operation.clone());
        self.mutations.insert(
            operation,
            MutationEvidence {
                record: record.clone(),
                before,
                after,
                accepted: self.accepted.clone(),
                journal_inclusion: self.journal_inclusion.clone(),
                undone_by: None,
                invalidated: false,
            },
        );
        Ok(())
    }
    /// Prepares a Core-derived mutation or its exact original retry. No caller
    /// patch/result/redo invalidation data can replace derived evidence.
    /// # Errors
    /// Refuses conflicting retries, unsupported commands, stale undo and bounds.
    pub fn prepare_mutation(
        &self,
        intent: &Value,
        receipt: &Value,
        owner: &Value,
        action: &Value,
        identity: JournalRecordIdentity,
        limits: JournalLimits,
    ) -> Result<JournalAppend, PackageError> {
        for value in [intent, receipt, owner, action] {
            bound(value, limits.json)?;
        }
        let body = if let Some(prior) = self.mutation(
            intent["operation_id"]
                .as_str()
                .ok_or(PackageError::Record)?,
        ) {
            check(
                prior["semantic_intent"] == *intent
                    && prior["operation_receipt"] == *receipt
                    && prior["owner"] == *owner
                    && prior["action"] == *action,
            )?;
            prior.clone()
        } else {
            self.mutation_body(intent, receipt, owner, action)?.0
        };
        self.prepare_local("Mutation", &body, identity, limits)
    }
    fn prepare_local(
        &self,
        kind: &str,
        body: &Value,
        identity: JournalRecordIdentity,
        limits: JournalLimits,
    ) -> Result<JournalAppend, PackageError> {
        VerifiedJournal {
            journal: self.journal,
            plans: BTreeMap::new(),
        }
        .prepare(kind, body, identity, limits)
    }
    /// Binds a checkpoint to the actual next accepted Mutation. Legacy checkpoint
    /// preparation cannot bypass local prefix verification once Mutations exist.
    /// # Errors
    /// Refuses a different stream, skipped Mutation or changed nested evidence.
    pub fn prepare_checkpoint_intent(
        &self,
        verified: &VerifiedJournal<'_>,
        plan: &RepeatablePlan,
        identity: JournalRecordIdentity,
        limits: JournalLimits,
    ) -> Result<JournalAppend, PackageError> {
        check(std::ptr::eq(self.journal, verified.journal))?;
        let body = intent_body(plan);
        let existing = verified.records().iter().any(|r| {
            r.value["kind"] == "CheckpointIntent"
                && r.value["body"]["operation_receipt"]["operation_id"]
                    == plan.prepared.receipt["operation_id"]
        });
        self.validate_checkpoint_intent(&body, existing)?;
        verified.prepare_intent_inner(plan, identity, limits)
    }
    fn validate_checkpoint_intent(&self, body: &Value, existing: bool) -> Result<(), PackageError> {
        let operation = body["operation_receipt"]["operation_id"]
            .as_str()
            .ok_or(PackageError::Record)?;
        if let Some(mutation) = self.mutations.get(operation) {
            check(
                existing
                    || self
                        .order
                        .get(self.checkpointed)
                        .is_some_and(|id| id == operation),
            )?;
            check(
                mutation.record.value["body"]["semantic_intent"] == body["semantic_intent"]
                    && mutation.record.value["body"]["operation_receipt"]
                        == body["operation_receipt"]
                    && mutation.record.value["body"]["accepted_frame"] == body["accepted_frame"],
            )
        } else {
            check(self.checkpointed == self.order.len())?;
            intent_receipt(body, &self.journal.header)?;
            let (ordinal, sequence) = self.next_receipt_coordinates()?;
            check(
                n(&body["operation_receipt"]["acceptance_ordinal"])? == ordinal
                    && n(&body["operation_receipt"]["journal_sequence"])? == sequence,
            )?;
            replay(
                &self.logical,
                &body["semantic_intent"],
                Some(&body["operation_receipt"]),
            )?;
            Ok(())
        }
    }
    /// Captures the current finite accepted target, not a Saved acknowledgement.
    /// # Errors
    /// Refuses invalid owners/reasons, damaged tails or caller budgets.
    pub fn prepare_barrier(
        &self,
        owner: &Value,
        reason: &str,
        identity: JournalRecordIdentity,
        limits: JournalLimits,
    ) -> Result<JournalAppend, PackageError> {
        self.prepare_local("SessionBarrier",&json!({"owner":owner,"reason":reason,"through":self.through(),"accepted_frame":self.accepted_frame(),"target":self.coordinate()}),identity,limits)
    }
    /// Optional redundant single-operation marker; never creates undo groups.
    /// # Errors
    /// Refuses a missing or another owner's exact operation.
    pub fn prepare_boundary(
        &self,
        owner: &Value,
        target: &Value,
        identity: JournalRecordIdentity,
        limits: JournalLimits,
    ) -> Result<JournalAppend, PackageError> {
        self.resolve_target(target, owner)?;
        self.prepare_local(
            "UndoBoundary",
            &json!({"owner":owner,"target":target,"reason":"single-operation"}),
            identity,
            limits,
        )
    }
    fn earlier(&self, link: &Value, index: usize) -> Result<&JournalRecord, PackageError> {
        self.journal.records[..index]
            .iter()
            .find(|r| record_link(r) == *link)
            .ok_or(PackageError::Integrity)
    }
    fn validate_aux(&self, record: &JournalRecord, index: usize) -> Result<(), PackageError> {
        let body = &record.value["body"];
        match record.value["kind"].as_str() {
            Some("UndoBoundary") => {
                self.resolve_target(&body["target"], &body["owner"])?;
                Ok(())
            }
            Some("SessionBarrier") => check(
                body["through"] == json!(self.through())
                    && body["accepted_frame"] == json!(self.accepted_frame())
                    && body["target"] == self.coordinate(),
            ),
            Some("RecoveryDecision") => {
                check(
                    body["verified_through"]
                        == self
                            .journal
                            .records
                            .get(index.wrapping_sub(1))
                            .map_or(Value::Null, record_link),
                )?;
                let outcome = &body["outcome"];
                match outcome["kind"].as_str() {
                    Some("prefix-replay") => check(
                        outcome["through_mutation"] == json!(self.through())
                            && outcome["result"] == self.coordinate()
                            && (body["observed"].is_null()
                                || body["observed"]["head"] == self.head),
                    ),
                    Some("checkpoint-reconciled") => {
                        let intent = self.earlier(&outcome["intent"], index)?;
                        let receipt = self.earlier(&outcome["receipt"], index)?;
                        check(
                            intent.value["kind"] == "CheckpointIntent"
                                && receipt.value["kind"] == "CheckpointReceipt"
                                && receipt.value["body"]["intent_record_id"]
                                    == intent.value["record_id"]
                                && receipt.value["body"]["intent_record_checksum"]
                                    == intent.checksum()
                                && body["observed"]["head"]
                                    == receipt.value["body"]["selected_head"]
                                && body["observed"]["commit"]
                                    == receipt.value["body"]["selected_commit"],
                        )
                    }
                    // The external conflict classifier must independently prove
                    // these outcomes; structural history cannot grant recovery.
                    _ => Err(PackageError::UnsupportedVersion),
                }
            }
            _ => Err(PackageError::UnsupportedVersion),
        }
    }
    /// Records the fully derived replay result without claiming storage success.
    /// # Errors
    /// Refuses damaged tails, malformed owner or bounds. Conflict decisions need
    /// independently qualified host observations and are not minted here.
    pub fn prepare_replay_decision(
        &self,
        owner: &Value,
        identity: JournalRecordIdentity,
        limits: JournalLimits,
    ) -> Result<JournalAppend, PackageError> {
        let through = self.journal.records.last().map_or(Value::Null, record_link);
        self.prepare_local("RecoveryDecision",&json!({"owner":owner,"verified_through":through,"observed":null,"outcome":{"kind":"prefix-replay","through_mutation":self.through(),"result":self.coordinate()}}),identity,limits)
    }
}
fn inverse(
    command: &Value,
    before: &Objects,
    after: &Objects,
    current: &Objects,
    redo: bool,
) -> Result<Value, PackageError> {
    let graph_id = &command["envelope"]["graph_id"];
    let graph = |pool: &Objects| -> Result<Value, PackageError> {
        pool.values()
            .find(|value| {
                value["schema"]["id"] == "photara.graph.document" && &value["graph_id"] == graph_id
            })
            .or_else(|| {
                pool.values()
                    .find(|value| &value["graph_id"] == graph_id && value["graph"].is_object())
            })
            .map(|value| value["graph"].clone())
            .ok_or(PackageError::Integrity)
    };
    let (expected, desired) = if redo {
        (graph(before)?, graph(after)?)
    } else {
        (graph(after)?, graph(before)?)
    };
    let actual = graph(current)?;
    let operation = &command["envelope"]["command"];
    let node_id = &operation["node_id"];
    let node = |graph: &Value| -> Result<Value, PackageError> {
        graph["nodes"]
            .as_array()
            .ok_or(PackageError::Record)?
            .iter()
            .find(|node| &node["id"] == node_id)
            .cloned()
            .ok_or(PackageError::Integrity)
    };
    let expected = node(&expected)?;
    let desired = node(&desired)?;
    let actual = node(&actual)?;
    let mut derived = command.clone();
    match operation["kind"].as_str() {
        Some("set-node-position") => {
            check(actual["photara.graph-position"] == expected["photara.graph-position"])?;
            fields(&desired["photara.graph-position"], &["x", "y"])?;
            for axis in ["x", "y"] {
                check(desired["photara.graph-position"][axis].as_i64().is_some())?;
                derived["envelope"]["command"][axis] =
                    desired["photara.graph-position"][axis].clone();
            }
        }
        Some("set-configuration") => {
            check(actual["configuration"] == expected["configuration"])?;
            derived["envelope"]["command"]["configuration"] = desired["configuration"].clone();
        }
        Some("set-authored-state") => {
            check(actual["authored_state"] == expected["authored_state"])?;
            derived["envelope"]["command"]["authored_state"] = desired["authored_state"].clone();
        }
        _ => return Err(PackageError::UnsupportedVersion),
    }
    Ok(derived)
}
