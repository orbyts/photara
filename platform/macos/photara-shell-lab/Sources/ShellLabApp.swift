import AppKit
import SwiftUI

@main
struct ShellLabApp: App {
    @StateObject private var model = ShellLabModel(persistsDraft: true)
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
                Picker("Title font", selection: $model.preset.launcherTitleFont) {
                    ForEach(ApplicationShellPreset.TitleFontFamily.allCases, id: \.self) {
                        Text($0.macOSLabel).tag($0)
                    }
                }
                Picker("Title weight", selection: $model.preset.launcherTitleWeight) {
                    ForEach(ApplicationShellPreset.TitleWeight.allCases, id: \.self) { Text($0.rawValue).tag($0) }
                }
                slider("Hero-to-recents gap", \.launcherSpacing, 12...48)
                slider("Hero spacing", \.heroSpacing, 12...36)
                slider("Horizontal edge inset", \.contentHorizontalInset, 16...120)
                slider("Vertical edge inset", \.contentInset, 16...120)
                slider("Opening vertical position", \.launcherVerticalOffset, -240...240)
            }
            Section("Launcher background") {
                Picker("Material", selection: $model.preset.launcherBackgroundStyle) {
                    ForEach(ApplicationShellPreset.LauncherBackgroundStyle.allCases, id: \.self) {
                        Text($0.label).tag($0)
                    }
                }
                adaptiveColorPicker("Tint", \.launcherBackgroundTint)
                Text("Tint opacity controls how strongly the selected material is colored.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Section("Hero icon tile") {
                Toggle("Show tile", isOn: $model.preset.heroShowsTile)
                slider("Tile size", \.heroSize, 104...160)
                slider("Symbol size", \.heroIconSize, 32...88)
                slider("Symbol vertical offset", \.heroSymbolOffsetY, -40...40)
                slider("Tile horizontal position", \.heroTileOffsetX, -160...160)
                slider("Tile vertical position", \.heroTileOffsetY, -160...160)
                adaptiveColorPicker("Symbol color", \.heroSymbolColor)
                adaptiveColorPicker("Background color", \.heroTileBackgroundColor)
                adaptiveColorPicker("Stroke color", \.heroTileStrokeColor)
                slider("Stroke width", \.heroTileStrokeWidth, 0...6)
                slider("Corner radius", \.heroTileCornerRadius, 0...52)
                adaptiveColorPicker("Glow color", \.heroGlowColor)
                slider("Glow radius", \.heroGlowRadius, 0...80)
                slider("Glow horizontal offset", \.heroGlowOffsetX, -40...40)
                slider("Glow vertical offset", \.heroGlowOffsetY, -40...40)
            }
            Section("Launcher buttons") {
                adaptiveColorPicker("Create Project tint", \.launcherCreateButtonTint)
                adaptiveColorPicker("Open Project tint", \.launcherOpenButtonTint)
                adaptiveColorPicker("Recent Projects tint", \.launcherRecentButtonTint)
                Text("Colors edit the currently selected Light or Dark appearance.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Section("Application surfaces") {
                Toggle("Use semantic Theme fills", isOn: $model.preset.frame.usesThemeFills)
                Picker("Canvas material", selection: $model.preset.frame.canvasMaterial) {
                    ForEach(ApplicationShellPreset.LauncherBackgroundStyle.allCases, id: \.self) { Text($0.label).tag($0) }
                }
                adaptiveColorPicker("Canvas fill", \.frame.canvasFill)
                adaptiveColorPicker("Module fill", \.frame.surfaceFill)
                slider("Gutter", \.frame.gutter, 4...32)
                slider("Outer inset", \.frame.outerInset, 0...40)
                slider("Content inset", \.frame.contentInset, 0...24)
                slider("Global module corner radius", \.frame.cornerRadius, 0...40)
                slider("Optional border", \.frame.borderWidth, 0...3)
                slider("Elevation", \.frame.elevation, 0...16)
                slider("Active emphasis", \.frame.activeEmphasis, 0...4)
                Toggle("Elevated header", isOn: $model.preset.frame.elevatedHeader)
                Toggle("Separate status surface", isOn: $model.preset.frame.separateStatusSurface)
                slider("Compact breakpoint", \.frame.compactBreakpoint, 800...1400)
            }
            Section("Workspace chrome") {
                TextField("Fixture project name", text: $model.fixtureProjectTitle)
                Toggle("Show project title", isOn: $model.preset.toolbarShowsProjectTitle)
                Picker("Project title weight", selection: $model.preset.toolbarTitleWeight) {
                    ForEach(ApplicationShellPreset.TitleWeight.allCases, id: \.self) { Text($0.rawValue).tag($0) }
                }
                slider("Project title size", \.toolbarTitleSize, 10...24)
                slider("Project title width", \.toolbarIdentityWidth, 120...320)
                slider("Leading pane", \.leadingIdealWidth, 230...360)
                slider("Trailing pane", \.trailingIdealWidth, 280...440)
                slider("Panel header height", \.panelHeaderHeight, 28...60)
                slider("Panel header title size", \.panelHeaderTitleSize, 10...22)
                slider("Panel header horizontal inset", \.panelHeaderHorizontalInset, 4...32)
                slider("Frame divider thickness", \.dividerThickness, 0...4)
                slider("Status bar height", \.statusBarHeight, 20...52)
                slider("Status text size", \.statusTextSize, 9...20)
                slider("Status horizontal inset", \.statusHorizontalInset, 4...32)
                slider("Status item spacing", \.statusItemSpacing, 4...28)
            }
            Section("Project chrome colors") {
                themeColorPicker("Workspace frame", .surfaceCanvas)
                themeColorPicker("Panel background", .surfacePanel)
                themeColorPicker("Header and status background", .surfaceElevated)
                themeColorPicker("Frame and dividers", .borderSubtle)
                themeColorPicker("Primary text", .textPrimary)
                themeColorPicker("Secondary text", .textSecondary)
                themeColorPicker("Status neutral", .statusTextNeutral)
                themeColorPicker("Status running", .statusTextRunning)
                themeColorPicker("Status success", .statusTextSuccess)
                themeColorPicker("Status warning", .statusTextWarning)
                themeColorPicker("Status error", .statusTextError)
                Text("These are shared Theme roles, so the same colors flow into every module that consumes them.")
                    .font(.caption).foregroundStyle(.secondary)
            }
            Section("Handoff") {
                Button("Apply to Photara") { model.applyPresetToPhotara() }
                Button("Remove Photara Override") { model.removePhotaraOverride() }
                Button("Export Shell Preset…") {
                    do { try LabPresetExport.save(model.preset.encoded(), filename: "photara-application-presentation-v1.json") }
                    catch { self.error = error.localizedDescription }
                }
                Button("Export Shared Theme…") {
                    do {
                        try model.themeDocument.validate()
                        let encoder = JSONEncoder(); encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
                        try LabPresetExport.save(encoder.encode(model.themeDocument), filename: "photara-default.json")
                    } catch { self.error = error.localizedDescription }
                }
                Button("Restore Shipped Preset") { model.restoreShippedPreset() }
                Text("Lab edits are saved automatically as a local draft.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Text(model.lastAction).font(.caption).foregroundStyle(.secondary)
            }
        }.formStyle(.grouped).frame(minWidth: 340, minHeight: 560)
        .onChange(of: model.scenario) { if model.scenario == .compact { resize(width: 820, height: 720) } }
        .alert("Export failed", isPresented: Binding(get: { error != nil }, set: { if !$0 { error = nil } })) {
            Button("OK") { error = nil }
        } message: { Text(error ?? "") }
    }
    private func themeColorPicker(_ title: String, _ role: PhotaraThemeRole) -> some View {
        let appearance: PhotaraThemeAppearance = model.dark ? .dark : .light
        return ColorPicker(title, selection: Binding(
            get: { model.themeDocument.resolved(for: appearance).color(role) },
            set: { newColor in
                guard let converted = NSColor(newColor).usingColorSpace(.sRGB) else { return }
                let rgba = PhotaraRGBA(red: converted.redComponent, green: converted.greenComponent,
                    blue: converted.blueComponent, alpha: converted.alphaComponent)
                model.themeDocument.setColor(rgba.hex, for: role, appearance: appearance)
            }
        ), supportsOpacity: true)
    }
    private func slider(_ title: String, _ key: WritableKeyPath<ApplicationShellPreset, Double>, _ range: ClosedRange<Double>) -> some View {
        VStack(alignment: .leading) {
            Text("\(title) · \(Int(model.preset[keyPath: key]))").font(.caption)
            Slider(value: Binding(get: { model.preset[keyPath: key] }, set: { model.preset[keyPath: key] = $0 }), in: range, step: 1)
                .accessibilityLabel(title)
        }
    }
    private func adaptiveColorPicker(
        _ title: String,
        _ key: WritableKeyPath<ApplicationShellPreset, ApplicationShellPreset.AdaptiveColor>
    ) -> some View {
        let scheme: ColorScheme = model.dark ? .dark : .light
        return ColorPicker(title, selection: Binding(
            get: { model.preset[keyPath: key].color(scheme) },
            set: { newColor in
                guard let converted = NSColor(newColor).usingColorSpace(.sRGB) else { return }
                let rgba = PhotaraRGBA(
                    red: converted.redComponent,
                    green: converted.greenComponent,
                    blue: converted.blueComponent,
                    alpha: converted.alphaComponent
                )
                var adaptive = model.preset[keyPath: key]
                adaptive.set(rgba.hex, for: scheme)
                model.preset[keyPath: key] = adaptive
            }
        ), supportsOpacity: true)
    }
    private func resize(width: Double, height: Double) {
        guard let window = NSApp.windows.first(where: { $0.title == "Photara — Shell Preview" }) else { return }
        window.setContentSize(.init(width: width, height: height))
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
