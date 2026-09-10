import SwiftUI

enum PhotaraGraphToolRailPlacement: String, CaseIterable, Identifiable {
    case topCenter, leftTop, leftCenter, leftBottom, rightTop, rightCenter, rightBottom
    var id: String { rawValue }
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
    var isLeft: Bool { [.leftTop, .leftCenter, .leftBottom].contains(self) }
    var isRight: Bool { [.rightTop, .rightCenter, .rightBottom].contains(self) }
    var isTop: Bool { [.topCenter, .leftTop, .rightTop].contains(self) }
    var isBottom: Bool { [.leftBottom, .rightBottom].contains(self) }
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

struct PhotaraGraphToolShortcut: Identifiable {
    let id: String
    let title: String
    let presentation: PhotaraGraphNodePresentationMetadata
}

struct PhotaraGraphToolRail: View {
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency
    @Environment(\.colorScheme) private var colorScheme
    let controller: PhotaraGraphInteractionController
    let placement: PhotaraGraphToolRailPlacement
    let preset: PhotaraGraphPresentationPreset
    let shortcuts: [PhotaraGraphToolShortcut]
    let centerScene: () -> Void
    let addNode: (String) -> Void
    let showCatalog: () -> Void

    var body: some View {
        Group {
            if placement.isHorizontal { rail(HStack(spacing: 0) { content }) }
            else { rail(VStack(spacing: 0) { content }) }
        }
        .accessibilityIdentifier("photara.graph.tool-palette")
    }

    @ViewBuilder private var content: some View {
        button("Center Graph", "scope", "Center Graph · Fit all nodes in the viewport", centerScene)
        button("Add Node", "plus", "Add Node · Tab", showCatalog)
        button("Toggle Overview", "map", "Graph Overview · Show or hide the graph map") {
            controller.overviewPolicy = controller.overviewPolicy == .always ? .whileZooming : .always
        }
        divider
        Button { controller.setKnifeMode(!controller.knifeMode) } label: {
            Image(nsImage: PhotaraGraphToolIconStore.knife).resizable().scaledToFit()
                .frame(width: 18, height: 18).frame(width: 30, height: 30).contentShape(Rectangle())
        }
        .buttonStyle(.plain).background(controller.knifeMode ? Color.accentColor.opacity(0.25) : .clear)
        .help("Knife Tool (Y) · Drag across noodles to cut")
        if case .noodle(let id) = controller.selection,
           controller.document.connections.first(where: { $0.id == id })?.routingPointID == nil {
            button("Add Routing Knot", "point.topleft.down.to.point.bottomright.curvepath", "Add Routing Knot · Option-click the selected noodle") {
                controller.setKnot(controller.defaultKnot(connectionID: id), connectionID: id)
            }
        } else if case .noodle = controller.selection {
            button("Delete Selection", "trash", "Delete selected noodle · Delete") { _ = controller.deleteSelection() }
        } else if case .knot = controller.selection {
            button("Delete Selection", "trash", "Delete selected noodle or routing knot · Delete") { _ = controller.deleteSelection() }
        }
        if !shortcuts.isEmpty {
            divider
            ForEach(shortcuts) { shortcut in
                Button { addNode(shortcut.id) } label: {
                    PhotaraGraphNodeIcon(resource: shortcut.presentation.iconResource,
                        color: shortcut.presentation.iconColor, size: 18)
                        .frame(width: 30, height: 30).contentShape(Rectangle())
                }.buttonStyle(.plain).help("Add \(shortcut.title) Node")
            }
        }
    }

    private func button(_ title: String, _ image: String, _ help: String, _ action: @escaping () -> Void) -> some View {
        Button(title, systemImage: image, action: action).labelStyle(.iconOnly)
            .frame(width: 30, height: 30).contentShape(Rectangle()).buttonStyle(.plain).help(help)
    }
    @ViewBuilder private var divider: some View {
        if placement.isHorizontal { Divider().frame(height: 30) } else { Divider().frame(width: 30) }
    }
    private func rail<Content: View>(_ content: Content) -> some View {
        let shape = RoundedRectangle(cornerRadius: preset.toolRailCornerRadius)
        return Group {
            if reduceTransparency { content.padding(3).background(.regularMaterial, in: shape) }
            else { content.padding(3).glassEffect(.regular.interactive(), in: shape) }
        }.shadow(color: .black.opacity(colorScheme == .dark ? preset.toolRailDarkShadowOpacity : preset.toolRailLightShadowOpacity),
                 radius: preset.toolRailShadowBlur, y: preset.toolRailShadowOffsetY)
    }
}
