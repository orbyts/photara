import AppKit
import SwiftUI
import UniformTypeIdentifiers

struct LabAppearance<Content: View>: View {
    var dark: Bool
    @ViewBuilder var content: () -> Content
    private let document = try! PhotaraThemeDocument.load(from: Bundle.main.url(forResource: "photara-default", withExtension: "json")!)
    var body: some View {
        let theme = document.resolved(for: dark ? .dark : .light)
        content().environment(\.photaraTheme, theme).tint(theme.color(.borderFocus))
            .preferredColorScheme(dark ? .dark : .light)
    }
}

@MainActor
enum LabPresetExport {
    static func save(_ data: Data, filename: String) throws {
        let panel = NSSavePanel()
        panel.nameFieldStringValue = filename
        panel.allowedContentTypes = [.json]
        guard panel.runModal() == .OK, let url = panel.url else { return }
        try data.write(to: url, options: .atomic)
    }
}
