use super::*;

const LARGE: u64 = 8_000_000_000;

// Stable semantic IDs never derive from extensions, coordinates or store keys.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct ResourceId(&'static str);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct VersionId(String);

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct CapturedVersion {
    resource: ResourceId,
    version: VersionId,
    evidence: String,
    length: u64,
    provenance: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Working {
    resource: ResourceId,
    coordinate: String,
    identity: u64,
    length: u64,
    mtime: u64,
    generation: u64,
    // Synthetic content token; no large file exists. Only strong boundaries
    // below may consult it, and they account for a full synthetic media read.
    content_token: u64,
}

impl Working {
    fn psb() -> Self {
        Self {
            resource: ResourceId("retained-master"),
            coordinate: "source-relative/edit.psb".into(),
            identity: 1,
            length: LARGE,
            mtime: 10,
            generation: 1,
            content_token: 1,
        }
    }

    fn observation(&self) -> Observation {
        Observation {
            identity: self.identity,
            length: self.length,
            mtime: self.mtime,
            generation: self.generation,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Observation {
    identity: u64,
    length: u64,
    mtime: u64,
    generation: u64,
}

#[derive(Debug, PartialEq, Eq)]
enum Change {
    Unobserved,
    NoChangeObserved,
    PossiblyChanged,
    CaptureInProgress,
}

fn observe(previous: Option<&Observation>, current: &Observation, missed_events: bool) -> Change {
    match previous {
        None => Change::Unobserved,
        Some(old) if old != current || missed_events => Change::PossiblyChanged,
        // This state intentionally makes no exact-byte equality claim.
        Some(_) => Change::NoChangeObserved,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
struct ReadCounters {
    source_reads: u64,
    source_bytes: u64,
    destination_reads: u64,
    destination_bytes: u64,
    synthetic_hash_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Condition {
    VerifiedOnline,
    Offline,
    AmbiguousLookup,
    Lost,
    Corrupt,
    Retired,
}

#[derive(Clone, Debug)]
struct Backing {
    store: &'static str,
    object: String,
    version: VersionId,
    evidence: String,
    length: u64,
    qualified: bool,
    immutable: bool,
    durable_receipt: bool,
    condition: Condition,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Pin {
    Authored,
    History,
    Recovery,
    Explicit,
    Pending(u64),
    Lease,
}

#[derive(Debug, PartialEq, Eq)]
enum Retention {
    NotRequired,
    Supported,
    Unconfirmed,
    Unsatisfied,
}

#[derive(Clone, Default)]
struct Resources {
    counters: ReadCounters,
    versions: BTreeMap<VersionId, CapturedVersion>,
    backings: BTreeMap<String, Backing>,
    pins: BTreeMap<VersionId, BTreeSet<Pin>>,
    pin_epoch: u64,
    unresolved: BTreeSet<u64>,
}

impl Resources {
    fn capture(&mut self, working: &Working, after: &Working) -> Check<CapturedVersion> {
        self.counters.source_reads += 1;
        self.counters.source_bytes += working.length;
        self.counters.synthetic_hash_bytes += working.length;
        if working.observation() != after.observation()
            || working.content_token != after.content_token
        {
            return Err(Refusal::Unstable);
        }
        let evidence = digest(&(working.content_token, working.length));
        let version = VersionId(digest(&(&working.resource, &evidence)));
        let captured = CapturedVersion {
            resource: working.resource.clone(),
            version: version.clone(),
            evidence,
            length: working.length,
            provenance: "capture-receipt:stable-input".into(),
        };
        self.versions.insert(version, captured.clone());
        // Exact capture records evidence but creates no indefinite pin.
        Ok(captured)
    }

    fn publish_verified(&mut self, captured: &CapturedVersion, store: &'static str, key: &str) {
        // Models the independently copied destination reread. A streaming source
        // digest alone cannot establish this receipt.
        self.counters.destination_reads += 1;
        self.counters.destination_bytes += captured.length;
        self.counters.synthetic_hash_bytes += captured.length;
        self.backings.insert(
            key.into(),
            Backing {
                store,
                object: key.into(),
                version: captured.version.clone(),
                evidence: captured.evidence.clone(),
                length: captured.length,
                qualified: true,
                immutable: true,
                durable_receipt: true,
                condition: Condition::VerifiedOnline,
            },
        );
    }

    fn pin(&mut self, version: &VersionId, pin: Pin) {
        self.pins.entry(version.clone()).or_default().insert(pin);
        self.pin_epoch += 1;
    }

    fn release(&mut self, version: &VersionId, pin: &Pin) {
        self.pins.entry(version.clone()).or_default().remove(pin);
        self.pin_epoch += 1;
    }

    fn retention(&self, version: &VersionId, replicas: usize, requires_online: bool) -> Retention {
        if self.pins.get(version).is_none_or(BTreeSet::is_empty) {
            return Retention::NotRequired;
        }
        let Some(captured) = self.versions.get(version) else {
            return Retention::Unsatisfied;
        };
        let mut supporting = BTreeSet::new();
        let mut online = BTreeSet::new();
        for backing in self.backings.values().filter(|b| &b.version == version) {
            if !backing.qualified
                || !backing.immutable
                || !backing.durable_receipt
                || backing.evidence != captured.evidence
                || backing.length != captured.length
            {
                continue;
            }
            match backing.condition {
                Condition::VerifiedOnline => {
                    supporting.insert(backing.store);
                    online.insert(backing.store);
                }
                Condition::Offline | Condition::AmbiguousLookup if !requires_online => {
                    supporting.insert(backing.store);
                }
                _ => (),
            }
        }
        if supporting.len() < replicas {
            Retention::Unsatisfied
        } else if online.len() >= replicas {
            Retention::Supported
        } else {
            Retention::Unconfirmed
        }
    }

    fn retirement_eligible(&self, version: &VersionId, scanned_epoch: u64) -> Check<bool> {
        if scanned_epoch != self.pin_epoch {
            return Err(Refusal::Conflict);
        }
        if !self.unresolved.is_empty() {
            return Err(Refusal::Reconcile);
        }
        Ok(self.pins.get(version).is_none_or(BTreeSet::is_empty))
    }

    fn materialization_preflight(&self, version: &VersionId) -> Check<&CapturedVersion> {
        let captured = self.versions.get(version).ok_or(Refusal::Integrity)?;
        if self.backings.values().any(|b| {
            &b.version == version
                && b.evidence == captured.evidence
                && b.length == captured.length
                && b.qualified
                && b.immutable
                && b.durable_receipt
                && b.condition == Condition::VerifiedOnline
        }) {
            Ok(captured)
        } else {
            Err(Refusal::Reconcile)
        }
    }
}

#[test]
fn eight_gb_working_changes_are_cheap_and_retained_v1_survives_v2() {
    let mut resources = Resources::default();
    let mut working = Working::psb();
    let v1 = resources.capture(&working, &working).unwrap();
    resources.publish_verified(&v1, "store-A", "immutable-v1");
    resources.pin(&v1.version, Pin::Explicit);
    assert_eq!(resources.counters.source_bytes, LARGE);
    assert_eq!(resources.counters.destination_bytes, LARGE);
    assert_eq!(resources.counters.synthetic_hash_bytes, 2 * LARGE);
    let observed = working.observation();
    resources.counters = ReadCounters::default();
    working.content_token = 2;
    working.generation += 1;
    working.mtime += 1;
    super::roots::ordinary_cycles(serde_json::json!({
        "captured": v1, "backing": "immutable-v1", "store": "store-A", "pin": "explicit"
    }));
    for _ in 0..64 {
        assert_eq!(
            observe(Some(&observed), &working.observation(), false),
            Change::PossiblyChanged
        );
        assert_eq!(
            resources.retention(&v1.version, 1, false),
            Retention::Supported
        );
        assert_eq!(
            resources.materialization_preflight(&v1.version).unwrap(),
            &v1
        );
    }
    assert_eq!(resources.counters, ReadCounters::default());
    let v2 = resources.capture(&working, &working).unwrap();
    assert_ne!(v1.version, v2.version);
    assert_eq!(v1.resource, v2.resource);
    assert_eq!(
        resources
            .materialization_preflight(&v1.version)
            .unwrap()
            .evidence,
        v1.evidence
    );
    assert_eq!(
        resources.materialization_preflight(&v2.version),
        Err(Refusal::Reconcile)
    );
    resources
        .backings
        .get_mut("immutable-v1")
        .unwrap()
        .immutable = false;
    assert_eq!(
        resources.retention(&v1.version, 1, false),
        Retention::Unsatisfied
    ); // Editable hardlink fails.
}

#[test]
fn same_metadata_missed_watchers_and_changing_capture_never_prove_equality() {
    let mut resources = Resources::default();
    let working = Working::psb();
    let prior = working.observation();
    let mut silent_edit = working.clone();
    silent_edit.content_token += 1; // Same size, mtime, file identity and generation.
    assert_eq!(observe(None, &prior, false), Change::Unobserved);
    assert_eq!(
        observe(Some(&prior), &silent_edit.observation(), false),
        Change::NoChangeObserved
    );
    assert_eq!(
        observe(Some(&prior), &silent_edit.observation(), true),
        Change::PossiblyChanged
    );
    assert_eq!(resources.counters, ReadCounters::default());
    let progress = Change::CaptureInProgress;
    assert_eq!(progress, Change::CaptureInProgress);
    assert_eq!(
        resources.capture(&working, &silent_edit),
        Err(Refusal::Unstable)
    );
    assert!(resources.versions.is_empty());
    assert_eq!(resources.counters.source_bytes, LARGE);
}

#[test]
fn finite_pins_gate_eligibility_and_descriptors_do_not_pin_forever() {
    let mut resources = Resources::default();
    let working = Working::psb();
    let captured = resources.capture(&working, &working).unwrap();
    resources.publish_verified(&captured, "store-A", "v1");
    assert_eq!(
        resources.retention(&captured.version, 1, false),
        Retention::NotRequired
    );
    assert!(
        resources
            .retirement_eligible(&captured.version, resources.pin_epoch)
            .unwrap()
    );
    let pins = [
        Pin::Authored,
        Pin::History,
        Pin::Recovery,
        Pin::Explicit,
        Pin::Pending(9),
        Pin::Lease,
    ];
    for pin in &pins {
        resources.pin(&captured.version, pin.clone());
    }
    for pin in &pins {
        assert!(
            !resources
                .retirement_eligible(&captured.version, resources.pin_epoch)
                .unwrap()
        );
        resources.release(&captured.version, pin);
    }
    assert!(
        resources
            .retirement_eligible(&captured.version, resources.pin_epoch)
            .unwrap()
    );
    let scanned = resources.pin_epoch;
    resources.pin(&captured.version, Pin::Recovery);
    assert_eq!(
        resources.retirement_eligible(&captured.version, scanned),
        Err(Refusal::Conflict)
    );
    resources.release(&captured.version, &Pin::Recovery);
    resources.unresolved.insert(99);
    assert_eq!(
        resources.retirement_eligible(&captured.version, resources.pin_epoch),
        Err(Refusal::Reconcile)
    );
    resources.unresolved.clear();
    assert!(
        resources
            .retirement_eligible(&captured.version, resources.pin_epoch)
            .unwrap()
    );
    // Simulated retirement receipt, no deletion implementation.
    resources.backings.get_mut("v1").unwrap().condition = Condition::Retired;
    assert_eq!(resources.versions[&captured.version], captured);
    assert_eq!(
        resources.materialization_preflight(&captured.version),
        Err(Refusal::Reconcile)
    );
    assert_eq!(
        resources.retention(&captured.version, 1, false),
        Retention::NotRequired
    );
    assert_eq!(working.coordinate, "source-relative/edit.psb");
}

#[test]
fn offline_ambiguous_loss_and_replica_policy_are_separate_facts() {
    let mut resources = Resources::default();
    let working = Working::psb();
    let captured = resources.capture(&working, &working).unwrap();
    resources.publish_verified(&captured, "store-A", "v1-A");
    resources.pin(&captured.version, Pin::Explicit);
    resources.counters = ReadCounters::default();
    for condition in [Condition::Offline, Condition::AmbiguousLookup] {
        resources.backings.get_mut("v1-A").unwrap().condition = condition;
        super::roots::ordinary_cycles(serde_json::json!({
            "captured": captured, "backing": "v1-A", "store": "store-A",
            "availability": "unobservable", "preserved_receipt": true
        }));
        assert_eq!(
            resources.retention(&captured.version, 1, false),
            Retention::Unconfirmed
        );
        assert!(resources.backings["v1-A"].durable_receipt);
        assert_eq!(
            resources.materialization_preflight(&captured.version),
            Err(Refusal::Reconcile)
        );
        assert_eq!(
            resources.retention(&captured.version, 1, true),
            Retention::Unsatisfied
        );
    }
    assert_eq!(resources.counters, ReadCounters::default());
    resources.publish_verified(&captured, "store-B", "v1-B");
    for condition in [Condition::Lost, Condition::Corrupt] {
        resources.backings.get_mut("v1-A").unwrap().condition = condition;
        assert_eq!(
            resources.retention(&captured.version, 1, false),
            Retention::Supported
        );
        assert_eq!(
            resources.retention(&captured.version, 2, false),
            Retention::Unsatisfied
        );
    }
    // Two names in the same modeled failure domain do not create two replicas.
    resources.publish_verified(&captured, "store-B", "v1-B-alias");
    assert_eq!(
        resources.retention(&captured.version, 2, false),
        Retention::Unsatisfied
    );
    resources.backings.get_mut("v1-B").unwrap().condition = Condition::Lost;
    resources.backings.get_mut("v1-B-alias").unwrap().condition = Condition::Lost;
    assert_eq!(
        resources.retention(&captured.version, 1, false),
        Retention::Unsatisfied
    );
    resources.release(&captured.version, &Pin::Explicit);
    assert_eq!(
        resources.retention(&captured.version, 2, false),
        Retention::NotRequired
    );
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Placement {
    AssetStore,
    PreferBeside,
    RequireBeside,
    Explicit(&'static str),
}

#[derive(Clone)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "Independent qualification facts are deliberately varied one at a time"
)]
struct Destination {
    id: &'static str,
    online: bool,
    writable: bool,
    qualified: bool,
    durable: bool,
    sibling_authorized: bool,
}

impl Destination {
    fn qualified(id: &'static str) -> Self {
        Self {
            id,
            online: true,
            writable: true,
            qualified: true,
            durable: true,
            sibling_authorized: true,
        }
    }

    fn permits(&self, sibling: bool) -> bool {
        self.online
            && self.writable
            && self.qualified
            && self.durable
            && (!sibling || self.sibling_authorized)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Selection {
    destination: &'static str,
    reason: &'static str,
}

fn select(
    // Highest first: operation, project kind, project default, library kind/default.
    precedence: &[Option<Placement>; 5],
    asset: &Destination,
    source_anchor: Option<&Destination>,
    explicit: Option<&Destination>,
    fallback_authorized: bool,
) -> Check<Selection> {
    let choice = precedence
        .iter()
        .flatten()
        .next()
        .ok_or(Refusal::Unqualified)?;
    let direct = |target: &Destination, reason| {
        if target.permits(false) {
            Ok(Selection {
                destination: target.id,
                reason,
            })
        } else {
            Err(Refusal::Unqualified)
        }
    };
    match choice {
        Placement::AssetStore => direct(asset, "asset policy"),
        Placement::Explicit(id) => match explicit {
            Some(target) if target.id == *id => direct(target, "explicit qualified location"),
            _ => Err(Refusal::Unqualified),
        },
        Placement::PreferBeside | Placement::RequireBeside => {
            if let Some(source) = source_anchor.filter(|s| s.permits(true)) {
                Ok(Selection {
                    destination: source.id,
                    reason: "authorized beside source",
                })
            } else if *choice == Placement::PreferBeside && fallback_authorized {
                direct(
                    asset,
                    "beside unavailable or unauthorized; preauthorized fallback",
                )
            } else {
                Err(Refusal::Unqualified)
            }
        }
    }
}

#[test]
fn placement_precedence_authorized_fallback_and_required_refusal() {
    let asset = Destination::qualified("assets");
    let beside = Destination::qualified("beside");
    let explicit = Destination::qualified("chosen");
    for level in 0..5 {
        let mut policy = [None, None, None, None, Some(Placement::AssetStore)];
        policy[level] = Some(Placement::Explicit("chosen"));
        assert_eq!(
            select(&policy, &asset, Some(&beside), Some(&explicit), true)
                .unwrap()
                .destination,
            "chosen"
        );
    }
    for failure in 0..6 {
        let mut unavailable = beside.clone();
        match failure {
            0 => unavailable.online = false,
            1 => unavailable.writable = false,
            2 => unavailable.qualified = false,
            3 => unavailable.durable = false,
            4 => unavailable.sibling_authorized = false,
            _ => (),
        }
        let source = if failure == 5 {
            None
        } else {
            Some(&unavailable)
        };
        let prefer = [Some(Placement::PreferBeside), None, None, None, None];
        let required = [Some(Placement::RequireBeside), None, None, None, None];
        let fallback = select(&prefer, &asset, source, None, true).unwrap();
        assert_eq!(fallback.destination, "assets");
        assert!(fallback.reason.contains("preauthorized fallback"));
        assert_eq!(
            select(&prefer, &asset, source, None, false),
            Err(Refusal::Unqualified)
        );
        assert_eq!(
            select(&required, &asset, source, None, true),
            Err(Refusal::Unqualified)
        );
    }
    let prefer = [Some(Placement::PreferBeside), None, None, None, None];
    assert_eq!(
        select(&prefer, &asset, Some(&beside), None, true)
            .unwrap()
            .destination,
        "beside"
    );
    let mut unqualified_fallback = asset;
    unqualified_fallback.durable = false;
    assert_eq!(
        select(&prefer, &unqualified_fallback, None, None, true),
        Err(Refusal::Unqualified)
    );
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Slot {
    identity: u64,
    spelling: String,
    revision: u64,
    location: &'static str,
}

#[derive(Debug, PartialEq, Eq)]
struct SlotCapture {
    identity: u64,
    revision: u64,
    location: &'static str,
}

impl Slot {
    fn capture(&self) -> SlotCapture {
        SlotCapture {
            identity: self.identity,
            revision: self.revision,
            location: self.location,
        }
    }
}

#[test]
fn slot_capture_survives_rename_retarget_and_host_remount() {
    let mut slot = Slot {
        identity: 7,
        spelling: "asset_store".into(),
        revision: 1,
        location: "location-A",
    };
    let default_asset_role = slot.identity;
    let captured = slot.capture();
    slot.spelling = "masters".into();
    slot.revision += 1;
    assert_eq!(slot.identity, captured.identity);
    slot.location = "location-B";
    slot.revision += 1;
    assert_eq!(captured.location, "location-A");
    assert_eq!(slot.capture().location, "location-B");
    assert_eq!(captured.revision, 1);
    let impersonator = Slot {
        identity: 8,
        spelling: "asset_store".into(),
        revision: 1,
        location: "location-C",
    };
    assert_ne!(impersonator.identity, default_asset_role);
    let mut bindings = BTreeMap::from([("location-A", "/Volumes/Original")]);
    bindings.insert("location-A", "/Volumes/Remounted");
    assert_eq!(bindings[captured.location], "/Volumes/Remounted");
    assert_eq!(captured.location, "location-A");
    // Ordinary same-scope name table includes variables and slots.
    let mut names = BTreeSet::from(["masters", "asset_store", "a_variable"]);
    assert!(!names.insert("a_variable"));
    assert!(!names.insert("masters"));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum PublicationStage {
    Intent,
    Staged,
    Verified,
    StorePublished,
    PackageCommitted,
    Receipt,
}

const STORE_CUTS: [PublicationStage; 6] = [
    PublicationStage::Intent,
    PublicationStage::Staged,
    PublicationStage::Verified,
    PublicationStage::StorePublished,
    PublicationStage::PackageCommitted,
    PublicationStage::Receipt,
];

#[derive(Clone)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "Separate persisted evidence and observed outcome at each fault cut"
)]
struct CrossStore {
    operation: u64,
    version: CapturedVersion,
    stage: PublicationStage,
    intent_preserved: bool,
    staging_preserved: bool,
    package_descriptor: bool,
    success_receipt: bool,
    unknown: bool,
}

impl CrossStore {
    fn new(operation: u64, version: CapturedVersion) -> Self {
        Self {
            operation,
            version,
            stage: PublicationStage::Intent,
            intent_preserved: true,
            staging_preserved: true,
            package_descriptor: false,
            success_receipt: false,
            unknown: false,
        }
    }

    fn advance(&mut self, through: PublicationStage, resources: &mut Resources) {
        for stage in STORE_CUTS {
            if stage <= self.stage || stage > through {
                continue;
            }
            match stage {
                PublicationStage::StorePublished => {
                    resources.publish_verified(&self.version, "store-B", "relocated-v1");
                }
                PublicationStage::PackageCommitted => self.package_descriptor = true,
                PublicationStage::Receipt => self.success_receipt = true,
                _ => (),
            }
            self.stage = stage;
        }
    }

    fn interrupted(&mut self, resources: &mut Resources) {
        self.unknown = true;
        resources.unresolved.insert(self.operation);
        resources.pin(&self.version.version, Pin::Pending(self.operation));
    }

    fn reconcile(&mut self, operation: u64, resources: &mut Resources) -> Check<()> {
        if operation != self.operation {
            return Err(Refusal::Conflict);
        }
        if resources.backings.get("relocated-v1").is_none_or(|b| {
            b.version != self.version.version
                || !b.durable_receipt
                || !b.qualified
                || !b.immutable
                || b.evidence != self.version.evidence
                || b.length != self.version.length
                || b.condition != Condition::VerifiedOnline
        }) {
            return Err(Refusal::Reconcile);
        }
        self.advance(PublicationStage::Receipt, resources);
        self.unknown = false;
        resources.unresolved.remove(&self.operation);
        resources.release(&self.version.version, &Pin::Pending(self.operation));
        Ok(())
    }
}

#[test]
fn cross_store_cuts_and_verified_relocation_keep_original_operation_and_version() {
    for cut in STORE_CUTS {
        let mut resources = Resources::default();
        let working = Working::psb();
        let captured = resources.capture(&working, &working).unwrap();
        resources.publish_verified(&captured, "store-A", "original-v1");
        resources.pin(&captured.version, Pin::Recovery);
        let mut publication = CrossStore::new(99, captured.clone());
        publication.advance(cut, &mut resources);
        publication.interrupted(&mut resources);
        assert_eq!(
            publication.reconcile(100, &mut resources),
            Err(Refusal::Conflict)
        );
        assert!(publication.intent_preserved && publication.staging_preserved);
        assert_eq!(
            resources.retirement_eligible(&captured.version, resources.pin_epoch),
            Err(Refusal::Reconcile)
        );
        if cut < PublicationStage::StorePublished {
            assert_eq!(
                publication.reconcile(99, &mut resources),
                Err(Refusal::Reconcile)
            );
            assert!(!publication.package_descriptor && !publication.success_receipt);
            assert!(!resources.backings.contains_key("relocated-v1"));
        } else {
            publication.reconcile(99, &mut resources).unwrap();
            assert!(publication.package_descriptor && publication.success_receipt);
            assert_eq!(resources.backings["relocated-v1"].version, captured.version);
            assert_ne!(
                resources.backings["relocated-v1"].object,
                resources.backings["original-v1"].object
            );
            assert!(
                !resources
                    .retirement_eligible(&captured.version, resources.pin_epoch)
                    .unwrap()
            );
        }
        assert_eq!(resources.versions.len(), 1);
        assert!(resources.backings.contains_key("original-v1"));
    }
}

#[test]
fn definite_publication_failure_and_uncertain_relocation_cannot_claim_retention() {
    let mut resources = Resources::default();
    let working = Working::psb();
    let captured = resources.capture(&working, &working).unwrap();
    resources.pin(&captured.version, Pin::Explicit);
    assert_eq!(
        resources.retention(&captured.version, 1, false),
        Retention::Unsatisfied
    );
    let mut publication = CrossStore::new(1, captured.clone());
    publication.advance(PublicationStage::StorePublished, &mut resources);
    resources
        .backings
        .get_mut("relocated-v1")
        .unwrap()
        .condition = Condition::AmbiguousLookup;
    publication.interrupted(&mut resources);
    assert_eq!(
        publication.reconcile(1, &mut resources),
        Err(Refusal::Reconcile)
    );
    assert!(!publication.package_descriptor && !publication.success_receipt);
    assert_eq!(
        resources.retention(&captured.version, 1, false),
        Retention::Unconfirmed
    );
    resources
        .backings
        .get_mut("relocated-v1")
        .unwrap()
        .condition = Condition::Corrupt;
    assert_eq!(
        resources.retention(&captured.version, 1, false),
        Retention::Unsatisfied
    );
}

#[derive(Default)]
struct Capacity {
    free: BTreeMap<&'static str, u64>,
    reservations: BTreeMap<u64, (&'static str, u64)>,
}

impl Capacity {
    fn reserve(&mut self, operation: u64, domain: &'static str, bytes: u64) -> Check<()> {
        if let Some(old) = self.reservations.get(&operation) {
            return if *old == (domain, bytes) {
                Ok(())
            } else {
                Err(Refusal::Conflict)
            };
        }
        let used = self
            .reservations
            .values()
            .filter(|(d, _)| *d == domain)
            .try_fold(0_u64, |sum, (_, n)| sum.checked_add(*n))
            .ok_or(Refusal::Capacity)?;
        if used.checked_add(bytes).ok_or(Refusal::Capacity)?
            > *self.free.get(domain).ok_or(Refusal::Capacity)?
        {
            return Err(Refusal::Capacity);
        }
        self.reservations.insert(operation, (domain, bytes));
        Ok(())
    }
}

#[test]
fn package_and_store_reserves_share_capacity_only_when_domains_overlap() {
    // Artificial units, never recommended production reserves or thresholds.
    let package_objects = 60;
    let journal_index = 10;
    let recovery_staging = 20;
    let newly_embedded = 10;
    let package_reserve = package_objects + journal_index + recovery_staging + newly_embedded;
    let external_described = 80 * LARGE;
    assert!(external_described > package_reserve);
    let mut capacity = Capacity {
        free: BTreeMap::from([("package-domain", 100), ("store-domain", LARGE)]),
        ..Capacity::default()
    };
    capacity
        .reserve(1, "package-domain", package_reserve)
        .unwrap();
    capacity.reserve(2, "store-domain", LARGE).unwrap();
    assert_eq!(
        capacity.reserve(3, "package-domain", 1),
        Err(Refusal::Capacity)
    );
    assert_eq!(capacity.reservations.len(), 2);
    capacity.reserve(2, "store-domain", LARGE).unwrap(); // Same operation does not double reserve.
    assert_eq!(capacity.reservations.len(), 2);
    assert_eq!(
        capacity.reserve(2, "store-domain", LARGE - 1),
        Err(Refusal::Conflict)
    );
    let mut shared = Capacity {
        free: BTreeMap::from([("same-volume", LARGE)]),
        ..Capacity::default()
    };
    shared.reserve(1, "same-volume", package_reserve).unwrap();
    assert_eq!(
        shared.reserve(2, "same-volume", LARGE),
        Err(Refusal::Capacity)
    );
    shared
        .reserve(2, "same-volume", LARGE - package_reserve)
        .unwrap();
    assert_eq!(shared.reserve(3, "same-volume", 1), Err(Refusal::Capacity));
    assert_eq!(
        shared.reserve(4, "same-volume", u64::MAX),
        Err(Refusal::Capacity)
    );
    assert_eq!(
        shared.reserve(5, "unknown-domain", 1),
        Err(Refusal::Capacity)
    );
}

#[derive(Debug, PartialEq, Eq)]
enum Backup {
    PackageState,
    CompleteProject,
    Incomplete,
}

fn backup(
    resources: &mut Resources,
    selected_versions: &[VersionId],
    complete: bool,
    copy_verified: bool,
) -> Backup {
    if !complete {
        return Backup::PackageState;
    }
    if selected_versions
        .iter()
        .any(|v| resources.materialization_preflight(v).is_err())
    {
        return Backup::Incomplete;
    }
    for version in selected_versions {
        let bytes = resources.versions[version].length;
        resources.counters.source_reads += 1;
        resources.counters.source_bytes += bytes;
        resources.counters.destination_reads += 1;
        resources.counters.destination_bytes += bytes;
        resources.counters.synthetic_hash_bytes += bytes;
    }
    if copy_verified {
        Backup::CompleteProject
    } else {
        Backup::Incomplete
    }
}

#[test]
fn package_backup_scope_does_not_claim_offline_media_or_user_sources() {
    let mut resources = Resources::default();
    let working = Working::psb();
    let captured = resources.capture(&working, &working).unwrap();
    resources.publish_verified(&captured, "store-A", "v1");
    let selected = [captured.version.clone()];
    resources.counters = ReadCounters::default();
    assert_eq!(
        backup(&mut resources, &selected, false, false),
        Backup::PackageState
    );
    assert_eq!(resources.counters, ReadCounters::default());
    assert_eq!(
        backup(&mut resources, &selected, true, false),
        Backup::Incomplete
    );
    resources.counters = ReadCounters::default();
    assert_eq!(
        backup(&mut resources, &selected, true, true),
        Backup::CompleteProject
    );
    assert_eq!(resources.counters.source_bytes, LARGE);
    assert_eq!(resources.counters.destination_bytes, LARGE);
    assert_eq!(resources.counters.synthetic_hash_bytes, LARGE);
    resources.backings.get_mut("v1").unwrap().condition = Condition::Offline;
    resources.counters = ReadCounters::default();
    assert_eq!(
        backup(&mut resources, &selected, false, true),
        Backup::PackageState
    );
    assert_eq!(
        backup(&mut resources, &selected, true, true),
        Backup::Incomplete
    );
    assert_eq!(resources.counters, ReadCounters::default());
    assert_eq!(working.resource, ResourceId("retained-master"));
    assert_eq!(working.coordinate, "source-relative/edit.psb");
}
