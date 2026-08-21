import CoreGraphics
import CoreImage
import Foundation
import ImageIO

enum ProxyMode: String {
    case thumbnailSDR = "thumbnail-sdr"
    case authoringHDR = "authoring-hdr"
}

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data("\(message)\n".utf8))
    exit(1)
}

guard CommandLine.arguments.count == 7 else {
    fail("usage: apple-proxy MODE INPUT OUTPUT_ICC LONG_EDGE cpu|gpu OUTPUT")
}
guard let mode = ProxyMode(rawValue: CommandLine.arguments[1]) else {
    fail("unknown mode \(CommandLine.arguments[1])")
}
let inputURL = URL(fileURLWithPath: CommandLine.arguments[2])
let profileURL = URL(fileURLWithPath: CommandLine.arguments[3])
guard let longEdge = Double(CommandLine.arguments[4]), longEdge > 0 else {
    fail("LONG_EDGE must be positive")
}
guard CommandLine.arguments[5] == "cpu" || CommandLine.arguments[5] == "gpu" else {
    fail("renderer must be cpu or gpu")
}
let useSoftwareRenderer = CommandLine.arguments[5] == "cpu"
let outputURL = URL(fileURLWithPath: CommandLine.arguments[6])
guard let profileData = try? Data(contentsOf: profileURL),
      let outputColorSpace = CGColorSpace(iccData: profileData as CFData) else {
    fail("could not load output ICC profile")
}
var image: CIImage
switch mode {
case .thumbnailSDR:
    guard let source = CGImageSourceCreateWithURL(inputURL as CFURL, [
        kCGImageSourceShouldCache: false
    ] as CFDictionary), let sourceImage = CGImageSourceCreateThumbnailAtIndex(source, 0, [
        kCGImageSourceCreateThumbnailFromImageAlways: true,
        kCGImageSourceCreateThumbnailWithTransform: true,
        kCGImageSourceThumbnailMaxPixelSize: Int(longEdge),
        kCGImageSourceShouldCacheImmediately: true
    ] as CFDictionary) else {
        fail("could not decode input thumbnail")
    }
    image = CIImage(cgImage: sourceImage)
case .authoringHDR:
    guard let source = CGImageSourceCreateWithURL(inputURL as CFURL, [
        kCGImageSourceShouldCache: false,
        kCGImageSourceShouldAllowFloat: true
    ] as CFDictionary), let sourceImage = CGImageSourceCreateThumbnailAtIndex(source, 0, [
        kCGImageSourceCreateThumbnailFromImageAlways: true,
        kCGImageSourceCreateThumbnailWithTransform: true,
        kCGImageSourceThumbnailMaxPixelSize: Int(longEdge),
        kCGImageSourceShouldAllowFloat: true,
        kCGImageSourceShouldCacheImmediately: true
    ] as CFDictionary) else {
        fail("could not decode floating-point input thumbnail")
    }
    image = CIImage(cgImage: sourceImage)
}

let sourceLongEdge = max(image.extent.width, image.extent.height)
let scale = min(1.0, longEdge / sourceLongEdge)
if scale < 1.0 {
    image = image.applyingFilter(
        "CILanczosScaleTransform",
        parameters: [
            kCIInputScaleKey: scale,
            kCIInputAspectRatioKey: 1.0
        ]
    )
}

let context = CIContext(options: [
    .cacheIntermediates: false,
    .useSoftwareRenderer: useSoftwareRenderer
])

do {
    switch mode {
    case .thumbnailSDR:
        try context.writePNGRepresentation(
            of: image,
            to: outputURL,
            format: .RGBA8,
            colorSpace: outputColorSpace,
            options: [:]
        )
    case .authoringHDR:
        try context.writeTIFFRepresentation(
            of: image,
            to: outputURL,
            format: .RGBAh,
            colorSpace: outputColorSpace,
            options: [:]
        )
    }
} catch {
    fail("proxy write failed: \(error)")
}
