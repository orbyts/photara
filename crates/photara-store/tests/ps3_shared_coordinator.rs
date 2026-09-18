//! PS3 disposable shared-authority/coordinator fixture, not production durability.
//!
//! Models `PROJECT_SESSION_DURABILITY.md` admission, finite-prefix flush, epochs,
//! cancellation and recovery using injected memory effects. Uses the existing
//! `OperationId` type but defines no wire, native UI, filesystem access, real lease,
//! IPC routing, OS/process liveness, Graph codec or hardware persistence claim.
//! GUI, CLI/headless and agent are logical attachments to ONE owner; a second
//! direct owner gets `WriterBusy`. Cross-process routing/handoff remains deferred.

#[path = "ps3_shared_coordinator/model.rs"]
mod model;

use model::*;
use photara_core::contracts::ids::OperationId;
use uuid::Uuid;

fn operation(n: u128) -> OperationId {
    OperationId::from_uuid(Uuid::from_u128(n)).unwrap()
}
fn request(model: &Coordinator, n: u128, value: i64) -> Request {
    Request {
        operation: operation(n),
        expected: model.current,
        value,
    }
}
fn clients(model: &mut Coordinator) -> [Attachment; 3] {
    [
        model.attach(1, Surface::Gui, 1).unwrap(),
        model.attach(2, Surface::Headless, 2).unwrap(),
        model.attach(3, Surface::Agent, 3).unwrap(),
    ]
}
fn accept(model: &mut Coordinator, client: Attachment, n: u128, value: i64) -> Accepted {
    assert_eq!(
        model.submit(client, request(model, n, value)),
        Ok(Submission::Queued)
    );
    model.process_next(JournalFault::None).unwrap()
}

#[test]
fn all_surfaces_share_one_order_queue_and_finite_flush() {
    let storage = FakeStorage::new();
    let mut model = Coordinator::claim(storage.clone(), 2).unwrap();
    let [gui, cli, agent] = clients(&mut model);
    assert_eq!(model.surface(gui), Ok(Surface::Gui));
    assert_eq!(model.surface(cli), Ok(Surface::Headless));
    assert_eq!(model.surface(agent), Ok(Surface::Agent));
    let initial = model.current;
    let gui_request = request(&model, 1, 10);
    let competing = request(&model, 2, 20);
    assert_eq!(model.submit(gui, gui_request), Ok(Submission::Queued));
    assert_eq!(model.submit(cli, competing), Ok(Submission::Queued));
    assert_eq!(
        model.submit(agent, request(&model, 3, 30)),
        Err(Refusal::Backpressure)
    );
    assert_eq!(model.current, initial); // queued is neither Accepted nor Saved
    assert_eq!(model.status(), Status::Saving);
    assert_eq!(
        model
            .process_next(JournalFault::None)
            .unwrap()
            .after
            .sequence,
        1
    );
    assert_eq!(model.process_next(JournalFault::None), Err(Refusal::Stale));
    let first_flush = model.begin_flush(gui).unwrap();
    accept(&mut model, agent, 3, 30);
    let first = model.complete_flush(first_flush, FlushFault::None).unwrap();
    assert_eq!(first.covered, Coordinate::new(1, 10));
    assert_eq!(model.current, Coordinate::new(2, 30));
    assert_eq!(model.status(), Status::Saving); // newer accepted prefix remains pending
    let latest = model.begin_flush(cli).unwrap();
    assert_eq!(
        model
            .complete_flush(latest, FlushFault::None)
            .unwrap()
            .covered,
        model.current
    );
    assert_eq!(model.status(), Status::Saved);
    assert_eq!(storage.borrow().journal_barriers, 2);
    assert_eq!(storage.borrow().replacements, 2);
    let noop = model.begin_flush(agent).unwrap();
    model.complete_flush(noop, FlushFault::None).unwrap();
    assert_eq!(storage.borrow().replacements, 2);
}

#[test]
fn operation_identity_retries_before_stale_but_never_before_authorization() {
    let storage = FakeStorage::new();
    let mut model = Coordinator::claim(storage.clone(), 3).unwrap();
    let [gui, cli, agent] = clients(&mut model);
    let exact = request(&model, 1, 8);
    model.submit(gui, exact.clone()).unwrap();
    let receipt = model.process_next(JournalFault::None).unwrap();
    accept(&mut model, cli, 2, 9);
    assert_eq!(
        model.submit(gui, exact.clone()),
        Ok(Submission::Existing(receipt))
    );
    let altered = Request {
        value: 99,
        ..exact.clone()
    };
    assert_eq!(model.submit(gui, altered), Err(Refusal::IntentConflict));
    assert_eq!(
        model.submit(agent, exact.clone()),
        Err(Refusal::Unauthorized)
    );
    storage.borrow_mut().grants.get_mut(&1).unwrap().active = false;
    assert_eq!(model.submit(gui, exact), Err(Refusal::Unauthorized));
    assert_eq!(storage.borrow().journal.len(), 2);
}

