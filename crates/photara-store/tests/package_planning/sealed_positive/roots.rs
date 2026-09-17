use super::*;
use serde_json::{Value, json};

// These identifiers deliberately have no filesystem layout or wire-format role.
type ObjectId = String;

#[derive(Clone, Debug, Serialize)]
struct Object {
    required: BTreeSet<ObjectId>,
    payload: Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct Accepted {
    operation: u64,
    intent: String,
    resulting_state: String,
    provenance: String,
}

#[derive(Clone, Debug, Serialize)]
struct Root {
    project: String,
    library: String,
    bootstrap: String,
    authored_revision: u64,
    authored: ObjectId,
    history: ObjectId,
    included: Vec<Accepted>,
    // Bound by the root digest, deliberately not a required object edge.
    predecessor_commitment: Option<ObjectId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
enum Feature {
    FixtureRootSet,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
struct Dispatch {
    revision: u64,
    root_set_discriminator: bool,
    features: Vec<Feature>,
    fixture_reader_required: bool,
    active: ObjectId,
    recovery: ObjectId,
    pinned: BTreeSet<ObjectId>,
    authored_mirror: ObjectId,
    history_mirror: ObjectId,
    inventory_mirror: BTreeSet<ObjectId>,
    inclusion: String,
}

#[derive(Clone, Debug, Default)]
struct Metadata {
    objects: BTreeMap<ObjectId, Object>,
    roots: BTreeMap<ObjectId, Root>,
    // Unknown objects are never candidates merely because they are old.
    unknown_files: BTreeMap<String, Vec<u8>>,
}

impl Metadata {
    fn object(&mut self, payload: Value, required: &[ObjectId]) -> ObjectId {
        let object = Object {
            required: required.iter().cloned().collect(),
            payload,
        };
        let id = digest(&object);
        self.objects.insert(id.clone(), object);
        id
    }

    fn root(&mut self, root: Root) -> ObjectId {
        let id = digest(&root);
        self.roots.insert(id.clone(), root);
        id
    }

    fn object_closure(&self, id: &str, keep: &mut BTreeSet<ObjectId>) -> Check<()> {
        if !keep.insert(id.into()) {
            return Ok(());
        }
        let object = self.objects.get(id).ok_or(Refusal::Integrity)?;
        if digest(object) != id {
            return Err(Refusal::Integrity);
        }
        for dependency in &object.required {
            self.object_closure(dependency, keep)?;
        }
        Ok(())
    }

    fn root_closure(&self, id: &str) -> Check<BTreeSet<ObjectId>> {
        let root = self.roots.get(id).ok_or(Refusal::Integrity)?;
        if digest(root) != id
            || root.project != "project"
            || root.library != "library"
            || root.bootstrap != "unchanged-bootstrap"
        {
            return Err(Refusal::Integrity);
        }
        let mut keep = BTreeSet::from([id.into()]);
        self.object_closure(&root.authored, &mut keep)?;
        self.object_closure(&root.history, &mut keep)?;
        Ok(keep)
    }

    fn closure(&self, dispatch: &Dispatch) -> Check<BTreeSet<ObjectId>> {
        if !dispatch.root_set_discriminator
            || dispatch.features != [Feature::FixtureRootSet]
            || !dispatch.fixture_reader_required
        {
            return Err(Refusal::Unsupported);
        }
        let mut keep = self.root_closure(&dispatch.active)?;
        let active_closure = keep.clone();
        keep.extend(self.root_closure(&dispatch.recovery)?);
        for pinned in &dispatch.pinned {
            keep.extend(self.root_closure(pinned)?);
        }
        let active = &self.roots[&dispatch.active];
        if dispatch.authored_mirror != active.authored
            || dispatch.history_mirror != active.history
            || dispatch.inventory_mirror != active_closure
            || dispatch.inclusion != digest(&active.included)
        {
            return Err(Refusal::Integrity);
        }
        Ok(keep)
    }

