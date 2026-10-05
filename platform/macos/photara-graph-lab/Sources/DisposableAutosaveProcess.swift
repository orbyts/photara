import Foundation
import Combine
import Darwin

enum DisposablePipeRead: Sendable {
    case bytes(Data)
    case end
    case deferred
    case failed(Int32)
}

/// Exactly one bounded read of currently available pipe bytes. Foundation's
/// read(upToCount:) can wait for additional bytes while a writer remains open.
func readDisposablePipe(_ descriptor: Int32) -> DisposablePipeRead {
    var buffer = Data(count: 65_536)
    let count = buffer.withUnsafeMutableBytes { memory in
        Darwin.read(descriptor, memory.baseAddress!, memory.count)
    }
    if count > 0 {
        buffer.count = count
        return .bytes(buffer)
    }
    if count == 0 { return .end }
    let failure = errno
    if failure == EINTR || failure == EAGAIN || failure == EWOULDBLOCK { return .deferred }
    return .failed(failure)
}

struct DisposablePositionNode: Decodable, Identifiable {
    let id: String
    let title: String
    let x: Int64
    let y: Int64
}
struct DisposablePendingPosition {
    let operationID: UUID
    let nodeID: String
    let title: String
    let x: Int64
    let y: Int64
}
private struct DisposableHostResult: Decodable {
    let nodes: [DisposablePositionNode]?
    let undo_available: Bool?
    let redo_available: Bool?
}
private struct DisposableHostResponse: Decodable {
    let id: UUID
    let snapshot: DisposableSessionSnapshot?
    let result: DisposableHostResult?
    let acknowledgement: DisposableOperationAcknowledgement?
    let error: String?
}
private struct DisposableHostRequest: Encodable {
    let id: UUID
    let command: String
    var node_id: String? = nil
    var x: Int64? = nil
    var y: Int64? = nil
    enum CodingKeys: String, CodingKey { case id, command, node_id, x, y }
    func encode(to encoder: Encoder) throws {
        var values = encoder.container(keyedBy: CodingKeys.self)
        try values.encode(id.uuidString.lowercased(), forKey: .id)
        try values.encode(command, forKey: .command)
        try values.encodeIfPresent(node_id, forKey: .node_id)
        try values.encodeIfPresent(x, forKey: .x)
        try values.encodeIfPresent(y, forKey: .y)
    }
}

/// Private, bounded stdio to the explicitly selected cfg(test) Rust native host.
/// No production registrar, network endpoint, or persistent transport format.
@MainActor
final class DisposableAutosaveProcess: ObservableObject {
    @Published private(set) var projection: DisposableAutosaveProjection?
    @Published private(set) var nodes: [DisposablePositionNode] = []
    @Published private(set) var pendingPosition: DisposablePendingPosition?
    @Published private(set) var transportFailure: String?
    @Published private(set) var busy = false
    @Published private(set) var closed = false
    @Published private(set) var undoAvailable = false
    @Published private(set) var redoAvailable = false
    private var process: Process?
    private var input: FileHandle?
    private var outputPipe: Pipe?
    private var errorPipe: Pipe?
    private var output = Data()
    private var stderrBytes = 0
    private var stdoutEnded = false
    private var hostEnded = false
    private var pending: DisposableHostRequest?
    private var failedRequest: DisposableHostRequest?
    private var deadline: Task<Void, Never>?
    private var closeCompletion: ((Bool) -> Void)?
    private static let frameLimit = 1_048_576
    private static let prefix = Data("PHOTARA_PS3 ".utf8)

    var canEdit: Bool { !busy && closeCompletion == nil && transportFailure == nil && !closed && projection?.canSubmit == true }

