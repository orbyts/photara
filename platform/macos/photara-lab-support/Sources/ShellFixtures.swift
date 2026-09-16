import AppKit
import SwiftUI

enum ShellScenario: String, CaseIterable, Identifiable {
    case opening, createProject, libraryLifecycle, recentsCollapsed, recentsExpanded, emptyProject, firstNode, selectionCleared
    case assets, layout, review, evaluating, diagnostics, compact, saved, syncing
    var id: String { rawValue }
    var isOpening: Bool { [.opening, .createProject, .recentsCollapsed, .recentsExpanded].contains(self) }
    var title: String {
        switch self {
        case .opening: "Opening"
        case .createProject: "Create Project"
        case .libraryLifecycle: "Library Lifecycle"
        case .recentsCollapsed: "Opening · legacy recents collapsed"
        case .recentsExpanded: "Opening · legacy recents expanded"
        case .emptyProject: "Project / Graph · empty"
        case .firstNode: "Project / Graph · first node"
        case .selectionCleared: "Inspector · selection cleared"
        case .layout: "Node Work Surface · Layout"
        default: rawValue.capitalized
        }
    }
    /// Only scenarios with a Project render the shared module geometry.
    var authorsModuleGeometry: Bool { !isOpening && self != .libraryLifecycle }
    static var browserCases: [Self] { allCases.filter { ![.recentsCollapsed, .recentsExpanded].contains($0) } }

    static let layoutSurface = NodeWorkSurfacePresentation(nodeID: "layout", contributionID: "photara.layout.work-surface",
        title: "Layout", iconResourceID: "photara.layout.compose", accentHex: "#A682CF")
    var presentation: ApplicationPresentation {
        let opening = [.opening, .createProject, .libraryLifecycle, .recentsCollapsed, .recentsExpanded].contains(self)
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
    @Published var fixtureProjectTitle = "Coastal Studies" {
        didSet { presentation.title = fixtureProjectTitle }
    }
    @Published var preset: ApplicationShellPreset {
        didSet {
            guard persistsDraft, let data = try? preset.encoded() else { return }
            draftDefaults.set(data, forKey: Self.draftKey)
        }
    }
    let lifecycle = LibraryLifecycleFixture()
    @Published var dark = false
    @Published var createProjectPresentation: CreateProjectPresentation = .shipped
    @Published var identifiesControls = false
    @Published var lastAction = "Choose a scenario to author the shell."
    @Published private(set) var session = EditorSessionModel(persists: false)
    let assets = GalleryFixtures.assets()
    private static let draftKey = "photara.shell-lab.authoring-draft.v1"
    private let persistsDraft: Bool
    private let draftDefaults: UserDefaults

    init(persistsDraft: Bool = false, draftDefaults: UserDefaults = .standard) {
        self.persistsDraft = persistsDraft
        self.draftDefaults = draftDefaults
        if persistsDraft,
           let data = draftDefaults.data(forKey: Self.draftKey),
           let saved = try? ApplicationShellPreset.decode(data)
        {
            preset = saved
            lastAction = "Restored the automatically saved Shell Lab draft."
        } else {
            preset = .shipped
        }
    }

    func applyPresetToPhotara() {
        do {
            try PhotaraShellDevelopmentSettings.setOverride(preset)
            lastAction = "Applied shared Shell geometry to Photara."
        } catch {
            lastAction = "Could not apply to Photara: \(error.localizedDescription)"
        }
    }

    func removePhotaraOverride() {
        do {
            try PhotaraShellDevelopmentSettings.setOverride(nil)
            lastAction = "Removed the Photara Shell override."
        } catch {
            lastAction = "Could not remove override: \(error.localizedDescription)"
        }
    }

    func restoreShippedPreset() {
        preset = .shipped
        lastAction = "Restored the shipped preset in Shell Lab."
    }
    func reset() {
        lifecycle.configure(.cloud)
        // Each scenario owns a fresh disposable presentation session, including
        // pending catalog requests and selections when both roots are Opening.
        session = EditorSessionModel(persists: false)
        presentation = scenario.presentation
        presentation.title = fixtureProjectTitle
        session.synchronizeProject(id: presentation.projectID, nodeIDs: presentation.nodeIDs)
        session.showsRecentProjects = scenario == .recentsExpanded
        if scenario == .selectionCleared { session.selectedNodeID = nil }
        if scenario == .layout { session.selectedNodeID = "layout"; session.activateWorkSurface(for: "layout") }
        if scenario == .review { session.activateReview() }
    }

    func send(_ action: ApplicationAction) {
        lastAction = String(describing: action)
        switch action {
        case .newProject: scenario = .createProject
        case .openProject, .openRecent: scenario = .assets
        case .closeProject: scenario = .opening
        case .evaluate: presentation.isEvaluating = true; presentation.progressLabel = "Evaluating fixture…"
        case .cancel: presentation.isEvaluating = false
        case .save: presentation.isDirty = false; presentation.syncLabel = "Saved just now"
        case .importPair: presentation.hasAssets = true
        }
    }
    func finishCreateProject(_ draft: CreateProjectDraft) {
        fixtureProjectTitle = draft.trimmedName
        scenario = .emptyProject
        lastAction = "Create \(draft.packageName) in \(draft.destination)"
    }
    func addNode() {
        let id = presentation.nodeIDs.isEmpty ? "disk" : "layout"
        if !presentation.nodeIDs.contains(id) { presentation.nodeIDs.append(id) }
        presentation.nodeCount = presentation.nodeIDs.count
        if id == "layout" { presentation.workSurfaces = [ShellScenario.layoutSurface] }
        presentation.isDirty = true
        session.synchronizeProject(id: presentation.projectID, nodeIDs: presentation.nodeIDs)
        lastAction = "Added \(id) fixture through catalog request"
    }
}

struct ShellLabPreview: View {
    @ObservedObject var model: ShellLabModel
    @ObservedObject var session: EditorSessionModel
    @StateObject private var themeStore: PhotaraThemeStore

