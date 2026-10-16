//! Registered maintenance startup. All parsed retained bytes are re-derived
//! from independently verified old package and journal evidence before effects.
use super::{
    ActivationLimits, CHECKPOINT, CheckpointRecords, IoResult, LocalSelection, Native, NativeLease,
    PackageError, PackageUuid, PreparedHistory, Records, ReopenState, TurnoverPlan, Value,
    canonical, frames, hash, invalid, read, refused, segment_name, snapshot, support, v1_3,
};
#[cfg(test)]
use super::{Write, record_name, regular};
use crate::package::v1_3::repeatable::{checkpoint, recovery};
use std::sync::Arc;

fn checked(value: bool) -> Result<(), PackageError> {
    if value {
        Ok(())
    } else {
        Err(PackageError::Integrity)
    }
}
fn value(records: &Records, reference: &Value) -> Result<Value, PackageError> {
    super::super::package::parse_canonical_json(
        &records.record(reference, support::limits().json.max_bytes)?,
        support::limits().json,
    )
}
fn source_package(
    current: &super::super::OriginalPackage,
    original: &Value,
    records: &Records,
) -> Result<super::super::OriginalPackage, PackageError> {
    let mut old = current.clone();
    old.head = original["old"]["head"].clone();
    old.commit = original["old"]["commit"].clone();
    let head_hash = hash(&canonical(&old.head));
    for corridor in original["attempt"]["planner"]["corridors"]
        .as_array()
        .ok_or(PackageError::Record)?
    {
        checked(
            corridor["base_head_sha256"] == head_hash && corridor.get("original_sha256").is_none(),
        )?;
        let allocation = old
            .allocations
            .get_mut(
                corridor["allocation_id"]
                    .as_str()
                    .ok_or(PackageError::Record)?,
            )
            .ok_or(PackageError::Integrity)?;
        checked(
            allocation.arena == corridor["arena"] && allocation.witness == corridor["witness"],
        )?;
        let end =
            usize::try_from(super::super::number(&corridor["original_end"]).map_err(refused)?)
                .map_err(|_| PackageError::Limit)?;
        allocation.bytes = allocation
            .bytes
            .prefix(end)
            .ok_or(PackageError::Integrity)?;
    }
    let overlay = value(records, &old.commit["root_set"]["inventory"])?;
    let controls = overlay["controls"].as_array().ok_or(PackageError::Record)?;
    checked(controls.len() <= super::super::budget().max_objects)?;
    old.loose.clear();
    let mut total = 0usize;
    for reference in controls
        .iter()
        .chain(std::iter::once(&old.commit["root_set"]["inventory"]))
    {
        let bytes = records.record(reference, support::limits().json.max_bytes)?;
        total = total.checked_add(bytes.len()).ok_or(PackageError::Limit)?;
        if total > super::super::budget().semantic.max_total_json_bytes {
            return Err(PackageError::Limit);
        }
        checked(old.loose.insert(hash(&bytes), bytes).is_none())?;
    }
    Ok(old)
}
impl Native {
    pub(in super::super) fn pending_turnover(&self) -> IoResult<bool> {
        let Some(namespace) = &self.checkpoint_namespace else {
            return Ok(false);
        };
        let selector = LocalSelection::parse(
            &namespace.selector()?,
            &self.checkpoint_identity().map_err(|_| invalid())?,
            support::limits().json,
        )
        .map_err(|_| invalid())?;
        Ok(!selector.value()["pending"].is_null())
    }
    #[expect(
        clippy::too_many_lines,
        reason = "One original-bound source audit and activation, without an unaudited recovery capability"
    )]
    pub(in super::super) fn activate_checkpoint(
        &mut self,
        lease: &NativeLease,
    ) -> Result<(), PackageError> {
        self.checkpoint_guard(lease).map_err(refused)?;
        let identity = self.checkpoint_identity()?;
        let reader = support::limits();
        let namespace = self.custody_namespace()?;
        let selected = LocalSelection::parse(
            &namespace.selector().map_err(refused)?,
            &identity,
            reader.json,
        )?;
        let mut records = namespace.records().map_err(refused)?;
        records.authority = self
            .local_authority
            .clone()
            .zip(self.local_provenance.clone());
        let records = Arc::new(records);
        let original = if selected.value()["pending"].is_null() {
            None
        } else {
            Some(value(&records, &selected.value()["pending"])?)
        };
        let old_selection = if let Some(original) = &original {
            LocalSelection::parse(
                &records.record(&original["old"]["selector"], reader.json.max_bytes)?,
                &identity,
                reader.json,
            )?
        } else {
            selected
        };
        checked(old_selection.value()["pending"].is_null())?;
        let selected = &old_selection.value()["selected"];
        let stream =
            PackageUuid::parse(selected["stream_id"].as_str().ok_or(PackageError::Record)?)?;
        let name = if stream == namespace.initial_stream {
            CHECKPOINT.into()
        } else {
            segment_name(stream)
        };
        let segment = read(&self.evidence, &name).map_err(refused)?;
        let length = u32::from_le_bytes(
            segment
                .get(8..12)
                .ok_or(PackageError::Record)?
                .try_into()
                .map_err(refused)?,
        ) as usize;
        let header = super::super::package::parse_canonical_json(
            segment
                .get(12..12usize.checked_add(length).ok_or(PackageError::Limit)?)
                .ok_or(PackageError::Record)?,
            reader.json,
        )?;
        let journal =
            super::super::journal::Journal::read(&segment, &header, self.journal_bounds())?;
        checked(
            header["stream_id"] == selected["stream_id"]
                && hash(&super::super::journal::Journal::create(
                    &header,
                    self.journal_bounds(),
                )?) == selected["header_sha256"],
        )?;
        let current = self.current_candidate().map_err(refused)?;
        let old = if let Some(original) = &original {
            source_package(&current, original, &records)?
        } else {
            self.current().map_err(refused)?
        };
        let observed = snapshot(&self.base, &current);
        let context = super::super::RestartContext {
            inspection: &observed,
            registration: &self.base.registration,
            directory: v1_3::DirectoryObservation {
                device: self.root_pin[0],
                inode: self.root_pin[1],
            },
            identity: &identity.package,
            frames: frames(),
            reader: &reader,
            planner: super::super::budget(),
            max_originals: self.journal_bounds().max_records,
        };
        let limits = ActivationLimits {
            journal: self.journal_bounds(),
            frames: frames(),
            reader: &reader,
            planner: super::super::budget(),
        };
        let verified = if selected["format"] == "PHPSJ002" {
            checkpoint::proof::Source {
                selector: canonical(old_selection.value()),
                base: records.record(&selected["base"], reader.json.max_bytes)?,
                header: header.clone(),
                segment: segment.clone(),
                records: records.clone(),
            }
            .verify(
                &old,
                &context,
                &mut self.compiled.borrow_mut(),
                &identity,
                limits,
            )?
        } else {
            super::super::restart::verify_original_prefix_cached(
                &old,
                &context,
                &mut self.compiled.borrow_mut(),
            )?
        };
        let checkpoint = if selected["format"] == "PHPSJ002" {
            Some(checkpoint::reopen_state(
                ReopenState {
                    selector: canonical(old_selection.value()),
                    base: records.record(&selected["base"], reader.json.max_bytes)?,
                    header: header.clone(),
                    segment: segment.clone(),
                    current: verified.shared_clone(),
                    records: records.clone(),
                },
                &identity,
                limits,
            )?)
        } else {
            checked(selected["format"] == "PHPSJ001" && header == self.registered_journal_header)?;
            None
        };
        let view = if let Some(checkpoint) = &checkpoint {
            checkpoint.verify_session(&journal, frames(), &reader, super::super::budget())?
        } else {
            let raw = if let Some(first) = journal
                .records()
                .iter()
                .find(|r| r.value()["kind"] == "CheckpointIntent")
            {
                recovery::reconstruct_shared_original(
                    &old,
                    &first.value()["body"]["original"],
                    super::super::budget(),
                )?
            } else {
                old.clone()
            };
            let base = super::super::restart::verify_original_prefix_cached(
                &raw,
                &context,
                &mut self.compiled.borrow_mut(),
            )?;
            let inputs = super::super::attempt(&base, 1000).map_err(refused)?.planner;
            journal
                .verify_cached(&old, &context, &mut self.compiled.borrow_mut())?
                .verify_session(&base, &inputs, super::super::budget())?
        };
        let mut raw = if let Some(original) = &original {
            original["attempt"].clone()
        } else {
            let mut attempt = self.attempt(&verified, 1000).map_err(refused)?;
            attempt.selectors.truncate(4);
            for selector in &mut attempt.selectors {
                selector.commit_id = uuid::Uuid::new_v4().to_string();
                selector.write_id = uuid::Uuid::new_v4().to_string();
            }
            attempt.planner.active_root_id = uuid::Uuid::new_v4().to_string();
            for association in &mut attempt.planner.associations {
                association.replacement = uuid::Uuid::new_v4().to_string();
            }
            attempt.nonce = hash(uuid::Uuid::new_v4().as_bytes());
            serde_json::to_value(attempt).map_err(refused)?
        };
        if original.is_none() {
            raw["planner"]["codec"] =
                serde_json::json!("photara.codec.ps2-checkpoint-turnover-layout-v1");
            for corridor in raw["planner"]["corridors"]
                .as_array_mut()
                .ok_or(PackageError::Record)?
            {
                corridor
                    .as_object_mut()
                    .ok_or(PackageError::Record)?
                    .remove("original_sha256");
                corridor["base_head_sha256"] = serde_json::json!(hash(&canonical(&old.head)));
            }
        }
        let archive_id = if let Some(original) = &original {
            PackageUuid::parse(
                value(&records, &original["journal"]["archive"])?["archive_id"]
                    .as_str()
                    .ok_or(PackageError::Record)?,
            )?
        } else {
            PackageUuid::parse(&uuid::Uuid::new_v4().to_string())?
        };
        let size = |key: &str| {
            usize::try_from(
                raw["planner"]["geometry"][key]
                    .as_u64()
                    .ok_or(PackageError::Record)?,
            )
            .map_err(|_| PackageError::Limit)
        };
        let geometry = (size("operation_leaf")?, size("branch")?);
        let history = if let Some(checkpoint) = &checkpoint {
            PreparedHistory::prepare_next(
                &view,
                &verified,
                &identity,
                archive_id,
                geometry,
                &reader,
                checkpoint.base(),
                records.as_ref(),
            )?
        } else {
            PreparedHistory::prepare(&view, &verified, &identity, archive_id, geometry, &reader)?
        };
        let plan = if let Some(original) = &original {
            TurnoverPlan::restore(
                &history,
                &old_selection,
                &canonical(original),
                &records.record(&original["target"]["base"], reader.json.max_bytes)?,
                super::super::budget(),
            )?
        } else {
            TurnoverPlan::compile(
                &history,
                &old_selection,
                checkpoint::TurnoverIds {
                    maintenance_id: uuid::Uuid::new_v4().to_string(),
                    checkpoint_id: uuid::Uuid::new_v4().to_string(),
                    next_stream_id: uuid::Uuid::new_v4().to_string(),
                },
                raw,
                super::super::budget(),
            )?
        };
        // Exact regeneration precedes promotion of coexisting package controls.
        self.allow_turnover(&plan)?;
        checkpoint::activate(
            self,
            lease,
            &plan,
            &history,
            &old_selection,
            &identity,
            limits,
        )?;
        self.load_selected_journal().map_err(refused)?;
        Ok(())
    }
}

