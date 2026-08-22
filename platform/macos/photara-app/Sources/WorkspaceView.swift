import SwiftUI

struct WorkspaceView: View {
    @EnvironmentObject private var app: AppModel
    @EnvironmentObject private var workspace: WorkspaceModel

    var body: some View {
        Group {
            if app.hasOpenProject {
                HSplitView {
                    region(.leading, minimumWidth: 190, idealWidth: 240)
                    region(.content, minimumWidth: 320, idealWidth: 560)
                    region(.trailing, minimumWidth: 230, idealWidth: 300)
                }
            } else {
                ContentUnavailableView(
                    "No Project Open",
                    systemImage: "doc",
                    description: Text("Create a project or reopen the most recent one.")
                )
            }
        }
        .frame(minWidth: 820, minHeight: 520)
        .toolbar {
            ToolbarItemGroup {
                Button("New", systemImage: "doc.badge.plus") { app.newProject() }
                Button("Reopen", systemImage: "folder") { app.reopenLastProject() }
                Button("Close", systemImage: "xmark.square") { app.closeProject() }
                    .disabled(!app.hasOpenProject)
                Button("Import Pair", systemImage: "photo.badge.plus") {
                    app.chooseAndImportTiffPair()
                }
                .disabled(!app.hasOpenProject)
                Button("Add Layout", systemImage: "rectangle.3.group") {
                    app.addLayout()
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
                .frame(minWidth: minimumWidth, idealWidth: idealWidth, maxWidth: .infinity)
        } else {
            VStack(spacing: 0) {
                ForEach(panels) { panel in
                    panelView(panel)
                    if panel != panels.last { Divider() }
                }
            }
            .frame(minWidth: minimumWidth, idealWidth: idealWidth, maxWidth: .infinity)
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
                PrimitiveGraphView()
            case .inspector:
                LayoutInspectorView()
            case .diagnostics:
                DiagnosticsView()
            }
        }
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

private struct PanelHeader: View {
    @EnvironmentObject private var workspace: WorkspaceModel
    let panel: WorkspacePanelID

    var body: some View {
        HStack {
            Text(panel.title).font(.headline)
            Spacer()
            Menu("Move", systemImage: "arrow.left.arrow.right") {
                ForEach(WorkspaceRegion.allCases, id: \.self) { region in
                    Button(region.rawValue.capitalized) {
                        workspace.move(panel, to: region)
                    }
                }
            }
            .menuStyle(.borderlessButton)
            Button("Hide", systemImage: "xmark") {
                workspace.toggle(panel)
            }
            .buttonStyle(.borderless)
        }
        .padding(.horizontal, 10)
        .frame(height: 36)
    }
}

private struct AssetGalleryView: View {
    @EnvironmentObject private var app: AppModel
    @EnvironmentObject private var workspace: WorkspaceModel

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
            TextField("Filter project assets", text: $workspace.galleryFilter)
                .textFieldStyle(.roundedBorder)
                .padding(8)
            if !assets.isEmpty {
                List(assets, id: \.assetId, selection: $workspace.selectedAssetID) { asset in
                    HStack(spacing: 8) {
                        GalleryThumbnail(reference: app.galleryProxies[asset.assetId])
                        VStack(alignment: .leading) {
                            Text(asset.displayName)
                            Text("\(asset.representationCount) representations")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    }
                    .tag(asset.assetId)
                    .task { app.requestGalleryThumbnail(assetID: asset.assetId) }
                }
                Button("Bind Selection to Layout") {
                    guard let assetID = workspace.selectedAssetID,
                          let selectedLayout else { return }
                    app.bind(assetID: assetID, to: selectedLayout)
                }
                .disabled(workspace.selectedAssetID == nil || selectedLayout == nil)
                .padding(8)
            } else {
                ContentUnavailableView(
                    workspace.galleryFilter.isEmpty ? "No Assets" : "No Matching Assets",
                    systemImage: "photo.on.rectangle.angled",
                    description: Text("Gallery reflects project Asset Context only.")
                )
            }
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
        .frame(width: 52, height: 52)
        .clipped()
        .background(.quaternary)
    }
}

private struct PrimitiveGraphView: View {
    @EnvironmentObject private var app: AppModel
    @EnvironmentObject private var workspace: WorkspaceModel

    var body: some View {
        List(selection: $workspace.selectedNodeID) {
            ForEach(app.snapshot?.nodes ?? [], id: \.nodeId) { node in
                HStack {
                    Image(systemName: "rectangle.roundedtop")
                    VStack(alignment: .leading) {
                        Text(node.displayName)
                        Text(node.definitionId)
                            .font(.caption.monospaced())
                            .foregroundStyle(.secondary)
                    }
                }
                .tag(node.nodeId)
            }
        }
        .overlay {
            if app.snapshot?.nodes.isEmpty != false {
                ContentUnavailableView(
                    "No Nodes",
                    systemImage: "point.3.connected.trianglepath.dotted",
                    description: Text("Add a Layout to begin.")
                )
            }
        }
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
        return nodes.first
    }

    var body: some View {
        if let node = selectedNode {
            Form {
                Section("Node") {
                    LabeledContent("Type", value: node.displayName)
                    LabeledContent("Version", value: String(node.definitionVersion))
                    LabeledContent("ID", value: node.nodeId)
                        .font(.caption.monospaced())
                }
                Section("Layout") {
                    LabeledContent("Canvas", value: canvasDescription(node))
                    if let layout = node.layout {
                        LabeledContent("Frames", value: String(layout.frames.count))
                        LabeledContent("Authored digest", value: layout.authoredStateDigest)
                            .font(.caption.monospaced())
                    }
                    LayoutPreview(reference: app.layoutPreview)
                    Button("Refresh Preview") {
                        app.requestLayoutPreview(for: node.nodeId)
                    }
                }
                Section("Project") {
                    LabeledContent(
                        "Graph revision",
                        value: String(app.snapshot?.graph.revision ?? 0)
                    )
                    LabeledContent("Evaluation", value: app.progressLabel)
                }
            }
            .formStyle(.grouped)
        } else {
            ContentUnavailableView(
                "No Selection",
                systemImage: "sidebar.trailing",
                description: Text("Select a Layout node in the graph list.")
            )
        }
    }

    private func canvasDescription(_ node: BridgeNodeDto) -> String {
        guard let canvas = node.layout?.canvas else { return "Unknown" }
        return "\(canvas.widthPixels) × \(canvas.heightPixels)"
    }
}

private struct LayoutPreview: View {
    let reference: BridgeProxyReference?

    var body: some View {
        Group {
            if let descriptor = reference?.descriptor(),
               let image = NSImage(contentsOfFile: descriptor.localPath) {
                let range = descriptor.dynamicRange == .hdr ? "HDR" : "SDR"
                VStack(alignment: .leading, spacing: 6) {
                    Image(nsImage: image)
                        .resizable()
                        .scaledToFit()
                        .frame(maxHeight: 220)
                    Text(verbatim: "\(range) · \(descriptor.colorSpaceId)")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            } else {
                ContentUnavailableView(
                    "No Preview",
                    systemImage: "rectangle.on.rectangle.slash",
                    description: Text("Bind a Gallery asset explicitly.")
                )
                .frame(minHeight: 130)
            }
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