    init(model: ShellLabModel, session: EditorSessionModel, usesDevelopmentTheme: Bool = false) {
        self.model = model
        self.session = session
        _themeStore = StateObject(wrappedValue: PhotaraThemeStore(usesDevelopmentOverride: usesDevelopmentTheme))
    }

    var body: some View {
        let appearance: PhotaraThemeAppearance = model.dark ? .dark : .light
        let theme = themeStore.document.resolved(for: appearance)
        Group {
        if model.scenario == .libraryLifecycle {
            LibraryLifecycleLabView(model: model.lifecycle)
        } else {
        ApplicationShell(presentation: model.presentation, actions: .init(send: model.send), preset: model.preset,
                workSurface: { surface in
                    AnyView(LayoutAuthoringSurfaceView(presentation: .init(node: InspectorFixture.layout.presentation.node),
                        actions: .init(requestPreviews: { _ in }, cell: { id, _, _, _ in model.lastAction = "Layout \(id)" })))
                }) { id in
                switch id {
                case .people: PeopleView(presentation: LibraryFixtures.presentation(.populated), actions: .init(send: { model.lastAction = String(describing: $0) }))
                case .locations: LocationsView(presentation: LibraryFixtures.presentation(.populated), actions: .init(send: { model.lastAction = String(describing: $0) }))
                case .scenes: ScenesView(presentation: LibraryFixtures.presentation(.populated), actions: .init(send: { model.lastAction = String(describing: $0) }))
                case .projectInfo: ProjectInfoView(presentation: .init(title: model.presentation.title, revision: 1, assignments: LibraryFixtures.assignments, library: LibraryFixtures.presentation(.populated), phase: .ready, hasProject: true), actions: .init(send: { model.lastAction = String(describing: $0) }))
                case .account: LibrarySyncView()
                case .graph: ShellFixtureGraph(model: model, session: session).id(model.scenario)
                case .assetGallery:
                    AssetGalleryView(presentation: .init(assets: model.presentation.hasAssets ? model.assets : [],
                        canAssign: false, hasSourceNodes: model.presentation.hasAssetProducingContext),
                        actions: .init(open: { model.lastAction = "Open \($0)" }, assign: { model.lastAction = "Assign \($0)" },
                            requestPreview: { _ in }, addSourceNode: { session.activateGraph(); session.requestNodeMenu() },
                            runWorkflow: { model.send(.evaluate) }),
                        filter: $session.galleryFilter, selectedAssetID: $session.selectedAssetID)
                case .inspector:
                    InspectorView(presentation: inspection, actions: .init(
                        chooseFolder: { model.lastAction = "Choose folder \($0)" }, scanDisk: { model.lastAction = "Scan \($0)" },
                        connectDisk: { model.lastAction = "Connect \($0)" }, structure: { id, _ in model.lastAction = "Structure \(id)" },
                        cell: { id, _, _, _ in model.lastAction = "Cell \(id)" },
                        showGraph: { session.activateGraph() }))
                case .nodeWorkSurface: EmptyView() // Hosted by the fixture contribution adapter.
                case .diagnostics:
                    DiagnosticsView(diagnostics: model.presentation.diagnosticCount > 0
                        ? [.init(code: "source.offline", message: "Reconnect the source folder to continue."),
                           .init(code: "asset.unavailable", message: "One source image needs to be located.")] : [])
                }
            }.id(model.scenario)
            .sheet(isPresented: Binding(
                get: { model.scenario == .createProject },
                set: { if !$0 && model.scenario == .createProject { model.scenario = .opening } }
            )) {
                CreateProjectView(
                    libraryName: ReleaseConfiguration.current.identity.defaultLibraryName,
                    isCloudLibrary: true,
                    chooseDestination: { model.lastAction = "Choose package destination" },
                    cancel: { model.scenario = .opening },
                    create: { model.finishCreateProject($0) },
                    presentation: model.createProjectPresentation
                )
            }
        }
        }
            .environmentObject(session)
            .environment(\.photaraTheme, theme)
            .preferredColorScheme(model.dark ? .dark : .light)
            .overlay {
                if model.identifiesControls {
                    ShellControlInspectorOverlay(preset: model.preset,
                                                 hasProject: model.presentation.hasOpenProject)
                }
            }
    }
    private var inspection: InspectorPresentation {
        guard let id = session.selectedNodeID else { return .init(node: nil) }
        return id == "layout" ? InspectorFixture.layout.presentation : InspectorFixture.disk.presentation
    }
}

private struct ShellControlInspectorOverlay: View {
    let preset: ApplicationShellPreset
    let hasProject: Bool
    @State private var location: CGPoint?

