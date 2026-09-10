import Foundation
import SwiftUI

/// Versioned visual values only. Availability and project state remain typed Swift.
struct ApplicationShellPreset: Codable, Equatable {
    struct AdaptiveColor: Codable, Equatable {
        var light: String
        var dark: String

        func color(_ scheme: ColorScheme) -> Color {
            PhotaraRGBA(hex: scheme == .dark ? dark : light)?.color ?? .pink
        }

        mutating func set(_ hex: String, for scheme: ColorScheme) {
            if scheme == .dark { dark = hex } else { light = hex }
        }

        var isValid: Bool {
            PhotaraRGBA(hex: light) != nil && PhotaraRGBA(hex: dark) != nil
        }
    }

    /// Portable design intent. Native shells map these roles to their own
    /// platform families rather than serializing a platform font name.
    enum TitleFontFamily: String, Codable, CaseIterable {
        case display, rounded, serif, monospaced

        var fontDesign: Font.Design {
            switch self {
            case .display: .default
            case .rounded: .rounded
            case .serif: .serif
            case .monospaced: .monospaced
            }
        }

        var macOSLabel: String {
            switch self {
            case .display: "SF Display"
            case .rounded: "SF Rounded"
            case .serif: "New York"
            case .monospaced: "SF Mono"
            }
        }
    }

    /// Cross-platform material intent. Each native shell maps the role to its
    /// closest system backdrop material.
    enum LauncherBackgroundStyle: String, Codable, CaseIterable {
        case theme, ultraThin, thin, regular, thick

        var label: String {
            switch self {
            case .theme: "Theme Background"
            case .ultraThin: "Ultra Thin Frost"
            case .thin: "Thin Frost"
            case .regular: "Regular Frost"
            case .thick: "Thick Frost"
            }
        }
    }

    enum TitleWeight: String, Codable, CaseIterable {
        case regular, medium, semibold, bold
        var fontWeight: Font.Weight {
            switch self { case .regular: .regular; case .medium: .medium; case .semibold: .semibold; case .bold: .bold }
        }
    }
    struct SurfaceFrame: Codable, Equatable {
        var canvasMaterial: LauncherBackgroundStyle = .theme
        var usesThemeFills = true
        var canvasFill = AdaptiveColor(light: "#E9EBEE", dark: "#141518")
        var surfaceFill = AdaptiveColor(light: "#FFFFFF", dark: "#242529")
        var gutter: Double = 12
        var outerInset: Double = 12
        var contentInset: Double = 6
        var cornerRadius: Double = 16
        var borderWidth: Double = 0
        var elevation: Double = 3
        var activeEmphasis: Double = 1.5
        var elevatedHeader = false
        var separateStatusSurface = true
        var compactBreakpoint: Double = 1100
        var isValid: Bool {
            canvasFill.isValid && surfaceFill.isValid && (4...32).contains(gutter)
                && (0...40).contains(outerInset) && (0...24).contains(contentInset)
                && (0...40).contains(cornerRadius) && (0...3).contains(borderWidth)
                && (0...16).contains(elevation) && (0...4).contains(activeEmphasis)
                && (800...1400).contains(compactBreakpoint)
        }
    }
    var frame: SurfaceFrame
    var schemaVersion: Int
    var launcherTitleSize: Double
    var launcherTitleFont: TitleFontFamily
    var launcherTitleWeight: TitleWeight
    var launcherBackgroundStyle: LauncherBackgroundStyle
    var launcherBackgroundTint: AdaptiveColor
    var heroShowsTile: Bool
    var heroSize: Double
    var heroIconSize: Double
    var heroSymbolOffsetY: Double
    var heroTileOffsetX: Double
    var heroTileOffsetY: Double
    var heroSymbolColor: AdaptiveColor
    var heroTileBackgroundColor: AdaptiveColor
    var heroTileStrokeColor: AdaptiveColor
    var heroTileStrokeWidth: Double
    var heroTileCornerRadius: Double
    var heroGlowColor: AdaptiveColor
    var heroGlowRadius: Double
    var heroGlowOffsetX: Double
    var heroGlowOffsetY: Double
    var launcherCreateButtonTint: AdaptiveColor
    var launcherOpenButtonTint: AdaptiveColor
    var launcherRecentButtonTint: AdaptiveColor
    var launcherSpacing: Double
    var heroSpacing: Double
    var contentInset: Double
    var contentHorizontalInset: Double
    var launcherVerticalOffset: Double
    var leadingIdealWidth: Double
    var trailingIdealWidth: Double
    var toolbarShowsProjectTitle: Bool
    var toolbarTitleSize: Double
    var toolbarTitleWeight: TitleWeight
    var panelHeaderHeight: Double
    var panelHeaderTitleSize: Double
    var panelHeaderHorizontalInset: Double
    var dividerThickness: Double
    var statusBarHeight: Double
    var statusTextSize: Double
    var statusHorizontalInset: Double
    var statusItemSpacing: Double
    var toolbarIdentityWidth: Double

