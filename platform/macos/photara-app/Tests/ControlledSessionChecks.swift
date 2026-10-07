import Foundation

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
        let manifest = "/private/tmp/photara-ps2-remount-test/manifest.json"
        for env in [[:], ["PHOTARA_PS2_REMOUNT_MANIFEST": manifest],
                    ["PHOTARA_PS2_REMOUNT_MANIFEST": "/real-library/manifest.json", "PHOTARA_PS2_REMOUNT_BINDING": "{}"],
                    ["PHOTARA_PS2_REMOUNT_MANIFEST": manifest, "PHOTARA_PS2_REMOUNT_BINDING": "[]"],
                    ["PHOTARA_PS2_REMOUNT_MANIFEST": manifest, "PHOTARA_PS2_REMOUNT_BINDING": String(repeating: "x", count: 65_537)]] {
            precondition((try? ControlledDisposableConfiguration.load(env)) == nil)
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
        print("PASS: controlled configuration, visible pending move, close queues, failed flush retains graph, exact retry, wake revalidation, sleep barrier, verified close")
    }
}
