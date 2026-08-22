import Foundation
import AppKit
import SwiftUI
import UniformTypeIdentifiers

struct RecentProject: Codable, Identifiable, Sendable {
    var projectID: String
    var title: String
    var documentPath: String?
    var lastOpened: Date

    var id: String { projectID }
}

@MainActor
final class AppModel: ObservableObject {
    @Published private(set) var snapshot: BridgeProjectSnapshotDto?
    @Published private(set) var progressLabel = "Idle"
    @Published private(set) var isEvaluating = false
    @Published private(set) var galleryProxies: [String: BridgeProxyReference] = [:]
    @Published private(set) var layoutCellProxies: [String: BridgeProxyReference] = [:]
    @Published private(set) var recentProjects: [RecentProject]
    @Published var presentedError: String?

    private var application: PhotaraApplication?
    private var project: PhotaraProject?
    private var evaluation: EvaluationHandle?
    private var observer: AppEvaluationObserver?
    private let defaults: UserDefaults
    private var projectsDirectory: URL?

    private static let recentProjectsKey = "photara.recent-projects.v1"

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
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
                proxyHelperExecutable: helper
            )
            self.application = application
            projectsDirectory = storeRoot.appending(path: "projects")
            if let documentPath = CommandLine.arguments.dropFirst().first(where: {
                $0.hasSuffix(".photara-project.json")
            }) {
                let project = try application.openProjectDocument(documentPath: documentPath)
                self.project = project
                snapshot = try project.snapshot()
                rememberCurrentProject(documentPath: documentPath)
            }
        } catch {
            presentedError = error.localizedDescription
        }
    }

    var hasOpenProject: Bool { project != nil }

    func newProject() {
        guard let application else { return }
        do {
            let project = try application.createProject(title: "Untitled Project")
            self.project = project
            snapshot = try project.snapshot()
            galleryProxies.removeAll()
            layoutCellProxies.removeAll()
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
        galleryProxies.removeAll()
        layoutCellProxies.removeAll()
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
            galleryProxies.removeAll()
            layoutCellProxies.removeAll()
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
            galleryProxies.removeAll()
            layoutCellProxies.removeAll()
            rememberCurrentProject(documentPath: url.path)
        } catch {
            presentedError = error.localizedDescription
        }
    }

    func addLayout() {
        guard let project, let snapshot else { return }
        accept(
            project.addLayoutNode(
                expectedGraphRevision: snapshot.graph.revision,
                canvas: .portrait3x4(longEdgePixels: 4000)
            )
        )
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

    func requestGalleryThumbnail(assetID: String) {
        guard galleryProxies[assetID] == nil, let project else { return }
        do {
            galleryProxies[assetID] = try project.requestGalleryThumbnail(assetId: assetID)
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
                do {
                    layoutCellProxies[cell.cellId] = try project.requestLayoutCellPreview(
                        layoutNodeId: node.nodeId,
                        frameId: frame.frameId,
                        cellId: cell.cellId
                    )
                } catch {
                    presentedError = error.localizedDescription
                }
            }
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
        } else {
            presentedError = response.error?.message ?? "Semantic command was rejected"
        }
    }

    private func refreshLayoutProxies() {
        layoutCellProxies.removeAll()
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
