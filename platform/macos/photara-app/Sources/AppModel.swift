import Foundation
import AppKit
import ImageIO
import SwiftUI
import UniformTypeIdentifiers
import QuickLookThumbnailing

struct RecentProject: Codable, Identifiable, Sendable {
    var projectID: String
    var title: String
    var documentPath: String?
    var lastOpened: Date

    var id: String { projectID }
}

actor NativeThumbnailScheduler {
    private let limit: Int
    private var active = 0
    private var waiters: [CheckedContinuation<Void, Never>] = []

    init(limit: Int) {
        self.limit = limit
    }

    func acquire() async {
        if active < limit {
            active += 1
            return
        }
        await withCheckedContinuation { continuation in
            waiters.append(continuation)
        }
    }

    func release() {
        if waiters.isEmpty {
            active -= 1
        } else {
            waiters.removeFirst().resume()
        }
    }
}

@MainActor
final class AppModel: ObservableObject {
    @Published private(set) var snapshot: BridgeProjectSnapshotDto?
    @Published private(set) var progressLabel = "Idle"
    @Published private(set) var isEvaluating = false
    @Published var gallery = GalleryPresentationState()
    @Published private(set) var layoutCellProxies: [String: BridgeProxyReference] = [:]
    @Published private(set) var layoutNativeThumbnails: [String: NSImage] = [:]
    @Published private(set) var recentProjects: [RecentProject]
    @Published private(set) var nodeDefinitions: [BridgeAvailableNodeDefinitionDto] = []
    @Published private(set) var scanningDiskNodeIDs: Set<String> = []
    @Published var presentedError: String?

    private var application: PhotaraApplication?
    var project: PhotaraProject?
    private var evaluation: EvaluationHandle?
    private var observer: AppEvaluationObserver?
    private let defaults: UserDefaults
    private var projectsDirectory: URL?
    private var activeFolderGrants: [String: URL] = [:]
    private var pendingLayoutProxyCellIDs: Set<String> = []
    private var pendingLayoutNativeCellIDs: Set<String> = []
    private let layoutAuthoringPreviewLongEdge: UInt32
    let nativeThumbnailScheduler = NativeThumbnailScheduler(limit: 2)

