//! Independent adversarial PS2 composition oracle, not a production codec.
//! Run `cargo run -p photara-store --example ps2_combined_faults`.
//! Models logical crash/interleaving cuts; no filesystem or power-loss claims.

use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

type Result<T> = std::result::Result<T, &'static str>;

fn require(value: bool, message: &'static str) -> Result<()> {
    if value { Ok(()) } else { Err(message) }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct Pin {
    object: u64,
    epoch: u64,
    kind: String,
    unknown: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct Receipt {
    operation: u64,
    request: String,
    ordinal: u64,
    before: u64,
    after: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct Group {
    id: u64,
    previous: String,
    receipts: Vec<Receipt>,
    through: u64,
    prefix: String,
}

fn digest(value: &impl Serialize) -> String {
    format!("{:x}", Sha256::digest(serde_json::to_vec(value).unwrap()))
}

fn link(previous: &str, receipt: &Receipt) -> String {
    digest(&("fixture-accepted-prefix", previous, receipt))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Hold {
    domain: u64,
    bytes: u64,
    accepted: bool,
    unknown: bool,
    owner_epoch: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Model {
    owner_epoch: u64,
    edges: BTreeMap<u64, Vec<u64>>,
    counts: BTreeMap<u64, u64>,
    pins: BTreeMap<u64, Pin>,
    completed_releases: BTreeSet<(u64, u64)>,
    receipts: BTreeMap<u64, Receipt>,
    groups: BTreeMap<u64, Group>,
    accepted_ordinal: u64,
    accepted_prefix: String,
    authored_revision: u64,
    saved_revision: Option<u64>,
    holds: BTreeMap<u64, Hold>,
    // Shared-domain identity, never independently keyed by package/store role.
    domains: BTreeMap<u64, (u64, u64)>, // capacity, already allocated old keep-set
    active_pack: u64,
    recovery_pack: u64,
    reader_packs: BTreeMap<(u64, u64), u64>,
    pending_selection: Option<(u64, u64, u64)>, // original op, old, candidate
    retired_packs: BTreeSet<u64>,
}

impl Model {
    fn new() -> Self {
        Self {
            owner_epoch: 1,
            edges: [(1, vec![]), (2, vec![1]), (3, vec![1])].into(),
            counts: [(1, 2), (2, 0), (3, 0)].into(),
            pins: BTreeMap::new(),
            completed_releases: BTreeSet::new(),
            receipts: BTreeMap::new(),
            groups: BTreeMap::new(),
            accepted_ordinal: 0,
            accepted_prefix: digest(&"fixture-prefix-zero"),
            authored_revision: 0,
            saved_revision: None,
            holds: BTreeMap::new(),
            domains: [(7, (1000, 400))].into(),
            active_pack: 10,
            recovery_pack: 10,
            reader_packs: BTreeMap::new(),
            pending_selection: None,
            retired_packs: BTreeSet::new(),
        }
    }

    fn audit(&self) -> Result<()> {
        let mut exact: BTreeMap<u64, u64> = self.edges.keys().map(|id| (*id, 0)).collect();
        for children in self.edges.values() {
            for child in children {
                *exact.get_mut(child).ok_or("missing child")? += 1;
            }
        }
        for pin in self.pins.values() {
            *exact.get_mut(&pin.object).ok_or("missing pin object")? += 1;
        }
        require(exact == self.counts, "pin/edge undercount or overcount")?;
        let mut prefix = digest(&"fixture-prefix-zero");
        let mut revision = 0;
        let mut ordered: Vec<_> = self.receipts.values().collect();
        ordered.sort_by_key(|receipt| receipt.ordinal);
        for (index, receipt) in ordered.iter().enumerate() {
            require(
                receipt.ordinal == u64::try_from(index).unwrap() + 1,
                "receipt ordinal gap",
            )?;
            require(
                receipt.before == revision
                    && receipt.after >= revision
                    && receipt.after <= revision + 1,
                "authored receipt chain mismatch",
            )?;
            revision = receipt.after;
            prefix = link(&prefix, receipt);
        }
        require(
            prefix == self.accepted_prefix && ordered.len() as u64 == self.accepted_ordinal,
            "receipt prefix mismatch",
        )?;
        require(
            revision == self.authored_revision,
            "selected authored coordinate differs from accepted receipt",
        )?;
        for (domain, (capacity, allocated)) in &self.domains {
            let held = self
                .holds
                .values()
                .filter(|hold| hold.domain == *domain)
                .try_fold(0u64, |total, hold| {
                    total.checked_add(hold.bytes).ok_or("overflow")
                })?;
            require(
                allocated.checked_add(held).ok_or("overflow")? <= *capacity,
                "shared capacity oversubscribed",
            )?;
        }
        require(
            !self.retired_packs.contains(&self.active_pack)
                && !self.retired_packs.contains(&self.recovery_pack),
            "selected pack retired",
        )?;
        require(
            self.reader_packs
                .values()
                .all(|pack| !self.retired_packs.contains(pack)),
            "reader pack retired",
        )?;
        Ok(())
    }

    fn pin(&mut self, token: u64, pin: Pin) -> Result<()> {
        require(!self.pins.contains_key(&token), "pin token collision")?;
        let count = self.counts.get_mut(&pin.object).ok_or("missing object")?;
        *count = count.checked_add(1).ok_or("overflow")?;
        self.pins.insert(token, pin);
        self.audit()
    }

    fn release(&mut self, token: u64, epoch: u64, owner_epoch: u64, resolved: bool) -> Result<()> {
        require(self.owner_epoch == owner_epoch, "stale writer incarnation")?;
        if self.completed_releases.contains(&(token, epoch)) {
            return Ok(());
        }
        let pin = self.pins.get(&token).ok_or("missing pin")?;
        require(pin.epoch == epoch, "stale pin epoch")?;
        require(!pin.unknown || resolved, "unresolved pin")?;
        let object = pin.object;
        self.pins.remove(&token);
        *self.counts.get_mut(&object).unwrap() -= 1;
        self.completed_releases.insert((token, epoch));
        self.audit()
    }

    fn reserve(&mut self, operation: u64, requests: &[(u64, u64)]) -> Result<()> {
        require(
            !self.holds.contains_key(&operation),
            "reservation collision",
        )?;
        require(!requests.is_empty(), "empty reserve")?;
        let domain = requests[0].0;
        require(
            requests.iter().all(|(id, _)| *id == domain),
            "fixture takes one physical domain",
        )?;
        let bytes = requests.iter().try_fold(0u64, |total, (_, bytes)| {
            total.checked_add(*bytes).ok_or("overflow")
        })?;
        let (capacity, allocated) = self.domains.get(&domain).ok_or("unqualified domain")?;
        let held = self
            .holds
            .values()
            .filter(|hold| hold.domain == domain)
            .try_fold(0u64, |total, hold| {
                total.checked_add(hold.bytes).ok_or("overflow")
            })?;
        require(
            allocated
                .checked_add(held)
                .and_then(|sum| sum.checked_add(bytes))
                .ok_or("overflow")?
                <= *capacity,
            "admission refused",
        )?;
        self.holds.insert(
            operation,
            Hold {
                domain,
                bytes,
                accepted: false,
                unknown: false,
                owner_epoch: self.owner_epoch,
            },
        );
        self.audit()
    }

    fn prepare_group(&self, id: u64, operations: &[(u64, &str)]) -> Result<Group> {
        require(
            !operations.is_empty() && operations.len() <= 8,
            "bounded group required",
        )?;
        let mut seen = BTreeSet::new();
        let mut prefix = self.accepted_prefix.clone();
        let mut receipts = Vec::new();
        for (index, (operation, request)) in operations.iter().enumerate() {
            require(
                seen.insert(*operation) && !self.receipts.contains_key(operation),
                "duplicate operation",
            )?;
            let ordinal = self.accepted_ordinal + index as u64 + 1;
            let receipt = Receipt {
                operation: *operation,
                request: (*request).into(),
                ordinal,
                before: self.authored_revision + index as u64,
                after: self.authored_revision + index as u64 + 1,
            };
            prefix = link(&prefix, &receipt);
            receipts.push(receipt);
        }
        Ok(Group {
            id,
            previous: self.accepted_prefix.clone(),
            through: self.accepted_ordinal + operations.len() as u64,
            receipts,
            prefix,
        })
    }

    fn accept_group(&mut self, group: &Group, journal_barrier: bool) -> Result<()> {
        if let Some(original) = self.groups.get(&group.id) {
            return require(original == group, "group ID reused with different bytes");
        }
        require(
            journal_barrier,
            "not Accepted before exact finite journal barrier",
        )?;
        require(group.previous == self.accepted_prefix, "group prefix fork")?;
        let hold = self.holds.get(&group.id).ok_or("no checkpoint reserve")?;
        require(
            !hold.unknown && hold.owner_epoch == self.owner_epoch,
            "unresolved or stale reserve",
        )?;
        let mut prefix = group.previous.clone();
        let mut seen = BTreeSet::new();
        let mut revision = self.authored_revision;
        for (index, receipt) in group.receipts.iter().enumerate() {
            require(
                receipt.ordinal == self.accepted_ordinal + index as u64 + 1,
                "group ordinal gap",
            )?;
            require(
                seen.insert(receipt.operation) && !self.receipts.contains_key(&receipt.operation),
                "duplicate accepted operation",
            )?;
            require(
                receipt.before == revision
                    && receipt.after >= revision
                    && receipt.after <= revision.checked_add(1).ok_or("revision overflow")?,
                "receipt authored-coordinate discontinuity",
            )?;
            revision = receipt.after;
            prefix = link(&prefix, receipt);
        }
        require(
            !group.receipts.is_empty() && group.receipts.len() <= 8,
            "empty/oversized accepted group",
        )?;
        require(
            prefix == group.prefix && group.through == group.receipts.last().unwrap().ordinal,
            "group commitment mismatch",
        )?;
        self.holds.get_mut(&group.id).unwrap().accepted = true;
        for receipt in &group.receipts {
            self.receipts.insert(receipt.operation, receipt.clone());
        }
        self.authored_revision = group.receipts.last().unwrap().after;
        self.saved_revision = None;
        self.accepted_ordinal = group.through;
        self.accepted_prefix.clone_from(&group.prefix);
        self.groups.insert(group.id, group.clone());
        self.audit()
    }

    fn retry(&self, operation: u64, request: &str) -> Result<&Receipt> {
        let original = self.receipts.get(&operation).ok_or("unknown operation")?;
        require(original.request == request, "operation digest conflict")?;
        Ok(original)
    }

    fn mark_saved(
        &mut self,
        through: u64,
        revision: u64,
        prefix: &str,
        barrier: bool,
    ) -> Result<()> {
        require(
            barrier && self.pending_selection.is_none(),
            "unresolved publication",
        )?;
        require(
            self.holds.values().all(|hold| !hold.unknown),
            "unknown reserve cannot certify current Saved",
        )?;
        require(
            through == self.accepted_ordinal
                && revision == self.authored_revision
                && prefix == self.accepted_prefix,
            "receipt does not cover current accepted state",
        )?;
        self.saved_revision = Some(revision);
        Ok(())
    }

    fn begin_selection(&mut self, operation: u64, candidate: u64) -> Result<()> {
        require(
            self.pending_selection.is_none(),
            "unknown selection freezes admission",
        )?;
        require(
            self.holds.contains_key(&operation),
            "selection missing reserve",
        )?;
        self.pending_selection = Some((operation, self.active_pack, candidate));
        Ok(())
    }

    fn reconcile_selection(
        &mut self,
        operation: u64,
        observed: u64,
        barrier: bool,
        owner_epoch: u64,
    ) -> Result<()> {
        require(owner_epoch == self.owner_epoch, "stale selector authority")?;
        let (original, old, candidate) = self.pending_selection.ok_or("no pending selection")?;
        require(
            operation == original && (observed == old || observed == candidate),
            "unrelated selection",
        )?;
        require(
            barrier,
            "selected bytes alone do not prove publication barrier",
        )?;
        self.active_pack = observed;
        self.pending_selection = None;
        self.audit()
    }

    fn advance_recovery(&mut self, candidate: u64, barrier: bool) -> Result<()> {
        require(
            self.pending_selection.is_none() && barrier,
            "recovery transition unresolved",
        )?;
        require(
            candidate == self.active_pack,
            "recovery must select verified replacement",
        )?;
        self.recovery_pack = candidate;
        self.audit()
    }

    fn retire(&mut self, pack: u64) -> Result<()> {
        require(
            self.pending_selection.is_none(),
            "unknown selector retains both generations",
        )?;
        require(
            pack != self.active_pack && pack != self.recovery_pack,
            "both selections must leave candidate",
        )?;
        require(
            self.reader_packs.values().all(|selected| *selected != pack),
            "outstanding reader lease",
        )?;
        require(
            self.holds.values().all(|hold| !hold.unknown),
            "unresolved reserve keeps compaction evidence",
        )?;
        self.retired_packs.insert(pack);
        self.audit()
    }
}

fn pins() -> Result<()> {
    let mut model = Model::new();
    for (token, kind) in [
        "active",
        "recovery",
        "accepted-journal",
        "undo",
        "history",
        "conversion",
        "export",
        "backup",
        "reader",
        "unresolved-intent",
        "relocation",
        "resource-obligation",
    ]
    .into_iter()
    .enumerate()
    {
        model.pin(
            token as u64 + 1,
            Pin {
                object: 2,
                epoch: 7,
                kind: kind.into(),
                unknown: true,
            },
        )?;
    }
    let baseline = digest(&model);
    require(
        model.release(1, 7, 1, false).is_err(),
        "unresolved pin release accepted",
    )?;
    require(digest(&model) == baseline, "failed release mutated counts")?;
    for token in 1..12 {
        model.release(token, 7, 1, true)?;
    }
    require(model.counts[&2] == 1, "remaining pin undercounted")?;
    let mut mutant = model.clone();
    *mutant.counts.get_mut(&2).unwrap() = 0;
    require(
        mutant.audit().is_err(),
        "undercount mutant escaped independent audit",
    )
}

fn lease_aba() -> Result<()> {
    let mut model = Model::new();
    model.pin(
        5,
        Pin {
            object: 2,
            epoch: 7,
            kind: "reader".into(),
            unknown: false,
        },
    )?;
    model.release(5, 7, 1, true)?;
    model.pin(
        5,
        Pin {
            object: 3,
            epoch: 8,
            kind: "reader".into(),
            unknown: false,
        },
    )?;
    model.release(5, 7, 1, true)?; // Original old release is idempotent, not a new release.
    require(
        model.pins[&5].epoch == 8 && model.counts[&3] == 1,
        "ABA release removed new lease",
    )?;
    model.owner_epoch = 2;
    require(
        model.release(5, 8, 1, true).is_err(),
        "stale owner released new lease",
    )?;
    model.audit()
}

fn grouped_receipts() -> Result<()> {
    let mut model = Model::new();
    model.reserve(100, &[(7, 100)])?;
    let group = model.prepare_group(100, &[(1, "a"), (2, "b"), (3, "c")])?;
    require(
        model.accept_group(&group, false).is_err(),
        "pre-barrier acceptance",
    )?;
    let baseline = digest(&model);
    let mut malformed = group.clone();
    malformed.receipts[1].ordinal += 1;
    require(
        model.accept_group(&malformed, true).is_err(),
        "ordinal gap accepted",
    )?;
    require(
        digest(&model) == baseline,
        "invalid group partially applied",
    )?;
    let mut disconnected = group.clone();
    disconnected.receipts[1].before = 0;
    disconnected.prefix = disconnected
        .receipts
        .iter()
        .fold(disconnected.previous.clone(), |prefix, receipt| {
            link(&prefix, receipt)
        });
    require(
        model.accept_group(&disconnected, true).is_err(),
        "hash-valid group with disconnected authored coordinates accepted",
    )?;
    model.accept_group(&group, true)?;
    let accepted = digest(&model);
    model.accept_group(&group, true)?;
    require(digest(&model) == accepted, "group replay applied twice")?;
    require(model.retry(1, "a")?.ordinal == 1, "original outcome lost")?;
    require(model.retry(1, "other").is_err(), "changed retry accepted")?;
    model.mark_saved(3, 3, &group.prefix, true)?;
    model.reserve(101, &[(7, 100)])?;
    let later = model.prepare_group(101, &[(4, "d")])?;
    model.accept_group(&later, true)?;
    require(
        model.saved_revision.is_none(),
        "later acceptance retained stale current-Saved flag",
    )?;
    require(
        model
            .mark_saved(
                group.through,
                group.receipts.last().unwrap().after,
                &group.prefix,
                true,
            )
            .is_err(),
        "old group labelled current Saved",
    )?;
    let prefix = model.accepted_prefix.clone();
    model.holds.get_mut(&101).unwrap().unknown = true;
    require(
        model.mark_saved(4, 4, &prefix, true).is_err(),
        "unknown allocation labelled Saved",
    )?;
    model.holds.get_mut(&101).unwrap().unknown = false;
    model.mark_saved(4, 4, &prefix, true)?;
    model.audit()
}

fn compaction_and_unknown() -> Result<()> {
    let mut model = Model::new();
    model.reserve(100, &[(7, 200)])?;
    model.reader_packs.insert((9, 1), 10);
    model.begin_selection(100, 11)?;
    require(
        model.retire(10).is_err(),
        "unknown selector retired old pack",
    )?;
    require(
        model.reconcile_selection(100, 12, true, 1).is_err(),
        "unrelated HEAD accepted",
    )?;
    require(
        model.reconcile_selection(100, 11, false, 1).is_err(),
        "observed candidate invented barrier",
    )?;
    model.reconcile_selection(100, 11, true, 1)?;
    require(
        model.retire(10).is_err(),
        "first selection retired recovery",
    )?;
    model.advance_recovery(11, true)?;
    require(model.retire(10).is_err(), "second selection ignored reader")?;
    require(
        model.reader_packs.remove(&(9, 2)).is_none(),
        "stale epoch matched reader",
    )?;
    require(
        model.retire(10).is_err(),
        "stale reader close authorized compaction",
    )?;
    model.reader_packs.remove(&(9, 1));
    model.holds.get_mut(&100).unwrap().unknown = true;
    require(
        model.retire(10).is_err(),
        "unknown reserve discarded evidence",
    )?;
    model.holds.get_mut(&100).unwrap().unknown = false; // Oracle receives exact original-ID allocation evidence.
    model.retire(10)?;
    require(
        model.domains[&7].1 == 400,
        "retirement spent deletion credit",
    )?;
    model.audit()
}

fn shared_capacity() -> Result<()> {
    let mut model = Model::new();
    // Package and Asset Store share physical domain 7: 350+350 cannot fit 600.
    require(
        model.reserve(1, &[(7, 350), (7, 350)]).is_err(),
        "roles bypassed shared-domain admission",
    )?;
    model.reserve(1, &[(7, 350)])?;
    let baseline = digest(&model);
    require(
        model.reserve(2, &[(7, 350)]).is_err(),
        "concurrent group oversubscribed",
    )?;
    require(
        digest(&model) == baseline,
        "refused reserve partially applied",
    )?;
    model.holds.get_mut(&1).unwrap().accepted = true;
    require(
        model.reserve(3, &[(7, 251)]).is_err(),
        "accepted-unsaved liability disappeared",
    )?;
    model.reserve(3, &[(7, 250)])?;
    let mut mutant = model.clone();
    mutant.holds.get_mut(&3).unwrap().bytes += 1;
    require(
        mutant.audit().is_err(),
        "oversubscription mutant escaped audit",
    )?;
    require(
        model.reserve(4, &[(7, u64::MAX), (7, 1)]).is_err(),
        "reserve overflow accepted",
    )?;
    model.audit()
}

fn receipt_fork() -> Result<()> {
    let mut model = Model::new();
    model.reserve(1, &[(7, 100)])?;
    model.reserve(2, &[(7, 100)])?;
    let first = model.prepare_group(1, &[(1, "a")])?;
    let fork = model.prepare_group(2, &[(2, "b")])?;
    model.accept_group(&first, true)?;
    require(
        model.accept_group(&fork, true).is_err(),
        "concurrent stale prefix accepted",
    )?;
    let mut tampered = first;
    tampered.receipts[0].request = "changed".into();
    require(
        model.accept_group(&tampered, true).is_err(),
        "group identity rebound",
    )?;
    model.audit()
}

fn no_op_receipt() -> Result<()> {
    let mut model = Model::new();
    model.reserve(1, &[(7, 100)])?;
    let mut group = model.prepare_group(1, &[(1, "edit"), (2, "no-op"), (3, "edit")])?;
    group.receipts[1].after = 1;
    group.receipts[2].before = 1;
    group.receipts[2].after = 2;
    group.prefix = group
        .receipts
        .iter()
        .fold(group.previous.clone(), |prefix, receipt| {
            link(&prefix, receipt)
        });
    model.accept_group(&group, true)?;
    require(
        model.accepted_ordinal == 3 && model.authored_revision == 2,
        "acceptance ordinal conflated with authored revision",
    )?;
    model.mark_saved(3, 2, &group.prefix, true)?;
    model.audit()
}

fn main() -> Result<()> {
    type Scenario = (&'static str, fn() -> Result<()>);
    let scenarios: [Scenario; 7] = [
        ("all-pin-classes-and-undercount", pins),
        ("reader-lease-ABA-and-owner-epoch", lease_aba),
        (
            "grouped-receipt-continuity-and-current-Saved",
            grouped_receipts,
        ),
        (
            "two-selections-readers-and-unknown-reserve",
            compaction_and_unknown,
        ),
        (
            "shared-domain-oversubscription-and-overflow",
            shared_capacity,
        ),
        ("concurrent-receipt-prefix-fork", receipt_fork),
        (
            "no-op-receipt-keeps-ordinal-and-authored-revision-distinct",
            no_op_receipt,
        ),
    ];
    for (name, scenario) in scenarios {
        scenario()?;
        println!(
            "{}",
            json!({"fixture":"independent-combined-adversarial-oracle","scenario":name,"result":"pass",
            "actual_v3_code_exercised":false,"physical_io":false,"qualification":false})
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_pin_classes_and_undercount() {
        pins().unwrap();
    }
    #[test]
    fn stale_reader_lease_aba() {
        lease_aba().unwrap();
    }
    #[test]
    fn grouped_receipt_continuity() {
        grouped_receipts().unwrap();
    }
    #[test]
    fn compaction_two_selections_and_unknowns() {
        compaction_and_unknown().unwrap();
    }
    #[test]
    fn shared_domain_and_overflow() {
        shared_capacity().unwrap();
    }
    #[test]
    fn concurrent_receipt_fork() {
        receipt_fork().unwrap();
    }
    #[test]
    fn accepted_no_op_coordinate() {
        no_op_receipt().unwrap();
    }
}
