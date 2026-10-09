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
    var positionX: Int64 = 0, positionY: Int64 = 0
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
            positionX = request.x!; positionY = request.y!
            accepted = .init(revision: accepted.revision + 1, authoredDigest: "new", coordinateSHA256: "new",
                mutation: .init(recordID: request.id, checksum: "mutation"), acceptedFrameSHA256: "frame", operationID: request.id)
            acknowledgement = .init(binding: binding, operationID: request.id, accepted: accepted, operationReceiptSHA256: "receipt")
        }
        let failure = request.command == "complete" && failComplete
        let saved: DisposableSavedEvidence? = ["snapshot", "complete", "close", "barrier", "revalidate"].contains(request.command) && !failure
            ? .init(target: accepted, headSHA256: "head", commitSHA256: "commit", checkpointReceipt: .init(recordID: UUID(), checksum: "checkpoint")) : nil
        return .init(id: request.id, snapshot: .init(binding: binding, eventSequence: sequence,
            accepted: accepted, saved: saved, frozen: failure, closed: request.command == "close", failure: failure ? "flush refused" : nil),
            result: .init(nodes: [.init(id: "node", title: "Node", x: positionX, y: positionY)],
                          undo_available: true, redo_available: false), acknowledgement: acknowledgement,
            error: failure ? "flush refused" : nil)
    }
    func releaseSubmit() async {
        for _ in 0..<500 {
            if let blockSubmit { self.blockSubmit = nil; blockSubmit.resume(); return }
            try? await Task.sleep(for: .milliseconds(10))
        }
        preconditionFailure("submit was not waiting")
    }
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

private actor ScriptedLibraries: DisposableLibraryBridge {
    let initiallyEmpty: Bool
    init(initiallyEmpty: Bool = false) { self.initiallyEmpty = initiallyEmpty }
    let source = ScriptedBridge()
    let first = UUID(), second = UUID()
    var libraries: [DisposableLibraryChoice] = []
    var selected: UUID?
    var current: DisposableHostResponse?
    var selection: CheckedContinuation<Void, Never>?
    var selections: [UUID] = []
    var requests: [DisposableLibraryRequest] = []
    var failLibrary = true
    func workspace() async throws -> DisposableActivationResponse {
        await source.allowComplete()
        current = try await source.execute(.init(id: UUID(), command: "snapshot"))
        libraries = [.init(id: first, name: "Library A", revision: 1)]; selected = first
        if initiallyEmpty { selected = nil; current = nil }
        return response("Current", nil)
    }
    func response(_ status: String, _ id: UUID?) -> DisposableActivationResponse {
        .init(status: status, activationID: id, state: .init(
            current: current.map { .init(snapshot: $0.snapshot, result: $0.result, error: $0.error) },
            projects: current.map { [.init(id: $0.snapshot!.binding.projectID, title: "Project A")] } ?? [],
            activeProjectID: current?.snapshot?.binding.projectID, view: nil,
            confirmationRequired: false, readOnly: false, libraries: libraries,
            selectedLibraryID: selected, creatableLibraryIDs: libraries.count == 1 ? [second] : [], localLibraryContext: true))
    }
    func execute(_ request: DisposableHostRequest) async throws -> DisposableHostResponse {
        if current == nil {
            precondition(request.expectedOwnerEpoch == nil && request.expectedAttachmentGeneration == nil)
            return .init(id: request.id, snapshot: nil,
                result: .init(nodes: nil, undo_available: nil, redo_available: nil, context_closed: request.command == "close", context_verified: request.command != "close"),
                acknowledgement: nil, error: nil)
        }
        current = try await source.execute(request); return current!
    }
    func selectLibrary(activationID: UUID, libraryID: UUID, view: DisposableSessionView?, binding: DisposableAutosaveBinding?) async throws -> DisposableActivationResponse {
        precondition(libraryID == second && binding == current?.snapshot?.binding)
        selections.append(activationID)
        await withCheckedContinuation { selection = $0 }
        selected = second; current = nil
        return response("Activated", activationID)
    }
    func closeProject(activationID: UUID, view: DisposableSessionView?, binding: DisposableAutosaveBinding?) async throws -> DisposableActivationResponse {
        precondition(current != nil && binding == current?.snapshot?.binding)
        current = nil
        return response("Activated", activationID)
    }
    func libraryCommand(_ request: DisposableLibraryRequest) async throws -> DisposableLibraryResponse {
        requests.append(request)
        if failLibrary { throw CocoaError(.fileWriteUnknown) }
        if let revision = request.expectedRevision {
            precondition(libraries.first { $0.id == request.libraryID }?.revision == revision)
            libraries = libraries.map { $0.id == request.libraryID ? .init(id: $0.id, name: request.name, revision: revision + 1) : $0 }
        } else {
            precondition(request.libraryID == second)
            libraries.append(.init(id: second, name: request.name, revision: 1))
        }
        return .init(receipt: .init(operationID: request.operationID, libraryID: request.libraryID,
            action: request.expectedRevision == nil ? "create" : "rename"), state: response("Current", nil).state)
    }
    func prepare(activationID: UUID, target: UUID, view: DisposableSessionView?, binding: DisposableAutosaveBinding?) throws -> DisposableActivationResponse { throw CocoaError(.featureUnsupported) }
    func respond(activationID: UUID, command: String) throws -> DisposableActivationResponse { throw CocoaError(.featureUnsupported) }
    func allowLibrary() { failLibrary = false }
    func originalRequests() -> [DisposableLibraryRequest] { requests }
    func selectionCount() -> Int { selections.count }
    func releaseSelection() async {
        while selection == nil { await Task.yield() }
        selection?.resume(); selection = nil
    }
    func releaseSubmit() async {
        while await source.commands().filter({ $0 == "submit" }).isEmpty { await Task.yield() }
        await source.releaseSubmit()
    }
}

