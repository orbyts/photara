import AppKit
import SwiftUI

/// The normal shell and editor, backed only by the registered disposable SQL
/// context and shared Project session. There is no legacy Application facade.
struct LocalLibrarySessionScene: Scene {
    let delegate: ControlledSessionApplicationDelegate
    @ObservedObject var model: DisposableAutosaveProcess
    var body: some Scene {
        Window("Photara — \(model.libraryTitle)", id: "local-libraries") {
            LocalLibrarySessionView(model: model)
                .task { delegate.model = model; model.start() }
        }
        .windowToolbarStyle(.unified)
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("Close Window") { model.close { if $0 { NSApp.terminate(nil) } } }.keyboardShortcut("w")
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

private struct LocalLibrarySessionView: View {
    @ObservedObject var model: DisposableAutosaveProcess
    @StateObject private var editor = EditorSessionModel(persists: false)
    @Environment(\.colorScheme) private var colorScheme
    @State private var controller = PhotaraGraphInteractionController(document: .init(nodes: [], connections: []))
    @State private var didCenter = false
    @State private var showsProjects = true
    @State private var selectedProject: UUID?
    @State private var nameRequest: LocalLibraryNameRequest?

    private var preset: ApplicationShellPreset {
        var value = ApplicationShellPreset.shipped
        value.toolbarApplicationTitle = "Photara · Disposable"
        return value
    }
    private var presentation: ApplicationPresentation {
        .init(hasOpenProject: model.activeProjectID != nil || model.readOnly,
            title: model.projectTitle, subtitle: model.libraryTitle,
            nodeCount: model.nodes.count, projectID: model.activeProjectID?.uuidString,
            nodeIDs: model.nodes.map(\.id), surfaceContext: "Disposable local Library", canAuthorProject: false)
    }
    private var libraryActionsEnabled: Bool {
        !model.switching && !model.readOnly && !model.libraryWorking && model.libraryRequestDescription == nil
    }
    private var document: PhotaraGraphDocument {
        .init(nodes: model.nodes.map { node in
            .init(id: node.id, kind: "controlled-position", title: node.title,
                subtitle: "x \(node.x) · y \(node.y)", ports: [],
                position: .init(x: Double(node.x) / 1_000, y: Double(node.y) / 1_000))
        }, connections: [])
    }

    var body: some View {
        ApplicationShell(presentation: presentation,
            actions: .init(send: shellAction), preset: preset,
            localNavigation: .init(account: AnyView(accountMenu), opening: AnyView(opening),
                browseProjects: { showsProjects = true }, showGraph: { showsProjects = false }),
            workSurface: { _ in AnyView(EmptyView()) }) { panel in
                if panel == .graph { graphPanel }
                else if panel == .account { opening }
                else { Text("This command is unavailable in the disposable session.").foregroundStyle(.secondary) }
            }
            .environmentObject(editor)
            .background(ControlledWindowCloseGuard(model: model).frame(width: 0, height: 0))
            .background(ControlledSwitchConfirmationPresenter(
                confirmation: model.switchPrompt.map { .init(id: $0.id, sourceTitle: $0.sourceTitle, targetTitle: $0.targetTitle) },
                respond: model.answerSwitch).frame(width: 0, height: 0))
            .sheet(item: $nameRequest) { request in
                LocalLibraryNameSheet(request: request, cancel: { nameRequest = nil }) { mode, name in
                    switch mode {
                    case let .create(id): model.createLibrary(id: id, name: name)
                    case let .rename(library): model.renameLibrary(library, name: name)
                    }
                    nameRequest = nil
                }
            }
            .onAppear {
                controller.configure(.init(portOffset: PhotaraGraphPresentationPreset.shipped.portOffset, noodleStyle: .curved))
                controller.commitMutation = { mutation in
                    if case let .moveNode(id, position) = mutation,
                       let node = model.nodes.first(where: { $0.id == id }), position.x.isFinite, position.y.isFinite,
                       let x = Int64(exactly: (position.x * 1_000).rounded()),
                       let y = Int64(exactly: (position.y * 1_000).rounded()) { model.move(node, x: x, y: y) }
                    return document
                }
                synchronize()
            }
            .onChange(of: model.nodes) { synchronize() }
            .onChange(of: model.activeProjectID) {
                showsProjects = model.activeProjectID == nil; didCenter = false; synchronize()
            }
            .onChange(of: model.selectedLibraryID) { selectedProject = nil; showsProjects = model.activeProjectID == nil }
            .onChange(of: model.restoredView) { synchronize() }
            .onReceive(NSWorkspace.shared.notificationCenter.publisher(for: NSWorkspace.willSleepNotification)) { _ in model.prepareForSleep() }
            .onReceive(NSWorkspace.shared.notificationCenter.publisher(for: NSWorkspace.didWakeNotification)) { _ in model.revalidateAfterWake() }
    }

    private var accountMenu: some View {
        Menu {
            Text("Disposable local Libraries")
            ForEach(model.libraries) { library in
                Button {
                    controller.cancel(resetTool: true)
                    model.selectLibrary(library.id, view: currentView())
                } label: {
                    if library.id == model.selectedLibraryID { Label(library.name, systemImage: "checkmark") }
                    else { Text(library.name) }
                }.disabled(!libraryActionsEnabled)
            }
            Divider()
            Button("Create Library…") {
                if let id = model.creatableLibraryIDs.first { nameRequest = .init(mode: .create(id)) }
            }
                .disabled(!libraryActionsEnabled || model.busy || model.creatableLibraryIDs.isEmpty)
            if let library = model.libraries.first(where: { $0.id == model.selectedLibraryID }) {
                Button("Rename Library…") { nameRequest = .init(mode: .rename(library)) }
                    .disabled(!libraryActionsEnabled || model.busy)
            }
        } label: {
            HStack { AccountAvatar(profile: nil); Text(model.libraryTitle).lineLimit(1); Spacer(); Image(systemName: "chevron.down") }
        }
        .menuStyle(.borderlessButton)
        .accessibilityLabel("Libraries")
        .accessibilityIdentifier("local-library-avatar")
    }

    private var opening: some View {
        HStack(spacing: 0) {
            VStack(alignment: .leading, spacing: 16) {
                Text(model.libraryTitle).font(.headline)
                Label("Projects", systemImage: "folder")
                Spacer()
                Text("Disposable local workspace").font(.caption).foregroundStyle(.secondary)
                accountMenu
            }.padding(16).frame(width: 230)
            Divider()
            VStack(spacing: 0) { status; projectBrowser }.frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }
    private var projectBrowser: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Projects").font(.title2)
            if model.selectedLibraryID == nil { Text("Choose a Library from the avatar menu.").foregroundStyle(.secondary) }
            List(model.projects, selection: $selectedProject) { project in
                HStack {
                    Image(systemName: "doc.richtext"); Text(project.title); Spacer()
                    if project.id == model.activeProjectID { Text("Open").foregroundStyle(.secondary) }
                }.contentShape(Rectangle()).onTapGesture(count: 2) { openProject(project.id) }.tag(project.id)
            }
            if let project = model.projects.first(where: { $0.id == selectedProject }) {
                HStack { Text(project.title); Spacer(); Button(project.id == model.activeProjectID ? "Show Graph" : "Open Project") { openProject(project.id) } }
            } else { Text("Select a Project to preview it. Double-click to open.").foregroundStyle(.secondary) }
        }.padding(16).disabled(!libraryActionsEnabled)
    }
    private var graphPanel: some View {
        VStack(spacing: 0) {
            status
            if showsProjects { projectBrowser }
            else { graph }
        }
    }
    private var graph: some View {
        let preset = PhotaraGraphPresentationPreset.shipped
        let palette = preset.palette(colorScheme)
        return VStack(spacing: 0) {
            Text(model.readOnly ? "The recovered Graph is read-only until recovery succeeds." : "Changes save automatically. Move nodes to edit; other authoring commands are unavailable in this disposable Project.")
                .font(.caption).foregroundStyle(.secondary).padding(8)
            PhotaraGraphCanvas(toolRailPlacement: .leftTop, controller: controller,
                backgroundStyle: preset.backgroundStyle, backgroundColor: palette.graphBackground?.color,
                minorColor: palette.minor?.color, majorColor: palette.major?.color,
                noodleColor: palette.noodle?.color ?? .accentColor, knifeCursorSize: 24,
                showsToolRail: false, centerScene: { controller.center(positions: [:], selectedNode: nil) },
                addNativeNode: { _ in }, nodeContent: { node, selected, inputs, outputs in
                    PhotaraGraphNodeView(node: node, presentation: .init(category: .utilitiesControl, iconResource: "node-metadata"),
                        selected: selected, connectedInputs: inputs, connectedOutputs: outputs, preset: preset)
                }) { EmptyView() }
                .allowsHitTesting(model.canEdit).accessibilityIdentifier("local-library-graph")
        }
    }
    private var status: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                if model.readOnly { Text("Recovery required"); Button("Retry Recovery", action: model.retry).disabled(model.switching) }
                else if let projection = model.projection { DisposableAutosaveStatusView(projection: projection, retry: model.retry) }
                else { Text(model.selectedLibraryID == nil ? "Local Libraries" : "Choose a Project to open") }
                Spacer()
                if model.checkingStorage { Text("Checking Project…") }
                if model.switching { Text(model.switchPrompt == nil ? "Preparing selection…" : "Waiting for confirmation…") }
            }
            if let error = model.transportFailure { HStack { Text(error); Button("Retry", action: model.retry).disabled(model.busy) }.foregroundStyle(.red) }
            if let error = model.switchFailure { HStack { Text(error); if model.switching { Button("Retry Selection", action: model.retrySwitch) } }.foregroundStyle(.red) }
            if let error = model.libraryFailure { HStack { Text(error); if model.libraryRequestDescription != nil { Button("Retry Library Request", action: model.retry).disabled(model.libraryWorking) } }.foregroundStyle(.red) }
            if let pending = model.libraryRequestDescription { Text(pending).font(.caption) }
            if let pending = model.pendingPosition {
                Text("Pending move: \(pending.title) to x \(pending.x), y \(pending.y). This draft has not been acknowledged.")
                    .accessibilityIdentifier("controlled-pending-position")
            }
        }.padding(12)
    }
    private func shellAction(_ action: ApplicationAction) {
        switch action {
        case .save: model.action("complete")
        case .closeProject:
            controller.cancel(resetTool: true)
            model.closeProject(view: currentView())
        case .openProject: showsProjects = true
        default: break // Unsupported authoring is visibly unavailable in this route.
        }
    }
    private func openProject(_ id: UUID) {
        if id == model.activeProjectID { showsProjects = false; return }
        controller.cancel(resetTool: true)
        model.beginSwitch(to: id, view: currentView())
    }
    private func currentView() -> DisposableSessionView? {
        var view = model.restoredView
        if let x = Int64(exactly: (controller.camera.pan.width * 1_000).rounded()),
           let y = Int64(exactly: (controller.camera.pan.height * 1_000).rounded()),
           let zoom = UInt64(exactly: (controller.camera.zoom * 1_000_000).rounded()) {
            view?.pan_x = x; view?.pan_y = y; view?.zoom_ppm = String(zoom); view?.visible_panels = ["graph"]
            if case let .node(id) = controller.selection { view?.selected_node_ids = [id] }
            else { view?.selected_node_ids = [] }
        }
        return view
    }
    private func synchronize() {
        try? controller.synchronizeDocument(document)
        if !didCenter, !model.nodes.isEmpty {
            if let view = model.restoredView, let zoom = Double(view.zoom_ppm) {
                controller.restoreSessionView(pan: .init(width: Double(view.pan_x) / 1_000,
                    height: Double(view.pan_y) / 1_000), zoom: zoom / 1_000_000, selectedNode: view.selected_node_ids.first)
            } else { controller.center(positions: [:], selectedNode: nil) }
            didCenter = true
        }
    }
}
