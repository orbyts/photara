//! LL1/PS3/PS4 disposable activation fixture, not production persistence evidence.
//! Synthetic single GUI slot and trusted evidence ledgers; all effects in memory.
//! No SQL, real package codec/lease, filesystem, native confirmation or wire.

#[path = "ll1_activation/model.rs"]
mod model;

use model::*;
use photara_core::contracts::ids::{CommitId, GraphId, LibraryId, OperationId, ProjectId};
use uuid::Uuid;

fn operation(n: u128) -> OperationId {
    OperationId::from_uuid(Uuid::from_u128(n)).unwrap()
}
fn saved(n: u128, through: u64) -> Saved {
    Saved {
        identity: Identity {
            library: LibraryId::from_uuid(Uuid::from_u128(n)).unwrap(),
            project: ProjectId::from_uuid(Uuid::from_u128(n + 10)).unwrap(),
            incarnation: 40 + u64::try_from(n).unwrap(),
            bootstrap: [u8::try_from(n).unwrap(); 32],
        },
        owner: (50 + u64::try_from(n).unwrap(), 1),
        through,
        authored_revision: through,
        authored_digest: [u8::try_from(through).unwrap(); 32],
        graph: (
            GraphId::from_uuid(Uuid::from_u128(n + 20)).unwrap(),
            through,
            [3; 32],
        ),
        commit: CommitId::from_uuid(Uuid::from_u128(n + 30 + u128::from(through))).unwrap(),
        commit_digest: [4; 32],
        head_digest: [5; 32],
        package_revision: through,
        capability: 6,
    }
}
fn target(s: &Saved, principal: u64) -> Target {
    Target {
        scope: Scope {
            authority: principal + 100,
            principal,
        },
        library: s.identity.library,
        project: Some(s.identity.clone()),
        graph: Some(s.graph.0),
        view_version: 1,
    }
}
fn fixture() -> (Coordinator, Saved, Request, TargetEvidence) {
    let source = saved(1, 1);
    let destination = saved(2, 1);
    let evidence = TargetEvidence {
        target: target(&destination, 2),
        saved: Some(destination),
        restored: true,
    };
    let request = Request {
        operation: operation(100),
        target: evidence.target.clone(),
    };
    let coordinator = Coordinator::new(
        Disk::new(target(&source, 1)),
        source.clone(),
        evidence.clone(),
    );
    (coordinator, source, request, evidence)
}
fn prepared(c: &mut Coordinator, s: &Saved, r: &Request) {
    c.discover(r.clone()).unwrap();
    assert_eq!(c.prepare(Some(s.clone())), Ok(Some(r.operation)));
}
fn detached(c: &mut Coordinator, s: &Saved, r: &Request) {
    prepared(c, s, r);
    c.freeze(r.operation).unwrap();
    c.detach(r.operation, Ok(Some(s.clone())), true).unwrap();
}

#[test]
fn discovery_supersedes_but_preparation_serializes_and_late_callbacks_refuse() {
    let (mut c, source, b, evidence) = fixture();
    let mut next = b.clone();
    next.operation = operation(101);
    c.discover(b.clone()).unwrap();
    c.discover(next.clone()).unwrap();
    assert_eq!(c.prepare(Some(source.clone())), Ok(Some(next.operation)));
    assert_eq!(c.freeze(b.operation), Err(Failure::Stale));
    assert_eq!(c.discover(b.clone()), Err(Failure::Busy));
    assert!(!c.source_editable);
    c.cancel(next.operation, true).unwrap();
    prepared(&mut c, &source, &b);
    assert_eq!(c.freeze(next.operation), Err(Failure::Stale));
    c.freeze(b.operation).unwrap();
    c.detach(b.operation, Ok(Some(source)), true).unwrap();
    c.publish(b.operation, evidence, PublishFault::None)
        .unwrap();
    assert_eq!(c.disk.borrow().receipts.len(), 1);
}

