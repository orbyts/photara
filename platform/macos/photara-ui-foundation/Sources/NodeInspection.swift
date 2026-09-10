import Foundation

/// Native immutable inspection values, mapped by the production adapter. No bridge objects or JSON.
struct NodeInspection {
    var nodeId: String
    var displayName: String
    var packageId: String
    var packageVersion: String
    var definitionId: String
    var iconResourceId: String
    var themeColorRole: String?
    var accentSrgbHex: String?
    var status: String
    var ports: [PortInspection] = []
    var disk: DiskInspection?
    var layout: LayoutInspection?
    var diagnostics: [DiagnosticPresentation] = []
}
struct DiagnosticPresentation: Equatable {
    var code: String
    var message: String
}
struct PortInspection {
    enum Direction { case input, output }
    struct Field { var label: String; var value: String }
    var portId: String
    var direction: Direction
    var valueTypeId: String
    var connectedNodeName: String?
    var summary: [Field] = []
}
struct DiskInspection {
    var folderBindingId: String
    var recursive: Bool
    var acceptedAssetCount: UInt64
}
struct LayoutInspection {
    struct Canvas { var widthPixels: UInt32; var heightPixels: UInt32 }
    var authoredStateDigest: String
    var canvas: Canvas
    var frames: [LayoutFrameInspection]
}
struct LayoutFrameInspection {
    var frameId: String
    var index: UInt64
    var arrangement: LayoutArrangement
    var cells: [LayoutCellInspection]
}
enum LayoutArrangement { case one, horizontalStack, verticalStack, uniformGrid, custom }
enum LayoutContentMode { case fit, fill, crop }
enum LayoutQuarterTurn { case zero, clockwise90, clockwise180, clockwise270 }
struct LayoutNormalizedRect: Equatable {
    var x: UInt32; var y: UInt32; var width: UInt32; var height: UInt32
}
struct LayoutCellInspection {
    var cellId: String
    var assetId: String?
    var contentMode: LayoutContentMode
    var cropRect: LayoutNormalizedRect?
    var resolvedRect: LayoutNormalizedRect
    var quarterTurn: LayoutQuarterTurn
}

/// Explicit intent payloads. Only a production adapter translates these to revision-checked Core commands.
enum LayoutArrangementAction {
    case one, horizontalStack, verticalStack, uniformGrid(columns: UInt32), custom
}
enum LayoutCellAction {
    case fit(alignmentX: UInt32, alignmentY: UInt32)
    case fill(focalX: UInt32, focalY: UInt32)
    case crop(x: UInt32, y: UInt32, width: UInt32, height: UInt32)
    case setQuarterTurn(quarterTurn: LayoutQuarterTurn)
}
enum LayoutStructureAction {
    case insertFrame(index: UInt64)
    case removeFrame(frameId: String)
    case moveFrame(frameId: String, toIndex: UInt64)
    case setFrameArrangement(frameId: String, arrangement: LayoutArrangementAction)
    case insertCell(frameId: String, index: UInt64)
    case removeCell(frameId: String, cellId: String)
}
