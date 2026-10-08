import Foundation
import AppKit

/// Synthetic presentation boundary only: these tests mint no native authority.
private actor ScriptedBridge: DisposableSessionBridge {
    let binding = DisposableAutosaveBinding(projectID: UUID(), incarnationID: UUID(), ownerEpoch: UUID(),
        attachmentID: UUID(), attachmentGeneration: 1, principalSHA256: "test")
    var calls: [DisposableHostRequest] = []
    var sequence: UInt64 = 0
    var accepted = DisposableAcceptedCoordinate(revision: 1, authoredDigest: "old", coordinateSHA256: "old",
        mutation: nil, acceptedFrameSHA256: nil, operationID: nil)
    var blockSubmit: CheckedContinuation<Void, Never>?
    var failComplete = true

    func execute(_ request: DisposableHostRequest) async throws -> DisposableHostResponse {
        if request.command != "snapshot" {
            precondition(request.expectedOwnerEpoch == binding.ownerEpoch)
            precondition(request.expectedAttachmentGeneration == binding.attachmentGeneration)
        }
        calls.append(request)
        if request.command == "submit" { await withCheckedContinuation { blockSubmit = $0 } }
        sequence += 1
        var acknowledgement: DisposableOperationAcknowledgement?
        if request.command == "submit" {
            accepted = .init(revision: 2, authoredDigest: "new", coordinateSHA256: "new",
                mutation: .init(recordID: request.id, checksum: "mutation"), acceptedFrameSHA256: "frame", operationID: request.id)
            acknowledgement = .init(binding: binding, operationID: request.id, accepted: accepted, operationReceiptSHA256: "receipt")
        }
        let failure = request.command == "complete" && failComplete
        let saved: DisposableSavedEvidence? = ["snapshot", "complete", "close", "barrier", "revalidate"].contains(request.command) && !failure
            ? .init(target: accepted, headSHA256: "head", commitSHA256: "commit", checkpointReceipt: .init(recordID: UUID(), checksum: "checkpoint")) : nil
        return .init(id: request.id, snapshot: .init(binding: binding, eventSequence: sequence,
            accepted: accepted, saved: saved, frozen: failure, closed: request.command == "close", failure: failure ? "flush refused" : nil),
            result: .init(nodes: [.init(id: "node", title: "Node", x: accepted.revision == 1 ? 0 : 16, y: 0)],
                          undo_available: true, redo_available: false), acknowledgement: acknowledgement,
            error: failure ? "flush refused" : nil)
    }
    func releaseSubmit() { blockSubmit?.resume(); blockSubmit = nil }
    func allowComplete() { failComplete = false }
    func commands() -> [String] { calls.map(\.command) }
    func completeIDs() -> [UUID] { calls.filter { $0.command == "complete" }.map(\.id) }
}