private actor QueuedMoveBridge: DisposableSessionBridge {
    enum Reply { case good, fail, stale }
    let binding = DisposableAutosaveBinding(projectID: UUID(), incarnationID: UUID(), ownerEpoch: UUID(),
        attachmentID: UUID(), attachmentGeneration: 1, principalSHA256: "test")
    let title: String
    init(title: String = "Node") { self.title = title }
    var sequence: UInt64 = 0
    var accepted = DisposableAcceptedCoordinate(revision: 1, authoredDigest: "old", coordinateSHA256: "old",
        mutation: nil, acceptedFrameSHA256: nil, operationID: nil)
    var x: Int64 = 0, y: Int64 = 0
    var calls: [DisposableHostRequest] = []
    var waiting: CheckedContinuation<Reply, Never>?
    var waitingCommand: String?
    var originals: [UUID: DisposableHostRequest] = [:]
    var receipts: [UUID: DisposableAcceptedCoordinate] = [:]
    func execute(_ request: DisposableHostRequest) async throws -> DisposableHostResponse {
        calls.append(request)
        var reply = Reply.good
        if ["submit", "complete"].contains(request.command) {
            waitingCommand = request.command
            reply = await withCheckedContinuation { waiting = $0 }
            waitingCommand = nil
        }
        sequence += 1
        var acknowledgement: DisposableOperationAcknowledgement?
        if request.command == "submit", reply == .good {
            if let old = originals[request.id] {
                precondition(old.node_id == request.node_id && old.x == request.x && old.y == request.y)
            } else {
                originals[request.id] = request; x = request.x!; y = request.y!
                accepted = .init(revision: accepted.revision + 1, authoredDigest: "a\(x)", coordinateSHA256: "c\(x)",
                    mutation: .init(recordID: request.id, checksum: "mutation"), acceptedFrameSHA256: "frame", operationID: request.id)
                receipts[request.id] = accepted
            }
            acknowledgement = .init(binding: binding, operationID: request.id, accepted: receipts[request.id]!, operationReceiptSHA256: "receipt")
        }
        let saved: DisposableSavedEvidence? = reply == .good && request.command != "submit"
            ? .init(target: accepted, headSHA256: "head", commitSHA256: "commit", checkpointReceipt: .init(recordID: UUID(), checksum: "checkpoint")) : nil
        return .init(id: request.id, snapshot: .init(binding: binding, eventSequence: reply == .stale ? 0 : sequence,
            accepted: accepted, saved: saved, frozen: reply == .fail, closed: request.command == "close",
            failure: reply == .fail ? "flush refused" : nil),
            result: .init(nodes: [.init(id: "node", title: title, x: x, y: y)], undo_available: true, redo_available: false),
            acknowledgement: acknowledgement, error: reply == .fail ? "flush refused" : nil)
    }
    func release(_ command: String, _ reply: Reply = .good) async {
        while waiting == nil { await Task.yield() }
        precondition(waitingCommand == command)
        let continuation = waiting; waiting = nil; continuation?.resume(returning: reply)
    }
    func waitingFor(_ command: String) -> Bool { waitingCommand == command && waiting != nil }
    func requests() -> [DisposableHostRequest] { calls }
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
        try await libraryChecks()
        try await queuedMoveChecks()
        try await AutosaveInteractionChecks.run()
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

    @MainActor static func libraryChecks() async throws {
        let named = DisposableLibraryChoice(id: UUID(), name: "My Library", revision: 7)
        let rename = LocalLibraryNameRequest(mode: .rename(named))
        precondition(rename.title == "Rename Library" && rename.buttonTitle == "Rename" && rename.initialName == "My Library")
        if case let .rename(target) = rename.mode { precondition(target == named) } else { preconditionFailure("Wrong sheet action") }
        let reserved = UUID(), create = LocalLibraryNameRequest(mode: .create(UUID()))
        precondition(create.title == "Create Library" && create.buttonTitle == "Create" && create.initialName.isEmpty)
        let originalCreate = LocalLibraryNameRequest(mode: .create(reserved))
        if case let .create(id) = originalCreate.mode { precondition(id == reserved) } else { preconditionFailure("Wrong sheet action") }
        precondition(rename.id != create.id)
        print("PASS: sheet item carries exact immutable Create or Rename action, original Library/revision, title, button and initial name")
        let bridge = ScriptedLibraries()
        // A single coordinator supplies both Library receipts and Project state.
        let actual = DisposableAutosaveProcess(bridge: bridge)
        actual.start(); await wait { actual.canEdit }
        let first = actual.selectedLibraryID!, originalProject = actual.activeProjectID
        actual.selectLibrary(first, view: nil)
        precondition(!actual.switching)
        let count = await bridge.selectionCount(); precondition(count == 0)
        actual.createLibrary(name: "Library B")
        await wait { actual.libraryFailure != nil && !actual.libraryWorking }
        let original = await bridge.originalRequests()
        precondition(original.count == 1 && original[0].libraryID == actual.creatableLibraryIDs.first)
        var close: Bool?
        actual.close { close = $0 }; precondition(close == false && !actual.closed)
        await bridge.allowLibrary(); actual.retry()
        await wait { actual.libraries.count == 2 && !actual.libraryWorking }
        let repeated = await bridge.originalRequests(); precondition(repeated.count == 2 && repeated[0] == repeated[1])
        precondition(actual.selectedLibraryID == first && actual.activeProjectID == originalProject)
        actual.renameLibrary(actual.libraries[1], name: "Renamed B")
        await wait { actual.libraries[1].revision == 2 }
        let renamed = await bridge.originalRequests()
        precondition(renamed.last?.expectedRevision == 1 && actual.libraries[1].name == "Renamed B")
        actual.move(actual.nodes[0], dx: 16, dy: 0)
        actual.selectLibrary(actual.libraries[1].id, view: nil)
        precondition(actual.switching && actual.switchPrompt == nil && actual.activeProjectID == originalProject)
        await bridge.releaseSubmit()
        while await bridge.selectionCount() == 0 { await Task.yield() }
        precondition(actual.selectedLibraryID == first && actual.activeProjectID == originalProject && !actual.nodes.isEmpty)
        await bridge.releaseSelection(); await wait { !actual.switching }
        precondition(actual.selectedLibraryID != first && actual.activeProjectID == nil && actual.nodes.isEmpty && actual.projection == nil)
        precondition(actual.switchPrompt == nil)
        actual.revalidateAfterWake(); await wait { !actual.checkingStorage }
        precondition(actual.projection == nil && actual.transportFailure == nil)
        actual.prepareForSleep(); await wait { !actual.busy }
        actual.close { close = $0 }; await wait { actual.closed }; precondition(close == true)
        let closing = DisposableAutosaveProcess(bridge: ScriptedLibraries())
        closing.start(); await wait { closing.canEdit }
        let originalLibrary = closing.selectedLibraryID
        closing.closeProject(view: nil); await wait { !closing.switching }
        precondition(closing.selectedLibraryID == originalLibrary && closing.activeProjectID == nil && !closing.closed)
        precondition(closing.nodes.isEmpty && closing.switchPrompt == nil)
        let empty = DisposableAutosaveProcess(bridge: ScriptedLibraries(initiallyEmpty: true))
        empty.start(); await wait { empty.localLibraryContext && !empty.busy }
        precondition(empty.selectedLibraryID == nil && empty.projection == nil)
        var emptyClosed: Bool?
        empty.close { emptyClosed = $0 }
        precondition(emptyClosed == nil) // Empty SQL slot still revalidates in shared authority.
        await wait { empty.closed }; precondition(emptyClosed == true)
        print("PASS: Close Project publishes same-Library browser without Quit or a generic dialog; empty SQL slot verifies Close without fake Saved")
        print("PASS: Library original create/rename retry and revision CAS, current-Library no-op, pending input drain, no-sheet Library selection, Project clears only after activation, verified Library-only lifecycle")
    }

    @MainActor static func queuedMoveChecks() async throws {
        let bridge = QueuedMoveBridge()
        let actual = DisposableAutosaveProcess(bridge: bridge)
        actual.start(); await wait { actual.canMove }
        let node = actual.nodes[0]
        actual.move(node, x: 10, y: 1)
        let firstID = actual.pendingPosition!.operationID
        actual.move(node, x: 20, y: 2); actual.move(node, x: 30, y: 3)
        let secondID = actual.queuedPositions[0].operationID
        precondition(actual.visibleNodes[0].x == 30 && actual.nodes[0].x == 0 && actual.visualStatus == .saving)
        precondition(actual.canMove && !actual.canEdit)
        // Real model remains on MainActor; held backend acknowledgement must not
        // block scheduled UI work or admission of the next visible draft.
        var beats = 0
        let heartbeat = Task { @MainActor in
            for _ in 0..<8 { try await Task.sleep(for: .milliseconds(5)); beats += 1 }
        }
        try await heartbeat.value
        precondition(beats == 8 && actual.pendingPosition?.operationID == firstID)
        precondition(actual.visibleNodes[0].x == 30 && actual.visualStatus == .saving)
        await bridge.release("submit")
        while !(await bridge.waitingFor("complete")) { await Task.yield() }
        precondition(actual.nodes[0].x == 10 && actual.visibleNodes[0].x == 30 && actual.canMove)
        actual.move(node, x: 40, y: 4)
        precondition(actual.visibleNodes[0].x == 40)
        await bridge.release("complete", .fail)
        await wait { actual.transportFailure != nil }
        precondition(!actual.canMove && actual.visibleNodes[0].x == 40 && actual.queuedPositions.count == 3)
        var closed: Bool?
        actual.close { closed = $0 }; precondition(closed == false && !actual.closed)
        actual.retry(); await bridge.release("complete")
        while !(await bridge.waitingFor("submit")) { await Task.yield() }
        precondition(actual.pendingPosition?.operationID == secondID && actual.visibleNodes[0].x == 40)
        // A matching request carrying stale snapshot evidence cannot erase drafts.
        await bridge.release("submit", .stale)
        await wait { actual.transportFailure != nil }
        precondition(actual.pendingPosition?.operationID == secondID && actual.visibleNodes[0].x == 40)
        actual.retry()
        actual.close { closed = $0 } // Drains every already queued original before closing.
        await bridge.release("submit"); await bridge.release("complete")
        await bridge.release("submit"); await bridge.release("complete")
        await bridge.release("submit"); await bridge.release("complete")
        await wait { actual.closed }
        precondition(closed == true && actual.nodes[0].x == 40 && actual.visibleNodes[0].x == 40)
        precondition(actual.queuedPositions.isEmpty && actual.pendingPosition == nil && actual.visualStatus == .saved)
        let calls = await bridge.requests()
        precondition(calls.map(\.command) == ["snapshot", "submit", "complete", "complete", "submit", "submit", "complete", "submit", "complete", "submit", "complete", "close"])
        precondition(calls[1].id == firstID && calls[2].id == calls[3].id && calls[4].id == secondID && calls[4].id == calls[5].id)
        let switchingBridge = ScriptedWorkspace()
        let switching = DisposableAutosaveProcess(bridge: switchingBridge)
        switching.start(); await wait { switching.canMove }
        await switchingBridge.permitSave()
        switching.move(switching.nodes[0], x: 16, y: 0)
        switching.move(switching.nodes[0], x: 32, y: 0)
        switching.beginSwitch(to: switching.projects[1].id, view: nil)
        precondition(switching.switchPrompt == nil && switching.visibleNodes[0].x == 32)
        let beforeDrain = await switchingBridge.preparedCount(); precondition(beforeDrain == 0)
        await switchingBridge.releaseSubmit(); await switchingBridge.releaseSubmit()
        await wait { switching.switchPrompt != nil }
        precondition(switching.nodes[0].x == 32 && switching.pendingPosition == nil && switching.queuedPositions.isEmpty)
        precondition(switching.visualStatus == .saved)
        switching.answerSwitch(switching.switchPrompt!.id, confirmed: false)
        await wait { !switching.switching }
        precondition(switching.nodes[0].x == 32)
        let boundBridge = QueuedMoveBridge(title: String(repeating: "x", count: 530_000))
        let bound = DisposableAutosaveProcess(bridge: boundBridge)
        bound.start(); await wait { bound.canMove }
        bound.move(bound.nodes[0], x: 1, y: 0)
        let admitted = bound.pendingPosition!.operationID
        bound.move(bound.nodes[0], x: 2, y: 0)
        precondition(bound.moveRefusal != nil && bound.pendingPosition?.operationID == admitted)
        precondition(bound.queuedPositions.isEmpty && bound.visibleNodes[0].x == 1)
        await boundBridge.release("submit"); await boundBridge.release("complete")
        await wait { bound.canEdit }
        print("PASS: immediate overlay survives delayed/stale replies and failed checkpoint retry; subsequent drops remain ordered originals; Quit drains all; bounded rejection retains prior visible draft")
    }

}
