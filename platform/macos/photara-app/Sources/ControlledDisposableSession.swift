import AppKit
import SwiftUI

/// The actor keeps synchronous UniFFI calls off MainActor and in request order.
/// Only Rust's controlled constructor can acquire project authority.
private actor ControlledDisposableBridge: DisposableLibraryBridge {
    private var session: ControlledDisposableSession?
    private var workspaceSession: ControlledDisposableWorkspace?
    private let environment: [String: String]
    private let localLibraryRoute: Bool
    init(environment: [String: String], localLibraryRoute: Bool = false) {
        self.environment = environment; self.localLibraryRoute = localLibraryRoute
    }

    private func open() throws {
        guard session == nil, workspaceSession == nil else { return }
        let config = try ControlledDisposableConfiguration.load(environment)
        if localLibraryRoute {
            guard let targets = config.targetBindingsJSON, let database = config.databaseRegistrationPath else {
                throw ControlledDisposableConfiguration.ConfigurationError.missingOrInvalid
            }
            workspaceSession = try ControlledDisposableWorkspace.openLocal(manifestPath: config.manifestPath,
                bindingJson: config.bindingJSON, targetBindingsJson: targets, databaseRegistrationPath: database)
        } else if config.databaseRegistrationPath != nil {
            throw ControlledDisposableConfiguration.ConfigurationError.missingOrInvalid
        } else if let targets = config.targetBindingsJSON {
            workspaceSession = try ControlledDisposableWorkspace.open(manifestPath: config.manifestPath,
                bindingJson: config.bindingJSON, targetBindingsJson: targets)
        } else {
            session = try ControlledDisposableSession.open(manifestPath: config.manifestPath,
                bindingJson: config.bindingJSON)
        }
    }

    func workspace() throws -> DisposableActivationResponse {
        try open()
        if let workspaceSession { return try decodeActivation(workspaceSession.state()) }
        let current = try execute(.init(id: UUID(), command: "snapshot"))
        guard let snapshot = current.snapshot else { throw CocoaError(.coderInvalidValue) }
        return .init(status: "Current", activationID: nil, state: .init(current: .init(snapshot: current.snapshot, result: current.result, error: current.error),
            projects: [.init(id: snapshot.binding.projectID, title: "Disposable Project")],
            activeProjectID: snapshot.binding.projectID, view: nil, confirmationRequired: false, readOnly: false))
    }

    func prepare(activationID: UUID, target: UUID, view: DisposableSessionView?, binding: DisposableAutosaveBinding?) throws -> DisposableActivationResponse {
        guard let workspaceSession else { throw CocoaError(.featureUnsupported) }
        let data = try view.map { try JSONEncoder().encode($0) } ?? Data("null".utf8)
        guard data.count <= 65_536 else { throw CocoaError(.coderInvalidValue) }
        return try decodeActivation(workspaceSession.prepare(activationId: activationID.uuidString.lowercased(),
            targetProjectId: target.uuidString.lowercased(), sourceViewJson: String(decoding: data, as: UTF8.self),
            expectedOwnerEpoch: binding?.ownerEpoch.uuidString.lowercased(),
            expectedAttachmentGeneration: binding?.attachmentGeneration))
    }

    func respond(activationID: UUID, command: String) throws -> DisposableActivationResponse {
        guard let workspaceSession else { throw CocoaError(.featureUnsupported) }
        let id = activationID.uuidString.lowercased()
        switch command {
        case "confirm": return try decodeActivation(workspaceSession.confirm(activationId: id))
        case "cancel": return try decodeActivation(workspaceSession.cancel(activationId: id))
        case "retry": return try decodeActivation(workspaceSession.retry(activationId: id))
        default: throw CocoaError(.coderInvalidValue)
        }
    }

    func selectLibrary(activationID: UUID, libraryID: UUID, view: DisposableSessionView?, binding: DisposableAutosaveBinding?) throws -> DisposableActivationResponse {
        guard localLibraryRoute, let workspaceSession else { throw CocoaError(.featureUnsupported) }
        let data = try view.map { try JSONEncoder().encode($0) } ?? Data("null".utf8)
        guard data.count <= 65_536 else { throw CocoaError(.coderInvalidValue) }
        return try decodeActivation(workspaceSession.selectLibrary(activationId: activationID.uuidString.lowercased(),
            libraryId: libraryID.uuidString.lowercased(), viewJson: String(decoding: data, as: UTF8.self),
            expectedOwnerEpoch: binding?.ownerEpoch.uuidString.lowercased(),
            expectedAttachmentGeneration: binding?.attachmentGeneration))
    }

    func closeProject(activationID: UUID, view: DisposableSessionView?, binding: DisposableAutosaveBinding?) throws -> DisposableActivationResponse {
        guard localLibraryRoute, let workspaceSession else { throw CocoaError(.featureUnsupported) }
        let data = try view.map { try JSONEncoder().encode($0) } ?? Data("null".utf8)
        guard data.count <= 65_536 else { throw CocoaError(.coderInvalidValue) }
        return try decodeActivation(workspaceSession.closeProject(activationId: activationID.uuidString.lowercased(),
            viewJson: String(decoding: data, as: UTF8.self), expectedOwnerEpoch: binding?.ownerEpoch.uuidString.lowercased(),
            expectedAttachmentGeneration: binding?.attachmentGeneration))
    }

    func libraryCommand(_ request: DisposableLibraryRequest) throws -> DisposableLibraryResponse {
        guard localLibraryRoute, let workspaceSession else { throw CocoaError(.featureUnsupported) }
        let result: ControlledActivationResponse
        if let revision = request.expectedRevision {
            result = try workspaceSession.renameLibrary(operationId: request.operationID.uuidString.lowercased(),
                libraryId: request.libraryID.uuidString.lowercased(), expectedRevision: revision, name: request.name)
        } else {
            result = try workspaceSession.createLibrary(operationId: request.operationID.uuidString.lowercased(),
                libraryId: request.libraryID.uuidString.lowercased(), name: request.name)
        }
        struct ReceiptEnvelope: Decodable {
            struct Receipt: Decodable {
                struct Result: Decodable { let kind: String }
                let operation_id: UUID; let library_id: UUID; let action: String; let result: Result
            }
            let library_receipt: Receipt?
        }
        guard let envelope = try decode(result.stateJson, as: ReceiptEnvelope.self), let receipt = envelope.library_receipt,
              receipt.result.kind == "rejected" || receipt.result.kind == (request.expectedRevision == nil ? "created" : "renamed") else {
            throw CocoaError(.coderInvalidValue)
        }
        return try .init(receipt: .init(operationID: receipt.operation_id, libraryID: receipt.library_id, action: receipt.action, rejected: receipt.result.kind == "rejected"),
            state: decodeActivation(result).state)
    }

    private struct WorkspaceDTO: Decodable {
        struct Active: Decodable {
            struct Body: Decodable {
                struct Target: Decodable { struct Project: Decodable { let project_id: UUID }; let project: Project? }
                let project_id: UUID?
                let target: Target?
            }
            let body: Body
        }
        struct Project: Decodable { let project_id: UUID; let title: String? }
        let status: String
        let activation_id: UUID?
        let active: Active?
        let current: DisposableSessionDisplay?
        let view: DisposableSessionView?
        let projects: [Project]
        let confirmation_required: Bool
        let read_only: Bool
        let error: String?
        let recovery_nodes: [DisposablePositionNode]?
        let pending_target_project_id: UUID?
        let pending_target_library_id: UUID?
        let libraries: [DisposableLibraryChoice]?
        let selected_library_id: UUID?
        let creatable_library_ids: [UUID]?
    }
    private func decodeActivation(_ response: ControlledActivationResponse) throws -> DisposableActivationResponse {
        guard let value = try decode(response.stateJson, as: WorkspaceDTO.self),
              value.status == response.status,
              response.activationId == nil || UUID(uuidString: response.activationId!) != nil,
              value.activation_id == response.activationId.flatMap(UUID.init(uuidString:)) else {
            throw CocoaError(.coderInvalidValue)
        }
        return .init(status: response.status, activationID: response.activationId.flatMap(UUID.init(uuidString:)),
            state: .init(current: value.current, projects: value.projects.map { .init(id: $0.project_id, title: $0.title ?? "Project \($0.project_id.uuidString.lowercased())") },
                activeProjectID: value.active?.body.project_id ?? value.active?.body.target?.project?.project_id, view: value.view,
                confirmationRequired: value.confirmation_required, readOnly: value.read_only, error: value.error,
                recoveryNodes: value.recovery_nodes, pendingTarget: value.pending_target_project_id ?? value.pending_target_library_id,
                libraries: value.libraries ?? [], selectedLibraryID: value.selected_library_id,
                creatableLibraryIDs: value.creatable_library_ids ?? [],
                pendingIsLibrary: value.pending_target_project_id == nil && value.pending_target_library_id != nil,
                localLibraryContext: value.libraries != nil))
    }

    func execute(_ request: DisposableHostRequest) throws -> DisposableHostResponse {
        try open()
        let command: ControlledSessionCommand
        switch request.command {
        case "snapshot": command = .snapshot
        case "revalidate": command = .revalidate
        case "submit": command = .submit
        case "complete": command = .complete
        case "barrier": command = .barrier
        case "undo": command = .undo
        case "redo": command = .redo
        case "retry": command = .retry
        case "close": command = .close
        default: throw CocoaError(.coderInvalidValue)
        }
        let typed = ControlledSessionRequest(id: request.id.uuidString.lowercased(),
            command: command, nodeId: request.node_id, x: request.x, y: request.y,
            expectedOwnerEpoch: request.expectedOwnerEpoch?.uuidString.lowercased(),
            expectedAttachmentGeneration: request.expectedAttachmentGeneration)
        let result: ControlledSessionResponse
        if let workspaceSession { result = try workspaceSession.execute(request: typed) }
        else if let session { result = try session.execute(request: typed) }
        else { throw CocoaError(.coderInvalidValue) }
        guard result.id == request.id.uuidString.lowercased() else { throw CocoaError(.coderInvalidValue) }
        return try .init(id: request.id,
            snapshot: decode(result.snapshotJson, as: DisposableSessionSnapshot.self),
            result: decode(result.resultJson, as: DisposableHostResult.self),
            acknowledgement: decode(result.acknowledgementJson, as: DisposableOperationAcknowledgement.self),
            error: result.error)
    }

    private func decode<T: Decodable>(_ value: String?, as type: T.Type) throws -> T? {
        guard let value else { return nil }
        guard value.utf8.count <= 1_048_576 else { throw CocoaError(.coderInvalidValue) }
        return try JSONDecoder().decode(type, from: Data(value.utf8))
    }
}

