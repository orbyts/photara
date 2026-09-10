import SwiftUI

enum ShellScenario: String, CaseIterable, Identifiable {
    case opening, recentsCollapsed, recentsExpanded, emptyProject, firstNode, selectionCleared
    case assets, layout, review, evaluating, diagnostics, compact, saved, syncing
    var id: String { rawValue }
    static let layoutSurface = NodeWorkSurfacePresentation(nodeID: "layout", contributionID: "photara.layout.workspace",
        title: "Layout", iconResourceID: "photara.layout.compose", accentHex: "#A682CF")
    var presentation: ApplicationPresentation {
        let opening = [.opening, .recentsCollapsed, .recentsExpanded].contains(self)
        let empty = self == .emptyProject
        let layout = [.layout, .review].contains(self)
        let nodes = opening || empty ? [] : layout ? ["disk", "layout"] : ["disk"]
        return .init(hasOpenProject: !opening, title: "Coastal Studies", subtitle: "",
            isDirty: [.firstNode, .layout].contains(self), nodeCount: nodes.count,
            diagnosticCount: self == .diagnostics ? 2 : 0,
            progressLabel: self == .evaluating ? "Evaluating workflow · 1 of 2 nodes" : "Idle",
            isEvaluating: self == .evaluating,
            workSurfaces: layout ? [Self.layoutSurface] : [],
            projectID: opening ? nil : "shell-fixture", nodeIDs: nodes,
            hasAssets: [.assets, .layout, .review, .compact].contains(self),
            hasReviewableResult: self == .review,
            syncLabel: self == .saved ? "Saved just now" : self == .syncing ? "Syncing changes…" : nil,
            surfaceContext: self == .review ? "3 images ready to review" : nil,
            recentProjects: [.recentsCollapsed, .recentsExpanded].contains(self)
                ? [.init(id: "coastal", title: "Coastal Studies", lastOpened: Date(timeIntervalSince1970: 1_789_000_000)),
                   .init(id: "editorial", title: "Editorial Portraits", lastOpened: Date(timeIntervalSince1970: 1_788_900_000))] : [])
    }
}

@MainActor
final class ShellLabModel: ObservableObject {
    @Published var scenario: ShellScenario = .opening { didSet { reset() } }
    @Published var presentation = ShellScenario.opening.presentation
    @Published var preset = ApplicationShellPreset.shipped
    @Published var dark = false
    @Published var lastAction = "Choose a scenario to author the shell."
    let workspace = WorkspaceModel(persists: false)
    let assets = GalleryFixtures.assets()
    func reset() {
        workspace.synchronizeProject(id: nil, nodeIDs: [])
        workspace.restoreLayoutAuthoringPreset()
        presentation = scenario.presentation
        workspace.synchronizeProject(id: presentation.projectID, nodeIDs: presentation.nodeIDs)
        workspace.showsRecentProjects = scenario == .recentsExpanded
        if scenario == .selectionCleared { workspace.selectedNodeID = nil }
        if scenario == .layout { workspace.selectedNodeID = "layout"; workspace.activateWorkspace(for: "layout") }
        if scenario == .review { workspace.activateReview() }
    }
    func send(_ action: ApplicationAction) {
        lastAction = String(describing: action)
        switch action {
        case .newProject: scenario = .emptyProject
        case .openProject, .openRecent: scenario = .assets
        case .closeProject: scenario = .opening
        case .evaluate: presentation.isEvaluating = true; presentation.progressLabel = "Evaluating fixture…"
        case .cancel: presentation.isEvaluating = false
        case .save: presentation.isDirty = false; presentation.syncLabel = "Saved just now"
        case .importPair: presentation.hasAssets = true
        }
    }
    func addNode() {
        let id = presentation.nodeIDs.isEmpty ? "disk" : "layout"
        if !presentation.nodeIDs.contains(id) { presentation.nodeIDs.append(id) }
        presentation.nodeCount = presentation.nodeIDs.count
        if id == "layout" { presentation.workSurfaces = [ShellScenario.layoutSurface] }
        presentation.isDirty = true
        workspace.synchronizeProject(id: presentation.projectID, nodeIDs: presentation.nodeIDs)
        lastAction = "Added \(id) fixture through catalog request"
    }
}