    private static let recentProjectsKey = "photara.recent-projects.v1"

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
        let configuredLongEdge = defaults.integer(
            forKey: "photara.layout-authoring-preview-long-edge.v1"
        )
        layoutAuthoringPreviewLongEdge = [512, 1_024, 2_048].contains(configuredLongEdge)
            ? UInt32(configuredLongEdge) : 1_024
        let legacyDefaults = UserDefaults(suiteName: "Photara")
        recentProjects = (defaults.data(forKey: Self.recentProjectsKey)
            ?? legacyDefaults?.data(forKey: Self.recentProjectsKey))
            .flatMap { try? JSONDecoder().decode([RecentProject].self, from: $0) } ?? []
        do {
            let support = try FileManager.default.url(
                for: .applicationSupportDirectory,
                in: .userDomainMask,
                appropriateFor: nil,
                create: true
            )
            let storeRoot = support.appending(path: "Photara/GenerationTwo")
            let proxyCacheRoot = support.appending(path: "Photara/ProxyCache")
            try FileManager.default.createDirectory(
                at: storeRoot,
                withIntermediateDirectories: true
            )
            let helper = Bundle.main.executableURL?
                .deletingLastPathComponent()
                .appending(path: "photara-proxy-imageio")
                .path ?? "photara-proxy-imageio"
            let application = try PhotaraApplication.open(
                storeRoot: storeRoot.path,
                proxyCacheRoot: proxyCacheRoot.path,
                proxyHelperExecutable: helper,
                proxyGenerationConcurrency: Self.recommendedProxyGenerationConcurrency(
                    defaults: defaults
                )
            )
            self.application = application
            nodeDefinitions = application.availableNodeDefinitions()
            projectsDirectory = storeRoot.appending(path: "projects")
            if let documentPath = CommandLine.arguments.dropFirst().first(where: {
                $0.hasSuffix(".photara-project.json")
            }) {
                let project = try application.openProjectDocument(documentPath: documentPath)
                self.project = project
                snapshot = try project.snapshot()
                restoreDiskFolderGrants()
                rememberCurrentProject(documentPath: documentPath)
            }
        } catch {
            presentedError = error.localizedDescription
        }
    }

    var hasOpenProject: Bool { project != nil }
    var galleryProxies: [String: BridgeProxyReference] { gallery.proxies }
    var galleryProxyDescriptors: [String: BridgeProxyDescriptorDto] { gallery.proxyDescriptors }
    var galleryProxyImages: [String: NSImage] { gallery.proxyImages }
    var galleryNativeThumbnails: [String: NSImage] { gallery.nativeThumbnails }
    var galleryDisplayedRevisions: [String: String] { gallery.displayedRevisions }
    var galleryPreviewActivities: [String: GalleryPreviewActivity] { gallery.activities }
    var galleryPreviewErrors: [String: String] { gallery.errors }

    private static func recommendedProxyGenerationConcurrency(
        defaults: UserDefaults
    ) -> UInt32 {
        let configured = defaults.integer(
            forKey: "photara.proxy-generation-concurrency.v1"
        )
        if (1...4).contains(configured) {
            return UInt32(configured)
        }
        let gibibytes = ProcessInfo.processInfo.physicalMemory / (1_024 * 1_024 * 1_024)
        if gibibytes >= 64 { return 4 }
        if gibibytes >= 24 { return 2 }
        return 1
    }

    func newProject() {
        guard let application else { return }
        do {
            let project = try application.createProject(title: "Untitled Project")
            self.project = project
            snapshot = try project.snapshot()
            gallery.reset()
            pendingLayoutProxyCellIDs.removeAll()
            pendingLayoutNativeCellIDs.removeAll()
            layoutCellProxies.removeAll()
            layoutNativeThumbnails.removeAll()
            rememberCurrentProject()
        } catch {
            presentedError = error.localizedDescription
        }
    }

    func closeProject() {
        evaluation?.cancel()
        evaluation = nil
        observer = nil
        project = nil
        snapshot = nil
        gallery.reset()
        pendingLayoutProxyCellIDs.removeAll()
        pendingLayoutNativeCellIDs.removeAll()
        scanningDiskNodeIDs.removeAll()
        layoutCellProxies.removeAll()
        layoutNativeThumbnails.removeAll()
        stopFolderGrants()
    }

    func reopenLastProject() {
        guard let recent = recentProjects.first else { return }
        openRecent(recent)
    }

    func openRecent(_ recent: RecentProject) {
        guard let application else { return }
        do {
            let project: PhotaraProject
            if let path = recent.documentPath,
               FileManager.default.fileExists(atPath: path)
            {
                project = try application.openProjectDocument(documentPath: path)
            } else {
                project = try application.openProject(projectId: recent.projectID)
            }
            self.project = project
            snapshot = try project.snapshot()
            restoreDiskFolderGrants()
            gallery.reset()
            pendingLayoutProxyCellIDs.removeAll()
            pendingLayoutNativeCellIDs.removeAll()
            layoutCellProxies.removeAll()
            layoutNativeThumbnails.removeAll()
            rememberCurrentProject(documentPath: recent.documentPath)
        } catch {
            presentedError = error.localizedDescription
        }
    }

    func chooseAndOpenProject() {
        guard let application else { return }
        let panel = NSOpenPanel()
        panel.title = "Open Photara Project"
        panel.message = "Choose a portable .photara-project.json document."
        panel.allowedContentTypes = [.json]
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        panel.directoryURL = projectsDirectory
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            let project = try application.openProjectDocument(documentPath: url.path)
            self.project = project
            snapshot = try project.snapshot()
            restoreDiskFolderGrants()
            gallery.reset()
            pendingLayoutProxyCellIDs.removeAll()
            pendingLayoutNativeCellIDs.removeAll()
            layoutCellProxies.removeAll()
            layoutNativeThumbnails.removeAll()
            rememberCurrentProject(documentPath: url.path)
        } catch {
            presentedError = error.localizedDescription
        }
    }

    func addLayout() {
        guard let definition = nodeDefinitions.first(where: {
            $0.definitionId == "photara.layout.compose"
        }) else { return }
        addNode(definition)
    }

    func addNode(_ definition: BridgeAvailableNodeDefinitionDto) {
        guard let project, let snapshot else { return }
        accept(project.addNode(
            expectedGraphRevision: snapshot.graph.revision,
            definition: BridgeNodeDefinitionRefDto(
                packageId: definition.packageId,
                packageVersion: definition.packageVersion,
                definitionId: definition.definitionId,
                definitionVersion: definition.definitionVersion
            )
        ))
    }

    func chooseFolder(for node: BridgeNodeDto) {
        guard let project, let disk = node.disk else { return }
        let panel = NSOpenPanel()
        panel.title = "Choose Folder for \(node.brandName)"
        panel.message = "Choose a folder. Individual files are dimmed because Disk grants folder-level access; supported media inside will be discovered."
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            stopFolderGrant(bindingID: disk.folderBindingId)
            guard url.startAccessingSecurityScopedResource() else {
                throw CocoaError(.fileReadNoPermission)
            }
            activeFolderGrants[disk.folderBindingId] = url
            let bookmark = try url.bookmarkData(
                options: [.withSecurityScope],
                includingResourceValuesForKeys: nil,
                relativeTo: nil
            )
            defaults.set(bookmark, forKey: folderBookmarkKey(disk.folderBindingId))
            _ = try project.attachDiskFolder(nodeId: node.nodeId, folderPath: url.path)
            let cleared = project.clearDiskAssets(
                expectedGraphRevision: snapshot?.graph.revision ?? 0,
                nodeId: node.nodeId
            )
            accept(cleared)
            guard cleared.applied else { return }
            gallery.reset()
            layoutCellProxies.removeAll()
            layoutNativeThumbnails.removeAll()
            pendingLayoutProxyCellIDs.removeAll()
            pendingLayoutNativeCellIDs.removeAll()
            scanDisk(node)
        } catch {
            presentedError = error.localizedDescription
        }
    }

    func scanDisk(_ node: BridgeNodeDto) {
        guard let project, let snapshot,
              scanningDiskNodeIDs.insert(node.nodeId).inserted
        else { return }
        let revision = snapshot.graph.revision
        progressLabel = "Discovering \(node.brandName)…"
        Task { [weak self] in
            let discovery = await Task.detached(priority: .userInitiated) {
                project.discoverDiskFolder(
                    expectedGraphRevision: revision,
                    nodeId: node.nodeId
                )
            }.value
            guard let self else { return }
            guard self.project === project else { return }
            accept(discovery)
            guard discovery.applied, let discovered = discovery.snapshot else {
                scanningDiskNodeIDs.remove(node.nodeId)
                progressLabel = "Disk discovery failed"
                return
            }
            progressLabel = "\(discovered.assets.count) assets found · loading previews…"
            let discoveredAssetIDs = Set(discovered.assets.map(\.assetId))
            guard await waitForInitialPreviews(
                assetIDs: discoveredAssetIDs,
                project: project
            ) else {
                scanningDiskNodeIDs.remove(node.nodeId)
                return
            }
            progressLabel = "Previews visible · verifying bytes…"
            let verificationRevision = self.snapshot?.graph.revision
                ?? discovered.graph.revision
            let verification = await Task.detached(priority: .utility) {
                project.scanDiskFolder(
                    expectedGraphRevision: verificationRevision,
                    nodeId: node.nodeId
                )
            }.value
            guard self.project === project else { return }
            scanningDiskNodeIDs.remove(node.nodeId)
            accept(verification)
            if verification.applied {
                for asset in self.snapshot?.assets ?? []
                    where discoveredAssetIDs.contains(asset.assetId)
                {
                    guard gallery.proxies[asset.assetId] != nil
                            || gallery.nativeThumbnails[asset.assetId] != nil,
                          let revision = asset.visualRevision
                    else { continue }
                    gallery.displayedRevisions[asset.assetId] = revision
                    gallery.activities[asset.assetId] = .ready
                }
                progressLabel = "\(verification.snapshot?.assets.count ?? 0) assets verified"
                refreshLayoutProxies()
            } else {
                progressLabel = "Assets visible · verification deferred"
            }
        }
    }

    func performDefaultActivation(for node: BridgeNodeDto) {
        switch node.defaultActivationId {
        case "photara.disk.open-folder":
            guard let disk = node.disk else { return }
            if activeFolderGrants[disk.folderBindingId] == nil {
                restoreDiskFolderGrants()
            }
            if let url = activeFolderGrants[disk.folderBindingId] {
                NSWorkspace.shared.open(url)
            } else {
                chooseFolder(for: node)
            }
        default:
            break
        }
    }

    func connectDiskToAvailableLayout(_ node: BridgeNodeDto) {
        guard let project, let snapshot,
              let layout = snapshot.nodes.first(where: { candidate in
                  candidate.layout != nil && candidate.ports.contains {
                      $0.direction == .input && $0.portId == "assets" && $0.connectedNodeId == nil
                  }
              })
        else {
            presentedError = "Add an unconnected Layout node before connecting Disk."
            return
        }
        accept(project.connectNodes(
            expectedGraphRevision: snapshot.graph.revision,
            outputNodeId: node.nodeId,
            outputPortId: "assets",
            inputNodeId: layout.nodeId,
            inputPortId: "assets"
        ))
    }

    func save() {
        guard let project else { return }
        do {
            snapshot = try project.save()
            rememberCurrentProject()
        } catch {
            presentedError = error.localizedDescription
        }
    }

    func chooseAndImportTiffPair() {
        let panel = NSOpenPanel()
        panel.title = "Import paired HDR and SDR TIFFs"
        panel.message = "Select exactly two TIFF files. Choose the HDR rendition first."
        panel.allowedContentTypes = [.tiff]
        panel.allowsMultipleSelection = true
        panel.canChooseDirectories = false
        guard panel.runModal() == .OK, panel.urls.count == 2 else { return }
        importTiffPair(
            displayName: panel.urls[0].deletingPathExtension().lastPathComponent,
            hdrURL: panel.urls[0],
            sdrURL: panel.urls[1]
        )
    }

    func importTiffPair(displayName: String, hdrURL: URL, sdrURL: URL) {
        guard let project else { return }
        do {
            let imported = try project.importLocalTiffPair(
                displayName: displayName,
                hdrSourcePath: hdrURL.path,
                sdrSourcePath: sdrURL.path
            )
            snapshot = imported.snapshot
            requestGalleryThumbnail(assetID: imported.assetId)
        } catch {
            presentedError = error.localizedDescription
        }
    }

    func bind(assetID: String, to node: BridgeNodeDto, frameID: String, cellID: String) {
        guard let project, let snapshot else { return }
        accept(
            project.bindAssetToLayout(
                expectedGraphRevision: snapshot.graph.revision,
                layoutNodeId: node.nodeId,
                frameId: frameID,
                cellId: cellID,
                assetId: assetID
            )
        )
        requestLayoutProxies(for: node.nodeId)
    }

    func requestLayoutProxies(for nodeID: String) {
        guard let project,
              let node = snapshot?.nodes.first(where: { $0.nodeId == nodeID })
        else { return }
        for frame in node.layout?.frames ?? [] {
            for cell in frame.cells where cell.assetId != nil {
                guard let asset = snapshot?.assets.first(where: {
                    $0.assetId == cell.assetId
                }) else { continue }
                requestLayoutNativeThumbnail(
                    assetID: cell.assetId!,
                    cellID: cell.cellId,
                    project: project
                )
                // Runtime materialization verifies bytes against the representation
                // fingerprint. Cheaply observed Disk revisions intentionally cannot
                // enter that path until background verification has completed.
                guard asset.visualRevisionVerified else { continue }
                guard layoutCellProxies[cell.cellId] == nil,
                      pendingLayoutProxyCellIDs.insert(cell.cellId).inserted
                else { continue }
                let cellID = cell.cellId
                let frameID = frame.frameId
                let authoringPreviewLongEdge = layoutAuthoringPreviewLongEdge
                Task { [weak self] in
                    let result = await Task.detached(priority: .userInitiated) {
                        Result {
                            try project.requestLayoutCellPreview(
                                layoutNodeId: node.nodeId,
                                frameId: frameID,
                                cellId: cellID,
                                maxLongEdge: authoringPreviewLongEdge
                            )
                        }
                    }.value
                    guard let self else { return }
                    pendingLayoutProxyCellIDs.remove(cellID)
                    guard self.project === project, !Task.isCancelled else { return }
                    switch result {
                    case let .success(reference):
                        layoutCellProxies[cellID] = reference
                    case let .failure(error):
                        presentedError = error.localizedDescription
                    }
                }
            }
        }
    }

    private func requestLayoutNativeThumbnail(
        assetID: String,
        cellID: String,
        project: PhotaraProject
    ) {
        guard layoutNativeThumbnails[cellID] == nil,
              pendingLayoutNativeCellIDs.insert(cellID).inserted
        else { return }
        guard let source = try? project.nativeThumbnailSource(assetId: assetID) else {
            pendingLayoutNativeCellIDs.remove(cellID)
            return
        }
        let request = QLThumbnailGenerator.Request(
            fileAt: URL(fileURLWithPath: source.localPath),
            size: CGSize(width: 512, height: 512),
            scale: 1,
            representationTypes: .thumbnail
        )
        Task { [weak self] in
            guard let self else { return }
            defer { pendingLayoutNativeCellIDs.remove(cellID) }
            guard let representation = try? await QLThumbnailGenerator.shared
                .generateBestRepresentation(for: request),
                  !Task.isCancelled,
                  self.project === project,
                  snapshot?.assets.contains(where: { $0.assetId == assetID }) == true
            else { return }
            layoutNativeThumbnails[cellID] = NSImage(
                cgImage: representation.cgImage,
                size: representation.contentRect.size
            )
        }
    }

    func editCell(
        node: BridgeNodeDto,
        frameID: String,
        cellID: String,
        edit: BridgeLayoutCellEdit
    ) {
        guard let project, let snapshot else { return }
        accept(
            project.editLayoutCell(
                expectedGraphRevision: snapshot.graph.revision,
                nodeId: node.nodeId,
                frameId: frameID,
                cellId: cellID,
                edit: edit
            )
        )
        requestLayoutProxies(for: node.nodeId)
    }

    func editStructure(node: BridgeNodeDto, edit: BridgeLayoutStructureEdit) {
        guard let project, let snapshot else { return }
        accept(
            project.editLayoutStructure(
                expectedGraphRevision: snapshot.graph.revision,
                nodeId: node.nodeId,
                edit: edit
            )
        )
        requestLayoutProxies(for: node.nodeId)
    }

    func undoLayout() {
        guard let project, let snapshot else { return }
        accept(project.undoLayout(expectedGraphRevision: snapshot.graph.revision))
        refreshLayoutProxies()
    }

    func redoLayout() {
        guard let project, let snapshot else { return }
        accept(project.redoLayout(expectedGraphRevision: snapshot.graph.revision))
        refreshLayoutProxies()
    }

    func evaluate() {
        guard let project else { return }
        do {
            let evaluation = try project.prepareEvaluation()
            let observer = AppEvaluationObserver(
                onProgress: { [weak self] progress in
                    Task { @MainActor in
                        self?.progressLabel = "\(progress.phase) \(progress.completedNodes)/\(progress.totalNodes)"
                    }
                },
                onFinished: { [weak self] result in
                    Task { @MainActor in
                        self?.isEvaluating = false
                        self?.progressLabel = "\(result.status)"
                        if let error = result.error {
                            self?.presentedError = error.message
                        }
                    }
                }
            )
            self.evaluation = evaluation
            self.observer = observer
            isEvaluating = true
            progressLabel = "Starting"
            try evaluation.start(observer: observer)
        } catch {
            isEvaluating = false
            presentedError = error.localizedDescription
        }
    }

    func cancelEvaluation() {
        evaluation?.cancel()
    }

    private func accept(_ response: BridgeCommandResponseDto) {
        if let snapshot = response.snapshot, response.applied {
            self.snapshot = snapshot
            let liveCellIDs = Set(
                snapshot.nodes.flatMap { node in
                    node.layout?.frames.flatMap(\.cells).map(\.cellId) ?? []
                }
            )
            layoutCellProxies = layoutCellProxies.filter { liveCellIDs.contains($0.key) }
            layoutNativeThumbnails = layoutNativeThumbnails.filter {
                liveCellIDs.contains($0.key)
            }
        } else {
            presentedError = response.error?.message ?? "Semantic command was rejected"
        }
    }

    private func restoreDiskFolderGrants() {
        guard let project else { return }
        for node in snapshot?.nodes ?? [] {
            guard let disk = node.disk,
                  activeFolderGrants[disk.folderBindingId] == nil,
                  let bookmark = defaults.data(forKey: folderBookmarkKey(disk.folderBindingId))
            else { continue }
            do {
                var stale = false
                let url = try URL(
                    resolvingBookmarkData: bookmark,
                    options: [.withSecurityScope],
                    relativeTo: nil,
                    bookmarkDataIsStale: &stale
                )
                guard url.startAccessingSecurityScopedResource() else { continue }
                activeFolderGrants[disk.folderBindingId] = url
                _ = try project.attachDiskFolder(nodeId: node.nodeId, folderPath: url.path)
                if stale {
                    let refreshed = try url.bookmarkData(
                        options: [.withSecurityScope],
                        includingResourceValuesForKeys: nil,
                        relativeTo: nil
                    )
                    defaults.set(refreshed, forKey: folderBookmarkKey(disk.folderBindingId))
                }
            } catch {
                presentedError = "Could not restore \(node.brandName) folder permission: \(error.localizedDescription)"
            }
        }
    }

    private func folderBookmarkKey(_ bindingID: String) -> String {
        "photara.disk.folder-bookmark.v1.\(bindingID)"
    }

    private func stopFolderGrant(bindingID: String) {
        activeFolderGrants.removeValue(forKey: bindingID)?.stopAccessingSecurityScopedResource()
    }

    private func stopFolderGrants() {
        for url in activeFolderGrants.values {
            url.stopAccessingSecurityScopedResource()
        }
        activeFolderGrants.removeAll()
    }

    private func refreshLayoutProxies() {
        layoutCellProxies.removeAll()
        layoutNativeThumbnails.removeAll()
        for node in snapshot?.nodes ?? [] where node.layout != nil {
            requestLayoutProxies(for: node.nodeId)
        }
    }

    private func rememberCurrentProject(documentPath: String? = nil) {
        guard let snapshot else { return }
        let retainedPath = documentPath ?? recentProjects
            .first { $0.projectID == snapshot.projectId }?
            .documentPath
        recentProjects.removeAll { $0.projectID == snapshot.projectId }
        recentProjects.insert(
            RecentProject(
                projectID: snapshot.projectId,
                title: snapshot.title,
                documentPath: retainedPath,
                lastOpened: Date()
            ),
            at: 0
        )
        recentProjects = Array(recentProjects.prefix(10))
        if let data = try? JSONEncoder().encode(recentProjects) {
            defaults.set(data, forKey: Self.recentProjectsKey)
        }
    }
}

private final class AppEvaluationObserver: EvaluationObserver, @unchecked Sendable {
    private let progress: @Sendable (BridgeEvaluationProgressDto) -> Void
    private let finished: @Sendable (BridgeEvaluationFinishedDto) -> Void

    init(
        onProgress: @escaping @Sendable (BridgeEvaluationProgressDto) -> Void,
        onFinished: @escaping @Sendable (BridgeEvaluationFinishedDto) -> Void
    ) {
        progress = onProgress
        finished = onFinished
    }

    func onProgress(progress: BridgeEvaluationProgressDto) {
        self.progress(progress)
    }

    func onFinished(result: BridgeEvaluationFinishedDto) {
        finished(result)
    }
}