@MainActor
func makeControlledDisposableModel() -> DisposableAutosaveProcess {
    DisposableAutosaveProcess(bridge: ControlledDisposableBridge(environment: ProcessInfo.processInfo.environment))
}

@MainActor
func makeLocalLibrarySessionModel() -> DisposableAutosaveProcess {
    DisposableAutosaveProcess(bridge: ControlledDisposableBridge(environment: ProcessInfo.processInfo.environment,
        localLibraryRoute: true))
}

struct ControlledDisposableScene: Scene {
    let delegate: ControlledSessionApplicationDelegate
    @ObservedObject var model: DisposableAutosaveProcess

    var body: some Scene {
        Window("Photara — \(model.projectTitle)", id: "controlled-project") {
            ControlledDisposableEditor(model: model)
                .task { delegate.model = model; model.start() }
        }
        .windowToolbarStyle(.unified)
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("Close Project") { model.close { if $0 { NSApp.terminate(nil) } } }.keyboardShortcut("w")
            }
            CommandGroup(replacing: .saveItem) {
                Button("Save") { model.action("complete") }.keyboardShortcut("s").disabled(!model.canEdit)
            }
            CommandGroup(replacing: .undoRedo) {
                Button("Undo Move") { model.action("undo") }.keyboardShortcut("z")
                    .disabled(!model.canEdit || !model.undoAvailable)
                Button("Redo Move") { model.action("redo") }.keyboardShortcut("z", modifiers: [.command, .shift])
                    .disabled(!model.canEdit || !model.redoAvailable)
            }
        }
    }
}

