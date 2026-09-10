import Foundation
import SwiftUI

/// Versioned visual values only. Availability and project state remain typed Swift.
struct ApplicationShellPreset: Codable, Equatable {
    enum TitleWeight: String, Codable, CaseIterable {
        case regular, medium, semibold, bold
        var fontWeight: Font.Weight {
            switch self { case .regular: .regular; case .medium: .medium; case .semibold: .semibold; case .bold: .bold }
        }
    }
    var schemaVersion: Int
    var launcherTitleSize: Double
    var launcherTitleWeight: TitleWeight
    var heroSize: Double
    var heroIconSize: Double
    var launcherSpacing: Double
    var heroSpacing: Double
    var contentInset: Double
    var leadingIdealWidth: Double
    var trailingIdealWidth: Double
    var panelHeaderHeight: Double
    var statusBarHeight: Double
    var toolbarIdentityWidth: Double

    static func decode(_ data: Data) throws -> Self {
        let p = try JSONDecoder().decode(Self.self, from: data)
        guard p.schemaVersion == 1,
              (24...56).contains(p.launcherTitleSize), (72...160).contains(p.heroSize),
              (32...88).contains(p.heroIconSize), p.heroIconSize <= p.heroSize - 16,
              (12...48).contains(p.launcherSpacing), (12...36).contains(p.heroSpacing),
              (16...44).contains(p.contentInset), (230...360).contains(p.leadingIdealWidth),
              (280...440).contains(p.trailingIdealWidth), (28...44).contains(p.panelHeaderHeight),
              (24...36).contains(p.statusBarHeight), (120...240).contains(p.toolbarIdentityWidth)
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

/// The authored presentation shared by all hosts, independent of project contracts.
typealias ApplicationShellPresentation = ApplicationShellPreset
