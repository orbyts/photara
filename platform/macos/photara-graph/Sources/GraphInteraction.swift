import Foundation
import Observation
import SwiftUI

enum PhotaraGraphSelection: Equatable {
    case node(String), noodle(String), knot(String)
}

enum PhotaraGraphMutation {
    case connect(source: PhotaraGraphPortID, destination: PhotaraGraphPortID, replacing: String?, routingPointID: String?)
    case moveNode(id: String, position: PhotaraGraphPoint)
    case removeConnections(Set<String>)
    case setRouting(connectionID: String, point: PhotaraGraphPoint?)
}

enum PhotaraGraphHit: Equatable {
    case port(PhotaraGraphPortID), node(String), noodle(String), knot(String), canvas
}

struct PhotaraGraphWireDrag: Equatable {
    let source: PhotaraGraphPortID
    let originalConnection: String?
    var routingPointID: String? = nil
    var location: CGPoint
    var target: PhotaraGraphPortID?
}

/// The only primary gesture state. No parallel gesture flags or live-offset
/// dictionaries exist. A cancelled sequence stays owned by the event surface
/// until its matching release; subsequent drags cannot resurrect it.
enum PhotaraGraphInteraction: Equatable {
    case idle
    case node(id: String, anchor: CGPoint, original: CGPoint, current: CGPoint)
    case pan(anchor: CGPoint, original: PhotaraGraphCamera)
    case wire(PhotaraGraphWireDrag)
    case knife(last: CGPoint, crossed: Set<String>)
    case knot(id: String, anchor: CGPoint, original: CGPoint, current: CGPoint)
    case pressed(PhotaraGraphHit)
}

@MainActor @Observable
final class PhotaraGraphInteractionController {
    private(set) var document: PhotaraGraphDocument
    private(set) var interaction = PhotaraGraphInteraction.idle
    private(set) var selection: PhotaraGraphSelection?
    private(set) var camera = PhotaraGraphCamera()
    private(set) var knifeMode = false
    private(set) var pointerLocation: CGPoint?
    private(set) var geometry = PhotaraGraphGeometry()
    var viewport = CGSize.zero
    var overviewSizeFraction = 0.16
    var overviewPosition = PhotaraGraphOverviewPosition.topRight
    var overviewCornerRadius = 12.0
    var overviewPolicy = PhotaraGraphOverviewPolicy.whileZooming {
        didSet { if overviewPolicy != oldValue { endZoomActivity() } }
    }
    private(set) var zoomActive = false
    @ObservationIgnored private var zoomGestureHeld = false
    @ObservationIgnored private var zoomDismissal: Task<Void, Never>?
    var overviewVisible: Bool { overviewPolicy == .always || (overviewPolicy == .whileZooming && zoomActive) }
    private func noteZoomActivity() {
        zoomDismissal?.cancel()
        zoomActive = true
        guard !zoomGestureHeld else { return }
        zoomDismissal = Task { [weak self] in
            do { try await Task.sleep(for: .milliseconds(650)) } catch { return }
            guard !Task.isCancelled else { return }
            self?.zoomActive = false
            self?.zoomDismissal = nil
        }
    }
    func beginZoomGesture() {
        guard interaction == .idle else { return }
        zoomGestureHeld = true
        noteZoomActivity()
    }
    func endZoomGesture() {
        guard zoomGestureHeld else { return }
        zoomGestureHeld = false
        noteZoomActivity()
    }
    var overviewSnapshot: PhotaraGraphOverviewSnapshot {
        PhotaraGraphOverviewSnapshot(
            nodes: document.nodes.map { PhotaraGraphGeometry.rect(of: $0, at: position(of: $0)) },
            paths: document.connections.map(path),
            knots: document.connections.compactMap { knot(of: $0) }, camera: camera, size: viewport)
    }
    func endZoomActivity() {
        zoomGestureHeld = false
        zoomDismissal?.cancel()
        zoomDismissal = nil
        zoomActive = false
    }
    @ObservationIgnored private var knifePaths: [(String, Path)] = []
    @ObservationIgnored var commitMutation: ((PhotaraGraphMutation) -> PhotaraGraphDocument?)?
    @ObservationIgnored var activateNode: ((String) -> Void)?

