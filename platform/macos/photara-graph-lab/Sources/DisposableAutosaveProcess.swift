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
struct DisposablePendingPosition: Codable, Equatable {
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
    var context_closed: Bool? = nil
    var context_verified: Bool? = nil
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
struct DisposableLibraryChoice: Decodable, Identifiable, Equatable, Sendable {
    let id: UUID
    let name: String
    let revision: UInt64
    init(id: UUID, name: String, revision: UInt64) { self.id = id; self.name = name; self.revision = revision }
    private enum CodingKeys: String, CodingKey { case id, name, revision }
    init(from decoder: any Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        id = try values.decode(UUID.self, forKey: .id)
        name = try values.decode(String.self, forKey: .name)
        let value = try values.decode(String.self, forKey: .revision)
        guard let parsed = UInt64(value), parsed > 0, String(parsed) == value else {
            throw DecodingError.dataCorruptedError(forKey: .revision, in: values, debugDescription: "Invalid Library revision")
        }
        revision = parsed
    }
}
struct DisposableLibraryRequest: Equatable, Sendable {
    let operationID: UUID
    let libraryID: UUID
    let name: String
    let expectedRevision: UInt64?
}
struct DisposableLibraryReceipt: Sendable {
    let operationID: UUID
    let libraryID: UUID
    let action: String
    var rejected = false
}
struct DisposableLibraryResponse: Sendable {
    let receipt: DisposableLibraryReceipt
    let state: DisposableWorkspaceState
}
protocol DisposableLibraryBridge: DisposableWorkspaceBridge {
    func selectLibrary(activationID: UUID, libraryID: UUID, view: DisposableSessionView?, binding: DisposableAutosaveBinding?) async throws -> DisposableActivationResponse
    func closeProject(activationID: UUID, view: DisposableSessionView?, binding: DisposableAutosaveBinding?) async throws -> DisposableActivationResponse
    func libraryCommand(_ request: DisposableLibraryRequest) async throws -> DisposableLibraryResponse
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
    var libraries: [DisposableLibraryChoice] = []
    var selectedLibraryID: UUID? = nil
    var creatableLibraryIDs: [UUID] = []
    var pendingIsLibrary = false
    var localLibraryContext = false
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
    @Published private(set) var queuedPositions: [DisposablePendingPosition] = []
    @Published private(set) var moveRefusal: String?
    @Published private(set) var transportFailure: String?
    @Published private(set) var busy = false
    @Published private(set) var closed = false
    @Published private(set) var undoAvailable = false
    @Published private(set) var redoAvailable = false
    @Published private(set) var checkingStorage = false
    @Published private(set) var libraries: [DisposableLibraryChoice] = []
    @Published private(set) var localLibraryContext = false
    @Published private(set) var selectedLibraryID: UUID?
    @Published private(set) var creatableLibraryIDs: [UUID] = []
    @Published private(set) var libraryFailure: String?
    @Published private(set) var libraryWorking = false
    private var libraryPending: DisposableLibraryRequest?
    var libraryRequestDescription: String? {
        libraryPending.map { "\($0.expectedRevision == nil ? "Create" : "Rename") Library: \($0.name)" }
    }
    var libraryTitle: String { libraries.first { $0.id == selectedLibraryID }?.name ?? "Local Libraries" }
    @Published private(set) var projects: [DisposableProjectChoice] = []
    @Published private(set) var activeProjectID: UUID?
    @Published private(set) var restoredView: DisposableSessionView?
    @Published private(set) var switchPrompt: DisposableSwitchPrompt?
    @Published private(set) var switching = false
    @Published private(set) var switchFailure: String?
    @Published private(set) var readOnly = false
    private var switchRequest: (id: UUID, target: UUID, view: DisposableSessionView?, library: Bool, closeProject: Bool)?
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

