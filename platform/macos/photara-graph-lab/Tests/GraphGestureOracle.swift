import Foundation

/// A deliberately separate specification model. It never calls the production
/// controller, collision resolver, camera transform, hit test or connect method.
/// The harness applies user intentions here, sends NSEvents to the real canvas,
/// then compares the two states. Fresh UUIDs are learned only after topology
/// matches, and must remain stable through every subsequent action.
struct GraphGestureOracle {
    struct Edge {
        var id: String?
        let source: PhotaraGraphPortID
        var destination: PhotaraGraphPortID
        var knot: CGPoint?
        var group: String?
    }
    var nodes = GraphLabFixtures.document.nodes
    var edges: [Edge] = GraphLabFixtures.document.connections.map {
        Edge(id: $0.id, source: $0.source, destination: $0.destination, knot: GraphLabFixtures.document.knot(of: $0)?.cgPoint)
    }
    var knownIDs = Set(GraphLabFixtures.document.connections.map(\.id))
    var junctionGroups = Set<String>()
    var routeIDs: [String: String] = [:]
    var pan = CGPoint.zero
    var zoom = 1.0
    var curved = true
    var viewport: CGSize

    func screen(_ p: CGPoint) -> CGPoint {
        CGPoint(x: viewport.width / 2 + pan.x + p.x * zoom, y: viewport.height / 2 + pan.y + p.y * zoom)
    }
    func world(_ p: CGPoint) -> CGPoint {
        CGPoint(x: (p.x - viewport.width / 2 - pan.x) / zoom, y: (p.y - viewport.height / 2 - pan.y) / zoom)
    }
    func point(_ port: PhotaraGraphPortID) -> CGPoint {
        let node = nodes.first { $0.id == port.node }!
        let definition = node.ports.first { $0.id == port.key }!
        let rows = node.ports.filter { $0.direction == definition.direction }
        let index = rows.firstIndex { $0.id == port.key }!
        let count = max(1, max(node.ports.filter { $0.direction == .input }.count, node.ports.filter { $0.direction == .output }.count))
        return CGPoint(x: node.position.x + (definition.direction == .input ? -112 : 112),
                       y: node.position.y - (74 + Double(count) * 25) / 2 + 74.5 + Double(index) * 25)
    }
    var outputs: [PhotaraGraphPortID] {
        nodes.flatMap { node in node.ports.filter { $0.direction == .output }.map { .init(node: node.id, key: $0.id) } }
    }
    var inputs: [PhotaraGraphPortID] {
        nodes.flatMap { node in node.ports.filter { $0.direction == .input }.map { .init(node: node.id, key: $0.id) } }
    }
    mutating func connect(_ source: PhotaraGraphPortID, _ destination: PhotaraGraphPortID) {
        if edges.contains(where: { $0.source == source && $0.destination == destination }) { return }
        edges.removeAll { $0.destination == destination }
        edges.append(.init(source: source, destination: destination))
    }
    mutating func setKnot(_ point: CGPoint?, edgeID: String) {
        guard let index = edges.firstIndex(where: { $0.id == edgeID }) else { return }
        if let group = edges[index].group {
            for i in edges.indices where edges[i].group == group {
                edges[i].knot = point
                if point == nil { edges[i].group = nil }
            }
        } else if let point {
            edges[index].knot = point
            edges[index].group = "route-\(edgeID)-\(routeIDs.count)"
        }
    }
    mutating func branch(_ edge: Edge, to destination: PhotaraGraphPortID) {
        if edges.contains(where: { $0.source == edge.source && $0.destination == destination }) { return }
        edges.removeAll { $0.destination == destination }
        edges.append(.init(source: edge.source, destination: destination, knot: edge.knot, group: edge.group))
        if let group = edge.group { junctionGroups.insert(group) }
    }
    mutating func rewire(_ edge: Edge, to destination: PhotaraGraphPortID) {
        edges.removeAll { $0.destination == destination && $0.id != edge.id }
        if let index = edges.firstIndex(where: { $0.id == edge.id }) { edges[index].destination = destination }
    }
    mutating func setZoom(_ target: Double, at anchor: CGPoint) {
        let fixed = world(anchor)
        zoom = min(1.8, max(0.55, target))
        pan = CGPoint(x: anchor.x - viewport.width / 2 - fixed.x * zoom, y: anchor.y - viewport.height / 2 - fixed.y * zoom)
    }

