import SwiftUI

enum PhotaraGraphOverviewPolicy: String, Codable, CaseIterable, Identifiable {
    case whileZooming, always, never
    var id: String { rawValue }
    var title: String {
        switch self {
        case .whileZooming: "Show While Zooming"
        case .always: "Always Show"
        case .never: "Never Show"
        }
    }
    init(savedValue: String?) { self = savedValue.flatMap(Self.init(rawValue:)) ?? .whileZooming }
}

enum PhotaraGraphOverviewPosition: String, Codable, CaseIterable, Identifiable {
    case topLeft, topRight, bottomLeft, bottomRight
    var id: String { rawValue }
    var title: String {
        switch self {
        case .topLeft: "Top Left"
        case .topRight: "Top Right"
        case .bottomLeft: "Bottom Left"
        case .bottomRight: "Bottom Right"
        }
    }
    var alignment: Alignment {
        switch self {
        case .topLeft: .topLeading
        case .topRight: .topTrailing
        case .bottomLeft: .bottomLeading
        case .bottomRight: .bottomTrailing
        }
    }
    var isBottom: Bool { self == .bottomLeft || self == .bottomRight }
    init(savedValue: String?) { self = savedValue.flatMap(Self.init(rawValue:)) ?? .topRight }
}

/// Presentation-only world geometry. The viewport participates in fitting so its
/// rectangle remains visible even when the user pans completely past the graph.
struct PhotaraGraphOverviewSnapshot {
    let nodes: [CGRect]
    let paths: [Path]
    let knots: [CGPoint]
    let graphBounds: CGRect
    let viewport: CGRect

    init(nodes: [CGRect], paths: [Path], knots: [CGPoint], camera: PhotaraGraphCamera, size: CGSize) {
        self.nodes = nodes; self.paths = paths; self.knots = knots
        var bounds = CGRect.null
        for rect in nodes { bounds = bounds.union(rect) }
        for path in paths { bounds = bounds.union(path.boundingRect) }
        for knot in knots { bounds = bounds.union(CGRect(x: knot.x - 4.5, y: knot.y - 4.5, width: 9, height: 9)) }
        graphBounds = bounds.isNull ? CGRect(x: -0.5, y: -0.5, width: 1, height: 1) : bounds
        let origin = camera.world(.zero, in: size)
        viewport = CGRect(origin: origin, size: CGSize(width: size.width / camera.zoom, height: size.height / camera.zoom))
    }
    func transform(in size: CGSize) -> CGAffineTransform {
        let bounds = graphBounds.union(viewport).insetBy(dx: -20, dy: -20)
        let scale = max(0.000001, min(max(1, size.width - 20) / max(1, bounds.width),
                                    max(1, size.height - 20) / max(1, bounds.height)))
        return CGAffineTransform(a: scale, b: 0, c: 0, d: scale,
                                 tx: size.width / 2 - bounds.midX * scale,
                                 ty: size.height / 2 - bounds.midY * scale)
    }
}

enum PhotaraGraphOverviewSizing {
    static func fraction(_ value: Double?) -> Double {
        guard let value, value.isFinite else { return 0.16 }
        return min(0.28, max(0.10, value))
    }
    static func cornerRadius(_ value: Double?) -> Double {
        guard let value, value.isFinite else { return 12 }
        return min(36, max(0, value))
    }
    static func size(in viewport: CGSize, fraction value: Double) -> CGSize {
        guard viewport.width > 0, viewport.height > 0, viewport.width.isFinite, viewport.height.isFinite else {
            return CGSize(width: 144, height: 96)
        }
        let width = min(360, max(144, viewport.width * fraction(value)))
        return CGSize(width: width, height: width * viewport.height / viewport.width)
    }
}

struct PhotaraGraphOverview: View {
    let snapshot: PhotaraGraphOverviewSnapshot
    let size: CGSize
    let cornerRadius: Double
    var body: some View {
        Canvas { context, size in
            let transform = snapshot.transform(in: size)
            for path in snapshot.paths {
                context.stroke(path.applying(transform), with: .color(.secondary.opacity(0.55)), lineWidth: 0.8)
            }
            for rect in snapshot.nodes {
                context.fill(Path(roundedRect: rect.applying(transform), cornerRadius: 2), with: .color(.primary.opacity(0.35)))
            }
            for point in snapshot.knots {
                let p = point.applying(transform)
                context.fill(Path(ellipseIn: CGRect(x: p.x - 1.5, y: p.y - 1.5, width: 3, height: 3)), with: .color(.primary))
            }
            let view = Path(snapshot.viewport.applying(transform))
            context.fill(view, with: .color(.accentColor.opacity(0.10)))
            context.stroke(view, with: .color(.accentColor), lineWidth: 1.5)
        }
        .frame(width: size.width, height: size.height)
        .background(Color(nsColor: .windowBackgroundColor).opacity(0.84), in: RoundedRectangle(cornerRadius: cornerRadius))
        .overlay { RoundedRectangle(cornerRadius: cornerRadius).strokeBorder(.primary.opacity(0.16), lineWidth: 1) }
        .allowsHitTesting(false)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Graph overview and current viewport")
        .accessibilityIdentifier("photara.graph.overview")
    }
}
