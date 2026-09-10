import AppKit
import SwiftUI
import CoreImage

@main
struct ProductionUIChecks {
    @MainActor static func main() async throws {
        NSApplication.shared.setActivationPolicy(.accessory)
        let root = URL(fileURLWithPath: CommandLine.arguments[1])
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        let suite = "photara.production-ui-tests.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defaults.set(Data("[]".utf8), forKey: "photara.recent-projects.v1")
        defer { defaults.removePersistentDomain(forName: suite) }
        let app = AppModel(defaults: defaults, supportRootOverride: root.appending(path: "store-\(UUID().uuidString)"))
        let workspace = WorkspaceModel(defaults: defaults)
        require(app.presentedError == nil, "Production host failed to open")
        app.newProject()
        // The fixture uses the bridge's explicit Layout + Project Assets setup;
        // Project Assets is intentionally hidden from the interactive node catalog.
        let added = app.project!.addLayoutNode(expectedGraphRevision: app.snapshot!.graph.revision,
                                               canvas: .portrait3x4(longEdgePixels: 4000))
        require(added.applied, "Could not create fixture Layout with explicit AssetSet input")
        app.save()
        let diskDefinition = app.nodeDefinitions.first { $0.definitionId == "photara.disk.folder" }!
        app.addNode(diskDefinition, graphPosition: .init(x: -240, y: -80))
        let layout = app.snapshot!.nodes.first { $0.layout != nil }!
        workspace.selectedNodeID = layout.nodeId
        let node = NodeInspection(layout)
        require(node.nodeId == layout.nodeId && node.layout?.frames.count == layout.layout?.frames.count,
                "Production inspection mapping")
        let frame = node.layout!.frames[0], cell = frame.cells[0]
        let revision = app.snapshot!.graph.revision
        app.inspectorActions.editCell(node: node.nodeId, frameID: frame.frameId, cellID: cell.cellId,
                                      edit: .crop(x: 100_000, y: 100_000, width: 800_000, height: 800_000))
        require(app.snapshot!.graph.revision > revision, "Shared Inspector action did not enter Core")
        let updated = app.snapshot!.nodes.first { $0.nodeId == layout.nodeId }!
        require(updated.layout!.frames[0].cells[0].cropRect?.width == 800_000, "Crop intent translation")
        app.undoLayout(); app.redoLayout()
        require(app.presentedError == nil, "Adapter command/undo failed")
        let image = CIImage(color: CIColor(red: 0.4, green: 0.6, blue: 0.8)).cropped(to: .init(x: 0, y: 0, width: 320, height: 240))
        let sdr = root.appending(path: "sdr.tiff"), hdr = root.appending(path: "hdr.tiff")
        let context = CIContext()
        try context.writeTIFFRepresentation(of: image, to: sdr, format: .RGBA8,
            colorSpace: CGColorSpace(name: CGColorSpace.sRGB)!, options: [:])
        try context.writeTIFFRepresentation(of: image.applyingFilter("CIExposureAdjust", parameters: ["inputEV": 2]),
            to: hdr, format: .RGBAh, colorSpace: CGColorSpace(name: CGColorSpace.extendedLinearDisplayP3)!, options: [:])
        app.importTiffPair(displayName: "UI Verification HDR/SDR", hdrURL: hdr, sdrURL: sdr)
        require(app.snapshot!.assets.count == 1, "Fixture import failed")
        let assetID = app.snapshot!.assets[0].assetId
        app.requestGalleryThumbnail(assetID: assetID)
        for _ in 0..<200 {
            if app.galleryProxyImages[assetID] != nil { break }
            try await Task.sleep(for: .milliseconds(100))
        }
        require(app.galleryProxyImages[assetID] != nil, "Native proxy did not become ready")
        app.bind(assetID: assetID, to: updated, frameID: frame.frameId, cellID: cell.cellId)
        app.save()
        require(app.presentedError == nil && app.snapshot?.dirty == false, "Save failed after shared actions: \(app.presentedError ?? "none"), dirty=\(String(describing: app.snapshot?.dirty))")
        let digest = app.snapshot!.graph.digest
        for dark in [false, true] {
            try await capture(LabAppearance(dark: dark) {
                WorkspaceView().environmentObject(app).environmentObject(workspace)
            }, name: "production-graph-\(dark)", size: .init(width: 1440, height: 900), directory: root)
            workspace.activateWorkspace(for: layout.nodeId)
            try await capture(LabAppearance(dark: dark) {
                WorkspaceView().environmentObject(app).environmentObject(workspace)
            }, name: "production-layout-\(dark)", size: .init(width: 1440, height: 900), directory: root)
            workspace.activateGraph()
        }
        require(app.snapshot!.graph.digest == digest, "Presentation mutated the graph")
        let projectID = app.snapshot!.projectId
        print("PASS: production composition, inspection adapter, Core action/undo/redo, HDR proxy, save, presentation purity")
        print("Verification project ID: \(projectID)")
        app.closeProject()
    }
}
