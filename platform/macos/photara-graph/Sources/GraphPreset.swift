import AppKit
import Foundation
import SwiftUI

struct PhotaraGraphColorValue: Codable, Equatable {
    let red: Double
    let green: Double
    let blue: Double
    let opacity: Double
    var color: Color { Color(.sRGB, red: red, green: green, blue: blue, opacity: opacity) }

    init(red: Double, green: Double, blue: Double, opacity: Double) {
        self.red = red; self.green = green; self.blue = blue; self.opacity = opacity
    }

    init?(srgbHex: String?) {
        guard var value = srgbHex?.trimmingCharacters(in: .whitespacesAndNewlines),
              value.hasPrefix("#") else { return nil }
        value.removeFirst()
        guard value.count == 6, let rgb = UInt64(value, radix: 16) else { return nil }
        self.init(red: Double((rgb >> 16) & 0xff) / 255,
                  green: Double((rgb >> 8) & 0xff) / 255,
                  blue: Double(rgb & 0xff) / 255, opacity: 1)
    }
}

struct PhotaraGraphPalettePreset: Codable, Equatable {
    var graphBackground: PhotaraGraphColorValue?
    var minor: PhotaraGraphColorValue?
    var major: PhotaraGraphColorValue?
    var noodle: PhotaraGraphColorValue?
    var idleNodeFill: PhotaraGraphColorValue?
    var selectedNodeFill: PhotaraGraphColorValue?
    var selectedNodeStroke: PhotaraGraphColorValue?
    var portGlassTint: PhotaraGraphColorValue?
    var titleText: PhotaraGraphColorValue?
    var detailText: PhotaraGraphColorValue?
}

/// Versioned shipped visual defaults shared by Photara and Graph Lab. User
/// workspace choices intentionally live in each application's own defaults.
struct PhotaraGraphPresentationPreset: Codable, Equatable {
    let schemaVersion: Int
    var pattern: PhotaraGraphPattern
    var gridSpacing: Double
    var minorOpacity: Double
    var minorLineWidth: Double
    var majorInterval: Int
    var majorOpacity: Double
    var majorLineWidth: Double
    var light: PhotaraGraphPalettePreset
    var dark: PhotaraGraphPalettePreset
    var selectedStrokeWidth: Double
    var cornerRadius: Double
    var portOffset: Double
    var portGlassTintOpacity: Double
    var portCoreSize: Double
    var inactivePortSaturation: Double
    var inactivePortShowsStroke: Bool
    var inactivePortStrokeWidth: Double
    var activePortShowsShadow: Bool
    var activePortShadowOpacity: Double
    var activePortShadowBlur: Double
    var activePortShadowOffsetY: Double
    var lightPortCoreBrightness: Double
    var darkPortCoreBrightness: Double
    var lightActivePortCoreBrightness: Double
    var darkActivePortCoreBrightness: Double
    var nodeShadowBlur: Double
    var nodeShadowOffsetY: Double
    var lightNodeShadowOpacity: Double
    var darkNodeShadowOpacity: Double
    var overviewSizeFraction: Double
    var overviewCornerRadius: Double
    var toolRailCornerRadius: Double
    var toolRailLightShadowOpacity: Double
    var toolRailDarkShadowOpacity: Double
    var toolRailShadowBlur: Double
    var toolRailShadowOffsetY: Double

    static let shipped: Self = {
        guard let url = Bundle.main.url(forResource: "photara-graph-presentation-v1", withExtension: "json"),
              let data = try? Data(contentsOf: url),
              let value = try? JSONDecoder().decode(Self.self, from: data), value.schemaVersion == 1
        else { return fallback }
        return value
    }()

    static let fallback = Self(
        schemaVersion: 1, pattern: .lines, gridSpacing: 23.428415697674417,
        minorOpacity: 0.14435954964395167, minorLineWidth: 0.5033693489536385,
        majorInterval: 5, majorOpacity: 0.48353682170542633, majorLineWidth: 1.1589772859578753,
        light: .init(graphBackground: .init(red: 0.9087992906570435, green: 0.9087992906570435, blue: 0.9087992906570435, opacity: 1), minor: .init(red: 0.5370979905128479, green: 0.5370979905128479, blue: 0.539517343044281, opacity: 1), major: .init(red: 0.7049574851989746, green: 0.7095056176185608, blue: 0.7186018824577332, opacity: 1), selectedNodeStroke: .init(red: 0.4898940920829773, green: 0.7537683248519897, blue: 0.8924444913864136, opacity: 1), portGlassTint: .init(red: 0.8519638776779175, green: 0.8519638776779175, blue: 0.8519638776779175, opacity: 1)),
        dark: .init(graphBackground: .init(red: 0.019531242549419403, green: 0.019531242549419403, blue: 0.019531242549419403, opacity: 1), minor: .init(red: 0.7355009913444519, green: 0.7355009913444519, blue: 0.7355009913444519, opacity: 1), major: .init(red: 0.24860872328281403, green: 0.24860872328281403, blue: 0.24860872328281403, opacity: 1), idleNodeFill: .init(red: 0.17556720972061157, green: 0.17556720972061157, blue: 0.17556720972061157, opacity: 1), selectedNodeStroke: .init(red: 0.4089040160179138, green: 0.5477041006088257, blue: 0.6694936752319336, opacity: 1), portGlassTint: .init(red: 0.5122003555297852, green: 0.5122003555297852, blue: 0.5122003555297852, opacity: 1), titleText: .init(red: 1, green: 1, blue: 1, opacity: 1), detailText: .init(red: 0.6900952458381653, green: 0.6900952458381653, blue: 0.6900952458381653, opacity: 1)),
        selectedStrokeWidth: 1.1612766472868217, cornerRadius: 12, portOffset: 0,
        portGlassTintOpacity: 0.0866418829328561, portCoreSize: 5.366642441860465,
        inactivePortSaturation: 0.7796062530046272, inactivePortShowsStroke: true,
        inactivePortStrokeWidth: 1.6718592726548884, activePortShowsShadow: true,
        activePortShadowOpacity: 0.42026647286821706, activePortShadowBlur: 6.968023255813952,
        activePortShadowOffsetY: 1.5, lightPortCoreBrightness: 0, darkPortCoreBrightness: -0.15135934025899883,
        lightActivePortCoreBrightness: 0.12, darkActivePortCoreBrightness: 0.35,
        nodeShadowBlur: 3.5347020348837206, nodeShadowOffsetY: 7.807314670843098,
        lightNodeShadowOpacity: 0.12, darkNodeShadowOpacity: 0.21649616732442378,
        overviewSizeFraction: 0.13779796511627906, overviewCornerRadius: 12,
        toolRailCornerRadius: 20.73002580223544, toolRailLightShadowOpacity: 0.09009265988372091,
        toolRailDarkShadowOpacity: 0.34, toolRailShadowBlur: 8.775920542635658,
        toolRailShadowOffsetY: 0.3459302325581399)

    func palette(_ scheme: ColorScheme) -> PhotaraGraphPalettePreset { scheme == .dark ? dark : light }
    var backgroundStyle: PhotaraGraphBackgroundStyle { .init(pattern: pattern, spacing: gridSpacing,
        opacity: minorOpacity, markSize: 1.4, lineWidth: minorLineWidth, majorInterval: majorInterval,
        majorOpacity: majorOpacity, majorMarkSize: 3, majorLineWidth: majorLineWidth) }
}
