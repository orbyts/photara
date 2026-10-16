//! Approved device-local checkpoint records. Parsed records are commitments,
//! never a registered custody grant, writable lease, or durable Saved evidence.
use super::wire;
use crate::package::v1_3::{SelectionIdentity, selected, tree};
use crate::package::{JsonLimits, PackageError, PackageUuid, parse_canonical_json};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub struct CheckpointIdentity {
    pub package: SelectionIdentity,
    pub device: PackageUuid,
    pub incarnation: PackageUuid,
}
impl Clone for CheckpointIdentity {
    fn clone(&self) -> Self {
        Self {
            package: SelectionIdentity {
                project: self.package.project,
                library: self.package.library,
                bootstrap_sha256: self.package.bootstrap_sha256.clone(),
            },
            device: self.device,
            incarnation: self.incarnation,
        }
    }
}

fn check(value: bool) -> Result<(), PackageError> {
    if value {
        Ok(())
    } else {
        Err(PackageError::Integrity)
    }
}
fn common(
    value: &Value,
    name: &str,
    extra: &[&str],
    identity: &CheckpointIdentity,
) -> Result<(), PackageError> {
    let mut fields = vec![
        "schema",
        "project_id",
        "library_id",
        "bootstrap_sha256",
        "extensions",
        "device_id",
        "incarnation_id",
    ];
    fields.extend_from_slice(extra);
    tree::fields(value, &fields)?;
    check(
        value["schema"] == json!({"id":name,"version":1})
            && value["project_id"] == identity.package.project.to_string()
            && value["library_id"] == identity.package.library.to_string()
            && value["bootstrap_sha256"] == identity.package.bootstrap_sha256.as_str()
            && value["device_id"] == identity.device.to_string()
            && value["incarnation_id"] == identity.incarnation.to_string()
            && value["extensions"] == json!({}),
    )?;
    for id in [
        identity.package.project,
        identity.package.library,
        identity.device,
        identity.incarnation,
    ] {
        check(!id.as_uuid().is_nil())?;
    }
    Ok(())
}
pub(super) fn link(value: &Value) -> Result<(), PackageError> {
    tree::fields(value, &["record_id", "checksum", "sequence"])?;
    selected::nonnil(&value["record_id"])?;
    selected::sha(&value["checksum"])?;
    check(tree::number(&value["sequence"])? > 0)
}
fn head(value: &Value, identity: &CheckpointIdentity) -> Result<(), PackageError> {
    tree::fields(
        value,
        &["schema", "project_id", "commit_id", "commit_sha256"],
    )?;
    selected::nonnil(&value["commit_id"])?;
    selected::sha(&value["commit_sha256"])?;
    check(
        value["schema"] == json!({"id":"photara.package.head","version":1})
            && value["project_id"] == identity.package.project.to_string(),
    )
}
/// Canonical local base with exact structural references. Package/journal
/// adoption must independently validate every referenced commitment.
#[derive(Clone)]
pub struct LocalBase {
    value: Value,
}
impl LocalBase {
    /// # Errors
    /// Refuses unknown fields, wrong identity, malformed links/coordinates,
    /// inconsistent mutation/frame presence or unordered/duplicate undo roots.
    pub fn parse(
        bytes: &[u8],
        identity: &CheckpointIdentity,
        limits: JsonLimits,
    ) -> Result<Self, PackageError> {
        let value = parse_canonical_json(bytes, limits)?;
        common(
            &value,
            "photara.session.checkpoint-base",
            &[
                "checkpoint",
                "package_head",
                "package_commit",
                "journal_id",
                "through",
                "mutation",
                "accepted_frame",
                "coordinate",
                "undo_roots",
                "lookup",
                "archive",
            ],
            identity,
        )?;
        for key in ["checkpoint", "package_commit", "lookup", "archive"] {
            selected::json_ref(&value[key])?;
        }
        head(&value["package_head"], identity)?;
        check(value["package_head"]["commit_sha256"] == value["package_commit"]["sha256"])?;
        selected::nonnil(&value["journal_id"])?;
        link(&value["through"])?;
        super::journal::session::coordinate(&value["coordinate"])?;
        check(value["mutation"].is_null() == value["accepted_frame"].is_null())?;
        if !value["mutation"].is_null() {
            link(&value["mutation"])?;
            check(
                tree::number(&value["mutation"]["sequence"])?
                    <= tree::number(&value["through"]["sequence"])?,
            )?;
            let frame = &value["accepted_frame"];
            tree::fields(
                frame,
                &[
                    "schema",
                    "journal_id",
                    "sequence",
                    "operation_id",
                    "request_sha256",
                    "receipt_sha256",
                ],
            )?;
            check(
                frame["schema"]
                    == json!({"id":"photara.package.accepted-journal-frame","version":1})
                    && frame["journal_id"] == value["journal_id"]
                    && tree::number(&frame["sequence"])? > 0,
            )?;
            selected::nonnil(&frame["operation_id"])?;
            selected::sha(&frame["request_sha256"])?;
            selected::sha(&frame["receipt_sha256"])?;
        }
        let mut prior = None;
        for target in value["undo_roots"].as_array().ok_or(PackageError::Record)? {
            super::journal::session::target(target)?;
            let key = (
                target["operation_id"]
                    .as_str()
                    .ok_or(PackageError::Record)?
                    .to_owned(),
                wire::encode(&target["mutation"]),
            );
            check(prior.as_ref().is_none_or(|old| old < &key))?;
            prior = Some(key);
        }
        Ok(Self { value })
    }
    #[must_use]
    pub fn value(&self) -> &Value {
        &self.value
    }
    /// Check the complete PHPSJ002 header's base/identity/continuation binding.
    /// This does not establish that the local selector selected this base.
    /// # Errors
    /// Refuses a substituted base, coordinate, old-segment link or identity.
    pub fn check_header(&self, header: &Value) -> Result<(), PackageError> {
        check(
            header["format_version"] == 2
                && header["base"] == wire::reference(&self.value)
                && header["base_head"] == self.value["package_head"]
                && header["journal_id"] == self.value["journal_id"]
                && header["sequence_base"] == self.value["through"]["sequence"]
                && header["previous_segment"]["last"] == self.value["through"],
        )?;
        for key in [
            "project_id",
            "library_id",
            "bootstrap_sha256",
            "device_id",
            "incarnation_id",
        ] {
            check(header[key] == self.value[key])?;
        }
        Ok(())
    }
}
/// Exact selector shape. CAS and package/segment qualification are separate.
pub struct LocalSelection {
    value: Value,
}
impl LocalSelection {
    /// # Errors
    /// Refuses unknown formats/fields, malformed identities or inconsistent base.
    pub fn parse(
        bytes: &[u8],
        identity: &CheckpointIdentity,
        limits: JsonLimits,
    ) -> Result<Self, PackageError> {
        let value = parse_canonical_json(bytes, limits)?;
        common(
            &value,
            "photara.session.checkpoint-selection",
            &["generation", "previous_sha256", "selected", "pending"],
            identity,
        )?;
        check(tree::number(&value["generation"])? > 0)?;
        if !value["previous_sha256"].is_null() {
            selected::sha(&value["previous_sha256"])?;
        }
        let selection = &value["selected"];
        tree::fields(selection, &["format", "stream_id", "header_sha256", "base"])?;
        selected::nonnil(&selection["stream_id"])?;
        selected::sha(&selection["header_sha256"])?;
        match selection["format"].as_str() {
            Some("PHPSJ001") => check(selection["base"].is_null())?,
            Some("PHPSJ002") => {
                selected::json_ref(&selection["base"])?;
            }
            _ => return Err(PackageError::UnsupportedVersion),
        }
        if !value["pending"].is_null() {
            selected::json_ref(&value["pending"])?;
        }
        Ok(Self { value })
    }
    #[must_use]
    pub fn value(&self) -> &Value {
        &self.value
    }
    /// Exact immutable predecessor commitment and monotonic generation check.
    /// # Errors
    /// Refuses changed registration identity, skipped generations or stale CAS.
    pub fn successor(&self, next: &Self, expected_sha256: &str) -> Result<(), PackageError> {
        check(wire::hash(&wire::encode(&self.value)) == expected_sha256)?;
        if self.value == next.value {
            return Ok(());
        }
        for key in [
            "project_id",
            "library_id",
            "bootstrap_sha256",
            "device_id",
            "incarnation_id",
        ] {
            check(self.value[key] == next.value[key])?;
        }
        check(
            next.value["previous_sha256"] == expected_sha256
                && tree::number(&next.value["generation"])?
                    == tree::number(&self.value["generation"])?
                        .checked_add(1)
                        .ok_or(PackageError::Limit)?,
        )
    }
}
/// Canonical closed-segment metadata; previous is a historical lookup edge.
pub struct LocalArchive {
    value: Value,
}
impl LocalArchive {
    /// # Errors
    /// Refuses duplicate streams, overlapping/unsorted sequence ranges, malformed
    /// digests/identities or empty segment ranges.
    pub fn parse(
        bytes: &[u8],
        identity: &CheckpointIdentity,
        limits: JsonLimits,
    ) -> Result<Self, PackageError> {
        let value = parse_canonical_json(bytes, limits)?;
        common(
            &value,
            "photara.session.checkpoint-archive",
            &["archive_id", "previous", "segments"],
            identity,
        )?;
        selected::nonnil(&value["archive_id"])?;
        if !value["previous"].is_null() {
            selected::json_ref(&value["previous"])?;
        }
        let mut prior: Option<(PackageUuid, u64)> = None;
        let mut streams = BTreeSet::new();
        for segment in value["segments"].as_array().ok_or(PackageError::Record)? {
            tree::fields(
                segment,
                &[
                    "stream_id",
                    "journal_id",
                    "header_sha256",
                    "byte_length",
                    "sha256",
                    "first_sequence",
                    "last_sequence",
                    "last_checksum",
                ],
            )?;
            check(streams.insert(selected::nonnil(&segment["stream_id"])?))?;
            let journal = selected::nonnil(&segment["journal_id"])?;
            for key in ["header_sha256", "sha256", "last_checksum"] {
                selected::sha(&segment[key])?;
            }
            let first = tree::number(&segment["first_sequence"])?;
            let last = tree::number(&segment["last_sequence"])?;
            check(first > 0 && first <= last && tree::number(&segment["byte_length"])? >= 44)?;
            if let Some((old, end)) = prior {
                check(old < journal || (old == journal && end < first))?;
            }
            prior = Some((journal, last));
        }
        Ok(Self { value })
    }
    #[must_use]
    pub fn value(&self) -> &Value {
        &self.value
    }
}

#[path = "checkpoint_history.rs"]
mod history;
pub use history::PreparedHistory;

#[path = "checkpoint_lookup.rs"]
pub(crate) mod lookup;
pub use lookup::CheckpointRecords;

#[path = "checkpoint_plan.rs"]
mod plan;
pub use plan::{TurnoverIds, TurnoverPlan};

#[path = "checkpoint_activate.rs"]
mod activate;
#[cfg(all(target_os = "macos", any(test, feature = "controlled-disposable")))]
pub(crate) use activate::prepare_installation;
pub use activate::{
    ActivatedCheckpoint, ActivationLimits, CheckpointIo, CheckpointRecoveryIo, ReopenState,
    activate, prepare_recovery_receipt, reopen,
};

#[path = "checkpoint_continue.rs"]
mod continuation;
