import Foundation

struct ImagePreviewMetadata: Equatable, Sendable {
    enum DynamicRange: Equatable, Sendable { case hdr, sdr }
    var pixelWidth: UInt32
    var pixelHeight: UInt32
    var dynamicRange: DynamicRange
    var colorSpaceId: String
}
