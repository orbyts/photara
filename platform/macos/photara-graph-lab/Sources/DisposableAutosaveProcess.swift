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

struct DisposablePositionNode: Decodable, Identifiable, Equatable, Sendable {
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
struct DisposableHostResult: Decodable, Sendable {
    let nodes: [DisposablePositionNode]?
    let undo_available: Bool?
    let redo_available: Bool?
}
struct DisposableHostResponse: Decodable, Sendable {
    let id: UUID
    let snapshot: DisposableSessionSnapshot?
    let result: DisposableHostResult?
    let acknowledgement: DisposableOperationAcknowledgement?
    let error: String?
}
struct DisposableHostRequest: Encodable, Sendable {
    let id: UUID
    let command: String
    var node_id: String? = nil
    var x: Int64? = nil
    var y: Int64? = nil
    var expectedOwnerEpoch: UUID? = nil
    var expectedAttachmentGeneration: UInt64? = nil
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

/// In-process implementations invoke only the typed shared Rust authority.
/// The original standalone lab keeps its private stdio transport.
protocol DisposableSessionBridge: Sendable {
    func execute(_ request: DisposableHostRequest) async throws -> DisposableHostResponse
}

struct DisposableProjectChoice: Decodable, Identifiable, Equatable, Sendable {
    let id: UUID
    let title: String
}
struct DisposableSessionView: Codable, Equatable, Sendable {
    let device_id: String
    let library_id: String
    let project_id: String
    let graph_id: String
    var pan_x: Int64
    var pan_y: Int64
    var zoom_ppm: String
    var selected_node_ids: [String]
    var visible_panels: [String]
}
struct DisposableSessionDisplay: Decodable, Sendable {
    let snapshot: DisposableSessionSnapshot?
    let result: DisposableHostResult?
    let error: String?
}
struct DisposableWorkspaceState: Sendable {
    let current: DisposableSessionDisplay?
    let projects: [DisposableProjectChoice]
    let activeProjectID: UUID?
    let view: DisposableSessionView?
    let confirmationRequired: Bool
    let readOnly: Bool
    var error: String? = nil
    var recoveryNodes: [DisposablePositionNode]? = nil
    var pendingTarget: UUID? = nil
}
struct DisposableActivationResponse: Sendable {
    let status: String
    let activationID: UUID?
    let state: DisposableWorkspaceState
}
protocol DisposableWorkspaceBridge: DisposableSessionBridge {
    func workspace() async throws -> DisposableActivationResponse
    func prepare(activationID: UUID, target: UUID, view: DisposableSessionView?, binding: DisposableAutosaveBinding?) async throws -> DisposableActivationResponse
    func respond(activationID: UUID, command: String) async throws -> DisposableActivationResponse
}
struct DisposableSwitchPrompt: Equatable, Sendable {
    let id: UUID
    let sourceTitle: String
    let targetTitle: String
}

/// Presentation and retained-request lifecycle shared by the controlled app and
/// original standalone lab. Neither path can construct storage authority.
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
    @Published private(set) var checkingStorage = false
    @Published private(set) var projects: [DisposableProjectChoice] = []
    @Published private(set) var activeProjectID: UUID?
    @Published private(set) var restoredView: DisposableSessionView?
    @Published private(set) var switchPrompt: DisposableSwitchPrompt?
    @Published private(set) var switching = false
    @Published private(set) var switchFailure: String?
    @Published private(set) var readOnly = false
    private var switchRequest: (id: UUID, target: UUID, view: DisposableSessionView?)?
    private var switchInFlight = false
    var projectTitle: String { projects.first { $0.id == activeProjectID }?.title ?? "Disposable Project" }
    private var barrierRequested = false
    private let bridge: (any DisposableSessionBridge)?
    private var bridgeStarted = false
    init(bridge: (any DisposableSessionBridge)? = nil) { self.bridge = bridge }
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

    var canEdit: Bool { !switching && !readOnly && !checkingStorage && !busy && closeCompletion == nil && transportFailure == nil && !closed && projection?.canSubmit == true }

    func start(environment: [String: String] = ProcessInfo.processInfo.environment) {
        if bridge != nil {
            guard !bridgeStarted else { return }
            bridgeStarted = true
            if let workspace = bridge as? any DisposableWorkspaceBridge {
                busy = true
                Task {
                    do {
                        let response = try await workspace.workspace()
                        busy = false
                        try installWorkspace(response.state, replace: true)
                        if let id = response.activationID,
                           let target = response.state.pendingTarget ?? (response.state.readOnly ? response.state.activeProjectID : nil) {
                            switchRequest = (id, target, response.state.view)
                            switching = response.state.pendingTarget != nil
                            switchFailure = "A Project switch needs recovery. Retry the original switch."
                        }
                    } catch { busy = false; transportFailure = error.localizedDescription }
                }
            } else { send(.init(id: UUID(), command: "snapshot")) }
            return
        }
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
        move(node, x: x, y: y)
    }

