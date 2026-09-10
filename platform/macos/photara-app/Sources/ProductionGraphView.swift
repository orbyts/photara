import SwiftUI

private enum PhotaraProductionGraphPresentationAdapter {
    static func presentation(iconResourceID: String, accentHex: String?,
                             catalogPath: [String]) -> PhotaraGraphNodePresentationMetadata {
        let category = PhotaraGraphNodeCategory.allCases.first { $0.title == catalogPath.first }
            ?? .utilitiesControl
        let resource: String
        switch iconResourceID {
        case "photara.disk.folder": resource = "node-disk-folder"
        case "photara.layout.compose": resource = "node-layout"
        case "photara.project.assets": resource = "node-disk-folder"
        default: resource = "node-metadata"
        }
        return .init(category: category, iconResource: resource,
                     accentColor: .init(srgbHex: accentHex))
    }
}

struct ProductionGraphView: View {
    @EnvironmentObject private var app: AppModel
    @EnvironmentObject private var workspace: WorkspaceModel
    @Environment(\.photaraTheme) private var theme
    @Environment(\.colorScheme) private var colorScheme
    @AppStorage("photara.graph.tool-rail-visible.v1") private var showsToolRail = true
    @AppStorage("photara.graph.tool-rail-placement.v1") private var placementRaw = PhotaraGraphToolRailPlacement.leftTop.rawValue
    @AppStorage("photara.graph.overview-policy.v1") private var overviewRaw = PhotaraGraphOverviewPolicy.whileZooming.rawValue
    @AppStorage("photara.graph.overview-position.v1") private var overviewPositionRaw = PhotaraGraphOverviewPosition.topRight.rawValue
    @AppStorage("photara.graph.knife-cursor-size.v1") private var knifeCursorSize = 24.0
    @State private var controller = PhotaraGraphInteractionController(document: .init(nodes: [], connections: []))
    @State private var showsCatalog = false
    @State private var nodeFilter = ""
    @State private var tabMonitor: Any?

    private var preset: PhotaraGraphPresentationPreset { .shipped }
    private var palette: PhotaraGraphPalettePreset { preset.palette(colorScheme) }
    private var placement: PhotaraGraphToolRailPlacement { .init(rawValue: placementRaw) ?? .leftTop }

    var body: some View {
        ZStack {
            PhotaraGraphCanvas(toolRailPlacement: placement, controller: controller,
                backgroundStyle: preset.backgroundStyle, backgroundColor: palette.graphBackground?.color,
                minorColor: palette.minor?.color, majorColor: palette.major?.color,
                noodleColor: palette.noodle?.color ?? theme?.color(.borderFocus) ?? .accentColor,
                knifeCursorSize: knifeCursorSize,
                showsToolRail: showsToolRail, centerScene: centerGraph, addNativeNode: addDefinition,
                nodeContent: { node, selected, inputs, outputs in
                    PhotaraGraphNodeView(node: node, presentation: presentation(for: node.id),
                        selected: selected, connectedInputs: inputs, connectedOutputs: outputs, preset: preset)
                }) {
                    PhotaraGraphToolRail(controller: controller, placement: placement, preset: preset,
                        shortcuts: shortcuts, centerScene: centerGraph, addNode: addDefinition,
                        showCatalog: { showsCatalog = true })
                }
            Color.clear.frame(width: 2, height: 2)
                .position(catalogAnchor).popover(isPresented: $showsCatalog, arrowEdge: .top) { catalog }
                .allowsHitTesting(false)
        }
        .onAppear { configure(); installTabMonitor() }
        .onDisappear { if let tabMonitor { NSEvent.removeMonitor(tabMonitor); self.tabMonitor = nil } }
        .onChange(of: app.snapshot?.graph.revision) { synchronize() }
        .onChange(of: workspace.nodeMenuRequest) { showsCatalog = true }
        .onChange(of: controller.selection) {
            if case .node(let id) = controller.selection { workspace.selectedNodeID = id }
        }
        .onChange(of: overviewRaw) { controller.overviewPolicy = .init(savedValue: overviewRaw) }
        .onChange(of: overviewPositionRaw) { controller.overviewPosition = .init(savedValue: overviewPositionRaw) }
    }