    func notePointer(at point: CGPoint) { pointerLocation = point }

    init(document: PhotaraGraphDocument, selection: PhotaraGraphSelection? = nil,
         commitMutation: ((PhotaraGraphMutation) -> PhotaraGraphDocument?)? = nil) {
        precondition((try? document.validate()) != nil, "Invalid graph document")
        self.document = document
        self.selection = selection
        self.commitMutation = commitMutation
    }

    var wire: PhotaraGraphWireDrag? {
        if case .wire(let drag) = interaction { return drag }
        return nil
    }
    var wireStart: CGPoint? {
        guard let wire else { return nil }
        if let id = wire.routingPointID { return document.routingPoints.first { $0.id == id }?.position.cgPoint }
        return portPoint(wire.source)
    }
    var isPanning: Bool {
        if case .pan = interaction { return true }
        return false
    }
    var hiddenConnections: Set<String> {
        if case .knife(_, let crossed) = interaction { return crossed }
        if let id = wire?.originalConnection { return [id] }
        return []
    }
    func position(of node: PhotaraGraphNode) -> CGPoint {
        if case .node(let id, _, _, let current) = interaction, node.id == id { return current }
        return node.position.cgPoint
    }
    func knot(of connection: PhotaraGraphConnection) -> CGPoint? {
        guard connection.routingPointID != nil else { return nil }
        if case .knot(let id, _, _, let current) = interaction,
           let moving = document.connections.first(where: { $0.id == id }),
           moving.routingPointID == connection.routingPointID { return current }
        return document.knot(of: connection)?.cgPoint
    }
    func portPoint(_ id: PhotaraGraphPortID) -> CGPoint? {
        guard let node = document.nodes.first(where: { $0.id == id.node }) else { return nil }
        return geometry.port(id, node: node, position: position(of: node))
    }
    func sharedRoutingPoint(_ connection: PhotaraGraphConnection) -> Bool {
        document.routingPoint(for: connection)?.isJunction == true
    }
    func path(_ connection: PhotaraGraphConnection) -> Path {
        guard let start = portPoint(connection.source), let end = portPoint(connection.destination) else { return Path() }
        guard connection.routingPointID != nil else { return geometry.path(from: start, to: end, through: nil) }
        if sharedRoutingPoint(connection), let knot = knot(of: connection) {
            var path = geometry.path(from: start, to: knot, through: nil)
            path.addPath(geometry.path(from: knot, to: end, through: nil))
            return path
        }
        return geometry.path(from: start, to: end, through: knot(of: connection))
    }
    func activePorts(for node: PhotaraGraphNode, direction: PhotaraGraphPortDirection) -> Set<Int> {
        var ports = Set(document.connections.filter { !hiddenConnections.contains($0.id) }.map {
            direction == .input ? $0.destination : $0.source
        })
        if let wire {
            ports.insert(wire.source)
            if let target = wire.target { ports.insert(target) }
        }
        return Set(node.ports(direction).enumerated().compactMap { index, port in
            ports.contains(.init(node: node.id, key: port.id)) ? index : nil
        })
    }

    func configure(_ geometry: PhotaraGraphGeometry) {
        guard self.geometry != geometry else { return }
        cancel()
        self.geometry = geometry
    }
    func setKnifeMode(_ enabled: Bool) {
        guard knifeMode != enabled else { return }
        cancel()
        knifeMode = enabled
    }
    func cancel(resetTool: Bool = false) {
        endZoomActivity()
        if case .pan(_, let original) = interaction { camera = original }
        interaction = .idle
        knifePaths.removeAll(keepingCapacity: true)
        if resetTool { knifeMode = false }
    }

