import AppKit
import SwiftUI

enum ShellScenario: String, CaseIterable, Identifiable {
    case opening, recentsCollapsed, recentsExpanded, emptyProject, firstNode, selectionCleared
    case assets, layout, review, evaluating, diagnostics, compact, saved, syncing
    var id: String { rawValue }
    static let layoutSurface = NodeWorkSurfacePresentation(nodeID: "layout", contributionID: "photara.layout.work-surface",
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
    @Published var fixtureProjectTitle = "Coastal Studies" {
        didSet { presentation.title = fixtureProjectTitle }
    }
    @Published var preset: ApplicationShellPreset {
        didSet {
            guard persistsDraft, let data = try? preset.encoded() else { return }
            draftDefaults.set(data, forKey: Self.draftKey)
        }
    }
    @Published var themeDocument: PhotaraThemeDocument {
        didSet {
            guard persistsDraft, (try? themeDocument.validate()) != nil,
                  let data = try? JSONEncoder().encode(themeDocument) else { return }
            draftDefaults.set(data, forKey: Self.themeDraftKey)
        }
    }
    @Published var dark = false
    @Published var identifiesControls = false
    @Published var lastAction = "Choose a scenario to author the shell."
    let session = EditorSessionModel(persists: false)
    let assets = GalleryFixtures.assets()
    private static let draftKey = "photara.shell-lab.authoring-draft.v1"
    private static let themeDraftKey = "photara.shell-lab.theme-draft.v1"
    private let persistsDraft: Bool
    private let draftDefaults: UserDefaults

    init(persistsDraft: Bool = false, draftDefaults: UserDefaults = .standard) {
        self.persistsDraft = persistsDraft
        self.draftDefaults = draftDefaults
        let shippedTheme = Self.loadShippedTheme()
        if persistsDraft, let data = draftDefaults.data(forKey: Self.themeDraftKey),
           let saved = try? JSONDecoder().decode(PhotaraThemeDocument.self, from: data),
           (try? saved.validate()) != nil {
            themeDocument = saved
        } else {
            themeDocument = shippedTheme
        }
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
            let root = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
                .appending(path: "Photara/Developer", directoryHint: .isDirectory)
            try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
            let url = root.appending(path: "ShellLabTheme.json")
            try themeDocument.write(to: url)
            PhotaraThemeDevelopmentSettings.setOverrideURL(url)
            lastAction = "Applied Shell and shared Theme to Photara. The open app updates automatically."
        } catch {
            lastAction = "Could not apply to Photara: \(error.localizedDescription)"
        }
    }

    func removePhotaraOverride() {
        do {
            try PhotaraShellDevelopmentSettings.setOverride(nil)
            PhotaraThemeDevelopmentSettings.setOverrideURL(nil)
            lastAction = "Removed the Photara Shell and Theme overrides."
        } catch {
            lastAction = "Could not remove override: \(error.localizedDescription)"
        }
    }

    func restoreShippedPreset() {
        preset = .shipped
        themeDocument = Self.loadShippedTheme()
        lastAction = "Restored the shipped preset in Shell Lab."
    }
    func reset() {
        session.synchronizeProject(id: nil, nodeIDs: [])
        session.restoreLayoutAuthoringPreset()
        presentation = scenario.presentation
        presentation.title = fixtureProjectTitle
        session.synchronizeProject(id: presentation.projectID, nodeIDs: presentation.nodeIDs)
        session.showsRecentProjects = scenario == .recentsExpanded
        if scenario == .selectionCleared { session.selectedNodeID = nil }
        if scenario == .layout { session.selectedNodeID = "layout"; session.activateWorkSurface(for: "layout") }
        if scenario == .review { session.activateReview() }
    }

    private static func loadShippedTheme() -> PhotaraThemeDocument {
        guard let url = Bundle.main.url(forResource: "photara-default", withExtension: "json"),
              let theme = try? PhotaraThemeDocument.load(from: url)
        else { fatalError("Shell Lab is missing the shipped Theme") }
        return theme
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
        session.synchronizeProject(id: presentation.projectID, nodeIDs: presentation.nodeIDs)
        lastAction = "Added \(id) fixture through catalog request"
    }
}

struct ShellLabPreview: View {
    @ObservedObject var model: ShellLabModel
    @ObservedObject var session: EditorSessionModel
    var body: some View {
        let appearance: PhotaraThemeAppearance = model.dark ? .dark : .light
        let theme = model.themeDocument.resolved(for: appearance)
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
            }.environmentObject(session)
            .environment(\.photaraTheme, theme)
            .tint(theme.color(.borderFocus))
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
            return "Editor Chrome → Status bar height, text size, inset and spacing\nProject Chrome Colors → Header and status background"
        }

        let nearOuterEdge = point.x < outer + preset.frame.gutter
            || point.x > size.width - outer - preset.frame.gutter
            || point.y < outer + preset.frame.gutter / 2
        let leadingDivider = outer + preset.leadingIdealWidth
        let trailingDivider = size.width - outer - preset.trailingIdealWidth
        let nearDivider = abs(point.x - leadingDivider) < max(10, preset.frame.gutter)
            || abs(point.x - trailingDivider) < max(10, preset.frame.gutter)
        if nearOuterEdge || nearDivider {
            return "Shared Visual System → Application base\nShared Visual System → Gutter and Outer inset"
        }

        let moduleTop = outer + preset.frame.gutter / 2
        if point.y <= moduleTop + preset.panelHeaderHeight {
            return "Advanced Shared Geometry → Module title bar metrics\nShared Visual System → Selection tint"
        }

        let nearCardEdge = point.y < moduleTop + preset.panelHeaderHeight + preset.frame.contentInset + 10
        if nearCardEdge {
            return "Shared Visual System → Module base and Module content\nShared Visual System → Corner radius and Content inset"
        }

        return "Shared Visual System → Module base and Content inset\nFeature Lab → Content inside this module"
    }

    private func launcherControlName(at point: CGPoint, in size: CGSize) -> String {
        let center = CGPoint(x: size.width / 2, y: size.height / 2 + preset.launcherVerticalOffset)
        if hypot(point.x - center.x, point.y - (center.y - 105)) < 95 {
            return "Hero Icon Tile → Size, color, stroke, corner radius, glow and position"
        }
        if abs(point.y - center.y) < 55 {
            return "Launcher → Title size, font, weight and spacing"
        }
        if point.y > center.y + 45 && point.y < center.y + 145 {
            return "Launcher Buttons → Create, Open and Recent tints\nLauncher → Hero-to-recents gap"
        }
        return "Launcher Background → Material and Tint\nLauncher → Edge insets and vertical position"
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
