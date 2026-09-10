import SwiftUI

/// Production adapter. The renderer and its local selection/filter UI are shared with Gallery Lab.
struct ProductionGalleryView: View {
    @EnvironmentObject private var app: AppModel
    @EnvironmentObject private var workspace: WorkspaceModel

    var body: some View {
        AssetGalleryView(presentation: presentation,
            actions: .init(open: { app.openGalleryAsset(assetID: $0) }, assign: assign,
                           requestPreview: { app.requestGalleryThumbnail(assetID: $0) }),
            filter: $workspace.galleryFilter, selectedAssetID: $workspace.selectedAssetID)
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
        }, canAssign: selectedLayout != nil)
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
