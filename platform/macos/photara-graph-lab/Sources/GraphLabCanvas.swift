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
    let nodeContent: (PhotaraGraphNode, Bool, Set<Int>, Set<Int>) -> NodeContent

    var body: some View {
        ZStack {
            GraphLabBackground(controller: controller, style: backgroundStyle, backgroundColor: backgroundColor, minorColor: minorColor, majorColor: majorColor)
            GraphLabNoodles(controller: controller, color: noodleColor)
            ForEach(controller.document.nodes) { node in
                GraphLabPlacedNode(controller: controller, node: node, content: nodeContent)
            }
            PhotaraGraphEventSurface(controller: controller)
        }
        .clipped()
        .transaction { $0.animation = nil }
        .overlay(alignment: .bottomLeading) {
            Text("Wire: drag from a port · Cut: hold Y and drag · Pan: empty, middle-drag, or two-finger scroll")
                .font(.caption)
                .foregroundStyle(.secondary)
                .padding(10)
                .background(.thinMaterial, in: RoundedRectangle(cornerRadius: 8))
                .padding(14)
                .allowsHitTesting(false)
        }
        .overlay(alignment: .bottom) {
            GraphLabZoomControl(controller: controller, reduceTransparency: reduceTransparency).padding(14)
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
            .scaleEffect(controller.camera.zoom)
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
        let lines = controller.document.connections.filter { !hidden.contains($0.id) }.map {
            ($0.id, controller.path($0), controller.knot(of: $0))
        }
        let selection = controller.selection
        let wirePath = controller.wire.flatMap { wire -> Path? in
            guard let start = controller.portPoint(wire.source) else { return nil }
            let end = wire.target.flatMap(controller.portPoint) ?? wire.location
            return controller.geometry.path(from: start, to: end, through: nil)
        }
        Canvas { context, size in
            context.translateBy(x: size.width / 2 + camera.pan.width, y: size.height / 2 + camera.pan.height)
            context.scaleBy(x: camera.zoom, y: camera.zoom)
            for (id, path, knot) in lines {
                let selected = selection == .noodle(id)
                context.stroke(path, with: .color(color.opacity(selected ? 1 : 0.92)),
                               lineWidth: max(1.5 / camera.zoom, selected ? 2.8 : 2.4))
                if let knot {
                    let circle = Path(ellipseIn: CGRect(x: knot.x - 4.5, y: knot.y - 4.5, width: 9, height: 9))
                    context.fill(circle, with: .color(Color(nsColor: .windowBackgroundColor)))
                    context.stroke(circle, with: .color(color), lineWidth: selection == .knot(id) ? 3 : 2)
                }
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
                controller.cancel()
                controller.zoom(to: $0, anchor: CGPoint(x: controller.viewport.width / 2, y: controller.viewport.height / 2))
            }), in: PhotaraGraphCamera.zoomRange)
            .frame(width: 150)
            .accessibilityIdentifier("photara.graph.zoom")
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 11)
        if reduceTransparency { control.background(.regularMaterial, in: Capsule()) }
        else { control.glassEffect(.regular.interactive(), in: Capsule()) }
    }
}
