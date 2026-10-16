//! Approved PHPSJ001 checkpoints and PS3 local records. Append preparation
//! never establish native durability or mint Accepted/Saved acknowledgements.
use super::shared::OriginalPackage;
use super::{RepeatablePlan, RestartContext, plan, restart, wire};
use crate::package::v1_3::{AllocationInspection, selected, tree};
use crate::package::{JsonLimits, PackageError, PackageUuid, Sha256Hex, parse_canonical_json};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[path = "journal_session.rs"]
pub(crate) mod session;
pub use session::VerifiedSession;

const MAGIC: &[u8; 8] = b"PHPSJ001";
const SNAPSHOT_MAGIC: &[u8; 8] = b"PHPSJ002";
fn magic(header: &Value) -> Result<&'static [u8; 8], PackageError> {
    match header["format_version"].as_u64() {
        Some(1) => Ok(MAGIC),
        Some(2) => Ok(SNAPSHOT_MAGIC),
        _ => Err(PackageError::UnsupportedVersion),
    }
}
fn sequence_base(header: &Value) -> Result<u64, PackageError> {
    if header["format_version"] == 2 {
        n(&header["sequence_base"])
    } else {
        Ok(0)
    }
}
#[derive(Clone, Copy)]
pub struct JournalLimits {
    pub json: JsonLimits,
    pub max_file_bytes: usize,
    pub max_records: usize,
    pub max_work_bytes: usize,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JournalTail {
    Complete,
    Incomplete,
}
#[derive(Clone)]
pub struct JournalRecord {
    value: Value,
    checksum: [u8; 32],
    frame_end: usize,
}
impl JournalRecord {
    #[must_use]
    pub fn value(&self) -> &Value {
        &self.value
    }
    #[must_use]
    pub fn checksum(&self) -> String {
        hex(&self.checksum)
    }
}
/// Framing-checked immutable bytes. Call `verify` to replay supported originals
/// before preparing any further checkpoint record.
pub struct Journal {
    bytes: Vec<u8>,
    header: Value,
    records: Vec<JournalRecord>,
    last: [u8; 32],
    tail: JournalTail,
    verified_bytes: usize,
    work: usize,
    json_limits: JsonLimits,
}
/// Checkpoint-history-verified stream, not native permission or durability.
/// Local Mutation/lifecycle evidence additionally requires `verify_session`.
pub struct VerifiedJournal<'a> {
    journal: &'a Journal,
    plans: BTreeMap<String, CheckpointPlan>,
}
// Completed history needs commitments and semantic replay inputs, not copies of
// every historical pack. Construction follows deterministic original validation;
// completed receipts still run the full current-prefix reader below.
#[derive(Clone)]
struct CheckpointPlan {
    original_ref: Value,
    receipt: Value,
    intent: Value,
    selected_state: Value,
    head: Value,
    json_shape: JsonLimits,
}
#[derive(Clone)]
struct JournalResume {
    prefix: Vec<u8>,
    count: usize,
    plans: BTreeMap<String, CheckpointPlan>,
    original: restart::CompactOriginal,
    lineage: BTreeSet<String>,
    head: Value,
}
#[cfg(any(test, feature = "controlled-disposable"))]
#[cfg_attr(test, derive(Clone))]
pub(super) struct JournalMemo {
    key: String,
    snapshot: restart::snapshot_memo::PrefixSnapshot,
    resume: JournalResume,
    observations: restart::snapshot_memo::Observations,
    pub(super) cost: usize,
}
#[cfg(any(test, feature = "controlled-disposable"))]
impl JournalMemo {
    #[cfg(test)]
    pub(crate) fn uncompressed_cost(&self, journal: &Journal) -> usize {
        let mut old = self.resume.clone();
        for (id, plan) in &mut old.plans {
            plan.original_ref = journal
                .records
                .iter()
                .find(|r| r.value["record_id"] == *id)
                .unwrap()
                .value["body"]["original"]
                .clone();
        }
        self.cost - self.resume.cost().unwrap() + old.cost().unwrap()
    }
    pub(super) fn lineage(&self) -> &BTreeSet<String> {
        &self.resume.lineage
    }
    pub(super) fn current_original<I: AllocationInspection>(
        &self,
        current: &OriginalPackage,
        context: &RestartContext<'_, I>,
    ) -> Result<Option<super::VerifiedOriginal>, PackageError> {
        if self.key != restart::CompiledPlans::snapshot_key(current, context)
            || !self.observations.matches(context.inspection)?
        {
            return Ok(None);
        }
        self.resume.original.exact_current(current, context)
    }
}
#[cfg(any(test, feature = "controlled-disposable"))]
impl JournalResume {
    fn cost(&self) -> Option<usize> {
        let metadata = json!({"plans":self.plans.iter().map(|(id,p)|json!([id,p.original_ref,p.receipt,p.intent,p.selected_state,p.head,format!("{:?}",p.json_shape)])).collect::<Vec<_>>(),"lineage":self.lineage,"head":self.head});
        restart::value_storage(&metadata)?
            .checked_add(self.prefix.capacity())?
            .checked_add(self.original.cost()?)?
            .checked_add(512)
    }
}
impl CheckpointPlan {
    fn from_plan(plan: &RepeatablePlan, limits: JsonLimits) -> Result<Self, PackageError> {
        let state_ref = &plan.stages[6].commit["root_set"]["active"];
        let mut json_shape = JsonLimits {
            max_bytes: 0,
            max_depth: 0,
            max_members: 0,
            max_array_elements: 0,
        };
        let values = plan.stages[6]
            .loose
            .values()
            .map(|bytes| {
                let value = parse_canonical_json(bytes, limits)?;
                json_shape.max_bytes = json_shape.max_bytes.max(bytes.len());
                measure_json(&value, 0, &mut json_shape);
                Ok(value)
            })
            .collect::<Result<Vec<_>, PackageError>>()?;
        let selected_state = values
            .into_iter()
            .find_map(|value| {
                let state = &value["target_projection"]["active_state"];
                (value["schema"]["id"] == "photara.storage.finalization-control"
                    && value["original"] == wire::reference(&plan.original)
                    && wire::reference(state) == *state_ref)
                    .then(|| state.clone())
            })
            .ok_or(PackageError::Integrity)?;
        Ok(Self {
            original_ref: wire::reference(&plan.original),
            receipt: plan.prepared.receipt.clone(),
            intent: intent_body(plan)["semantic_intent"].clone(),
            selected_state,
            head: plan.stages[6].head.clone(),
            json_shape,
        })
    }
}
fn measure_json(value: &Value, depth: usize, shape: &mut JsonLimits) {
    shape.max_depth = shape.max_depth.max(depth);
    match value {
        Value::Array(a) => {
            shape.max_array_elements = shape.max_array_elements.max(a.len());
            for value in a {
                measure_json(value, depth + 1, shape);
            }
        }
        Value::Object(o) => {
            shape.max_members = shape.max_members.max(o.len());
            for value in o.values() {
                measure_json(value, depth + 1, shape);
            }
        }
        _ => {}
    }
}
/// Exact append or exact-existing-record retry. Existing bytes must be rebarriered
/// after uncertainty; an empty append is never evidence of durability by itself.
pub struct JournalAppend {
    bytes: Vec<u8>,
    expected_extent: usize,
    expected_prefix_sha256: String,
    record: JournalRecord,
    existing: bool,
}
impl JournalAppend {
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    #[must_use]
    pub fn expected_extent(&self) -> usize {
        self.expected_extent
    }
    #[must_use]
    pub fn expected_prefix_sha256(&self) -> &str {
        &self.expected_prefix_sha256
    }
    #[must_use]
    pub fn record(&self) -> &JournalRecord {
        &self.record
    }
    #[must_use]
    pub fn existing(&self) -> bool {
        self.existing
    }
}
#[derive(Clone, Copy)]
pub struct JournalRecordIdentity {
    pub record_id: PackageUuid,
    pub session_generation: u64,
}
fn check(ok: bool) -> Result<(), PackageError> {
    if ok {
        Ok(())
    } else {
        Err(PackageError::Integrity)
    }
}
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut s, b| {
        write!(&mut s, "{b:02x}").expect("String output");
        s
    })
}
fn hash_parts(parts: &[&[u8]]) -> [u8; 32] {
    let mut sha = Sha256::new();
    for p in parts {
        sha.update(p);
    }
    sha.finalize().into()
}
fn fields(v: &Value, names: &[&str]) -> Result<(), PackageError> {
    tree::fields(v, names)
}
fn id(v: &Value) -> Result<PackageUuid, PackageError> {
    selected::nonnil(v)
}
fn digest(v: &Value) -> Result<(), PackageError> {
    Sha256Hex::parse(v.as_str().ok_or(PackageError::Record)?).map(|_| ())
}
fn n(v: &Value) -> Result<u64, PackageError> {
    tree::number(v)
}
fn reference(v: &Value) -> Value {
    wire::reference(v)
}
fn header(v: &Value) -> Result<(), PackageError> {
    magic(v)?;
    let mut names = vec![
        "format_version",
        "stream_id",
        "journal_id",
        "device_id",
        "project_id",
        "library_id",
        "incarnation_id",
        "bootstrap_sha256",
        "base_head",
        "base_package_revision",
    ];
    if v["format_version"] == 2 {
        names.extend(["base", "sequence_base", "previous_segment"]);
        selected::json_ref(&v["base"])?;
        let previous = &v["previous_segment"];
        fields(previous, &["stream_id", "header_sha256", "last"])?;
        id(&previous["stream_id"])?;
        digest(&previous["header_sha256"])?;
        super::checkpoint::link(&previous["last"])?;
        check(
            previous["stream_id"] != v["stream_id"]
                && previous["last"]["sequence"] == v["sequence_base"],
        )?;
    }
    fields(v, &names)?;
    for k in [
        "stream_id",
        "journal_id",
        "device_id",
        "project_id",
        "library_id",
        "incarnation_id",
    ] {
        id(&v[k])?;
    }
    digest(&v["bootstrap_sha256"])?;
    n(&v["base_package_revision"])?;
    fields(
        &v["base_head"],
        &["schema", "project_id", "commit_id", "commit_sha256"],
    )?;
    check(v["base_head"]["schema"] == json!({"id":"photara.package.head","version":1}))?;
    id(&v["base_head"]["project_id"])?;
    id(&v["base_head"]["commit_id"])?;
    digest(&v["base_head"]["commit_sha256"])?;
    check(v["base_head"]["project_id"] == v["project_id"])
}
fn body(record: &Value, header: &Value) -> Result<(), PackageError> {
    match record["kind"].as_str() {
        Some("CheckpointBaseReceipt") => {
            check(header["format_version"] == 2)?;
            let body = &record["body"];
            fields(
                body,
                &[
                    "maintenance_id",
                    "original",
                    "journal_base",
                    "selected_head",
                    "selected_commit",
                ],
            )?;
            id(&body["maintenance_id"])?;
            selected::json_ref(&body["original"])?;
            selected::json_ref(&body["journal_base"])?;
            selected::json_ref(&body["selected_commit"])?;
            check(
                body["journal_base"] == header["base"]
                    && body["selected_head"] == header["base_head"]
                    && body["selected_head"]["commit_sha256"] == body["selected_commit"]["sha256"],
            )
        }
        Some("CheckpointIntent") => {
            let body = &record["body"];
            fields(
                body,
                &[
                    "original",
                    "semantic_intent",
                    "operation_receipt",
                    "accepted_frame",
                ],
            )?;
            let codec = body["original"]["original_codec"].as_str();
            check(
                matches!(
                    codec,
                    Some(
                        "photara.codec.ps2-repeatable-admission-v1"
                            | "photara.codec.ps2-repeatable-admission-v2"
                            | "photara.codec.ps2-repeatable-admission-v3"
                    )
                ) || (codec == Some("photara.codec.ps2-repeatable-admission-v4")
                    && header["format_version"] == 2
                    && body["original"]["payload_recipe"]["checkpoint_base"] == header["base"]),
            )?;
            let receipt = &body["operation_receipt"];
            let original = &body["original"];
            let intent = &body["semantic_intent"];
            let identity = crate::package::v1_3::SelectionIdentity {
                project: id(&header["project_id"])?,
                library: id(&header["library_id"])?,
                bootstrap_sha256: Sha256Hex::parse(
                    header["bootstrap_sha256"]
                        .as_str()
                        .ok_or(PackageError::Record)?,
                )?,
            };
            crate::package::v1_3::operations::validate_receipt(receipt, &identity)?;
            check(
                original["request"]["intent"] == reference(intent)
                    && original["request"]["receipt"] == reference(receipt)
                    && original["request"]["operation_id"] == receipt["operation_id"]
                    && original["request"]["request_sha256"] == receipt["request_sha256"]
                    && intent["operation_id"] == receipt["operation_id"]
                    && receipt["request_sha256"] == wire::hash(&wire::encode(intent))
                    && receipt["journal_id"] == header["journal_id"]
                    && original["scope"]["incarnation"] == header["incarnation_id"]
                    && original["project_id"] == header["project_id"]
                    && body["accepted_frame"] == plan::journal_frame(receipt),
            )
        }
        Some("CheckpointReceipt") => {
            let body = &record["body"];
            fields(
                body,
                &[
                    "intent_record_id",
                    "intent_record_checksum",
                    "original_sha256",
                    "operation_receipt",
                    "selected_head",
                    "selected_commit",
                ],
            )?;
            id(&body["intent_record_id"])?;
            digest(&body["intent_record_checksum"])?;
            digest(&body["original_sha256"])?;
            Ok(())
        }
        Some("Mutation" | "UndoBoundary" | "SessionBarrier" | "RecoveryDecision") => {
            session::shape(record, header)
        }
        _ => Err(PackageError::UnsupportedVersion),
    }
}
fn envelope(v: &Value, h: &Value, sequence: u64) -> Result<(), PackageError> {
    fields(
        v,
        &[
            "format_version",
            "sequence",
            "record_id",
            "session_generation",
            "project_id",
            "incarnation_id",
            "kind",
            "body",
        ],
    )?;
    if v["format_version"] != h["format_version"] {
        return Err(PackageError::UnsupportedVersion);
    }
    if h["format_version"] == 2 {
        check(
            (sequence
                == sequence_base(h)?
                    .checked_add(1)
                    .ok_or(PackageError::Limit)?)
                == (v["kind"] == "CheckpointBaseReceipt"),
        )?;
    }
    id(&v["record_id"])?;
    id(&v["project_id"])?;
    id(&v["incarnation_id"])?;
    check(
        n(&v["sequence"])? == sequence
            && n(&v["session_generation"])? > 0
            && v["project_id"] == h["project_id"]
            && v["incarnation_id"] == h["incarnation_id"],
    )?;
    body(v, h)
}
#[cfg(test)]
thread_local! { static DECODED_BYTES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
#[cfg(test)]
pub(crate) fn take_decoded_bytes() -> usize {
    DECODED_BYTES.with(|n| n.replace(0))
}

fn charge_payload(
    bytes: &[u8],
    limits: JournalLimits,
    work: &mut usize,
) -> Result<(), PackageError> {
    *work = work.checked_add(bytes.len()).ok_or(PackageError::Limit)?;
    if *work > limits.max_work_bytes {
        return Err(PackageError::Limit);
    }
    Ok(())
}
fn payload(bytes: &[u8], limits: JournalLimits, work: &mut usize) -> Result<Value, PackageError> {
    charge_payload(bytes, limits, work)?;
    #[cfg(test)]
    DECODED_BYTES.with(|n| n.set(n.get().saturating_add(bytes.len())));
    parse_canonical_json(bytes, limits.json)
}
impl Journal {
    pub(crate) fn snapshot_receipt(
        &self,
        current: &super::VerifiedOriginal,
        plan: &RepeatablePlan,
        intent: PackageUuid,
        identity: JournalRecordIdentity,
        limits: JournalLimits,
    ) -> Result<JournalAppend, PackageError> {
        check(
            self.header["format_version"] == 2
                && plan.original["payload_recipe"]["checkpoint_base"] == self.header["base"]
                && current.package.head == plan.stages[6].head
                && current.package.commit == plan.stages[6].commit
                && current.scope == plan.original["scope"],
        )?;
        check(self.records.iter().any(|record| {
            record.value["kind"] == "Mutation"
                && record.value["body"]["operation_receipt"] == plan.prepared.receipt
                && record.value["body"]["semantic_intent"] == intent_body(plan)["semantic_intent"]
        }))?;
        let verified = VerifiedJournal {
            journal: self,
            plans: BTreeMap::from([(
                intent.to_string(),
                CheckpointPlan::from_plan(plan, limits.json)?,
            )]),
        };
        verified.prepare_receipt(plan, intent, identity, limits)
    }
    // Fresh exact clean-plan reader evidence permits finishing a local receipt
    // lost after package publication. This prepares bytes, never durability.
    pub(crate) fn reconcile_snapshot_receipt(
        &self,
        current: &super::VerifiedOriginal,
        identity: JournalRecordIdentity,
        limits: JournalLimits,
    ) -> Result<JournalAppend, PackageError> {
        check(self.header["format_version"] == 2)?;
        let original = current
            .completed_original
            .as_ref()
            .ok_or(PackageError::Integrity)?;
        let intent = self
            .records
            .iter()
            .find(|r| {
                r.value["kind"] == "CheckpointIntent"
                    && reference(&r.value["body"]["original"]) == json!(original)
            })
            .ok_or(PackageError::Integrity)?;
        let body = &intent.value["body"];
        let receipt = &body["operation_receipt"];
        let active = current
            .proof
            .roles
            .get("active")
            .ok_or(PackageError::Integrity)?;
        check(
            body["original"]["original_codec"] == "photara.codec.ps2-repeatable-admission-v4"
                && body["original"]["payload_recipe"]["checkpoint_base"] == self.header["base"]
                && body["original"]["scope"] == current.scope
                && active.receipts.last() == Some(receipt)
                && self.records.iter().any(|r| {
                    r.value["kind"] == "Mutation"
                        && r.value["body"]["operation_receipt"] == *receipt
                        && r.value["body"]["semantic_intent"] == body["semantic_intent"]
                        && r.value["body"]["base_head"] == body["original"]["old"]["head"]
                }),
        )?;
        let body = json!({"intent_record_id":intent.value["record_id"],"intent_record_checksum":intent.checksum(),"original_sha256":original.sha256,"operation_receipt":receipt,"selected_head":current.package.head,"selected_commit":current.package.commit});
        let append = VerifiedJournal {
            journal: self,
            plans: BTreeMap::new(),
        }
        .prepare("CheckpointReceipt", &body, identity, limits)?;
        if !append.existing() {
            // Do not move the cold-replay boundary past newer accepted work.
            // Existing receipts keep their original location and remain retryable.
            let included = n(&receipt["acceptance_ordinal"])?;
            for record in &self.records {
                if matches!(
                    record.value["kind"].as_str(),
                    Some("Mutation" | "CheckpointIntent")
                ) {
                    check(
                        n(&record.value["body"]["operation_receipt"]["acceptance_ordinal"])?
                            <= included,
                    )?;
                }
            }
        }
        Ok(append)
    }
    pub(crate) fn base_prefix(&self) -> Result<&[u8], PackageError> {
        let first = self.records.first().ok_or(PackageError::Integrity)?;
        check(first.value["kind"] == "CheckpointBaseReceipt")?;
        Ok(&self.bytes[..first.frame_end])
    }
    /// Creates only the approved exact header bytes, with explicit process bounds.
    /// # Errors
    /// Refuses unsupported/malformed header fields or caller byte/work ceilings.
    pub fn create(header_value: &Value, limits: JournalLimits) -> Result<Vec<u8>, PackageError> {
        header(header_value)?;
        let magic = magic(header_value)?;
        let bytes = wire::encode(header_value);
        let len = u32::try_from(bytes.len())
            .map_err(|_| PackageError::Limit)?
            .to_le_bytes();
        if bytes.is_empty()
            || bytes.len() > limits.json.max_bytes
            || bytes.len() > limits.max_work_bytes
        {
            return Err(PackageError::Limit);
        }
        let mut out = Vec::new();
        let end = 12usize
            .checked_add(bytes.len())
            .and_then(|v| v.checked_add(32))
            .ok_or(PackageError::Limit)?;
        if end > limits.max_file_bytes {
            return Err(PackageError::Limit);
        }
        let mut work = 0;
        payload(&bytes, limits, &mut work)?;
        out.extend(magic);
        out.extend(len);
        out.extend(&bytes);
        out.extend(hash_parts(&[magic, &len, &bytes]));
        Ok(out)
    }
    /// Prepare only the first original-bound checkpoint-base receipt. The caller
    /// must independently verify the clean package and barrier the base first.
    pub(super) fn base_receipt(
        &self,
        body: &Value,
        identity: JournalRecordIdentity,
        limits: JournalLimits,
    ) -> Result<JournalAppend, PackageError> {
        check(self.header["format_version"] == 2 && self.records.len() <= 1)?;
        VerifiedJournal {
            journal: self,
            plans: BTreeMap::new(),
        }
        .prepare("CheckpointBaseReceipt", body, identity, limits)
    }
    /// Reads the verified prefix without truncating or interpreting an incomplete
    /// tail as successful append. A checksum mismatch is corruption even at EOF.
    /// `expected_header` must come from independently validated local registration.
    /// # Errors
    /// Refuses identity changes, unsupported envelopes, corrupt frames and limits.
    pub fn read(
        bytes: &[u8],
        expected_header: &Value,
        limits: JournalLimits,
    ) -> Result<Self, PackageError> {
        Self::read_inner(bytes, expected_header, limits, None)
    }
    // Canonical decode reuse only: callers still run current package/session
    // verification. Actual bytes, header context, all framing/checksums, work
    // limits, record IDs and suffix validation are checked in this invocation.
    #[cfg(any(test, feature = "controlled-disposable"))]
    pub(crate) fn read_reusing(
        bytes: &[u8],
        expected_header: &Value,
        limits: JournalLimits,
        prior: &Self,
    ) -> Result<Self, PackageError> {
        Self::read_inner(bytes, expected_header, limits, Some(prior))
    }
    #[cfg(any(test, feature = "controlled-disposable"))]
    pub(crate) fn record_end(&self, record_id: &Value) -> Option<usize> {
        self.records
            .iter()
            .find(|r| r.value["record_id"] == *record_id)
            .map(|r| r.frame_end)
    }
    fn read_inner(
        bytes: &[u8],
        expected_header: &Value,
        limits: JournalLimits,
        prior: Option<&Self>,
    ) -> Result<Self, PackageError> {
        if bytes.len() > limits.max_file_bytes {
            return Err(PackageError::Limit);
        }
        header(expected_header)?;
        let magic = magic(expected_header)?;
        check(bytes.get(..8) == Some(magic.as_slice()))?;
        let raw_len: [u8; 4] = bytes
            .get(8..12)
            .ok_or(PackageError::Integrity)?
            .try_into()
            .map_err(|_| PackageError::Record)?;
        let length = u32::from_le_bytes(raw_len) as usize;
        check(length > 0)?;
        if length > limits.json.max_bytes {
            return Err(PackageError::Limit);
        }
        let header_end = 12usize.checked_add(length).ok_or(PackageError::Limit)?;
        let mut at = header_end.checked_add(32).ok_or(PackageError::Limit)?;
        let raw = bytes.get(12..header_end).ok_or(PackageError::Integrity)?;
        let mut last = hash_parts(&[magic, &raw_len, raw]);
        check(bytes.get(header_end..at) == Some(last.as_slice()))?;
        let mut work = 0;
        let h = payload(raw, limits, &mut work)?;
        header(&h)?;
        check(h == *expected_header)?;
        let mut reusable = prior.filter(|old| {
            old.header == h
                && limits.json.max_bytes >= old.json_limits.max_bytes
                && limits.json.max_depth >= old.json_limits.max_depth
                && limits.json.max_members >= old.json_limits.max_members
                && limits.json.max_array_elements >= old.json_limits.max_array_elements
        });
        let mut records = Vec::new();
        let mut ids = BTreeSet::new();
        let mut tail = JournalTail::Complete;
        while at < bytes.len() {
            if records.len() >= limits.max_records {
                return Err(PackageError::Limit);
            }
            let Some(raw_len) = bytes.get(at..at.checked_add(4).ok_or(PackageError::Limit)?) else {
                tail = JournalTail::Incomplete;
                break;
            };
            let length =
                u32::from_le_bytes(raw_len.try_into().map_err(|_| PackageError::Record)?) as usize;
            check(length > 0)?;
            if length > limits.json.max_bytes {
                return Err(PackageError::Limit);
            }
            let start = at + 4;
            let end = start.checked_add(length).ok_or(PackageError::Limit)?;
            let next = end.checked_add(32).ok_or(PackageError::Limit)?;
            if next > limits.max_file_bytes {
                return Err(PackageError::Limit);
            }
            let Some(frame) = bytes.get(start..next) else {
                tail = JournalTail::Incomplete;
                break;
            };
            let raw = &frame[..length];
            let checksum = hash_parts(&[&last, raw_len, raw]);
            check(frame[length..] == checksum)?;
            let prior_record = reusable.and_then(|old| {
                old.records.get(records.len()).filter(|r| {
                    r.frame_end == next && old.bytes.get(at..next) == bytes.get(at..next)
                })
            });
            let value = if let Some(record) = prior_record {
                charge_payload(raw, limits, &mut work)?;
                record.value.clone()
            } else {
                reusable = None;
                payload(raw, limits, &mut work)?
            };
            let sequence = u64::try_from(records.len())
                .map_err(|_| PackageError::Limit)?
                .checked_add(sequence_base(&h)?)
                .and_then(|n| n.checked_add(1))
                .ok_or(PackageError::Limit)?;
            envelope(&value, &h, sequence)?;
            check(ids.insert(id(&value["record_id"])?))?;
            records.push(JournalRecord {
                value,
                checksum,
                frame_end: next,
            });
            last = checksum;
            at = next;
        }
        Ok(Self {
            bytes: bytes.to_vec(),
            header: h,
            records,
            last,
            tail,
            verified_bytes: at,
            work,
            json_limits: limits.json,
        })
    }
    #[must_use]
    pub fn header(&self) -> &Value {
        &self.header
    }
    #[must_use]
    pub fn records(&self) -> &[JournalRecord] {
        &self.records
    }
    #[must_use]
    pub fn tail(&self) -> JournalTail {
        self.tail
    }
    #[must_use]
    pub fn verified_bytes(&self) -> usize {
        self.verified_bytes
    }
    /// Replays every supported original through shared Core/reader recovery and
    /// verifies completed checkpoint closures against actual historical prefixes.
    /// Local records are framing/schema checked here; their authored replay and
    /// lifecycle evidence require the returned proof's `verify_session`.
    /// # Errors
    /// Refuses unsupported originals, changed intent/receipt identities, false
    /// completed selectors, missing original bytes, or explicit recovery bounds.
    pub fn verify<'a, I: AllocationInspection>(
        &'a self,
        current: &super::input::OriginalPackage,
        context: &RestartContext<'_, I>,
    ) -> Result<VerifiedJournal<'a>, PackageError> {
        self.verify_shared(&current.clone().into(), context)
    }
    pub(crate) fn verify_shared<'a, I: AllocationInspection>(
        &'a self,
        current: &OriginalPackage,
        context: &RestartContext<'_, I>,
    ) -> Result<VerifiedJournal<'a>, PackageError> {
        self.verify_inner(current, context, None, None)
            .map(|(verified, _)| verified)
    }
    #[cfg(any(test, feature = "controlled-disposable"))]
    pub(crate) fn verify_cached<'a, I: AllocationInspection>(
        &'a self,
        current: &OriginalPackage,
        context: &RestartContext<'_, I>,
        memo: &mut restart::CompiledPlans,
    ) -> Result<VerifiedJournal<'a>, PackageError> {
        memo.trim(context.planner, context.max_originals);
        let key = restart::CompiledPlans::snapshot_key(current, context);
        let mut prior_observations = None;
        if let Some(entry) = &memo.journal
            && self.bytes.starts_with(&entry.resume.prefix)
        {
            if entry.key == key && entry.observations.matches(context.inspection)? {
                prior_observations = Some(entry.observations.clone());
            } else {
                // A changed selected HEAD can use historical metadata only if
                // this invocation will fully prove its new completed receipt.
                // Interrupted/unknown selectors conservatively use cold replay.
                let proves_current = entry.resume.original.selector_matches(current)
                    || self.records.iter().skip(entry.resume.count).any(|r| {
                        r.value["kind"] == "CheckpointReceipt"
                            && r.value["body"]["selected_head"] == current.head
                            && r.value["body"]["selected_commit"] == current.commit
                    });
                let tips = entry.resume.original.tips(context.reader.json)?;
                if proves_current
                    && entry.snapshot.matches(
                        current,
                        &restart::CompiledPlans::snapshot_context(current, context),
                        &tips,
                        context.inspection,
                        (
                            u64::try_from(context.frames.max_allocation_bytes)
                                .map_err(|_| PackageError::Limit)?
                                .min(context.planner.max_allocation_bytes),
                            context.planner.max_total_allocation_bytes,
                        ),
                    )?
                {
                    prior_observations = entry.observations.extended(context.inspection, &tips)?;
                }
            }
        }
        let hit = prior_observations.is_some();
        #[cfg(test)]
        if hit {
            memo.journal_hits += 1;
        }
        let (seed, observations) = if hit {
            let entry = memo.journal.as_ref().ok_or(PackageError::Integrity)?;
            (
                Some(entry.resume.clone()),
                prior_observations.ok_or(PackageError::Integrity)?,
            )
        } else {
            (None, restart::snapshot_memo::Observations::default())
        };
        let recording = restart::snapshot_memo::Recording::new(context.inspection, observations);
        let fresh = restart::RestartContext {
            inspection: &recording,
            registration: context.registration,
            directory: context.directory,
            identity: context.identity,
            frames: context.frames,
            reader: context.reader,
            planner: context.planner,
            max_originals: context.max_originals,
        };
        let (verified, resume) = self.verify_inner(current, &fresh, Some(memo), seed.as_ref())?;
        if let Some(resume) = resume {
            let observations = recording.finish();
            let snapshot = restart::snapshot_memo::PrefixSnapshot::new(
                current,
                restart::CompiledPlans::snapshot_context(current, context),
            );
            let cost = resume
                .cost()
                .and_then(|v| v.checked_add(observations.cost()?))
                .and_then(|v| v.checked_add(snapshot.cost()?))
                .and_then(|v| v.checked_add(key.capacity()));
            #[cfg(test)]
            if std::env::var_os("PHOTARA_COMPILER_CACHE_PROFILE").is_some() {
                eprintln!(
                    "journal memo candidate: cost={cost:?} limit={} hit={hit} records={}",
                    memo.raw_maximum(context.planner) / 8,
                    self.records.len()
                );
            }
            if let Some(cost) = cost.filter(|c| *c <= memo.raw_maximum(context.planner) / 8) {
                memo.journal = Some(JournalMemo {
                    key,
                    snapshot,
                    resume,
                    observations,
                    cost,
                });
                memo.trim(context.planner, context.max_originals);
            }
        }
        Ok(verified)
    }
    #[expect(
        clippy::too_many_lines,
        reason = "Ordered checkpoint validation and exact completed-prefix capture share one state machine"
    )]
    fn verify_inner<'a, I: AllocationInspection>(
        &'a self,
        current: &OriginalPackage,
        context: &RestartContext<'_, I>,
        mut memo: Option<&mut restart::CompiledPlans>,
        seed: Option<&JournalResume>,
    ) -> Result<(VerifiedJournal<'a>, Option<JournalResume>), PackageError> {
        // Snapshot segments require independently verified local-base custody;
        // framing/header equality alone must never seed semantic acceptance.
        if self.header["format_version"] != 1 {
            return Err(PackageError::UnsupportedVersion);
        }
        check(
            self.header["project_id"] == context.identity.project.to_string()
                && self.header["library_id"] == context.identity.library.to_string()
                && self.header["bootstrap_sha256"] == context.identity.bootstrap_sha256.as_str()
                && self.header["incarnation_id"] == context.registration.incarnation.to_string(),
        )?;
        let mut replay = restart::IntentReplay::new(current, context);
        let mut plans = seed
            .as_ref()
            .map_or_else(BTreeMap::new, |v| v.plans.clone());
        let mut operations = plans
            .values()
            .map(|p| id(&p.receipt["operation_id"]))
            .collect::<Result<BTreeSet<_>, _>>()?;
        let mut receipts = plans.keys().cloned().collect::<BTreeSet<_>>();
        let start = seed.as_ref().map_or(0, |v| v.count);
        let mut completed_count = start;
        let mut at = seed.as_ref().map_or_else(
            || 12 + wire::encode(&self.header).len() + 32,
            |v| v.prefix.len(),
        );
        let mut completed_end = at;
        if let Some(seed) = &seed {
            replay.seed(seed.original.clone(), seed.lineage.clone());
        }
        #[cfg(test)]
        if let Some(memo) = memo.as_deref_mut() {
            memo.journal_replayed = 0;
        }
        let mut pending: Option<String> = None;
        let mut pending_plan: Option<Arc<RepeatablePlan>> = None;
        let mut prior_head = seed
            .as_ref()
            .map_or_else(|| self.header["base_head"].clone(), |v| v.head.clone());
        if self.records.is_empty() {
            check(
                self.header["base_head"] == current.head
                    && n(&self.header["base_package_revision"])?
                        == n(&current.commit["package_revision"])?,
            )?;
        }
        for (index, record) in self.records.iter().enumerate().skip(start) {
            at = at
                .checked_add(36)
                .and_then(|n| n.checked_add(wire::encode(&record.value).len()))
                .ok_or(PackageError::Limit)?;
            #[cfg(test)]
            if let Some(memo) = memo.as_deref_mut() {
                memo.journal_replayed += 1;
            }
            let b = &record.value["body"];
            if record.value["kind"] == "CheckpointIntent" {
                let p = replay.restore_shared(
                    &b["original"],
                    &b["semantic_intent"],
                    &b["operation_receipt"],
                    memo.as_deref_mut(),
                )?;
                if plans.is_empty() {
                    check(
                        p.old.head == self.header["base_head"]
                            && p.old.commit["package_revision"]
                                == self.header["base_package_revision"],
                    )?;
                }
                check(pending.is_none() && p.old.head == prior_head)?;
                check(operations.insert(id(&b["operation_receipt"]["operation_id"])?))?;
                pending = Some(
                    record.value["record_id"]
                        .as_str()
                        .ok_or(PackageError::Record)?
                        .to_owned(),
                );
                plans.insert(
                    record.value["record_id"]
                        .as_str()
                        .ok_or(PackageError::Record)?
                        .to_owned(),
                    CheckpointPlan::from_plan(&p, context.planner.semantic.json)?,
                );
                pending_plan = Some(p);
            } else if record.value["kind"] == "CheckpointReceipt" {
                let target = b["intent_record_id"].as_str().ok_or(PackageError::Record)?;
                let p = pending_plan.as_ref().ok_or(PackageError::Integrity)?;
                let intent = self
                    .records
                    .iter()
                    .find(|r| r.value["record_id"] == target)
                    .ok_or(PackageError::Integrity)?;
                check(
                    pending.as_deref() == Some(target)
                        && receipts.insert(target.to_owned())
                        && *b == receipt_body(p, intent),
                )?;
                replay.complete(p, memo.as_deref_mut())?;
                prior_head = p.stages[6].head.clone();
                pending = None;
                pending_plan = None;
                completed_count = index + 1;
                completed_end = at;
            }
        }
        let resume = if memo.is_some() && completed_count > 0 {
            let (original, lineage) = replay.completed_metadata().ok_or(PackageError::Integrity)?;
            Some(JournalResume {
                prefix: self
                    .bytes
                    .get(..completed_end)
                    .ok_or(PackageError::Integrity)?
                    .to_vec(),
                count: completed_count,
                plans: plans
                    .iter()
                    .filter(|(id, _)| receipts.contains(*id))
                    .map(|(id, p)| (id.clone(), p.clone()))
                    .collect(),
                original,
                lineage,
                head: prior_head,
            })
        } else {
            None
        };
        #[cfg(test)]
        if let Some(memo) = memo {
            memo.journal_prefix_hashed = replay.prefix_hashed();
        }
        Ok((
            VerifiedJournal {
                journal: self,
                plans,
            },
            resume,
        ))
    }
}
fn intent_body(p: &RepeatablePlan) -> Value {
    let intent = p.old.loose(&p.original["request"]["intent"]).ok();
    // The intent lives in every generated selected control set, even before the
    // first package effect. Its canonical commitment is original-bound.
    let bytes = &p.stages[0].loose[p.original["request"]["intent"]["sha256"]
        .as_str()
        .expect("compiled intent")];
    let intent = intent.unwrap_or_else(|| wire::parse(bytes).expect("compiled canonical intent"));
    json!({"original":p.original,"semantic_intent":intent,"operation_receipt":p.prepared.receipt,"accepted_frame":plan::journal_frame(&p.prepared.receipt)})
}
fn receipt_body(p: &RepeatablePlan, intent: &JournalRecord) -> Value {
    json!({"intent_record_id":intent.value["record_id"],"intent_record_checksum":hex(&intent.checksum),"original_sha256":wire::hash(&p.original_bytes()),"operation_receipt":p.prepared.receipt,"selected_head":p.stages[6].head,"selected_commit":p.stages[6].commit})
}
impl VerifiedJournal<'_> {
    #[cfg(test)]
    pub(crate) fn retained_metadata_bytes(&self) -> usize {
        self.plans
            .values()
            .map(|p| {
                [
                    &p.original_ref,
                    &p.receipt,
                    &p.intent,
                    &p.selected_state,
                    &p.head,
                ]
                .into_iter()
                .map(|v| wire::encode(v).len())
                .sum::<usize>()
            })
            .sum()
    }
    #[must_use]
    pub fn records(&self) -> &[JournalRecord] {
        &self.journal.records
    }
    /// Produces exact original-bound intent bytes before any package effect.
    /// # Errors
    /// Refuses incomplete tails, identity reuse, unsupported plans or byte limits.
    pub fn prepare_intent(
        &self,
        p: &RepeatablePlan,
        identity: JournalRecordIdentity,
        limits: JournalLimits,
    ) -> Result<JournalAppend, PackageError> {
        check(!self.records().iter().any(|r| r.value["kind"] == "Mutation"))?;
        self.prepare_intent_inner(p, identity, limits)
    }
    fn prepare_intent_inner(
        &self,
        p: &RepeatablePlan,
        identity: JournalRecordIdentity,
        limits: JournalLimits,
    ) -> Result<JournalAppend, PackageError> {
        let b = intent_body(p);
        check(
            p.original["project_id"] == self.journal.header["project_id"]
                && p.prepared.receipt["journal_id"] == self.journal.header["journal_id"],
        )?;
        let existing = self.records().iter().any(|r| {
            r.value["kind"] == "CheckpointIntent"
                && r.value["body"]["operation_receipt"]["operation_id"]
                    == p.prepared.receipt["operation_id"]
        });
        if !existing {
            if let Some(last) = self.records().iter().rev().find(|r| {
                matches!(
                    r.value["kind"].as_str(),
                    Some("CheckpointIntent" | "CheckpointReceipt")
                )
            }) {
                check(
                    last.value["kind"] == "CheckpointReceipt"
                        && p.old.head == last.value["body"]["selected_head"],
                )?;
            } else {
                check(
                    p.old.head == self.journal.header["base_head"]
                        && p.old.commit["package_revision"]
                            == self.journal.header["base_package_revision"],
                )?;
            }
        }
        self.prepare("CheckpointIntent", &b, identity, limits)
    }
    /// Prepares a completed-selector receipt; caller must first complete shared
    /// live closure verification and qualified package barriers. These bytes are
    /// not a durable completion until the journal's own barriers also succeed.
    /// # Errors
    /// Refuses absent/different original intents, incomplete tails or conflicts.
    pub fn prepare_receipt(
        &self,
        p: &RepeatablePlan,
        intent_record_id: PackageUuid,
        identity: JournalRecordIdentity,
        limits: JournalLimits,
    ) -> Result<JournalAppend, PackageError> {
        let name = intent_record_id.to_string();
        let original = self.plans.get(&name).ok_or(PackageError::Integrity)?;
        check(original.original_ref == wire::reference(&p.original))?;
        let intent = self
            .records()
            .iter()
            .find(|r| r.value["record_id"] == name)
            .ok_or(PackageError::Integrity)?;
        // The compact proof stores only the commitment; exact Original equality
        // is still checked against the current, canonically parsed journal.
        check(intent.value["body"]["original"] == p.original)?;
        self.prepare(
            "CheckpointReceipt",
            &receipt_body(p, intent),
            identity,
            limits,
        )
    }
    fn prepare(
        &self,
        kind: &str,
        b: &Value,
        identity: JournalRecordIdentity,
        limits: JournalLimits,
    ) -> Result<JournalAppend, PackageError> {
        check(self.journal.tail == JournalTail::Complete && identity.session_generation > 0)?;
        if self.journal.bytes.len() > limits.max_file_bytes
            || self.records().len() > limits.max_records
        {
            return Err(PackageError::Limit);
        }
        let existing_work = self.journal.work;
        if existing_work > limits.max_work_bytes {
            return Err(PackageError::Limit);
        }
        let record_id = identity.record_id.to_string();
        let generation = identity.session_generation.to_string();
        id(&json!(record_id))?;
        for record in self.records() {
            let same_operation = record.value["kind"] == kind
                && if matches!(kind, "CheckpointIntent" | "Mutation") {
                    record.value["body"]["operation_receipt"]["operation_id"]
                        == b["operation_receipt"]["operation_id"]
                } else if kind == "CheckpointReceipt" {
                    record.value["body"]["intent_record_id"] == b["intent_record_id"]
                } else {
                    false
                };
            if record.value["record_id"] == record_id || same_operation {
                check(
                    record.value["record_id"] == record_id
                        && record.value["session_generation"].as_str() == Some(generation.as_str())
                        && record.value["kind"] == kind
                        && &record.value["body"] == b,
                )?;
                return Ok(JournalAppend {
                    bytes: Vec::new(),
                    expected_extent: self.journal.bytes.len(),
                    expected_prefix_sha256: wire::hash(&self.journal.bytes),
                    record: record.clone(),
                    existing: true,
                });
            }
        }
        if self.records().len() >= limits.max_records {
            return Err(PackageError::Limit);
        }
        let sequence = u64::try_from(self.records().len())
            .map_err(|_| PackageError::Limit)?
            .checked_add(1)
            .and_then(|n| n.checked_add(sequence_base(&self.journal.header).ok()?))
            .ok_or(PackageError::Limit)?;
        let value = json!({"format_version":self.journal.header["format_version"],"sequence":sequence.to_string(),"record_id":record_id,"session_generation":identity.session_generation.to_string(),"project_id":self.journal.header["project_id"],"incarnation_id":self.journal.header["incarnation_id"],"kind":kind,"body":b});
        envelope(&value, &self.journal.header, sequence)?;
        let raw = wire::encode(&value);
        let len = u32::try_from(raw.len())
            .map_err(|_| PackageError::Limit)?
            .to_le_bytes();
        let mut work = existing_work;
        payload(&raw, limits, &mut work)?;
        let end = self
            .journal
            .bytes
            .len()
            .checked_add(4)
            .and_then(|n| n.checked_add(raw.len()))
            .and_then(|n| n.checked_add(32))
            .ok_or(PackageError::Limit)?;
        if end > limits.max_file_bytes {
            return Err(PackageError::Limit);
        }
        let checksum = hash_parts(&[&self.journal.last, &len, &raw]);
        let mut bytes = Vec::with_capacity(end - self.journal.bytes.len());
        bytes.extend(len);
        bytes.extend(raw);
        bytes.extend(checksum);
        Ok(JournalAppend {
            bytes,
            expected_extent: self.journal.bytes.len(),
            expected_prefix_sha256: wire::hash(&self.journal.bytes),
            record: JournalRecord {
                value,
                checksum,
                frame_end: end,
            },
            existing: false,
        })
    }
}

