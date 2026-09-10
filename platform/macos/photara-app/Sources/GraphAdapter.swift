import Foundation

enum PhotaraProductionGraphAdapter {
    static func document(_ snapshot: BridgeProjectSnapshotDto) -> PhotaraGraphDocument {
        let nodes = snapshot.nodes.map { node in
            PhotaraGraphNode(id: node.nodeId, kind: node.definitionId, title: node.brandName,
                subtitle: node.status, ports: node.ports.map { port in
                    PhotaraGraphPortDefinition(id: port.portId,
                        direction: port.direction == .input ? .input : .output,
                        dataType: "\(port.valueTypeId)@\(port.valueTypeVersion)", label: port.displayName)
                }, position: .init(x: Double(node.graphX) / 1_000, y: Double(node.graphY) / 1_000))
        }
        var routes: [String: PhotaraGraphRoutingPoint] = [:]
        let connections = snapshot.graph.connections.map { edge in
            if let id = edge.routingId, let x = edge.routingX, let y = edge.routingY {
                routes[id] = .init(id: id, source: .init(node: edge.outputNodeId, key: edge.outputPortId),
                    position: .init(x: Double(x) / 1_000, y: Double(y) / 1_000), isJunction: edge.routingIsJunction)
            }
            return PhotaraGraphConnection(id: edge.connectionId,
                source: .init(node: edge.outputNodeId, key: edge.outputPortId),
                destination: .init(node: edge.inputNodeId, key: edge.inputPortId), routingPointID: edge.routingId)
        }
        return PhotaraGraphDocument(nodes: nodes, connections: connections,
            routingPoints: Array(routes.values).sorted { $0.id < $1.id })
    }
}