#[test]
fn every_saved_coordinate_is_bound_and_missing_evidence_never_detaches() {
    let changes: &[fn(&mut Saved)] = &[
        |s| s.owner.0 += 1,
        |s| s.owner.1 += 1,
        |s| s.identity.incarnation += 1,
        |s| s.identity.bootstrap[0] ^= 1,
        |s| s.identity.library = saved(3, 1).identity.library,
        |s| s.identity.project = saved(3, 1).identity.project,
        |s| s.capability += 1,
        |s| s.through += 1,
        |s| s.authored_revision += 1,
        |s| s.authored_digest[0] ^= 1,
        |s| s.graph.0 = saved(3, 1).graph.0,
        |s| s.graph.1 += 1,
        |s| s.graph.2[0] ^= 1,
        |s| s.commit = saved(3, 1).commit,
        |s| s.commit_digest[0] ^= 1,
        |s| s.head_digest[0] ^= 1,
        |s| s.package_revision += 1,
    ];
    for change in changes {
        let (mut c, source, request, _) = fixture();
        let mut bad = source.clone();
        change(&mut bad);
        c.discover(request.clone()).unwrap();
        assert_eq!(c.prepare(Some(bad.clone())), Err(Failure::Evidence));
        assert_eq!(c.disk.borrow().capsule_writes, 0);
        c.prepare(Some(source)).unwrap();
        c.freeze(request.operation).unwrap();
        assert_eq!(
            c.detach(request.operation, Ok(Some(bad)), true),
            Err(Failure::Evidence)
        );
        assert_eq!(c.disk.borrow().capsule_writes, 0);
    }
    let (mut c, source, request, _) = fixture();
    prepared(&mut c, &source, &request);
    c.freeze(request.operation).unwrap();
    c.source_ledger.clear(); // Reference/checksum cannot recover its evidence object.
    assert_eq!(
        c.detach(request.operation, Ok(Some(source)), true),
        Err(Failure::Evidence)
    );
}

#[test]
fn two_barriers_cover_finite_prefix_without_freezing_agent_or_claiming_global_saved() {
    let (mut c, source, request, evidence) = fixture();
    prepared(&mut c, &source, &request);
    assert!(c.confirmation_claim_current(request.operation));
    let second = saved(1, 2);
    c.source_ledger.insert(2, second.clone());
    c.accepted_sequence = 2; // Agent accepted more work while GUI sheet is present.
    assert!(!c.confirmation_claim_current(request.operation));
    c.freeze(request.operation).unwrap();
    let third = saved(1, 3);
    c.source_ledger.insert(3, third);
    c.accepted_sequence = 3; // Still not an unbounded wait for every client's work.
    assert_eq!(
        c.detach(request.operation, Ok(Some(source)), true),
        Err(Failure::Evidence)
    );
    c.detach(request.operation, Ok(Some(second.clone())), true)
        .unwrap();
    assert_eq!(c.disk.borrow().capsules[&request.operation].barrier, second);
    c.publish(request.operation, evidence, PublishFault::None)
        .unwrap();
    assert!(c.agent_run_active);
    assert_eq!(c.owner_exits, 0);
    assert!(c.accepted_sequence > second.through); // Global state would remain Saving.
}

#[test]
fn library_only_destination_still_flushes_source_and_same_library_is_noop() {
    let (mut c, source, mut request, mut evidence) = fixture();
    request.target.project = None;
    request.target.graph = None;
    evidence.target = request.target.clone();
    evidence.saved = None;
    c.target_evidence = Some(evidence.clone());
    c.discover(request.clone()).unwrap();
    assert_eq!(c.prepare(None), Err(Failure::Evidence));
    c.prepare(Some(source.clone())).unwrap();
    c.freeze(request.operation).unwrap();
    assert_eq!(
        c.detach(request.operation, Err(Failure::SaveUnknown), true),
        Err(Failure::SaveUnknown)
    );
    assert_eq!(c.disk.borrow().generation, 1);
    c.detach(request.operation, Ok(Some(source)), true).unwrap();
    c.publish(request.operation, evidence, PublishFault::None)
        .unwrap();
    c.establish(request.operation, true).unwrap();
    assert!(c.disk.borrow().visible.as_ref().unwrap().project.is_none());
    assert!(!c.target_editable);

    let (mut c, _, mut request, _) = fixture();
    request.target = c.disk.borrow().visible.clone().unwrap();
    request.target.project = None;
    request.target.graph = None;
    c.discover(request).unwrap();
    assert_eq!(c.prepare(None), Ok(None));
    assert!(c.disk.borrow().visible.as_ref().unwrap().project.is_some());
    assert_eq!(c.disk.borrow().generation, 1);
    assert_eq!(c.disk.borrow().capsule_writes, 0);
}

#[test]
fn explicit_no_source_project_needs_no_fabricated_save_or_capsule() {
    let (mut c, _, request, evidence) = fixture();
    {
        let mut disk = c.disk.borrow_mut();
        let current = disk.visible.as_mut().unwrap();
        current.project = None;
        current.graph = None;
    }
    c.discover(request.clone()).unwrap();
    assert_eq!(c.prepare(None), Ok(Some(request.operation)));
    c.freeze(request.operation).unwrap();
    c.detach(request.operation, Ok(None), true).unwrap();
    assert_eq!(c.disk.borrow().capsule_writes, 0);
    c.publish(request.operation, evidence, PublishFault::None)
        .unwrap();
    c.establish(request.operation, true).unwrap();
    assert!(c.target_editable);
}

