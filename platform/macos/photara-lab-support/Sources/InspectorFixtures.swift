import Foundation

/// Exact, portable presentation specimens. These are lab data, never Core node registrations.
enum InspectorFixture: String, CaseIterable, Identifiable {
    case disk, scanning, layout, singleCell, missingPackage, diagnostics, noSelection
    var id: String { rawValue }
    var presentation: InspectorPresentation {
        switch self {
        case .noSelection: return .init(node: nil)
        case .disk, .scanning:
            return .init(node: .init(nodeId: "disk", displayName: "Disk Folder",
                packageId: "photara.disk", packageVersion: "1.0.0", definitionId: "photara.disk.folder",
                iconResourceId: "photara.disk.folder", accentSrgbHex: "#66AD72",
                status: self == .scanning ? "Running" : "Ready",
                ports: [.init(portId: "assets", direction: .output, valueTypeId: "photara.asset-set",
                              connectedNodeName: "Layout", summary: [.init(label: "Assets", value: "42")])],
                disk: .init(folderBindingId: "fixture-folder-binding", recursive: true, acceptedAssetCount: 42)),
                isScanning: self == .scanning, graphRevision: 7,
                progressLabel: self == .scanning ? "Fingerprinting changed files…" : "Idle")
        case .layout, .singleCell:
            let single = self == .singleCell
            let count = single ? 1 : 2
            var frames: [LayoutFrameInspection] = []
            for frame in 0..<count {
                var cells: [LayoutCellInspection] = []
                for cell in 0..<count {
                    let crop: LayoutNormalizedRect? = cell == 0
                        ? .init(x: 100_000, y: 100_000, width: 800_000, height: 800_000) : nil
                    let rect = LayoutNormalizedRect(x: UInt32(cell * 500_000), y: 0,
                        width: single ? 1_000_000 : 500_000, height: 1_000_000)
                    cells.append(LayoutCellInspection(cellId: "cell-\(frame)-\(cell)", assetId: "asset-\(cell)",
                        contentMode: cell == 0 ? .crop : .fit, cropRect: crop, resolvedRect: rect, quarterTurn: .zero))
                }
                frames.append(LayoutFrameInspection(frameId: "frame-\(frame)", index: UInt64(frame),
                    arrangement: single ? .one : .horizontalStack, cells: cells))
            }
            return .init(node: .init(nodeId: "layout", displayName: "Layout", packageId: "photara.layout",
                packageVersion: "1.0.0", definitionId: "photara.layout", iconResourceId: "photara.layout.compose",
                accentSrgbHex: "#A682CF", status: "Ready",
                ports: [.init(portId: "assets", direction: .input, valueTypeId: "photara.asset-set",
                              connectedNodeName: "Disk Folder", summary: [.init(label: "Assets", value: "42")]),
                        .init(portId: "plan", direction: .output, valueTypeId: "photara.layout-plan")],
                layout: .init(authoredStateDigest: "fixture-authored-digest-v1",
                    canvas: .init(widthPixels: 3000, heightPixels: 4000), frames: frames)),
                selectedFrameID: "frame-0", selectedCellID: "cell-0-0",
                previews: ["cell-0-0": .init(pixelWidth: 240, pixelHeight: 360, dynamicRange: .hdr,
                                            colorSpaceId: "extended-linear-display-p3")], graphRevision: 7)
        case .missingPackage, .diagnostics:
            return .init(node: .init(nodeId: "unresolved", displayName: "Unavailable Extension",
                packageId: "example.missing", packageVersion: "2.0.0", definitionId: "example.missing.source",
                iconResourceId: "example.unknown", status: "Error",
                ports: [.init(portId: "assets", direction: .output, valueTypeId: "photara.asset-set")],
                diagnostics: [.init(code: "package.unavailable", message: "The exact node package is unavailable."),
                              .init(code: "evaluation.blocked", message: "Evaluation is waiting for the required package.")]),
                graphRevision: 7, progressLabel: "Blocked", actionsEnabled: self != .missingPackage)
        }
    }
}