    // Independent Bezier evaluation for selecting visible wire spans and
    // predicting knife intersections; production uses SwiftUI Path hit areas.
    func polyline(_ edge: Edge) -> [CGPoint] {
        let a = point(edge.source), b = point(edge.destination)
        if !curved {
            let vertices = [a] + (edge.knot.map { [$0] } ?? []) + [b]
            return zip(vertices, vertices.dropFirst()).flatMap { start, end in
                (0...80).map { step in
                    let t = Double(step) / 80
                    return CGPoint(x: start.x + (end.x - start.x) * t, y: start.y + (end.y - start.y) * t)
                }
            }
        }
        let lead = min(90.0, max(28.0, abs(b.x - a.x) * 0.36))
        func cubic(_ a: CGPoint, _ b: CGPoint, _ c: CGPoint, _ d: CGPoint) -> [CGPoint] {
            (0...160).map { step in
                let t = Double(step) / 160, u = 1 - t
                return CGPoint(x: u*u*u*a.x + 3*u*u*t*b.x + 3*u*t*t*c.x + t*t*t*d.x,
                               y: u*u*u*a.y + 3*u*u*t*b.y + 3*u*t*t*c.y + t*t*t*d.y)
            }
        }
        if let k = edge.knot {
            if let group = edge.group, junctionGroups.contains(group) {
                func segment(_ start: CGPoint, _ end: CGPoint) -> [CGPoint] {
                    let reach = min(90.0, max(28.0, abs(end.x - start.x) * 0.36))
                    return cubic(start, CGPoint(x: start.x + reach, y: start.y), CGPoint(x: end.x - reach, y: end.y), end)
                }
                return segment(a, k) + segment(k, b).dropFirst()
            }
            let tangent = CGPoint(x: (b.x - a.x) * 0.14, y: (b.y - a.y) * 0.14)
            return cubic(a, CGPoint(x: a.x + lead, y: a.y), CGPoint(x: k.x - tangent.x, y: k.y - tangent.y), k)
                + cubic(k, CGPoint(x: k.x + tangent.x, y: k.y + tangent.y), CGPoint(x: b.x - lead, y: b.y), b).dropFirst()
        }
        return cubic(a, CGPoint(x: a.x + lead, y: a.y), CGPoint(x: b.x - lead, y: b.y), b)
    }
    func nodeContains(_ p: CGPoint) -> Bool {
        nodes.contains { node in
            let rows = max(1, max(node.ports.filter { $0.direction == .input }.count, node.ports.filter { $0.direction == .output }.count))
            return abs(p.x - node.position.x) <= 114 && abs(p.y - node.position.y) <= (74 + Double(rows) * 25) / 2 + 2
        }
    }
    func portContains(_ p: CGPoint) -> Bool {
        (outputs + inputs).contains { id in
            let center = point(id)
            return abs(p.x - center.x) <= max(18, 12 / zoom) && abs(p.y - center.y) <= 11
        }
    }
    func selectablePoint(_ edge: Edge) -> CGPoint? {
        let samples = polyline(edge)
        // Test from the middle out to avoid picking endpoint beads.
        let candidates = [samples[samples.count / 2]] + samples
        return candidates.first { p in
            guard !nodeContains(p), !portContains(p),
                  screen(p).x > 5, screen(p).x < viewport.width - 5,
                  screen(p).y > 5, screen(p).y < viewport.height - 60,
                  !edges.contains(where: { $0.knot.map { hypot($0.x - p.x, $0.y - p.y) <= 13 } ?? false }) else { return false }
            // Select a span clearly outside every higher edge's hit stroke.
            // Bezier sampling and native stroked paths differ slightly right
            // at the stroke boundary; those ambiguous pixels are not reliable
            // user targets for asserting which overlapping edge was intended.
            guard let index = edges.firstIndex(where: { $0.id == edge.id }) else { return false }
            return edges.dropFirst(index + 1).allSatisfy { other in
                let points = polyline(other)
                return zip(points, points.dropFirst()).allSatisfy { Self.distance(p, $0.0, $0.1) > 10 / zoom }
            }
        }
    }
    static func distance(_ p: CGPoint, _ a: CGPoint, _ b: CGPoint) -> Double {
        let dx = b.x - a.x, dy = b.y - a.y
        let length = dx*dx + dy*dy
        let t = length == 0 ? 0 : min(1, max(0, ((p.x - a.x)*dx + (p.y - a.y)*dy) / length))
        return hypot(p.x - a.x - t*dx, p.y - a.y - t*dy)
    }
    /// A full-height vertical knife sweep. Reject ambiguous tangent cases so
    /// raster hit tolerance cannot mask an ownership/topology failure.
    func cutIDs(atX x: Double) -> Set<String>? {
        var cut = Set<String>()
        for edge in edges {
            let points = polyline(edge)
            let start = world(CGPoint(x: screen(CGPoint(x: x, y: 0)).x, y: 5))
            let end = world(CGPoint(x: screen(CGPoint(x: x, y: 0)).x, y: viewport.height - 3))
            let distance = zip(points, points.dropFirst()).map { a, b -> Double in
                if a.x != b.x {
                    let t = (x - a.x) / (b.x - a.x)
                    let y = a.y + t * (b.y - a.y)
                    if t >= 0 && t <= 1 && y >= start.y && y <= end.y { return 0 }
                }
                return min(Self.distance(a, start, end), Self.distance(b, start, end),
                           Self.distance(start, a, b), Self.distance(end, a, b))
            }.min()!
            // Only demand a cut for a centerline crossing. A near miss at a
            // butt-capped endpoint is not a crossing (a capsule-distance test
            // incorrectly treats that as a round cap). Avoid raster-only cases.
            if distance > 0.000001 && distance < max(1.2, 1 / zoom) + 1 / zoom { return nil }
            if distance <= 0.000001, let id = edge.id { cut.insert(id) }
        }
        return cut
    }

