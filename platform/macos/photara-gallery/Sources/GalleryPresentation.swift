import AppKit

/// Immutable client presentation. Proxy ownership, requests and opening files stay in the host.
struct GalleryAsset {
    var assetId: String
    var displayName: String
    var formatLabel: String?
    var representationCount: UInt64 = 1
    var visualRevision: String?
    var isCurrent = false
    var isStale = false
    var isProxyAvailable = true
    var preview: ImagePreviewMetadata?
    var proxyImage: NSImage?
    var nativeThumbnail: NSImage?
    var activity: GalleryPreviewActivity?
    var previewError: String?

    var canViewFull: Bool { isProxyAvailable && preview != nil && proxyImage != nil }
    var aspectRatio: CGFloat {
        if let preview, preview.pixelHeight > 0 {
            return CGFloat(preview.pixelWidth) / CGFloat(preview.pixelHeight)
        }
        if let image = nativeThumbnail, image.size.height > 0 {
            return image.size.width / image.size.height
        }
        return 1
    }
}

enum GalleryPreviewActivity: Equatable, Sendable {
    case loading, updating, ready, failed
}

struct GalleryPresentation {
    var assets: [GalleryAsset]
    var canAssign: Bool
    func asset(_ id: String) -> GalleryAsset? { assets.first { $0.assetId == id } }
}

struct GalleryActions {
    var open: (String) -> Void
    var assign: (String) -> Void
    var requestPreview: (String) -> Void
}
