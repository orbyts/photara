import AppKit
import SwiftUI

enum GalleryViewStyle: Hashable {
    case photoGrid
    case squareGrid
}

struct AssetCard: View {
    @Environment(\.photaraTheme) private var theme

    let asset: GalleryAsset
    let selectionStrokeWidth: Double
    let proxyImage: NSImage?
    let nativeThumbnail: NSImage?
    let activity: GalleryPreviewActivity?
    let previewError: String?
    let isStale: Bool
    let aspectRatio: CGFloat
    let style: GalleryViewStyle
    let selected: Bool
    let canAssign: Bool
    let select: () -> Void
    let open: () -> Void
    let viewFull: () -> Void
    let assign: () -> Void

    private var cardBackground: Color {
        if selected {
            return theme?.color(.selectionBackground) ?? Color.accentColor.opacity(0.18)
        }
        if style == .squareGrid {
            return theme?.color(.galleryCell) ?? .clear
        }
        return .clear
    }

    private var cardForeground: Color {
        selected
            ? (theme?.color(.selectionForeground) ?? .primary)
            : (theme?.color(.textPrimary) ?? .primary)
    }

    private var cardBorder: Color {
        selected ? (theme?.color(.borderFocus) ?? .accentColor) : .clear
    }

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
            cardBackground,
            in: RoundedRectangle(cornerRadius: style == .photoGrid ? 2 : 7)
        )
        .foregroundStyle(cardForeground)
        .overlay {
            RoundedRectangle(cornerRadius: style == .photoGrid ? 2 : 7)
                .stroke(cardBorder, lineWidth: selectionStrokeWidth)
        }
        .simultaneousGesture(TapGesture(count: 2).onEnded(open))
        .contextMenu {
            Button("View Full Image", action: viewFull)
                .disabled(!asset.canViewFull)
            Button("Open in Default Application", action: open)
            Button("Assign to Selected Cell", action: assign)
                .disabled(!canAssign)
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
    @Environment(\.photaraTheme) private var theme

    let proxyImage: NSImage?
    let nativeThumbnail: NSImage?
    let aspectRatio: CGFloat
    let fillsFrame: Bool
    let cornerRadius: CGFloat

    var body: some View {
        GeometryReader { geometry in
            previewContent
                .frame(
                    width: geometry.size.width,
                    height: geometry.size.height,
                    alignment: .center
                )
                .clipped()
        }
        .aspectRatio(aspectRatio, contentMode: .fit)
        .clipped()
        .background(
            theme?.color(.galleryCell) ?? Color(nsColor: .controlBackgroundColor),
            in: RoundedRectangle(cornerRadius: cornerRadius)
        )
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
        PhotaraHDRImageView(
            image: image,
            sizingMode: fillsFrame ? .fill : .fit,
            preferredDynamicRange: .constrainedHigh
        )
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .clipped()
    }
}
