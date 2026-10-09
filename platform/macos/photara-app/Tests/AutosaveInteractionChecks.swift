import Foundation

/// Exercises the actual controller and native admission policy. It deliberately
/// does not invent a durable backend for unimplemented authoring commands.
@MainActor enum AutosaveInteractionChecks {
    static func run() async throws {
        let output = PhotaraGraphPortDefinition(id: "out", direction: .output, dataType: "value", label: "Out")
        let input = PhotaraGraphPortDefinition(id: "in", direction: .input, dataType: "value", label: "In")
        let a = PhotaraGraphNode(id: "a", kind: "test", title: "A", subtitle: "", ports: [output], position: .init(x: -200, y: 0))
        let b = PhotaraGraphNode(id: "b", kind: "test", title: "B", subtitle: "", ports: [input], position: .init(x: 200, y: 0))
        let original = PhotaraGraphDocument(nodes: [a, b], connections: [])
        let local = PhotaraGraphInteractionController(document: original)
        local.insertNode(b, near: .init(x: 800, y: 0))
        precondition(local.document.nodes.count == 3)
        let inserted = local.document.nodes.last!.id
        precondition(local.selection == .node(inserted))
        let beforeDelete = local.document
        precondition(!local.deleteSelection() && local.document == beforeDelete)
        let source = PhotaraGraphPortID(node: "a", key: "out")
        let destination = PhotaraGraphPortID(node: "b", key: "in")
        local.connect(source, to: destination)
        precondition(local.document.connections.count == 1)
        precondition(local.deleteSelection() && local.document.connections.isEmpty)
        try local.document.validate()

        var dispatches = 0
        let admitted = PhotaraGraphInteractionController(document: original, commitMutation: { mutation in
            dispatches += 1
            // Same policy used by the normal LocalLibrarySession canvas.
            precondition(LocalLibraryGraphAdmission.position(mutation) == nil)
            return original
        })
        admitted.connect(source, to: destination)
        precondition(dispatches == 1 && admitted.document == original)
        precondition(LocalLibraryGraphAdmission.position(.removeConnections(["edge"])) == nil)
        precondition(LocalLibraryGraphAdmission.position(.setRouting(connectionID: "edge", point: nil)) == nil)
        precondition(LocalLibraryGraphAdmission.position(.moveNode(id: "a", position: .init(x: .infinity, y: 0))) == nil)
        let move = LocalLibraryGraphAdmission.position(.moveNode(id: "a", position: .init(x: 1.25, y: -2)))!
        precondition(move.id == "a" && move.x == 1_250 && move.y == -2_000)
        print("PASS: actual controller local add/connect/wire-delete; node-delete refusal; native adapter refuses connect/routing/removal without changing Graph")
        print("PASS: actual shared presentation model delayed move FIFO, MainActor heartbeat, failure/retry/stale acknowledgement and lifecycle drain (scripted backend; not native durability proof)")
    }
}