#[test]
fn authorization_is_rechecked_after_queueing_and_for_every_surface() {
    let storage = FakeStorage::new();
    let mut model = Coordinator::claim(storage.clone(), 3).unwrap();
    let [gui, cli, agent] = clients(&mut model);
    for (client, principal) in [(gui, 1), (cli, 2), (agent, 3)] {
        let command = request(&model, u128::from(principal), 1);
        model.submit(client, command).unwrap();
        storage
            .borrow_mut()
            .grants
            .get_mut(&principal)
            .unwrap()
            .generation += 1;
        assert_eq!(
            model.process_next(JournalFault::None),
            Err(Refusal::Unauthorized)
        );
    }
    assert_eq!(storage.borrow().journal_barriers, 0);
    storage.borrow_mut().grants.get_mut(&1).unwrap().incarnation = 999;
    assert_eq!(model.attach(4, Surface::Agent, 1), Err(Refusal::Scope));
}

#[test]
fn freeze_is_attachment_local_and_flush_drains_that_clients_pending_input() {
    let storage = FakeStorage::new();
    let mut model = Coordinator::claim(storage, 3).unwrap();
    let [gui, _, agent] = clients(&mut model);
    model.submit(gui, request(&model, 1, 10)).unwrap();
    model.freeze(gui).unwrap();
    assert_eq!(model.begin_flush(gui), Err(Refusal::Pending));
    assert_eq!(
        model.submit(gui, request(&model, 2, 20)),
        Err(Refusal::Frozen)
    );
    model.process_next(JournalFault::None).unwrap();
    let ticket = model.begin_flush(gui).unwrap();
    accept(&mut model, agent, 3, 30); // unrelated client remains writable
    assert_eq!(
        model
            .complete_flush(ticket, FlushFault::None)
            .unwrap()
            .covered
            .sequence,
        1
    );
    assert_eq!(model.status(), Status::Saving);
    model.thaw(gui).unwrap();
    accept(&mut model, gui, 4, 40);
}

#[test]
fn cancellation_before_dispatch_has_no_effect_and_acceptance_cannot_be_cancelled() {
    let storage = FakeStorage::new();
    let mut model = Coordinator::claim(storage.clone(), 3).unwrap();
    let [gui, _, agent] = clients(&mut model);
    model.submit(gui, request(&model, 1, 10)).unwrap();
    assert_eq!(
        model.cancel_queued(agent, operation(1)),
        Err(Refusal::Unauthorized)
    );
    model.cancel_queued(gui, operation(1)).unwrap();
    assert_eq!(model.process_next(JournalFault::None), Err(Refusal::Empty));
    assert_eq!(storage.borrow().journal_barriers, 0);
    accept(&mut model, gui, 2, 20);
    assert_eq!(
        model.cancel_queued(agent, operation(2)),
        Err(Refusal::Unauthorized)
    );
    assert_eq!(
        model.cancel_queued(gui, operation(2)),
        Err(Refusal::TooLate)
    );
    assert_eq!(storage.borrow().journal.len(), 1);
}

#[test]
fn stop_request_is_not_terminal_completion_or_permission_to_freeze() {
    let storage = FakeStorage::new();
    let mut model = Coordinator::claim(storage, 3).unwrap();
    let [gui, _, agent] = clients(&mut model);
    model.start_run(gui).unwrap();
    model.start_run(agent).unwrap();
    assert_eq!(model.freeze(gui), Err(Refusal::RunActive));
    model.request_stop(gui).unwrap();
    assert_eq!(model.freeze(gui), Err(Refusal::RunActive));
    model.run_settled(gui).unwrap();
    model.freeze(gui).unwrap();
    assert_eq!(model.request_exit(), Err(Refusal::RunActive)); // agent work is independent
    assert_eq!(model.freeze(agent), Err(Refusal::RunActive));
    model.request_stop(agent).unwrap();
    model.run_settled(agent).unwrap();
    model.request_exit().unwrap();
    model.finish_exit().unwrap();
}

