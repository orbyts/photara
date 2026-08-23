import Foundation
import SwiftUI

/// Stable semantic roles shared by Photara and the developer Theme Lab.
/// The portable JSON stores these raw values and contains both appearances.
enum PhotaraThemeRole: String, CaseIterable, Identifiable {
    case surfaceCanvas = "surface.canvas"
    case surfacePanel = "surface.panel"
    case surfaceElevated = "surface.elevated"
    case surfaceControl = "surface.control"
    case textPrimary = "text.primary"
    case textSecondary = "text.secondary"
    case textDisabled = "text.disabled"
    case borderSubtle = "border.subtle"
    case borderStrong = "border.strong"
    case borderFocus = "border.focus"
    case selectionBackground = "selection.background"
    case selectionForeground = "selection.foreground"
    case graphBackground = "graph.background"
    case graphGrid = "graph.grid"
    case graphNode = "graph.node"
    case graphNodeSelected = "graph.node-selected"
    case galleryBackground = "gallery.background"
    case galleryCell = "gallery.cell"
    case workspaceSurround = "workspace.surround"
    case statusTextNeutral = "status.text.neutral"
    case statusTextRunning = "status.text.running"
    case statusTextSuccess = "status.text.success"
    case statusTextWarning = "status.text.warning"
    case statusTextError = "status.text.error"
    case statusTextCancelled = "status.text.cancelled"
    case nodeNative = "node.native"
    case nodeIO = "node.io"
    case nodeTransform = "node.transform"
    case nodeCreative = "node.creative"
    case nodeAutomation = "node.automation"
    case nodeIntegration = "node.integration"
    case nodeCompute = "node.compute"

    var id: String { rawValue }
}

enum PhotaraThemeAppearance: String, CaseIterable, Identifiable {
    case light
    case dark

    var id: String { rawValue }
}

struct PhotaraThemeMode: Codable, Equatable {
    var colors: [String: String]
}

struct PhotaraThemeModes: Codable, Equatable {
    var light: PhotaraThemeMode
    var dark: PhotaraThemeMode
}

struct PhotaraThemeDocument: Codable, Equatable {
    var schemaVersion: Int
    var id: String
    var displayName: String
    var colorSpace: String
    var modes: PhotaraThemeModes

    static func load(from url: URL) throws -> Self {
        let data = try Data(contentsOf: url)
        let document = try JSONDecoder().decode(Self.self, from: data)
        try document.validate()
        return document
    }

    func write(to url: URL) throws {
        try validate()
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        try encoder.encode(self).write(to: url, options: .atomic)
    }

    func validate() throws {
        guard schemaVersion == 1 else {
            throw PhotaraThemeError.unsupportedSchema(schemaVersion)
        }
        guard !id.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            throw PhotaraThemeError.emptyIdentifier
        }
        guard !displayName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            throw PhotaraThemeError.emptyDisplayName
        }
        guard colorSpace.lowercased() == "srgb" else {
            throw PhotaraThemeError.unsupportedColorSpace(colorSpace)
        }

        let lightKeys = Set(modes.light.colors.keys)
        let darkKeys = Set(modes.dark.colors.keys)
        guard lightKeys == darkKeys else {
            throw PhotaraThemeError.appearanceSlotMismatch(
                lightOnly: lightKeys.subtracting(darkKeys).sorted(),
                darkOnly: darkKeys.subtracting(lightKeys).sorted()
            )
        }
        let required = Set(PhotaraThemeRole.allCases.map(\.rawValue))
        let missing = required.subtracting(lightKeys).sorted()
        guard missing.isEmpty else {
            throw PhotaraThemeError.missingSlots(missing)
        }
        for appearance in PhotaraThemeAppearance.allCases {
            for (slot, value) in mode(appearance).colors {
                guard PhotaraRGBA(hex: value) != nil else {
                    throw PhotaraThemeError.invalidColor(
                        appearance: appearance.rawValue,
                        slot: slot,
                        value: value
                    )
                }
            }
        }
    }

    func mode(_ appearance: PhotaraThemeAppearance) -> PhotaraThemeMode {
        appearance == .dark ? modes.dark : modes.light
    }

    mutating func setColor(
        _ hex: String,
        for role: PhotaraThemeRole,
        appearance: PhotaraThemeAppearance
    ) {
        if appearance == .dark {
            modes.dark.colors[role.rawValue] = hex
        } else {
            modes.light.colors[role.rawValue] = hex
        }
    }

    func resolved(for appearance: PhotaraThemeAppearance) -> PhotaraResolvedTheme {
        PhotaraResolvedTheme(
            id: id,
            displayName: displayName,
            appearance: appearance,
            colors: mode(appearance).colors.compactMapValues(PhotaraRGBA.init(hex:))
        )
    }

    func contrastWarnings() -> [String] {
        let pairs: [(PhotaraThemeRole, PhotaraThemeRole, Double)] = [
            (.textPrimary, .surfaceCanvas, 4.5),
            (.textPrimary, .surfacePanel, 4.5),
            (.selectionForeground, .selectionBackground, 4.5),
        ]
        return PhotaraThemeAppearance.allCases.flatMap { appearance in
            let palette = resolved(for: appearance)
            return pairs.compactMap { pair -> String? in
                let (foreground, background, minimum) = pair
                guard let foregroundColor = palette.rgba(foreground),
                      let backgroundColor = palette.rgba(background)
                else { return nil }
                let ratio = foregroundColor.contrastRatio(with: backgroundColor)
                return ratio < minimum
                    ? "\(appearance.rawValue) \(foreground.rawValue) on "
                        + "\(background.rawValue) is \(String(format: "%.2f", ratio)):1; "
                        + "target \(String(format: "%.1f", minimum)):1"
                    : nil
            }
        }
    }
}