/// Checks an embedded checkpoint envelope's exact existing shape. Its checksum
/// still requires resolution in the original registered journal, by the caller.
pub(crate) fn validate_detached_checkpoint(value: &Value) -> Result<(), PackageError> {
    if !matches!(
        value["kind"].as_str(),
        Some("CheckpointIntent" | "CheckpointReceipt")
    ) {
        return Err(PackageError::UnsupportedVersion);
    }
    let receipt = &value["body"]["operation_receipt"];
    let header = json!({"project_id":value["project_id"],"incarnation_id":value["incarnation_id"],"library_id":receipt["library_id"],"bootstrap_sha256":receipt["bootstrap_sha256"],"journal_id":receipt["journal_id"]});
    let sequence = n(&value["sequence"])?;
    check(sequence > 0)?;
    envelope(value, &header, sequence)
}

pub(super) fn validate_archived_mutation(
    value: &Value,
    identity: &super::checkpoint::CheckpointIdentity,
) -> Result<(), PackageError> {
    check(value["kind"] == "Mutation")?;
    check(matches!(value["format_version"].as_u64(), Some(1 | 2)))?;
    let receipt = &value["body"]["operation_receipt"];
    let header = json!({"format_version":value["format_version"],"sequence_base":"0",
        "project_id":identity.package.project.to_string(),"library_id":identity.package.library.to_string(),
        "bootstrap_sha256":identity.package.bootstrap_sha256.as_str(),
        "incarnation_id":identity.incarnation.to_string(),"journal_id":receipt["journal_id"]});
    let sequence = n(&value["sequence"])?;
    check(sequence > 0)?;
    envelope(value, &header, sequence)
}