    private var catalogAnchor: CGPoint {
        let p = controller.pointerLocation ?? CGPoint(x: controller.viewport.width / 2, y: controller.viewport.height / 2)
        return CGPoint(x: min(max(24, p.x), max(24, controller.viewport.width - 24)),
                       y: min(max(24, p.y), max(24, controller.viewport.height - 24)))
    }
    private var shortcuts: [PhotaraGraphToolShortcut] {
        app.nodeDefinitions.prefix(3).map { definition in .init(id: definition.definitionId,
            title: definition.brandName, presentation: presentation(for: definition)) }
    }
    private func presentation(for nodeID: String) -> PhotaraGraphNodePresentationMetadata {
        guard let node = app.snapshot?.nodes.first(where: { $0.nodeId == nodeID }) else {
            return .init(category: .utilitiesControl, iconResource: "node-metadata")
        }
        let definition = app.nodeDefinitions.first { $0.definitionId == node.definitionId }
        return PhotaraProductionGraphPresentationAdapter.presentation(
            iconResourceID: node.iconResourceId, accentHex: node.accentSrgbHex,
            catalogPath: definition?.catalogPath ?? [])
    }
    private func presentation(for definition: BridgeAvailableNodeDefinitionDto) -> PhotaraGraphNodePresentationMetadata {
        PhotaraProductionGraphPresentationAdapter.presentation(
            iconResourceID: definition.iconResourceId, accentHex: definition.accentSrgbHex,
            catalogPath: definition.catalogPath)
    }
    private var catalog: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Add Node").font(.headline)
            TextField("Search nodes", text: $nodeFilter).textFieldStyle(.roundedBorder)
            ForEach(filteredDefinitions, id: \.definitionId) { definition in
                Button { addDefinition(definition.definitionId); showsCatalog = false } label: {
                    HStack { PhotaraGraphNodeIcon(resource: presentation(for: definition).iconResource,
                        color: presentation(for: definition).iconColor, size: 24)
                        VStack(alignment: .leading) { Text(definition.brandName); Text(definition.catalogPath.joined(separator: " › ")).font(.caption).foregroundStyle(.secondary) }
                    }.contentShape(Rectangle())
                }.buttonStyle(.plain)
            }
        }.padding(12).frame(width: 310)
    }
    private var filteredDefinitions: [BridgeAvailableNodeDefinitionDto] {
        let query = nodeFilter.trimmingCharacters(in: .whitespacesAndNewlines)
        return query.isEmpty ? app.nodeDefinitions : app.nodeDefinitions.filter {
            ([$0.brandName, $0.displayName, $0.definitionId] + $0.catalogPath + $0.searchTerms)
                .contains { $0.localizedCaseInsensitiveContains(query) }
        }
    }
    private func configure() {
        controller.commitMutation = { mutation in app.commitGraphMutation(mutation, document: controller.document) }
        controller.activateNode = { id in
            workspace.selectedNodeID = id
            if let node = app.snapshot?.nodes.first(where: { $0.nodeId == id }) {
                if node.hasWorkspace { workspace.activateWorkspace(for: id) } else { app.performDefaultActivation(for: node) }
            }
        }
        controller.configure(.init(portOffset: preset.portOffset, noodleStyle: .curved))
        controller.overviewSizeFraction = preset.overviewSizeFraction
        controller.overviewCornerRadius = preset.overviewCornerRadius
        controller.overviewPolicy = .init(savedValue: overviewRaw)
        controller.overviewPosition = .init(savedValue: overviewPositionRaw)
        synchronize()
    }
    private func synchronize() {
        guard let snapshot = app.snapshot else { return }
        try? controller.synchronizeDocument(PhotaraProductionGraphAdapter.document(snapshot))
    }
    private func centerGraph() { controller.center(positions: [:], selectedNode: workspace.selectedNodeID) }
    private func addDefinition(_ id: String) {
        guard let definition = app.nodeDefinitions.first(where: { $0.definitionId == id }) else { return }
        let point = controller.camera.world(controller.pointerLocation ?? CGPoint(x: controller.viewport.width / 2,
            y: controller.viewport.height / 2), in: controller.viewport)
        app.addNode(definition, graphPosition: .init(point)); synchronize()
    }
    private func installTabMonitor() {
        guard tabMonitor == nil else { return }
        tabMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { event in
            guard event.keyCode == 48, event.modifierFlags.intersection(.deviceIndependentFlagsMask).isEmpty,
                  !showsCatalog else { return event }
            showsCatalog = true; return nil
        }
    }
}
