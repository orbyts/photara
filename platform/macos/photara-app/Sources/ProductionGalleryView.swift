import SwiftUI

/// Production adapter. The renderer and its local selection/filter UI are shared with Gallery Lab.
struct ProductionGalleryView: View {
    @EnvironmentObject private var app: AppModel
    @EnvironmentObject private var workspace: WorkspaceModel
    @State private var preset = GalleryPreset.developmentOrShipped

    var body: some View {
        AssetGalleryView(presentation: presentation,
            actions: .init(open: { app.openGalleryAsset(assetID: $0) }, assign: assign,
                           requestPreview: { app.requestGalleryThumbnail(assetID: $0) },
                           addSourceNode: addSourceNode,
                           runWorkflow: { app.performApplicationAction(.evaluate) }),
            preset: preset,
            filter: $workspace.galleryFilter, selectedAssetID: $workspace.selectedAssetID)
            .task { await reloadDevelopmentPreset() }
    }

    private var selectedLayout: BridgeNodeDto? {
        let layouts = app.snapshot?.nodes.filter { $0.layout != nil } ?? []
        if let selected = workspace.selectedNodeID { return layouts.first { $0.nodeId == selected } }
        return layouts.first
    }

    private var presentation: GalleryPresentation {
        .init(assets: (app.snapshot?.assets ?? []).map { asset in
            let id = asset.assetId
            let current = asset.visualRevision.map { app.galleryDisplayedRevisions[id] == $0 } ?? false
            return GalleryAsset(assetId: id, displayName: asset.displayName,
                formatLabel: asset.formatLabel, representationCount: UInt64(asset.representationCount),
                visualRevision: asset.visualRevision, isCurrent: current,
                isStale: asset.visualRevision != nil && !current,
                isProxyAvailable: app.galleryProxies[id] != nil,
                preview: app.galleryProxyDescriptors[id].map(ImagePreviewMetadata.init),
                proxyImage: app.galleryProxyImages[id], nativeThumbnail: app.galleryNativeThumbnails[id],
                activity: app.galleryPreviewActivities[id], previewError: app.galleryPreviewErrors[id])
        }, canAssign: selectedLayout != nil, hasSourceNodes: hasSourceNodes)
    }

    private var hasSourceNodes: Bool {
        app.snapshot?.nodes.contains { node in
            node.ports.contains { $0.direction == .output && $0.valueTypeId == "photara.asset-set" }
        } == true
    }

    private func addSourceNode() {
        workspace.activateGraph()
        workspace.requestNodeMenu()
    }

    private func reloadDevelopmentPreset() async {
        while !Task.isCancelled {
            let latest = GalleryPreset.developmentOrShipped
            if latest != preset { preset = latest }
            try? await Task.sleep(for: .milliseconds(500))
        }
    }

    private func assign(_ assetID: String) {
        guard let node = selectedLayout, let frames = node.layout?.frames,
              let frame = frames.first(where: { $0.frameId == workspace.selectedFrameID }) ?? frames.first,
              let cell = frame.cells.first(where: { $0.cellId == workspace.selectedCellID }) ?? frame.cells.first
        else { return }
        app.bind(assetID: assetID, to: node, frameID: frame.frameId, cellID: cell.cellId)
    }
}

extension ImagePreviewMetadata {
    init(_ value: BridgeProxyDescriptorDto) {
        self.init(pixelWidth: value.pixelWidth, pixelHeight: value.pixelHeight,
                  dynamicRange: value.dynamicRange == .hdr ? .hdr : .sdr, colorSpaceId: value.colorSpaceId)
    }
}