impl Journal {
    pub(super) fn checkpoint_archive(&self, index: usize) -> Result<Value, PackageError> {
        let record = self.records.get(index).ok_or(PackageError::Integrity)?;
        let start = if index == 0 {
            12usize
                .checked_add(wire::encode(&self.header).len())
                .and_then(|v| v.checked_add(32))
                .ok_or(PackageError::Limit)?
        } else {
            self.records[index - 1].frame_end
        };
        let length = record
            .frame_end
            .checked_sub(start)
            .ok_or(PackageError::Integrity)?;
        Ok(
            json!({"stream_id":self.header["stream_id"],"offset":start.to_string(),"byte_length":length.to_string(),"record":wire::reference(&record.value),"checksum":record.checksum()}),
        )
    }
    pub(super) fn checkpoint_segment(&self) -> Result<Value, PackageError> {
        check(self.tail == JournalTail::Complete)?;
        let first = self.records.first().ok_or(PackageError::Integrity)?;
        let last = self.records.last().ok_or(PackageError::Integrity)?;
        let header_end = 12usize
            .checked_add(wire::encode(&self.header).len())
            .and_then(|v| v.checked_add(32))
            .ok_or(PackageError::Limit)?;
        Ok(
            json!({"stream_id":self.header["stream_id"],"journal_id":self.header["journal_id"],"header_sha256":wire::hash(&self.bytes[..header_end]),"byte_length":self.bytes.len().to_string(),"sha256":wire::hash(&self.bytes),"first_sequence":first.value["sequence"],"last_sequence":last.value["sequence"],"last_checksum":last.checksum()}),
        )
    }
}
