//! Optional deterministic reader memo. Actual inspection calls are replayed on
//! every hit; no lease, authority, durability or Saved result is cached.
use super::{
    AllocationInspection, AllocationObservation, BTreeMap, ObjectRef, OriginalPackage,
    PackageError, PackageUuid, RefCell, RepeatablePlan, RestartContext, Value, VerifiedOriginal,
    check, planner_limits, v1_3, value_storage, wire,
};
use crate::package::Sha256Hex;
use serde_json::json;

#[derive(Default)]
pub(super) struct ReaderMemo {
    entries: BTreeMap<String, Entry>,
    pub(super) bytes: usize,
    #[cfg(test)]
    pub(super) misses: usize,
    #[cfg(test)]
    pub(super) hits: usize,
}
struct Entry {
    cost: usize,
    proof: super::super::input::OriginalProof,
    scope: Value,
    charge: u64,
    empty: ObjectRef,
    completed_original: Option<ObjectRef>,
    retained: BTreeMap<ObjectRef, v1_3::Membership>,
    observations: BTreeMap<PackageUuid, Value>,
    prefixes: BTreeMap<(PackageUuid, u64, u64), Sha256Hex>,
}
pub(super) struct Recording<'a, I> {
    source: &'a I,
    observations: RefCell<BTreeMap<PackageUuid, Value>>,
    prefixes: RefCell<BTreeMap<(PackageUuid, u64, u64), Sha256Hex>>,
}
fn observation(v: &AllocationObservation) -> Value {
    json!([
        v.arena,
        format!("{:?}", v.layout),
        v.extent,
        v.registered_charge,
        v.device,
        v.inode
    ])
}
impl<I: AllocationInspection> AllocationInspection for Recording<'_, I> {
    fn observe(&self, id: PackageUuid) -> Result<AllocationObservation, PackageError> {
        let value = self.source.observe(id)?;
        let encoded = observation(&value);
        if let Some(previous) = self.observations.borrow_mut().insert(id, encoded.clone()) {
            check(previous == encoded)?;
        }
        Ok(value)
    }
    fn framed_prefix_digest(
        &self,
        id: PackageUuid,
        length: u64,
        max: u64,
    ) -> Result<Sha256Hex, PackageError> {
        let value = self.source.framed_prefix_digest(id, length, max)?;
        if let Some(previous) = self
            .prefixes
            .borrow_mut()
            .insert((id, length, max), value.clone())
        {
            check(previous == value)?;
        }
        Ok(value)
    }
}
impl ReaderMemo {
    pub(super) fn trim_entries(&mut self, maximum: usize) {
        while self.entries.len() > maximum {
            if let Some((_, entry)) = self.entries.pop_first() {
                self.bytes -= entry.cost;
            }
        }
    }
    pub(super) fn clear(&mut self) {
        self.entries.clear();
        self.bytes = 0;
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "Independent inspection and optional immutable snapshot index are separate from proof inputs"
    )]
    pub(super) fn verify<I: AllocationInspection, J>(
        &mut self,
        package: &OriginalPackage,
        inspection: &I,
        context: &RestartContext<'_, J>,
        operation: Option<&RepeatablePlan>,
        maximum: usize,
        prefixes: Option<&super::prefix_index::Prefixes<'_>>,
        verify: impl FnOnce(&Recording<'_, I>) -> Result<VerifiedOriginal, PackageError>,
    ) -> Result<VerifiedOriginal, PackageError> {
        let key = fingerprint(package, context, operation, prefixes);
        if let Some(entry) = self.entries.get(&key) {
            // Re-read every independent input used by the full proof. A changed
            // observation is a cache miss, not a new acceptance/refusal rule.
            // Actual provider errors still refuse immediately.
            let mut same = true;
            for (id, expected) in &entry.observations {
                same &= observation(&inspection.observe(*id)?) == *expected;
            }
            for ((id, length, max), expected) in &entry.prefixes {
                same &= inspection.framed_prefix_digest(*id, *length, *max)? == *expected;
            }
            if same {
                #[cfg(test)]
                {
                    self.hits += 1;
                }
                let mut result = VerifiedOriginal::from_reader(
                    package.clone(),
                    entry.proof.clone(),
                    entry.scope.clone(),
                    entry.charge,
                    entry.empty.clone(),
                );
                result.retained_controls = entry.retained.clone();
                result
                    .completed_original
                    .clone_from(&entry.completed_original);
                return Ok(result);
            }
        }
        if let Some(previous) = self.entries.remove(&key) {
            self.bytes -= previous.cost;
        }
        #[cfg(test)]
        {
            self.misses += 1;
        }
        let recorder = Recording {
            source: inspection,
            observations: RefCell::new(BTreeMap::new()),
            prefixes: RefCell::new(BTreeMap::new()),
        };
        let result = verify(&recorder)?;
        let entry = Entry {
            cost: 0,
            proof: result.proof.clone(),
            scope: result.scope.clone(),
            charge: result.charge_unit,
            empty: result.empty_hold.clone(),
            completed_original: result.completed_original.clone(),
            retained: result.retained_controls.clone(),
            observations: recorder.observations.into_inner(),
            prefixes: recorder.prefixes.into_inner(),
        };
        self.insert(key, entry, maximum, context.max_originals);

        Ok(result)
    }
    fn insert(&mut self, key: String, mut entry: Entry, maximum: usize, max_entries: usize) {
        if let Some(previous) = self.entries.remove(&key) {
            self.bytes -= previous.cost;
        }
        let metadata = json!({"roles":entry.proof.roles.iter().map(|(name,r)| json!([name,r.state,r.semantic,r.receipts])).collect::<Vec<_>>(),"global":entry.proof.global,"scope":entry.scope,"empty":entry.empty,"completed_original":entry.completed_original,"retained":entry.retained.keys().collect::<Vec<_>>(),"observations":entry.observations,"prefixes":entry.prefixes.iter().map(|(k,v)|json!([k.0,k.1,k.2,v])).collect::<Vec<_>>()});
        let cost = value_storage(&metadata)
            .and_then(|n| n.checked_add(entry.retained.len().checked_mul(256)?))
            .and_then(|n| n.checked_add(512));
        if let Some(cost) = cost.filter(|n| {
            self.entries.len() < max_entries
                && self
                    .bytes
                    .checked_add(*n)
                    .is_some_and(|total| total <= maximum)
        }) {
            entry.cost = cost;
            self.entries.insert(key, entry);
            self.bytes += cost;
        }
    }
    /// Reuse the exact clean executor closure at the receipt reader. This path
    /// never owns package bytes and retains only the already authenticated proof.
    #[cfg(any(test, feature = "controlled-disposable"))]
    pub(super) fn inspect_shared_candidate<I: AllocationInspection>(
        &mut self,
        package: &OriginalPackage,
        context: &RestartContext<'_, I>,
        plan: &RepeatablePlan,
        stage: usize,
        maximum: usize,
    ) -> Result<(), PackageError> {
        let recorder = Recording {
            source: context.inspection,
            observations: RefCell::new(BTreeMap::new()),
            prefixes: RefCell::new(BTreeMap::new()),
        };
        let proof = v1_3::reader::inspect_operation_package(
            package,
            super::identity(context.identity),
            &recorder,
            context.registration,
            context.frames,
            context.reader,
            plan,
            stage,
        )?;
        // These are the additional requirements of verify_operation_original.
        // Ineligible candidates keep their original inspection result but cannot
        // seed that stronger proof. No failed inspection is ever retried here.
        let scope = json!({"profile":context.registration.profile,"incarnation":context.registration.incarnation.to_string(),"directory":{"device":context.directory.device.to_string(),"inode":context.directory.inode.to_string()}});
        let ids = package
            .allocations
            .keys()
            .map(|id| PackageUuid::parse(id))
            .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
        if stage != 6
            || context.directory.inode == 0
            || scope != plan.original["scope"]
            || ids != context.registration.allocations
        {
            return Ok(());
        }
        let to_key = |r: &ObjectRef| -> Result<(String, u64), PackageError> {
            check(r.kind == crate::package::ObjectKind::Json)?;
            Ok((r.sha256.as_str().to_owned(), r.byte_length.get()))
        };
        let converted: Result<super::super::input::OriginalProof, PackageError> = (|| {
            Ok(super::super::input::OriginalProof {
                roles: proof
                    .roles
                    .into_iter()
                    .map(|(label, role)| {
                        Ok((
                            label,
                            super::super::input::Role {
                                state: role.state,
                                semantic: role.semantic.iter().map(to_key).collect::<Result<
                                    _,
                                    PackageError,
                                >>(
                                )?,
                                receipts: role.receipts,
                            },
                        ))
                    })
                    .collect::<Result<_, PackageError>>()?,
                global: proof
                    .global
                    .iter()
                    .map(to_key)
                    .collect::<Result<_, PackageError>>()?,
            })
        })();
        let Ok(proof) = converted else {
            return Ok(());
        };
        let entry = Entry {
            cost: 0,
            proof,
            scope,
            charge: context.registration.charge_unit,
            // controls() selects this exact packed hold for operation overlays.
            empty: plan.empty_hold.clone(),
            completed_original: Some(
                serde_json::from_value(wire::reference(&plan.original))
                    .map_err(|_| PackageError::Record)?,
            ),
            retained: plan.retained_controls.clone(),
            observations: recorder.observations.into_inner(),
            prefixes: recorder.prefixes.into_inner(),
        };
        self.insert(
            fingerprint(package, context, Some(plan), None),
            entry,
            maximum,
            context.max_originals,
        );
        Ok(())
    }
}
pub(super) fn fingerprint<I>(
    p: &OriginalPackage,
    c: &RestartContext<'_, I>,
    operation: Option<&RepeatablePlan>,
    prefixes: Option<&super::prefix_index::Prefixes<'_>>,
) -> String {
    let r = c.registration;
    let l = c.reader;
    // Debug representations here are process-local exact limit values, never a
    // permanent codec. Every limit member is included through its derived Debug.
    let limits = format!(
        "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{}|{}",
        c.frames,
        l.json,
        l.tree,
        l.semantic,
        l.resources,
        l.accounting,
        l.max_roles,
        l.max_prefix_bytes
    );
    let body = json!({"manifest":p.manifest,"head":p.head,"commit":p.commit,
        "loose":p.loose.iter().map(|(k,v)|json!([k,v.len(),wire::hash(v)])).collect::<Vec<_>>(),
        "allocations":p.allocations.iter().map(|(k,v)|json!([k,v.arena,v.witness,v.bytes.len(),prefixes.map_or_else(||wire::hash(&v.bytes),|p|p.hash(k,&v.bytes))])).collect::<Vec<_>>(),
        "registration":[r.profile,r.incarnation,r.allocations,r.conversion,r.standing_control,r.charge_unit,r.directory_allowance,r.retained_directory_allowance,
            r.retained_files.iter().map(|(path,v)|json!([path,v.extent,v.sha256,v.registered_charge,v.device,v.inode])).collect::<Vec<_>>()],
        "directory":[c.directory.device,c.directory.inode],"identity":[c.identity.project,c.identity.library,c.identity.bootstrap_sha256],
        "limits":limits,"planner":planner_limits(c.planner),"max_originals":c.max_originals,
        "operation":operation.map(|p|json!([p.original,p.stages[6].commit,p.stages[6].loose.iter().map(|(k,v)|json!([k,v.len(),wire::hash(v)])).collect::<Vec<_>>(),p.empty_hold,p.extra_global.iter().map(|(r,m)|json!([r,format!("{m:?}")])).collect::<Vec<_>>()]))});
    wire::hash(&wire::encode(&body))
}