    func start(environment: [String: String] = ProcessInfo.processInfo.environment) {
        guard process == nil else { return }
        guard let executable = environment["PHOTARA_PS3_HOST"], executable.hasPrefix("/"),
              environment["PHOTARA_PS2_REMOUNT_MANIFEST"] != nil,
              environment["PHOTARA_PS2_REMOUNT_BINDING"] != nil else {
            transportFailure = "Launch with the disposable image controller. No project was opened."
            return
        }
        let child = Process(), stdin = Pipe(), stdout = Pipe(), stderr = Pipe()
        child.executableURL = URL(fileURLWithPath: executable)
        child.arguments = ["package::v1_3::repeatable::tests::native_test::native_repeatable_phase",
                           "--exact", "--ignored", "--nocapture"]
        var scoped = environment
        scoped["PHOTARA_PS2_REPEATABLE_PHASE"] = "session"
        child.environment = scoped
        child.standardInput = stdin; child.standardOutput = stdout; child.standardError = stderr
        stdout.fileHandleForReading.readabilityHandler = { [weak self] handle in
            let result = readDisposablePipe(handle.fileDescriptor)
            switch result {
            case .end, .failed: handle.readabilityHandler = nil
            case .bytes, .deferred: break
            }
            DispatchQueue.main.async {
                guard let self else { return }
                switch result {
                case .bytes(let bytes): self.consume(bytes)
                case .end: self.stdoutEnded = true; self.finishExit()
                case .deferred: break
                case .failed: self.fail("Could not read the session response; preserve the original request")
                }
            }
        }
        stderr.fileHandleForReading.readabilityHandler = { [weak self] handle in
            let result = readDisposablePipe(handle.fileDescriptor)
            switch result {
            case .end, .failed: handle.readabilityHandler = nil
            case .bytes, .deferred: break
            }
            DispatchQueue.main.async {
                guard let self else { return }
                switch result {
                case .bytes(let bytes):
                    if self.stderrBytes > Self.frameLimit - bytes.count {
                        self.fail("Host diagnostic output exceeded its bound")
                    } else { self.stderrBytes += bytes.count }
                case .end, .deferred: break
                case .failed: self.fail("Could not read session diagnostics")
                }
            }
        }
        child.terminationHandler = { [weak self] _ in
            DispatchQueue.main.async {
                guard let self else { return }
                self.hostEnded = true; self.finishExit()
            }
        }
        process = child; input = stdin.fileHandleForWriting
        outputPipe = stdout; errorPipe = stderr
        do { try child.run(); send(.init(id: UUID(), command: "snapshot")) }
        catch { fail("Could not start the disposable Rust session") }
    }

    func move(_ node: DisposablePositionNode, dx: Int64, dy: Int64) {
        guard canEdit else { return }
        let (x, overflowX) = node.x.addingReportingOverflow(dx)
        let (y, overflowY) = node.y.addingReportingOverflow(dy)
        guard !overflowX, !overflowY else { return }
        let id = UUID()
        guard projection?.begin(id) == true else { return }
        pendingPosition = .init(operationID: id, nodeID: node.id, title: node.title, x: x, y: y)
        send(.init(id: id, command: "submit", node_id: node.id, x: x, y: y))
    }

    func action(_ command: String) {
        guard canEdit, ["undo", "redo", "complete"].contains(command) else { return }
        guard command != "undo" || undoAvailable, command != "redo" || redoAvailable else { return }
        let id = UUID()
        if command != "complete", projection?.begin(id) != true { return }
        send(.init(id: id, command: command))
    }

    func retry() {
        guard !busy, !closed else { return }
        if let failedRequest {
            transportFailure = nil
            // Exact same request/operation ID; never generate a replacement edit.
            send(failedRequest)
        } else if projection?.snapshot?.frozen == true {
            // A reopened host owns persisted original inputs. This fresh ID is
            // only for a recovery request, never a mutation operation.
            transportFailure = nil
            send(.init(id: UUID(), command: "retry"))
        }
    }

    func close(_ completion: @escaping (Bool) -> Void) {
        if closed { completion(true); return }
        if projection == nil && process?.isRunning != true {
            closed = true; completion(true); return // No editable project was attached.
        }
        guard closeCompletion == nil else { completion(false); return }
        guard transportFailure == nil else {
            transportFailure = "Close canceled. Retry the original request before closing."
            completion(false); return
        }
        closeCompletion = completion
        if busy { return } // Drain this finite in-flight request and its checkpoint first.
        guard projection?.pendingOperationID == nil else {
            fail("Close canceled. An edit still needs recovery before closing."); return
        }
        send(.init(id: UUID(), command: "close"))
    }