// Invoked only by the opt-in, registered disposable native test process.
// Cuts terminate this process, never a machine or another application.
#[cfg(test)]
pub(super) fn cut(stage: &str) {
    if std::env::var("PHOTARA_PS2_CHECKPOINT_CUT").as_deref() == Ok(stage) {
        std::process::exit(86);
    }
}

#[cfg(test)]
#[test]
fn session_tests_native_maintenance_source_exact_controls_and_prefixes() {
    let (_temp, namespace, _, _) = super::tests::setup();
    let fixture = support::Fixture::load("integrated");
    let old = super::super::package(&fixture);
    for bytes in old.loose.values() {
        namespace.retain(bytes, |_, _| Ok(())).unwrap();
    }
    let records = namespace.records().unwrap();
    let corridors = old.allocations.iter().map(|(id,a)|serde_json::json!({
        "allocation_id":id,"arena":a.arena,"witness":a.witness,
        "original_end":a.bytes.len().to_string(),"base_head_sha256":hash(&canonical(&old.head))
    })).collect::<Vec<_>>();
    let original = serde_json::json!({"old":{"head":old.head,"commit":old.commit},"attempt":{"planner":{"corridors":corridors}}});
    let mut current = old.clone();
    for allocation in current.allocations.values_mut() {
        allocation.bytes.extend_from_slice(b"unselected suffix");
    }
    current.loose.clear();
    let restored = source_package(&current, &original, &records).unwrap();
    assert_eq!(restored.head, old.head);
    assert_eq!(restored.commit, old.commit);
    assert_eq!(restored.loose, old.loose);
    for (id, a) in &restored.allocations {
        assert_eq!(a.bytes, old.allocations[id].bytes);
    }
    let mut bad = original.clone();
    bad["attempt"]["planner"]["corridors"][0]["base_head_sha256"] =
        serde_json::json!("0".repeat(64));
    assert!(source_package(&current, &bad, &records).is_err());
    bad = original.clone();
    bad["attempt"]["planner"]["corridors"][0]["original_end"] =
        serde_json::json!(u64::MAX.to_string());
    assert!(source_package(&current, &bad, &records).is_err());
    let reference = &old.commit["root_set"]["inventory"];
    let bytes = records
        .record(reference, support::limits().json.max_bytes)
        .unwrap();
    let mut file = regular(&namespace.root, &record_name(&bytes), true).unwrap();
    file.write_all(b"!").unwrap();
    assert!(source_package(&current, &original, &records).is_err());
}
