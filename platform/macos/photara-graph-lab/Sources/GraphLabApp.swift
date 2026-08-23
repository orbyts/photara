import SwiftUI

@main
struct PhotaraGraphLabApp: App {
    @State private var appearance = PhotaraThemeAppearance.dark
    private let document: PhotaraThemeDocument

    init() {
        guard let url = Bundle.main.url(forResource: "photara-default", withExtension: "json") else {
            fatalError("Graph Lab is missing photara-default.json")
        }
        do {
            document = try PhotaraThemeDocument.load(from: url)
        } catch {
            fatalError("Graph Lab default theme is invalid: \(error.localizedDescription)")
        }
    }

    var body: some Scene {
        WindowGroup("Photara Graph Lab") {
            GraphLabView(appearance: $appearance)
                .environment(\.photaraTheme, document.resolved(for: appearance))
                .preferredColorScheme(appearance == .dark ? .dark : .light)
                .frame(minWidth: 1_080, minHeight: 700)
        }
    }
}
