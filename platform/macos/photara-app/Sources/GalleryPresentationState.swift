import AppKit

enum GalleryPreviewActivity: Equatable, Sendable {
    case loading
    case updating
    case ready
    case failed
}

/// Client-only Gallery presentation and request state.
///
/// None of these values are authoritative project semantics or graph inputs.
struct GalleryPresentationState {
    var proxies: [String: BridgeProxyReference] = [:]
    var proxyDescriptors: [String: BridgeProxyDescriptorDto] = [:]
    var proxyImages: [String: NSImage] = [:]
    var nativeThumbnails: [String: NSImage] = [:]
    var displayedRevisions: [String: String] = [:]
    var activities: [String: GalleryPreviewActivity] = [:]
    var errors: [String: String] = [:]
    var pendingProxyRevisions: [String: String] = [:]
    var pendingNativeRevisions: [String: String] = [:]

    mutating func reset() {
        proxies.removeAll()
        proxyDescriptors.removeAll()
        proxyImages.removeAll()
        nativeThumbnails.removeAll()
        displayedRevisions.removeAll()
        activities.removeAll()
        errors.removeAll()
        pendingProxyRevisions.removeAll()
        pendingNativeRevisions.removeAll()
    }
}