#[test]
fn distinct_target_identity_and_graph_view_must_restore_before_atomic_publication() {
    let changes: &[fn(&mut TargetEvidence)] = &[
        |e| e.saved = Some(saved(1, 1)), // A valid source receipt is not target evidence.
        |e| e.target.graph = Some(saved(3, 1).graph.0),
        |e| e.target.view_version += 1,
        |e| e.target.scope.principal += 1,
        |e| e.target.project.as_mut().unwrap().incarnation += 1,
        |e| e.restored = false,
    ];
    for change in changes {
        let (mut c, source, request, mut evidence) = fixture();
        detached(&mut c, &source, &request);
        change(&mut evidence);
        assert_eq!(
            c.publish(request.operation, evidence, PublishFault::None),
            Err(Failure::RestoreFailed)
        );
        assert_eq!(c.disk.borrow().generation, 1);
        assert!(!c.target_editable);
        assert_eq!(c.cancel(request.operation, false), Ok(Recovery::ReadOnly));
    }
}

#[test]
fn one_slot_serializes_cross_scope_requests_and_publishes_all_coordinates_together() {
    let (mut first, source, request, evidence) = fixture();
    let mut second = Coordinator::new(first.disk.clone(), source.clone(), evidence.clone());
    let mut other = request.clone();
    other.operation = operation(101);
    other.target.scope = Scope {
        authority: 999,
        principal: 888,
    };
    second.discover(other).unwrap();
    detached(&mut first, &source, &request);
    assert_eq!(second.prepare(Some(source.clone())), Err(Failure::Busy));
    let old = first.disk.borrow().visible.clone().unwrap();
    let receipt = first
        .publish(request.operation, evidence, PublishFault::None)
        .unwrap();
    let disk = first.disk.borrow();
    assert_eq!(disk.visible, Some(request.target.clone()));
    assert_eq!(disk.generation, receipt.generation);
    assert_eq!(disk.receipts.len(), 1);
    assert!(disk.preferences.contains(&old));
    assert_eq!(disk.visible.as_ref().unwrap().graph, request.target.graph);
    drop(disk);
    assert_eq!(second.prepare(Some(source)), Err(Failure::Evidence));
    assert_eq!(second.freeze(request.operation), Err(Failure::Stale));
}

#[test]
fn capsule_precedes_detach_and_survives_commit_until_successful_establishment() {
    let (mut c, source, request, evidence) = fixture();
    prepared(&mut c, &source, &request);
    c.freeze(request.operation).unwrap();
    assert_eq!(
        c.detach(request.operation, Ok(Some(source.clone())), false),
        Err(Failure::CapsuleFailed)
    );
    assert_eq!(
        c.publish(request.operation, evidence.clone(), PublishFault::None),
        Err(Failure::NotReady)
    );
    assert!(matches!(
        Coordinator::recover(&c.disk, request.operation, true),
        Recovery::Current(_)
    ));
    c.detach(request.operation, Ok(Some(source)), true).unwrap();
    assert!(matches!(
        Coordinator::recover(&c.disk, request.operation, true),
        Recovery::Current(_)
    ));
    assert_eq!(
        Coordinator::recover(&c.disk, request.operation, false),
        Recovery::ReadOnly
    );
    c.publish(request.operation, evidence, PublishFault::None)
        .unwrap();
    assert_eq!(
        Coordinator::recover(&c.disk, request.operation, true),
        Recovery::Target(request.target)
    );
    assert_eq!(
        c.establish(request.operation, false),
        Err(Failure::RestoreFailed)
    );
    assert!(c.disk.borrow().capsules.contains_key(&request.operation));
    assert!(!c.target_editable);
    c.establish(request.operation, true).unwrap();
    assert!(!c.disk.borrow().capsules.contains_key(&request.operation));
    assert!(c.target_editable);
    assert!(!c.source_editable);
}

