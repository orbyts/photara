import Foundation

/// Authored visual defaults. Filter, selection, current size and grid choice are transient viewing state.
struct GalleryPreset: Codable, Equatable {
    var schemaVersion: Int
    var defaultThumbnailSize: Double
    var photoSpacing: Double
    var squareRowSpacing: Double
    var selectionStrokeWidth: Double

    static func decode(_ data: Data) throws -> Self {
        let preset = try JSONDecoder().decode(Self.self, from: data)
        guard preset.schemaVersion == 1,
              (76...220).contains(preset.defaultThumbnailSize),
              (0...16).contains(preset.photoSpacing),
              (0...24).contains(preset.squareRowSpacing),
              (0.5...4).contains(preset.selectionStrokeWidth) else {
            throw CocoaError(.coderReadCorrupt)
        }
        return preset
    }
    static let shipped: Self = {
        guard let url = Bundle.main.url(forResource: "photara-gallery-presentation-v1", withExtension: "json"),
              let data = try? Data(contentsOf: url), let preset = try? decode(data) else {
            fatalError("Missing or invalid shared Gallery preset")
        }
        return preset
    }()
    func encoded() throws -> Data {
        let encoder = JSONEncoder(); encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        let data = try encoder.encode(self)
        _ = try Self.decode(data)
        return data
    }
}