    mutating func mismatches(document: PhotaraGraphDocument, actualZoom: Double, actualPan: CGSize) -> [String] {
        var errors: [String] = []
        func near(_ a: CGPoint, _ b: CGPoint) -> Bool { hypot(a.x - b.x, a.y - b.y) < 0.00001 }
        if abs(actualZoom - zoom) > 0.00001 || !near(pan, CGPoint(x: actualPan.width, y: actualPan.height)) {
            errors.append("camera expected \(zoom), \(pan); actual \(actualZoom), \(actualPan)")
        }
        if document.nodes.count != nodes.count { errors.append("node count") }
        for node in nodes {
            guard let actual = document.nodes.first(where: { $0.id == node.id }) else { errors.append("missing node \(node.id)"); continue }
            if !near(actual.position.cgPoint, node.position.cgPoint) { errors.append("node \(node.id) position") }
            if actual.ports != node.ports || actual.kind != node.kind { errors.append("node \(node.id) definition changed") }
        }
        if document.connections.count != edges.count { errors.append("connection count expected \(edges.count), actual \(document.connections.count)") }
        if Set(document.connections.map(\.destination)).count != document.connections.count { errors.append("duplicate incoming edges") }
        if Set(document.connections.map(\.id)).count != document.connections.count { errors.append("duplicate connection IDs") }
        for index in edges.indices {
            let expected = edges[index]
            guard let actual = document.connections.first(where: { $0.source == expected.source && $0.destination == expected.destination }) else {
                errors.append("missing \(expected.source) -> \(expected.destination)"); continue
            }
            if let id = expected.id {
                if actual.id != id { errors.append("connection identity changed") }
            } else {
                if knownIDs.contains(actual.id) || UUID(uuidString: actual.id) == nil { errors.append("new connection ID was reused/invalid") }
                edges[index].id = actual.id
                knownIDs.insert(actual.id)
            }
            if let knot = expected.knot {
                if !near(knot, document.knot(of: actual)?.cgPoint ?? CGPoint(x: Double.infinity, y: Double.infinity)) { errors.append("routing knot changed") }
            } else if document.knot(of: actual) != nil { errors.append("unexpected routing knot") }
            if let group = expected.group {
                guard let route = document.routingPoint(for: actual) else { errors.append("missing shared routing point"); continue }
                if let known = routeIDs[group], known != route.id { errors.append("shared routing identity changed") }
                if routeIDs[group] == nil {
                    if routeIDs.values.contains(route.id) { errors.append("routing identity reused") }
                    routeIDs[group] = route.id
                }
                if route.source != expected.source || (route.isJunction == true) != junctionGroups.contains(group) {
                    errors.append("routing source/junction semantics changed")
                }
            } else if actual.routingPointID != nil { errors.append("unexpected routing group") }
        }
        if document.routingPoints.count != Set(edges.compactMap(\.group)).count { errors.append("orphan/duplicate routing point") }
        return errors
    }
}

struct GraphSeededRandom {
    var state: UInt64
    mutating func next() -> UInt64 {
        state &+= 0x9e3779b97f4a7c15
        var z = state
        z = (z ^ (z >> 30)) &* 0xbf58476d1ce4e5b9
        z = (z ^ (z >> 27)) &* 0x94d049bb133111eb
        return z ^ (z >> 31)
    }
    mutating func index(_ count: Int) -> Int { Int(next() % UInt64(count)) }
    mutating func value(_ lower: Double, _ upper: Double) -> Double { lower + Double(next() >> 11) / Double(1 << 53) * (upper - lower) }
}

struct GraphVerificationOptions {
    let randomOnly: Bool
    let seed: UInt64?
    let steps: Int
    let style: String?
    let appearance: String?
    init() {
        let args = Array(CommandLine.arguments.dropFirst())
        var values: [String: String] = [:]
        var only = false
        var index = 0
        func invalid(_ message: String) -> Never {
            GraphTestLog.write("Invalid verification arguments: \(message)")
            exit(2)
        }
        while index < args.count {
            let key = args[index]
            if key == "--random-only" { only = true; index += 1; continue }
            guard ["--seed", "--steps", "--style", "--appearance"].contains(key), index + 1 < args.count else { invalid(key) }
            values[key] = args[index + 1]
            index += 2
        }
        randomOnly = only
        if let raw = values["--seed"] {
            guard let value = UInt64(raw) else { invalid("--seed requires an unsigned decimal integer") }
            seed = value
        } else { seed = nil }
        if let raw = values["--steps"] {
            guard let value = Int(raw), value > 0 else { invalid("--steps requires a positive integer") }
            steps = value
        } else { steps = 180 }
        style = values["--style"]
        appearance = values["--appearance"]
        if let style, !["straight", "curved"].contains(style) { invalid("--style must be straight or curved") }
        if let appearance, !["light", "dark"].contains(appearance) { invalid("--appearance must be light or dark") }
    }
}