    /// Ports win over their node; nodes occlude wires and knots beneath them.
    /// Targets are nearest within a row-bounded rectangle, never ambiguous at
    /// the minimum zoom even when the pointer tolerance expands horizontally.
    func hitTest(_ screen: CGPoint) -> PhotaraGraphHit {
        let point = camera.world(screen, in: viewport)
        if let port = nearestPort(to: point, source: nil, dropping: false) { return .port(port) }
        for node in document.nodes.reversed() {
            if PhotaraGraphGeometry.rect(of: node, at: position(of: node)).contains(point) { return .node(node.id) }
        }
        for connection in document.connections.reversed() {
            if let knot = knot(of: connection), hypot(point.x - knot.x, point.y - knot.y) <= 12 { return .knot(connection.id) }
        }
        for connection in document.connections.reversed() {
            if path(connection).strokedPath(StrokeStyle(lineWidth: 18 / camera.zoom)).contains(point) { return .noodle(connection.id) }
        }
        return .canvas
    }

    private func nearestPort(to point: CGPoint, source: PhotaraGraphPortID?, dropping: Bool) -> PhotaraGraphPortID? {
        var nearest: (PhotaraGraphPortID, CGFloat)?
        // Adjacent rows have disjoint vertical targets; their midline is a gap.
        let halfHeight = dropping ? 12.0 : 11.0
        let halfWidth = dropping ? max(24, 24 / camera.zoom) : max(18, 12 / camera.zoom)
        for node in document.nodes.reversed() {
            for port in node.ports {
                let id = PhotaraGraphPortID(node: node.id, key: port.id)
                if let source, !document.canConnect(source, id) { continue }
                guard let center = portPoint(id) else { continue }
                let dx = abs(point.x - center.x), dy = abs(point.y - center.y)
                guard dx <= halfWidth, dy <= halfHeight else { continue }
                let distance = dx * dx + dy * dy
                if nearest == nil || distance < nearest!.1 { nearest = (id, distance) }
            }
        }
        return nearest?.0
    }

    func pointerDown(at screen: CGPoint, middle: Bool = false, option: Bool = false) {
        guard interaction == .idle else { return }
        let point = camera.world(screen, in: viewport)
        if middle { interaction = .pan(anchor: screen, original: camera); return }
        if knifeMode {
            knifePaths = document.connections.map { ($0.id, path($0).strokedPath(StrokeStyle(lineWidth: max(2.4, 2 / camera.zoom)))) }
            interaction = .knife(last: point, crossed: [])
            return
        }
        let hit = hitTest(screen)
        switch hit {
        case .port(let port):
            if document.port(port)?.direction == .output {
                interaction = .wire(.init(source: port, originalConnection: nil, location: point))
            } else if let connection = document.connections.first(where: { $0.destination == port }) {
                selection = .noodle(connection.id)
                interaction = .wire(.init(source: connection.source, originalConnection: connection.id,
                                          location: point, target: port))
            } else {
                interaction = .pressed(hit)
            }
        case .node(let id):
            guard let node = document.nodes.first(where: { $0.id == id }) else { return }
            selection = .node(id)
            interaction = .node(id: id, anchor: point, original: node.position.cgPoint, current: node.position.cgPoint)
        case .knot(let id):
            guard let connection = document.connections.first(where: { $0.id == id }),
                  let knot = knot(of: connection) else { return }
            selection = .knot(id)
            if option {
                interaction = .knot(id: id, anchor: point, original: knot, current: knot)
            } else {
                interaction = .wire(.init(source: connection.source, originalConnection: nil,
                    routingPointID: connection.routingPointID, location: knot))
            }
        case .noodle(let id):
            if option, document.connections.first(where: { $0.id == id })?.routingPointID == nil { setKnot(point, connectionID: id) }
            else { selection = .noodle(id) }
            interaction = .pressed(hit)
        case .canvas:
            selection = nil
            interaction = .pan(anchor: screen, original: camera)
        }
    }

