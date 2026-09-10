import AppKit
import CoreGraphics

/// Generated, deterministic test imagery; the float fixture contains linear values up to 4.0.
/// No project, filesystem source, network provider, or proxy generation is involved.
@MainActor
enum GalleryFixtures {
    static func image(width: Int, height: Int, hdr: Bool) -> NSImage {
        var pixels = [Float](repeating: 0, count: width * height * 4)
        for y in 0..<height {
            for x in 0..<width {
                let index = (y * width + x) * 4
                let u = Float(x) / Float(width), v = Float(y) / Float(height)
                let highlight: Float = x > width * 2 / 3 && y < height / 3 ? (hdr ? 4 : 1) : 0
                pixels[index] = max(highlight, u * 0.8)
                pixels[index + 1] = max(highlight, 0.2 + v * 0.6)
                pixels[index + 2] = max(highlight, 0.65 - u * 0.4)
                pixels[index + 3] = 1
            }
        }
        let data = pixels.withUnsafeBytes { Data($0) }
        let bitmap = CGBitmapInfo.floatComponents.union(.byteOrder32Little)
            .union(CGBitmapInfo(rawValue: CGImageAlphaInfo.premultipliedLast.rawValue))
        let cg = CGImage(width: width, height: height, bitsPerComponent: 32, bitsPerPixel: 128,
            bytesPerRow: width * 16, space: CGColorSpace(name: CGColorSpace.extendedLinearDisplayP3)!,
            bitmapInfo: bitmap, provider: CGDataProvider(data: data as CFData)!, decode: nil,
            shouldInterpolate: true, intent: .defaultIntent)!
        return NSImage(cgImage: cg, size: NSSize(width: width, height: height))
    }
    static func assets() -> [GalleryAsset] {
        let specs: [(String, Int, Int, Bool)] = [
            ("Landscape SDR", 480, 300, false), ("Portrait HDR", 240, 360, true),
            ("Square SDR", 320, 320, false), ("Landscape HDR · 3 representations", 500, 280, true),
            ("Loading", 240, 320, false), ("Updating portrait", 240, 360, false),
            ("Failed", 320, 320, false)
        ]
        return specs.enumerated().map { index, spec in
            let (name, width, height, hdr) = spec
            let ready = index < 4
            let image = Self.image(width: width, height: height, hdr: hdr)
            return .init(assetId: "asset-\(index)", displayName: name, formatLabel: hdr ? "HDR TIFF" : "TIFF",
                representationCount: index == 3 ? 3 : 1, visualRevision: "fixture-v1",
                isCurrent: ready, isStale: index == 5,
                preview: ready ? .init(pixelWidth: UInt32(width), pixelHeight: UInt32(height),
                    dynamicRange: hdr ? .hdr : .sdr, colorSpaceId: "extended-linear-display-p3") : nil,
                proxyImage: ready ? image : nil, nativeThumbnail: index == 5 ? image : nil,
                activity: ready ? .ready : index == 4 ? .loading : index == 5 ? .updating : .failed,
                previewError: index == 6 ? "Deterministic preview failure" : nil)
        }
    }
}
