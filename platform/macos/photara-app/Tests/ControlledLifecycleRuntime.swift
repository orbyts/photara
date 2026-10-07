import AppKit
import SwiftUI

private func marker(_ text: String) {
    FileHandle.standardOutput.write(Data((text + "\n").utf8))
}

/// No storage effects. This bridge only holds a presentation request across a
/// real NSApplication termination attempt, exercising the actual App/delegate.
private actor LifecycleBridge: DisposableSessionBridge {
    let binding = DisposableAutosaveBinding(projectID: UUID(), incarnationID: UUID(), ownerEpoch: UUID(),
        attachmentID: UUID(), attachmentGeneration: 1, principalSHA256: "test")
    var sequence: UInt64 = 0
    var accepted = DisposableAcceptedCoordinate(revision: 1, authoredDigest: "old", coordinateSHA256: "old",
        mutation: nil, acceptedFrameSHA256: nil, operationID: nil)
    var pending: CheckedContinuation<Void, Never>?
    var fail = CommandLine.arguments.contains("--fail-flush")
    func execute(_ request: DisposableHostRequest) async throws -> DisposableHostResponse {
        marker("EXECUTE_" + request.command)
        if request.command == "submit" { await withCheckedContinuation { pending = $0 } }
        sequence += 1
        var acknowledgement: DisposableOperationAcknowledgement?
        if request.command == "submit" {
            accepted = .init(revision: 2, authoredDigest: "new", coordinateSHA256: "new",
                mutation: .init(recordID: request.id, checksum: "mutation"), acceptedFrameSHA256: "frame", operationID: request.id)
            acknowledgement = .init(binding: binding, operationID: request.id, accepted: accepted, operationReceiptSHA256: "receipt")
        }
        let failed = request.command == "complete" && fail
        let saved: DisposableSavedEvidence? = request.command != "submit" && !failed
            ? .init(target: accepted, headSHA256: "head", commitSHA256: "commit", checkpointReceipt: .init(recordID: UUID(), checksum: "checkpoint")) : nil
        if request.command == "close" {
            precondition(accepted.revision == 2 && !fail)
            marker("CLOSE_VERIFIED_REVISION_2")
        }
        return .init(id: request.id, snapshot: .init(binding: binding, eventSequence: sequence,
            accepted: accepted, saved: saved, frozen: failed, closed: request.command == "close", failure: failed ? "flush refused" : nil),
            result: .init(nodes: [.init(id: "node", title: "Node", x: accepted.revision == 1 ? 0 : 16, y: 0)],
                          undo_available: true, redo_available: false), acknowledgement: acknowledgement,
            error: failed ? "flush refused" : nil)
    }
    func isBlocked() -> Bool { pending != nil }
    func release() { pending?.resume(); pending = nil }
    func recover() { fail = false }
}

@MainActor private let bridge = LifecycleBridge()
@MainActor func makeControlledDisposableModel() -> DisposableAutosaveProcess {
    marker("MODEL_CREATED")
    Task { @MainActor in
        NSApp.setActivationPolicy(.regular)
        NSApp.activate()
    }
    return .init(bridge: bridge)
}

// PhotaraMacApp.swift is compiled unchanged for this test, including its actual
// @NSApplicationDelegateAdaptor declaration. Only the editor/backend are replaced.
struct ControlledDisposableScene: Scene {
    let delegate: ControlledSessionApplicationDelegate
    @ObservedObject var model: DisposableAutosaveProcess
    var body: some Scene {
        Window("Controlled lifecycle verification", id: "lifecycle-test") {
            Text("Runtime lifecycle check").onAppear { Task { await run() } }
        }
    }
    @MainActor private func wait(_ condition: @MainActor () -> Bool) async {
        for _ in 0..<500 {
            if condition() { return }
            try? await Task.sleep(for: .milliseconds(10))
        }
        preconditionFailure("Runtime lifecycle check timed out")
    }
    @MainActor private func run() async {
        marker("RUNTIME_STARTED")
        delegate.model = model
        model.start()
        await wait { model.canEdit }
        marker("SNAPSHOT_READY")
        model.move(model.nodes[0], dx: 16, dy: 0)
        while !(await bridge.isBlocked()) { try? await Task.sleep(for: .milliseconds(10)) }
        marker("SUBMIT_BLOCKED")
        // terminate may remain in AppKit's deferred-termination run loop until
        // reply; arrange the simulated IO completion before entering that loop.
        Task { @MainActor in
            try? await Task.sleep(for: .milliseconds(100))
            precondition(delegate.isTerminationPending && model.busy && !model.closed)
            marker("ACTUAL_QUIT_DEFERRED_WHILE_SUBMIT_PENDING")
            await bridge.release()
            if CommandLine.arguments.contains("--fail-flush") {
                await wait { !delegate.isTerminationPending && model.transportFailure != nil }
                precondition(!model.closed && model.nodes[0].x == 16)
                marker("ACTUAL_QUIT_CANCELED_ON_FLUSH_FAILURE_GRAPH_RETAINED")
                await bridge.recover()
                model.retry()
                await wait { model.projection?.status == .saved && !model.busy }
                NSApp.perform(#selector(NSApplication.terminate(_:)), with: nil, afterDelay: 0)
            }
        }
        NSApp.perform(#selector(NSApplication.terminate(_:)), with: nil, afterDelay: 0)
    }
}
