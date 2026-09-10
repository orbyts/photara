import Foundation

/// Authored visual defaults. Filter, selection, current size and grid choice are transient viewing state.
struct GalleryPreset: Codable, Equatable {
    var schemaVersion: Int
    var defaultThumbnailSize: Double
    var photoSpacing: Double
    var squareRowSpacing: Double
    var selectionStrokeWidth: Double
    var noSourceState: PhotaraEmptyStatePreset
    var awaitingAssetsState: PhotaraEmptyStatePreset
    var noMatchesState: PhotaraEmptyStatePreset

    private enum CodingKeys: String, CodingKey {
        case schemaVersion, defaultThumbnailSize, photoSpacing, squareRowSpacing
        case selectionStrokeWidth, noSourceState, awaitingAssetsState, noMatchesState
    }

    init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        schemaVersion = try values.decode(Int.self, forKey: .schemaVersion)
        defaultThumbnailSize = try values.decode(Double.self, forKey: .defaultThumbnailSize)
        photoSpacing = try values.decode(Double.self, forKey: .photoSpacing)
        squareRowSpacing = try values.decode(Double.self, forKey: .squareRowSpacing)
        selectionStrokeWidth = try values.decode(Double.self, forKey: .selectionStrokeWidth)
        noSourceState = try values.decodeIfPresent(PhotaraEmptyStatePreset.self, forKey: .noSourceState)
            ?? Self.defaultNoSource
        awaitingAssetsState = try values.decodeIfPresent(PhotaraEmptyStatePreset.self, forKey: .awaitingAssetsState)
            ?? Self.defaultAwaitingAssets
        noMatchesState = try values.decodeIfPresent(PhotaraEmptyStatePreset.self, forKey: .noMatchesState)
            ?? Self.defaultNoMatches
    }

    init(schemaVersion: Int, defaultThumbnailSize: Double, photoSpacing: Double,
         squareRowSpacing: Double, selectionStrokeWidth: Double,
         noSourceState: PhotaraEmptyStatePreset = Self.defaultNoSource,
         awaitingAssetsState: PhotaraEmptyStatePreset = Self.defaultAwaitingAssets,
         noMatchesState: PhotaraEmptyStatePreset = Self.defaultNoMatches) {
        self.schemaVersion = schemaVersion
        self.defaultThumbnailSize = defaultThumbnailSize
        self.photoSpacing = photoSpacing
        self.squareRowSpacing = squareRowSpacing
        self.selectionStrokeWidth = selectionStrokeWidth
        self.noSourceState = noSourceState
        self.awaitingAssetsState = awaitingAssetsState
        self.noMatchesState = noMatchesState
    }

    static func decode(_ data: Data) throws -> Self {
        let preset = try JSONDecoder().decode(Self.self, from: data)
        guard preset.schemaVersion == 1,
              (76...220).contains(preset.defaultThumbnailSize),
              (0...16).contains(preset.photoSpacing),
              (0...24).contains(preset.squareRowSpacing),
              (0.5...4).contains(preset.selectionStrokeWidth),
              preset.noSourceState.isValid, preset.awaitingAssetsState.isValid,
              preset.noMatchesState.isValid else {
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

    private static let defaultNoSource = PhotaraEmptyStatePreset(
        icon: "photo.on.rectangle.angled", title: "Your project assets will appear here",
        message: "Add a source node to your workflow to bring photos and other media into this project.",
        actionTitle: "Add Source Node", iconSize: 38, titleSize: 20, messageSize: 14,
        spacing: 12, verticalOffset: 0, maximumTextWidth: 380
    )
    private static let defaultAwaitingAssets = PhotaraEmptyStatePreset(
        icon: "photo.stack", title: "No assets yet",
        message: "Run the workflow to load assets from your source nodes.",
        actionTitle: "Run Workflow", iconSize: 38, titleSize: 20, messageSize: 14,
        spacing: 12, verticalOffset: 0, maximumTextWidth: 380
    )
    private static let defaultNoMatches = PhotaraEmptyStatePreset(
        icon: "magnifyingglass", title: "No matching assets",
        message: "Try changing your search or filters.", actionTitle: "Clear Filter",
        iconSize: 38, titleSize: 20, messageSize: 14, spacing: 12,
        verticalOffset: 0, maximumTextWidth: 380
    )
}

enum PhotaraGalleryDevelopmentSettings {
    static let applicationID = "com.photara.desktop"
    static let presetDataKey = "PhotaraDeveloperGalleryPresetV1"
    static var overridePreset: GalleryPreset? {
        guard let data = CFPreferencesCopyAppValue(presetDataKey as CFString, applicationID as CFString) as? Data
        else { return nil }
        return try? GalleryPreset.decode(data)
    }
    static func setOverride(_ preset: GalleryPreset?) throws {
        let value = try preset?.encoded()
        CFPreferencesSetAppValue(presetDataKey as CFString, value as CFData?, applicationID as CFString)
        CFPreferencesAppSynchronize(applicationID as CFString)
    }
}

extension GalleryPreset {
    static var developmentOrShipped: Self { PhotaraGalleryDevelopmentSettings.overridePreset ?? .shipped }
}
