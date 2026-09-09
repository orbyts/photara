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
    var knot: PhotaraGraphPoint?
}

struct PhotaraGraphDocument: Codable, Equatable {
    var nodes: [PhotaraGraphNode]
    var connections: [PhotaraGraphConnection]

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
              connections.allSatisfy({ canConnect($0.source, $0.destination)
                  && ($0.knot.map { $0.x.isFinite && $0.y.isFinite } ?? true) })
        else { throw ValidationError.invalidDocument }
    }

    enum ValidationError: Error { case invalidDocument }

    /// One transaction handles duplicate creation, occupied inputs and rewires.
    /// A failed rewire leaves the original connection and its routing intact.
    @discardableResult
    mutating func connect(from source: PhotaraGraphPortID, to destination: PhotaraGraphPortID,
                          replacing id: String? = nil) -> String? {
        guard canConnect(source, destination) else { return nil }
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
        connections.append(.init(id: id, source: source, destination: destination))
        return id
    }
}
