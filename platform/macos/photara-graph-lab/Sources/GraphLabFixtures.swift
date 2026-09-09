import Foundation

/// The one-, three-, and six-row specimens are data for the same renderer and
/// controller. No fixture IDs occur in reusable interaction code.
enum GraphLabFixtures {
    static let presentationByKind: [String: PhotaraGraphNodePresentationMetadata] = [
        "source": .init(category: .sourcesImport, iconResource: "node-disk-folder"),
        "transform": .init(category: .transform, iconResource: "node-rotate"),
        "composite": .init(category: .layoutComposition, iconResource: "node-layout")
    ]

    static func presentation(for kind: String) -> PhotaraGraphNodePresentationMetadata {
        presentationByKind[kind]
            ?? .init(category: .utilitiesControl, iconResource: "node-unknown")
    }

    static let document: PhotaraGraphDocument = {
        func node(_ id: String, _ title: String, _ subtitle: String, _ inputs: [String], _ outputs: [String], _ x: Double, _ y: Double) -> PhotaraGraphNode {
            let ports = inputs.map { PhotaraGraphPortDefinition(id: "in.\($0)", direction: .input, dataType: "graph-value", label: $0) }
                + outputs.map { PhotaraGraphPortDefinition(id: "out.\($0)", direction: .output, dataType: "graph-value", label: $0) }
            return .init(id: id, kind: id, title: title, subtitle: subtitle, ports: ports, position: .init(x: x, y: y))
        }
        return PhotaraGraphDocument(nodes: [
            node("source", "Disk Folder", "Sources & Import", [], ["Assets"], -190, -120),
            node("transform", "Rotate", "Transform", ["Input", "Mask", "Control"], ["Result", "Preview", "Metadata"], 150, -90),
            node("composite", "Layout", "Layout & Composition", ["Layer 1", "Layer 2", "Mask", "Depth", "Color", "Control"], ["Image", "Preview"], 0, 150)
        ], connections: [.init(id: "fixture-assets-input", source: .init(node: "source", key: "out.Assets"), destination: .init(node: "transform", key: "in.Input"))])
    }()
}
