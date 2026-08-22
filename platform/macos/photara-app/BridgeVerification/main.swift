import Foundation
import CoreGraphics
import CoreImage

private enum VerificationFailure: Error, CustomStringConvertible {
    case failed(String)

    var description: String {
        switch self {
        case let .failed(message): message
        }
    }
}

private func require(_ condition: @autoclosure () -> Bool, _ message: String) throws {
    guard condition() else {
        throw VerificationFailure.failed(message)
    }
}

private final class RecordingObserver: EvaluationObserver, @unchecked Sendable {
    private let condition = NSCondition()
    private var recordedProgress: [BridgeEvaluationProgressDto] = []
    private var recordedResult: BridgeEvaluationFinishedDto?

    func onProgress(progress: BridgeEvaluationProgressDto) {
        condition.lock()
        recordedProgress.append(progress)
        condition.unlock()
    }

    func onFinished(result: BridgeEvaluationFinishedDto) {
        condition.lock()
        recordedResult = result
        condition.broadcast()
        condition.unlock()
    }

    func wait() throws -> (BridgeEvaluationFinishedDto, [BridgeEvaluationProgressDto]) {
        let deadline = Date().addingTimeInterval(10)
        condition.lock()
        defer { condition.unlock() }
        while recordedResult == nil {
            guard condition.wait(until: deadline) else {
                throw VerificationFailure.failed("evaluation callback timed out")
            }
        }
        return (recordedResult!, recordedProgress)
    }
}

private func layoutIdentities(from node: BridgeNodeDto) throws -> (String, String) {
    guard let layout = node.layout,
          let frame = layout.frames.first,
          let cell = frame.cells.first
    else {
        throw VerificationFailure.failed("could not inspect typed Layout DTO")
    }
    return (frame.frameId, cell.cellId)
}

private func writeTIFF(to url: URL, red: CGFloat) throws {
    guard let colorSpace = CGColorSpace(name: CGColorSpace.sRGB) else {
        throw VerificationFailure.failed("could not create sRGB color space")
    }
    let image = CIImage(color: CIColor(red: red, green: 0.35, blue: 0.7, alpha: 1))
        .cropped(to: CGRect(x: 0, y: 0, width: 640, height: 480))
    try CIContext(options: [.cacheIntermediates: false]).writeTIFFRepresentation(
        of: image,
        to: url,
        format: .RGBA8,
        colorSpace: colorSpace,
        options: [:]
    )
}

