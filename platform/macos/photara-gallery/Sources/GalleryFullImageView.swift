import AppKit
import SwiftUI

struct GalleryFullImageView: View {
    @Environment(\.dismiss) private var dismiss
    let asset: GalleryAsset
    let descriptor: ImagePreviewMetadata
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
                PhotaraHDRImageView(
                    image: image,
                    sizingMode: .fit,
                    preferredDynamicRange: .high
                )
                    .frame(width: geometry.size.width, height: geometry.size.height)
            }
            .background(Color(nsColor: .controlBackgroundColor))
        }
        .frame(minWidth: 720, minHeight: 520)
    }
}
