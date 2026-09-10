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
        require(preset.noSourceState.title == "Your project assets will appear here",
                "Gallery no-source guidance changed unexpectedly")
        var legacyGalleryJSON = try JSONSerialization.jsonObject(with: preset.encoded()) as! [String: Any]
        legacyGalleryJSON.removeValue(forKey: "noSourceState")
        legacyGalleryJSON.removeValue(forKey: "awaitingAssetsState")
        legacyGalleryJSON.removeValue(forKey: "noMatchesState")
        let legacyGallery = try GalleryPreset.decode(JSONSerialization.data(withJSONObject: legacyGalleryJSON))
        require(legacyGallery.noSourceState.actionTitle == "Add Source Node",
                "Legacy Gallery preset did not receive empty-state defaults")
        for invalid in [GalleryPreset(schemaVersion: 2, defaultThumbnailSize: 112, photoSpacing: 2, squareRowSpacing: 10, selectionStrokeWidth: 1.5),
                        GalleryPreset(schemaVersion: 1, defaultThumbnailSize: -1, photoSpacing: 2, squareRowSpacing: 10, selectionStrokeWidth: 1.5)] {
            do { _ = try GalleryPreset.decode(JSONEncoder().encode(invalid)); fatalError("Accepted invalid preset") }
            catch { }
        }
        var invalidGalleryState = preset
        invalidGalleryState.noSourceState.iconSize = 999
        do { _ = try GalleryPreset.decode(invalidGalleryState.encoded()); fatalError("Accepted invalid Gallery empty state") }
        catch { }
        let inspectorPreset = InspectorPreset.shipped
        let decodedInspectorPreset = try InspectorPreset.decode(inspectorPreset.encoded())
        require(decodedInspectorPreset == inspectorPreset, "Inspector preset round trip")
        var invalidInspectorPreset = inspectorPreset
        invalidInspectorPreset.noSelectionState.title = ""
        do { _ = try InspectorPreset.decode(invalidInspectorPreset.encoded()); fatalError("Accepted invalid Inspector empty state") }
        catch { }
        let shellPreset = ApplicationShellPreset.shipped
        let shellDecoded = try ApplicationShellPreset.decode(shellPreset.encoded())
        require(shellDecoded == shellPreset, "Shell preset round trip")
        require(shellPreset.launcherTitleFont == .display, "Shipped shell title font is not Display")
        require(shellPreset.heroShowsTile, "Shipped shell hid the hero icon tile")
        require(shellPreset.heroSymbolColor.isValid && shellPreset.heroTileBackgroundColor.isValid,
                "Shipped hero colors are invalid")
        var legacyShellJSON = try JSONSerialization.jsonObject(with: shellPreset.encoded()) as! [String: Any]
        legacyShellJSON.removeValue(forKey: "launcherTitleFont")
        legacyShellJSON.removeValue(forKey: "heroSymbolOffsetY")
        legacyShellJSON.removeValue(forKey: "heroTileOffsetX")
        legacyShellJSON.removeValue(forKey: "heroTileOffsetY")
        legacyShellJSON.removeValue(forKey: "contentHorizontalInset")
        legacyShellJSON.removeValue(forKey: "launcherVerticalOffset")
        legacyShellJSON.removeValue(forKey: "toolbarShowsProjectTitle")
        legacyShellJSON.removeValue(forKey: "toolbarTitleSize")
        legacyShellJSON.removeValue(forKey: "toolbarTitleWeight")
        legacyShellJSON.removeValue(forKey: "panelHeaderTitleSize")
        legacyShellJSON.removeValue(forKey: "panelHeaderHorizontalInset")
        legacyShellJSON.removeValue(forKey: "dividerThickness")
        legacyShellJSON.removeValue(forKey: "statusTextSize")
        legacyShellJSON.removeValue(forKey: "statusHorizontalInset")
        legacyShellJSON.removeValue(forKey: "statusItemSpacing")
        let legacyShellPreset = try ApplicationShellPreset.decode(JSONSerialization.data(withJSONObject: legacyShellJSON))
        require(legacyShellPreset.launcherTitleFont == .display, "Legacy shell preset did not default to Display")
        require(legacyShellPreset.heroSymbolOffsetY == 0 && legacyShellPreset.heroTileOffsetX == 0 && legacyShellPreset.heroTileOffsetY == 0,
                "Legacy shell preset did not preserve neutral hero positioning")
        require(legacyShellPreset.contentHorizontalInset == 44 && legacyShellPreset.launcherVerticalOffset == 0,
                "Legacy shell preset did not preserve launcher positioning")
        require(legacyShellPreset.toolbarShowsProjectTitle && legacyShellPreset.toolbarTitleSize == 13,
                "Legacy shell preset did not preserve project identity defaults")
        require(legacyShellPreset.dividerThickness == 1 && legacyShellPreset.statusTextSize == 11,
                "Legacy shell preset did not preserve Project Chrome defaults")
        var badVersion = shellPreset; badVersion.schemaVersion = 2
        var badSize = shellPreset; badSize.launcherTitleSize = -1
        var badRatio = shellPreset; badRatio.heroSize = 72; badRatio.heroIconSize = 88
        var badPane = shellPreset; badPane.trailingIdealWidth = 999
        var badColor = shellPreset; badColor.heroSymbolColor.light = "systemBlue"
        var badGlow = shellPreset; badGlow.heroGlowRadius = 999
        var badSymbolOffset = shellPreset; badSymbolOffset.heroSymbolOffsetY = 999
        var badTileOffset = shellPreset; badTileOffset.heroTileOffsetX = 999
        var badLauncherOffset = shellPreset; badLauncherOffset.launcherVerticalOffset = 999
        var badDivider = shellPreset; badDivider.dividerThickness = 999
        for invalid in [badVersion, badSize, badRatio, badPane, badColor, badGlow,
                        badSymbolOffset, badTileOffset, badLauncherOffset, badDivider] {
            do { _ = try ApplicationShellPreset.decode(JSONEncoder().encode(invalid)); fatalError("Accepted invalid shell preset") }
            catch { }
        }
        let shellModel = ShellLabModel()
        shellModel.scenario = .opening
        require(Set(ApplicationShellAvailability(presentation: shellModel.presentation).panels) == Set([.people, .locations, .scenes, .account]), "Opening exposed project commands")
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
        for dark in [false, true] {
            for fixture in LibraryFixtureState.allCases {
                let library = LibraryFixtures.presentation(fixture)
                let actions = LibraryActions(send: { _ in })
                let views: [(String, AnyView)] = [
                    ("people", AnyView(PeopleView(presentation: library, actions: actions))),
                    ("locations", AnyView(LocationsView(presentation: library, actions: actions))),
                    ("scenes", AnyView(ScenesView(presentation: library, actions: actions))),
                    ("project-info", AnyView(ProjectInfoView(presentation: .init(title: "Coastal Studies", revision: 1,
                        assignments: fixture == .populated ? LibraryFixtures.assignments : [], library: library, phase: library.phase, hasProject: true), actions: .init(send: { _ in }))))
                ]
                for (name, view) in views {
                    try await capture(LabAppearance(dark: dark) { view }, name: "library-\(name)-\(fixture.rawValue)-\(dark)", size: .init(width: 440, height: 720), directory: directory)
                }
            }
            shellModel.scenario = .assets; shellModel.dark = dark
            shellModel.workspace.show(.people); shellModel.workspace.show(.projectInfo)
            try await capture(ShellLabPreview(model: shellModel, workspace: shellModel.workspace), name: "shell-library-\(dark)", size: .init(width: 1440, height: 900), directory: directory)
        }
        require(LibraryFixtures.items.allSatisfy { $0.thumbnailData != nil }, "Library thumbnail fixtures missing")
        require(Set(LibraryFixtures.assignments.filter { $0.kind == .scene }.map(\.id)).count == 2, "Scene occurrences collapsed")
        var invalidFrame = shellPreset; invalidFrame.frame.cornerRadius = 999
        do { _ = try invalidFrame.encoded(); fatalError("Accepted invalid surface frame") } catch { }
        legacyShellJSON.removeValue(forKey: "frame")
        let migratedFrame = try ApplicationShellPreset.decode(JSONSerialization.data(withJSONObject: legacyShellJSON))
        require(migratedFrame.frame.cornerRadius == 16, "Legacy Shell frame migration failed")
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
        require(workspace.isVisible(.graph) && workspace.isVisible(.layoutAuthoring), "Independent Graph and Work Surface visibility")
        workspace.toggle(.graph)
        require(!workspace.isVisible(.graph) && workspace.isVisible(.layoutAuthoring), "Closing Graph also closed Work Surface")
        workspace.show(.graph)
        for panel in WorkspacePanelID.allCases {
            require(!panel.symbol.isEmpty, "Workspace module missing icon")
            workspace.show(panel); workspace.toggle(panel)
            require(!workspace.isVisible(panel), "Panel cannot close")
            workspace.show(panel)
        }
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
            let galleryEmptyStates: [(String, GalleryPresentation, String)] = [
                ("no-source", .init(assets: [], canAssign: false, hasSourceNodes: false), ""),
                ("awaiting", .init(assets: [], canAssign: false, hasSourceNodes: true), ""),
                ("no-matches", .init(assets: assets, canAssign: false, hasSourceNodes: true), "No fixture matches"),
            ]
            for state in galleryEmptyStates {
                try await capture(LabAppearance(dark: dark) {
                    AssetGalleryView(presentation: state.1,
                        actions: .init(open: { _ in }, assign: { _ in }, requestPreview: { _ in }),
                        filter: .constant(state.2), selectedAssetID: .constant(nil))
                }, name: "gallery-\(state.0)-\(dark)", size: .init(width: 720, height: 620), directory: directory)
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
        print("PASS: shared contracts, feature presets, float HDR/native policy, workspace preferences, Inspector targets, rendered Gallery/Inspector/Shell/Library states, thumbnails and independent modules")
    }
}
