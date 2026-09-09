import Foundation
import SwiftUI

extension PhotaraGraphPoint {
    init(_ point: CGPoint) { self.init(x: point.x, y: point.y) }
    var cgPoint: CGPoint { CGPoint(x: x, y: y) }
}

struct PhotaraGraphCamera: Equatable {
    static let zoomRange = 0.55...1.8
    var pan = CGSize.zero
    var zoom = 1.0
    func screen(_ world: CGPoint, in size: CGSize) -> CGPoint {
        CGPoint(x: size.width / 2 + world.x * zoom + pan.width,
                y: size.height / 2 + world.y * zoom + pan.height)
    }
    func world(_ screen: CGPoint, in size: CGSize) -> CGPoint {
        CGPoint(x: (screen.x - size.width / 2 - pan.width) / zoom,
                y: (screen.y - size.height / 2 - pan.height) / zoom)
    }
    mutating func setZoom(_ value: Double, anchor: CGPoint, size: CGSize) {
        guard value.isFinite else { return }
        let next = min(Self.zoomRange.upperBound, max(Self.zoomRange.lowerBound, value))
        let worldAnchor = world(anchor, in: size)
        zoom = next
        pan = CGSize(width: anchor.x - size.width / 2 - worldAnchor.x * next,
                     height: anchor.y - size.height / 2 - worldAnchor.y * next)
    }
}

enum PhotaraGraphNoodleStyle: String, CaseIterable, Identifiable {
    case straight, curved
    var id: String { rawValue }
    var title: String { rawValue.capitalized }
}

/// One geometry definition shared by rendering, picking, snapping and cutting.
struct PhotaraGraphGeometry: Equatable {
    var portOffset = 0.0
    var noodleStyle = PhotaraGraphNoodleStyle.curved
    static let nodeWidth = 224.0
    static let rowHeight = 25.0
    static let firstPortY = 74.5
    static func size(of node: PhotaraGraphNode) -> CGSize {
        CGSize(width: nodeWidth, height: 74 + Double(max(1, max(node.ports(.input).count, node.ports(.output).count))) * rowHeight)
    }
    static func rect(of node: PhotaraGraphNode, at point: CGPoint) -> CGRect {
        let size = size(of: node)
        return CGRect(x: point.x - size.width / 2, y: point.y - size.height / 2,
                      width: size.width, height: size.height)
    }
    func port(_ id: PhotaraGraphPortID, node: PhotaraGraphNode, position: CGPoint) -> CGPoint? {
        guard let port = node.ports.first(where: { $0.id == id.key }),
              let index = node.ports(port.direction).firstIndex(where: { $0.id == id.key }) else { return nil }
        let size = Self.size(of: node)
        return CGPoint(x: position.x + (port.direction == .input ? -1 : 1) * (size.width / 2 + portOffset),
                       y: position.y - size.height / 2 + Self.firstPortY + Double(index) * Self.rowHeight)
    }
    func path(from start: CGPoint, to end: CGPoint, through knot: CGPoint?) -> Path {
        var path = Path()
        path.move(to: start)
        guard noodleStyle == .curved else {
            if let knot { path.addLine(to: knot) }
            path.addLine(to: end)
            return path
        }
        let lead = max(28, min(90, abs(end.x - start.x) * 0.36))
        guard let knot else {
            path.addCurve(to: end, control1: CGPoint(x: start.x + lead, y: start.y),
                          control2: CGPoint(x: end.x - lead, y: end.y))
            return path
        }
        let dx = (end.x - start.x) * 0.14, dy = (end.y - start.y) * 0.14
        path.addCurve(to: knot, control1: CGPoint(x: start.x + lead, y: start.y),
                      control2: CGPoint(x: knot.x - dx, y: knot.y - dy))
        path.addCurve(to: end, control1: CGPoint(x: knot.x + dx, y: knot.y + dy),
                      control2: CGPoint(x: end.x - lead, y: end.y))
        return path
    }

    /// Axis sweeps from the *last accepted* position prevent tunneling even
    /// with coalesced/large pointer moves and allow sliding along obstacles.
    static func move(_ node: PhotaraGraphNode, from start: CGPoint, to target: CGPoint,
                     among nodes: [PhotaraGraphNode]) -> CGPoint {
        let size = size(of: node)
        let obstacles = nodes.filter { $0.id != node.id }.map {
            rect(of: $0, at: $0.position.cgPoint).insetBy(dx: -10, dy: -10)
        }
        // Screen/world round trips can place a touching center a few ULPs
        // beyond its boundary. Compare with tolerance before sweeping.
        let epsilon = 0.0000001
        var point = start
        var x = target.x
        for rect in obstacles where start.y + size.height / 2 > rect.minY && start.y - size.height / 2 < rect.maxY {
            if target.x > start.x, start.x <= rect.minX - size.width / 2 + epsilon { x = min(x, rect.minX - size.width / 2) }
            if target.x < start.x, start.x >= rect.maxX + size.width / 2 - epsilon { x = max(x, rect.maxX + size.width / 2) }
        }
        point.x = x
        var y = target.y
        for rect in obstacles where x + size.width / 2 > rect.minX && x - size.width / 2 < rect.maxX {
            if target.y > start.y, start.y <= rect.minY - size.height / 2 + epsilon { y = min(y, rect.minY - size.height / 2) }
            if target.y < start.y, start.y >= rect.maxY + size.height / 2 - epsilon { y = max(y, rect.maxY + size.height / 2) }
        }
        point.y = y
        return point
    }
}
