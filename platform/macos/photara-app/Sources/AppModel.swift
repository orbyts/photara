import Foundation
import AppKit
import SwiftUI
import UniformTypeIdentifiers

@MainActor
final class AppModel: ObservableObject {
    @Published private(set) var snapshot: BridgeProjectSnapshotDto?
    @Published private(set) var progressLabel = "Idle"
    @Published private(set) var isEvaluating = false
    @Published private(set) var galleryProxies: [String: BridgeProxyReference] = [:]
    @Published private(set) var layoutPreview: BridgeProxyReference?
    @Published var presentedError: String?

    private var application: PhotaraApplication?
    private var project: PhotaraProject?
    private var evaluation: EvaluationHandle?
    private var observer: AppEvaluationObserver?
    private let defaults: UserDefaults

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
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
            let project: PhotaraProject
            if let projectID = defaults.string(forKey: "photara.last-project-id"),
               let reopened = try? application.openProject(projectId: projectID)
            {
                project = reopened
            } else {
                project = try application.createProject(title: "Untitled Project")
            }
            self.application = application
            self.project = project
            snapshot = try project.snapshot()
            defaults.set(snapshot?.projectId, forKey: "photara.last-project-id")
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
            layoutPreview = nil
            defaults.set(snapshot?.projectId, forKey: "photara.last-project-id")
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
        layoutPreview = nil
    }

    func reopenLastProject() {
        guard let application,
              let projectID = defaults.string(forKey: "photara.last-project-id")
        else { return }
        do {
            let project = try application.openProject(projectId: projectID)
            self.project = project
            snapshot = try project.snapshot()
            galleryProxies.removeAll()
            layoutPreview = nil
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

    func bind(assetID: String, to node: BridgeNodeDto) {
        guard let project, let snapshot,
              let frame = node.layout?.frames.first,
              let cell = frame.cells.first
        else { return }
        accept(
            project.bindAssetToLayout(
                expectedGraphRevision: snapshot.graph.revision,
                layoutNodeId: node.nodeId,
                frameId: frame.frameId,
                cellId: cell.cellId,
                assetId: assetID
            )
        )
        requestLayoutPreview(for: node.nodeId)
    }

    func requestLayoutPreview(for nodeID: String) {
        guard let project,
              let node = snapshot?.nodes.first(where: { $0.nodeId == nodeID }),
              let frame = node.layout?.frames.first,
              let cell = frame.cells.first,
              cell.assetId != nil
        else {
            layoutPreview = nil
            return
        }
        do {
            layoutPreview = try project.requestLayoutCellPreview(
                layoutNodeId: node.nodeId,
                frameId: frame.frameId,
                cellId: cell.cellId
            )
        } catch {
            presentedError = error.localizedDescription
        }
    }

    func applyCenteredCrop(to node: BridgeNodeDto) {
        guard let project, let snapshot else { return }
        guard let frame = node.layout?.frames.first,
              let cell = frame.cells.first
        else {
            presentedError = "The selected node has no inspectable Layout cell"
            return
        }
        accept(
            project.setLayoutCellCrop(
                expectedGraphRevision: snapshot.graph.revision,
                nodeId: node.nodeId,
                frameId: frame.frameId,
                cellId: cell.cellId,
                x: 100_000,
                y: 100_000,
                width: 800_000,
                height: 800_000
            )
        )
    }

    func undoLayout() {
        guard let project, let snapshot else { return }
        accept(project.undoLayout(expectedGraphRevision: snapshot.graph.revision))
    }

    func redoLayout() {
        guard let project, let snapshot else { return }
        accept(project.redoLayout(expectedGraphRevision: snapshot.graph.revision))
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
        } else {
            presentedError = response.error?.message ?? "Semantic command was rejected"
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