#[test]
fn lost_journal_reply_recovers_same_operation_without_duplicate_or_memory_ack() {
    for fault in [JournalFault::BeforeBarrier, JournalFault::AfterBarrier] {
        let storage = FakeStorage::new();
        let mut model = Coordinator::claim(storage.clone(), 2).unwrap();
        let gui = model.attach(1, Surface::Gui, 1).unwrap();
        let command = request(&model, 1, 90);
        model.submit(gui, command.clone()).unwrap();
        let error = model.process_next(fault).unwrap_err();
        assert_eq!(model.current, Coordinate::new(0, 0));
        assert_eq!(model.queued(), 1);
        assert_eq!(model.cancel_queued(gui, operation(1)), Err(error));
        assert_ne!(model.status(), Status::Saved);
        assert_eq!(
            storage.borrow().journal.len(),
            usize::from(fault == JournalFault::AfterBarrier)
        );
        model.recover().unwrap();
        let receipt = model.process_next(JournalFault::None).unwrap();
        assert_eq!(receipt.request, command);
        assert_eq!(receipt.after.sequence, 1);
        assert_eq!(storage.borrow().journal.len(), 1);
        assert_eq!(storage.borrow().journal_barriers, 1);
        assert_eq!(model.status(), Status::Saving);
    }
}

#[test]
fn publication_failures_preserve_exact_prefix_and_reconcile_without_replacement_ids() {
    for fault in [
        FlushFault::BeforeReplace,
        FlushFault::AfterReplace,
        FlushFault::AfterReceipt,
    ] {
        let storage = FakeStorage::new();
        let mut model = Coordinator::claim(storage.clone(), 2).unwrap();
        let gui = model.attach(1, Surface::Gui, 1).unwrap();
        let accepted = accept(&mut model, gui, 77, 70);
        let ticket = model.begin_flush(gui).unwrap();
        assert!(model.complete_flush(ticket, fault).is_err());
        assert_ne!(model.status(), Status::Saved);
        let current = model.current;
        model.recover().unwrap();
        assert_eq!(model.current, current);
        assert_eq!(
            model
                .complete_flush(ticket, FlushFault::None)
                .unwrap()
                .covered,
            current
        );
        assert_eq!(model.status(), Status::Saved);
        assert_eq!(storage.borrow().replacements, 1);
        assert_eq!(storage.borrow().journal, [accepted]);
    }
}

#[test]
fn old_attachment_and_owner_callbacks_never_mutate_replacement_context() {
    let storage = FakeStorage::new();
    let mut model = Coordinator::claim(storage.clone(), 3).unwrap();
    let old = model.attach(1, Surface::Gui, 1).unwrap();
    accept(&mut model, old, 1, 1);
    let ticket = model.begin_flush(old).unwrap();
    let current = model.attach(1, Surface::Agent, 1).unwrap();
    assert_eq!(
        model.complete_flush(ticket, FlushFault::None),
        Err(Refusal::OldAttachment)
    );
    assert_eq!(model.run_settled(old), Err(Refusal::OldAttachment));
    let current_ticket = model.begin_flush(current).unwrap();
    storage.borrow_mut().crash_owner();
    let replacement = Coordinator::claim(storage.clone(), 3).unwrap();
    assert_eq!(
        model.complete_flush(current_ticket, FlushFault::None),
        Err(Refusal::OldOwner)
    );
    assert_eq!(model.run_settled(current), Err(Refusal::OldOwner));
    assert_eq!(replacement.current, Coordinate::new(1, 1));
    assert_eq!(storage.borrow().replacements, 0);
}

#[test]
fn lifetime_lease_requires_verified_drain_for_clean_owner_exit() {
    let storage = FakeStorage::new();
    let mut model = Coordinator::claim(storage.clone(), 3).unwrap();
    assert!(matches!(
        Coordinator::claim(storage.clone(), 3),
        Err(Refusal::WriterBusy)
    ));
    let [gui, cli, _] = clients(&mut model);
    model.submit(gui, request(&model, 1, 3)).unwrap();
    model.request_exit().unwrap();
    assert_eq!(
        model.submit(cli, request(&model, 2, 4)),
        Err(Refusal::Exiting)
    );
    assert_eq!(model.finish_exit(), Err(Refusal::Pending));
    model.process_next(JournalFault::None).unwrap();
    assert_eq!(model.finish_exit(), Err(Refusal::Unsaved));
    let ticket = model.begin_flush(gui).unwrap();
    assert_eq!(
        model.complete_flush(ticket, FlushFault::BeforeReplace),
        Err(Refusal::FlushFailed)
    );
    assert_eq!(model.finish_exit(), Err(Refusal::Unsaved));
    assert!(matches!(
        Coordinator::claim(storage.clone(), 3),
        Err(Refusal::WriterBusy)
    ));
    model.recover().unwrap();
    model.complete_flush(ticket, FlushFault::None).unwrap();
    model.finish_exit().unwrap();
    let replacement = Coordinator::claim(storage, 3).unwrap();
    assert_eq!(replacement.status(), Status::Saved);
    assert_eq!(model.thaw(gui), Err(Refusal::OldOwner));
}

