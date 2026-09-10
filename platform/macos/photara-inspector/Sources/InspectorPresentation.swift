import Foundation

enum InspectorSection: String, CaseIterable, Identifiable {
    case all, identity, disk, inputs, parameters, frameAndCell, outputs, evaluation, diagnostics
    var id: String { rawValue }
}
struct InspectorPresentation {
    var node: NodeInspection?
    var selectedFrameID: String?
    var selectedCellID: String?
    var previews: [String: ImagePreviewMetadata] = [:]
    var isScanning = false
    var graphRevision: UInt64 = 0
    var progressLabel = "Idle"
    var actionsEnabled = true
    var showsGraph = true
    var hasEditableSettings = true
    var selectedFrame: LayoutFrameInspection? {
        let frames = node?.layout?.frames ?? []
        return frames.first { $0.frameId == selectedFrameID } ?? frames.first
    }
    var selectedCell: LayoutCellInspection? {
        let cells = selectedFrame?.cells ?? []
        return cells.first { $0.cellId == selectedCellID } ?? cells.first
    }
}
struct InspectorActions {
    var chooseFolder: (String) -> Void
    var scanDisk: (String) -> Void
    var connectDisk: (String) -> Void
    var structure: (String, LayoutStructureAction) -> Void
    var cell: (String, String, String, LayoutCellAction) -> Void
    var showGraph: () -> Void = {}

    func editStructure(node: String, edit: LayoutStructureAction) { structure(node, edit) }
    func editCell(node: String, frameID: String, cellID: String, edit: LayoutCellAction) {
        cell(node, frameID, cellID, edit)
    }
}
