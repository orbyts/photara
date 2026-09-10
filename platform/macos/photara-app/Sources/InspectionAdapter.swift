import AppKit
import SwiftUI

extension NodeInspection {
    init(_ node: BridgeNodeDto) {
        self.init(nodeId: node.nodeId, displayName: node.displayName, packageId: node.packageId,
            packageVersion: node.packageVersion, definitionId: node.definitionId,
            iconResourceId: node.iconResourceId, themeColorRole: node.themeColorRole,
            accentSrgbHex: node.accentSrgbHex, status: node.status,
            ports: node.ports.map { .init(portId: $0.portId, direction: $0.direction == .input ? .input : .output,
                valueTypeId: $0.valueTypeId, connectedNodeName: $0.connectedNodeName,
                summary: $0.summary.map { .init(label: $0.label, value: $0.value) }) },
            disk: node.disk.map { .init(folderBindingId: $0.folderBindingId, recursive: $0.recursive,
                                       acceptedAssetCount: $0.acceptedAssetCount) },
            layout: node.layout.map { layout in
                .init(authoredStateDigest: layout.authoredStateDigest,
                    canvas: .init(widthPixels: layout.canvas.widthPixels, heightPixels: layout.canvas.heightPixels),
                    frames: layout.frames.map { frame in
                        .init(frameId: frame.frameId, index: frame.index, arrangement: .init(frame.arrangement),
                            cells: frame.cells.map { cell in
                                .init(cellId: cell.cellId, assetId: cell.assetId, contentMode: .init(cell.contentMode),
                                    cropRect: cell.cropRect.map(LayoutNormalizedRect.init),
                                    resolvedRect: .init(cell.resolvedRect), quarterTurn: .init(cell.quarterTurn))
                            })
                    })
            }, diagnostics: node.diagnostics.map { .init(code: $0.code, message: $0.message) })
    }
}
private extension LayoutArrangement {
    init(_ value: BridgeLayoutArrangement) {
        self = switch value { case .one: .one; case .horizontalStack: .horizontalStack
        case .verticalStack: .verticalStack; case .uniformGrid: .uniformGrid; case .custom: .custom }
    }
}
private extension LayoutContentMode {
    init(_ value: BridgeLayoutContentMode) {
        self = switch value { case .fit: .fit; case .fill: .fill; case .crop: .crop }
    }
}
private extension LayoutQuarterTurn {
    init(_ value: BridgeQuarterTurn) {
        self = switch value { case .zero: .zero; case .clockwise90: .clockwise90
        case .clockwise180: .clockwise180; case .clockwise270: .clockwise270 }
    }
    var bridge: BridgeQuarterTurn {
        switch self { case .zero: .zero; case .clockwise90: .clockwise90
        case .clockwise180: .clockwise180; case .clockwise270: .clockwise270 }
    }
}
private extension LayoutNormalizedRect {
    init(_ value: BridgeNormalizedRectDto) {
        self.init(x: value.x, y: value.y, width: value.width, height: value.height)
    }
}
private extension LayoutArrangementAction {
    var bridge: BridgeLayoutArrangementEdit {
        switch self { case .one: .one; case .horizontalStack: .horizontalStack
        case .verticalStack: .verticalStack; case .uniformGrid(let columns): .uniformGrid(columns: columns)
        case .custom: .custom }
    }
}
private extension LayoutCellAction {
    var bridge: BridgeLayoutCellEdit {
        switch self {
        case .fit(let x, let y): .fit(alignmentX: x, alignmentY: y)
        case .fill(let x, let y): .fill(focalX: x, focalY: y)
        case .crop(let x, let y, let w, let h): .crop(x: x, y: y, width: w, height: h)
        case .setQuarterTurn(let turn): .setQuarterTurn(quarterTurn: turn.bridge)
        }
    }
}
private extension LayoutStructureAction {
    var bridge: BridgeLayoutStructureEdit {
        switch self {
        case .insertFrame(let i): .insertFrame(index: i)
        case .removeFrame(let f): .removeFrame(frameId: f)
        case .moveFrame(let f, let i): .moveFrame(frameId: f, toIndex: i)
        case .setFrameArrangement(let f, let a): .setFrameArrangement(frameId: f, arrangement: a.bridge)
        case .insertCell(let f, let i): .insertCell(frameId: f, index: i)
        case .removeCell(let f, let c): .removeCell(frameId: f, cellId: c)
        }
    }
}

extension AppModel {
    var inspectorActions: InspectorActions {
        .init(chooseFolder: { id in if let node = self.inspectedNode(id) { self.chooseFolder(for: node) } },
            scanDisk: { id in if let node = self.inspectedNode(id) { self.scanDisk(node) } },
            connectDisk: { id in if let node = self.inspectedNode(id) { self.connectDiskToAvailableLayout(node) } },
            structure: { id, edit in
                if let node = self.inspectedNode(id) { self.editStructure(node: node, edit: edit.bridge) }
            }, cell: performCellAction)
    }
    func performCellAction(_ id: String, _ frame: String, _ cell: String, _ edit: LayoutCellAction) {
        if let node = inspectedNode(id) { editCell(node: node, frameID: frame, cellID: cell, edit: edit.bridge) }
    }
    private func inspectedNode(_ id: String) -> BridgeNodeDto? { snapshot?.nodes.first { $0.nodeId == id } }
}

struct ProductionInspectorView: View {
    @EnvironmentObject private var app: AppModel
    @EnvironmentObject private var workspace: WorkspaceModel
    var body: some View {
        let node = app.snapshot?.nodes.first { $0.nodeId == workspace.selectedNodeID }
        InspectorView(presentation: .init(node: node.map(NodeInspection.init),
            selectedFrameID: workspace.selectedFrameID, selectedCellID: workspace.selectedCellID,
            previews: app.layoutCellProxies.mapValues { .init($0.descriptor()) },
            isScanning: node.map { app.scanningDiskNodeIDs.contains($0.nodeId) } ?? false,
            graphRevision: app.snapshot?.graph.revision ?? 0, progressLabel: app.progressLabel),
            actions: app.inspectorActions)
    }
}

struct ProductionLayoutView: View {
    var nodeID: String? = nil
    @EnvironmentObject private var app: AppModel
    @EnvironmentObject private var workspace: WorkspaceModel
    private var node: BridgeNodeDto? {
        let layouts = app.snapshot?.nodes.filter { $0.hasLayoutWorkSurface } ?? []
        if let nodeID { return layouts.first { $0.nodeId == nodeID } }
        if let active = layouts.first(where: { $0.nodeId == workspace.activeWorkspaceNodeID }) { return active }
        return layouts.first { $0.nodeId == workspace.selectedNodeID } ?? layouts.first
    }
    var body: some View {
        LayoutAuthoringSurfaceView(presentation: .init(node: node.map(NodeInspection.init),
            previews: app.layoutCellProxies.compactMapValues { reference in
                let descriptor = reference.descriptor()
                guard let image = NSImage(contentsOfFile: descriptor.localPath) else { return nil }
                return .init(image: image, metadata: .init(descriptor))
            }, nativeThumbnails: app.layoutNativeThumbnails),
            actions: .init(requestPreviews: { app.requestLayoutProxies(for: $0) }, cell: app.performCellAction))
    }
}
