import Foundation

/// Document values only. IDs are opaque strings; row order is presentation data,
/// never port identity. This is the boundary for a future Rust graph DTO.
struct PhotaraGraphPoint: Codable, Equatable {
    var x: Double
    var y: Double
    static let zero = Self(x: 0, y: 0)
}

enum PhotaraGraphPortDirection: String, Codable {
    case input, output
}

struct PhotaraGraphPortID: Codable, Hashable {
    let node: String
    let key: String
}

struct PhotaraGraphPortDefinition: Codable, Equatable, Identifiable {
    let id: String
    let direction: PhotaraGraphPortDirection
    let dataType: String
    let label: String
}

struct PhotaraGraphNode: Codable, Equatable, Identifiable {
    let id: String
    let kind: String
    let title: String
    let subtitle: String
    let ports: [PhotaraGraphPortDefinition]
    var position: PhotaraGraphPoint

    func ports(_ direction: PhotaraGraphPortDirection) -> [PhotaraGraphPortDefinition] {
        ports.filter { $0.direction == direction }
    }
}

struct PhotaraGraphConnection: Codable, Equatable, Identifiable {
    let id: String
    let source: PhotaraGraphPortID
    var destination: PhotaraGraphPortID
    var routingPointID: String?
    fileprivate var legacyKnot: PhotaraGraphPoint?

    init(id: String, source: PhotaraGraphPortID, destination: PhotaraGraphPortID, routingPointID: String? = nil) {
        self.id = id; self.source = source; self.destination = destination
        self.routingPointID = routingPointID
    }

    enum CodingKeys: String, CodingKey {
        case id, source, destination, routingPointID
        case legacyKnot = "knot"
    }
}

/// A shared geometric junction, with its upstream semantic output explicit.
/// Edges retain their own identity and output/input endpoints across fan-out.
struct PhotaraGraphRoutingPoint: Codable, Equatable, Identifiable {
    let id: String
    let source: PhotaraGraphPortID
    var position: PhotaraGraphPoint
    var isJunction: Bool? = nil
}

struct PhotaraGraphDocument: Codable, Equatable {
    var nodes: [PhotaraGraphNode]
    var connections: [PhotaraGraphConnection]
    var routingPoints: [PhotaraGraphRoutingPoint]

    init(nodes: [PhotaraGraphNode], connections: [PhotaraGraphConnection],
         routingPoints: [PhotaraGraphRoutingPoint] = []) {
        self.nodes = nodes
        self.connections = connections
        self.routingPoints = routingPoints
    }

    enum CodingKeys: String, CodingKey { case nodes, connections, routingPoints }
    init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        nodes = try values.decode([PhotaraGraphNode].self, forKey: .nodes)
        connections = try values.decode([PhotaraGraphConnection].self, forKey: .connections)
        routingPoints = try values.decodeIfPresent([PhotaraGraphRoutingPoint].self, forKey: .routingPoints) ?? []
        // Migrate the original connection-owned knot DTO without losing routing.
        for index in connections.indices {
            if let position = connections[index].legacyKnot {
                guard connections[index].routingPointID == nil else { throw ValidationError.invalidDocument }
                let id = UUID().uuidString
                routingPoints.append(.init(id: id, source: connections[index].source, position: position))
                connections[index].routingPointID = id
                connections[index].legacyKnot = nil
            }
        }
        try validate()
    }

    func routingPoint(for connection: PhotaraGraphConnection) -> PhotaraGraphRoutingPoint? {
        routingPoints.first { $0.id == connection.routingPointID }
    }
    func knot(of connection: PhotaraGraphConnection) -> PhotaraGraphPoint? {
        routingPoint(for: connection)?.position
    }
    mutating func pruneRoutingPoints() {
        let used = Set(connections.compactMap(\.routingPointID))
        routingPoints.removeAll { !used.contains($0.id) }
    }
    mutating func removeConnections(_ ids: Set<String>) {
        connections.removeAll { ids.contains($0.id) }
        pruneRoutingPoints()
    }
    mutating func setKnot(_ point: PhotaraGraphPoint?, connectionID: String) {
        guard let index = connections.firstIndex(where: { $0.id == connectionID }) else { return }
        if let routeID = connections[index].routingPointID {
            if let point, let route = routingPoints.firstIndex(where: { $0.id == routeID }) {
                routingPoints[route].position = point
            } else if point == nil {
                for i in connections.indices where connections[i].routingPointID == routeID {
                    connections[i].routingPointID = nil
                }
                pruneRoutingPoints()
            }
        } else if let point {
            let id = UUID().uuidString
            routingPoints.append(.init(id: id, source: connections[index].source, position: point))
            connections[index].routingPointID = id
        }
    }

    func port(_ id: PhotaraGraphPortID) -> PhotaraGraphPortDefinition? {
        nodes.first { $0.id == id.node }?.ports.first { $0.id == id.key }
    }

    func canConnect(_ source: PhotaraGraphPortID, _ destination: PhotaraGraphPortID) -> Bool {
        guard source.node != destination.node,
              let output = port(source), output.direction == .output,
              let input = port(destination), input.direction == .input else { return false }
        return output.dataType == input.dataType || input.dataType == "any"
    }

    /// Validate imported DTOs before displaying them. No UI objects or caches
    /// participate in decoding, identity, or graph integrity.
    func validate() throws {
        guard Set(nodes.map(\.id)).count == nodes.count,
              nodes.allSatisfy({ Set($0.ports.map(\.id)).count == $0.ports.count
                  && $0.position.x.isFinite && $0.position.y.isFinite }),
              Set(connections.map(\.id)).count == connections.count,
              Set(connections.map(\.destination)).count == connections.count,
              Set(routingPoints.map(\.id)).count == routingPoints.count,
              routingPoints.allSatisfy({ route in
                  port(route.source)?.direction == .output && route.position.x.isFinite && route.position.y.isFinite
                      && connections.contains { $0.routingPointID == route.id }
              }),
              connections.allSatisfy({ edge in
                  canConnect(edge.source, edge.destination) && edge.legacyKnot == nil
                      && (edge.routingPointID == nil || routingPoint(for: edge)?.source == edge.source)
              })
        else { throw ValidationError.invalidDocument }
    }

    enum ValidationError: Error { case invalidDocument }

    /// One transaction handles duplicate creation, occupied inputs and rewires.
    /// A failed rewire leaves the original connection and its routing intact.
    @discardableResult
    mutating func connect(from source: PhotaraGraphPortID, to destination: PhotaraGraphPortID,
                          replacing id: String? = nil, via routingPointID: String? = nil) -> String? {
        guard canConnect(source, destination),
              routingPointID == nil || routingPoints.contains(where: { $0.id == routingPointID && $0.source == source })
        else { return nil }
        defer { pruneRoutingPoints() }
        if let id {
            guard let original = connections.first(where: { $0.id == id }), original.source == source
            else { return nil }
            connections.removeAll { $0.destination == destination && $0.id != id }
            let index = connections.firstIndex { $0.id == id }!
            connections[index].destination = destination
            return id
        }
        if let existing = connections.first(where: { $0.source == source && $0.destination == destination }) {
            return existing.id
        }
        connections.removeAll { $0.destination == destination }
        let id = UUID().uuidString
        connections.append(.init(id: id, source: source, destination: destination, routingPointID: routingPointID))
        if let route = routingPoints.firstIndex(where: { $0.id == routingPointID }) {
            routingPoints[route].isJunction = true
        }
        return id
    }
}
