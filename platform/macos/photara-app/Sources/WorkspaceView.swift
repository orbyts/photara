import SwiftUI

struct WorkspaceView: View {
    @EnvironmentObject private var app: AppModel
    @EnvironmentObject private var workspace: WorkspaceModel

    var body: some View {
        Group {
            if app.hasOpenProject {
                VStack(spacing: 0) {
                    ProjectCommandBar()
                    Divider()
                    HSplitView {
                        region(.leading, minimumWidth: 230, idealWidth: 280)
                        region(.content, minimumWidth: 420, idealWidth: 680)
                        region(.trailing, minimumWidth: 280, idealWidth: 360)
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    Divider()
                    ProjectStatusBar()
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                ProjectLauncherView()
            }
        }
        .frame(minWidth: 980, minHeight: 620)
        .toolbar {
            ToolbarItemGroup {
                Button("New", systemImage: "doc.badge.plus") { app.newProject() }
                Button("Open", systemImage: "folder") { app.chooseAndOpenProject() }
                Button("Close", systemImage: "xmark.square") { app.closeProject() }
                    .disabled(!app.hasOpenProject)
                Button("Import Pair", systemImage: "photo.badge.plus") {
                    app.chooseAndImportTiffPair()
                }
                .disabled(!app.hasOpenProject)
                Button("Add Node", systemImage: "square.grid.2x2") {
                    workspace.requestNodeMenu()
                }
                Button("Save", systemImage: "square.and.arrow.down") {
                    app.save()
                }
                Button("Evaluate", systemImage: "play.fill") {
                    app.evaluate()
                }
                .disabled(app.isEvaluating)
                Button("Cancel", systemImage: "stop.fill") {
                    app.cancelEvaluation()
                }
                .disabled(!app.isEvaluating)
                panelsMenu
            }
        }
        .alert(
            "Photara",
            isPresented: Binding(
                get: { app.presentedError != nil },
                set: { if !$0 { app.presentedError = nil } }
            )
        ) {
            Button("OK") { app.presentedError = nil }
        } message: {
            Text(app.presentedError ?? "Unknown error")
        }
        .task(id: app.snapshot?.graph.digest) {
            let nodes = app.snapshot?.nodes ?? []
            if workspace.selectedNodeID.flatMap({ selected in
                nodes.first(where: { $0.nodeId == selected })
            }) == nil {
                workspace.selectedNodeID = nodes.first(where: { $0.layout != nil })?.nodeId
                    ?? nodes.first?.nodeId
            }
        }
    }

    @ViewBuilder
    private func region(
        _ region: WorkspaceRegion,
        minimumWidth: CGFloat,
        idealWidth: CGFloat
    ) -> some View {
        let panels = workspace.visiblePanels(in: region)
        if panels.isEmpty {
            ContentUnavailableView("Empty Region", systemImage: "rectangle.dashed")
                .frame(
                    minWidth: minimumWidth,
                    idealWidth: idealWidth,
                    maxWidth: .infinity,
                    maxHeight: .infinity
                )
        } else {
            VStack(spacing: 0) {
                ForEach(panels) { panel in
                    panelView(panel)
                    if panel != panels.last { Divider() }
                }
            }
            .frame(
                minWidth: minimumWidth,
                idealWidth: idealWidth,
                maxWidth: .infinity,
                maxHeight: .infinity
            )
        }
    }

    @ViewBuilder
    private func panelView(_ panel: WorkspacePanelID) -> some View {
        VStack(spacing: 0) {
            PanelHeader(panel: panel)
            Divider()
            switch panel {
            case .assetGallery:
                AssetGalleryView()
            case .graph:
                SpatialGraphView()
            case .layoutAuthoring:
                LayoutAuthoringSurfaceView()
            case .inspector:
                LayoutInspectorView()
            case .diagnostics:
                DiagnosticsView()
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var panelsMenu: some View {
        Menu("Panels", systemImage: "rectangle.3.group") {
            ForEach(WorkspacePanelID.allCases) { panel in
                Toggle(panel.title, isOn: Binding(
                    get: { workspace.isVisible(panel) },
                    set: { _ in workspace.toggle(panel) }
                ))
            }
            Divider()
            Button("Restore Layout Authoring") {
                workspace.restoreLayoutAuthoringPreset()
            }
        }
    }
}

private struct ProjectCommandBar: View {
    @EnvironmentObject private var app: AppModel
    @EnvironmentObject private var workspace: WorkspaceModel

    private var projectInitials: String {
        let words = (app.snapshot?.title ?? "Photara Project")
            .split(separator: " ")
            .prefix(2)
        return words.compactMap(\.first).map(String.init).joined().uppercased()
    }

    var body: some View {
        HStack(spacing: 12) {
            HStack(spacing: 9) {
                Text(projectInitials)
                    .font(.caption.weight(.bold))
                    .foregroundStyle(.white)
                    .frame(width: 32, height: 32)
                    .background(.tint, in: RoundedRectangle(cornerRadius: 7))
                VStack(alignment: .leading, spacing: 1) {
                    HStack(spacing: 4) {
                        Text(app.snapshot?.title ?? "Project")
                            .font(.subheadline.weight(.semibold))
                        if app.snapshot?.dirty == true {
                            Circle()
                                .fill(.orange)
                                .frame(width: 5, height: 5)
                                .help("Unsaved changes")
                        }
                    }
                    Text(projectSubtitle)
                        .font(.caption2.monospaced())
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
            }

            Divider().frame(height: 28)

            Button("New", systemImage: "plus") { app.newProject() }
                .buttonStyle(.bordered)
            Button("Open", systemImage: "folder") { app.chooseAndOpenProject() }
                .buttonStyle(.bordered)

            Spacer(minLength: 12)

            HStack(spacing: 2) {
                modeButton(.graph) { workspace.activateGraph() }
                modeButton(.layout) {
                    if let nodeID = selectedLayoutNodeID {
                        workspace.selectedNodeID = nodeID
                        workspace.activateWorkspace(for: nodeID)
                    }
                }
                modeButton(.review) { workspace.activateReview() }
                    .disabled(true)
                    .help("Review workspace is planned after Layout authoring")
            }
            .padding(3)
            .background(.quaternary.opacity(0.55), in: RoundedRectangle(cornerRadius: 8))

            Spacer(minLength: 12)

            Button("Run", systemImage: "play.fill") { app.evaluate() }
                .buttonStyle(.borderedProminent)
                .disabled(app.isEvaluating)
            Button("Save", systemImage: "square.and.arrow.down") { app.save() }
                .buttonStyle(.bordered)
        }
        .controlSize(.small)
        .padding(.horizontal, 12)
        .frame(height: 54)
        .background(Color(nsColor: .windowBackgroundColor))
    }

    private var selectedLayoutNodeID: String? {
        let layouts = app.snapshot?.nodes.filter { $0.layout != nil } ?? []
        return layouts.first(where: { $0.nodeId == workspace.selectedNodeID })?.nodeId
            ?? layouts.first?.nodeId
    }

    private var projectSubtitle: String {
        guard let snapshot = app.snapshot else { return "Not loaded" }
        return "rev \(snapshot.projectRevision) · \(snapshot.projectId.prefix(12))"
    }

    private func modeButton(
        _ mode: WorkspaceMode,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            Label(mode.title, systemImage: mode.symbol)
                .font(.caption.weight(workspace.mode == mode ? .semibold : .regular))
                .padding(.horizontal, 11)
                .frame(height: 26)
                .background {
                    if workspace.mode == mode {
                        RoundedRectangle(cornerRadius: 6)
                            .fill(Color.accentColor.opacity(0.18))
                    }
                }
        }
        .buttonStyle(.plain)
        .foregroundStyle(workspace.mode == mode ? Color.accentColor : Color.secondary)
    }
}

private struct ProjectStatusBar: View {
    @EnvironmentObject private var app: AppModel

    var body: some View {
        HStack(spacing: 9) {
            Circle()
                .fill(app.hasOpenProject ? Color.green : Color.secondary)
                .frame(width: 7, height: 7)
            Text("Project Loaded")
            Divider().frame(height: 15)
            Text(app.snapshot?.title ?? "No Project")
                .foregroundStyle(.secondary)
            Divider().frame(height: 15)
            Text("\(app.snapshot?.nodes.count ?? 0) nodes")
            Text(diagnosticSummary)
                .foregroundStyle(hasErrors ? Color.orange : Color.secondary)
            Spacer()
            if app.snapshot?.dirty == true {
                Text("Unsaved")
                    .foregroundStyle(.orange)
            }
            Divider().frame(height: 15)
            Text(app.progressLabel)
                .foregroundStyle(.secondary)
        }
        .font(.caption)
        .padding(.horizontal, 12)
        .frame(height: 28)
        .background(Color(nsColor: .windowBackgroundColor))
    }

    private var hasErrors: Bool {
        app.snapshot?.diagnostics.isEmpty == false
    }

    private var diagnosticSummary: String {
        let count = app.snapshot?.diagnostics.count ?? 0
        return count == 0 ? "No errors" : "\(count) diagnostic\(count == 1 ? "" : "s")"
    }
}

private struct ProjectLauncherView: View {
    @EnvironmentObject private var app: AppModel

    var body: some View {
        HStack(spacing: 0) {
            VStack(alignment: .leading, spacing: 22) {
                Spacer()
                Image(systemName: "point.3.connected.trianglepath.dotted")
                    .font(.system(size: 54, weight: .light))
                    .foregroundStyle(.tint)
                VStack(alignment: .leading, spacing: 6) {
                    Text("Photara")
                        .font(.largeTitle.weight(.semibold))
                    Text("Build visual workflows around your creative projects.")
                        .foregroundStyle(.secondary)
                }
                VStack(alignment: .leading, spacing: 10) {
                    Button("Create New Project", systemImage: "doc.badge.plus") {
                        app.newProject()
                    }
                    .buttonStyle(.borderedProminent)
                    .controlSize(.large)
                    Button("Open Existing Project…", systemImage: "folder") {
                        app.chooseAndOpenProject()
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.large)
                }
                Spacer()
            }
            .frame(minWidth: 340, maxWidth: 440, maxHeight: .infinity, alignment: .leading)
            .padding(44)
            Divider()
            VStack(alignment: .leading, spacing: 12) {
                Text("Recent Projects")
                    .font(.title2.weight(.semibold))
                if app.recentProjects.isEmpty {
                    ContentUnavailableView(
                        "No Recent Projects",
                        systemImage: "clock",
                        description: Text("Projects you create or open will appear here.")
                    )
                } else {
                    List(app.recentProjects) { recent in
                        Button {
                            app.openRecent(recent)
                        } label: {
                            HStack(spacing: 12) {
                                Image(systemName: "doc.richtext")
                                    .font(.title2)
                                    .foregroundStyle(.tint)
                                VStack(alignment: .leading, spacing: 3) {
                                    Text(recent.title)
                                        .font(.headline)
                                    Text(recent.lastOpened, style: .relative)
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                }
                                Spacer()
                                Image(systemName: "chevron.right")
                                    .foregroundStyle(.tertiary)
                            }
                            .contentShape(Rectangle())
                            .padding(.vertical, 5)
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .padding(32)
        }
        .background(.background)
    }
}

private struct PanelHeader: View {
    @EnvironmentObject private var workspace: WorkspaceModel
    let panel: WorkspacePanelID

    var body: some View {
        HStack(spacing: 7) {
            Text(panel.title)
                .font(.subheadline.weight(.semibold))
            Spacer()
            Menu {
                ForEach(WorkspaceRegion.allCases, id: \.self) { region in
                    Button(region.rawValue.capitalized) {
                        workspace.move(panel, to: region)
                    }
                }
            } label: {
                Image(systemName: "pin")
                    .frame(width: 20, height: 20)
            }
            .menuStyle(.borderlessButton)
            .help("Move \(panel.title)")
            Button {
                workspace.toggle(panel)
            } label: {
                Image(systemName: "xmark")
                    .frame(width: 20, height: 20)
            }
            .buttonStyle(.borderless)
            .help("Hide \(panel.title)")
        }
        .padding(.horizontal, 10)
        .frame(height: 34)
        .background(Color(nsColor: .windowBackgroundColor))
    }
}

private struct AssetGalleryView: View {
    @EnvironmentObject private var app: AppModel
    @EnvironmentObject private var workspace: WorkspaceModel
    @State private var gridScale: GalleryGridScale = .comfortable

    private var assets: [BridgeAssetDto] {
        let all = app.snapshot?.assets ?? []
        let query = workspace.galleryFilter.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else { return all }
        return all.filter { $0.displayName.localizedCaseInsensitiveContains(query) }
    }

    private var selectedLayout: BridgeNodeDto? {
        let layouts = app.snapshot?.nodes.filter { $0.layout != nil } ?? []
        if let selected = workspace.selectedNodeID {
            return layouts.first { $0.nodeId == selected }
        }
        return layouts.first
    }

    var body: some View {
        VStack(spacing: 0) {
            VStack(spacing: 8) {
                TextField("Filter assets…", text: $workspace.galleryFilter)
                    .textFieldStyle(.roundedBorder)
                HStack {
                    Text("\(assets.count) asset\(assets.count == 1 ? "" : "s")")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Spacer()
                    Picker("View", selection: $gridScale) {
                        Image(systemName: "square.grid.2x2").tag(GalleryGridScale.comfortable)
                        Image(systemName: "square.grid.3x3").tag(GalleryGridScale.compact)
                    }
                    .pickerStyle(.segmented)
                    .labelsHidden()
                    .frame(width: 72)
                }
            }
            .padding(10)
            Divider()
            if !assets.isEmpty {
                ScrollView {
                    LazyVGrid(columns: columns, spacing: 10) {
                        ForEach(assets, id: \.assetId) { asset in
                            AssetCard(
                                asset: asset,
                                reference: app.galleryProxies[asset.assetId],
                                selected: workspace.selectedAssetID == asset.assetId
                            ) {
                                workspace.selectedAssetID = asset.assetId
                            } assign: {
                                assign(asset.assetId)
                            }
                            .task { app.requestGalleryThumbnail(assetID: asset.assetId) }
                        }
                    }
                    .padding(10)
                }
                Divider()
                HStack {
                    Text(workspace.selectedAssetID == nil ? "Select an asset" : "Asset selected")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Spacer()
                    Button("Assign to Cell", systemImage: "arrow.left.circle") {
                        guard let assetID = workspace.selectedAssetID else { return }
                        assign(assetID)
                    }
                    .buttonStyle(.borderedProminent)
                    .controlSize(.small)
                }
                .disabled(workspace.selectedAssetID == nil || selectedLayout == nil)
                .padding(8)
            } else {
                ContentUnavailableView(
                    workspace.galleryFilter.isEmpty ? "No Assets" : "No Matching Assets",
                    systemImage: "photo.on.rectangle.angled",
                    description: Text("Gallery reflects project Asset Context only.")
                )
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var columns: [GridItem] {
        let width: CGFloat = gridScale == .comfortable ? 112 : 86
        return [GridItem(.adaptive(minimum: width, maximum: width * 1.35), spacing: 10)]
    }

    private func assign(_ assetID: String) {
        guard let selectedLayout,
              let frame = selectedFrame(in: selectedLayout),
              let cell = selectedCell(in: frame)
        else { return }
        app.bind(
            assetID: assetID,
            to: selectedLayout,
            frameID: frame.frameId,
            cellID: cell.cellId
        )
    }

    private func selectedFrame(in node: BridgeNodeDto) -> BridgeLayoutFrameInspectionDto? {
        let frames = node.layout?.frames ?? []
        return frames.first { $0.frameId == workspace.selectedFrameID } ?? frames.first
    }

    private func selectedCell(
        in frame: BridgeLayoutFrameInspectionDto
    ) -> BridgeLayoutCellInspectionDto? {
        frame.cells.first { $0.cellId == workspace.selectedCellID } ?? frame.cells.first
    }
}

private enum GalleryGridScale: Hashable {
    case comfortable
    case compact
}

private struct AssetCard: View {
    let asset: BridgeAssetDto
    let reference: BridgeProxyReference?
    let selected: Bool
    let select: () -> Void
    let assign: () -> Void

    var body: some View {
        Button(action: select) {
            VStack(alignment: .leading, spacing: 5) {
                ZStack(alignment: .topLeading) {
                    GalleryThumbnail(reference: reference)
                    Text("\(asset.representationCount) reps")
                        .font(.system(size: 9, weight: .semibold))
                        .padding(.horizontal, 5)
                        .padding(.vertical, 3)
                        .background(.ultraThinMaterial, in: Capsule())
                        .padding(5)
                }
                Text(asset.displayName)
                    .font(.caption)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .padding(4)
        .background(
            selected ? Color.accentColor.opacity(0.18) : Color.clear,
            in: RoundedRectangle(cornerRadius: 7)
        )
        .overlay {
            RoundedRectangle(cornerRadius: 7)
                .stroke(selected ? Color.accentColor : .clear, lineWidth: 1.5)
        }
        .simultaneousGesture(TapGesture(count: 2).onEnded(assign))
        .contextMenu {
            Button("Assign to Selected Cell", action: assign)
        }
    }
}

private struct GalleryThumbnail: View {
    let reference: BridgeProxyReference?

    var body: some View {
        Group {
            if let path = reference?.descriptor().localPath,
               let image = NSImage(contentsOfFile: path) {
                Image(nsImage: image)
                    .resizable()
                    .scaledToFill()
            } else {
                Image(systemName: "photo")
                    .foregroundStyle(.secondary)
            }
        }
        .aspectRatio(4 / 3, contentMode: .fit)
        .frame(maxWidth: .infinity)
        .clipped()
        .background(.quaternary, in: RoundedRectangle(cornerRadius: 5))
        .clipShape(RoundedRectangle(cornerRadius: 5))
    }
}

private struct SpatialGraphView: View {
    @EnvironmentObject private var app: AppModel
    @EnvironmentObject private var workspace: WorkspaceModel
    @State private var pan: CGSize = .zero
    @State private var zoom: CGFloat = 1
    @State private var showsNodeMenu = false
    @State private var nodeFilter = ""
    @FocusState private var graphHasFocus: Bool
    @GestureState private var dragPan: CGSize = .zero
    @GestureState private var magnification: CGFloat = 1

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 5) {
                Button("Add Node", systemImage: "plus") {
                    showsNodeMenu.toggle()
                }
                .popover(isPresented: $showsNodeMenu, arrowEdge: .bottom) {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Add Node")
                            .font(.headline)
                        TextField("Search nodes", text: $nodeFilter)
                            .textFieldStyle(.roundedBorder)
                        let definitions = filteredDefinitions
                        let categories = Dictionary(grouping: definitions) {
                            $0.catalogPath.first ?? "Other"
                        }
                        ForEach(categories.keys.sorted(), id: \.self) { category in
                            Text(category.uppercased())
                                .font(.caption2.weight(.semibold))
                                .foregroundStyle(.secondary)
                            ForEach(categories[category] ?? [], id: \.definitionId) { definition in
                                Button {
                                    app.addNode(definition)
                                    showsNodeMenu = false
                                    nodeFilter = ""
                                } label: {
                                    HStack(spacing: 10) {
                                        NodeBrandIcon(
                                            resourceID: definition.iconResourceId,
                                            accentHex: definition.accentSrgbHex,
                                            size: 28
                                        )
                                        VStack(alignment: .leading, spacing: 2) {
                                            Text(definition.brandName)
                                                .font(.subheadline.weight(.semibold))
                                            Text(definition.catalogPath.dropFirst().joined(separator: " › "))
                                                .font(.caption)
                                                .foregroundStyle(.secondary)
                                        }
                                        Spacer(minLength: 8)
                                    }
                                    .contentShape(Rectangle())
                                }
                                .buttonStyle(.plain)
                            }
                        }
                    }
                    .padding(12)
                    .frame(width: 310, alignment: .leading)
                }
                Button("Lock", systemImage: "lock.open") {}
                    .disabled(true)
                Spacer()
                Button {
                    zoom = max(0.55, zoom - 0.1)
                } label: {
                    Image(systemName: "minus")
                }
                Text("\(Int(zoom * 100))%")
                    .font(.caption.monospacedDigit())
                    .foregroundStyle(.secondary)
                    .frame(width: 42)
                Button {
                    zoom = min(2, zoom + 0.1)
                } label: {
                    Image(systemName: "plus")
                }
                Button("Reset View", systemImage: "arrow.up.left.and.down.right.magnifyingglass") {
                    pan = .zero
                    zoom = 1
                }
            }
            .labelStyle(.iconOnly)
            .buttonStyle(.borderless)
            .controlSize(.small)
            .padding(.horizontal, 9)
            .frame(height: 31)
            .background(Color(nsColor: .windowBackgroundColor).opacity(0.8))
            Divider()
            GeometryReader { geometry in
                let nodes = app.snapshot?.nodes ?? []
                let positions = graphPositions(nodes: nodes, size: geometry.size)
                ZStack {
                    GraphBackground()
                    Canvas { context, _ in
                        for connection in app.snapshot?.graph.connections ?? [] {
                            guard let source = positions[connection.outputNodeId],
                                  let target = positions[connection.inputNodeId]
                            else { continue }
                            var path = Path()
                            let start = CGPoint(x: source.x + 88, y: source.y)
                            let end = CGPoint(x: target.x - 88, y: target.y)
                            path.move(to: start)
                            path.addCurve(
                                to: end,
                                control1: CGPoint(x: start.x + 70, y: start.y),
                                control2: CGPoint(x: end.x - 70, y: end.y)
                            )
                            context.stroke(
                                path,
                                with: .color(.accentColor.opacity(0.55)),
                                lineWidth: 2
                            )
                        }
                    }
                    ForEach(nodes, id: \.nodeId) { node in
                        GraphNodeCard(
                            node: node,
                            selected: workspace.selectedNodeID == node.nodeId
                        )
                        .position(positions[node.nodeId] ?? .zero)
                        .onTapGesture(count: 2) {
                            workspace.selectedNodeID = node.nodeId
                            if node.hasWorkspace {
                                workspace.activateWorkspace(for: node.nodeId)
                            }
                        }
                        .onTapGesture {
                            workspace.selectedNodeID = node.nodeId
                        }
                    }
                }
                .scaleEffect(min(2, max(0.55, zoom * magnification)))
                .offset(
                    x: pan.width + dragPan.width,
                    y: pan.height + dragPan.height
                )
                .contentShape(Rectangle())
                .gesture(
                    DragGesture()
                        .updating($dragPan) { value, state, _ in state = value.translation }
                        .onEnded {
                            pan.width += $0.translation.width
                            pan.height += $0.translation.height
                        }
                )
                .simultaneousGesture(
                    MagnificationGesture()
                        .updating($magnification) { value, state, _ in state = value }
                        .onEnded { zoom = min(2, max(0.55, zoom * $0)) }
                )
                .overlay {
                    if nodes.isEmpty {
                        ContentUnavailableView {
                            Label(
                                "No Nodes",
                                systemImage: "point.3.connected.trianglepath.dotted"
                            )
                        } description: {
                            Text("Add a node to begin.")
                        } actions: {
                            Button("Add Node", systemImage: "square.grid.2x2") {
                                showsNodeMenu = true
                            }
                            .buttonStyle(.borderedProminent)
                        }
                    }
                }
            }
            .focusable()
            .focused($graphHasFocus)
            .onKeyPress(.tab) {
                showsNodeMenu = true
                return .handled
            }
            .onChange(of: workspace.nodeMenuRequest) {
                showsNodeMenu = true
                graphHasFocus = true
            }
            .onTapGesture {
                graphHasFocus = true
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var filteredDefinitions: [BridgeAvailableNodeDefinitionDto] {
        let query = nodeFilter.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else { return app.nodeDefinitions }
        return app.nodeDefinitions.filter { definition in
            ([definition.brandName, definition.displayName, definition.definitionId]
                + definition.catalogPath + definition.searchTerms)
                .contains { $0.localizedCaseInsensitiveContains(query) }
        }
    }

    private func graphPositions(
        nodes: [BridgeNodeDto],
        size: CGSize
    ) -> [String: CGPoint] {
        var positions: [String: CGPoint] = [:]
        var sourceIndex = 0
        var workspaceIndex = 0
        for node in nodes {
            if node.ports.allSatisfy({ $0.direction == .output }) {
                positions[node.nodeId] = CGPoint(x: max(130, size.width * 0.28), y: 130 + CGFloat(sourceIndex * 170))
                sourceIndex += 1
            } else {
                positions[node.nodeId] = CGPoint(x: max(360, size.width * 0.66), y: 130 + CGFloat(workspaceIndex * 190))
                workspaceIndex += 1
            }
        }
        return positions
    }
}

/// Resolves package-neutral icon resources into this macOS client's skin.
private enum NativeNodeResources {
    static func symbol(for resourceID: String) -> String {
        switch resourceID {
        case "photara.layout.compose": "rectangle.3.group"
        case "photara.project.assets": "photo.stack"
        case "photara.disk.folder": "folder"
        default: "square.dashed"
        }
    }
}

private struct NodeBrandIcon: View {
    let resourceID: String
    let accentHex: String?
    let size: CGFloat

    var body: some View {
        let accent = Color(srgbHex: accentHex) ?? .accentColor
        Image(systemName: NativeNodeResources.symbol(for: resourceID))
            .font(.system(size: size * 0.56, weight: .medium))
            .foregroundStyle(accent)
            .frame(width: size, height: size)
            .background(accent.opacity(0.12), in: RoundedRectangle(cornerRadius: size * 0.22))
    }
}

private extension Color {
    init?(srgbHex: String?) {
        guard var value = srgbHex?.trimmingCharacters(in: .whitespacesAndNewlines),
              value.hasPrefix("#")
        else { return nil }
        value.removeFirst()
        guard value.count == 6, let rgb = UInt64(value, radix: 16) else { return nil }
        self.init(
            .sRGB,
            red: Double((rgb >> 16) & 0xff) / 255,
            green: Double((rgb >> 8) & 0xff) / 255,
            blue: Double(rgb & 0xff) / 255,
            opacity: 1
        )
    }
}

private struct GraphBackground: View {
    var body: some View {
        Canvas { context, size in
            let spacing: CGFloat = 24
            for x in stride(from: 0 as CGFloat, through: size.width, by: spacing) {
                for y in stride(from: 0 as CGFloat, through: size.height, by: spacing) {
                    context.fill(
                        Path(ellipseIn: CGRect(x: x, y: y, width: 1.5, height: 1.5)),
                        with: .color(.secondary.opacity(0.22))
                    )
                }
            }
        }
        .background(Color(nsColor: .controlBackgroundColor).opacity(0.55))
    }
}

private struct GraphNodeCard: View {
    let node: BridgeNodeDto
    let selected: Bool

    private var inputs: [BridgePortInspectionDto] {
        node.ports.filter { $0.direction == .input }
    }

    private var outputs: [BridgePortInspectionDto] {
        node.ports.filter { $0.direction == .output }
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 8) {
                NodeBrandIcon(
                    resourceID: node.iconResourceId,
                    accentHex: node.accentSrgbHex,
                    size: 24
                )
                VStack(alignment: .leading, spacing: 1) {
                    Text(node.displayName).font(.headline)
                    if let canvas = node.layout?.canvas {
                        Text("\(canvas.widthPixels) × \(canvas.heightPixels)")
                            .font(.caption2.monospacedDigit())
                            .foregroundStyle(.secondary)
                    }
                }
                Spacer()
                Circle()
                    .fill(node.status == "Ready" ? Color.green : Color.orange)
                    .frame(width: 7, height: 7)
            }
            .padding(10)
            Divider()
            HStack(alignment: .top) {
                portColumn(inputs, leading: true)
                Spacer(minLength: 14)
                portColumn(outputs, leading: false)
            }
            .padding(.vertical, 8)
        }
        .frame(width: 176)
        .frame(minHeight: 86)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 10))
        .overlay {
            RoundedRectangle(cornerRadius: 10)
                .stroke(selected ? Color.accentColor : .secondary.opacity(0.3), lineWidth: selected ? 3 : 1)
        }
        .shadow(color: .black.opacity(selected ? 0.18 : 0.08), radius: selected ? 8 : 3, y: 2)
    }

    @ViewBuilder
    private func portColumn(_ ports: [BridgePortInspectionDto], leading: Bool) -> some View {
        VStack(alignment: leading ? .leading : .trailing, spacing: 5) {
            ForEach(ports, id: \.portId) { port in
                HStack(spacing: 4) {
                    if leading { portDot }
                    Text(port.portId.capitalized)
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                    if !leading { portDot }
                }
            }
        }
    }

    private var portDot: some View {
        Circle()
            .fill(Color.accentColor)
            .frame(width: 7, height: 7)
    }
}

private struct LayoutInspectorView: View {
    @EnvironmentObject private var app: AppModel
    @EnvironmentObject private var workspace: WorkspaceModel

    private var selectedNode: BridgeNodeDto? {
        let nodes = app.snapshot?.nodes ?? []
        if let selected = workspace.selectedNodeID {
            return nodes.first { $0.nodeId == selected }
        }
        return nil
    }

    private var selectedFrame: BridgeLayoutFrameInspectionDto? {
        let frames = selectedNode?.layout?.frames ?? []
        return frames.first { $0.frameId == workspace.selectedFrameID } ?? frames.first
    }

    private var selectedCell: BridgeLayoutCellInspectionDto? {
        guard let frame = selectedFrame else { return nil }
        return frame.cells.first { $0.cellId == workspace.selectedCellID } ?? frame.cells.first
    }

    var body: some View {
        if let node = selectedNode {
            Form {
                Section {
                    HStack(spacing: 12) {
                        NodeBrandIcon(
                            resourceID: node.iconResourceId,
                            accentHex: node.accentSrgbHex,
                            size: 30
                        )
                        VStack(alignment: .leading, spacing: 2) {
                            Text(node.displayName).font(.headline)
                            Text(node.status)
                                .font(.caption)
                                .foregroundStyle(statusColor(node.status))
                        }
                    }
                    LabeledContent("Definition", value: node.definitionId)
                        .font(.caption)
                    LabeledContent("Package", value: "\(node.packageId) \(node.packageVersion)")
                        .font(.caption)
                    LabeledContent("Node ID", value: node.nodeId)
                        .font(.caption.monospaced())
                }
                if let disk = node.disk {
                    Section("Folder Source") {
                        LabeledContent("Accepted Assets", value: String(disk.acceptedAssetCount))
                        LabeledContent("Scan", value: disk.recursive ? "Recursive" : "Top Level")
                        LabeledContent("Portable Binding", value: disk.folderBindingId)
                            .font(.caption.monospaced())
                            .lineLimit(1)
                        Button("Choose or Rebind Folder", systemImage: "folder.badge.plus") {
                            app.chooseFolder(for: node)
                        }
                        Button("Scan Folder", systemImage: "arrow.clockwise") {
                            app.scanDisk(node)
                        }
                        Button("Connect to Available Layout", systemImage: "point.3.connected.trianglepath.dotted") {
                            app.connectDiskToAvailableLayout(node)
                        }
                    }
                }
                if !inputPorts(node).isEmpty {
                    Section("Inputs") {
                        ForEach(inputPorts(node), id: \.portId) { port in
                            InspectorPortView(port: port)
                        }
                    }
                }
                if node.layout != nil {
                    Section("Parameters") {
                        LabeledContent("Canvas", value: canvasDescription(node))
                    if let layout = node.layout {
                        LabeledContent("Frames", value: String(layout.frames.count))
                        LabeledContent("Authored digest", value: layout.authoredStateDigest)
                            .font(.caption.monospaced())
                    }
                    }
                }
                if let frame = selectedFrame, let cell = selectedCell {
                    Section("Frame") {
                        LabeledContent("Index", value: String(frame.index + 1))
                        Picker("Arrangement", selection: arrangementBinding(node, frame)) {
                            Text("One").tag(LayoutArrangementChoice.one)
                            Text("Horizontal").tag(LayoutArrangementChoice.horizontal)
                            Text("Vertical").tag(LayoutArrangementChoice.vertical)
                            Text("Grid").tag(LayoutArrangementChoice.grid)
                            if frame.arrangement == .custom {
                                Text("Custom").tag(LayoutArrangementChoice.custom)
                            }
                        }
                        HStack {
                            Button("Add Cell") {
                                app.editStructure(
                                    node: node,
                                    edit: .insertCell(
                                        frameId: frame.frameId,
                                        index: UInt64(frame.cells.count)
                                    )
                                )
                            }
                            .disabled(frame.arrangement == .one || frame.arrangement == .custom)
                            Button("Remove Cell") {
                                app.editStructure(
                                    node: node,
                                    edit: .removeCell(
                                        frameId: frame.frameId,
                                        cellId: cell.cellId
                                    )
                                )
                            }
                            .disabled(frame.cells.count <= 1)
                        }
                        HStack {
                            Button("Add Frame") {
                                app.editStructure(
                                    node: node,
                                    edit: .insertFrame(
                                        index: UInt64(node.layout?.frames.count ?? 0)
                                    )
                                )
                            }
                            Button("Remove Frame") {
                                app.editStructure(
                                    node: node,
                                    edit: .removeFrame(frameId: frame.frameId)
                                )
                            }
                            .disabled((node.layout?.frames.count ?? 0) <= 1)
                        }
                        HStack {
                            Button("Move Earlier") {
                                app.editStructure(
                                    node: node,
                                    edit: .moveFrame(
                                        frameId: frame.frameId,
                                        toIndex: frame.index - 1
                                    )
                                )
                            }
                            .disabled(frame.index == 0)
                            Button("Move Later") {
                                app.editStructure(
                                    node: node,
                                    edit: .moveFrame(
                                        frameId: frame.frameId,
                                        toIndex: frame.index + 1
                                    )
                                )
                            }
                            .disabled(frame.index + 1 >= UInt64(node.layout?.frames.count ?? 0))
                        }
                    }
                    Section("Cell") {
                        if let descriptor = app.layoutCellProxies[cell.cellId]?.descriptor() {
                            LabeledContent(
                                "Preview range",
                                value: descriptor.dynamicRange == .hdr ? "HDR" : "SDR"
                            )
                            LabeledContent("Color", value: descriptor.colorSpaceId)
                        }
                        Picker("Mode", selection: contentModeBinding(node, frame, cell)) {
                            Text("Fit").tag(LayoutContentChoice.fit)
                            Text("Fill").tag(LayoutContentChoice.fill)
                            Text("Crop").tag(LayoutContentChoice.crop)
                        }
                        HStack {
                            Text("Focal / alignment")
                            Spacer()
                            AlignmentPad { x, y in
                                let edit: BridgeLayoutCellEdit = cell.contentMode == .fit
                                    ? .fit(alignmentX: x, alignmentY: y)
                                    : .fill(focalX: x, focalY: y)
                                app.editCell(
                                    node: node,
                                    frameID: frame.frameId,
                                    cellID: cell.cellId,
                                    edit: edit
                                )
                            }
                        }
                        Button("Rotate 90°") {
                            app.editCell(
                                node: node,
                                frameID: frame.frameId,
                                cellID: cell.cellId,
                                edit: .setQuarterTurn(
                                    quarterTurn: nextQuarterTurn(after: cell.quarterTurn)
                                )
                            )
                        }
                        if let rect = cell.cropRect {
                            LabeledContent(
                                "Crop",
                                value: "\(rect.x), \(rect.y) · \(rect.width) × \(rect.height)"
                            )
                            .font(.caption.monospaced())
                            HStack {
                                Button("Crop Tighter") {
                                    app.editCell(
                                        node: node,
                                        frameID: frame.frameId,
                                        cellID: cell.cellId,
                                        edit: resizedCrop(rect, factor: 0.8)
                                    )
                                }
                                Button("Crop Looser") {
                                    app.editCell(
                                        node: node,
                                        frameID: frame.frameId,
                                        cellID: cell.cellId,
                                        edit: resizedCrop(rect, factor: 1.25)
                                    )
                                }
                            }
                        }
                    }
                }
                if !outputPorts(node).isEmpty {
                    Section("Outputs") {
                        ForEach(outputPorts(node), id: \.portId) { port in
                            InspectorPortView(port: port)
                        }
                    }
                }
                Section("Evaluation") {
                    LabeledContent("State", value: node.status)
                    LabeledContent(
                        "Graph revision",
                        value: String(app.snapshot?.graph.revision ?? 0)
                    )
                    LabeledContent("Progress", value: app.progressLabel)
                }
                if !node.diagnostics.isEmpty {
                    Section("Diagnostics") {
                        ForEach(node.diagnostics, id: \.code) { diagnostic in
                            VStack(alignment: .leading, spacing: 2) {
                                Text(diagnostic.message)
                                Text(diagnostic.code)
                                    .font(.caption.monospaced())
                                    .foregroundStyle(.secondary)
                            }
                        }
                    }
                }
            }
            .formStyle(.grouped)
        } else {
            ContentUnavailableView(
                "No Selection",
                systemImage: "sidebar.trailing",
                description: Text("Select a node in Graph to inspect it.")
            )
        }
    }

    private func canvasDescription(_ node: BridgeNodeDto) -> String {
        guard let canvas = node.layout?.canvas else { return "Unknown" }
        return "\(canvas.widthPixels) × \(canvas.heightPixels)"
    }

    private func inputPorts(_ node: BridgeNodeDto) -> [BridgePortInspectionDto] {
        node.ports.filter { $0.direction == .input }
    }

    private func outputPorts(_ node: BridgeNodeDto) -> [BridgePortInspectionDto] {
        node.ports.filter { $0.direction == .output }
    }

    private func statusColor(_ status: String) -> Color {
        switch status {
        case "Ready": .green
        case "Error": .red
        default: .orange
        }
    }

    private func arrangementBinding(
        _ node: BridgeNodeDto,
        _ frame: BridgeLayoutFrameInspectionDto
    ) -> Binding<LayoutArrangementChoice> {
        Binding(
            get: { LayoutArrangementChoice(frame.arrangement) },
            set: { choice in
                app.editStructure(
                    node: node,
                    edit: .setFrameArrangement(
                        frameId: frame.frameId,
                        arrangement: choice.bridgeValue
                    )
                )
            }
        )
    }

    private func contentModeBinding(
        _ node: BridgeNodeDto,
        _ frame: BridgeLayoutFrameInspectionDto,
        _ cell: BridgeLayoutCellInspectionDto
    ) -> Binding<LayoutContentChoice> {
        Binding(
            get: { LayoutContentChoice(cell.contentMode) },
            set: { choice in
                let edit: BridgeLayoutCellEdit = switch choice {
                case .fit: .fit(alignmentX: 500_000, alignmentY: 500_000)
                case .fill: .fill(focalX: 500_000, focalY: 500_000)
                case .crop: .crop(x: 100_000, y: 100_000, width: 800_000, height: 800_000)
                }
                app.editCell(
                    node: node,
                    frameID: frame.frameId,
                    cellID: cell.cellId,
                    edit: edit
                )
            }
        )
    }

    private func resizedCrop(
        _ rect: BridgeNormalizedRectDto,
        factor: Double
    ) -> BridgeLayoutCellEdit {
        let width = UInt32(max(100_000, min(1_000_000, Double(rect.width) * factor)))
        let height = UInt32(max(100_000, min(1_000_000, Double(rect.height) * factor)))
        let centerX = Int64(rect.x) + Int64(rect.width) / 2
        let centerY = Int64(rect.y) + Int64(rect.height) / 2
        let x = UInt32(max(0, min(Int64(1_000_000 - width), centerX - Int64(width) / 2)))
        let y = UInt32(max(0, min(Int64(1_000_000 - height), centerY - Int64(height) / 2)))
        return .crop(x: x, y: y, width: width, height: height)
    }
}

private struct InspectorPortView: View {
    let port: BridgePortInspectionDto

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            HStack {
                Circle()
                    .fill(Color.accentColor)
                    .frame(width: 7, height: 7)
                Text(port.portId.capitalized).font(.headline)
                Spacer()
                Text(port.valueTypeId)
                    .font(.caption.monospaced())
                    .foregroundStyle(.secondary)
            }
            if let connected = port.connectedNodeName {
                LabeledContent(
                    port.direction == .input ? "Source" : "Consumer",
                    value: connected
                )
            } else {
                LabeledContent("Connection", value: "Unconnected")
                    .foregroundStyle(.secondary)
            }
            ForEach(Array(port.summary.enumerated()), id: \.offset) { _, field in
                LabeledContent(field.label, value: field.value)
            }
        }
        .padding(.vertical, 3)
    }
}

private enum LayoutArrangementChoice: Hashable {
    case one
    case horizontal
    case vertical
    case grid
    case custom

    init(_ value: BridgeLayoutArrangement) {
        switch value {
        case .one: self = .one
        case .horizontalStack: self = .horizontal
        case .verticalStack: self = .vertical
        case .uniformGrid: self = .grid
        case .custom: self = .custom
        }
    }

    var bridgeValue: BridgeLayoutArrangementEdit {
        switch self {
        case .one: .one
        case .horizontal: .horizontalStack
        case .vertical: .verticalStack
        case .grid: .uniformGrid(columns: 2)
        case .custom: .custom
        }
    }
}

private enum LayoutContentChoice: Hashable {
    case fit
    case fill
    case crop

    init(_ value: BridgeLayoutContentMode) {
        switch value {
        case .fit: self = .fit
        case .fill: self = .fill
        case .crop: self = .crop
        }
    }
}

private func nextQuarterTurn(after value: BridgeQuarterTurn) -> BridgeQuarterTurn {
    switch value {
    case .zero: .clockwise90
    case .clockwise90: .clockwise180
    case .clockwise180: .clockwise270
    case .clockwise270: .zero
    }
}

private struct AlignmentPad: View {
    let select: (UInt32, UInt32) -> Void

    private let values: [UInt32] = [0, 500_000, 1_000_000]

    var body: some View {
        Grid(horizontalSpacing: 3, verticalSpacing: 3) {
            ForEach(values, id: \.self) { y in
                GridRow {
                    ForEach(values, id: \.self) { x in
                        Button {
                            select(x, y)
                        } label: {
                            Circle().frame(width: 5, height: 5)
                        }
                        .buttonStyle(.bordered)
                        .controlSize(.mini)
                    }
                }
            }
        }
    }
}

/// The visual authoring surface is deliberately independent from Inspector
/// placement. It consumes immutable resolved DTOs and shared proxy references.
private struct LayoutAuthoringSurfaceView: View {
    @EnvironmentObject private var app: AppModel
    @EnvironmentObject private var workspace: WorkspaceModel

    private var selectedNode: BridgeNodeDto? {
        let layouts = app.snapshot?.nodes.filter { $0.layout != nil } ?? []
        if let active = workspace.activeWorkspaceNodeID,
           let activeNode = layouts.first(where: { $0.nodeId == active })
        {
            return activeNode
        }
        if let selected = workspace.selectedNodeID {
            return layouts.first { $0.nodeId == selected } ?? layouts.first
        }
        return layouts.first
    }

    private func selectedFrame(in node: BridgeNodeDto) -> BridgeLayoutFrameInspectionDto? {
        let frames = node.layout?.frames ?? []
        return frames.first { $0.frameId == workspace.selectedFrameID } ?? frames.first
    }

    var body: some View {
        if let node = selectedNode, let layout = node.layout,
           let frame = selectedFrame(in: node)
        {
            VStack(spacing: 0) {
                HStack {
                    Picker("Frame", selection: Binding(
                        get: { workspace.selectedFrameID ?? frame.frameId },
                        set: { value in
                            workspace.selectedNodeID = node.nodeId
                            workspace.selectedFrameID = value
                            workspace.selectedCellID = nil
                        }
                    )) {
                        ForEach(layout.frames, id: \.frameId) { candidate in
                            Text("Frame \(candidate.index + 1)").tag(candidate.frameId)
                        }
                    }
                    .frame(maxWidth: 220)
                    Spacer()
                    Text("\(layout.canvas.widthPixels) × \(layout.canvas.heightPixels)")
                        .font(.caption.monospacedDigit())
                        .foregroundStyle(.secondary)
                }
                .padding(8)
                Divider()
                LayoutCanvasView(node: node, frame: frame)
                    .padding(20)
            }
            .task(id: layout.authoredStateDigest) {
                if !layout.frames.contains(where: { $0.frameId == workspace.selectedFrameID }) {
                    workspace.selectedFrameID = layout.frames.first?.frameId
                }
                let currentFrame = selectedFrame(in: node)
                if currentFrame?.cells.contains(where: {
                    $0.cellId == workspace.selectedCellID
                }) != true {
                    workspace.selectedCellID = currentFrame?.cells.first?.cellId
                }
                app.requestLayoutProxies(for: node.nodeId)
            }
        } else {
            ContentUnavailableView(
                "No Layout Selected",
                systemImage: "rectangle.3.group",
                description: Text("Add or select a Layout node to author it visually.")
            )
        }
    }
}

private struct LayoutCanvasView: View {
    @EnvironmentObject private var app: AppModel
    @EnvironmentObject private var workspace: WorkspaceModel
    let node: BridgeNodeDto
    let frame: BridgeLayoutFrameInspectionDto

    private var canvas: BridgeLayoutCanvasInspectionDto { node.layout!.canvas }

    var body: some View {
        GeometryReader { available in
            let fitted = fittedCanvas(in: available.size)
            ZStack {
                Color.black.opacity(0.06)
                ZStack {
                    Color.white
                    ForEach(frame.cells, id: \.cellId) { cell in
                        LayoutCanvasCell(
                            node: node,
                            frame: frame,
                            cell: cell,
                            canvasSize: fitted
                        )
                    }
                }
                .frame(width: fitted.width, height: fitted.height)
                .shadow(color: .black.opacity(0.18), radius: 12, y: 5)
            }
        }
    }

    private func fittedCanvas(in available: CGSize) -> CGSize {
        let width = CGFloat(canvas.widthPixels)
        let height = CGFloat(canvas.heightPixels)
        let scale = min(available.width / width, available.height / height)
        return CGSize(width: width * scale, height: height * scale)
    }
}

private struct LayoutCanvasCell: View {
    @EnvironmentObject private var app: AppModel
    @EnvironmentObject private var workspace: WorkspaceModel
    let node: BridgeNodeDto
    let frame: BridgeLayoutFrameInspectionDto
    let cell: BridgeLayoutCellInspectionDto
    let canvasSize: CGSize
    @State private var draftTranslation: CGSize = .zero

    private let scale = CGFloat(1_000_000)

    var body: some View {
        let rect = cell.resolvedRect
        let width = canvasSize.width * CGFloat(rect.width) / scale
        let height = canvasSize.height * CGFloat(rect.height) / scale
        let x = canvasSize.width * CGFloat(rect.x) / scale + width / 2
        let y = canvasSize.height * CGFloat(rect.y) / scale + height / 2
        ZStack {
            if let reference = app.layoutCellProxies[cell.cellId],
               let image = NSImage(contentsOfFile: reference.descriptor().localPath)
            {
                let descriptor = reference.descriptor()
                proxyImage(image)
                    .rotationEffect(rotation(cell.quarterTurn))
                    .offset(draftTranslation)
                VStack {
                    Spacer()
                    HStack {
                        Spacer()
                        Text(descriptor.dynamicRange == .hdr ? "HDR" : "SDR")
                            .font(.caption2.bold())
                            .padding(4)
                            .background(.thinMaterial, in: Capsule())
                    }
                }
                .padding(5)
            } else {
                Image(systemName: cell.assetId == nil ? "plus" : "photo")
                    .font(.title2)
                    .foregroundStyle(.secondary)
            }
        }
        .frame(width: width, height: height)
        .clipped()
        .overlay {
            Rectangle()
                .stroke(
                    workspace.selectedCellID == cell.cellId ? Color.accentColor : .white.opacity(0.8),
                    lineWidth: workspace.selectedCellID == cell.cellId ? 3 : 1
                )
        }
        .contentShape(Rectangle())
        .position(x: x, y: y)
        .onTapGesture {
            workspace.selectedNodeID = node.nodeId
            workspace.selectedFrameID = frame.frameId
            workspace.selectedCellID = cell.cellId
        }
        .gesture(cropGesture(cellSize: CGSize(width: width, height: height)))
    }

    @ViewBuilder
    private func proxyImage(_ image: NSImage) -> some View {
        switch cell.contentMode {
        case .fit:
            Image(nsImage: image).resizable().scaledToFit()
        case .fill, .crop:
            Image(nsImage: image).resizable().scaledToFill()
        }
    }

    private func cropGesture(cellSize: CGSize) -> some Gesture {
        DragGesture(minimumDistance: 2)
            .onChanged { value in
                guard workspace.selectedCellID == cell.cellId, cell.cropRect != nil else { return }
                draftTranslation = value.translation
            }
            .onEnded { value in
                defer { draftTranslation = .zero }
                guard workspace.selectedCellID == cell.cellId,
                      let crop = cell.cropRect,
                      cellSize.width > 0,
                      cellSize.height > 0
                else { return }
                let dx = Int64((value.translation.width / cellSize.width * scale).rounded())
                let dy = Int64((value.translation.height / cellSize.height * scale).rounded())
                let maximumX = Int64(1_000_000 - crop.width)
                let maximumY = Int64(1_000_000 - crop.height)
                let x = UInt32(max(0, min(maximumX, Int64(crop.x) - dx)))
                let y = UInt32(max(0, min(maximumY, Int64(crop.y) - dy)))
                app.editCell(
                    node: node,
                    frameID: frame.frameId,
                    cellID: cell.cellId,
                    edit: .crop(x: x, y: y, width: crop.width, height: crop.height)
                )
            }
    }

    private func rotation(_ turn: BridgeQuarterTurn) -> Angle {
        switch turn {
        case .zero: .degrees(0)
        case .clockwise90: .degrees(90)
        case .clockwise180: .degrees(180)
        case .clockwise270: .degrees(270)
        }
    }
}

private struct DiagnosticsView: View {
    @EnvironmentObject private var app: AppModel

    var body: some View {
        if let diagnostics = app.snapshot?.diagnostics, !diagnostics.isEmpty {
            List(diagnostics, id: \.code) { diagnostic in
                VStack(alignment: .leading) {
                    Text(diagnostic.message)
                    Text(diagnostic.code)
                        .font(.caption.monospaced())
                        .foregroundStyle(.secondary)
                }
            }
        } else {
            ContentUnavailableView("No Diagnostics", systemImage: "checkmark.circle")
        }
    }
}
