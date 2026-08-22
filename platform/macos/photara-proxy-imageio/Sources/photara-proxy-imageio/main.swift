import CoreGraphics
import CoreImage
import CryptoKit
import Foundation
import ImageIO

enum ProxyMode: String {
    case thumbnailSDR = "thumbnail-sdr"
    case authoringHDR = "authoring-hdr"
}

struct HelperMetadata: Encodable {
    let pixelWidth: Int
    let pixelHeight: Int
    let colorSpaceID: String
    let embeddedICCFingerprint: String?
    let headroomMillistops: UInt32?

    enum CodingKeys: String, CodingKey {
        case pixelWidth = "pixel_width"
        case pixelHeight = "pixel_height"
        case colorSpaceID = "color_space_id"
        case embeddedICCFingerprint = "embedded_icc_fingerprint"
        case headroomMillistops = "headroom_millistops"
    }
}

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data("\(message)\n".utf8))
    exit(1)
}

func loadSystemProfile(_ path: String) -> CGColorSpace {
    let url = URL(fileURLWithPath: path)
    guard let data = try? Data(contentsOf: url),
          let colorSpace = CGColorSpace(iccData: data as CFData) else {
        fail("could not load system ICC profile at \(path)")
    }
    return colorSpace
}

guard CommandLine.arguments.count == 6 else {
    fail("usage: photara-proxy-imageio MODE INPUT OUTPUT LONG_EDGE METADATA_JSON")
}
guard let mode = ProxyMode(rawValue: CommandLine.arguments[1]) else {
    fail("unknown mode \(CommandLine.arguments[1])")
}
let inputURL = URL(fileURLWithPath: CommandLine.arguments[2])
let outputURL = URL(fileURLWithPath: CommandLine.arguments[3])
guard let longEdge = Double(CommandLine.arguments[4]), longEdge > 0 else {
    fail("LONG_EDGE must be positive")
}
let metadataURL = URL(fileURLWithPath: CommandLine.arguments[5])

var sourceOptions: [CFString: Any] = [kCGImageSourceShouldCache: false]
var thumbnailOptions: [CFString: Any] = [
    kCGImageSourceCreateThumbnailFromImageAlways: true,
    kCGImageSourceCreateThumbnailWithTransform: true,
    kCGImageSourceThumbnailMaxPixelSize: Int(longEdge),
    kCGImageSourceShouldCacheImmediately: true,
]
if mode == .authoringHDR {
    sourceOptions[kCGImageSourceShouldAllowFloat] = true
    thumbnailOptions[kCGImageSourceShouldAllowFloat] = true
}
guard let source = CGImageSourceCreateWithURL(inputURL as CFURL, sourceOptions as CFDictionary),
      let sourceImage = CGImageSourceCreateThumbnailAtIndex(
        source,
        0,
        thumbnailOptions as CFDictionary
      ) else {
    fail("could not decode input through ImageIO")
}
let sourceProperties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any]
let sourceProfileName = sourceProperties?[kCGImagePropertyProfileName] as? String ?? ""
if mode == .thumbnailSDR {
    let profileIsMeasuredSDR = sourceProfileName.localizedCaseInsensitiveContains("display p3")
        || sourceProfileName.localizedCaseInsensitiveContains("srgb")
    if sourceProperties?[kCGImagePropertyIsFloat] as? Bool == true || !profileIsMeasuredSDR {
        fail("SDR thumbnail path requires a measured sRGB or Display P3 SDR representation")
    }
}

var image = CIImage(cgImage: sourceImage)
let sourceLongEdge = max(image.extent.width, image.extent.height)
let scale = min(1.0, longEdge / sourceLongEdge)
if scale < 1.0 {
    image = image.applyingFilter(
        "CILanczosScaleTransform",
        parameters: [kCIInputScaleKey: scale, kCIInputAspectRatioKey: 1.0]
    )
}
let opaqueBackground = CIImage(color: CIColor(red: 0, green: 0, blue: 0, alpha: 1))
    .cropped(to: image.extent)