struct ControlledWindowCloseGuard: NSViewRepresentable {
    let model: DisposableAutosaveProcess
    func makeCoordinator() -> Coordinator { Coordinator(model) }
    func makeNSView(context: Context) -> NSView { NSView() }
    func updateNSView(_ view: NSView, context: Context) {
        let coordinator = context.coordinator
        DispatchQueue.main.async { [weak view] in view?.window?.delegate = coordinator }
    }
    @MainActor final class Coordinator: NSObject, NSWindowDelegate {
        let model: DisposableAutosaveProcess
        init(_ model: DisposableAutosaveProcess) { self.model = model }
        func windowShouldClose(_ sender: NSWindow) -> Bool {
            if model.closed { return true }
            model.close { success in if success { sender.performClose(nil) } }
            return false
        }
    }
}

private struct ControlledDisposableEditor: View {
    @ObservedObject var model: DisposableAutosaveProcess
    @Environment(\.colorScheme) private var colorScheme
    @State private var controller = PhotaraGraphInteractionController(document: .init(nodes: [], connections: []))
    @State private var didCenter = false
    @State private var selectedProject: UUID?
    @State private var showsProjects = false

    private func prepareSwitch(_ target: UUID) {
        if target == model.activeProjectID { showsProjects = false; return }
        if model.activeProjectID != nil { showsProjects = false }
        controller.cancel(resetTool: true)
        var view = model.restoredView
        if let panX = Int64(exactly: (controller.camera.pan.width * 1_000).rounded()),
           let panY = Int64(exactly: (controller.camera.pan.height * 1_000).rounded()),
           let zoom = UInt64(exactly: (controller.camera.zoom * 1_000_000).rounded()) {
            view?.pan_x = panX; view?.pan_y = panY; view?.zoom_ppm = String(zoom)
            view?.visible_panels = ["graph"]
            if case let .node(id) = controller.selection { view?.selected_node_ids = [id] }
            else { view?.selected_node_ids = [] }
        }
        model.beginSwitch(to: target, view: view)
    }

