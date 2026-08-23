import AppKit
import SwiftUI

struct AssetGalleryView: View {
    @EnvironmentObject private var app: AppModel
    @EnvironmentObject private var workspace: WorkspaceModel
    @State private var viewStyle: GalleryViewStyle = .photoGrid
    @State private var fullImageAssetID: String?
    @State private var thumbnailSize: Double = 112

    private var assets: [BridgeAssetDto] {
        let all = app.snapshot?.assets ?? []
        let query = workspace.galleryFilter.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else { return all }
        return all.filter { $0.displayName.localizedCaseInsensitiveContains(query) }
    }

    private var selectedLayout: BridgeNodeDto? {
        let layouts = app.snapshot?.nodes.filter { $0.layout != nil } ?? []
        if let selected = workspace.selectedNodeID {
            return layouts.first { $0.nodeId == selected }
        }
        return layouts.first
    }

    private var currentPreviewCount: Int {
        assets.count { asset in
            guard let revision = asset.visualRevision else { return false }
            return app.galleryDisplayedRevisions[asset.assetId] == revision
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            VStack(spacing: 8) {
                TextField("Filter assets…", text: $workspace.galleryFilter)
                    .textFieldStyle(.roundedBorder)
                HStack {
                    Text("\(assets.count) asset\(assets.count == 1 ? "" : "s")")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    if !assets.isEmpty, currentPreviewCount < assets.count {
                        ProgressView(
                            value: Double(currentPreviewCount),
                            total: Double(assets.count)
                        )
                        .progressViewStyle(.linear)
                        .frame(maxWidth: 88)
                        .help("Showing the best available previews while fresher ones load")
                    }
                    Spacer()
                    Picker("View", selection: $viewStyle) {
                        Image(systemName: "rectangle.grid.1x2")
                            .tag(GalleryViewStyle.photoGrid)
                            .help("Photo Grid")
                        Image(systemName: "square.grid.2x2")
                            .tag(GalleryViewStyle.squareGrid)
                            .help("Square Grid")
                    }
                    .pickerStyle(.segmented)
                    .labelsHidden()
                    .frame(width: 72)
                }
            }
            .padding(10)
            Divider()
            if !assets.isEmpty {
                ScrollView {
                    if viewStyle == .photoGrid {
                        PhotoGridLayout(
                            spacing: 2,
                            targetRowHeight: CGFloat(thumbnailSize)
                        ) {
                            ForEach(assets, id: \.assetId) { asset in
                                card(for: asset)
                                    .layoutValue(
                                        key: GalleryAspectRatioKey.self,
                                        value: aspectRatio(for: asset)
                                    )
                            }
                        }
                        .padding(4)
                    } else {
                        LazyVGrid(columns: squareColumns, spacing: 10) {
                            ForEach(assets, id: \.assetId) { asset in
                                card(for: asset)
                            }
                        }
                        .padding(10)
                    }
                }
                Divider()
                HStack {
                    Text(workspace.selectedAssetID == nil ? "Select an asset" : "Asset selected")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Spacer()
                    Image(systemName: "photo")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                    Slider(value: $thumbnailSize, in: 76...220)
                        .frame(width: 92)
                        .help("Thumbnail size")
                    Image(systemName: "photo.fill")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Button("View", systemImage: "arrow.up.left.and.arrow.down.right") {
                        fullImageAssetID = workspace.selectedAssetID
                    }
                    .controlSize(.small)
                    .disabled(!selectedProxyIsReady)
                    Button("Assign to Cell", systemImage: "arrow.left.circle") {
                        guard let assetID = workspace.selectedAssetID else { return }
                        assign(assetID)
                    }
                    .buttonStyle(.borderedProminent)
                    .controlSize(.small)
                    .disabled(workspace.selectedAssetID == nil || selectedLayout == nil)
                }
                .padding(8)
            } else {
                ContentUnavailableView(
                    workspace.galleryFilter.isEmpty ? "No Assets" : "No Matching Assets",
                    systemImage: "photo.on.rectangle.angled",
                    description: Text("Gallery reflects project Asset Context only.")
                )
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .onChange(of: assets.map(\.assetId)) {
            if let selected = workspace.selectedAssetID,
               !assets.contains(where: { $0.assetId == selected })
            {
                workspace.selectedAssetID = nil
            }
        }
        .sheet(
            isPresented: Binding(
                get: {
                    guard let assetID = fullImageAssetID else { return false }
                    return app.galleryProxies[assetID] != nil
                        && app.galleryProxyDescriptors[assetID] != nil
                        && app.galleryProxyImages[assetID] != nil
                },
                set: { if !$0 { fullImageAssetID = nil } }
            )
        ) {
            if let assetID = fullImageAssetID,
               let asset = assets.first(where: { $0.assetId == assetID }),
               let descriptor = app.galleryProxyDescriptors[assetID],
               let image = app.galleryProxyImages[assetID]
            {
                GalleryFullImageView(asset: asset, descriptor: descriptor, image: image)
            }
        }
    }

    private var squareColumns: [GridItem] {
        let size = CGFloat(thumbnailSize)
        return [GridItem(.adaptive(minimum: size, maximum: size * 1.18), spacing: 8)]
    }

    private var selectedProxyIsReady: Bool {
        guard let assetID = workspace.selectedAssetID else { return false }
        return app.galleryProxies[assetID] != nil
            && app.galleryProxyDescriptors[assetID] != nil
            && app.galleryProxyImages[assetID] != nil
    }

    private func aspectRatio(for asset: BridgeAssetDto) -> CGFloat {
        if let descriptor = app.galleryProxyDescriptors[asset.assetId],
           descriptor.pixelHeight > 0
        {
            return CGFloat(descriptor.pixelWidth) / CGFloat(descriptor.pixelHeight)
        }
        if let image = app.galleryNativeThumbnails[asset.assetId], image.size.height > 0 {
            return image.size.width / image.size.height
        }
        return 1
    }

    private func card(for asset: BridgeAssetDto) -> some View {
        AssetCard(
            asset: asset,
            reference: app.galleryProxies[asset.assetId],
            proxyImage: app.galleryProxyImages[asset.assetId],
            nativeThumbnail: app.galleryNativeThumbnails[asset.assetId],
            activity: app.galleryPreviewActivities[asset.assetId],
            previewError: app.galleryPreviewErrors[asset.assetId],
            isStale: asset.visualRevision.map {
                app.galleryDisplayedRevisions[asset.assetId] != $0
            } ?? false,
            aspectRatio: aspectRatio(for: asset),
            style: viewStyle,
            selected: workspace.selectedAssetID == asset.assetId
        ) {
            workspace.selectedAssetID = asset.assetId
        } open: {
            app.openGalleryAsset(assetID: asset.assetId)
        } viewFull: {
            guard app.galleryProxies[asset.assetId] != nil,
                  app.galleryProxyDescriptors[asset.assetId] != nil,
                  app.galleryProxyImages[asset.assetId] != nil
            else { return }
            fullImageAssetID = asset.assetId
        } assign: {
            assign(asset.assetId)
        }
        .task(id: asset.visualRevision) {
            app.requestGalleryThumbnail(assetID: asset.assetId)
        }
    }

    private func assign(_ assetID: String) {
        guard let selectedLayout,
              let frame = selectedFrame(in: selectedLayout),
              let cell = selectedCell(in: frame)
        else { return }
        app.bind(
            assetID: assetID,
            to: selectedLayout,
            frameID: frame.frameId,
            cellID: cell.cellId
        )
    }

    private func selectedFrame(in node: BridgeNodeDto) -> BridgeLayoutFrameInspectionDto? {
        let frames = node.layout?.frames ?? []
        return frames.first { $0.frameId == workspace.selectedFrameID } ?? frames.first
    }

    private func selectedCell(
        in frame: BridgeLayoutFrameInspectionDto
    ) -> BridgeLayoutCellInspectionDto? {
        frame.cells.first { $0.cellId == workspace.selectedCellID } ?? frame.cells.first
    }
}

private enum GalleryViewStyle: Hashable {
    case photoGrid
    case squareGrid
}

private struct AssetCard: View {
    let asset: BridgeAssetDto
    let reference: BridgeProxyReference?
    let proxyImage: NSImage?
    let nativeThumbnail: NSImage?
    let activity: GalleryPreviewActivity?
    let previewError: String?
    let isStale: Bool
    let aspectRatio: CGFloat
    let style: GalleryViewStyle
    let selected: Bool
    let select: () -> Void
    let open: () -> Void
    let viewFull: () -> Void
    let assign: () -> Void

    var body: some View {
        Button(action: select) {
            VStack(alignment: .leading, spacing: 5) {
                ZStack(alignment: .topLeading) {
                    GalleryThumbnail(
                        proxyImage: proxyImage,
                        nativeThumbnail: nativeThumbnail,
                        aspectRatio: style == .photoGrid ? aspectRatio : 1,
                        fillsFrame: style == .photoGrid,
                        cornerRadius: style == .photoGrid ? 2 : 5
                    )
                    if asset.representationCount > 1 {
                        Text("\(asset.representationCount) reps")
                            .font(.system(size: 8, weight: .semibold))
                            .padding(.horizontal, 5)
                            .padding(.vertical, 3)
                            .background(.ultraThinMaterial, in: Capsule())
                            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomLeading)
                            .padding(5)
                    }
                    PreviewActivityBadge(
                        activity: activity,
                        isStale: isStale,
                        errorMessage: previewError
                    )
                        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topTrailing)
                        .padding(5)
                }
                if style == .squareGrid {
                    HStack(spacing: 5) {
                        Text(asset.displayName)
                            .font(.caption2)
                            .lineLimit(1)
                            .truncationMode(.middle)
                        Spacer(minLength: 2)
                        if let format = asset.formatLabel {
                            Text(format)
                                .font(.system(size: 8, weight: .semibold))
                                .foregroundStyle(.secondary)
                                .padding(.horizontal, 4)
                                .padding(.vertical, 2)
                                .background(
                                    Color.primary.opacity(0.08),
                                    in: RoundedRectangle(cornerRadius: 2)
                                )
                        }
                    }
                }
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .padding(style == .photoGrid ? 0 : 4)
        .background(
            selected ? Color.accentColor.opacity(0.18) : Color.clear,
            in: RoundedRectangle(cornerRadius: style == .photoGrid ? 2 : 7)
        )
        .overlay {
            RoundedRectangle(cornerRadius: style == .photoGrid ? 2 : 7)
                .stroke(selected ? Color.accentColor : .clear, lineWidth: 1.5)
        }
        .simultaneousGesture(TapGesture(count: 2).onEnded(open))
        .contextMenu {
            Button("View Full Image", action: viewFull)
                .disabled(reference == nil || proxyImage == nil)
            Button("Open in Default Application", action: open)
            Button("Assign to Selected Cell", action: assign)
        }
    }
}

private struct PreviewActivityBadge: View {
    let activity: GalleryPreviewActivity?
    let isStale: Bool
    let errorMessage: String?

    var body: some View {
        if activity == .loading {
            ProgressView()
                .controlSize(.mini)
                .padding(5)
                .background(.ultraThinMaterial, in: Circle())
                .help("Loading preview")
        } else if activity == .updating || isStale {
            Image(systemName: "arrow.triangle.2.circlepath")
                .font(.system(size: 10, weight: .semibold))
                .padding(5)
                .background(.ultraThinMaterial, in: Circle())
                .help("Showing an older preview while the current preview loads")
        } else if activity == .failed {
            Image(systemName: "exclamationmark.triangle.fill")
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(.yellow)
                .padding(5)
                .background(.ultraThinMaterial, in: Circle())
                .help(errorMessage ?? "Preview unavailable")
        }
    }
}

private struct GalleryThumbnail: View {
    let proxyImage: NSImage?
    let nativeThumbnail: NSImage?
    let aspectRatio: CGFloat
    let fillsFrame: Bool
    let cornerRadius: CGFloat

    var body: some View {
        GeometryReader { geometry in
            previewContent
                .allowedDynamicRange(.constrainedHigh)
                .frame(
                    width: geometry.size.width,
                    height: geometry.size.height,
                    alignment: .center
                )
                .clipped()
        }
        .aspectRatio(aspectRatio, contentMode: .fit)
        .clipped()
        .background(.quaternary, in: RoundedRectangle(cornerRadius: cornerRadius))
        .clipShape(RoundedRectangle(cornerRadius: cornerRadius))
    }

    @ViewBuilder
    private var previewContent: some View {
        if let proxyImage {
            renderedImage(proxyImage)
        } else if let nativeThumbnail {
            renderedImage(nativeThumbnail)
        } else {
            Image(systemName: "photo.on.rectangle")
                .font(.title2)
                .foregroundStyle(.secondary)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }

    @ViewBuilder
    private func renderedImage(_ image: NSImage) -> some View {
        if fillsFrame {
            Image(nsImage: image)
                .resizable()
                .scaledToFill()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .clipped()
        } else {
            Image(nsImage: image)
                .resizable()
                .scaledToFit()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .clipped()
        }
    }
}

private struct GalleryAspectRatioKey: LayoutValueKey {
    static let defaultValue: CGFloat = 1
}

/// A compact justified photo layout. Unknown items begin square; when a native
/// or project proxy supplies dimensions, the affected rows adopt their actual
/// aspect ratios without ever drawing outside their assigned frames.
private struct PhotoGridLayout: Layout {
    let spacing: CGFloat
    let targetRowHeight: CGFloat

    func sizeThatFits(
        proposal: ProposedViewSize,
        subviews: Subviews,
        cache: inout ()
    ) -> CGSize {
        let width = max(proposal.width ?? 480, 1)
        let result = layout(width: width, subviews: subviews)
        return CGSize(width: width, height: result.height)
    }

    func placeSubviews(
        in bounds: CGRect,
        proposal: ProposedViewSize,
        subviews: Subviews,
        cache: inout ()
    ) {
        let result = layout(width: max(bounds.width, 1), subviews: subviews)
        for (index, frame) in result.frames.enumerated() {
            subviews[index].place(
                at: CGPoint(x: bounds.minX + frame.minX, y: bounds.minY + frame.minY),
                anchor: .topLeading,
                proposal: ProposedViewSize(width: frame.width, height: frame.height)
            )
        }
    }

    private func layout(width: CGFloat, subviews: Subviews) -> (frames: [CGRect], height: CGFloat) {
        guard !subviews.isEmpty else { return ([], 0) }
        var rows: [[Int]] = []
        var row: [Int] = []
        var ratioSum: CGFloat = 0

        for index in subviews.indices {
            let ratio = normalized(subviews[index][GalleryAspectRatioKey.self])
            row.append(index)
            ratioSum += ratio
            let occupied = ratioSum * targetRowHeight
                + spacing * CGFloat(max(row.count - 1, 0))
            if occupied >= width {
                rows.append(row)
                row = []
                ratioSum = 0
            }
        }
        if !row.isEmpty { rows.append(row) }

        var frames = Array(repeating: CGRect.zero, count: subviews.count)
        var y: CGFloat = 0
        for (rowIndex, indices) in rows.enumerated() {
            let sum = indices.reduce(CGFloat.zero) {
                $0 + normalized(subviews[$1][GalleryAspectRatioKey.self])
            }
            let available = max(width - spacing * CGFloat(max(indices.count - 1, 0)), 1)
            let isLastIncompleteRow = rowIndex == rows.count - 1
                && sum * targetRowHeight < available
            let height = isLastIncompleteRow ? targetRowHeight : available / max(sum, 0.01)
            var x: CGFloat = 0
            for index in indices {
                let itemWidth = height * normalized(subviews[index][GalleryAspectRatioKey.self])
                frames[index] = CGRect(x: x, y: y, width: itemWidth, height: height)
                x += itemWidth + spacing
            }
            y += height + spacing
        }
        return (frames, max(y - spacing, 0))
    }

    private func normalized(_ ratio: CGFloat) -> CGFloat {
        guard ratio.isFinite else { return 1 }
        return min(max(ratio, 0.35), 4)
    }
}

private struct GalleryFullImageView: View {
    @Environment(\.dismiss) private var dismiss
    let asset: BridgeAssetDto
    let descriptor: BridgeProxyDescriptorDto
    let image: NSImage

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 10) {
                Text(asset.displayName)
                    .font(.headline)
                    .lineLimit(1)
                if let format = asset.formatLabel {
                    Text(format)
                        .font(.caption2.weight(.semibold))
                        .padding(.horizontal, 6)
                        .padding(.vertical, 3)
                        .background(.quaternary, in: Capsule())
                }
                Spacer()
                Text("\(descriptor.pixelWidth) × \(descriptor.pixelHeight)")
                    .font(.caption.monospacedDigit())
                    .foregroundStyle(.secondary)
                Button("Close", action: dismiss.callAsFunction)
                    .keyboardShortcut(.cancelAction)
            }
            .padding(12)
            Divider()
            GeometryReader { geometry in
                Image(nsImage: image)
                    .resizable()
                    .scaledToFit()
                    .allowedDynamicRange(.constrainedHigh)
                    .frame(width: geometry.size.width, height: geometry.size.height)
            }
            .background(Color(nsColor: .controlBackgroundColor))
        }
        .frame(minWidth: 720, minHeight: 520)
    }
}