image = image.composited(over: opaqueBackground)

let outputColorSpace: CGColorSpace
let colorSpaceID: String
let embeddedICCFingerprint: String?
let sourceColorDescription: String
switch mode {
case .thumbnailSDR:
    guard let sRGB = CGColorSpace(name: CGColorSpace.sRGB) else {
        fail("could not create sRGB color space")
    }
    outputColorSpace = sRGB
    colorSpaceID = "photara.color.srgb"
    embeddedICCFingerprint = nil
    sourceColorDescription = sourceProfileName.isEmpty ? "unlabeled" : sourceProfileName
case .authoringHDR:
    guard let embedded = sourceImage.colorSpace else {
        fail("HDR source has no embedded ICC color space")
    }
    let embeddedName = sourceProfileName.isEmpty
        ? (embedded.name as String? ?? "unnamed embedded profile")
        : sourceProfileName
    sourceColorDescription = embeddedName
    if embeddedName.localizedCaseInsensitiveContains("aces") {
        outputColorSpace = loadSystemProfile(
            "/System/Library/ColorSync/Profiles/ACESCG Linear.icc"
        )
        colorSpaceID = "photara.color.acescg-linear"
        embeddedICCFingerprint = nil
    } else if embeddedName.localizedCaseInsensitiveContains("p3")
        && embeddedName.localizedCaseInsensitiveContains("pq") {
        guard let linearDisplayP3 = CGColorSpace(name: CGColorSpace.extendedLinearDisplayP3) else {
            fail("could not create extended-linear Display P3 color space")
        }
        outputColorSpace = linearDisplayP3
        colorSpaceID = "photara.color.display-p3-linear"
        embeddedICCFingerprint = nil
    } else if embeddedName.localizedCaseInsensitiveContains("display p3") {
        outputColorSpace = loadSystemProfile(
            "/System/Library/ColorSync/Profiles/Display P3.icc"
        )
        colorSpaceID = "photara.color.display-p3"
        embeddedICCFingerprint = nil
    } else if embeddedName.localizedCaseInsensitiveContains("srgb") {
        outputColorSpace = loadSystemProfile(
            "/System/Library/ColorSync/Profiles/sRGB Profile.icc"
        )
        colorSpaceID = "photara.color.srgb"
        embeddedICCFingerprint = nil
    } else {
        guard let iccData = embedded.copyICCData() as Data? else {
            fail("HDR source profile \(embeddedName) has no reusable ICC payload")
        }
        outputColorSpace = embedded
        colorSpaceID = "photara.color.embedded-icc"
        embeddedICCFingerprint = SHA256.hash(data: iccData)
            .map { String(format: "%02x", $0) }
            .joined()
    }
}

let context = CIContext(options: [.cacheIntermediates: false])
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
    guard let written = CGImageSourceCreateWithURL(outputURL as CFURL, nil),
          let properties = CGImageSourceCopyPropertiesAtIndex(written, 0, nil) as? [CFString: Any],
          let width = properties[kCGImagePropertyPixelWidth] as? Int,
          let height = properties[kCGImagePropertyPixelHeight] as? Int,
          width > 0,
          height > 0 else {
        fail("could not verify written proxy dimensions")
    }
    let metadata = HelperMetadata(
        pixelWidth: width,
        pixelHeight: height,
        colorSpaceID: colorSpaceID,
        embeddedICCFingerprint: embeddedICCFingerprint,
        headroomMillistops: nil
    )
    let data = try JSONEncoder().encode(metadata)
    try data.write(to: metadataURL, options: .atomic)
} catch {
    let outputName = outputColorSpace.name as String? ?? "unnamed output color space"
    fail(
        "proxy generation failed for source profile \(sourceColorDescription) "
            + "using \(colorSpaceID) [\(outputName)] at "
            + "\(Int(image.extent.width))x\(Int(image.extent.height)): \(error)"
    )
}
