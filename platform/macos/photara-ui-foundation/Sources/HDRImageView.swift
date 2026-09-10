import AppKit
import SwiftUI

/// AppKit's documented HDR presentation path for macOS image content.
///
/// The containing view owns sizing so the `NSImageView` always receives an
/// aspect-correct frame, including the crop required by fill-mode thumbnails.
struct PhotaraHDRImageView: NSViewRepresentable {
    enum SizingMode {
        case fit
        case fill
    }

    let image: NSImage
    let sizingMode: SizingMode
    let preferredDynamicRange: NSImage.DynamicRange

    func makeNSView(context: Context) -> HDRImageContainerView {
        HDRImageContainerView()
    }

    func updateNSView(_ view: HDRImageContainerView, context: Context) {
        view.update(
            image: image,
            sizingMode: sizingMode,
            preferredDynamicRange: preferredDynamicRange
        )
    }
}

final class HDRImageContainerView: NSView {
    private let imageView = NSImageView()
    private var sizingMode: PhotaraHDRImageView.SizingMode = .fit

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        wantsLayer = true
        layer?.masksToBounds = true
        imageView.imageAlignment = .alignCenter
        imageView.imageFrameStyle = .none
        imageView.imageScaling = .scaleProportionallyUpOrDown
        addSubview(imageView)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func update(
        image: NSImage,
        sizingMode: PhotaraHDRImageView.SizingMode,
        preferredDynamicRange: NSImage.DynamicRange
    ) {
        imageView.image = image
        imageView.preferredImageDynamicRange = preferredDynamicRange
        self.sizingMode = sizingMode
        needsLayout = true
    }

    override func layout() {
        super.layout()
        guard let image = imageView.image,
              image.size.width > 0,
              image.size.height > 0,
              bounds.width > 0,
              bounds.height > 0
        else {
            imageView.frame = bounds
            return
        }

        let widthScale = bounds.width / image.size.width
        let heightScale = bounds.height / image.size.height
        let scale = switch sizingMode {
        case .fit: min(widthScale, heightScale)
        case .fill: max(widthScale, heightScale)
        }
        let size = NSSize(
            width: image.size.width * scale,
            height: image.size.height * scale
        )
        imageView.frame = NSRect(
            x: bounds.midX - size.width / 2,
            y: bounds.midY - size.height / 2,
            width: size.width,
            height: size.height
        )
    }
}
