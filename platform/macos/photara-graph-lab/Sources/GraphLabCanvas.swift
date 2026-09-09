import SwiftUI

/// Only this subtree observes camera/interaction changes. The authoring form
/// and saved palettes do not subscribe to pointer motion.
struct GraphLabCanvas<NodeContent: View>: View {
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency
    let controller: PhotaraGraphInteractionController
    let backgroundStyle: PhotaraGraphBackgroundStyle
    let backgroundColor: Color?
    let minorColor: Color?
    let majorColor: Color?
    let noodleColor: Color
    let knifeCursorSize: Double
    let showsToolRail: Bool
    let centerScene: () -> Void
    let addNativeNode: (String) -> Void
    let nodeContent: (PhotaraGraphNode, Bool, Set<Int>, Set<Int>) -> NodeContent

    var body: some View {
        ZStack {
            GraphLabBackground(controller: controller, style: backgroundStyle, backgroundColor: backgroundColor, minorColor: minorColor, majorColor: majorColor)
            GraphLabNoodles(controller: controller, color: noodleColor)
            ForEach(controller.document.nodes) { node in
                GraphLabPlacedNode(controller: controller, node: node, content: nodeContent)
            }
            PhotaraGraphEventSurface(controller: controller, knifeCursorSize: knifeCursorSize)
        }
        .clipped()
        .transaction { $0.animation = nil }
        .overlay(alignment: .bottomLeading) {
            Text("Wire: port/knot drag · Move knot: Option-drag · Cut: Y-drag · Pan: canvas, middle-drag, or scroll")
                .font(.caption)
                .foregroundStyle(.secondary)
                .padding(10)
                .background(.thinMaterial, in: RoundedRectangle(cornerRadius: 8))
                .padding(14)
                .allowsHitTesting(false)
        }
        .overlay(alignment: controller.overviewPosition.alignment) {
            GraphLabOverviewOverlay(controller: controller)

        }
        .overlay(alignment: .bottom) {
            GraphLabZoomControl(controller: controller, reduceTransparency: reduceTransparency).padding(14)
        }
        .overlay(alignment: .topLeading) {
            if showsToolRail {
                GraphLabToolPalette(controller: controller, centerScene: centerScene, addNativeNode: addNativeNode)
                    .padding(.leading, 14)
                    .padding(.top, 14)
            }
        }
    }
}

private struct GraphLabBackground: View {
    let controller: PhotaraGraphInteractionController
    let style: PhotaraGraphBackgroundStyle
    let backgroundColor: Color?
    let minorColor: Color?
    let majorColor: Color?
    var body: some View {
        PhotaraGraphBackground(pan: controller.camera.pan, zoom: controller.camera.zoom, style: style,
                               backgroundColor: backgroundColor, minorColor: minorColor, majorColor: majorColor)
            .allowsHitTesting(false)
    }
}

private struct GraphLabPlacedNode<Content: View>: View {
    let controller: PhotaraGraphInteractionController
    let node: PhotaraGraphNode
    let content: (PhotaraGraphNode, Bool, Set<Int>, Set<Int>) -> Content
    var body: some View {
        content(node, controller.selection == .node(node.id), controller.activePorts(for: node, direction: .input),
                controller.activePorts(for: node, direction: .output))
            .environment(\.photaraGraphPresentationZoom, controller.camera.zoom)
            .position(controller.camera.screen(controller.position(of: node), in: controller.viewport))
            .allowsHitTesting(false)
    }
}

