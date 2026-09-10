import AppKit

struct LayoutPresentation {
    struct Preview { var image: NSImage; var metadata: ImagePreviewMetadata }
    var node: NodeInspection?
    var previews: [String: Preview] = [:]
    var nativeThumbnails: [String: NSImage] = [:]
}
struct LayoutActions {
    var requestPreviews: (String) -> Void
    var cell: (String, String, String, LayoutCellAction) -> Void
    func editCell(node: String, frameID: String, cellID: String, edit: LayoutCellAction) {
        cell(node, frameID, cellID, edit)
    }
}
