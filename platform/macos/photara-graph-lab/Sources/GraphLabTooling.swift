import AppKit
import SwiftUI

enum GraphLabSettingsKeys {
    static let knifeCursorSize = "graph-lab.knife-cursor-size"
    static let defaultKnifeCursorSize = 20.0
    static let pinnedNativeNodes = "graph-lab.pinned-native-nodes"
    static let showsToolRail = "graph-lab.shows-tool-rail"
    static let toolRailCornerRadius = "graph-lab.tool-rail-corner-radius"
    static let toolRailLightShadowOpacity = "graph-lab.tool-rail-light-shadow-opacity"
    static let toolRailDarkShadowOpacity = "graph-lab.tool-rail-dark-shadow-opacity"
    static let toolRailShadowBlur = "graph-lab.tool-rail-shadow-blur"
    static let toolRailShadowOffsetY = "graph-lab.tool-rail-shadow-offset-y"
    static let toolRailPlacement = "graph-lab.tool-rail-placement"
    static let overviewPolicy = "graph-lab.overview-policy"
    static let overviewPosition = "graph-lab.overview-position"
}

enum GraphLabToolRailPlacement: String, CaseIterable, Identifiable {
    case topCenter, leftTop, leftCenter, leftBottom, rightTop, rightCenter, rightBottom
    var id: String { rawValue }
    var title: String {
        switch self {
        case .topCenter: "Top — Centered"
        case .leftTop: "Left — Top"
        case .leftCenter: "Left — Centered"
        case .leftBottom: "Left — Bottom"
        case .rightTop: "Right — Top"
        case .rightCenter: "Right — Centered"
        case .rightBottom: "Right — Bottom"
        }
    }
    var alignment: Alignment {
        switch self {
        case .topCenter: .top
        case .leftTop: .topLeading
        case .leftCenter: .leading
        case .leftBottom: .bottomLeading
        case .rightTop: .topTrailing
        case .rightCenter: .trailing
        case .rightBottom: .bottomTrailing
        }
    }
    var isHorizontal: Bool { self == .topCenter }
    var isLeft: Bool { self == .leftTop || self == .leftCenter || self == .leftBottom }
    var isRight: Bool { self == .rightTop || self == .rightCenter || self == .rightBottom }
    var isTop: Bool { self == .topCenter || self == .leftTop || self == .rightTop }
    var isBottom: Bool { self == .leftBottom || self == .rightBottom }

    func overviewPosition(avoiding requested: PhotaraGraphOverviewPosition) -> PhotaraGraphOverviewPosition {
        switch (self, requested) {
        case (.leftTop, .topLeft): .topRight
        case (.leftBottom, .bottomLeft): .bottomRight
        case (.rightTop, .topRight): .topLeft
        case (.rightBottom, .bottomRight): .bottomLeft
        default: requested
        }
    }
}

enum GraphLabNativeNodeShortcut: String, CaseIterable, Identifiable {
    case source, transform, composite
    var id: String { rawValue }
    var title: String { GraphLabFixtures.document.nodes.first { $0.kind == rawValue }?.title ?? rawValue.capitalized }
    var presentation: PhotaraGraphNodePresentationMetadata { GraphLabFixtures.presentation(for: rawValue) }
}

struct GraphLabToolPalette: View {
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency
    @Environment(\.colorScheme) private var colorScheme
    @AppStorage(GraphLabSettingsKeys.pinnedNativeNodes) private var pinnedRaw =
        GraphLabNativeNodeShortcut.allCases.map(\.rawValue).joined(separator: ",")
    @AppStorage(GraphLabSettingsKeys.toolRailCornerRadius) private var cornerRadius = 10.0
    @AppStorage(GraphLabSettingsKeys.toolRailLightShadowOpacity) private var lightShadowOpacity = 0.18
    @AppStorage(GraphLabSettingsKeys.toolRailDarkShadowOpacity) private var darkShadowOpacity = 0.34
    @AppStorage(GraphLabSettingsKeys.toolRailShadowBlur) private var shadowBlur = 8.0
    @AppStorage(GraphLabSettingsKeys.toolRailShadowOffsetY) private var shadowOffsetY = 3.0
    @AppStorage(GraphLabSettingsKeys.toolRailPlacement) private var placementRaw = GraphLabToolRailPlacement.leftTop.rawValue
    let controller: PhotaraGraphInteractionController
    let centerScene: () -> Void
    let addNativeNode: (String) -> Void

    private var selectedConnectionID: String? {
        switch controller.selection {
        case .noodle(let id), .knot(let id): id
        default: nil
        }
    }

    private var canAddKnot: Bool {
        guard let id = selectedConnectionID,
              let edge = controller.document.connections.first(where: { $0.id == id }) else { return false }
        return edge.routingPointID == nil
    }

    private var canRemoveSelection: Bool { selectedConnectionID != nil }

