import SwiftUI

@main
struct PhotaraGraphLabApp: App {
    @State private var appearance = PhotaraThemeAppearance.dark
    @StateObject private var themeStore = PhotaraThemeStore()

    var body: some Scene {
        WindowGroup("Photara Graph Lab") {
            GraphLabView(appearance: $appearance)
                .environment(\.photaraTheme, themeStore.document.resolved(for: appearance))
                .preferredColorScheme(appearance == .dark ? .dark : .light)
                .frame(minWidth: 1_080, minHeight: 700)
        }
        Settings {
            GraphLabSettingsView()
        }
    }
}