struct ShellLabPreview: View {
    @ObservedObject var model: ShellLabModel
    @ObservedObject var workspace: WorkspaceModel
    var body: some View {
        LabAppearance(dark: model.dark) {
            ApplicationShell(presentation: model.presentation, actions: .init(send: model.send), preset: model.preset,
                workSurface: { surface in
                    AnyView(LayoutAuthoringSurfaceView(presentation: .init(node: InspectorFixture.layout.presentation.node),
                        actions: .init(requestPreviews: { _ in }, cell: { id, _, _, _ in model.lastAction = "Layout \(id)" })))
                }) { id in
                switch id {
                case .graph: ShellFixtureGraph(model: model, workspace: workspace).id(model.scenario)
                case .assetGallery:
                    AssetGalleryView(presentation: .init(assets: model.presentation.hasAssets ? model.assets : [], canAssign: false),
                        actions: .init(open: { model.lastAction = "Open \($0)" }, assign: { model.lastAction = "Assign \($0)" }, requestPreview: { _ in }),
                        filter: $workspace.galleryFilter, selectedAssetID: $workspace.selectedAssetID)
                case .inspector:
                    InspectorView(presentation: inspection, actions: .init(
                        chooseFolder: { model.lastAction = "Choose folder \($0)" }, scanDisk: { model.lastAction = "Scan \($0)" },
                        connectDisk: { model.lastAction = "Connect \($0)" }, structure: { id, _ in model.lastAction = "Structure \(id)" },
                        cell: { id, _, _, _ in model.lastAction = "Cell \(id)" }))
                case .nodeWorkSurface: EmptyView() // Hosted by the fixture contribution adapter.
                case .diagnostics:
                    DiagnosticsView(diagnostics: model.presentation.diagnosticCount > 0
                        ? [.init(code: "source.offline", message: "Reconnect the source folder to continue."),
                           .init(code: "asset.unavailable", message: "One source image needs to be located.")] : [])
                }
            }.environmentObject(workspace)
        }
    }
    private var inspection: InspectorPresentation {
        guard let id = workspace.selectedNodeID else { return .init(node: nil) }
        return id == "layout" ? InspectorFixture.layout.presentation : InspectorFixture.disk.presentation
    }
}

/// Fixture adapter composes the unchanged production Graph views and shipped preset.
private struct ShellFixtureGraph: View {
    @ObservedObject var model: ShellLabModel
    @ObservedObject var workspace: WorkspaceModel
    @Environment(\.colorScheme) private var colorScheme
    @State private var controller = PhotaraGraphInteractionController(document: .init(nodes: [], connections: []))
    @State private var catalog = false
    private let preset = PhotaraGraphPresentationPreset.shipped
    var body: some View {
        let palette = preset.palette(colorScheme)
        PhotaraGraphCanvas(toolRailPlacement: .leftTop, controller: controller,
            backgroundStyle: preset.backgroundStyle, backgroundColor: palette.graphBackground?.color,
            minorColor: palette.minor?.color, majorColor: palette.major?.color,
            noodleColor: palette.noodle?.color ?? .accentColor, knifeCursorSize: 24,
            showsToolRail: true, centerScene: center, addNativeNode: { _ in model.addNode() },
            nodeContent: { node, selected, inputs, outputs in
                PhotaraGraphNodeView(node: node, presentation: .init(category: .utilitiesControl,
                    iconResource: node.id == "disk" ? "node-disk-folder" : "node-layout"),
                    selected: selected, connectedInputs: inputs, connectedOutputs: outputs, preset: preset)
            }) {
                PhotaraGraphToolRail(controller: controller, placement: .leftTop, preset: preset,
                    shortcuts: [], centerScene: center, addNode: { _ in model.addNode() }, showCatalog: { catalog = true })
            }
            .popover(isPresented: $catalog) {
                Button("Add fixture node") { model.addNode(); catalog = false }.padding(20)
            }
            .onAppear { controller.configure(.init(portOffset: preset.portOffset, noodleStyle: .curved)); synchronize(); if workspace.consumeNodeMenuRequest() { catalog = true } }
            .onChange(of: model.presentation.nodeIDs) { synchronize() }
            .onChange(of: workspace.nodeMenuRequest) { if workspace.consumeNodeMenuRequest() { catalog = true } }
            .onChange(of: controller.selection) {
                if case .node(let id) = controller.selection { workspace.selectedNodeID = id }
                else { workspace.selectedNodeID = nil }
            }
    }
    private func center() { controller.center(positions: [:], selectedNode: workspace.selectedNodeID) }
    private func synchronize() {
        let nodes = model.presentation.nodeIDs.enumerated().map { index, id in
            PhotaraGraphNode(id: id, kind: id, title: id == "disk" ? "Disk Folder" : "Layout", subtitle: "",
                ports: [.init(id: "assets", direction: id == "disk" ? .output : .input, dataType: "photara.asset-set", label: "Assets")],
                position: .init(x: Double(index) * 260 - 130, y: -70))
        }
        let edges: [PhotaraGraphConnection] = nodes.count < 2 ? [] : [.init(id: "assets-to-layout",
            source: .init(node: "disk", key: "assets"), destination: .init(node: "layout", key: "assets"))]
        try? controller.synchronizeDocument(.init(nodes: nodes, connections: edges))
        center()
    }
}