private actor ScriptedWorkspace: DisposableWorkspaceBridge {
    let source = ScriptedBridge()
    let first = UUID(), second = UUID()
    let startEmpty: Bool
    init(startEmpty: Bool = false) { self.startEmpty = startEmpty }
    var active: UUID?
    var titles: [DisposableProjectChoice] = []
    var current: DisposableHostResponse?
    var prepareIDs: [UUID] = []
    var confirmation: CheckedContinuation<Void, Never>?
    var outcome = "Activated"
    var reacquired = false
    var diagnostic: String?

    func workspace() async throws -> DisposableActivationResponse {
        current = try await source.execute(.init(id: UUID(), command: "snapshot"))
        active = current!.snapshot!.binding.projectID
        titles = [.init(id: active!, title: "Current"), .init(id: second, title: "Target")]
        if startEmpty { active = nil }
        return result("Current", nil)
    }
    func execute(_ request: DisposableHostRequest) async throws -> DisposableHostResponse {
        current = try await source.execute(request)
        return current!
    }
    func prepare(activationID: UUID, target: UUID, view: DisposableSessionView?, binding: DisposableAutosaveBinding?) async throws -> DisposableActivationResponse {
        precondition(target == second && (active == nil ? binding == nil : binding == current?.snapshot?.binding))
        prepareIDs.append(activationID)
        if active == nil { return result("Prepared", activationID) }
        current = try await source.execute(.init(id: UUID(), command: "complete",
            expectedOwnerEpoch: binding?.ownerEpoch, expectedAttachmentGeneration: binding?.attachmentGeneration))
        if current?.error != nil { return result("RetainedCurrent", activationID) }
        return result("Prepared", activationID)
    }
    func respond(activationID: UUID, command: String) async throws -> DisposableActivationResponse {
        precondition(prepareIDs.contains(activationID))
        if command == "cancel" { return result("RetainedCurrent", activationID) }
        await withCheckedContinuation { confirmation = $0 }
        if outcome == "Activated" {
            let old = current!.snapshot!
            let binding = DisposableAutosaveBinding(projectID: second, incarnationID: UUID(), ownerEpoch: UUID(),
                attachmentID: UUID(), attachmentGeneration: 1, principalSHA256: "test")
            current = .init(id: UUID(), snapshot: .init(binding: binding, eventSequence: 1,
                accepted: old.accepted, saved: old.saved, frozen: false, closed: false, failure: nil),
                result: .init(nodes: [.init(id: "target-node", title: "Target Node", x: 40, y: 0)],
                undo_available: false, redo_available: false), acknowledgement: nil, error: nil)
            active = second
        }
        return result(outcome, activationID)
    }
    func result(_ status: String, _ id: UUID?) -> DisposableActivationResponse {
        var state = DisposableWorkspaceState(current: (status == "RetainedReadOnlyRecovery" && !reacquired) || active == nil ? nil : current.map { .init(snapshot: $0.snapshot, result: $0.result, error: $0.error) }, projects: titles, activeProjectID: active,
            view: nil, confirmationRequired: status == "Prepared" && active != nil, readOnly: outcome == "RetainedReadOnlyRecovery" && !reacquired)
        state.error = diagnostic ?? current?.error
        return .init(status: status, activationID: id, state: state)
    }
    func setDiagnostic(_ value: String?) { diagnostic = value }
    func clearReacquireFault() { reacquired = true; diagnostic = nil }
    func chooseOutcome(_ value: String) { outcome = value }
    func permitSave() async { await source.allowComplete() }
    func releaseSubmit() async {
        for _ in 0..<500 {
            if await source.commands().contains("submit") { await source.releaseSubmit(); return }
            try? await Task.sleep(for: .milliseconds(10))
        }
        preconditionFailure("submit was not delivered")
    }
    func releaseConfirmation() async {
        for _ in 0..<500 {
            if let confirmation { self.confirmation = nil; confirmation.resume(); return }
            try? await Task.sleep(for: .milliseconds(10))
        }
        preconditionFailure("confirmation was not delivered")
    }
    func preparedCount() -> Int { prepareIDs.count }
}

@main
struct ControlledSessionChecks {
    @MainActor static func wait(_ condition: @MainActor () -> Bool) async {
        for _ in 0..<500 {
            if condition() { return }
            try? await Task.sleep(for: .milliseconds(10))
        }
        preconditionFailure("Timed out waiting for bounded presentation test")
    }

