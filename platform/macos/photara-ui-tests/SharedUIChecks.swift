import AppKit
import SwiftUI

@main
struct SharedUIChecks {
    @MainActor static func main() async throws {
        NSApplication.shared.setActivationPolicy(.accessory)
        let directory = URL(fileURLWithPath: CommandLine.arguments[1])
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let preset = GalleryPreset.shipped
        let decoded = try GalleryPreset.decode(preset.encoded())
        require(decoded == preset, "Gallery preset round trip")
        for invalid in [GalleryPreset(schemaVersion: 2, defaultThumbnailSize: 112, photoSpacing: 2, squareRowSpacing: 10, selectionStrokeWidth: 1.5),
                        GalleryPreset(schemaVersion: 1, defaultThumbnailSize: -1, photoSpacing: 2, squareRowSpacing: 10, selectionStrokeWidth: 1.5)] {
            do { _ = try GalleryPreset.decode(JSONEncoder().encode(invalid)); fatalError("Accepted invalid preset") }
            catch { }
        }
        let shellPreset = ApplicationShellPreset.shipped
        let shellDecoded = try ApplicationShellPreset.decode(shellPreset.encoded())
        require(shellDecoded == shellPreset, "Shell preset round trip")
        var badVersion = shellPreset; badVersion.schemaVersion = 2
        var badSize = shellPreset; badSize.launcherTitleSize = -1
        var badRatio = shellPreset; badRatio.heroSize = 72; badRatio.heroIconSize = 88
        var badPane = shellPreset; badPane.trailingIdealWidth = 999
        for invalid in [badVersion, badSize, badRatio, badPane] {
            do { _ = try ApplicationShellPreset.decode(JSONEncoder().encode(invalid)); fatalError("Accepted invalid shell preset") }
            catch { }
        }
        let shellModel = ShellLabModel()
        shellModel.scenario = .opening
        require(ApplicationShellAvailability(presentation: shellModel.presentation).panels.isEmpty, "Opening exposed project commands")
        shellModel.scenario = .emptyProject
        func visible() -> [WorkspacePanelID] {
            let policy = ApplicationShellAvailability(presentation: shellModel.presentation)
            return WorkspaceRegion.allCases.flatMap { policy.visiblePanels(in: $0, workspace: shellModel.workspace) }
        }
        require(visible() == [.graph], "Empty project reserved a pane")
        require(ApplicationShellAvailability(presentation: shellModel.presentation).modes == [.graph], "Empty project exposed unavailable modes")
        shellModel.addNode()
        require(shellModel.workspace.selectedNodeID == "disk" && visible().contains(.inspector), "First node did not reveal Inspector")
        shellModel.workspace.selectedNodeID = nil
        require(visible().contains(.inspector), "Clearing selection collapsed Inspector")
        require(!visible().contains(.assetGallery), "Gallery opened without context")
        shellModel.workspace.show(.assetGallery)
        require(visible().contains(.assetGallery), "Explicit Gallery request ignored")
        shellModel.workspace.show(.diagnostics)
        require(visible().contains(.diagnostics), "Explicit diagnostics request ignored")
        shellModel.scenario = .emptyProject
        require(visible() == [.graph], "New empty project leaked previous session disclosure")
        shellModel.scenario = .layout
        require(ApplicationShellAvailability(presentation: shellModel.presentation).modes == [.graph, .nodeWorkSurface], "Layout capability unavailable")
        var otherSurface = ShellScenario.layoutSurface
        otherSurface.nodeID = "another-node"; otherSurface.contributionID = "example.other.workspace"
        otherSurface.iconResourceID = "example.other.icon"
        shellModel.presentation.workSurfaces.append(otherSurface)
        require(shellModel.presentation.workSurfaces.count == 2 && shellModel.presentation.workSurfaces.last?.iconResourceID == "example.other.icon",
                "Shell lost a second node-owned work-surface contribution")
        shellModel.presentation.workSurfaces = []
        require(ApplicationShellAvailability(presentation: shellModel.presentation).modes == [.graph], "Removed surfaces left navigation behind")
        shellModel.scenario = .review
        require(ApplicationShellAvailability(presentation: shellModel.presentation).modes.contains(.review), "Review result unavailable")
        shellModel.workspace.toggle(.assetGallery)
        require(shellModel.workspace.mode == .graph, "Hiding the active review surface did not return to Graph")
        shellModel.workspace.activateWorkspace(for: "layout")
        shellModel.workspace.requestNodeMenu()
        require(shellModel.workspace.mode == .graph && shellModel.workspace.consumeNodeMenuRequest(), "Catalog request lost while switching surfaces")
        require(!shellModel.workspace.consumeNodeMenuRequest(), "Catalog request replayed")
        shellModel.scenario = .saved
        require(ApplicationShellAvailability(presentation: shellModel.presentation).hasStatus, "Save status hidden")
        shellModel.scenario = .evaluating
        shellModel.send(.cancel)
        require(!shellModel.presentation.isEvaluating, "Cancel did not clear progress")
        for dark in [false, true] {
            for scenario in ShellScenario.allCases {
                shellModel.scenario = scenario; shellModel.dark = dark
                try await capture(ShellLabPreview(model: shellModel, workspace: shellModel.workspace),
                    name: "shell-\(scenario.rawValue)-\(dark)",
                    size: .init(width: scenario == .compact ? 820 : 1440, height: scenario == .compact ? 720 : 900), directory: directory)
            }
        }
        let assets = GalleryFixtures.assets()
        require(assets.contains { $0.aspectRatio < 1 } && assets.contains { $0.aspectRatio > 1 }
                && assets.contains { $0.aspectRatio == 1 }, "Gallery aspect coverage")
        for activity in [GalleryPreviewActivity.loading, .updating, .failed, .ready] {
            require(assets.contains { $0.activity == activity }, "Missing Gallery activity")
        }
        require(assets.contains { $0.representationCount > 1 }, "Missing representations")
        require(!assets[4].canViewFull && assets[1].canViewFull, "Focused image readiness contract")
        var retainedDimensions = assets[1]
        retainedDimensions.isProxyAvailable = false
        require(!retainedDimensions.canViewFull && retainedDimensions.aspectRatio == assets[1].aspectRatio,
                "Retained proxy dimensions must survive unavailable focused-image access")
        let hdr = assets[1].proxyImage!
        let container = HDRImageContainerView(frame: .init(x: 0, y: 0, width: 300, height: 100))
        container.update(image: hdr, sizingMode: .fit, preferredDynamicRange: .constrainedHigh)
        container.layoutSubtreeIfNeeded()
        let native = container.subviews.compactMap { $0 as? NSImageView }.first!
        require(native.preferredImageDynamicRange == .constrainedHigh, "Native thumbnail HDR policy")
        require(abs(native.frame.height - 100) < 0.01 && native.frame.width < 100, "Native fit geometry")
        container.update(image: hdr, sizingMode: .fill, preferredDynamicRange: .high)
        container.layoutSubtreeIfNeeded()
        require(native.preferredImageDynamicRange == .high && native.frame.width == 300, "Native focused HDR/fill policy")
        var proposed = NSRect(origin: .zero, size: hdr.size)
        let cg = hdr.cgImage(forProposedRect: &proposed, context: nil, hints: nil)!
        require(cg.bitsPerComponent == 32 && cg.bitmapInfo.contains(.floatComponents), "HDR fixture lost float pixels")
        let pixelData = cg.dataProvider!.data! as Data
        let maximum = pixelData.withUnsafeBytes { bytes in bytes.bindMemory(to: Float.self).max()! }
        require(maximum > 1, "HDR fixture contains no extended range")
        let suite = "photara.ui-tests.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let workspace = WorkspaceModel(defaults: defaults)
        let original = workspace.placements
        workspace.move(.assetGallery, to: .leading)
        workspace.activateWorkspace(for: "layout")
        require(!workspace.isVisible(.graph) && workspace.isVisible(.layoutAuthoring), "Shell primary surface switching")
        require(WorkspaceModel(defaults: defaults).placements == workspace.placements, "Workspace preference persistence")
        workspace.selectedNodeID = "transient"
        require(WorkspaceModel(defaults: defaults).selectedNodeID == nil, "Selection leaked into user preferences")
        workspace.restoreLayoutAuthoringPreset()
        require(workspace.placements == original, "Workspace restore")
        var event = ""
        let actions = InspectorActions(chooseFolder: { event = $0 }, scanDisk: { event = $0 }, connectDisk: { event = $0 },
            structure: { id, _ in event = id }, cell: { id, frame, cell, _ in event = "\(id)/\(frame)/\(cell)" })
        let inspector = InspectorView(presentation: InspectorFixture.layout.presentation, actions: actions)
        let node = inspector.selectedNode!, frame = inspector.selectedFrame!, cell = inspector.selectedCell!
        inspector.contentModeBinding(node, frame, cell).wrappedValue = .fill
        require(event == "layout/frame-0/cell-0-0", "Inspector callback lost target identity")
        inspector.arrangementBinding(node, frame).wrappedValue = .vertical
        require(event == "layout", "Inspector structure callback lost node identity")
        for dark in [false, true] {
            for style in [GalleryViewStyle.photoGrid, .squareGrid] {
                try await capture(LabAppearance(dark: dark) {
                    AssetGalleryView(presentation: .init(assets: assets, canAssign: true),
                        actions: .init(open: { _ in }, assign: { _ in }, requestPreview: { _ in }),
                        initialStyle: style, filter: .constant(""), selectedAssetID: .constant("asset-1"))
                }, name: "gallery-\(style)-\(dark)", size: .init(width: 720, height: 620), directory: directory)
            }
            for fixture in InspectorFixture.allCases {
                try await capture(LabAppearance(dark: dark) {
                    InspectorView(presentation: fixture.presentation, actions: actions)
                }, name: "inspector-\(fixture.rawValue)-\(dark)", size: .init(width: 360, height: 900), directory: directory)
            }
        }
        for section in InspectorSection.allCases where section != .all {
            let fixture = section == .disk ? InspectorFixture.disk : section == .diagnostics ? .diagnostics : .layout
            try await capture(LabAppearance(dark: true) {
                InspectorView(presentation: fixture.presentation, actions: actions, section: section)
            }, name: "section-\(section.rawValue)", size: .init(width: 320, height: 820), directory: directory)
        }
        print("PASS: shared contracts, Gallery preset, float HDR/native policy, workspace preferences, Inspector targets, 54 rendered states including Shell disclosure")
    }
}