    func pointerDragged(to screen: CGPoint) {
        let point = camera.world(screen, in: viewport)
        switch interaction {
        case .node(let id, let anchor, let original, let current):
            guard let node = document.nodes.first(where: { $0.id == id }) else { return }
            let target = CGPoint(x: original.x + point.x - anchor.x, y: original.y + point.y - anchor.y)
            let allowed = PhotaraGraphGeometry.move(node, from: current, to: target, among: document.nodes)
            interaction = .node(id: id, anchor: anchor, original: original, current: allowed)
        case .pan(let anchor, let original):
            camera.pan = CGSize(width: original.pan.width + screen.x - anchor.x, height: original.pan.height + screen.y - anchor.y)
        case .wire(var wire):
            wire.location = point
            wire.target = nearestPort(to: point, source: wire.source, dropping: true)
            interaction = .wire(wire)
        case .knot(let id, let anchor, let original, _):
            interaction = .knot(id: id, anchor: anchor, original: original,
                                current: CGPoint(x: original.x + point.x - anchor.x, y: original.y + point.y - anchor.y))
        case .knife(let last, var crossed):
            let steps = max(1, Int(ceil(hypot(point.x - last.x, point.y - last.y) * camera.zoom)))
            for (id, area) in knifePaths where !crossed.contains(id) {
                for step in 0...steps {
                    let t = Double(step) / Double(steps)
                    if area.contains(CGPoint(x: last.x + (point.x - last.x) * t, y: last.y + (point.y - last.y) * t)) {
                        crossed.insert(id)
                        break
                    }
                }
            }
            interaction = .knife(last: point, crossed: crossed)
        case .idle, .pressed: break
        }
    }

    func pointerUp(at screen: CGPoint) {
        pointerDragged(to: screen)
        let completed = interaction
        interaction = .idle
        knifePaths.removeAll(keepingCapacity: true)
        switch completed {
        case .wire(let wire):
            // The release position is authoritative. Never fall back to the
            // previous hover target when a release lands on empty canvas.
            if let target = wire.target {
                commitConnect(wire.source, target, replacing: wire.originalConnection, routingPointID: wire.routingPointID)
            }
        case .node(let id, _, _, let current):
            let point = PhotaraGraphPoint(current)
            if let committed = commitMutation?(.moveNode(id: id, position: point)) { document = committed }
            else if commitMutation == nil, let index = document.nodes.firstIndex(where: { $0.id == id }) { document.nodes[index].position = point }
        case .knot(let id, _, _, let current): setKnot(current, connectionID: id)
        case .knife(_, let crossed): removeConnections(crossed)
        default: break
        }
    }

