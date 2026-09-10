import AppKit
import SwiftUI

@main
struct ShellLabApp: App {
    @StateObject private var model = ShellLabModel()
    var body: some Scene {
        Window("Photara — Shell Preview", id: "preview") {
            PreviewWindow(model: model)
        }.defaultSize(width: 1440, height: 900)
        Window("Shell Authoring", id: "controls") {
            ShellControls(model: model)
        }.defaultSize(width: 360, height: 760)
    }
}
private struct PreviewWindow: View {
    @ObservedObject var model: ShellLabModel
    @Environment(\.openWindow) private var openWindow
    var body: some View {
        ShellLabPreview(model: model, workspace: model.workspace)
            .onAppear { openWindow(id: "controls") }
    }
}
private struct ShellControls: View {
    @ObservedObject var model: ShellLabModel
    @Environment(\.openWindow) private var openWindow
    @State private var error: String?
    var body: some View {
        Form {
            Section("Preview") {
                Picker("Scenario", selection: $model.scenario) {
                    ForEach(ShellScenario.allCases) { Text($0.rawValue).tag($0) }
                }
                Toggle("Dark appearance", isOn: $model.dark)
                HStack {
                    Button("Compact") { resize(width: 820, height: 720) }
                    Button("Standard") { resize(width: 1440, height: 900) }
                    Button("Wide") { resize(width: 1720, height: 1000) }
                }
                Button("Open Preview") { openWindow(id: "preview") }
            }
            Section("Launcher") {
                slider("Title size", \.launcherTitleSize, 24...56)
                Picker("Title weight", selection: $model.preset.launcherTitleWeight) {
                    ForEach(ApplicationShellPreset.TitleWeight.allCases, id: \.self) { Text($0.rawValue).tag($0) }
                }
                slider("Hero size", \.heroSize, 104...160)
                slider("Icon size", \.heroIconSize, 32...88)
                slider("Launcher spacing", \.launcherSpacing, 12...48)
                slider("Hero spacing", \.heroSpacing, 12...36)
                slider("Content inset", \.contentInset, 16...44)
            }
            Section("Workspace chrome") {
                slider("Leading pane", \.leadingIdealWidth, 230...360)
                slider("Trailing pane", \.trailingIdealWidth, 280...440)
                slider("Panel header", \.panelHeaderHeight, 28...44)
                slider("Status bar", \.statusBarHeight, 24...36)
                slider("Toolbar identity", \.toolbarIdentityWidth, 120...240)
            }
            Section("Handoff") {
                Button("Export Shell Preset…") {
                    do { try LabPresetExport.save(model.preset.encoded(), filename: "photara-application-presentation-v1.json") }
                    catch { self.error = error.localizedDescription }
                }
                Button("Restore Shipped Preset") { model.preset = .shipped }
                Text(model.lastAction).font(.caption).foregroundStyle(.secondary)
            }
        }.formStyle(.grouped).frame(minWidth: 340, minHeight: 560)
        .onChange(of: model.scenario) { if model.scenario == .compact { resize(width: 820, height: 720) } }
        .alert("Export failed", isPresented: Binding(get: { error != nil }, set: { if !$0 { error = nil } })) {
            Button("OK") { error = nil }
        } message: { Text(error ?? "") }
    }
    private func slider(_ title: String, _ key: WritableKeyPath<ApplicationShellPreset, Double>, _ range: ClosedRange<Double>) -> some View {
        VStack(alignment: .leading) {
            Text("\(title) · \(Int(model.preset[keyPath: key]))").font(.caption)
            Slider(value: Binding(get: { model.preset[keyPath: key] }, set: { model.preset[keyPath: key] = $0 }), in: range, step: 1)
                .accessibilityLabel(title)
        }
    }
    private func resize(width: Double, height: Double) {
        guard let window = NSApp.windows.first(where: { $0.title == "Photara — Shell Preview" }) else { return }
        window.setContentSize(.init(width: width, height: height))
    }
}
