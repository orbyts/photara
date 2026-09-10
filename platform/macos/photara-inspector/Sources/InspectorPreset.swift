import Foundation

struct InspectorPreset: Codable, Equatable {
    var schemaVersion: Int
    var noSelectionState: PhotaraEmptyStatePreset
    var graphHiddenState: PhotaraEmptyStatePreset
    var noSettingsState: PhotaraEmptyStatePreset

    static func decode(_ data: Data) throws -> Self {
        let preset = try JSONDecoder().decode(Self.self, from: data)
        guard preset.schemaVersion == 1, preset.noSelectionState.isValid,
              preset.graphHiddenState.isValid, preset.noSettingsState.isValid
        else { throw CocoaError(.coderReadCorrupt) }
        return preset
    }
    static let shipped: Self = {
        guard let url = Bundle.main.url(forResource: "photara-inspector-presentation-v1", withExtension: "json"),
              let data = try? Data(contentsOf: url), let preset = try? decode(data)
        else { fatalError("Missing or invalid shared Inspector preset") }
        return preset
    }()
    func encoded() throws -> Data {
        let encoder = JSONEncoder(); encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        let data = try encoder.encode(self)
        _ = try Self.decode(data)
        return data
    }
}

enum PhotaraInspectorDevelopmentSettings {
    static let applicationID = "com.photara.desktop"
    static let presetDataKey = "PhotaraDeveloperInspectorPresetV1"
    static var overridePreset: InspectorPreset? {
        guard let data = CFPreferencesCopyAppValue(presetDataKey as CFString, applicationID as CFString) as? Data
        else { return nil }
        return try? InspectorPreset.decode(data)
    }
    static func setOverride(_ preset: InspectorPreset?) throws {
        let value = try preset?.encoded()
        CFPreferencesSetAppValue(presetDataKey as CFString, value as CFData?, applicationID as CFString)
        CFPreferencesAppSynchronize(applicationID as CFString)
    }
}

extension InspectorPreset {
    static var developmentOrShipped: Self { PhotaraInspectorDevelopmentSettings.overridePreset ?? .shipped }
}