    var body: some View {
        GeometryReader { geometry in
            ZStack(alignment: .topLeading) {
                Color.clear
                    .contentShape(Rectangle())
                    .onContinuousHover { phase in
                        switch phase {
                        case let .active(point): location = point
                        case .ended: location = nil
                        }
                    }
                    .onHover { hovering in
                        if hovering { NSCursor.crosshair.push() } else { NSCursor.pop() }
                    }

                if let location {
                    Image(systemName: "scope")
                        .font(.system(size: 22, weight: .medium))
                        .foregroundStyle(.tint)
                        .position(location)
                        .allowsHitTesting(false)

                    Text(controlName(at: location, in: geometry.size))
                        .font(.system(size: 12, weight: .medium))
                        .multilineTextAlignment(.leading)
                        .padding(.horizontal, 10)
                        .padding(.vertical, 7)
                        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 8))
                        .overlay {
                            RoundedRectangle(cornerRadius: 8)
                                .strokeBorder(.primary.opacity(0.14), lineWidth: 1)
                        }
                        .fixedSize(horizontal: true, vertical: true)
                        .position(labelPosition(for: location, in: geometry.size))
                        .allowsHitTesting(false)
                }
            }
        }
        .accessibilityLabel("Shell control identifier")
    }

    private func controlName(at point: CGPoint, in size: CGSize) -> String {
        if !hasProject {
            return launcherControlName(at: point, in: size)
        }

        let outer = preset.frame.outerInset
        let statusTop = size.height - outer - preset.statusBarHeight - preset.frame.gutter / 2
        if point.y >= statusTop {
            return "Editor Chrome → Status bar height, text size, inset and spacing\nTheme Lab → Primary"
        }

        let nearOuterEdge = point.x < outer + preset.frame.gutter
            || point.x > size.width - outer - preset.frame.gutter
            || point.y < outer + preset.frame.gutter / 2
        let leadingDivider = outer + preset.leadingIdealWidth
        let trailingDivider = size.width - outer - preset.trailingIdealWidth
        let nearDivider = abs(point.x - leadingDivider) < max(10, preset.frame.gutter)
            || abs(point.x - trailingDivider) < max(10, preset.frame.gutter)
        if nearOuterEdge || nearDivider {
            return "Theme Lab → Foundation\nShared geometry → Gutter and Outer inset"
        }

        let moduleTop = outer + preset.frame.gutter / 2
        if point.y <= moduleTop + preset.panelHeaderHeight {
            return "Advanced Shared Geometry → Module title bar metrics\nTheme Lab → Selection"
        }

        let nearCardEdge = point.y < moduleTop + preset.panelHeaderHeight + preset.frame.contentInset + 10
        if nearCardEdge {
            return "Theme Lab → Primary and Inset\nShared geometry → Corner radius and Content inset"
        }

        return "Shared geometry → Content inset\nFeature Lab → Content inside this module"
    }

    private func launcherControlName(at point: CGPoint, in size: CGSize) -> String {
        if point.x < 238 { return "macOS → Native sidebar, selection and account controls" }
        return "macOS → Native window and text colors\nOpening → Production content and native actions"
    }

    private func labelPosition(for point: CGPoint, in size: CGSize) -> CGPoint {
        let estimatedWidth = 390.0
        let proposedX = point.x + estimatedWidth / 2 + 24 < size.width
            ? point.x + estimatedWidth / 2 + 24
            : point.x - estimatedWidth / 2 - 24
        let x = max(estimatedWidth / 2 + 8,
                    min(proposedX, size.width - estimatedWidth / 2 - 8))
        let y = min(max(point.y + 44, 44), size.height - 44)
        return CGPoint(x: x, y: y)
    }
}

/// Fixture adapter composes the unchanged production Graph views and shipped preset.
private struct ShellFixtureGraph: View {
    @ObservedObject var model: ShellLabModel
    @ObservedObject var session: EditorSessionModel
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
            .onAppear { controller.configure(.init(portOffset: preset.portOffset, noodleStyle: .curved)); synchronize(); if session.consumeNodeMenuRequest() { catalog = true } }
            .onChange(of: model.presentation.nodeIDs) { synchronize() }
            .onChange(of: session.nodeMenuRequest) { if session.consumeNodeMenuRequest() { catalog = true } }
            .onChange(of: controller.selection) {
                if case .node(let id) = controller.selection { session.selectedNodeID = id }
                else { session.selectedNodeID = nil }
            }
    }
    private func center() { controller.center(positions: [:], selectedNode: session.selectedNodeID) }
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
