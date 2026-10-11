//! Finite first-turnover append and four original-bound selections. Compilation
//! has no effects; only independent complete reader inspection produces a proof.
use super::{LocalSelection, PreparedHistory};
use crate::package::v1_3::repeatable::{
    self, Attempt, Limits, SelectedStage, VerifiedOriginal, layout,
    shared::{Bytes, OriginalPackage},
    wire::{self, Result, encode, ensure, hash, key, number, reference, text},
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub struct TurnoverIds {
    pub maintenance_id: String,
    pub checkpoint_id: String,
    pub next_stream_id: String,
}
/// Regenerated immutable maintenance original, payload and finite selections.
/// This is not a custody token or a durable completion receipt.
pub struct TurnoverPlan {
    pub(crate) old: OriginalPackage,
    original: Value,
    base: Value,
    stages: Vec<SelectedStage>,
    pub(crate) files: BTreeMap<String, Bytes>,
    empty_hold: crate::package::ObjectRef,
    empty_extra: BTreeMap<crate::package::ObjectRef, crate::package::v1_3::Membership>,
}
fn object(old: &OriginalPackage, name: &str, mut value: Value) -> Value {
    value["schema"] = json!({"id":name,"version":1});
    value["project_id"] = old.manifest["project_id"].clone();
    value["extensions"] = json!({});
    value
}
fn common(old: &OriginalPackage, name: &str, value: Value) -> Value {
    let mut value = object(old, name, value);
    value["library_id"] = old.commit["root_set"]["library_id"].clone();
    value["bootstrap_sha256"] = json!(hash(&encode(&old.manifest)));
    value
}
fn round(n: u64, unit: u64) -> Result<u64> {
    ensure(unit > 0, "charge unit")?;
    n.checked_add(unit - 1)
        .and_then(|v| (v / unit).checked_mul(unit))
        .ok_or("charge overflow")
}
/// The approved new wire is preserved verbatim. This private conversion only
/// feeds the established bounded append engine during the required old audit.
fn inputs(raw: &Value, old: &OriginalPackage) -> Result<Attempt> {
    let mut normalized = raw.clone();
    ensure(
        raw["planner"]["codec"] == "photara.codec.ps2-checkpoint-turnover-layout-v1",
        "turnover codec",
    )?;
    // A turnover builds fresh finite trees at the original requested geometry.
    // Old trees were independently audited under reader limits; their leaf size
    // need not equal this new layout geometry. Future v4 uses these new seeds.
    normalized["planner"]["codec"] = json!("photara.codec.ps2-repeatable-layout-v1");
    let expected_head = hash(&encode(&old.head));
    for corridor in normalized["planner"]["corridors"]
        .as_array_mut()
        .ok_or("corridors")?
    {
        ensure(
            corridor["base_head_sha256"] == expected_head
                && corridor.get("original_sha256").is_none(),
            "base head corridor",
        )?;
        let id = text(&corridor["allocation_id"])?;
        let allocation = old.allocations.get(id).ok_or("registered corridor")?;
        let map = corridor.as_object_mut().ok_or("corridor")?;
        map.remove("base_head_sha256");
        map.insert("original_sha256".into(), json!(hash(&allocation.bytes)));
    }
    serde_json::from_value(normalized).map_err(|_| "exact turnover attempt")
}
impl TurnoverPlan {
    /// Compile the approved first-turnover recipe from the fully verified,
    /// drained old session and its exact local selector. No IDs are generated.
    /// # Errors
    /// Refuses changed binding, unsupported prior bases, duplicate/freshness
    /// failures, overflow, or any original finite append/control/charge bound.
    pub fn compile(
        history: &PreparedHistory<'_>,
        selector: &LocalSelection,
        ids: TurnoverIds,
        attempt: Value,
        limits: Limits,
    ) -> std::result::Result<Self, crate::package::PackageError> {
        Self::compile_inner(history, selector, ids, attempt, limits)
            .map_err(|_| crate::package::PackageError::Integrity)
    }
    #[expect(
        clippy::too_many_lines,
        reason = "One deterministic finite maintenance recipe"
    )]
    pub(crate) fn compile_inner(
        history: &PreparedHistory<'_>,
        selector: &LocalSelection,
        ids: TurnoverIds,
        raw: Value,
        limits: Limits,
    ) -> Result<Self> {
        let verified = history.source;
        let old = &verified.package;
        let selected = selector.value();
        let segment = &history.archive()["segments"][0];
        ensure(
            selected["pending"].is_null()
                && selected["selected"]["format"] == "PHPSJ001"
                && selected["selected"]["stream_id"] == segment["stream_id"]
                && selected["selected"]["header_sha256"] == segment["header_sha256"]
                && selected["project_id"] == old.manifest["project_id"]
                && selected["library_id"] == old.commit["root_set"]["library_id"]
                && selected["bootstrap_sha256"] == hash(&encode(&old.manifest))
                && selected["device_id"] == history.archive()["device_id"]
                && selected["incarnation_id"] == history.archive()["incarnation_id"],
            "old selector binding",
        )?;
        let attempt = inputs(&raw, old)?;
        ensure(attempt.selectors.len() == 4, "four maintenance selectors")?;
        let mut fresh = BTreeSet::new();
        for id in [&ids.maintenance_id, &ids.checkpoint_id, &ids.next_stream_id]
            .into_iter()
            .chain(
                attempt
                    .selectors
                    .iter()
                    .flat_map(|s| [&s.commit_id, &s.write_id]),
            )
        {
            wire::uuid(&json!(id))?;
            ensure(
                fresh.insert(id)
                    && old.commit["commit_id"] != *id
                    && old.commit["write_id"] != *id
                    && segment["stream_id"] != *id,
                "fresh maintenance identities",
            )?;
        }
        ensure(
            number(&json!(attempt.token))? > 0
                && attempt.nonce.len() == 64
                && attempt
                    .nonce
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "attempt identity",
        )?;
        attempt.planner.validate(old, &verified.proof, limits)?;
        let builder = layout::Builder::load(old, &attempt.planner, limits)?;
        let active = &verified.proof.roles.get("active").ok_or("active")?.state;
        let index = builder
            .pool
            .get(&key(&active["operation_index"])?)
            .ok_or("old receipt index")?;
        let base = common(
            old,
            "photara.package.acceptance-base",
            json!({
            "checkpoint_id":ids.checkpoint_id,"source_commit":{"commit_id":old.commit["commit_id"],"commit_sha256":old.head["commit_sha256"]},"source_state":reference(active),
            "authored_revision":active["authored_revision"],"authored_sha256":active["authored"]["sha256"],"accepted":active["accepted"],"journal_inclusion":active["journal_inclusion"],
            "receipts_by_id":index["by_id"],"receipts_by_ordinal":index["by_ordinal"],"receipt_count":active["accepted"]["through_ordinal"]}),
        );
        let layout = layout::compile_checkpoint(
            builder,
            old,
            &verified.proof,
            &base,
            &attempt.planner,
            limits,
        )?;
        let root = &old.commit["root_set"];
        let envelope = old.loose(&root["placement"]["accounting"])?;
        let ledger = old.loose(&envelope["ledger"])?;
        let overlay = old.loose(&root["inventory"])?;
        let unit = number(&json!(attempt.charge_unit))?;
        ensure(unit == verified.charge_unit, "registered charge unit")?;
        let cleanup = number(&json!(attempt.cleanup_bound))?;
        let mut nextledger = ledger.clone();
        let mut consumed = 0u64;
        for tip in nextledger["tips"].as_array_mut().ok_or("registered tips")? {
            let id = text(&tip["allocation_id"])?;
            let extent = layout.files.get(id).ok_or("target tip")?.len() as u64;
            let prior = number(&tip["registered_charge"])?;
            let charge = round(extent, unit)?.max(prior);
            consumed = consumed
                .checked_add(charge - prior)
                .ok_or("growth charge")?;
            tip["extent"] = json!(extent.to_string());
            tip["registered_charge"] = json!(charge.to_string());
            tip["observation"]["measured_extent"] = json!(extent.to_string());
            tip["observation"]["charged_high_water"] = json!(charge.to_string());
        }
        let reserve = consumed.checked_add(cleanup).ok_or("reserve")?;
        let before = number(&ledger["total_charge"])?;
        ensure(
            before.checked_add(reserve).ok_or("peak charge")?
                <= number(&json!(attempt.project_limit))?,
            "project capacity refusal",
        )?;
        nextledger["total_charge"] = json!(
            before
                .checked_add(consumed)
                .ok_or("total charge")?
                .to_string()
        );
        let mut writes = attempt.planner.corridors.iter().map(|c| {
            let prior = &old.allocations[&c.allocation_id];
            let target = &layout.files[&c.allocation_id];
            ensure(target.starts_with(&prior.bytes), "append only")?;
            Ok(json!({"allocation_id":c.allocation_id,"arena":c.arena,"witness":c.witness,"original_end":c.original_end,"maximum_end":c.maximum_end,"target_end":target.len().to_string(),"suffix_sha256":hash(&target[prior.bytes.len()..])}))
        }).collect::<Result<Vec<_>>>()?;
        writes.sort_by(|a, b| {
            a["allocation_id"]
                .as_str()
                .cmp(&b["allocation_id"].as_str())
        });
        let mut original = common(
            old,
            "photara.storage.checkpoint-turnover-original",
            json!({"maintenance_id":ids.maintenance_id,"scope":verified.scope,"old":{"head":old.head,"commit":old.commit,"selector":reference(selected)},"target":{"base":reference(&base),"state":layout.target["active"],"recovery":layout.target["recovery"],"pinned_roots":layout.target["pinned_roots"],"archive_placement":layout.target["archive_placement"]},"journal":{"next_stream_id":ids.next_stream_id,"through":history.through,"lookup":history.lookup(),"archive":reference(history.archive())},"attempt":null,"writes":writes}),
        );
        original["attempt"] = raw;
        original["maintenance_id"] = Value::String(ids.maintenance_id);
        let state = layout.active_state.clone();
        let basecount = layout.base_count;
        let mut stages = Vec::new();
        let mut prior_head = old.head.clone();
        let mut prior_commit = old.commit.clone();
        let mut prior_loose = old.loose.clone();
        for (i, name) in ["reserved", "written", "published", "clean"]
            .iter()
            .enumerate()
        {
            let published = i >= 2;
            let charged = if published { consumed } else { 0 };
            let remaining = if i == 3 { 0 } else { reserve - charged };
            let phase = common(
                old,
                "photara.storage.checkpoint-turnover-phase",
                json!({"maintenance_id":original["maintenance_id"],"original":reference(&original),"stage":name,"consumed":charged.to_string(),"remaining":remaining.to_string()}),
            );
            ensure(
                encode(&phase).len() as u64 <= number(&json!(attempt.phase_bytes))?,
                "phase bytes",
            )?;
            let hold = object(
                old,
                "photara.storage.hold-leaf",
                json!({"count":"1","reserved":reserve.to_string(),"consumed":charged.to_string(),"remaining":remaining.to_string(),"entries":[{"token":attempt.token,"original":reference(&original),"phase":reference(&phase)}]}),
            );
            let ledger = if published {
                nextledger.clone()
            } else {
                ledger.clone()
            };
            let holds = if i == 3 {
                serde_json::to_value(&verified.empty_hold).map_err(|_| "hold ref")?
            } else {
                reference(&hold)
            };
            let envelope = object(
                old,
                "photara.storage.accounting-envelope",
                json!({"ledger":reference(&ledger),"holds":holds}),
            );
            let mut controls = vec![ledger, envelope.clone(), original.clone(), phase];
            if i != 3 {
                controls.push(hold);
            }
            if !published {
                for r in overlay["controls"].as_array().ok_or("original controls")? {
                    if *r != old.loose(&root["placement"]["accounting"])?["ledger"]
                        && *r != root["placement"]["accounting"]
                    {
                        controls.push(old.loose(r)?);
                    }
                }
            }
            let unique = controls
                .into_iter()
                .map(|v| (key(&reference(&v)).expect("canonical ref"), v))
                .collect::<BTreeMap<_, _>>();
            let controls = unique.into_values().collect::<Vec<_>>();
            let count = if published {
                basecount
            } else {
                number(&overlay["count"])?
                    .checked_sub(overlay["controls"].as_array().ok_or("controls")?.len() as u64)
                    .ok_or("old base count")?
            };
            let overlay = object(
                old,
                "photara.package.inventory-overlay",
                json!({"base":if published{layout.base_inventory.clone()}else{overlay["base"].clone()},"controls":controls.iter().map(reference).collect::<Vec<_>>(),"count":count.checked_add(controls.len() as u64).ok_or("inventory count")?.to_string()}),
            );
            let mut nextroot = root.clone();
            nextroot["inventory"] = reference(&overlay);
            nextroot["placement"]["accounting"] = reference(&envelope);
            if published {
                nextroot["schema"]["version"] = json!(3);
                for (key, value) in layout.target.as_object().ok_or("target")? {
                    nextroot[key] = value.clone();
                }
                nextroot["placement"]["generation"] = json!(
                    number(&root["placement"]["generation"])?
                        .checked_add(1)
                        .ok_or("placement generation")?
                        .to_string()
                );
                nextroot["placement"]["root_placements"] = layout.root_placements.clone();
            }
            let mut commit = old.commit.clone();
            commit["root_set"] = nextroot;
            commit["commit_id"] = json!(attempt.selectors[i].commit_id);
            commit["write_id"] = json!(attempt.selectors[i].write_id);
            commit["package_revision"] = json!(
                number(&prior_commit["package_revision"])?
                    .checked_add(1)
                    .ok_or("revision")?
                    .to_string()
            );
            commit["parent"] = json!({"commit_id":prior_commit["commit_id"],"commit_sha256":hash(&encode(&prior_commit))});
            if published {
                commit["minimum_reader"] = json!({"major":1,"minor":4});
                let features = commit["required_features"]
                    .as_array_mut()
                    .ok_or("features")?;
                features.push(json!("photara.snapshot-recovery.v1"));
                features.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
                features.dedup();
                for name in ["authored", "history", "inventory"] {
                    commit[name] = state[name].clone();
                }
            }
            let mut head = old.head.clone();
            head["commit_id"] = commit["commit_id"].clone();
            head["commit_sha256"] = json!(hash(&encode(&commit)));
            let loose = controls
                .iter()
                .chain([&overlay])
                .map(|v| (hash(&encode(v)), encode(v)))
                .collect::<BTreeMap<_, _>>();
            let mut coexist = BTreeMap::new();
            for map in [&prior_loose, &loose] {
                for (sha, bytes) in map {
                    coexist.insert(sha.clone(), bytes.len() as u64);
                }
            }
            for value in [&old.manifest, &prior_head, &prior_commit, &head, &commit] {
                coexist.insert(hash(&encode(value)), encode(value).len() as u64);
            }
            let selector = object(
                old,
                "photara.storage.admission-selector",
                json!({"token":attempt.token,"original":reference(&original),"old_head_sha256":hash(&encode(&prior_head)),"next_head_sha256":hash(&encode(&head))}),
            );
            let scratch = round(encode(&selector).len() as u64, unit)?
                .checked_add(round(encode(&head).len() as u64, unit)?)
                .and_then(|v| v.checked_add(unit))
                .ok_or("scratch charge")?;
            let charge = coexist.values().try_fold(scratch, |sum, len| {
                sum.checked_add(round(*len, unit)?).ok_or("control charge")
            })?;
            ensure(
                charge <= number(&json!(attempt.aggregate_control_bytes))?
                    && coexist.len() as u64 + 3 <= number(&json!(attempt.aggregate_control_count))?
                    && number(&json!(attempt.aggregate_control_bytes))?
                        <= number(&nextledger["standing_control"])?,
                "finite adjacent controls",
            )?;
            prior_head = head.clone();
            prior_commit = commit.clone();
            prior_loose = loose.clone();
            stages.push(SelectedStage {
                head,
                commit,
                loose,
            });
        }
        Ok(Self {
            old: old.clone(),
            original,
            base,
            stages,
            files: layout.files,
            empty_hold: verified.empty_hold.clone(),
            empty_extra: BTreeMap::new(),
        })
    }
    /// Regenerate an existing original using its exact retained base dependency
    /// and independently audited old session. No replacement IDs are allocated.
    /// # Errors
    /// Refuses changed original/base bytes, source, geometry, bounds or target.
    pub fn restore(
        history: &PreparedHistory<'_>,
        selector: &LocalSelection,
        original_bytes: &[u8],
        base_bytes: &[u8],
        limits: Limits,
    ) -> std::result::Result<Self, crate::package::PackageError> {
        let original = crate::package::parse_canonical_json(original_bytes, limits.semantic.json)?;
        let base = crate::package::parse_canonical_json(base_bytes, limits.semantic.json)?;
        super::check(original["target"]["base"] == reference(&base))?;
        let id = |value: &Value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or(crate::package::PackageError::Record)
        };
        let ids = TurnoverIds {
            maintenance_id: id(&original["maintenance_id"])?,
            checkpoint_id: id(&base["checkpoint_id"])?,
            next_stream_id: id(&original["journal"]["next_stream_id"])?,
        };
        let plan = Self::compile(history, selector, ids, original["attempt"].clone(), limits)?;
        super::check(plan.original == original && plan.base == base)?;
        Ok(plan)
    }
    #[must_use]
    pub fn original(&self) -> &Value {
        &self.original
    }
    #[must_use]
    pub fn acceptance_base(&self) -> &Value {
        &self.base
    }
    #[must_use]
    pub fn stages(&self) -> &[SelectedStage] {
        &self.stages
    }
    pub(crate) fn selection(
        &self,
        stage: usize,
    ) -> std::result::Result<
        crate::package::v1_3::reader::OperationSelection<'_>,
        crate::package::PackageError,
    > {
        let selected = self
            .stages
            .get(stage)
            .ok_or(crate::package::PackageError::Integrity)?;
        Ok(crate::package::v1_3::reader::OperationSelection {
            original: &self.original,
            expected_commit: &selected.commit,
            expected_loose: &selected.loose,
            packed_hold: &self.empty_hold,
            extra_global: &self.empty_extra,
            packed_selected_hold: stage == 3,
        })
    }
    /// Verify exact compiled candidate plus fresh independent observations.
    /// Only the clean stage returns a subsequent planning input; a published
    /// stage still carries cleanup liability and returns `None`.
    /// # Errors
    /// Refuses changed bytes, stages, observations, registration or reader bounds.
    #[expect(
        clippy::too_many_arguments,
        reason = "Independent physical proof boundary"
    )]
    pub fn verify_candidate(
        &self,
        package: repeatable::OriginalPackage,
        stage: usize,
        identity: crate::package::v1_3::SelectionIdentity,
        inspection: &impl crate::package::v1_3::AllocationInspection,
        registration: &crate::package::v1_3::SettledRegistration,
        directory: crate::package::v1_3::DirectoryObservation,
        frames: crate::package::v1_3::FrameLimits,
        limits: &crate::package::v1_3::ReaderLimits,
    ) -> std::result::Result<Option<VerifiedOriginal>, crate::package::PackageError> {
        let verified = crate::package::v1_3::reader::verify_checkpoint_original(
            package.into(),
            identity,
            inspection,
            registration,
            directory,
            frames,
            limits,
            self,
            stage,
        )?;
        Ok((stage == 3).then_some(verified))
    }
}
