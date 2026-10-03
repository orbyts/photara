//! Original-bound repeatable admission and finite selected O/P/F states.
use super::{
    Limits, core,
    input::{OriginalPackage, OriginalProof, PlannerInputs, PreparedChange},
    layout,
    wire::{self, Result, encode, ensure, hash, key, number, reference, text},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub const STAGES: [&str; 7] = [
    "admitted",
    "journal-durable",
    "payload-durable",
    "finalizer-selected",
    "finalizer-durable",
    "published",
    "clean",
];
/// Only the complete shared reader may construct this wrapper. DTOs and matching
/// public identifiers do not grant a path into publication.
pub struct VerifiedOriginal {
    pub(crate) package: OriginalPackage,
    pub(crate) proof: OriginalProof,
    pub(crate) scope: Value,
    pub(crate) charge_unit: u64,
    pub(crate) empty_hold: crate::package::ObjectRef,
    pub(crate) retained_controls:
        BTreeMap<crate::package::ObjectRef, crate::package::v1_3::Membership>,
}
impl VerifiedOriginal {
    pub(crate) fn from_reader(
        package: OriginalPackage,
        proof: OriginalProof,
        scope: Value,
        charge_unit: u64,
        empty_hold: crate::package::ObjectRef,
    ) -> Self {
        Self {
            package,
            proof,
            scope,
            charge_unit,
            empty_hold,
            retained_controls: BTreeMap::new(),
        }
    }
    #[must_use]
    pub fn package(&self) -> &OriginalPackage {
        &self.package
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectorIds {
    pub commit_id: String,
    pub write_id: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attempt {
    pub token: String,
    pub nonce: String,
    pub selectors: Vec<SelectorIds>,
    pub planner: PlannerInputs,
    pub project_limit: String,
    pub cleanup_bound: String,
    pub charge_unit: String,
    pub aggregate_control_bytes: String,
    pub aggregate_control_count: String,
    pub phase_bytes: String,
    pub finalization_bytes: String,
}
#[derive(Clone)]
pub struct SelectedStage {
    pub head: Value,
    pub commit: Value,
    pub loose: BTreeMap<String, Vec<u8>>,
}
pub struct RepeatablePlan {
    pub(crate) old: OriginalPackage,
    pub(crate) original: Value,
    pub(crate) stages: Vec<SelectedStage>,
    pub(crate) files: BTreeMap<String, Vec<u8>>,
    pub(crate) payload_ends: BTreeMap<String, usize>,
    pub(crate) prepared: PreparedChange,
    pub(crate) reservation: u64,
    pub(crate) peak: u64,
    pub(crate) attempt: Attempt,
    pub(crate) empty_hold: crate::package::ObjectRef,
    pub(crate) retained_controls:
        BTreeMap<crate::package::ObjectRef, crate::package::v1_3::Membership>,
}
impl RepeatablePlan {
    pub(crate) fn operation_selection(
        &self,
        stage: usize,
    ) -> std::result::Result<
        crate::package::v1_3::reader::OperationSelection<'_>,
        crate::package::PackageError,
    > {
        if !(5..=6).contains(&stage) {
            return Err(crate::package::PackageError::Integrity);
        }
        Ok(crate::package::v1_3::reader::OperationSelection {
            expected_commit: &self.stages[stage].commit,
            expected_loose: &self.stages[stage].loose,
            packed_hold: &self.empty_hold,
            extra_global: &self.retained_controls,
        })
    }

    #[must_use]
    pub fn original_bytes(&self) -> Vec<u8> {
        encode(&self.original)
    }
    #[must_use]
    pub fn original(&self) -> &Value {
        &self.original
    }
    #[must_use]
    pub fn stages(&self) -> &[SelectedStage] {
        &self.stages
    }
    #[must_use]
    pub fn target_allocations(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.files
    }
    #[must_use]
    pub fn reserved(&self) -> u64 {
        self.reservation
    }
    #[must_use]
    pub fn attempt(&self) -> &Attempt {
        &self.attempt
    }
    #[must_use]
    pub fn control_peak(&self) -> u64 {
        self.peak
    }
}
/// Compiles a new original and checks all bounds before an adapter receives any effect.
/// Use `admit` for caller requests so exact existing-operation retries are resolved first.
/// No IO, original-ID generation, host authorization or durable acknowledgment.
/// # Errors
/// Refuses invalid original semantic replay, identity collisions, unsupported
/// topology, arithmetic overflow or plans outside the original finite envelope.
pub fn compile(
    original: &VerifiedOriginal,
    intent: &Value,
    receipt: &Value,
    attempt: Attempt,
    limits: Limits,
) -> std::result::Result<RepeatablePlan, crate::package::PackageError> {
    compile_inner(original, intent, receipt, attempt, limits)
        .map_err(|_| crate::package::PackageError::Integrity)
}
fn object(project: &Value, name: &str, mut value: Value) -> Value {
    value["schema"] = json!({"id":name,"version":1});
    value["project_id"] = project.clone();
    value["extensions"] = json!({});
    value
}
fn round(n: u64, unit: u64) -> Result<u64> {
    ensure(unit > 0, "charge unit")?;
    n.checked_add(unit - 1)
        .and_then(|n| n.checked_div(unit))
        .and_then(|n| n.checked_mul(unit))
        .ok_or("charge overflow")
}
fn decimal(s: &str) -> Result<u64> {
    number(&json!(s))
}
type Split = (Vec<Value>, Vec<Value>, BTreeMap<String, usize>);
fn split(old: &OriginalPackage, files: &BTreeMap<String, Vec<u8>>) -> Result<Split> {
    let mut payload = Vec::new();
    let mut writes = Vec::new();
    let mut stops = BTreeMap::new();
    for (id, bytes) in files {
        let a = &old.allocations[id];
        if bytes == &a.bytes {
            continue;
        }
        let start = a.bytes.len();
        let mut stop = start;
        if a.arena == "data" {
            while stop < bytes.len() && bytes[stop + 8] == 1 {
                stop = frame_end(bytes, stop)?;
            }
        }
        stops.insert(id.clone(), stop);
        if stop > start {
            payload.push(span(old, id, bytes, start, stop)?);
        }
        let mut write = span(old, id, bytes, stop, bytes.len())?;
        write["original_sha256"] = json!(hash(&a.bytes));
        write["framed_bytes"] = json!((bytes.len() - stop).to_string());
        write["recipe_sha256"] = json!(hash(&encode(&write)));
        writes.push(write);
    }
    Ok((payload, writes, stops))
}
fn frame_end(bytes: &[u8], at: usize) -> Result<usize> {
    let h = bytes
        .get(at..at.checked_add(16).ok_or("frame overflow")?)
        .ok_or("frame header")?;
    ensure(
        &h[..8] == b"PS2PKD01" && h[8] <= 3 && h[9..12] == [0, 0, 0],
        "frame header",
    )?;
    let end = at
        .checked_add(16)
        .and_then(|v| v.checked_add(u32::from_le_bytes(h[12..16].try_into().unwrap()) as usize))
        .ok_or("frame overflow")?;
    ensure(end <= bytes.len(), "frame end")?;
    Ok(end)
}
fn span(old: &OriginalPackage, id: &str, bytes: &[u8], lo: usize, hi: usize) -> Result<Value> {
    let mut at = lo;
    let mut count = 0u64;
    while at < hi {
        at = frame_end(bytes, at)?;
        count = count.checked_add(1).ok_or("frame count")?;
    }
    ensure(at == hi, "complete append frames")?;
    Ok(
        json!({"allocation_id":id,"arena":old.allocations[id].arena,"witness":old.allocations[id].witness,"original_end":lo.to_string(),"final_end":hi.to_string(),"frame_count":count.to_string(),"framed_sha256":hash(&bytes[lo..hi])}),
    )
}
fn packed(files: &BTreeMap<String, Vec<u8>>, r: &Value) -> Result<Value> {
    let k = key(r)?;
    for bytes in files.values() {
        let mut at = 0;
        while at < bytes.len() {
            let end = frame_end(bytes, at)?;
            if bytes[at + 8] != 0
                && bytes[at + 16..end].len() as u64 == k.1
                && hash(&bytes[at + 16..end]) == k.0
            {
                return wire::parse(&bytes[at + 16..end]);
            }
            at = end;
        }
    }
    Err("selected packed object")
}
pub(super) fn journal_frame(receipt: &Value) -> Value {
    json!({"schema":{"id":"photara.package.accepted-journal-frame","version":1},"journal_id":receipt["journal_id"],"sequence":receipt["journal_sequence"],"operation_id":receipt["operation_id"],"request_sha256":receipt["request_sha256"],"receipt_sha256":reference(receipt)["sha256"]})
}
#[expect(
    clippy::too_many_lines,
    reason = "Compile the complete original-bound finite state chain before effects"
)]
pub(crate) fn compile_inner(
    original: &VerifiedOriginal,
    intent: &Value,
    receipt: &Value,
    attempt: Attempt,
    limits: Limits,
) -> Result<RepeatablePlan> {
    let old = &original.package;
    let proof = &original.proof;
    let project = &old.manifest["project_id"];
    attempt.planner.validate(old, proof, limits)?;
    ensure(
        attempt.selectors.len() == STAGES.len(),
        "exact selector ID schedule",
    )?;
    let mut ids = BTreeSet::new();
    for phase in &attempt.selectors {
        for id in [&phase.commit_id, &phase.write_id] {
            wire::uuid(&json!(id))?;
            ensure(
                ids.insert(id) && old.commit["commit_id"] != *id && old.commit["write_id"] != *id,
                "fresh selector IDs",
            )?;
        }
    }
    ensure(
        decimal(&attempt.token)? > 0
            && attempt.nonce.len() == 64
            && attempt
                .nonce
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
        "original token and nonce",
    )?;
    let prepared = core::replay(old, proof, intent, receipt, &attempt.planner, limits)?;
    let mut retained_controls = original.retained_controls.clone();
    if !original.retained_controls.is_empty()
        || old.loose.values().any(|bytes| {
            wire::parse(bytes)
                .is_ok_and(|v| v["schema"]["id"] == "photara.storage.original-admission")
        })
    {
        let old_overlay = old.loose(&old.commit["root_set"]["inventory"])?;
        for r in old_overlay["controls"]
            .as_array()
            .ok_or("original controls")?
        {
            let body = old.loose(r)?;
            let membership = if body["schema"]["id"]
                .as_str()
                .is_some_and(|s| s.starts_with("photara.storage."))
            {
                crate::package::v1_3::Membership::Ownership
            } else {
                crate::package::v1_3::Membership::Semantic
            };
            retained_controls.insert(
                serde_json::from_value(r.clone()).map_err(|_| "control reference")?,
                membership,
            );
        }
    }
    let layout = layout::compile(
        old,
        proof,
        &prepared,
        &attempt.planner,
        &retained_controls,
        limits,
    )?;
    let oldroot = &old.commit["root_set"];
    let envelope = old.loose(&oldroot["placement"]["accounting"])?;
    let ledger = old.loose(&envelope["ledger"])?;
    let overlay = old.loose(&oldroot["inventory"])?;
    let unit = decimal(&attempt.charge_unit)?;
    ensure(
        unit > 0 && unit == original.charge_unit,
        "registered charge policy",
    )?;
    let cleanup = decimal(&attempt.cleanup_bound)?;
    let mut nextledger = ledger.clone();
    let mut consumed = 0u64;
    for tip in nextledger["tips"].as_array_mut().ok_or("registered tips")? {
        let id = text(&tip["allocation_id"])?;
        let bytes = layout.files.get(id).ok_or("registered target tip")?;
        let extent = bytes.len() as u64;
        let charge = round(extent, unit)?.max(number(&tip["registered_charge"])?);
        consumed = consumed
            .checked_add(
                charge
                    .checked_sub(number(&tip["registered_charge"])?)
                    .ok_or("no shrinking credit")?,
            )
            .ok_or("charge overflow")?;
        tip["extent"] = json!(extent.to_string());
        tip["registered_charge"] = json!(charge.to_string());
        tip["observation"]["measured_extent"] = json!(extent.to_string());
        tip["observation"]["charged_high_water"] = json!(charge.to_string());
    }
    let reserve = consumed.checked_add(cleanup).ok_or("reserve overflow")?;
    let before = number(&ledger["total_charge"])?;
    ensure(
        before.checked_add(reserve).ok_or("admission overflow")?
            <= decimal(&attempt.project_limit)?,
        "project capacity refusal",
    )?;
    nextledger["total_charge"] = json!(
        before
            .checked_add(consumed)
            .ok_or("ledger overflow")?
            .to_string()
    );
    let mut allocations = Vec::new();
    for (id, bytes) in &layout.files {
        let a = &old.allocations[id];
        let mut range = span(old, id, bytes, a.bytes.len(), bytes.len())?;
        range["original_sha256"] = json!(hash(&a.bytes));
        allocations.push(range);
    }
    let placement = json!({"generation":number(&oldroot["placement"]["generation"] )?.checked_add(1).ok_or("placement overflow")?.to_string(),"root_placements":layout.root_placements});
    let mut old_states = proof
        .roles
        .values()
        .map(|r| json!({"root":reference(&r.state),"value":r.state}))
        .collect::<Vec<_>>();
    old_states.sort_by_key(|row| key(&row["root"]).expect("verified state"));
    old_states.dedup();
    let original_record = object(
        project,
        "photara.storage.original-admission",
        json!({"token":attempt.token,"kind":"graph","nonce":attempt.nonce,"scope":original.scope,"original_codec":"photara.codec.ps2-repeatable-admission-v1","request":{"operation_id":receipt["operation_id"],"request_sha256":receipt["request_sha256"],"intent":reference(intent),"receipt":reference(receipt)},"old":{"head":old.head,"commit":old.commit,"envelope":envelope,"ledger":ledger,"overlay":overlay,"states":old_states},"semantic_target":layout.target,"payload_recipe":{"codec":"photara.codec.ps2-repeatable-layout-v1","planner":attempt,"retained_controls":retained_controls.iter().map(|(r,m)| json!({"record":r,"membership":match m {crate::package::v1_3::Membership::Ownership=>"ownership",crate::package::v1_3::Membership::Semantic=>"semantic",crate::package::v1_3::Membership::Blob=>"blob"}})).collect::<Vec<_>>(),"allocations":allocations,"base_inventory":layout.base_inventory,"target_placement":placement},"old_hold":envelope["holds"],"generation_plan":null,"retention_intent":null,"reserve":reserve.to_string(),"cleanup_bound":cleanup.to_string(),"project_limit":attempt.project_limit,"control_bounds":{"aggregate_control_bytes":attempt.aggregate_control_bytes,"aggregate_control_count":attempt.aggregate_control_count,"phase_bytes":attempt.phase_bytes,"finalization_bytes":attempt.finalization_bytes,"marker_bytes":"0","newborn_binding_bytes":"0"}}),
    );
    let (payload, writes, payload_ends) = split(old, &layout.files)?;
    let state = packed(&layout.files, &layout.target["active"])?;
    let mut projection = layout.target.clone();
    projection["retention_evidence"] = oldroot["retention_evidence"].clone();
    projection["base_inventory"] = layout.base_inventory.clone();
    projection["placement"] = placement;
    projection["active_state"] = state.clone();
    let finalizer = object(
        project,
        "photara.storage.finalization-control",
        json!({"token":attempt.token,"original":reference(&original_record),"prepared_receipt":receipt,"generation_plan":null,"newborn_binding":null,"payload_completion":payload,"sealed_allocations":[],"recipe_codec":"photara.codec.ps2-repeatable-layout-v1","writes":writes,"target_projection":projection}),
    );
    ensure(
        encode(&finalizer).len() as u64 <= decimal(&attempt.finalization_bytes)?,
        "finite finalizer bytes",
    )?;
    let basecount = number(&packed(&layout.files, &layout.base_inventory)?["count"])?;
    let mut stages = Vec::new();
    let mut priorhead = old.head.clone();
    let mut priorcommit = old.commit.clone();
    let mut priorloose = old.loose.clone();
    let mut peak = 0u64;
    for (i, stage) in STAGES.iter().enumerate() {
        let published = i >= 5;
        let charged = if published { consumed } else { 0 };
        let remaining = if i == 6 { 0 } else { reserve - charged };
        let phase = object(
            project,
            "photara.storage.operation-phase",
            json!({"token":attempt.token,"original":reference(&original_record),"stage":stage,"consumed":charged.to_string(),"remaining":remaining.to_string(),"newborn_binding":null,"finalization":if i>=3{reference(&finalizer)}else{Value::Null},"cleanup":if i==6{json!({"remaining_roles":[],"directory_barrier":true,"released":cleanup.to_string()})}else{Value::Null},"journal_completion":if i>=1{json!({"frame":journal_frame(receipt),"barrier":true})}else{Value::Null}}),
        );
        ensure(
            encode(&phase).len() as u64 <= decimal(&attempt.phase_bytes)?,
            "finite phase bytes",
        )?;
        let holds = object(
            project,
            "photara.storage.hold-leaf",
            json!({"count":"1","reserved":reserve.to_string(),"consumed":charged.to_string(),"remaining":remaining.to_string(),"entries":[{"token":attempt.token,"original":reference(&original_record),"phase":reference(&phase)}]}),
        );
        let selectedledger = if published {
            nextledger.clone()
        } else {
            ledger.clone()
        };
        let selectedenvelope = object(
            project,
            "photara.storage.accounting-envelope",
            json!({"ledger":reference(&selectedledger),"holds":reference(&holds)}),
        );
        let mut controls = vec![
            selectedledger,
            selectedenvelope.clone(),
            original_record.clone(),
            phase,
            holds,
            intent.clone(),
        ];
        if !published {
            controls.push(receipt.clone());
        }
        if i >= 3 {
            controls.push(finalizer.clone());
        }
        // Prior terminal controls remain selected until their exact packed copies
        // and new locators are durable. Publication transfers them into immutable
        // retained dependencies; cleanup never strands original recovery bytes.
        if !published {
            for r in retained_controls.keys() {
                if let Some(bytes) = old.loose.get(r.sha256.as_str()) {
                    controls.push(wire::parse(bytes)?);
                }
            }
        }
        let unique = controls
            .into_iter()
            .map(|v| (key(&reference(&v)).expect("control ref"), v))
            .collect::<BTreeMap<_, _>>();
        let controls = unique.into_values().collect::<Vec<_>>();
        let base = if published {
            layout.base_inventory.clone()
        } else {
            overlay["base"].clone()
        };
        let count = if published {
            basecount
        } else {
            number(&overlay["count"])?
                .checked_sub(
                    overlay["controls"]
                        .as_array()
                        .ok_or("original controls")?
                        .len() as u64,
                )
                .ok_or("overlay count")?
        };
        let mut references = controls.iter().map(reference).collect::<Vec<_>>();
        references.sort_by_key(|r| key(r).unwrap());
        let nextoverlay = object(
            project,
            "photara.package.inventory-overlay",
            json!({"base":base,"controls":references,"count":count.checked_add(controls.len()as u64).ok_or("overlay overflow")?.to_string()}),
        );
        let mut root = oldroot.clone();
        root["inventory"] = reference(&nextoverlay);
        root["placement"]["accounting"] = reference(&selectedenvelope);
        if published {
            for (k, v) in layout.target.as_object().ok_or("target")? {
                root[k] = v.clone();
            }
            root["placement"]["generation"] =
                original_record["payload_recipe"]["target_placement"]["generation"].clone();
            root["placement"]["root_placements"] = layout.root_placements.clone();
        }
        let mut commit = old.commit.clone();
        commit["root_set"] = root;
        commit["commit_id"] = json!(attempt.selectors[i].commit_id);
        commit["write_id"] = json!(attempt.selectors[i].write_id);
        commit["package_revision"] = json!(
            number(&priorcommit["package_revision"])?
                .checked_add(1)
                .ok_or("package revision overflow")?
                .to_string()
        );
        commit["parent"] = json!({"commit_id":priorcommit["commit_id"],"commit_sha256":hash(&encode(&priorcommit))});
        if published {
            for name in ["authored", "history", "inventory"] {
                commit[name] = state[name].clone();
            }
        }
        let mut head = old.head.clone();
        head["commit_id"] = commit["commit_id"].clone();
        head["commit_sha256"] = json!(hash(&encode(&commit)));
        let loose = controls
            .iter()
            .chain([&nextoverlay])
            .map(|v| (hash(&encode(v)), encode(v)))
            .collect::<BTreeMap<_, _>>();
        let mut coexist = BTreeMap::new();
        for map in [&priorloose, &loose] {
            for (k, v) in map {
                coexist.insert(k.clone(), v.len() as u64);
            }
        }
        for v in [&old.manifest, &priorhead, &priorcommit, &head, &commit] {
            coexist.insert(hash(&encode(v)), encode(v).len() as u64);
        }
        let selector = object(
            project,
            "photara.storage.admission-selector",
            json!({"token":attempt.token,"original":reference(&original_record),"old_head_sha256":hash(&encode(&priorhead)),"next_head_sha256":hash(&encode(&head))}),
        );
        let scratch = round(encode(&selector).len() as u64, unit)?
            .checked_add(round(encode(&head).len() as u64, unit)?)
            .and_then(|n| n.checked_add(unit))
            .ok_or("scratch overflow")?;
        let charge = coexist.values().try_fold(scratch, |n, len| {
            n.checked_add(round(*len, unit)?).ok_or("control overflow")
        })?;
        ensure(
            charge <= decimal(&attempt.aggregate_control_bytes)?
                && coexist.len() as u64 + 3 <= decimal(&attempt.aggregate_control_count)?,
            "original adjacent control bound",
        )?;
        ensure(
            decimal(&attempt.aggregate_control_bytes)? <= number(&ledger["standing_control"])?,
            "standing control charge",
        )?;
        peak = peak.max(charge);
        priorhead = head.clone();
        priorcommit = commit.clone();
        priorloose = loose.clone();
        stages.push(SelectedStage {
            head,
            commit,
            loose,
        });
    }
    Ok(RepeatablePlan {
        old: old.clone(),
        original: original_record,
        stages,
        files: layout.files,
        payload_ends,
        prepared,
        reservation: reserve,
        peak,
        attempt,
        empty_hold: original.empty_hold.clone(),
        retained_controls,
    })
}

impl RepeatablePlan {
    /// Independently verifies exact prospective or selected candidate bytes with
    /// the complete shared reader. Provider metadata remains independently bound.
    /// # Errors
    /// Refuses any changed bytes, registration, closure, role or runtime budget.
    #[expect(
        clippy::too_many_arguments,
        reason = "Independent reader boundary inputs"
    )]
    pub fn inspect_candidate(
        &self,
        package: &OriginalPackage,
        stage: usize,
        identity: crate::package::v1_3::SelectionIdentity,
        inspection: &impl crate::package::v1_3::AllocationInspection,
        registration: &crate::package::v1_3::SettledRegistration,
        frames: crate::package::v1_3::FrameLimits,
        limits: &crate::package::v1_3::ReaderLimits,
    ) -> std::result::Result<crate::package::v1_3::SettledPackage, crate::package::PackageError>
    {
        crate::package::v1_3::reader::inspect_operation_package(
            package,
            identity,
            inspection,
            registration,
            frames,
            limits,
            self,
            stage,
        )
    }
    /// Converts only the complete regenerated clean selection into a subsequent
    /// read-verified original. This is still not permission to write.
    /// # Errors
    /// Refuses a non-clean selection or any failed complete candidate inspection.
    #[expect(
        clippy::too_many_arguments,
        reason = "Independent reader boundary inputs"
    )]
    pub fn verify_next_original(
        &self,
        package: OriginalPackage,
        identity: crate::package::v1_3::SelectionIdentity,
        inspection: &impl crate::package::v1_3::AllocationInspection,
        registration: &crate::package::v1_3::SettledRegistration,
        directory: crate::package::v1_3::DirectoryObservation,
        frames: crate::package::v1_3::FrameLimits,
        limits: &crate::package::v1_3::ReaderLimits,
    ) -> std::result::Result<VerifiedOriginal, crate::package::PackageError> {
        crate::package::v1_3::reader::verify_operation_original(
            package,
            identity,
            inspection,
            registration,
            directory,
            frames,
            limits,
            self,
            6,
        )
    }
}