    @MainActor static func main() async throws {
        _ = NSApplication.shared
        let alert = ControlledSwitchConfirmationPresenter.makeAlert(.init(id: UUID(), sourceTitle: "A", targetTitle: "B"))
        precondition(alert.messageText == "Switch to “B”?")
        precondition(alert.informativeText == "Photara will close “A” and open “B.” Changes to “A” have been saved automatically.")
        precondition(alert.buttons.map(\.title) == ["Cancel", "Switch Project"])
        precondition(alert.buttons[0].keyEquivalent == "\u{1b}" && alert.buttons[1].keyEquivalent == "\r")
        precondition(alert.window.defaultButtonCell === alert.buttons[1].cell)
        let manifest = "/private/tmp/photara-ps2-remount-test/manifest.json"
        for env in [[:], ["PHOTARA_PS2_REMOUNT_MANIFEST": manifest],
                    ["PHOTARA_PS2_REMOUNT_MANIFEST": "/real-library/manifest.json", "PHOTARA_PS2_REMOUNT_BINDING": "{}"],
                    ["PHOTARA_PS2_REMOUNT_MANIFEST": manifest, "PHOTARA_PS2_REMOUNT_BINDING": "[]"],
                    ["PHOTARA_PS2_REMOUNT_MANIFEST": manifest, "PHOTARA_PS2_REMOUNT_BINDING": String(repeating: "x", count: 65_537)]] {
            precondition((try? ControlledDisposableConfiguration.load(env)) == nil)
        }
        for targets in ["", "[]", "invalid", String(repeating: "x", count: 65_537)] {
            precondition((try? ControlledDisposableConfiguration.load(["PHOTARA_PS2_REMOUNT_MANIFEST": manifest,
                "PHOTARA_PS2_REMOUNT_BINDING": "{}", "PHOTARA_PS4_TARGET_BINDINGS": targets])) == nil)
        }
        // Syntactically valid controller input remains subject to Rust authority checks.
        _ = try ControlledDisposableConfiguration.load(["PHOTARA_PS2_REMOUNT_MANIFEST": manifest, "PHOTARA_PS2_REMOUNT_BINDING": "{}"])
        let bridge = ScriptedBridge()
        let model = DisposableAutosaveProcess(bridge: bridge)
        model.start(environment: [:])
        await wait { model.canEdit }
        model.move(model.nodes[0], dx: 16, dy: 0)
        precondition(model.pendingPosition?.x == 16 && !model.canEdit)
        var closeResult: Bool?
        model.close { closeResult = $0 }
        precondition(closeResult == nil) // Close waits for acknowledgement and checkpoint.
        while await bridge.commands().count < 2 { try await Task.sleep(for: .milliseconds(10)) }
        await bridge.releaseSubmit()
        await wait { closeResult != nil }
        precondition(closeResult == false && !model.closed && model.nodes[0].x == 16)
        precondition(model.pendingPosition == nil && model.projection?.status != .saved)
        let commands = await bridge.commands()
        precondition(commands == ["snapshot", "submit", "complete"])
        await bridge.allowComplete()
        model.retry()
        await wait { model.projection?.status == .saved && !model.busy }
        let ids = await bridge.completeIDs()
        precondition(ids.count == 2 && ids[0] == ids[1])
        model.revalidateAfterWake()
        precondition(!model.canEdit && model.checkingStorage)
        await wait { model.canEdit }
        model.prepareForSleep()
        await wait { !model.busy }
        let lifecycle = await bridge.commands()
        precondition(lifecycle.suffix(2) == ["revalidate", "barrier"])
        model.close { closeResult = $0 }
        await wait { model.closed }
        precondition(closeResult == true)
        try await switchingChecks()
        print("PASS: controlled configuration, visible pending move, close queues, failed flush retains graph, exact retry, wake revalidation, sleep barrier, verified close")
    }
    @MainActor static func switchingChecks() async throws {
        let workspace = ScriptedWorkspace()
        // Each model owns one typed coordinator; no mock state grants native authority.
        let actual = DisposableAutosaveProcess(bridge: workspace)
        actual.start()
        await wait { actual.canEdit }
        let target = actual.projects[1].id
        actual.beginSwitch(to: actual.activeProjectID!, view: nil)
        let initiallyPrepared = await workspace.preparedCount()
        precondition(!actual.switching && initiallyPrepared == 0)
        actual.move(actual.nodes[0], dx: 16, dy: 0)
        actual.beginSwitch(to: target, view: nil)
        precondition(actual.switching && actual.switchPrompt == nil && !actual.canEdit)
        await workspace.releaseSubmit()
        await wait { actual.transportFailure != nil }
        precondition(actual.switchPrompt == nil && !actual.switching)
        let afterFailurePrepared = await workspace.preparedCount()
        precondition(afterFailurePrepared == 0) // Failed pre-save never shows past tense.
        await workspace.permitSave()
        actual.retry()
        await wait { actual.canEdit }
        actual.beginSwitch(to: target, view: nil)
        await wait { actual.switchPrompt != nil }
        let first = actual.switchPrompt!.id
        actual.answerSwitch(UUID(), confirmed: true) // Stale callback cannot activate.
        precondition(actual.switchPrompt?.id == first)
        var closed: Bool?
        actual.close { closed = $0 }
        precondition(closed == false && !actual.closed)
        actual.answerSwitch(first, confirmed: false)
        await wait { !actual.switching }
        precondition(actual.projectTitle == "Current" && actual.nodes[0].title == "Node")
        actual.beginSwitch(to: target, view: nil)
        await wait { actual.switchPrompt != nil }
        let second = actual.switchPrompt!.id
        actual.answerSwitch(second, confirmed: true)
        actual.answerSwitch(second, confirmed: true) // Duplicate press is ignored.
        precondition(actual.projectTitle == "Current" && !actual.canEdit)
        for _ in 0..<10 { await Task.yield() }
        await workspace.releaseConfirmation()
        await wait { !actual.switching }
        precondition(actual.projectTitle == "Target" && actual.nodes[0].title == "Target Node")
        let empty = ScriptedWorkspace(startEmpty: true)
        let opening = DisposableAutosaveProcess(bridge: empty)
        opening.start()
        await wait { opening.projects.count == 2 && !opening.busy }
        precondition(opening.activeProjectID == nil && opening.projection == nil)
        opening.beginSwitch(to: opening.projects[1].id, view: nil)
        await empty.releaseConfirmation()
        await wait { !opening.switching }
        precondition(opening.switchPrompt == nil && opening.projectTitle == "Target")
        let refusedWorkspace = ScriptedWorkspace()
        let refused = DisposableAutosaveProcess(bridge: refusedWorkspace)
        refused.start()
        await wait { refused.canEdit }
        await refusedWorkspace.permitSave()
        refused.beginSwitch(to: refused.projects[1].id, view: nil)
        await wait { refused.switchPrompt != nil }
        await refusedWorkspace.chooseOutcome("RetainedCurrent")
        await refusedWorkspace.setDiagnostic("Resource temporarily unavailable (os error 35)")
        refused.answerSwitch(refused.switchPrompt!.id, confirmed: true)
        await refusedWorkspace.releaseConfirmation()
        await wait { !refused.switching }
        precondition(refused.canEdit && refused.switchFailure?.contains("previous Project is still open") == true)
        precondition(refused.switchFailure?.contains("os error") == false)
        let failing = ScriptedWorkspace()
        let recovery = DisposableAutosaveProcess(bridge: failing)
        recovery.start()
        await wait { recovery.canEdit }
        await failing.permitSave()
        recovery.beginSwitch(to: recovery.projects[1].id, view: nil)
        await wait { recovery.switchPrompt != nil }
        let recoveryAttempt = recovery.switchPrompt!.id
        await failing.chooseOutcome("RetainedReadOnlyRecovery")
        await failing.setDiagnostic("Resource temporarily unavailable (os error 35)")
        recovery.answerSwitch(recoveryAttempt, confirmed: true)
        await failing.releaseConfirmation()
        await wait { !recovery.switching }
        precondition(recovery.readOnly && !recovery.canEdit && recovery.projectTitle == "Current")
        precondition(recovery.switchFailure?.contains("read-only") == true && recovery.switchFailure?.contains("os error") == false)
        precondition(recovery.nodes.first?.title == "Node")
        var recoveryClose: Bool?
        recovery.close { recoveryClose = $0 }
        precondition(recoveryClose == false && !recovery.closed && recovery.nodes.first?.title == "Node")
        await failing.clearReacquireFault()
        recovery.retry()
        await failing.releaseConfirmation()
        await wait { !recovery.switching && !recovery.readOnly }
        precondition(recovery.canEdit && recovery.projectTitle == "Current")
        recovery.close { recoveryClose = $0 }
        await wait { recovery.closed }
        precondition(recoveryClose == true)
        print("PASS: failed reacquisition retains graph read-only, Quit cancels, original Retry reacquires then verified Close succeeds")
        print("PASS: pending input drains before confirmation, failed pre-save suppresses dialog, current activation is idempotent, stale/double callbacks refused, cancel retains graph, title changes only after Activated")
    }

}