@main
private enum PhotaraBridgeVerification {
    @MainActor
    static func main() throws {
        let info = applicationInfo()
        try require(info.apiVersion == 1, "unexpected facade API version")
        try require(info.productCodename == "Photara", "unexpected product codename")

        let storeRoot = FileManager.default.temporaryDirectory
            .appending(path: "photara-uniffi-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: storeRoot) }
        guard CommandLine.arguments.count == 2 else {
            throw VerificationFailure.failed("proxy helper path argument is missing")
        }
        let proxyHelper = CommandLine.arguments[1]

        let app = try PhotaraApplication.open(
            storeRoot: storeRoot.appending(path: "store").path,
            proxyCacheRoot: storeRoot.appending(path: "proxy-cache").path,
            proxyHelperExecutable: proxyHelper
        )
        let project = try app.createProject(title: "Quasar UniFFI verification")
        let initial = try project.snapshot()
        try require(initial.nodes.isEmpty, "new project should be empty")
        try require(!initial.dirty, "new durable project should be clean")

        let completionObserver = RecordingObserver()
        let emptyEvaluation = try project.prepareEvaluation()
        try emptyEvaluation.start(observer: completionObserver)
        let (completed, completionProgress) = try completionObserver.wait()
        try require(completed.status == .completed, "empty graph evaluation did not complete")
        try require(
            completionProgress.contains { $0.phase == .validating },
            "evaluation did not stream validating progress"
        )
        try require(
            completionProgress.contains { $0.phase == .completed },
            "evaluation did not stream completed progress"
        )

        let added = project.addLayoutNode(
            expectedGraphRevision: initial.graph.revision,
            canvas: .portrait3x4(longEdgePixels: 4000)
        )
        try require(added.applied, "Layout Core command was rejected")
        let withLayout = try requireSnapshot(added)
        try require(withLayout.nodes.count == 2, "Layout slice did not include AssetSet source")
        guard let layoutNode = withLayout.nodes.first(where: { $0.layout != nil }) else {
            throw VerificationFailure.failed("typed node inspection did not return Layout")
        }
        try require(layoutNode.displayName == "Layout", "wrong inspected node")
        try require(layoutNode.diagnostics.isEmpty, "valid Layout reported diagnostics")
        try require(
            layoutNode.layout?.canvas.widthPixels == 3000,
            "Rust did not resolve the typed Layout canvas"
        )

        let stale = project.addLayoutNode(
            expectedGraphRevision: initial.graph.revision,
            canvas: .vertical9x16(longEdgePixels: 3840)
        )
        try require(!stale.applied, "stale Core command unexpectedly applied")
        try require(stale.error?.code == "revision-conflict", "missing structured revision error")

        let node = layoutNode
        let sources = storeRoot.appending(path: "fixture-sources")
        try FileManager.default.createDirectory(at: sources, withIntermediateDirectories: true)
        let hdrSource = sources.appending(path: "sample-hdr.tiff")
        let sdrSource = sources.appending(path: "sample-sdr.tiff")
        try writeTIFF(to: hdrSource, red: 0.8)
        try writeTIFF(to: sdrSource, red: 0.6)
        let imported = try project.importLocalTiffPair(
            displayName: "Verification photograph",
            hdrSourcePath: hdrSource.path,
            sdrSourcePath: sdrSource.path
        )
        try require(imported.snapshot.assets.count == 1, "Asset Context import was not visible")
        try require(
            imported.snapshot.graph.digest == withLayout.graph.digest,
            "asset import unexpectedly changed the graph"
        )

        let workspaceDefaults = UserDefaults(suiteName: "photara-verification-\(UUID().uuidString)")!
        let workspace = WorkspaceModel(defaults: workspaceDefaults)
        workspace.selectedNodeID = node.nodeId
        workspace.selectedAssetID = imported.assetId
        workspace.galleryFilter = "Verification"
        let semanticDigestBeforeWorkspaceChange = imported.snapshot.graph.digest
        workspace.move(.inspector, to: .leading)
        workspace.toggle(.assetGallery)
        try require(
            workspace.visiblePanels(in: .leading).contains(.inspector),
            "Inspector identity was coupled to its original placement"
        )
        try require(workspace.selectedNodeID == node.nodeId, "moving Inspector lost node selection")
        try require(workspace.selectedAssetID == imported.assetId, "Gallery selection was lost")
        let semanticDigestAfterWorkspaceChange = try project.snapshot().graph.digest
        try require(
            semanticDigestAfterWorkspaceChange == semanticDigestBeforeWorkspaceChange,
            "workspace or Gallery state changed the semantic graph digest"
        )

        let (frameID, cellID) = try layoutIdentities(from: node)
        let bound = project.bindAssetToLayout(
            expectedGraphRevision: imported.snapshot.graph.revision,
            layoutNodeId: node.nodeId,
            frameId: frameID,
            cellId: cellID,
            assetId: imported.assetId
        )
        try require(bound.applied, "explicit AssetSet/Layout binding was rejected")
        let boundSnapshot = try requireSnapshot(bound)
        try require(
            boundSnapshot.graph.digest != imported.snapshot.graph.digest,
            "explicit Core binding did not change graph semantics"
        )
        let boundLayout = boundSnapshot.nodes.first { $0.nodeId == node.nodeId }!
        try require(
            boundLayout.layout?.frames.first?.cells.first?.assetId == imported.assetId,
            "typed Layout inspection did not expose the explicit binding"
        )

        let thumbnail = try project.requestGalleryThumbnail(assetId: imported.assetId)
        let thumbnailDescriptor = thumbnail.descriptor()
        try require(
            FileManager.default.fileExists(atPath: thumbnailDescriptor.localPath),
            "Gallery proxy reference did not point to a verified file"
        )
        try require(thumbnailDescriptor.dynamicRange == .sdr, "Gallery proxy was not SDR-safe")
        try require(
            thumbnailDescriptor.colorSpaceId == "photara.color.srgb",
            "Gallery proxy lost its color description"
        )
        let preview = try project.requestLayoutCellPreview(
            layoutNodeId: node.nodeId,
            frameId: frameID,
            cellId: cellID
        )
        let previewDescriptor = preview.descriptor()
        try require(
            FileManager.default.fileExists(atPath: previewDescriptor.localPath),
            "Layout preview reference did not point to a verified file"
        )
        try require(previewDescriptor.dynamicRange == .hdr, "Layout preview lost HDR description")

        let cropped = project.setLayoutCellCrop(
            expectedGraphRevision: boundSnapshot.graph.revision,
            nodeId: node.nodeId,
            frameId: frameID,
            cellId: cellID,
            x: 100_000,
            y: 100_000,
            width: 800_000,
            height: 800_000
        )
        try require(cropped.applied, "undoable Layout crop did not apply through Core")
        let croppedSnapshot = try requireSnapshot(cropped)
        let undone = project.undoLayout(expectedGraphRevision: croppedSnapshot.graph.revision)
        try require(undone.applied, "Layout undo did not apply through Core")

        let saved = try project.save()
        try require(!saved.dirty, "saved project remained dirty")
        let reopened = try app.openProject(projectId: saved.projectId)
        let reopenedSnapshot = try reopened.snapshot()
        try require(reopenedSnapshot == saved, "project open/save DTO round-trip changed")

        let evaluationObserver = RecordingObserver()
        let graphEvaluation = try reopened.prepareEvaluation()
        try graphEvaluation.start(observer: evaluationObserver)
        let (evaluated, _) = try evaluationObserver.wait()
        try require(evaluated.status == .completed, "connected Layout graph did not evaluate")

        let cancellationObserver = RecordingObserver()
        let cancelledEvaluation = try reopened.prepareEvaluation()
        cancelledEvaluation.cancel()
        try cancelledEvaluation.start(observer: cancellationObserver)
        let (cancelled, cancellationProgress) = try cancellationObserver.wait()
        try require(cancelled.status == .cancelled, "Swift-triggered cancellation was ignored")
        try require(
            cancellationProgress.contains { $0.phase == .cancelled },
            "cancelled progress was not delivered to Swift"
        )
        try require(cancelled.error?.diagnostic.severity == .error, "missing cancellation diagnostic")

        print("Photara UniFFI verification passed")
        print("project=\(saved.projectId) graphRevision=\(saved.graph.revision)")
        print("progressEvents=\(completionProgress.count) cancellationEvents=\(cancellationProgress.count)")
    }

    private static func requireSnapshot(
        _ response: BridgeCommandResponseDto
    ) throws -> BridgeProjectSnapshotDto {
        guard let snapshot = response.snapshot else {
            throw VerificationFailure.failed(
                response.error?.message ?? "applied command returned no snapshot"
            )
        }
        return snapshot
    }
}