    private func send(_ request: DisposableHostRequest) {
        guard pending == nil else { return }
        do {
            var data = try JSONEncoder().encode(request)
            guard data.count < Self.frameLimit else { throw CocoaError(.fileWriteInapplicableStringEncoding) }
            data.append(0x0a)
            pending = request; busy = true
            guard let input else { throw CocoaError(.fileWriteUnknown) }
            try input.write(contentsOf: data)
            deadline?.cancel()
            deadline = Task { [weak self] in
                do { try await Task.sleep(for: .seconds(60)) } catch { return }
                self?.fail("Rust session response is unknown; later edits are frozen")
            }
        } catch { fail("Command delivery is unknown; retain the same operation for retry") }
    }

    private func consume(_ bytes: Data) {
        guard !bytes.isEmpty else { return }
        guard bytes.count <= Self.frameLimit, output.count <= Self.frameLimit - bytes.count else {
            fail("Rust response exceeded the private frame bound"); return
        }
        output.append(bytes)
        while let end = output.firstIndex(of: 0x0a) {
            let line = Data(output[..<end]); output.removeSubrange(...end)
            // libtest emits fixed harness lines. Only the explicit host prefix is
            // a response; never decode or display arbitrary diagnostic text.
            guard line.starts(with: Self.prefix) else { continue }
            do {
                let value = try JSONDecoder().decode(DisposableHostResponse.self, from: line.dropFirst(Self.prefix.count))
                accept(value)
            } catch { fail("Invalid typed response from the Rust session") }
        }
    }

    private func accept(_ response: DisposableHostResponse) {
        guard let request = pending ?? failedRequest, response.id == request.id else { return }
        deadline?.cancel(); pending = nil; busy = false; failedRequest = request
        if let snapshot = response.snapshot {
            if projection == nil { projection = .init(binding: snapshot.binding) }
            let unchanged = response.acknowledgement == nil && projection?.snapshot == snapshot
            guard unchanged || projection?.receive(snapshot, acknowledgement: response.acknowledgement) == true else {
                fail("Rust response has stale or inconsistent session coordinates"); return
            }
        }
        guard response.snapshot != nil else {
            fail(response.error ?? "Session response omitted save evidence"); return
        }
        if let actualNodes = response.result?.nodes { nodes = actualNodes }
        if let available = response.result?.undo_available { undoAvailable = available }
        if let available = response.result?.redo_available { redoAvailable = available }
        if let acknowledgement = response.acknowledgement,
           acknowledgement.operationID == pendingPosition?.operationID,
           projection?.pendingOperationID == nil {
            pendingPosition = nil
        }
        if let error = response.error {
            failedRequest = request
            projection?.submissionFailed(request.id, message: error)
            transportFailure = error
            closeCompletion?(false); closeCompletion = nil
            return
        }
        failedRequest = nil; transportFailure = nil
        if request.command == "close" {
            guard projection?.status == .saved, projection?.snapshot?.closed == true else {
                fail("Close did not verify the current saved coordinate and closed session"); return
            }
            closed = true; try? input?.close()
            closeCompletion?(true); closeCompletion = nil
        } else if ["submit", "undo", "redo"].contains(request.command) {
            guard projection?.pendingOperationID == nil else {
                fail("Mutation response did not acknowledge the original operation"); return
            }
            send(.init(id: UUID(), command: "complete"))
        } else if request.command == "retry", projection?.status == .saving {
            send(.init(id: UUID(), command: "complete"))
        } else if closeCompletion != nil {
            send(.init(id: UUID(), command: "close"))
        }
    }

    private func finishExit() {
        guard hostEnded && stdoutEnded && !closed else { return }
        fail("Rust session ended; preserve the original request and reopen through recovery")
    }

    private func fail(_ message: String) {
        deadline?.cancel()
        if let pending { failedRequest = pending }
        if let id = projection?.pendingOperationID { projection?.submissionFailed(id, message: message) }
        pending = nil; busy = false; transportFailure = message
        closeCompletion?(false); closeCompletion = nil
    }
}
