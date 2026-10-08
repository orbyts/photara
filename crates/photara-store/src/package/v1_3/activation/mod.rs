//! Approved device-local PS4 snapshots. Structural consistency is not live
//! storage qualification, authorization, journal verification, or Saved evidence.
use super::{selected, tree};
use crate::package::{JsonLimits, PackageError, PackageUuid, Sha256Hex, parse_canonical_json};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
mod records;
mod transition;
pub use records::{default_view, validate_view};
pub use transition::{Transition, validate_transition};
#[derive(Clone, Copy)]
pub struct Limits {
    pub json: JsonLimits,
    pub max_records: usize,
    pub max_snapshot_bytes: usize,
}
#[derive(Clone)]
pub struct Snapshot {
    bytes: Vec<u8>,
    value: Value,
    records: BTreeMap<String, Value>,
    digest: String,
}
fn check(ok: bool) -> Result<(), PackageError> {
    if ok {
        Ok(())
    } else {
        Err(PackageError::Integrity)
    }
}
fn id(value: &Value) -> Result<PackageUuid, PackageError> {
    selected::nonnil(value)
}
fn digest(value: &Value) -> Result<(), PackageError> {
    Sha256Hex::parse(value.as_str().ok_or(PackageError::Record)?).map(|_| ())
}
fn number(value: &Value) -> Result<u64, PackageError> {
    tree::number(value)
}
fn fields(value: &Value, names: &[&str]) -> Result<(), PackageError> {
    tree::fields(value, names)
}
fn encode(value: &Value) -> Result<Vec<u8>, PackageError> {
    photara_core::canonical_json(value).map_err(|_| PackageError::Record)
}
fn hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}
// Validate caller-owned Values before the canonical encoder allocates/clones.
fn bounded(value: &Value, limits: Limits, depth: usize) -> Result<(), PackageError> {
    if depth > limits.json.max_depth {
        return Err(PackageError::Limit);
    }
    match value {
        Value::Array(rows) => {
            if rows.len() > limits.json.max_array_elements {
                return Err(PackageError::Limit);
            }
            for row in rows {
                bounded(row, limits, depth + 1)?;
            }
        }
        Value::Object(rows) => {
            if rows.len() > limits.json.max_members {
                return Err(PackageError::Limit);
            }
            for row in rows.values() {
                bounded(row, limits, depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}
struct ByteBound(usize);
impl std::io::Write for ByteBound {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self
            .0
            .checked_sub(bytes.len())
            .ok_or_else(|| std::io::Error::other("snapshot byte bound"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
/// Exact immutable record reference; does not grant use of its evidence.
/// # Errors
/// Refuses a malformed record identity or noncanonical JSON representation.
pub fn record_ref(record: &Value) -> Result<Value, PackageError> {
    id(&record["id"])?;
    Ok(json!({"id":record["id"],"sha256":hash(&encode(record)?)}))
}
fn reference(value: &Value) -> Result<(), PackageError> {
    fields(value, &["id", "sha256"])?;
    id(&value["id"])?;
    digest(&value["sha256"])
}
impl Snapshot {
    /// Parses bounded canonical bytes and validates all authority record shapes,
    /// immutable references and linked identity constraints. Optional view errors
    /// remain distinguishable; callers must persist valid defaults before use.
    /// # Errors
    /// Refuses unsupported formats, corruption, inconsistent evidence or limits.
    pub fn parse(bytes: &[u8], limits: Limits) -> Result<Self, PackageError> {
        if bytes.len() > limits.max_snapshot_bytes {
            return Err(PackageError::Limit);
        }
        let value = parse_canonical_json(bytes, limits.json)?;
        fields(&value, &["format", "version", "body", "body_sha256"])?;
        if value["format"] != "photara.local.activation-snapshot" || value["version"] != 1 {
            return Err(PackageError::UnsupportedVersion);
        }
        digest(&value["body_sha256"])?;
        check(value["body_sha256"] == hash(&encode(&value["body"])?))?;
        let body = &value["body"];
        fields(
            body,
            &[
                "device_id",
                "workspace_slot_id",
                "revision",
                "request_generation",
                "committed_generation",
                "authority_scope_sha256",
                "active",
                "pending",
                "records",
            ],
        )?;
        id(&body["device_id"])?;
        id(&body["workspace_slot_id"])?;
        digest(&body["authority_scope_sha256"])?;
        for key in ["revision", "request_generation", "committed_generation"] {
            number(&body[key])?;
        }
        check(number(&body["committed_generation"])? <= number(&body["request_generation"])?)?;
        let rows = body["records"].as_array().ok_or(PackageError::Record)?;
        if rows.len() > limits.max_records {
            return Err(PackageError::Limit);
        }
        let mut records = BTreeMap::new();
        let mut prior = None;
        for record in rows {
            fields(record, &["id", "kind", "version", "body"])?;
            let key = id(&record["id"])?;
            check(prior.is_none_or(|p| p < key))?;
            prior = Some(key);
            records::shape(record)?;
            records.insert(key.to_string(), record.clone());
        }
        let snapshot = Self {
            bytes: bytes.to_vec(),
            value,
            records,
            digest: hash(bytes),
        };
        for record in snapshot.records.values() {
            records::links(&snapshot, record)?;
        }
        snapshot.pointers()?;
        Ok(snapshot)
    }
    /// Creates the exact canonical envelope and immediately validates it.
    /// # Errors
    /// Refuses the same malformed state and budgets as `parse`.
    pub fn from_body(body: Value, limits: Limits) -> Result<Self, PackageError> {
        // Count/canonical byte budgets before retaining another full snapshot.
        if body["records"]
            .as_array()
            .is_some_and(|rows| rows.len() > limits.max_records)
        {
            return Err(PackageError::Limit);
        }
        bounded(&body, limits, 1)?;
        serde_json::to_writer(
            ByteBound(limits.max_snapshot_bytes.min(limits.json.max_bytes)),
            &body,
        )
        .map_err(|_| PackageError::Limit)?;
        let raw = encode(&body)?;
        if raw.len() > limits.max_snapshot_bytes || raw.len() > limits.json.max_bytes {
            return Err(PackageError::Limit);
        }
        let mut value = json!({"format":"photara.local.activation-snapshot","version":1,"body_sha256":hash(&raw),"body":null});
        value["body"] = body;
        Self::parse(&encode(&value)?, limits)
    }
    #[must_use]
    pub fn body(&self) -> &Value {
        &self.value["body"]
    }
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.digest
    }
    /// Resolves an exact immutable reference in this snapshot.
    /// # Errors
    /// Refuses malformed, missing or digest-mismatched references.
    pub fn record(&self, r: &Value) -> Result<&Value, PackageError> {
        reference(r)?;
        let value = self
            .records
            .get(r["id"].as_str().ok_or(PackageError::Record)?)
            .ok_or(PackageError::Integrity)?;
        check(record_ref(value)? == *r)?;
        Ok(value)
    }
    fn kind(&self, r: &Value, kind: &str) -> Result<&Value, PackageError> {
        let value = self.record(r)?;
        check(value["kind"] == kind)?;
        Ok(&value["body"])
    }
    fn optional(&self, r: &Value, kind: &str) -> Result<(), PackageError> {
        if !r.is_null() {
            self.kind(r, kind)?;
        }
        Ok(())
    }
    #[must_use]
    pub fn active(&self) -> Option<&Value> {
        (!self.body()["active"].is_null()).then_some(&self.body()["active"])
    }
    #[must_use]
    pub fn pending(&self) -> Option<&Value> {
        (!self.body()["pending"].is_null()).then_some(&self.body()["pending"])
    }
    /// Finds a retained original receipt without creating a replacement request.
    #[must_use]
    pub fn receipt(&self, activation_id: &str) -> Option<&Value> {
        self.records.values().find(|r| {
            r["kind"] == "ActivationReceipt" && r["body"]["intent"]["id"] == activation_id
        })
    }
    fn pointers(&self) -> Result<(), PackageError> {
        if let Some(active) = self.active() {
            let body = self.kind(active, "ActiveSession")?;
            check(
                body["device_id"] == self.body()["device_id"]
                    && body["workspace_slot_id"] == self.body()["workspace_slot_id"]
                    && body["committed_generation"] == self.body()["committed_generation"]
                    && body["authority_scope_sha256"] == self.body()["authority_scope_sha256"],
            )?;
            let receipt = self
                .receipt(body["activation_id"].as_str().ok_or(PackageError::Record)?)
                .ok_or(PackageError::Integrity)?;
            check(
                receipt["body"]["outcome"] == "Activated" && receipt["body"]["active"] == *active,
            )?;
        } else {
            check(number(&self.body()["committed_generation"])? == 0)?;
        }
        if let Some(pending) = self.pending() {
            let progress = self.kind(pending, "ActivationProgress")?;
            let intent = self.kind(&progress["intent"], "ActivationIntent")?;
            check(
                intent["request_generation"] == self.body()["request_generation"]
                    && intent["expected_committed_generation"]
                        == self.body()["committed_generation"]
                    && intent["source_active"] == self.body()["active"]
                    && intent["device_id"] == self.body()["device_id"]
                    && intent["workspace_slot_id"] == self.body()["workspace_slot_id"]
                    && intent["authority_scope_sha256"] == self.body()["authority_scope_sha256"]
                    && self
                        .receipt(
                            progress["intent"]["id"]
                                .as_str()
                                .ok_or(PackageError::Record)?,
                        )
                        .is_none(),
            )?;
        }
        let mut receipts = BTreeSet::new();
        for value in self
            .records
            .values()
            .filter(|r| r["kind"] == "ActivationReceipt")
        {
            check(
                receipts.insert(
                    value["body"]["intent"]["id"]
                        .as_str()
                        .ok_or(PackageError::Record)?,
                ),
            )?;
        }
        Ok(())
    }
}