    private enum CodingKeys: String, CodingKey {
        case frame
        case schemaVersion, launcherTitleSize, launcherTitleFont, launcherTitleWeight
        case launcherBackgroundStyle, launcherBackgroundTint
        case heroShowsTile, heroSize, heroIconSize, heroSymbolOffsetY
        case heroTileOffsetX, heroTileOffsetY, heroSymbolColor
        case heroTileBackgroundColor, heroTileStrokeColor, heroTileStrokeWidth
        case heroTileCornerRadius, heroGlowColor, heroGlowRadius, heroGlowOffsetX
        case heroGlowOffsetY, launcherCreateButtonTint, launcherOpenButtonTint
        case launcherRecentButtonTint, launcherSpacing, heroSpacing, contentInset
        case contentHorizontalInset, launcherVerticalOffset
        case leadingIdealWidth, trailingIdealWidth, toolbarShowsProjectTitle
        case toolbarTitleSize, toolbarTitleWeight, panelHeaderHeight
        case panelHeaderTitleSize, panelHeaderHorizontalInset, dividerThickness, statusBarHeight
        case statusTextSize, statusHorizontalInset, statusItemSpacing
        case toolbarIdentityWidth
    }

    init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        frame = try values.decodeIfPresent(SurfaceFrame.self, forKey: .frame) ?? .init()
        schemaVersion = try values.decode(Int.self, forKey: .schemaVersion)
        launcherTitleSize = try values.decode(Double.self, forKey: .launcherTitleSize)
        launcherTitleFont = try values.decodeIfPresent(TitleFontFamily.self, forKey: .launcherTitleFont) ?? .display
        launcherTitleWeight = try values.decode(TitleWeight.self, forKey: .launcherTitleWeight)
        launcherBackgroundStyle = try values.decodeIfPresent(
            LauncherBackgroundStyle.self,
            forKey: .launcherBackgroundStyle
        ) ?? .theme
        launcherBackgroundTint = try values.decodeIfPresent(AdaptiveColor.self, forKey: .launcherBackgroundTint)
            ?? .init(light: "#FFFFFF00", dark: "#00000000")
        heroShowsTile = try values.decodeIfPresent(Bool.self, forKey: .heroShowsTile) ?? true
        heroSize = try values.decode(Double.self, forKey: .heroSize)
        heroIconSize = try values.decode(Double.self, forKey: .heroIconSize)
        heroSymbolOffsetY = try values.decodeIfPresent(Double.self, forKey: .heroSymbolOffsetY) ?? 0
        heroTileOffsetX = try values.decodeIfPresent(Double.self, forKey: .heroTileOffsetX) ?? 0
        heroTileOffsetY = try values.decodeIfPresent(Double.self, forKey: .heroTileOffsetY) ?? 0
        heroSymbolColor = try values.decodeIfPresent(AdaptiveColor.self, forKey: .heroSymbolColor)
            ?? .init(light: "#007AFF", dark: "#0A84FF")
        heroTileBackgroundColor = try values.decodeIfPresent(AdaptiveColor.self, forKey: .heroTileBackgroundColor)
            ?? .init(light: "#007AFF1C", dark: "#0A84FF1C")
        heroTileStrokeColor = try values.decodeIfPresent(AdaptiveColor.self, forKey: .heroTileStrokeColor)
            ?? .init(light: "#007AFF33", dark: "#0A84FF33")
        heroTileStrokeWidth = try values.decodeIfPresent(Double.self, forKey: .heroTileStrokeWidth) ?? 1
        heroTileCornerRadius = try values.decodeIfPresent(Double.self, forKey: .heroTileCornerRadius) ?? 28
        heroGlowColor = try values.decodeIfPresent(AdaptiveColor.self, forKey: .heroGlowColor)
            ?? .init(light: "#00000014", dark: "#00000014")
        heroGlowRadius = try values.decodeIfPresent(Double.self, forKey: .heroGlowRadius) ?? 22
        heroGlowOffsetX = try values.decodeIfPresent(Double.self, forKey: .heroGlowOffsetX) ?? 0
        heroGlowOffsetY = try values.decodeIfPresent(Double.self, forKey: .heroGlowOffsetY) ?? 10
        launcherCreateButtonTint = try values.decodeIfPresent(AdaptiveColor.self, forKey: .launcherCreateButtonTint)
            ?? .init(light: "#007AFF", dark: "#0A84FF")
        launcherOpenButtonTint = try values.decodeIfPresent(AdaptiveColor.self, forKey: .launcherOpenButtonTint)
            ?? .init(light: "#007AFF", dark: "#0A84FF")
        launcherRecentButtonTint = try values.decodeIfPresent(AdaptiveColor.self, forKey: .launcherRecentButtonTint)
            ?? .init(light: "#007AFF", dark: "#0A84FF")
        launcherSpacing = try values.decode(Double.self, forKey: .launcherSpacing)
        heroSpacing = try values.decode(Double.self, forKey: .heroSpacing)
        contentInset = try values.decode(Double.self, forKey: .contentInset)
        contentHorizontalInset = try values.decodeIfPresent(Double.self, forKey: .contentHorizontalInset) ?? 44
        launcherVerticalOffset = try values.decodeIfPresent(Double.self, forKey: .launcherVerticalOffset) ?? 0
        leadingIdealWidth = try values.decode(Double.self, forKey: .leadingIdealWidth)
        trailingIdealWidth = try values.decode(Double.self, forKey: .trailingIdealWidth)
        toolbarShowsProjectTitle = try values.decodeIfPresent(Bool.self, forKey: .toolbarShowsProjectTitle) ?? true
        toolbarTitleSize = try values.decodeIfPresent(Double.self, forKey: .toolbarTitleSize) ?? 13
        toolbarTitleWeight = try values.decodeIfPresent(TitleWeight.self, forKey: .toolbarTitleWeight) ?? .semibold
        panelHeaderHeight = try values.decode(Double.self, forKey: .panelHeaderHeight)
        panelHeaderTitleSize = try values.decodeIfPresent(Double.self, forKey: .panelHeaderTitleSize) ?? 13
        panelHeaderHorizontalInset = try values.decodeIfPresent(Double.self, forKey: .panelHeaderHorizontalInset) ?? 10
        dividerThickness = try values.decodeIfPresent(Double.self, forKey: .dividerThickness) ?? 1
        statusBarHeight = try values.decode(Double.self, forKey: .statusBarHeight)
        statusTextSize = try values.decodeIfPresent(Double.self, forKey: .statusTextSize) ?? 11
        statusHorizontalInset = try values.decodeIfPresent(Double.self, forKey: .statusHorizontalInset) ?? 12
        statusItemSpacing = try values.decodeIfPresent(Double.self, forKey: .statusItemSpacing) ?? 12
        toolbarIdentityWidth = try values.decode(Double.self, forKey: .toolbarIdentityWidth)
    }

    static func decode(_ data: Data) throws -> Self {
        let p = try JSONDecoder().decode(Self.self, from: data)
        guard p.schemaVersion == 1, p.frame.isValid,
              (24...56).contains(p.launcherTitleSize), (72...160).contains(p.heroSize),
              (32...88).contains(p.heroIconSize), p.heroIconSize <= p.heroSize - 16,
              (-40...40).contains(p.heroSymbolOffsetY),
              (-160...160).contains(p.heroTileOffsetX), (-160...160).contains(p.heroTileOffsetY),
              p.heroSymbolColor.isValid, p.heroTileBackgroundColor.isValid,
              p.launcherBackgroundTint.isValid,
              p.heroTileStrokeColor.isValid, p.heroGlowColor.isValid,
              p.launcherCreateButtonTint.isValid, p.launcherOpenButtonTint.isValid,
              p.launcherRecentButtonTint.isValid,
              (0...6).contains(p.heroTileStrokeWidth),
              (0...60).contains(p.heroTileCornerRadius), p.heroTileCornerRadius <= p.heroSize / 2,
              (0...80).contains(p.heroGlowRadius),
              (-40...40).contains(p.heroGlowOffsetX), (-40...40).contains(p.heroGlowOffsetY),
              (12...48).contains(p.launcherSpacing), (12...36).contains(p.heroSpacing),
              (16...120).contains(p.contentInset), (16...120).contains(p.contentHorizontalInset),
              (-240...240).contains(p.launcherVerticalOffset),
              (230...360).contains(p.leadingIdealWidth),
              (280...440).contains(p.trailingIdealWidth),
              (10...24).contains(p.toolbarTitleSize),
              (28...60).contains(p.panelHeaderHeight), (10...22).contains(p.panelHeaderTitleSize),
              (4...32).contains(p.panelHeaderHorizontalInset), (0...4).contains(p.dividerThickness),
              (20...52).contains(p.statusBarHeight), (9...20).contains(p.statusTextSize),
              (4...32).contains(p.statusHorizontalInset), (4...28).contains(p.statusItemSpacing),
              (120...320).contains(p.toolbarIdentityWidth)
        else { throw CocoaError(.coderReadCorrupt) }
        return p
    }
    static let shipped: Self = {
        guard let url = Bundle.main.url(forResource: "photara-application-presentation-v1", withExtension: "json"),
              let data = try? Data(contentsOf: url), let preset = try? decode(data)
        else { fatalError("Missing or invalid Application Shell preset") }
        return preset
    }()
    func encoded() throws -> Data {
        let encoder = JSONEncoder(); encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        let data = try encoder.encode(self)
        _ = try Self.decode(data)
        return data
    }
}

enum PhotaraShellDevelopmentSettings {
    static let applicationID = "com.photara.desktop"
    static let presetDataKey = "PhotaraDeveloperShellPresetV1"

    static var overridePreset: ApplicationShellPreset? {
        guard let data = CFPreferencesCopyAppValue(
            presetDataKey as CFString,
            applicationID as CFString
        ) as? Data else { return nil }
        return try? ApplicationShellPreset.decode(data)
    }

    static func setOverride(_ preset: ApplicationShellPreset?) throws {
        let value = try preset?.encoded()
        CFPreferencesSetAppValue(
            presetDataKey as CFString,
            value as CFData?,
            applicationID as CFString
        )
        CFPreferencesAppSynchronize(applicationID as CFString)
    }
}

extension ApplicationShellPreset {
    static var developmentOrShipped: Self {
        PhotaraShellDevelopmentSettings.overridePreset ?? .shipped
    }
}

/// The authored presentation shared by all hosts, independent of project contracts.
typealias ApplicationShellPresentation = ApplicationShellPreset