    var visibleNodes: [DisposablePositionNode] {
        var values = Dictionary(uniqueKeysWithValues: nodes.map { ($0.id, $0) })
        for draft in (pendingPosition.map { [$0] } ?? []) + queuedPositions {
            if let node = values[draft.nodeID] {
                values[draft.nodeID] = .init(id: node.id, title: node.title, x: draft.x, y: draft.y)
            }
        }
        return nodes.compactMap { values[$0.id] }
    }
    var visualStatus: DisposableAutosaveStatus {
        if transportFailure != nil { return .failed("Changes could not be saved") }
        if case .failed = projection?.status { return .failed("Changes could not be saved") }
        if pendingPosition != nil || !queuedPositions.isEmpty { return .saving }
        return projection?.status ?? .unavailable
    }
    /// Extra drops remain explicitly volatile drafts until their original FIFO
    /// request is acknowledged. Only position edits can queue across a checkpoint.
    var canMove: Bool {
        guard libraryPending == nil, !libraryWorking, !switching, !readOnly, !checkingStorage,
              closeCompletion == nil, transportFailure == nil, !closed,
              let snapshot = projection?.snapshot, !snapshot.frozen, !snapshot.closed,
              snapshot.failure == nil, projection?.submissionFailure == nil else { return false }
        return !busy || pending.map { ["submit", "complete"].contains($0.command) } == true
    }
    var canEdit: Bool { queuedPositions.isEmpty && libraryPending == nil && !libraryWorking && !switching && !readOnly && !checkingStorage && !busy && closeCompletion == nil && transportFailure == nil && !closed && projection?.canSubmit == true }

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
                            switchRequest = (id, target, response.state.view, response.state.pendingIsLibrary, false)
                            switching = response.state.pendingTarget != nil
                            switchFailure = "A Project switch needs recovery. Retry the original switch."
                        }
                        if let completion = closeCompletion {
                            closeCompletion = nil
                            close(completion)
                        } else { drainLifecycle() }
                    } catch {
                        busy = false; transportFailure = error.localizedDescription
                        closeCompletion?(false); closeCompletion = nil
                    }
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
        guard canMove, let visible = visibleNodes.first(where: { $0.id == node.id }) else { return }
        let (x, overflowX) = visible.x.addingReportingOverflow(dx)
        let (y, overflowY) = visible.y.addingReportingOverflow(dy)
        guard !overflowX, !overflowY else { return }
        move(node, x: x, y: y)
    }

    func move(_ node: DisposablePositionNode, x: Int64, y: Int64) {
        guard canMove, nodes.contains(where: { $0.id == node.id }) else { return }
        let draft = DisposablePendingPosition(operationID: UUID(), nodeID: node.id, title: node.title, x: x, y: y)
        let proposed = (pendingPosition.map { [$0] } ?? []) + queuedPositions + [draft]
        guard let bytes = try? JSONEncoder().encode(proposed), bytes.count <= Self.frameLimit else {
            moveRefusal = "This move could not be queued. Let saving finish, then try again."
            return
        }
        moveRefusal = nil
        queuedPositions.append(draft)
        _ = drainMoves()
    }

    @discardableResult private func drainMoves() -> Bool {
        guard !busy, !closed, !readOnly, !checkingStorage, transportFailure == nil,
              pendingPosition == nil, let draft = queuedPositions.first,
              projection?.begin(draft.operationID) == true else { return false }
        queuedPositions.removeFirst()
        pendingPosition = draft
        send(.init(id: draft.operationID, command: "submit", node_id: draft.nodeID, x: draft.x, y: draft.y))
        return true
    }

    func action(_ command: String) {
        guard canEdit, ["undo", "redo", "complete"].contains(command) else { return }
        guard command != "undo" || undoAvailable, command != "redo" || redoAvailable else { return }
        let id = UUID()
        if command != "complete", projection?.begin(id) != true { return }
        send(.init(id: id, command: command))
    }

    func retry() {
        if libraryPending != nil { retryLibraryCommand(); return }
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
        if drainMoves() { return }
        guard !busy, transportFailure == nil, !closed, closeCompletion == nil,
              projection?.pendingOperationID == nil else { return }
        if checkingStorage { send(.init(id: UUID(), command: "revalidate")) }
        else if barrierRequested { send(.init(id: UUID(), command: "barrier")) }
    }

    func close(_ completion: @escaping (Bool) -> Void) {
        if closed { completion(true); return }
        guard libraryPending == nil, !libraryWorking else {
            libraryFailure = "Close canceled. Retry the original Library request first."
            completion(false); return
        }
        guard !switching else {
            switchFailure = "Close canceled. Finish or cancel the Project switch first."
            completion(false); return
        }
        if projection == nil && !localLibraryContext && selectedLibraryID == nil && !readOnly && !busy && process?.isRunning != true {
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
        if checkingStorage { send(.init(id: UUID(), command: "revalidate")); return }
        if drainMoves() { return }
        guard projection?.pendingOperationID == nil, queuedPositions.isEmpty else {
            fail("Close canceled. An edit still needs recovery before closing."); return
        }
        send(.init(id: UUID(), command: "close"))
    }

    func beginSwitch(to target: UUID, view: DisposableSessionView?) {
        guard bridge is any DisposableWorkspaceBridge, !closed, !switching, libraryPending == nil,
              closeCompletion == nil, projects.contains(where: { $0.id == target }) else { return }
        guard target != activeProjectID else { return }
        switchRequest = (UUID(), target, view, false, false)
        switching = true; switchFailure = nil
        drainSwitch()
    }

    func selectLibrary(_ id: UUID, view: DisposableSessionView?) {
        guard bridge is any DisposableLibraryBridge, !closed, !switching, libraryPending == nil,
              closeCompletion == nil, libraries.contains(where: { $0.id == id }) else { return }
        guard id != selectedLibraryID else { return } // Preserve its current Project.
        switchRequest = (UUID(), id, view, true, false)
        switching = true; switchFailure = nil
        drainSwitch()
    }

    func closeProject(view: DisposableSessionView?) {
        guard bridge is any DisposableLibraryBridge, let library = selectedLibraryID,
              activeProjectID != nil, !readOnly, !closed, !switching, libraryPending == nil,
              closeCompletion == nil else { return }
        switchRequest = (UUID(), library, view, true, true)
        switching = true; switchFailure = nil
        drainSwitch()
    }

    func createLibrary(name: String) {
        guard let id = creatableLibraryIDs.first else {
            libraryFailure = "No additional disposable Library is registered for this session."; return
        }
        createLibrary(id: id, name: name)
    }

    func createLibrary(id: UUID, name: String) {
        guard creatableLibraryIDs.contains(id) else {
            libraryFailure = "The original disposable Library is no longer available for creation."
            return
        }
        beginLibraryCommand(.init(operationID: UUID(), libraryID: id, name: name, expectedRevision: nil))
    }

    func renameLibrary(_ library: DisposableLibraryChoice, name: String) {
        beginLibraryCommand(.init(operationID: UUID(), libraryID: library.id, name: name, expectedRevision: library.revision))
    }

    static func libraryNameIsValid(_ name: String) -> Bool {
        let normalized = name.trimmingCharacters(in: .whitespacesAndNewlines)
        return !normalized.isEmpty && normalized.utf8.count <= 128
            && normalized.unicodeScalars.allSatisfy { $0.properties.generalCategory != .control }
    }

    private func beginLibraryCommand(_ request: DisposableLibraryRequest) {
        guard !busy, !closed, !switching, !readOnly, !checkingStorage, transportFailure == nil,
              closeCompletion == nil, !libraryWorking, libraryPending == nil else { return }
        guard Self.libraryNameIsValid(request.name) else {
            libraryFailure = "Choose a shorter Library name without control characters."
            return
        }
        libraryPending = request; libraryFailure = nil
        retryLibraryCommand()
    }

    func retryLibraryCommand() {
        guard let request = libraryPending, !libraryWorking,
              let local = bridge as? any DisposableLibraryBridge else { return }
        libraryWorking = true; libraryFailure = nil
        Task {
            do {
                let response = try await local.libraryCommand(request)
                guard response.receipt.operationID == request.operationID,
                      response.receipt.libraryID == request.libraryID,
                      response.receipt.action == (request.expectedRevision == nil ? "create" : "rename") else {
                    throw CocoaError(.coderInvalidValue)
                }
                try installWorkspace(response.state, replace: false)
                libraryPending = nil; libraryWorking = false
                if response.receipt.rejected {
                    libraryFailure = "The Library request was refused. Check the name and current Library access, then try again."
                }
            } catch {
                libraryWorking = false; switchDiagnostic(error.localizedDescription)
                libraryFailure = "The Library request could not be completed. Retry the original request."
            }
        }
    }

    private func drainSwitch() {
        guard switching, !busy, !switchInFlight, switchPrompt == nil,
              switchFailure == nil, let request = switchRequest,
              let workspace = bridge as? any DisposableWorkspaceBridge else { return }
        if checkingStorage { send(.init(id: UUID(), command: "revalidate")); return }
        if drainMoves() { return }
        guard transportFailure == nil, projection?.pendingOperationID == nil, queuedPositions.isEmpty else {
            switchFailure = "The current edit could not be saved. Retry it before switching."
            // Preparation never reached Rust; return to the original edit's retry.
            switching = false; switchRequest = nil
            return
        }
        switchInFlight = true
        Task {
            do {
                let response: DisposableActivationResponse
                if request.closeProject, let local = workspace as? any DisposableLibraryBridge {
                    response = try await local.closeProject(activationID: request.id, view: request.view, binding: projection?.binding)
                } else if request.library, let local = workspace as? any DisposableLibraryBridge {
                    response = try await local.selectLibrary(activationID: request.id, libraryID: request.target,
                        view: request.view, binding: projection?.binding)
                } else {
                    response = try await workspace.prepare(activationID: request.id,
                        target: request.target, view: request.view, binding: projection?.binding)
                }
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
                guard !request.library else { throw CocoaError(.coderInvalidValue) }
                guard projection?.status == .saved, pendingPosition == nil, queuedPositions.isEmpty,
                      let target = projects.first(where: { $0.id == request.target }) else {
                    throw CocoaError(.coderInvalidValue)
                }
                switchPrompt = .init(id: expected, sourceTitle: projectTitle, targetTitle: target.title)
            } else { sendSwitch(expected, command: "confirm") }
        case "Activated":
            if request.library {
                guard response.state.selectedLibraryID == request.target, response.state.activeProjectID == nil,
                      response.state.current == nil else { throw CocoaError(.coderInvalidValue) }
            } else {
                guard response.state.activeProjectID == request.target,
                      response.state.current?.snapshot != nil else { throw CocoaError(.coderInvalidValue) }
            }
            try installWorkspace(response.state, replace: true, libraryOnly: request.library)
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
                if request.library && activeProjectID == nil {
                    switchFailure = "Could not select the Library. The previous Library is still selected. Try again."
                } else {
                    switchFailure = projection?.status == .saved
                        ? "Could not switch Projects. The previous Project is still open. Check that the selected Project is available, then try again."
                        : "The current Project could not be saved. Retry saving before switching."
                }
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

    private func installWorkspace(_ state: DisposableWorkspaceState, replace: Bool, libraryOnly: Bool = false) throws {
        guard Set(state.projects.map(\.id)).count == state.projects.count,
              Set(state.libraries.map(\.id)).count == state.libraries.count,
              Set(state.creatableLibraryIDs).count == state.creatableLibraryIDs.count,
              state.selectedLibraryID == nil || state.libraries.contains(where: { $0.id == state.selectedLibraryID }),
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
        } else if libraryOnly && replace && state.selectedLibraryID != nil && state.activeProjectID == nil {
            guard pendingPosition == nil, queuedPositions.isEmpty else { throw CocoaError(.coderInvalidValue) }
            projection = nil; nodes = []; pendingPosition = nil
            undoAvailable = false; redoAvailable = false
        } else if state.activeProjectID != nil || projection != nil { throw CocoaError(.coderInvalidValue) }
        localLibraryContext = state.localLibraryContext
        libraries = state.libraries; selectedLibraryID = state.selectedLibraryID
        creatableLibraryIDs = state.creatableLibraryIDs
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
        if request.command == "close", response.error == nil, response.result?.context_closed == true,
           activeProjectID == nil, localLibraryContext, !readOnly {
            failedRequest = nil; transportFailure = nil; closed = true
            closeCompletion?(true); closeCompletion = nil
            return
        }
        if ["snapshot", "revalidate", "barrier"].contains(request.command), response.error == nil,
           response.result?.context_verified == true, activeProjectID == nil,
           localLibraryContext, projection == nil, !readOnly {
            failedRequest = nil; transportFailure = nil
            if request.command == "revalidate" { checkingStorage = false }
            if request.command == "barrier" { barrierRequested = false }
            if closeCompletion != nil { send(.init(id: UUID(), command: "close")) }
            else { drainLifecycle() }
            return
        }
        guard response.snapshot != nil else {
            fail(response.error ?? "Session response omitted save evidence"); return
        }
        if let actualNodes = response.result?.nodes { nodes = actualNodes }
        if let available = response.result?.undo_available { undoAvailable = available }
        if let available = response.result?.redo_available { redoAvailable = available }
        if let acknowledgement = response.acknowledgement,
           let draft = pendingPosition, acknowledgement.operationID == draft.operationID,
           projection?.pendingOperationID == nil {
            guard response.result?.nodes?.contains(where: { $0.id == draft.nodeID && $0.x == draft.x && $0.y == draft.y }) == true else {
                fail("The acknowledged move needs an exact current Graph response before continuing."); return
            }
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
        } else if checkingStorage {
            send(.init(id: UUID(), command: "revalidate"))
        } else if drainMoves() {
            // Every original move completes in FIFO order before lifecycle work.
        } else if closeCompletion != nil {
            guard queuedPositions.isEmpty else { fail("Queued edits still need recovery before closing."); return }
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
