import AppKit
import SwiftUI

struct AssetGalleryView: View {
    let presentation: GalleryPresentation
    let actions: GalleryActions
    let preset: GalleryPreset
    @Binding var filter: String
    @Binding var selectedAssetID: String?
    @Environment(\.photaraTheme) private var theme
    @State private var viewStyle: GalleryViewStyle = .photoGrid
    @State private var fullImageAssetID: String?
    @State private var thumbnailSize: Double

    init(presentation: GalleryPresentation, actions: GalleryActions, preset: GalleryPreset = .shipped,
         initialStyle: GalleryViewStyle = .photoGrid, filter: Binding<String>, selectedAssetID: Binding<String?>) {
        self.presentation = presentation; self.actions = actions; self.preset = preset
        _viewStyle = State(initialValue: initialStyle)
        _filter = filter; _selectedAssetID = selectedAssetID
        _thumbnailSize = State(initialValue: preset.defaultThumbnailSize)
    }

    private var assets: [GalleryAsset] {
        let all = presentation.assets
        let query = filter.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else { return all }
        return all.filter { $0.displayName.localizedCaseInsensitiveContains(query) }
    }

    private var currentPreviewCount: Int { assets.count { $0.isCurrent } }

    var body: some View {
        VStack(spacing: 0) {
            VStack(spacing: 8) {
                TextField("Filter assets…", text: $filter)
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
                            spacing: preset.photoSpacing,
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
                        LazyVGrid(columns: squareColumns, spacing: preset.squareRowSpacing) {
                            ForEach(assets, id: \.assetId) { asset in
                                card(for: asset)
                            }
                        }
                        .padding(10)
                    }
                }
                Divider()
                HStack {
                    Text(selectedAssetID == nil ? "Select an asset" : "Asset selected")
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
                        fullImageAssetID = selectedAssetID
                    }
                    .controlSize(.small)
                    .disabled(!selectedProxyIsReady)
                    Button("Assign to Cell", systemImage: "arrow.left.circle") {
                        guard let assetID = selectedAssetID else { return }
                        actions.assign(assetID)
                    }
                    .buttonStyle(.borderedProminent)
                    .controlSize(.small)
                    .disabled(selectedAssetID == nil || !presentation.canAssign)
                }
                .padding(8)
            } else {
                emptyState
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(theme?.color(.galleryBackground) ?? Color(nsColor: .windowBackgroundColor))
        .onChange(of: assets.map(\.assetId)) {
            if let selected = selectedAssetID,
               !assets.contains(where: { $0.assetId == selected })
            {
                selectedAssetID = nil
            }
        }
        .onChange(of: preset.defaultThumbnailSize) { thumbnailSize = preset.defaultThumbnailSize }
        .sheet(
            isPresented: Binding(
                get: {
                    guard let assetID = fullImageAssetID else { return false }
                    return presentation.asset(assetID)?.canViewFull == true
                },
                set: { if !$0 { fullImageAssetID = nil } }
            )
        ) {
            if let assetID = fullImageAssetID,
               let asset = assets.first(where: { $0.assetId == assetID }),
               let descriptor = asset.preview,
               let image = asset.proxyImage
            {
                GalleryFullImageView(asset: asset, descriptor: descriptor, image: image)
            }
        }
    }

    @ViewBuilder
    private var emptyState: some View {
        if !filter.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            PhotaraEmptyStateView(preset: preset.noMatchesState) {
                filter = ""
                actions.clearFilter()
            }
        } else if presentation.hasSourceNodes {
            PhotaraEmptyStateView(preset: preset.awaitingAssetsState, action: actions.runWorkflow)
        } else {
            PhotaraEmptyStateView(preset: preset.noSourceState, action: actions.addSourceNode)
        }
    }

    private var squareColumns: [GridItem] {
        let size = CGFloat(thumbnailSize)
        return [GridItem(.adaptive(minimum: size, maximum: size * 1.18), spacing: 8)]
    }

    private var selectedProxyIsReady: Bool {
        selectedAssetID.flatMap { presentation.asset($0) }?.canViewFull == true
    }

    private func aspectRatio(for asset: GalleryAsset) -> CGFloat { asset.aspectRatio }

    private func card(for asset: GalleryAsset) -> some View {
        AssetCard(asset: asset, selectionStrokeWidth: preset.selectionStrokeWidth, proxyImage: asset.proxyImage,
            nativeThumbnail: asset.nativeThumbnail, activity: asset.activity,
            previewError: asset.previewError, isStale: asset.isStale,
            aspectRatio: asset.aspectRatio, style: viewStyle,
            selected: selectedAssetID == asset.assetId, canAssign: presentation.canAssign,
            select: { selectedAssetID = asset.assetId },
            open: { actions.open(asset.assetId) },
            viewFull: { if asset.canViewFull { fullImageAssetID = asset.assetId } },
            assign: { actions.assign(asset.assetId) })
            .task(id: asset.visualRevision) { actions.requestPreview(asset.assetId) }
    }
}
