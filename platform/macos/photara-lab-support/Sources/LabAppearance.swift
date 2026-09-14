import AppKit
import SwiftUI
import UniformTypeIdentifiers

struct LabAppearance<Content: View>: View {
    var dark: Bool
    @ViewBuilder var content: () -> Content
    @StateObject private var themeStore: PhotaraThemeStore

    init(dark: Bool, usesDevelopmentTheme: Bool = true, @ViewBuilder content: @escaping () -> Content) {
        self.dark = dark
        self.content = content
        _themeStore = StateObject(wrappedValue: PhotaraThemeStore(usesDevelopmentOverride: usesDevelopmentTheme))
    }
    var body: some View {
        let theme = themeStore.document.resolved(for: dark ? .dark : .light)
        content().environment(\.photaraTheme, theme)
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