    func move(_ node: DisposablePositionNode, x: Int64, y: Int64) {
        guard canEdit else { return }
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
        if readOnly, switchRequest != nil { switching = true; retrySwitch(); return }
        if switching { retrySwitch(); return }
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

    /// Sleep is a best-effort flush; already synced acceptance owns durability.
    func prepareForSleep() {
        guard bridge != nil, !closed else { return }
        barrierRequested = true
        drainLifecycle()
    }

    func revalidateAfterWake() {
        guard bridge != nil, !closed else { return }
        checkingStorage = true
        drainLifecycle()
    }

    private func drainLifecycle() {
        if switching { drainSwitch(); return }
        guard !busy, transportFailure == nil, !closed, closeCompletion == nil,
              projection?.pendingOperationID == nil else { return }
        if checkingStorage { send(.init(id: UUID(), command: "revalidate")) }
        else if barrierRequested { send(.init(id: UUID(), command: "barrier")) }
    }

    func close(_ completion: @escaping (Bool) -> Void) {
        if closed { completion(true); return }
        guard !switching else {
            switchFailure = "Close canceled. Finish or cancel the Project switch first."
            completion(false); return
        }
        if projection == nil && !readOnly && !busy && process?.isRunning != true {
            closed = true; completion(true); return // No editable project was attached.
        }
        guard closeCompletion == nil else { completion(false); return }
        guard !readOnly else {
            switchFailure = "Close canceled. Retry recovery before closing this read-only Project."
            completion(false); return
        }
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

    func beginSwitch(to target: UUID, view: DisposableSessionView?) {
        guard bridge is any DisposableWorkspaceBridge, !closed, !switching,
              closeCompletion == nil, projects.contains(where: { $0.id == target }) else { return }
        guard target != activeProjectID else { return }
        switchRequest = (UUID(), target, view)
        switching = true; switchFailure = nil
        drainSwitch()
    }

    private func drainSwitch() {
        guard switching, !busy, !switchInFlight, switchPrompt == nil,
              switchFailure == nil, let request = switchRequest,
              let workspace = bridge as? any DisposableWorkspaceBridge else { return }
        guard transportFailure == nil, projection?.pendingOperationID == nil else {
            switchFailure = "The current edit could not be saved. Retry it before switching."
            // Preparation never reached Rust; return to the original edit's retry.
            switching = false; switchRequest = nil
            return
        }
        switchInFlight = true
        Task {
            do {
                let response = try await workspace.prepare(activationID: request.id,
                    target: request.target, view: request.view, binding: projection?.binding)
                switchInFlight = false
                try acceptActivation(response, expected: request.id)
            } catch {
                switchInFlight = false
                switchDiagnostic(error.localizedDescription)
                switchFailure = "The switch outcome is unknown. Retry the original switch before editing or closing."
            }
        }
    }

    func answerSwitch(_ id: UUID, confirmed: Bool) {
        guard switchPrompt?.id == id, !switchInFlight else { return }
        switchPrompt = nil
        sendSwitch(id, command: confirmed ? "confirm" : "cancel")
    }

    func retrySwitch() {
        guard let request = switchRequest, !switchInFlight, !busy else { return }
        switchFailure = nil
        sendSwitch(request.id, command: "retry")
    }

    private func sendSwitch(_ id: UUID, command: String) {
        guard switchRequest?.id == id,
              let workspace = bridge as? any DisposableWorkspaceBridge else { return }
        switchInFlight = true
        Task {
            do {
                let response = try await workspace.respond(activationID: id, command: command)
                switchInFlight = false
                try acceptActivation(response, expected: id)
            } catch {
                switchInFlight = false
                switchDiagnostic(error.localizedDescription)
                switchFailure = "The switch outcome is unknown. Retry the original switch before editing or closing."
            }
        }
    }

    private func acceptActivation(_ response: DisposableActivationResponse, expected: UUID) throws {
        guard response.activationID == expected, let request = switchRequest,
              request.id == expected else { throw CocoaError(.coderInvalidValue) }
        switch response.status {
        case "Prepared":
            try installWorkspace(response.state, replace: false)
            if response.state.confirmationRequired {
                guard projection?.status == .saved, pendingPosition == nil,
                      let target = projects.first(where: { $0.id == request.target }) else {
                    throw CocoaError(.coderInvalidValue)
                }
                switchPrompt = .init(id: expected, sourceTitle: projectTitle, targetTitle: target.title)
            } else { sendSwitch(expected, command: "confirm") }
        case "Activated":
            guard response.state.activeProjectID == request.target,
                  response.state.current?.snapshot != nil else { throw CocoaError(.coderInvalidValue) }
            try installWorkspace(response.state, replace: true)
            switchRequest = nil; switching = false; switchFailure = nil
            drainLifecycle()
        case "RetainedCurrent", "RetainedReadOnlyRecovery", "Refused":
            if response.state.pendingTarget != nil {
                readOnly = response.state.readOnly
                switchDiagnostic(response.state.error)
                switchFailure = response.state.readOnly
                    ? "The previous Project is read-only. Retry recovery before editing or closing."
                    : "The switch needs recovery. Retry the original switch before editing or closing."
                return
            }
            guard response.state.activeProjectID == activeProjectID else { throw CocoaError(.coderInvalidValue) }
            // A failed activation may reacquire the same source under a new
            // Rust owner epoch. This explicit outcome can replace its binding.
            try installWorkspace(response.state, replace: response.state.current != nil)
            if !readOnly { switchRequest = nil }
            switching = false
            switchDiagnostic(response.state.error)
            if readOnly {
                switchFailure = "The previous Project is read-only. Retry recovery before editing or closing."
            } else if response.state.error != nil {
                switchFailure = projection?.status == .saved
                    ? "Could not switch Projects. The previous Project is still open. Check that the selected Project is available, then try again."
                    : "The current Project could not be saved. Retry saving before switching."
            } else { switchFailure = nil }
            if !readOnly { drainLifecycle() }
        case "Pending":
            switchDiagnostic(response.state.error)
            switchFailure = "The switch outcome is unknown. Retry the original switch before editing or closing."
        default: throw CocoaError(.coderInvalidValue)
        }
    }

    private func switchDiagnostic(_ raw: String?) {
        guard let raw else { return }
        // Preserve bounded native diagnostics in the process log, never in the
        // Project workflow or as a claim about current write authority.
        let bytes = Array(raw.utf8.prefix(4096))
        try? FileHandle.standardError.write(contentsOf: Data("Project switch diagnostic: ".utf8) + Data(bytes) + Data([10]))
    }

    private func installWorkspace(_ state: DisposableWorkspaceState, replace: Bool) throws {
        guard Set(state.projects.map(\.id)).count == state.projects.count,
              replace || state.activeProjectID == activeProjectID else { throw CocoaError(.coderInvalidValue) }
        if let current = state.current {
            guard let snapshot = current.snapshot, current.error == nil,
                  state.activeProjectID == snapshot.binding.projectID else { throw CocoaError(.coderInvalidValue) }
            var next = replace ? DisposableAutosaveProjection(binding: snapshot.binding) : projection
            guard next?.snapshot == snapshot || next?.receive(snapshot) == true else { throw CocoaError(.coderInvalidValue) }
            projection = next
            if let value = current.result?.nodes { nodes = value }
            undoAvailable = current.result?.undo_available ?? false
            redoAvailable = current.result?.redo_available ?? false
        } else if state.readOnly {
            // Only the shared authority can enter capsule/read-only retention.
            // Keep the already visible graph; no synthetic editable snapshot.
            if nodes.isEmpty, let capsuleNodes = state.recoveryNodes { nodes = capsuleNodes }
        } else if state.activeProjectID != nil || projection != nil { throw CocoaError(.coderInvalidValue) }
        projects = state.projects; activeProjectID = state.activeProjectID; readOnly = state.readOnly
        if replace { restoredView = state.view }
        if state.readOnly { undoAvailable = false; redoAvailable = false }
    }

    private func send(_ originalRequest: DisposableHostRequest) {
        guard pending == nil else { return }
        var request = originalRequest
        if bridge != nil, request.expectedOwnerEpoch == nil {
            request.expectedOwnerEpoch = projection?.binding.ownerEpoch
            request.expectedAttachmentGeneration = projection?.binding.attachmentGeneration
        }
        if let bridge {
            pending = request; busy = true
            Task {
                do { accept(try await bridge.execute(request)) }
                catch { fail("Session outcome is unknown: \(error.localizedDescription). Retain the original request for retry.") }
            }
            return
        }
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
            if switching { drainSwitch() }
            return
        }
        failedRequest = nil; transportFailure = nil
        if request.command == "revalidate" { checkingStorage = false }
        if request.command == "barrier" { barrierRequested = false }
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
        } else { drainLifecycle() }
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
        if switching && !switchInFlight && switchPrompt == nil {
            switching = false; switchRequest = nil
            switchFailure = "The current edit could not be saved. Retry it before switching."
        }
        closeCompletion?(false); closeCompletion = nil
    }
}
