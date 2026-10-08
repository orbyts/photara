//! Approved PHPSJ001 checkpoints and PS3 local records. Append preparation
//! never establish native durability or mint Accepted/Saved acknowledgements.
use super::{OriginalPackage, RepeatablePlan, RestartContext, plan, restart, wire};
use crate::package::v1_3::{AllocationInspection, selected, tree};
use crate::package::{JsonLimits, PackageError, PackageUuid, Sha256Hex, parse_canonical_json};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[path = "journal_session.rs"]
pub(crate) mod session;
pub use session::VerifiedSession;

const MAGIC: &[u8; 8] = b"PHPSJ001";
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
}
/// Checkpoint-history-verified stream, not native permission or durability.
/// Local Mutation/lifecycle evidence additionally requires `verify_session`.
pub struct VerifiedJournal<'a> {
    journal: &'a Journal,
    plans: BTreeMap<String, RepeatablePlan>,
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
    fields(
        v,
        &[
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
        ],
    )?;
    if v["format_version"] != 1 {
        return Err(PackageError::UnsupportedVersion);
    }
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
            check(
                body["original"]["original_codec"] == "photara.codec.ps2-repeatable-admission-v1",
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
    if v["format_version"] != 1 {
        return Err(PackageError::UnsupportedVersion);
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
fn payload(bytes: &[u8], limits: JournalLimits, work: &mut usize) -> Result<Value, PackageError> {
    *work = work.checked_add(bytes.len()).ok_or(PackageError::Limit)?;
    if *work > limits.max_work_bytes {
        return Err(PackageError::Limit);
    }
    parse_canonical_json(bytes, limits.json)
}
impl Journal {
    /// Creates only the approved exact header bytes, with explicit process bounds.
    /// # Errors
    /// Refuses unsupported/malformed header fields or caller byte/work ceilings.
    pub fn create(header_value: &Value, limits: JournalLimits) -> Result<Vec<u8>, PackageError> {
        header(header_value)?;
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
        out.extend(MAGIC);
        out.extend(len);
        out.extend(&bytes);
        out.extend(hash_parts(&[MAGIC, &len, &bytes]));
        Ok(out)
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
        if bytes.len() > limits.max_file_bytes {
            return Err(PackageError::Limit);
        }
        check(bytes.get(..8) == Some(MAGIC.as_slice()))?;
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
        let mut last = hash_parts(&[MAGIC, &raw_len, raw]);
        check(bytes.get(header_end..at) == Some(last.as_slice()))?;
        let mut work = 0;
        let h = payload(raw, limits, &mut work)?;
        header(&h)?;
        check(h == *expected_header)?;
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
            let value = payload(raw, limits, &mut work)?;
            let sequence = u64::try_from(records.len())
                .map_err(|_| PackageError::Limit)?
                .checked_add(1)
                .ok_or(PackageError::Limit)?;
            envelope(&value, &h, sequence)?;
            check(ids.insert(id(&value["record_id"])?))?;
            records.push(JournalRecord { value, checksum });
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
        current: &OriginalPackage,
        context: &RestartContext<'_, I>,
    ) -> Result<VerifiedJournal<'a>, PackageError> {
        check(
            self.header["project_id"] == context.identity.project.to_string()
                && self.header["library_id"] == context.identity.library.to_string()
                && self.header["bootstrap_sha256"] == context.identity.bootstrap_sha256.as_str()
                && self.header["incarnation_id"] == context.registration.incarnation.to_string(),
        )?;
        let mut replay = restart::IntentReplay::new(current, context);
        let mut plans = BTreeMap::new();
        let mut operations = BTreeSet::new();
        let mut receipts = BTreeSet::new();
        let mut pending: Option<String> = None;
        let mut prior_head = self.header["base_head"].clone();
        if self.records.is_empty() {
            check(
                self.header["base_head"] == current.head
                    && n(&self.header["base_package_revision"])?
                        == n(&current.commit["package_revision"])?,
            )?;
        }
        for record in &self.records {
            let b = &record.value["body"];
            if record.value["kind"] == "CheckpointIntent" {
                let p = replay.restore(
                    &b["original"],
                    &b["semantic_intent"],
                    &b["operation_receipt"],
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
                    p,
                );
            } else if record.value["kind"] == "CheckpointReceipt" {
                let target = b["intent_record_id"].as_str().ok_or(PackageError::Record)?;
                let p = plans.get(target).ok_or(PackageError::Integrity)?;
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
                replay.complete(p)?;
                prior_head = p.stages[6].head.clone();
                pending = None;
            }
        }
        Ok(VerifiedJournal {
            journal: self,
            plans,
        })
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
        check(original.original == p.original)?;
        let intent = self
            .records()
            .iter()
            .find(|r| r.value["record_id"] == name)
            .ok_or(PackageError::Integrity)?;
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
        let existing_work =
            self.records()
                .iter()
                .try_fold(wire::encode(&self.journal.header).len(), |n, r| {
                    n.checked_add(wire::encode(&r.value).len())
                        .ok_or(PackageError::Limit)
                })?;
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
            .ok_or(PackageError::Limit)?;
        let value = json!({"format_version":1,"sequence":sequence.to_string(),"record_id":record_id,"session_generation":identity.session_generation.to_string(),"project_id":self.journal.header["project_id"],"incarnation_id":self.journal.header["incarnation_id"],"kind":kind,"body":b});
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
            record: JournalRecord { value, checksum },
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