    var body: some View {
        Group {
            if placement.isHorizontal {
                rail(HStack(spacing: 0) { paletteContent })
            } else {
                ViewThatFits(in: .vertical) {
                    rail(VStack(spacing: 0) { paletteContent })
                    rail(LazyVGrid(columns: [GridItem(.fixed(30), spacing: 0), GridItem(.fixed(30), spacing: 0)], spacing: 0) {
                        paletteContent
                    })
                }
            }
        }
        .accessibilityIdentifier("photara.graph.tool-palette")
    }

    private var placement: GraphLabToolRailPlacement {
        GraphLabToolRailPlacement(rawValue: placementRaw) ?? .leftTop
    }

    @ViewBuilder
    private var paletteContent: some View {
        toolButton("Center Graph", systemImage: "scope", help: "Center Graph · Fit all nodes in the viewport") { centerScene() }
        Menu {
            Button("Zoom to 55%") { setZoom(0.55) }
            Button("Zoom to 100%") { setZoom(1.0) }
            Button("Zoom to 180%") { setZoom(1.8) }
            Divider()
            Text("Mouse wheel or pinch to zoom")
        } label: {
            Image(systemName: "magnifyingglass")
                .frame(width: 30, height: 30)
                .contentShape(Rectangle())
        }
        .menuStyle(.borderlessButton)
        .menuIndicator(.hidden)
        .help("Zoom options · Mouse wheel or pinch")
        .accessibilityLabel("Zoom options")

        toolButton("Toggle Overview", systemImage: "map", help: "Graph Overview · Show or hide the graph map") {
            controller.overviewPolicy = controller.overviewPolicy == .always ? .whileZooming : .always
        }

        railDivider

        Button {
            controller.setKnifeMode(!controller.knifeMode)
        } label: {
            Image(nsImage: PhotaraGraphToolIconStore.knife)
                .resizable()
                .scaledToFit()
                .frame(width: 18, height: 18)
                .frame(width: 30, height: 30)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .background(controller.knifeMode ? Color.accentColor.opacity(0.25) : .clear)
        .help("Knife Tool (Y) · Drag across noodles to cut")
        .accessibilityLabel("Knife Tool")
        .accessibilityValue(controller.knifeMode ? "On" : "Off")

        if canAddKnot {
            toolButton("Add Routing Knot", systemImage: "point.topleft.down.to.point.bottomright.curvepath",
                       help: "Add Routing Knot · Option-click the selected noodle") {
                guard let id = selectedConnectionID else { return }
                controller.setKnot(controller.defaultKnot(connectionID: id), connectionID: id)
            }
        } else if canRemoveSelection {
            toolButton(removeTitle, systemImage: removeImage, help: "\(removeTitle) · Delete") {
                _ = controller.deleteSelection()
            }
        }

        commandsMenu

        if !pinnedShortcuts.isEmpty {
            railDivider
            ForEach(pinnedShortcuts) { shortcut in
                Button {
                    addNativeNode(shortcut.rawValue)
                } label: {
                    PhotaraGraphNodeIcon(resource: shortcut.presentation.iconResource,
                                         color: shortcut.presentation.category.color, size: 18)
                        .frame(width: 30, height: 30)
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .help("Add \(shortcut.title) Node · Right-click to unpin")
                .accessibilityLabel("Add \(shortcut.title) node")
                .contextMenu {
                    Button("Remove \(shortcut.title) Shortcut", systemImage: "pin.slash") {
                        setPinned(shortcut, false)
                    }
                }
            }
        }
    }

    private var commandsMenu: some View {
        Menu {
            Section("Selected Item") {
                Button("Add Knot to Selected Noodle     ⌥-click") {
                    guard canAddKnot, let id = selectedConnectionID else { return }
                    controller.setKnot(controller.defaultKnot(connectionID: id), connectionID: id)
                }
                .disabled(!canAddKnot)
                Button("Delete Selection          ⌫") { _ = controller.deleteSelection() }
                    .disabled(!canRemoveSelection)
            }
            Section("Native Node Shortcuts") {
                ForEach(GraphLabNativeNodeShortcut.allCases) { shortcut in
                    Button {
                        setPinned(shortcut, !isPinned(shortcut))
                    } label: {
                        Label(shortcut.title, systemImage: isPinned(shortcut) ? "pin.fill" : "pin")
                    }
                }
            }
            Section("Graph Gestures") {
                Text("Knife Tool                              Y")
                Text("Wire                              Port drag")
                Text("Branch                         Knot drag")
                Text("Move Knot                       ⌥-drag")
                Text("Pan                  Middle-drag / scroll")
                Text("Context Actions             Right-click")
            }
        } label: {
            Image(systemName: "ellipsis")
                .frame(width: 30, height: 30)
                .contentShape(Rectangle())
        }
        .menuStyle(.borderlessButton)
        .menuIndicator(.hidden)
        .help("Graph commands, shortcuts, and pinned nodes")
        .accessibilityLabel("Graph commands and shortcut customization")
    }

    private var removeTitle: String {
        if case .knot = controller.selection { return "Delete Routing Knot" }
        return "Disconnect"
    }

    private var removeImage: String {
        if case .knot = controller.selection { return "point.topleft.down.to.point.bottomright.curvepath.fill" }
        return "link.badge.minus"
    }

    private func setZoom(_ zoom: Double) {
        if controller.interaction != .idle { controller.cancel() }
        controller.zoom(to: zoom, anchor: CGPoint(x: controller.viewport.width / 2, y: controller.viewport.height / 2))
    }

    private func toolButton(_ title: String, systemImage: String, help: String, disabled: Bool = false,
                            action: @escaping () -> Void) -> some View {
        Button(title, systemImage: systemImage, action: action)
            .labelStyle(.iconOnly)
            .frame(width: 30, height: 30)
            .contentShape(Rectangle())
            .buttonStyle(.plain)
            .disabled(disabled)
            .help(help)
    }

    @ViewBuilder private var railDivider: some View {
        if placement.isHorizontal {
            Divider().frame(height: 30).accessibilityHidden(true)
        } else {
            Divider().frame(width: 30).accessibilityHidden(true)
        }
    }

    private func rail<Content: View>(_ content: Content) -> some View {
        let shape = RoundedRectangle(cornerRadius: cornerRadius)
        return Group {
            if reduceTransparency {
                content.padding(3).background(.regularMaterial, in: shape)
            } else {
                content.padding(3).glassEffect(.regular.interactive(), in: shape)
            }
        }
        .shadow(color: .black.opacity(colorScheme == .dark ? darkShadowOpacity : lightShadowOpacity),
                radius: shadowBlur, y: shadowOffsetY)
    }

    private var pinnedShortcuts: [GraphLabNativeNodeShortcut] {
        GraphLabNativeNodeShortcut.allCases.filter(isPinned)
    }

    private func isPinned(_ shortcut: GraphLabNativeNodeShortcut) -> Bool {
        Set(pinnedRaw.split(separator: ",").map(String.init)).contains(shortcut.rawValue)
    }

    private func setPinned(_ shortcut: GraphLabNativeNodeShortcut, _ pinned: Bool) {
        var values = Set(pinnedRaw.split(separator: ",").map(String.init))
        if pinned { values.insert(shortcut.rawValue) } else { values.remove(shortcut.rawValue) }
        pinnedRaw = GraphLabNativeNodeShortcut.allCases.filter { values.contains($0.rawValue) }
            .map(\.rawValue).joined(separator: ",")
    }
}

struct GraphLabSettingsView: View {
    @AppStorage(GraphLabSettingsKeys.showsToolRail) private var showsToolRail = true
    @AppStorage(GraphLabSettingsKeys.knifeCursorSize) private var knifeCursorSize = GraphLabSettingsKeys.defaultKnifeCursorSize
    @AppStorage(GraphLabSettingsKeys.toolRailPlacement) private var placementRaw = GraphLabToolRailPlacement.leftTop.rawValue
    @AppStorage(GraphLabSettingsKeys.overviewPolicy) private var overviewPolicyRaw = PhotaraGraphOverviewPolicy.whileZooming.rawValue
    @AppStorage(GraphLabSettingsKeys.overviewPosition) private var overviewPositionRaw = PhotaraGraphOverviewPosition.topRight.rawValue

    var body: some View {
        Form {
            Section("Graph Tools") {
                VStack(alignment: .leading, spacing: 6) {
                    HStack {
                        Label {
                            Text("Knife cursor size")
                        } icon: {
                            Image(nsImage: PhotaraGraphToolIconStore.knife)
                                .resizable()
                                .scaledToFit()
                                .frame(width: 16, height: 16)
                        }
                        Spacer()
                        Text("\(knifeCursorSize.formatted(.number.precision(.fractionLength(0)))) pt")
                            .foregroundStyle(.secondary)
                            .monospacedDigit()
                    }
                    Slider(value: $knifeCursorSize, in: 16...32, step: 1)
                    Text("The cursor remains a flat vector symbol; this changes only its displayed point size.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
            Section("Floating Tool Rail") {
                Toggle("Show floating tool rail", isOn: $showsToolRail)
                Picker("Position", selection: $placementRaw) {
                    ForEach(GraphLabToolRailPlacement.allCases) { placement in
                        Text(placement.title).tag(placement.rawValue)
                    }
                }
                .disabled(!showsToolRail)
                Text("Top is horizontal. Left and right can align to the top, center, or bottom. Bottom-center remains reserved for zoom.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Section("Graph Overview") {
                Picker("Visibility", selection: $overviewPolicyRaw) {
                    ForEach(PhotaraGraphOverviewPolicy.allCases) { policy in
                        Text(policy.title).tag(policy.rawValue)
                    }
                }
                Picker("Position", selection: $overviewPositionRaw) {
                    ForEach(PhotaraGraphOverviewPosition.allCases) { position in
                        Text(position.title).tag(position.rawValue)
                    }
                }
                Text("If the rail occupies the same corner, the overview moves to the opposite corner automatically.")
                    .font(.caption).foregroundStyle(.secondary)
            }
            Section {
                Text("These are user preferences. Graph colors, node styling, rail corner radius, and shadows are Photara defaults authored only in Graph Lab.")
                    .font(.caption).foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
        .frame(width: 440, height: 430)
    }

}