struct PhotaraResolvedTheme: Equatable {
    let id: String
    let displayName: String
    let appearance: PhotaraThemeAppearance
    fileprivate let colors: [String: PhotaraRGBA]

    func rgba(_ role: PhotaraThemeRole) -> PhotaraRGBA? {
        colors[role.rawValue]
    }

    func color(_ role: PhotaraThemeRole) -> Color {
        colors[role.rawValue]?.color ?? .pink
    }
}

struct PhotaraRGBA: Equatable {
    let red: Double
    let green: Double
    let blue: Double
    let alpha: Double

    init?(hex: String) {
        let value = hex.trimmingCharacters(in: .whitespacesAndNewlines)
        guard value.first == "#" else { return nil }
        let digits = String(value.dropFirst())
        guard digits.count == 6 || digits.count == 8,
              let raw = UInt64(digits, radix: 16)
        else { return nil }
        if digits.count == 6 {
            red = Double((raw >> 16) & 0xFF) / 255
            green = Double((raw >> 8) & 0xFF) / 255
            blue = Double(raw & 0xFF) / 255
            alpha = 1
        } else {
            red = Double((raw >> 24) & 0xFF) / 255
            green = Double((raw >> 16) & 0xFF) / 255
            blue = Double((raw >> 8) & 0xFF) / 255
            alpha = Double(raw & 0xFF) / 255
        }
    }

    var color: Color {
        Color(.sRGB, red: red, green: green, blue: blue, opacity: alpha)
    }

    var hex: String {
        let components = [red, green, blue, alpha].map {
            Int(($0.clamped(to: 0...1) * 255).rounded())
        }
        if components[3] == 255 {
            return String(
                format: "#%02X%02X%02X",
                components[0], components[1], components[2]
            )
        }
        return String(
            format: "#%02X%02X%02X%02X",
            components[0], components[1], components[2], components[3]
        )
    }

    fileprivate func contrastRatio(with other: Self) -> Double {
        let light = max(relativeLuminance, other.relativeLuminance)
        let dark = min(relativeLuminance, other.relativeLuminance)
        return (light + 0.05) / (dark + 0.05)
    }

    private var relativeLuminance: Double {
        func linear(_ component: Double) -> Double {
            component <= 0.04045
                ? component / 12.92
                : pow((component + 0.055) / 1.055, 2.4)
        }
        return 0.2126 * linear(red) + 0.7152 * linear(green) + 0.0722 * linear(blue)
    }
}

enum PhotaraThemeError: LocalizedError {
    case unsupportedSchema(Int)
    case emptyIdentifier
    case emptyDisplayName
    case unsupportedColorSpace(String)
    case appearanceSlotMismatch(lightOnly: [String], darkOnly: [String])
    case missingSlots([String])
    case invalidColor(appearance: String, slot: String, value: String)

    var errorDescription: String? {
        switch self {
        case let .unsupportedSchema(version):
            "unsupported theme schema version \(version)"
        case .emptyIdentifier:
            "theme identifier must not be empty"
        case .emptyDisplayName:
            "theme display name must not be empty"
        case let .unsupportedColorSpace(colorSpace):
            "unsupported theme color space \(colorSpace); expected srgb"
        case let .appearanceSlotMismatch(lightOnly, darkOnly):
            "light and dark modes must contain identical slots "
                + "(light-only: \(lightOnly), dark-only: \(darkOnly))"
        case let .missingSlots(slots):
            "theme is missing required slots: \(slots.joined(separator: ", "))"
        case let .invalidColor(appearance, slot, value):
            "invalid \(appearance) sRGB color \(value) for \(slot)"
        }
    }
}

enum PhotaraThemeDevelopmentSettings {
    static let applicationID = "com.photara.desktop"
    static let themePathKey = "PhotaraDeveloperThemePath"

    static var overrideURL: URL? {
        guard let path = CFPreferencesCopyAppValue(
            themePathKey as CFString,
            applicationID as CFString
        ) as? String, !path.isEmpty else { return nil }
        return URL(fileURLWithPath: path).standardizedFileURL
    }

    static func setOverrideURL(_ url: URL?) {
        CFPreferencesSetAppValue(
            themePathKey as CFString,
            url?.standardizedFileURL.path as CFPropertyList?,
            applicationID as CFString
        )
        CFPreferencesAppSynchronize(applicationID as CFString)
    }
}

private extension Double {
    func clamped(to range: ClosedRange<Double>) -> Double {
        min(max(self, range.lowerBound), range.upperBound)
    }
}