    fn validate(&self, dispatch: &Dispatch, inventory: &BTreeSet<ObjectId>) -> Check<()> {
        if self.closure(dispatch)? != *inventory {
            return Err(Refusal::Integrity);
        }
        Ok(())
    }
}

fn state(metadata: &mut Metadata, n: u64, prior: Option<ObjectId>) -> ObjectId {
    let embedded = metadata.object(json!({"embedded":[1,2,3]}), &[]);
    let evidence = metadata.object(
        json!({"resource_identity":"logical-r1", "version":"v1", "length":8_000_000_000_u64,
            "backing_record":"opaque-store-object", "verified":"receipt", "retention":"explicit"}),
        &[],
    );
    let snapshot = metadata.object(json!({"captured_graph_context_input":true}), &[]);
    let authored = metadata.object(
        json!({"edit":n,"optional":{"$ref":"does-not-exist","path":"/not/a/dependency"}}),
        &[embedded, evidence, snapshot.clone()],
    );
    let history = metadata.object(json!({"history_promises_reconstruction":true}), &[snapshot]);
    let resulting_state = authored.clone();
    metadata.root(Root {
        project: "project".into(),
        library: "library".into(),
        bootstrap: "unchanged-bootstrap".into(),
        authored_revision: n,
        authored,
        history,
        included: vec![Accepted {
            operation: n,
            intent: format!("intent-{n}"),
            resulting_state,
            provenance: "credential-free-original".into(),
        }],
        predecessor_commitment: prior,
    })
}

fn dispatch(metadata: &Metadata, active: ObjectId, recovery: ObjectId, revision: u64) -> Dispatch {
    let root = &metadata.roots[&active];
    Dispatch {
        revision,
        root_set_discriminator: true,
        features: vec![Feature::FixtureRootSet],
        fixture_reader_required: true,
        recovery,
        pinned: BTreeSet::new(),
        authored_mirror: root.authored.clone(),
        history_mirror: root.history.clone(),
        inventory_mirror: metadata.root_closure(&active).unwrap(),
        inclusion: digest(&root.included),
        active,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Stage {
    Intent,
    Objects,
    Validated,
    Ownership,
    Head,
    Barrier,
    Reopened,
    Receipt,
    Retirement,
}

const CUTS: [Stage; 9] = [
    Stage::Intent,
    Stage::Objects,
    Stage::Validated,
    Stage::Ownership,
    Stage::Head,
    Stage::Barrier,
    Stage::Reopened,
    Stage::Receipt,
    Stage::Retirement,
];

#[derive(Clone)]
struct Attempt {
    operation: u64,
    old_head: String,
    epoch: u64,
    next: Dispatch,
    inventory: BTreeSet<ObjectId>,
    stage: Stage,
}

#[derive(Clone)]
struct Publisher {
    metadata: Metadata,
    selected: Dispatch,
    inventory: BTreeSet<ObjectId>,
    pin_epoch: u64,
    lease_owned: bool,
    barrier_available: bool,
    attempt: Option<Attempt>,
    ledger: BTreeMap<u64, Accepted>,
    journal: Vec<Accepted>,
    pending_roots: BTreeSet<ObjectId>,
    // Conversion source is outside rolling recovery retention.
    conversion_source: BTreeMap<String, Vec<u8>>,
    bootstrap_bytes: Vec<u8>,
}

impl Publisher {
    fn seed() -> Self {
        let mut metadata = Metadata::default();
        let first = state(&mut metadata, 1, None);
        let selected = dispatch(&metadata, first.clone(), first, 1);
        let inventory = metadata.closure(&selected).unwrap();
        Self {
            metadata,
            selected,
            inventory,
            pin_epoch: 0,
            lease_owned: true,
            barrier_available: true,
            attempt: None,
            ledger: BTreeMap::new(),
            journal: vec![],
            pending_roots: BTreeSet::new(),
            conversion_source: BTreeMap::new(),
            bootstrap_bytes: b"byte-identical-bootstrap".to_vec(),
        }
    }

    fn begin(&mut self, next: Dispatch, operation: u64) -> Check<()> {
        if self.attempt.is_some() {
            return Err(Refusal::Reconcile);
        }
        if next.revision
            != self
                .selected
                .revision
                .checked_add(1)
                .ok_or(Refusal::Integrity)?
        {
            return Err(Refusal::Integrity);
        }
        let inventory = self.metadata.closure(&next)?;
        if next.active != self.selected.active {
            let root = &self.metadata.roots[&next.active];
            if !self.journal.starts_with(&root.included)
                || root
                    .included
                    .last()
                    .is_none_or(|a| a.resulting_state != root.authored)
            {
                return Err(Refusal::Integrity);
            }
        }
        self.attempt = Some(Attempt {
            operation,
            old_head: digest(&self.selected),
            epoch: self.pin_epoch,
            next,
            inventory,
            stage: Stage::Intent,
        });
        Ok(())
    }

    fn advance(&mut self, through: Stage) -> Check<()> {
        let attempt = self.attempt.as_ref().ok_or(Refusal::Reconcile)?.clone();
        for stage in CUTS {
            if stage <= attempt.stage || stage > through {
                continue;
            }
            match stage {
                Stage::Validated | Stage::Reopened => {
                    self.metadata.validate(&attempt.next, &attempt.inventory)?;
                }
                Stage::Ownership | Stage::Head => {
                    if !self.lease_owned
                        || self.pin_epoch != attempt.epoch
                        || digest(&self.selected) != attempt.old_head
                    {
                        return Err(Refusal::Conflict);
                    }
                    if stage == Stage::Head {
                        self.selected = attempt.next.clone();
                        self.inventory.clone_from(&attempt.inventory);
                    }
                }
                Stage::Receipt => self.acknowledge(&attempt)?,
                Stage::Retirement => self.retire(attempt.epoch)?,
                Stage::Barrier if !self.barrier_available => return Err(Refusal::Reconcile),
                Stage::Intent | Stage::Objects | Stage::Barrier => (),
            }
            self.attempt.as_mut().unwrap().stage = stage;
        }
        Ok(())
    }

    fn acknowledge(&mut self, attempt: &Attempt) -> Check<()> {
        if self
            .attempt
            .as_ref()
            .is_none_or(|a| a.stage < Stage::Reopened)
        {
            return Err(Refusal::Reconcile);
        }
        let root = &self.metadata.roots[&attempt.next.active];
        for accepted in &root.included {
            if let Some(old) = self.ledger.get(&accepted.operation)
                && old != accepted
            {
                return Err(Refusal::Conflict);
            }
        }
        for accepted in &root.included {
            self.ledger.insert(accepted.operation, accepted.clone());
        }
        self.journal.retain(|entry| !root.included.contains(entry));
        Ok(())
    }

    fn reconcile(&mut self) -> Check<()> {
        let attempt = self.attempt.as_ref().ok_or(Refusal::Reconcile)?.clone();
        if digest(&self.selected) == attempt.old_head {
            return Err(Refusal::Reconcile); // Evidence retained, no inferred failure.
        }
        if digest(&self.selected) != digest(&attempt.next) {
            return Err(Refusal::Conflict);
        }
        // Visible HEAD is insufficient. Model completing the missing root-dir
        // barrier and structural reopen before repairing any acknowledgement.
        if !self.barrier_available {
            return Err(Refusal::Reconcile);
        }
        self.attempt.as_mut().unwrap().stage = Stage::Barrier;
        self.metadata.validate(&self.selected, &attempt.inventory)?;
        self.attempt.as_mut().unwrap().stage = Stage::Reopened;
        self.acknowledge(&attempt)?;
        self.attempt.as_mut().unwrap().stage = Stage::Receipt;
        Ok(())
    }

    fn retire(&mut self, scanned_epoch: u64) -> Check<()> {
        if scanned_epoch != self.pin_epoch {
            return Err(Refusal::Conflict);
        }
        if self
            .attempt
            .as_ref()
            .is_some_and(|a| a.stage < Stage::Receipt)
        {
            return Err(Refusal::Reconcile);
        }
        self.metadata.validate(&self.selected, &self.inventory)?;
        let mut protected = self.inventory.clone();
        for pending in &self.pending_roots {
            protected.extend(self.metadata.root_closure(pending)?);
        }
        self.metadata.objects.retain(|id, _| protected.contains(id));
        self.metadata.roots.retain(|id, _| protected.contains(id));
        Ok(())
    }

    fn retry(&self, accepted: &Accepted) -> Check<Option<&Accepted>> {
        match self.ledger.get(&accepted.operation) {
            Some(original) if original != accepted => Err(Refusal::Conflict),
            original => Ok(original),
        }
    }

    fn accept_for_fixture(&mut self, next: &Dispatch) {
        self.journal
            .extend(self.metadata.roots[&next.active].included.clone());
    }
}

// The metadata API has no media-reader capability. Feed actual resource-model
// records through 64 opens/autosaves/turnovers while the caller audits its media
// counters. Only small metadata payloads and embedded fixture bytes are hashed.
pub(super) fn ordinary_cycles(external_descriptor: Value) {
    let mut publisher = Publisher::seed();
    let descriptor = publisher.metadata.object(external_descriptor, &[]);
    for n in 2..=65 {
        let seed = state(
            &mut publisher.metadata,
            n,
            Some(publisher.selected.active.clone()),
        );
        let mut root = publisher.metadata.roots[&seed].clone();
        let mut authored = publisher.metadata.objects[&root.authored].clone();
        authored.required.insert(descriptor.clone());
        root.authored = publisher.metadata.object(
            authored.payload,
            &authored.required.into_iter().collect::<Vec<_>>(),
        );
        root.included[0].resulting_state.clone_from(&root.authored);
        let active = publisher.metadata.root(root);
        let next = dispatch(
            &publisher.metadata,
            active,
            publisher.selected.active.clone(),
            n,
        );
        publisher
            .metadata
            .validate(&publisher.selected, &publisher.inventory)
            .unwrap();
        publisher.accept_for_fixture(&next);
        publisher.begin(next, n).unwrap();
        publisher.advance(Stage::Retirement).unwrap();
        publisher
            .metadata
            .validate(&publisher.selected, &publisher.inventory)
            .unwrap();
        publisher.attempt = None;
    }
}

#[test]
fn independent_roots_exact_closure_and_opaque_bytes() {
    let mut publisher = Publisher::seed();
    let recovery = publisher.selected.active.clone();
    let active = state(&mut publisher.metadata, 2, Some(recovery.clone()));
    let selected = dispatch(&publisher.metadata, active.clone(), recovery.clone(), 2);
    let inventory = publisher.metadata.closure(&selected).unwrap();
    assert_eq!(inventory.len(), 8); // Four shared objects, authored + root per state.
    publisher.metadata.validate(&selected, &inventory).unwrap();
    let mut too_small = inventory.clone();
    too_small.remove(&recovery);
    assert_eq!(
        publisher.metadata.validate(&selected, &too_small),
        Err(Refusal::Integrity)
    );
    let mut too_large = inventory.clone();
    too_large.insert("unregistered-extra".into());
    assert_eq!(
        publisher.metadata.validate(&selected, &too_large),
        Err(Refusal::Integrity)
    );

    for kept_root in [active, recovery] {
        let closure = publisher.metadata.root_closure(&kept_root).unwrap();
        let mut isolated = publisher.metadata.clone();
        isolated.roots.retain(|id, _| id == &kept_root);
        isolated.objects.retain(|id, _| closure.contains(id));
        assert_eq!(isolated.root_closure(&kept_root).unwrap(), closure);
        let root = &isolated.roots[&kept_root];
        let before = serde_json::to_vec(&isolated.objects[&root.authored].payload).unwrap();
        assert!(
            String::from_utf8(before.clone())
                .unwrap()
                .contains("does-not-exist")
        );
        isolated.root_closure(&kept_root).unwrap();
        assert_eq!(
            serde_json::to_vec(&isolated.objects[&root.authored].payload).unwrap(),
            before
        );
    }
}

#[test]
fn dispatch_gates_mirrors_identity_and_embedded_damage_refuse() {
    let publisher = Publisher::seed();
    let original = publisher.selected;
    for mutation in 0..7 {
        let mut selected = original.clone();
        match mutation {
            0 => selected.root_set_discriminator = false,
            1 => selected.features.clear(),
            2 => selected.features.push(Feature::Unknown),
            3 => selected.fixture_reader_required = false,
            4 => selected.authored_mirror = "other".into(),
            5 => selected.history_mirror = "other".into(),
            _ => selected.inventory_mirror.clear(),
        }
        assert_eq!(
            publisher.metadata.closure(&selected),
            Err(if mutation < 4 {
                Refusal::Unsupported
            } else {
                Refusal::Integrity
            })
        );
    }
    let mut damaged = publisher.metadata.clone();
    let embedded = damaged
        .objects
        .iter()
        .find(|(_, o)| o.payload.get("embedded").is_some())
        .unwrap()
        .0
        .clone();
    damaged.objects.remove(&embedded);
    assert_eq!(damaged.closure(&original), Err(Refusal::Integrity));
    let mut wrong_identity = publisher.metadata.clone();
    let mut root = wrong_identity.roots[&original.active].clone();
    root.project = "different-project".into();
    let wrong = wrong_identity.root(root);
    assert_eq!(wrong_identity.root_closure(&wrong), Err(Refusal::Integrity));
    let mut corrupted = publisher.metadata;
    corrupted.objects.get_mut(&embedded).unwrap().payload = json!("corrupt");
    assert_eq!(corrupted.closure(&original), Err(Refusal::Integrity));
}

#[test]
fn sixty_four_turnovers_bound_ancestry_preserve_dedupe_and_finite_prefix() {
    let mut publisher = Publisher::seed();
    publisher
        .metadata
        .unknown_files
        .insert("user-note".into(), b"keep".to_vec());
    let bootstrap = publisher.bootstrap_bytes.clone();
    let mut max_keep = 0;
    for n in 2..=65 {
        let initial = state(
            &mut publisher.metadata,
            n,
            Some(publisher.selected.active.clone()),
        );
        let mut root = publisher.metadata.roots[&initial].clone();
        publisher.journal.extend(root.included.clone());
        root.included = publisher.journal.clone();
        let active = publisher.metadata.root(root);
        let accepted = publisher.metadata.roots[&active].included[0].clone();
        let later = Accepted {
            operation: 10_000 + n,
            intent: "later".into(),
            resulting_state: "later-state".into(),
            provenance: "original".into(),
        };
        publisher.journal.push(later.clone());
        let next = dispatch(
            &publisher.metadata,
            active,
            publisher.selected.active.clone(),
            n,
        );
        publisher.begin(next, n).unwrap();
        publisher.advance(Stage::Retirement).unwrap();
        assert!(publisher.journal.contains(&later));
        assert!(!publisher.journal.contains(&accepted));
        assert_eq!(publisher.metadata.roots.len(), 2);
        assert_eq!(publisher.metadata.objects.len(), 6);
        max_keep = max_keep.max(publisher.inventory.len());
        assert_eq!(publisher.retry(&accepted).unwrap(), Some(&accepted));
        publisher.attempt = None;
    }
    assert_eq!(max_keep, 8);
    assert_eq!(publisher.ledger.len(), 127);
    assert_eq!(publisher.journal.len(), 1);
    assert_eq!(publisher.bootstrap_bytes, bootstrap);
    assert_eq!(publisher.metadata.unknown_files["user-note"], b"keep");
    let original = publisher.ledger[&2].clone();
    assert_eq!(publisher.retry(&original).unwrap(), Some(&original));
    let mut collision = original;
    collision.intent = "changed".into();
    assert_eq!(publisher.retry(&collision), Err(Refusal::Conflict));
}

#[test]
fn all_publication_cuts_reconcile_original_ids_and_preserve_conversion_source() {
    // Real frozen 1.1 bytes, held as an independent conversion-source pin.
    let legacy = super::super::super::build(super::super::super::add_history);
    for cut in CUTS {
        let mut publisher = Publisher::seed();
        publisher.conversion_source = legacy.clone();
        let old = digest(&publisher.selected);
        let active = state(
            &mut publisher.metadata,
            2,
            Some(publisher.selected.active.clone()),
        );
        let next = dispatch(
            &publisher.metadata,
            active,
            publisher.selected.active.clone(),
            2,
        );
        publisher.accept_for_fixture(&next);
        publisher.begin(next.clone(), 700).unwrap();
        publisher.advance(cut).unwrap();
        assert_eq!(publisher.conversion_source, legacy);
        publisher
            .metadata
            .validate(&publisher.selected, &publisher.inventory)
            .unwrap();
        assert_eq!(publisher.attempt.as_ref().unwrap().operation, 700);
        assert_eq!(publisher.begin(next, 701), Err(Refusal::Reconcile));
        if cut < Stage::Head {
            assert_eq!(digest(&publisher.selected), old);
            assert_eq!(publisher.reconcile(), Err(Refusal::Reconcile));
            assert!(publisher.ledger.is_empty());
            assert_eq!(
                publisher.retire(publisher.pin_epoch),
                Err(Refusal::Reconcile)
            );
        } else {
            if cut < Stage::Receipt {
                publisher.barrier_available = false;
                assert_eq!(publisher.reconcile(), Err(Refusal::Reconcile));
                assert!(publisher.ledger.is_empty());
                publisher.barrier_available = true;
            }
            publisher.reconcile().unwrap();
            assert_eq!(publisher.ledger.len(), 1);
            publisher.reconcile().unwrap(); // Missing receipt repair is idempotent.
            assert_eq!(publisher.ledger.len(), 1);
            publisher.retire(publisher.pin_epoch).unwrap();
        }
        assert_eq!(publisher.conversion_source, legacy);
    }
}

#[test]
fn concurrent_head_pins_and_ownership_block_switch_or_stale_retirement() {
    for interference in 0..3 {
        let mut publisher = Publisher::seed();
        let active = state(&mut publisher.metadata, 2, None);
        let next = dispatch(
            &publisher.metadata,
            active,
            publisher.selected.active.clone(),
            2,
        );
        publisher.accept_for_fixture(&next);
        publisher.begin(next, 9).unwrap();
        publisher.advance(Stage::Validated).unwrap();
        match interference {
            0 => publisher.selected.revision += 1,
            1 => publisher.pin_epoch += 1,
            _ => publisher.lease_owned = false,
        }
        assert_eq!(publisher.advance(Stage::Head), Err(Refusal::Conflict));
        assert!(publisher.ledger.is_empty());
    }
    let mut publisher = Publisher::seed();
    let first = publisher.selected.active.clone();
    let second = state(&mut publisher.metadata, 2, Some(first.clone()));
    let next = dispatch(&publisher.metadata, second, first.clone(), 2);
    publisher.accept_for_fixture(&next);
    publisher.begin(next, 2).unwrap();
    publisher.advance(Stage::Receipt).unwrap();
    let scanned = publisher.pin_epoch;
    publisher.pin_epoch += 1;
    assert_eq!(publisher.retire(scanned), Err(Refusal::Conflict));
    assert!(publisher.metadata.roots.contains_key(&first));
    publisher.selected.revision += 1; // Unrelated HEAD is never overwritten.
    assert_eq!(publisher.reconcile(), Err(Refusal::Conflict));
}

#[test]
fn explicit_history_pin_and_compaction_revision_are_independent() {
    let mut publisher = Publisher::seed();
    let first = publisher.selected.active.clone();
    for n in 2..=4 {
        let active = state(
            &mut publisher.metadata,
            n,
            Some(publisher.selected.active.clone()),
        );
        let mut next = dispatch(
            &publisher.metadata,
            active,
            publisher.selected.active.clone(),
            n,
        );
        next.pinned.insert(first.clone());
        publisher.accept_for_fixture(&next);
        publisher.begin(next, n).unwrap();
        publisher.advance(Stage::Retirement).unwrap();
        publisher.attempt = None;
    }
    assert_eq!(publisher.metadata.roots.len(), 3);
    publisher.metadata.root_closure(&first).unwrap();
    let authored_revision = publisher.metadata.roots[&publisher.selected.active].authored_revision;
    let mut compacted = publisher.selected.clone();
    compacted.revision += 1;
    compacted.pinned.clear();
    publisher.begin(compacted, 5).unwrap();
    publisher.advance(Stage::Retirement).unwrap();
    assert!(!publisher.metadata.roots.contains_key(&first));
    assert_eq!(
        publisher.metadata.roots[&publisher.selected.active].authored_revision,
        authored_revision
    );
    assert_eq!(publisher.selected.revision, 5);
}

#[test]
fn inclusion_binds_intent_result_and_provenance_not_only_sequence() {
    let publisher = Publisher::seed();
    for field in 0..3 {
        let mut metadata = publisher.metadata.clone();
        let mut root = metadata.roots[&publisher.selected.active].clone();
        match field {
            0 => root.included[0].intent = "substitution".into(),
            1 => root.included[0].resulting_state = "substitution".into(),
            _ => root.included[0].provenance = "substitution".into(),
        }
        let changed = metadata.root(root);
        let mut selected = dispatch(&metadata, changed, publisher.selected.recovery.clone(), 2);
        selected.inclusion = publisher.selected.inclusion.clone();
        assert_eq!(metadata.closure(&selected), Err(Refusal::Integrity));
    }
    let mut publisher = publisher;
    let mut bad = publisher.selected.clone();
    bad.revision = 1; // Rollback/non-monotonic package coordinates refuse.
    assert_eq!(publisher.begin(bad, 9), Err(Refusal::Integrity));
}

#[test]
fn checkpoint_requires_exact_accepted_prefix_and_checked_revision() {
    let mut publisher = Publisher::seed();
    let active = state(&mut publisher.metadata, 2, None);
    let next = dispatch(
        &publisher.metadata,
        active.clone(),
        publisher.selected.active.clone(),
        2,
    );
    let accepted = publisher.metadata.roots[&active].included[0].clone();
    assert_eq!(publisher.begin(next.clone(), 2), Err(Refusal::Integrity));
    let mut earlier = accepted.clone();
    earlier.operation = 999;
    publisher.journal = vec![earlier, accepted.clone()];
    assert_eq!(publisher.begin(next.clone(), 2), Err(Refusal::Integrity));
    publisher.journal = vec![accepted];
    publisher.begin(next, 2).unwrap();
    publisher.advance(Stage::Receipt).unwrap();
    assert!(publisher.journal.is_empty());
    publisher.attempt = None;
    publisher.selected.revision = u64::MAX;
    let mut next = publisher.selected.clone();
    next.revision = 0;
    assert_eq!(publisher.begin(next, 3), Err(Refusal::Integrity));
}

#[test]
fn pending_root_closure_survives_retirement_until_obligation_release() {
    let mut publisher = Publisher::seed();
    let pending = state(&mut publisher.metadata, 99, None);
    let pending_closure = publisher.metadata.root_closure(&pending).unwrap();
    publisher.pending_roots.insert(pending.clone());
    publisher.pin_epoch += 1;
    publisher.retire(publisher.pin_epoch).unwrap();
    assert_eq!(
        publisher.metadata.root_closure(&pending).unwrap(),
        pending_closure
    );
    publisher.pending_roots.clear();
    publisher.pin_epoch += 1;
    publisher.retire(publisher.pin_epoch).unwrap();
    assert!(!publisher.metadata.roots.contains_key(&pending));
    publisher
        .metadata
        .validate(&publisher.selected, &publisher.inventory)
        .unwrap();
}
