import AppKit
import SwiftUI

@main
@MainActor
struct PhotaraThemeLabApp: App {
    @StateObject private var model = ThemeLabModel()
    @StateObject private var previewApp: AppModel
    @StateObject private var previewWorkspace: WorkspaceModel

    init() {
        let defaults = UserDefaults(suiteName: "photara.theme-lab.preview") ?? .standard
        defaults.removePersistentDomain(forName: "photara.theme-lab.preview")
        let supportRoot = FileManager.default.temporaryDirectory
            .appending(path: "PhotaraThemeLab")
            .appending(path: String(ProcessInfo.processInfo.processIdentifier))
        _previewApp = StateObject(wrappedValue: AppModel(
            defaults: defaults,
            supportRootOverride: supportRoot
        ))
        _previewWorkspace = StateObject(wrappedValue: WorkspaceModel(defaults: defaults))
    }

    var body: some Scene {
        WindowGroup("Photara Theme Lab") {
            ThemeLabView()
                .environmentObject(model)
                .environmentObject(previewApp)
                .environmentObject(previewWorkspace)
                .frame(minWidth: 1_360, minHeight: 760)
        }
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("Open Theme…") { model.open() }
                    .keyboardShortcut("o")
            }
            CommandGroup(replacing: .saveItem) {
                Button("Save Theme") { model.save() }
                    .keyboardShortcut("s")
                Button("Save Theme As…") { model.saveAs() }
                    .keyboardShortcut("s", modifiers: [.command, .shift])
            }
        }
    }
}

@MainActor
final class ThemeLabModel: ObservableObject {
    @Published var document: PhotaraThemeDocument
    @Published var appearance: PhotaraThemeAppearance = .dark
    @Published var message = ""
    @Published private(set) var documentURL: URL?
    @Published private var hexDrafts: [String: String] = [:]

    init() {
        guard let bundled = Bundle.main.url(
            forResource: "photara-default",
            withExtension: "json"
        ) else {
            fatalError("Theme Lab is missing photara-default.json")
        }
        do {
            document = try PhotaraThemeDocument.load(from: bundled)
        } catch {
            fatalError("Theme Lab default theme is invalid: \(error.localizedDescription)")
        }
    }

    var resolved: PhotaraResolvedTheme {
        document.resolved(for: appearance)
    }

    var warnings: [String] {
        document.contrastWarnings()
    }

    func colorBinding(_ role: PhotaraThemeRole) -> Binding<Color> {
        Binding(
            get: { self.resolved.color(role) },
            set: { newColor in
                guard let converted = NSColor(newColor).usingColorSpace(.sRGB) else { return }
                let rgba = PhotaraRGBA(
                    red: converted.redComponent,
                    green: converted.greenComponent,
                    blue: converted.blueComponent,
                    alpha: converted.alphaComponent
                )
                self.document.setColor(
                    rgba.hex,
                    for: role,
                    appearance: self.appearance
                )
                self.hexDrafts[self.draftKey(role)] = rgba.hex
                self.message = "Modified \(role.rawValue)"
            }
        )
    }

    func hexBinding(_ role: PhotaraThemeRole) -> Binding<String> {
        Binding(
            get: {
                self.hexDrafts[self.draftKey(role)]
                    ?? self.document.mode(self.appearance).colors[role.rawValue]
                    ?? ""
            },
            set: { value in
                self.hexDrafts[self.draftKey(role)] = value
                guard PhotaraRGBA(hex: value) != nil else {
                    self.message = "Enter #RRGGBB or #RRGGBBAA"
                    return
                }
                self.document.setColor(value.uppercased(), for: role, appearance: self.appearance)
                self.message = "Modified \(role.rawValue)"
            }
        )
    }

    func open() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.json]
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            document = try PhotaraThemeDocument.load(from: url)
            hexDrafts.removeAll()
            documentURL = url
            message = "Loaded \(url.lastPathComponent)"
        } catch {
            message = error.localizedDescription
        }
    }

    func save() {
        guard let documentURL else {
            saveAs()
            return
        }
        write(to: documentURL)
    }

    func saveAs() {
        let panel = NSSavePanel()
        panel.allowedContentTypes = [.json]
        panel.nameFieldStringValue = "\(document.id).json"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        write(to: url)
    }

    private func write(to url: URL) {
        do {
            try document.write(to: url)
            documentURL = url
            message = "Saved \(url.lastPathComponent)"
        } catch {
            message = error.localizedDescription
        }
    }

    private func draftKey(_ role: PhotaraThemeRole) -> String {
        "\(appearance.rawValue).\(role.rawValue)"
    }
}

private extension PhotaraRGBA {
    init(red: Double, green: Double, blue: Double, alpha: Double) {
        self.red = red
        self.green = green
        self.blue = blue
        self.alpha = alpha
    }
}
