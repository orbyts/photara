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
        ViewThatFits(in: .vertical) {
            rail(VStack(spacing: 0) { paletteContent })
            rail(LazyVGrid(columns: [GridItem(.fixed(30), spacing: 0), GridItem(.fixed(30), spacing: 0)], spacing: 0) {
                paletteContent
            })
        }
        .accessibilityIdentifier("photara.graph.tool-palette")
    }

    @ViewBuilder
    private var paletteContent: some View {
        toolButton("Center Graph", systemImage: "scope", help: "Center Graph") { centerScene() }
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

        toolButton("Toggle Overview", systemImage: "map", help: "Show or hide graph overview") {
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
                       help: "Add Routing Knot to selected noodle · Option-click") {
                guard let id = selectedConnectionID else { return }
                controller.setKnot(controller.defaultKnot(connectionID: id), connectionID: id)
            }
        } else if canRemoveSelection {
            toolButton(removeTitle, systemImage: removeImage, help: "\(removeTitle) (Delete)") {
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
                .help("Add \(shortcut.title) node")
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

    private var railDivider: some View {
        Divider().frame(width: 30).accessibilityHidden(true)
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
    @AppStorage(GraphLabSettingsKeys.toolRailCornerRadius) private var cornerRadius = 10.0
    @AppStorage(GraphLabSettingsKeys.toolRailLightShadowOpacity) private var lightShadowOpacity = 0.18
    @AppStorage(GraphLabSettingsKeys.toolRailDarkShadowOpacity) private var darkShadowOpacity = 0.34
    @AppStorage(GraphLabSettingsKeys.toolRailShadowBlur) private var shadowBlur = 8.0
    @AppStorage(GraphLabSettingsKeys.toolRailShadowOffsetY) private var shadowOffsetY = 3.0

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
                Group {
                    settingSlider("Corner radius", value: $cornerRadius, range: 4...24, suffix: " pt")
                    settingSlider("Light shadow opacity", value: $lightShadowOpacity, range: 0...0.5)
                    settingSlider("Dark shadow opacity", value: $darkShadowOpacity, range: 0...0.5)
                    settingSlider("Shadow blur", value: $shadowBlur, range: 0...24, suffix: " pt")
                    settingSlider("Shadow vertical offset", value: $shadowOffsetY, range: -4...12, suffix: " pt")
                }
                .disabled(!showsToolRail)
                Text("The rail uses Apple's native translucent material. Icons and node shortcuts remain flat.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
        .frame(width: 440, height: 440)
    }

    private func settingSlider(_ title: String, value: Binding<Double>, range: ClosedRange<Double>, suffix: String = "") -> some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(title)
                Spacer()
                Text(value.wrappedValue.formatted(.number.precision(.fractionLength(2))) + suffix)
                    .foregroundStyle(.secondary)
                    .monospacedDigit()
            }
            Slider(value: value, in: range)
        }
    }
}