    private var projectBrowser: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Projects").font(.title2)
            List(model.projects, selection: $selectedProject) { project in
                HStack {
                    Text(project.title)
                    Spacer()
                    if project.id == model.activeProjectID { Text("Open").foregroundStyle(.secondary) }
                }
                .contentShape(Rectangle())
                .onTapGesture(count: 2) { prepareSwitch(project.id) }
                .tag(project.id)
            }
            if let selected = model.projects.first(where: { $0.id == selectedProject }) {
                HStack {
                    Text(selected.title)
                    Spacer()
                    Button(selected.id == model.activeProjectID ? "Show Graph" : "Open Project") { prepareSwitch(selected.id) }
                }
            } else { Text("Select a Project to preview it. Double-click to open.").foregroundStyle(.secondary) }
        }.padding(16).disabled(model.switching)
    }

    private var document: PhotaraGraphDocument {
        .init(nodes: model.nodes.map { node in
            .init(id: node.id, kind: "controlled-position", title: node.title,
                  subtitle: "x \(node.x) · y \(node.y)", ports: [],
                  position: .init(x: Double(node.x) / 1_000, y: Double(node.y) / 1_000))
        }, connections: [])
    }

    var body: some View {
        let preset = PhotaraGraphPresentationPreset.shipped
        let palette = preset.palette(colorScheme)
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Button("Graph") { showsProjects = false }
                Button("Projects") { showsProjects = true }.disabled(model.projects.count < 2 || model.switching)
                Text(model.projectTitle).font(.headline)
                Spacer()
                if model.checkingStorage { Text("Checking project…") }
                if model.readOnly { Text("Recovery required") }
                else if let projection = model.projection {
                    DisposableAutosaveStatusView(projection: projection, retry: model.retry)
                } else { Text(model.projects.isEmpty ? "Opening the disposable project…" : "Choose a Project to open") }
            }.padding(12)
            if model.switching { Text(model.switchPrompt == nil ? "Preparing Project switch…" : "Waiting for confirmation…").padding(.horizontal, 12) }
            if model.readOnly {
                HStack {
                    Text("This Project is read-only while recovery is required.").foregroundStyle(.orange)
                    Button("Retry Recovery", action: model.retry).disabled(model.switching)
                }.padding(12)
            }
            if let error = model.switchFailure {
                HStack {
                    Text(error).foregroundStyle(.red)
                    if model.switching { Button("Retry Switch", action: model.retrySwitch) }
                }.padding(12)
            }
            Text(model.readOnly
                ? "The recovered Graph is shown read-only until recovery succeeds."
                : "Move nodes to try autosave and undo. Other editing commands are unavailable in this disposable project.")
                .font(.caption).foregroundStyle(.secondary).padding(.horizontal, 12)
            if let error = model.transportFailure {
                HStack {
                    Text(error).foregroundStyle(.red)
                    Button("Retry", action: model.retry).disabled(model.busy)
                }.padding(12)
            }
            if let pending = model.pendingPosition {
                Text("Pending move: \(pending.title) to x \(pending.x), y \(pending.y). This draft has not been acknowledged.")
                    .padding(12).accessibilityIdentifier("controlled-pending-position")
            }
            if showsProjects { projectBrowser }
            else { PhotaraGraphCanvas(toolRailPlacement: .leftTop, controller: controller,
                backgroundStyle: preset.backgroundStyle, backgroundColor: palette.graphBackground?.color,
                minorColor: palette.minor?.color, majorColor: palette.major?.color,
                noodleColor: palette.noodle?.color ?? .accentColor, knifeCursorSize: 24,
                showsToolRail: true, centerScene: { controller.center(positions: [:], selectedNode: nil) },
                addNativeNode: { _ in },
                nodeContent: { node, selected, inputs, outputs in
                    PhotaraGraphNodeView(node: node,
                        presentation: .init(category: .utilitiesControl, iconResource: "node-metadata"),
                        selected: selected, connectedInputs: inputs, connectedOutputs: outputs, preset: preset)
                }) { EmptyView() }
                .allowsHitTesting(model.canEdit)
                .accessibilityIdentifier("controlled-session-graph")
            }
            HStack {
                Button("Undo") { model.action("undo") }.disabled(!model.canEdit || !model.undoAvailable)
                Button("Redo") { model.action("redo") }.disabled(!model.canEdit || !model.redoAvailable)
                Spacer()
                Button("Save Now") { model.action("complete") }.disabled(!model.canEdit)
                Button("Close") { model.close { if $0 { NSApp.terminate(nil) } } }
            }.padding(12)
        }
        .frame(minWidth: 820, minHeight: 540)
        .background(ControlledSwitchConfirmationPresenter(
            confirmation: model.switchPrompt.map { .init(id: $0.id, sourceTitle: $0.sourceTitle, targetTitle: $0.targetTitle) },
            respond: model.answerSwitch).frame(width: 0, height: 0))
        .background(ControlledWindowCloseGuard(model: model).frame(width: 0, height: 0))
        .onAppear {
            controller.configure(.init(portOffset: preset.portOffset, noodleStyle: .curved))
            controller.commitMutation = { mutation in
                if case let .moveNode(id, position) = mutation,
                   let node = model.nodes.first(where: { $0.id == id }),
                   position.x.isFinite, position.y.isFinite,
                   let x = Int64(exactly: (position.x * 1_000).rounded()),
                   let y = Int64(exactly: (position.y * 1_000).rounded()) {
                    model.move(node, x: x, y: y)
                }
                // Canvas retains acknowledged coordinates while the visible
                // pending draft waits for the shared authority's response.
                return document
            }
            synchronize()
        }
        .onChange(of: model.nodes) { synchronize() }
        .onChange(of: model.projects) { if model.activeProjectID == nil { showsProjects = true } }
        .onChange(of: model.activeProjectID) { showsProjects = false; didCenter = false; synchronize() }
        .onChange(of: model.restoredView) { synchronize() }
        .onReceive(NSWorkspace.shared.notificationCenter.publisher(for: NSWorkspace.willSleepNotification)) { _ in
            model.prepareForSleep()
        }
        .onReceive(NSWorkspace.shared.notificationCenter.publisher(for: NSWorkspace.didWakeNotification)) { _ in
            model.revalidateAfterWake()
        }
    }

    private func synchronize() {
        try? controller.synchronizeDocument(document)
        if !didCenter, !model.nodes.isEmpty {
            if let view = model.restoredView, let zoom = Double(view.zoom_ppm) {
                controller.restoreSessionView(pan: .init(width: Double(view.pan_x) / 1_000,
                    height: Double(view.pan_y) / 1_000), zoom: zoom / 1_000_000,
                    selectedNode: view.selected_node_ids.first)
            } else { controller.center(positions: [:], selectedNode: nil) }
            didCenter = true
        }
    }
}
