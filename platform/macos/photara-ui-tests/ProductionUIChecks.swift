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
        let support = root.appending(path: "store-\(UUID().uuidString)")
        let app = AppModel(defaults: defaults, supportRootOverride: support)
        require(!FileManager.default.fileExists(atPath: support.appending(path: "Library/library.sqlite").path), "Library was not lazy")
        let workspace = WorkspaceModel(defaults: defaults)
        require(app.presentedError == nil, "Production host failed to open")
        for dark in [false, true] {
            try await capture(LabAppearance(dark: dark) {
                WorkspaceView().environmentObject(app).environmentObject(workspace)
            }, name: "production-launcher-\(dark)", size: .init(width: 980, height: 720), directory: root)
        }
        app.newProject()
        workspace.synchronizeProject(id: app.snapshot?.projectId, nodeIDs: [])
        let emptyDigest = app.snapshot!.graph.digest
        for dark in [false, true] {
            try await capture(LabAppearance(dark: dark) {
                WorkspaceView().environmentObject(app).environmentObject(workspace)
            }, name: "production-empty-\(dark)", size: .init(width: 820, height: 720), directory: root)
        }
        require(app.snapshot!.graph.digest == emptyDigest, "Empty shell mutated graph")
        let emptyPolicy = ApplicationShellAvailability(presentation: app.applicationPresentation(workspace))
        require(Set(WorkspaceRegion.allCases.flatMap { emptyPolicy.visiblePanels(in: $0, workspace: workspace) }) == Set([.graph, .projectInfo]), "New Project did not disclose Project Info setup alongside Graph")
        require(emptyPolicy.modes == [.graph], "Layout icon appeared before a Layout node existed")
        require(app.recentProjects.isEmpty, "Untitled draft leaked into Recent Projects")
        // The fixture uses the bridge's explicit Layout + Project Assets setup;
        // Project Assets is intentionally hidden from the interactive node catalog.
        let added = app.project!.addLayoutNode(expectedGraphRevision: app.snapshot!.graph.revision,
                                               canvas: .portrait3x4(longEdgePixels: 4000))
        require(added.applied, "Could not create fixture Layout with explicit AssetSet input")
        app.save()
        require(app.recentProjects.isEmpty, "Saved untitled draft leaked into Recent Projects")
        let diskDefinition = app.nodeDefinitions.first { $0.definitionId == "photara.disk.folder" }!
        app.addNode(diskDefinition, graphPosition: .init(x: -240, y: -80))
        let layout = app.snapshot!.nodes.first { $0.layout != nil }!
        require(layout.hasLayoutWorkSurface, "Layout node contribution was not recognized")
        require(ApplicationShellAvailability(presentation: app.applicationPresentation(workspace)).modes.contains(.nodeWorkSurface),
                "Layout icon did not appear after adding the Layout node")
        var otherSurface = layout
        otherSurface.workspaceContributionId = "example.other.workspace"
        require(!otherSurface.hasLayoutWorkSurface, "Unrelated node surface was routed to Layout")
        require(ProductionWorkSurfaceRegistry.presentation(for: otherSurface) == nil, "Unsupported contribution advertised a dead toolbar icon")
        let contribution = ProductionWorkSurfaceRegistry.presentation(for: layout)!
        require(contribution.nodeID == layout.nodeId && contribution.iconResourceID == layout.iconResourceId,
                "Toolbar contribution lost its node identity or icon")
        workspace.selectedNodeID = layout.nodeId
        var secondLayout = layout
        secondLayout.nodeId = "second-layout-fixture"
        let surfaces = [layout, secondLayout].compactMap { ProductionWorkSurfaceRegistry.presentation(for: $0) }
        require(Set(surfaces.map(\.nodeID)).count == 2, "Work surfaces collapsed distinct node instances")
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
        workspace.synchronizeProject(id: app.snapshot!.projectId, nodeIDs: app.snapshot!.nodes.map(\.nodeId))
        workspace.selectedNodeID = layout.nodeId
        let digest = app.snapshot!.graph.digest
        workspace.selectedNodeID = nil
        require(workspace.inspectorActivated, "Production deselection collapsed Inspector")
        workspace.move(.diagnostics, to: .leading)
        workspace.toggle(.diagnostics)
        workspace.selectedNodeID = layout.nodeId
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
        // Exercise the exact production actor, thumbnail preparation, Library facade and portable assignments.
        var personDraft = LibraryDraft(kind: .person)
        personDraft.name = "Maya Chen"; personDraft.labels = "model"; personDraft.thumbnailSource = sdr
        let person = try await app.library.session.save(personDraft)
        require(person.thumbnailDigest != nil, "Record lost its thumbnail identity")
        await app.library.load()
        require(app.library.presentation.items.first?.thumbnailData != nil, "Thumbnail failed to reopen")
        let library = try await app.library.session.library()
        var info = try app.project!.assignLibraryRecord(library: library, recordId: person.recordId, expectedRevision: 0, relationship: "model")
        for kind in [LibraryKind.client, .location, .scene] {
            var draft = LibraryDraft(kind: kind); draft.name = "Fixture \(kind.title)"
            let record = try await app.library.session.save(draft)
            info = try app.project!.assignLibraryRecord(library: library, recordId: record.recordId, expectedRevision: info.revision, relationship: kind.rawValue)
            if kind == .scene {
                info = try app.project!.assignLibraryRecord(library: library, recordId: record.recordId, expectedRevision: info.revision, relationship: "second occurrence")
            }
        }
        require(info.assignments.count == 5, "Project assignments lost a category or scene occurrence")
        require(Set(info.assignments.map(\.assignmentId)).count == 5, "Project occurrences are not unique")
        do {
            _ = try app.project!.editLibraryAssignment(assignmentId: info.assignments[0].assignmentId, expectedRevision: 0, date: "", notes: "stale", remove: false)
            fatalError("Accepted stale Project Info edit")
        } catch { }
        let first = info.assignments[0]
        info = try app.project!.editLibraryAssignment(assignmentId: first.assignmentId, expectedRevision: info.revision, date: "2026-09-12", notes: "Call time 18:00", remove: false)
        info = try app.project!.editLibraryAssignment(assignmentId: first.assignmentId, expectedRevision: info.revision, date: "", notes: "", remove: true)
        info = try app.project!.undoLibraryAssignment(expectedRevision: info.revision)
        require(info.assignments.first?.notes == "Call time 18:00", "Assignment undo lost occurrence details")
        var rename = LibraryDraft(LibraryItem(person)); rename.name = "Maya Rivera"
        let renamed = try await app.library.session.save(rename)
        _ = try await app.library.session.save(.init(LibraryItem(renamed)), deleted: true)
        let contextAfterDelete = try app.project!.libraryContext()
        require(contextAfterDelete.assignments[0].displayName == "Maya Chen", "Library edits changed project snapshots")
        app.refreshAfterLibraryEdit()
        require(app.snapshot!.dirty && app.snapshot!.graph.digest == digest, "Project context did not stay separate from graph semantics")
        app.save()
        require(app.snapshot?.dirty == false, "Project Info failed to save")
        let reopenedHost = try PhotaraApplication.open(storeRoot: support.appending(path: "GenerationTwo").path,
            proxyCacheRoot: support.appending(path: "ProxyCache").path,
            proxyHelperExecutable: Bundle.main.executableURL!.deletingLastPathComponent().appending(path: "photara-proxy-imageio").path,
            proxyGenerationConcurrency: 1)
        let reopened = try reopenedHost.openProject(projectId: app.snapshot!.projectId)
        let reopenedInfo = try reopened.libraryContext()
        require(reopenedInfo.assignments.count == 5 && reopenedInfo.assignments[0].displayName == "Maya Chen", "Portable Library references failed save/reopen")
        await app.library.load()
        for panel in [WorkspacePanelID.people, .locations, .scenes, .projectInfo, .account] {
            workspace.show(panel)
            workspace.move(panel, to: .trailing)
            workspace.toggle(panel)
            require(!workspace.isVisible(panel), "Library module cannot close")
        }
        workspace.show(.projectInfo); workspace.show(.people)
        for dark in [false, true] {
            try await capture(LabAppearance(dark: dark) {
                WorkspaceView().environmentObject(app).environmentObject(workspace)
            }, name: "production-library-\(dark)", size: .init(width: 1440, height: 1000), directory: root)
        }
        require(app.snapshot?.dirty == false && app.snapshot?.graph.digest == digest, "Module preferences dirtied project semantics")
        let projectID = app.snapshot!.projectId
        print("PASS: production composition, inspection adapter, Core action/undo/redo, HDR proxy, SQLite Library, thumbnails, project snapshots, save/reopen, presentation purity")
        print("Verification project ID: \(projectID)")
        app.closeProject()
    }
}