#[test]
fn forced_owner_exit_replays_verified_journal_and_keeps_original_operation() {
    let storage = FakeStorage::new();
    let mut model = Coordinator::claim(storage.clone(), 3).unwrap();
    let gui = model.attach(1, Surface::Gui, 1).unwrap();
    let first = accept(&mut model, gui, 91, 7);
    let unknown = request(&model, 92, 8);
    model.submit(gui, unknown.clone()).unwrap();
    assert_eq!(
        model.process_next(JournalFault::AfterBarrier),
        Err(Refusal::OutcomeUnknown)
    );
    storage.borrow_mut().crash_owner();
    let mut replacement = Coordinator::claim(storage.clone(), 3).unwrap();
    let agent = replacement.attach(1, Surface::Agent, 1).unwrap();
    assert_eq!(replacement.status(), Status::Saving);
    assert_eq!(replacement.current, Coordinate::new(2, 8));
    assert_eq!(
        replacement.submit(agent, first.request.clone()),
        Ok(Submission::Existing(first))
    );
    assert!(matches!(
        replacement.submit(agent, unknown),
        Ok(Submission::Existing(_))
    ));
    let ticket = replacement.begin_flush(agent).unwrap();
    replacement
        .complete_flush(ticket, FlushFault::None)
        .unwrap();
    assert_eq!(storage.borrow().journal.len(), 2);
    assert_eq!(replacement.status(), Status::Saved);
}

#[test]
fn unrelated_head_rollback_and_identity_change_refuse_recovery_without_writes() {
    for replacement in [Coordinate::new(1, 999), Coordinate::new(0, 0)] {
        let storage = FakeStorage::new();
        let mut model = Coordinator::claim(storage.clone(), 2).unwrap();
        let gui = model.attach(1, Surface::Gui, 1).unwrap();
        accept(&mut model, gui, 1, 20);
        let ticket = model.begin_flush(gui).unwrap();
        model.complete_flush(ticket, FlushFault::None).unwrap();
        storage.borrow_mut().head = replacement;
        assert_eq!(model.recover(), Err(Refusal::StorageConflict));
        assert_eq!(model.status(), Status::RecoveryRequired);
        assert_eq!(
            model.complete_flush(ticket, FlushFault::None),
            Err(Refusal::StorageConflict)
        );
        assert_eq!(storage.borrow().head, replacement);
        assert_eq!(storage.borrow().replacements, 1);
    }
    let storage = FakeStorage::new();
    let mut model = Coordinator::claim(storage.clone(), 2).unwrap();
    storage.borrow_mut().incarnation = 42;
    assert_eq!(model.recover(), Err(Refusal::StorageConflict));
}

#[test]
fn corrupt_or_gapped_journal_never_replays_past_verified_prefix() {
    for corruption in 0..3 {
        let storage = FakeStorage::new();
        let mut model = Coordinator::claim(storage.clone(), 3).unwrap();
        let gui = model.attach(1, Surface::Gui, 1).unwrap();
        accept(&mut model, gui, 1, 10);
        accept(&mut model, gui, 2, 20);
        {
            let mut state = storage.borrow_mut();
            match corruption {
                0 => state.journal[1].request_digest[0] ^= 1,
                1 => {
                    state.journal.remove(0);
                }
                _ => state.journal[1].after = Coordinate::new(2, 999),
            }
            state.crash_owner();
        }
        let mut replacement = Coordinator::claim(storage.clone(), 3).unwrap();
        assert_eq!(replacement.status(), Status::RecoveryRequired);
        assert_eq!(replacement.recover(), Err(Refusal::CorruptJournal));
        assert_eq!(replacement.current, Coordinate::new(0, 0));
        assert_eq!(storage.borrow().head, Coordinate::new(0, 0));
        assert!(matches!(
            Coordinator::claim(storage, 3),
            Err(Refusal::WriterBusy)
        ));
    }
}
