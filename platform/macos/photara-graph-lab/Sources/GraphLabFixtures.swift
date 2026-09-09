import Foundation

/// The one-, three-, and six-row specimens are data for the same renderer and
/// controller. No fixture IDs occur in reusable interaction code.
enum GraphLabFixtures {
    static let document: PhotaraGraphDocument = {
        func node(_ id: String, _ title: String, _ subtitle: String, _ inputs: [String], _ outputs: [String], _ x: Double, _ y: Double) -> PhotaraGraphNode {
            let ports = inputs.map { PhotaraGraphPortDefinition(id: "in.\($0)", direction: .input, dataType: "graph-value", label: $0) }
                + outputs.map { PhotaraGraphPortDefinition(id: "out.\($0)", direction: .output, dataType: "graph-value", label: $0) }
            return .init(id: id, kind: id, title: title, subtitle: subtitle, ports: ports, position: .init(x: x, y: y))
        }
        return PhotaraGraphDocument(nodes: [
            node("source", "Source", "One port", [], ["Assets"], -190, -120),
            node("transform", "Transform", "Three port rows", ["Input", "Mask", "Control"], ["Result", "Preview", "Metadata"], 150, -90),
            node("composite", "Composite", "Six port rows", ["Layer 1", "Layer 2", "Mask", "Depth", "Color", "Control"], ["Image", "Preview"], 0, 150)
        ], connections: [.init(id: "fixture-assets-input", source: .init(node: "source", key: "out.Assets"), destination: .init(node: "transform", key: "in.Input"))])
    }()
}