#[test]
fn pointer_failure_rolls_back_but_lost_reply_recovers_exact_activation_once() {
    let (mut c, source, request, evidence) = fixture();
    detached(&mut c, &source, &request);
    let old = c.disk.borrow().visible.clone();
    assert_eq!(
        c.publish(
            request.operation,
            evidence.clone(),
            PublishFault::BeforeCommit
        ),
        Err(Failure::PointerFailed)
    );
    assert_eq!(c.disk.borrow().visible, old);
    assert!(c.disk.borrow().receipts.is_empty());
    assert_eq!(
        c.publish(request.operation, evidence, PublishFault::LostReply),
        Err(Failure::ReplyLost)
    );
    assert_eq!(c.disk.borrow().generation, 2);
    let recovered = c.lookup(&request, request.target.scope).unwrap();
    assert_eq!(recovered.generation, 2);
    assert_eq!(c.cancel(request.operation, true), Err(Failure::Stale));
    let mut changed = request.clone();
    changed.target.view_version += 1;
    assert_eq!(
        c.lookup(&changed, changed.target.scope),
        Err(Failure::IntentConflict)
    );
    assert_eq!(c.discover(changed), Err(Failure::IntentConflict));
    assert_eq!(c.discover(request.clone()), Err(Failure::NotReady));
    assert_eq!(
        c.lookup(
            &request,
            Scope {
                authority: 99,
                principal: 99
            }
        ),
        Err(Failure::Fenced)
    );
    assert_eq!(c.disk.borrow().receipts.len(), 1);
    assert!(c.disk.borrow().capsules.contains_key(&request.operation));
}

#[test]
fn stop_completion_and_fresh_grant_owner_attachment_are_required_without_owner_exit() {
    let changes: &[fn(&mut Coordinator)] = &[
        |c| c.owner.1 += 1,
        |c| c.attachment_generation += 1,
        |c| c.authorization_generation += 1,
        |c| c.grant_valid = false,
    ];
    for change in changes {
        let (mut c, source, request, evidence) = fixture();
        c.gui_run_active = true;
        c.discover(request.clone()).unwrap();
        assert_eq!(c.prepare(Some(source.clone())), Err(Failure::RunActive));
        c.stop_requested = true;
        assert_eq!(c.prepare(Some(source.clone())), Err(Failure::RunActive));
        c.gui_run_active = false; // Inject terminal callbacks/effect bookkeeping settled.
        c.prepare(Some(source.clone())).unwrap();
        c.freeze(request.operation).unwrap();
        c.detach(request.operation, Ok(Some(source)), true).unwrap();
        change(&mut c);
        assert!(matches!(
            c.publish(request.operation, evidence, PublishFault::None),
            Err(Failure::Stale | Failure::Fenced)
        ));
        assert_eq!(c.disk.borrow().generation, 1);
        assert!(c.agent_run_active);
        assert_eq!(c.owner_exits, 0);
        assert!(!c.target_editable);
        assert_eq!(c.cancel(request.operation, true), Ok(Recovery::ReadOnly));
    }
}

#[test]
fn removal_before_and_after_commit_fences_without_capsule_or_package_file_effects() {
    for remove_source in [false, true] {
        let (mut c, source, request, evidence) = fixture();
        detached(&mut c, &source, &request);
        let effects = c.disk.borrow().file_effects.clone();
        let capsule = c.disk.borrow().capsules[&request.operation].barrier.clone();
        c.remove_library(if remove_source {
            source.identity.library
        } else {
            request.target.library
        });
        assert_eq!(
            c.publish(request.operation, evidence, PublishFault::None),
            Err(Failure::Stale)
        );
        assert_eq!(c.disk.borrow().file_effects, effects);
        assert_eq!(
            c.disk.borrow().capsules[&request.operation].barrier,
            capsule
        );
        assert!(!c.source_editable && !c.target_editable);
        assert!(c.disk.borrow().receipts.is_empty());
    }
    let (mut c, source, request, evidence) = fixture();
    detached(&mut c, &source, &request);
    c.publish(request.operation, evidence, PublishFault::None)
        .unwrap();
    let effects = c.disk.borrow().file_effects.clone();
    c.remove_library(request.target.library);
    assert!(c.disk.borrow().visible.is_none());
    assert_eq!(c.establish(request.operation, true), Err(Failure::Fenced));
    assert_eq!(
        Coordinator::recover(&c.disk, request.operation, true),
        Recovery::Fenced
    );
    assert_eq!(c.disk.borrow().file_effects, effects);
    assert!(c.disk.borrow().capsules.contains_key(&request.operation));
    assert_eq!(c.disk.borrow().receipts.len(), 1);
}

#[test]
fn unrelated_removal_preserves_slot_and_target_marker_blocks_new_activation() {
    let (mut c, source, request, _) = fixture();
    let old = c.disk.borrow().visible.clone();
    c.remove_library(saved(3, 1).identity.library);
    assert_eq!(c.disk.borrow().visible, old);
    assert!(c.source_editable);
    assert!(c.disk.borrow().file_effects.is_empty());
    c.remove_library(request.target.library);
    assert_eq!(c.discover(request), Err(Failure::Fenced));
    assert_eq!(
        c.disk.borrow().visible.as_ref().unwrap().library,
        source.identity.library
    );
}