    func zoom(to value: Double, anchor: CGPoint) {
        guard interaction == .idle else { return }
        guard value.isFinite else { return }
        noteZoomActivity()
        camera.setZoom(value, anchor: anchor, size: viewport)
    }
    func scroll(by delta: CGSize) {
        guard interaction == .idle else { return }
        camera.pan.width += delta.width
        camera.pan.height += delta.height
    }
    func select(_ hit: PhotaraGraphHit) {
        cancel()
        switch hit {
        case .node(let id): selection = .node(id)
        case .noodle(let id): selection = .noodle(id)
        case .knot(let id): selection = .knot(id)
        default: break
        }
    }
    func connect(_ source: PhotaraGraphPortID, to destination: PhotaraGraphPortID) {
        cancel()
        commitConnect(source, destination, replacing: nil, routingPointID: nil)
    }
    private func commitConnect(_ source: PhotaraGraphPortID, _ destination: PhotaraGraphPortID,
                               replacing: String?, routingPointID: String?) {
        if let committed = commitMutation?(.connect(source: source, destination: destination,
                                                     replacing: replacing, routingPointID: routingPointID)) {
            document = committed
            selection = committed.connections.first { $0.source == source && $0.destination == destination }.map { .noodle($0.id) }
        } else if commitMutation == nil,
                  let id = document.connect(from: source, to: destination, replacing: replacing, via: routingPointID) {
            selection = .noodle(id)
        }
    }
    func disconnect(port: PhotaraGraphPortID) {
        removeConnections(Set(document.connections.filter { $0.source == port || $0.destination == port }.map(\.id)))
    }
    func removeConnections(_ ids: Set<String>) {
        cancel()
        if let committed = commitMutation?(.removeConnections(ids)) { document = committed }
        else if commitMutation == nil { document.removeConnections(ids) }
        switch selection {
        case .noodle(let id), .knot(let id): if ids.contains(id) { selection = nil }
        default: break
        }
    }
    func setKnot(_ point: CGPoint?, connectionID: String) {
        cancel()
        guard document.connections.contains(where: { $0.id == connectionID }) else { return }
        let graphPoint = point.map(PhotaraGraphPoint.init)
        if let committed = commitMutation?(.setRouting(connectionID: connectionID, point: graphPoint)) { document = committed }
        else if commitMutation == nil { document.setKnot(graphPoint, connectionID: connectionID) }
        selection = point == nil ? .noodle(connectionID) : .knot(connectionID)
    }
    @discardableResult func deleteSelection() -> Bool {
        switch selection {
        case .noodle(let id): removeConnections([id]); return true
        case .knot(let id): setKnot(nil, connectionID: id); return true
        default: return false
        }
    }
    func defaultKnot(connectionID: String) -> CGPoint {
        guard let connection = document.connections.first(where: { $0.id == connectionID }),
              let a = portPoint(connection.source), let b = portPoint(connection.destination) else { return .zero }
        return CGPoint(x: (a.x + b.x) / 2, y: (a.y + b.y) / 2)
    }
    func insertNode(_ template: PhotaraGraphNode, near requestedPosition: CGPoint) {
        cancel(resetTool: true)
        var position = requestedPosition
        let size = PhotaraGraphGeometry.size(of: template)
        func overlaps(_ point: CGPoint) -> Bool {
            let candidate = CGRect(x: point.x - size.width / 2, y: point.y - size.height / 2,
                                   width: size.width, height: size.height).insetBy(dx: -10, dy: -10)
            return document.nodes.contains {
                candidate.intersects(PhotaraGraphGeometry.rect(of: $0, at: $0.position.cgPoint))
            }
        }
        if overlaps(position) {
            let step = max(size.width, size.height) + 24
            for ring in 1...12 {
                let candidates = [CGPoint(x: requestedPosition.x + Double(ring) * step, y: requestedPosition.y),
                                  CGPoint(x: requestedPosition.x, y: requestedPosition.y + Double(ring) * step),
                                  CGPoint(x: requestedPosition.x - Double(ring) * step, y: requestedPosition.y),
                                  CGPoint(x: requestedPosition.x, y: requestedPosition.y - Double(ring) * step)]
                if let available = candidates.first(where: { !overlaps($0) }) { position = available; break }
            }
        }
        let id = "\(template.kind)-\(UUID().uuidString)"
        document.nodes.append(.init(id: id, kind: template.kind, title: template.title, subtitle: template.subtitle,
                                    ports: template.ports, position: PhotaraGraphPoint(position)))
        selection = .node(id)
    }
    func replaceDocument(_ document: PhotaraGraphDocument) throws {
        try document.validate()
        cancel(resetTool: true)
        self.document = document
        selection = nil
    }
    func synchronizeDocument(_ document: PhotaraGraphDocument) throws {
        try document.validate()
        guard interaction == .idle else { return }
        self.document = document
        if case .node(let id) = selection, !document.nodes.contains(where: { $0.id == id }) { selection = nil }
    }
    func center(positions: [String: PhotaraGraphPoint], selectedNode: String?) {
        cancel(resetTool: true)
        camera = .init()
        for index in document.nodes.indices {
            if let position = positions[document.nodes[index].id] { document.nodes[index].position = position }
        }
        selection = selectedNode.map(PhotaraGraphSelection.node)
    }
}