private struct GraphLabNoodles: View {
    let controller: PhotaraGraphInteractionController
    let color: Color
    var body: some View {
        // Read the snapshot during body evaluation so Observation tracks every
        // dependency even if Canvas schedules its rendering closure later.
        let camera = controller.camera
        let hidden = controller.hiddenConnections
        let visible = controller.document.connections.filter { !hidden.contains($0.id) }
        var trunks: [String: Path] = [:]
        var knots: [String: CGPoint] = [:]
        let lines = visible.map { edge in
            var path = controller.path(edge)
            if let id = edge.routingPointID, let knot = controller.knot(of: edge) {
                knots[id] = knot
                if controller.sharedRoutingPoint(edge), let start = controller.portPoint(edge.source),
                   let end = controller.portPoint(edge.destination) {
                    trunks[id] = controller.geometry.path(from: start, to: knot, through: nil)
                    path = controller.geometry.path(from: knot, to: end, through: nil)
                }
            }
            return (edge.id, path)
        }
        let selectedRoute: String? = {
            guard case .knot(let id) = controller.selection else { return nil }
            return controller.document.connections.first { $0.id == id }?.routingPointID
        }()
        let selection = controller.selection
        let wirePath = controller.wire.flatMap { wire -> Path? in
            guard let start = controller.wireStart else { return nil }
            let end = wire.target.flatMap(controller.portPoint) ?? wire.location
            return controller.geometry.path(from: start, to: end, through: nil)
        }
        Canvas { context, size in
            context.translateBy(x: size.width / 2 + camera.pan.width, y: size.height / 2 + camera.pan.height)
            context.scaleBy(x: camera.zoom, y: camera.zoom)
            for path in trunks.values {
                context.stroke(path, with: .color(color.opacity(0.92)), lineWidth: max(1.5 / camera.zoom, 2.4))
            }
            for (id, path) in lines {
                let selected = selection == .noodle(id)
                context.stroke(path, with: .color(color.opacity(selected ? 1 : 0.92)),
                               lineWidth: max(1.5 / camera.zoom, selected ? 2.8 : 2.4))
            }
            for (id, knot) in knots {
                let circle = Path(ellipseIn: CGRect(x: knot.x - 4.5, y: knot.y - 4.5, width: 9, height: 9))
                context.fill(circle, with: .color(Color(nsColor: .windowBackgroundColor)))
                context.stroke(circle, with: .color(color), lineWidth: selectedRoute == id ? 3 : 2)
            }
            if let wirePath { context.stroke(wirePath, with: .color(color.opacity(0.92)), lineWidth: max(1.5 / camera.zoom, 2.4)) }
        }
        .allowsHitTesting(false)
    }
}

private struct GraphLabZoomControl: View {
    let controller: PhotaraGraphInteractionController
    let reduceTransparency: Bool
    var body: some View {
        let control = HStack(spacing: 12) {
            Text("Zoom \(Int(controller.camera.zoom * 100))%")
                .font(.caption.monospacedDigit())
                .frame(width: 74, alignment: .leading)
            Slider(value: Binding(get: { controller.camera.zoom }, set: {
                if controller.interaction != .idle { controller.cancel() }
                controller.zoom(to: $0, anchor: CGPoint(x: controller.viewport.width / 2, y: controller.viewport.height / 2))
            }), in: PhotaraGraphCamera.zoomRange, onEditingChanged: { editing in
                if editing { controller.beginZoomGesture() }
                else { controller.endZoomGesture() }
            })
            .frame(width: 150)
            .accessibilityIdentifier("photara.graph.zoom")
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 11)
        if reduceTransparency { control.background(.regularMaterial, in: Capsule()) }
        else { control.glassEffect(.regular.interactive(), in: Capsule()) }
    }
}

private struct GraphLabOverviewOverlay: View {
    let controller: PhotaraGraphInteractionController
    var body: some View {
        ZStack {
            if controller.overviewVisible {
                PhotaraGraphOverview(snapshot: controller.overviewSnapshot,
                    size: PhotaraGraphOverviewSizing.size(in: controller.viewport, fraction: controller.overviewSizeFraction),
                    cornerRadius: controller.overviewCornerRadius)
                    .padding(.horizontal, 14)
                    .padding(.top, 14)
                    .padding(.bottom, controller.overviewPosition.isBottom ? 64 : 14)
                    .transition(.opacity)
            }
        }
        .animation(.easeOut(duration: 0.18), value: controller.overviewVisible)
        .allowsHitTesting(false)
    }
}
