import AppKit
import SwiftUI

@main
struct ShellLabApp: App {
  @StateObject private var model = ShellLabModel(persistsDraft: true)
  var body: some Scene {
    Window("Photara — Shell Preview", id: "preview") {
      PreviewWindow(model: model)
    }
    .windowToolbarStyle(.unified)
    .defaultSize(width: 1440, height: 900)
    Window("Shell Authoring", id: "controls") {
      ShellControls(model: model)
    }.defaultSize(width: 360, height: 760)
  }
}
private struct PreviewWindow: View {
  @ObservedObject var model: ShellLabModel
  @Environment(\.openWindow) private var openWindow
  var body: some View {
    ShellLabPreview(model: model, session: model.session, usesDevelopmentTheme: true)
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
          ForEach(ShellScenario.browserCases) { Text($0.title).tag($0) }
        }
        Toggle("Dark appearance", isOn: $model.dark)
        Toggle(isOn: $model.identifiesControls) {
          Label("Identify controls in preview", systemImage: "scope")
        }
        if model.identifiesControls {
          Text(
            "Move the crosshair over the preview to see which authoring controls own that area. Preview interaction is paused while identifying."
          )
          .font(.caption)
          .foregroundStyle(.secondary)
        }
        HStack {
          Button("Compact") { resize(width: 820, height: 720) }
          Button("Standard") { resize(width: 1440, height: 900) }
          Button("Wide") { resize(width: 1720, height: 1000) }
        }
        Button("Open Preview") { openWindow(id: "preview") }
      }
      if model.scenario == .libraryLifecycle {
        LifecycleControls(model: model.lifecycle)
      }
      if model.scenario.isOpening {
        Section("Opening") {
          Text(
            "Production Opening uses native window and text colors. macOS owns the sidebar, selection, controls and toolbar optics."
          )
          .font(.caption).foregroundStyle(.secondary)
          Text(model.scenario == .createProject
            ? "UI1 preview uses the shared Create Project sheet. It records fixture actions only and performs no package or database writes."
            : "Create Project is authored in its UI1 scenario; Project Browser awaits UI2.")
          .font(.caption).foregroundStyle(.secondary)
          if model.scenario == .createProject {
            Picker("Create Project style", selection: $model.createProjectPresentation) {
              ForEach(CreateProjectPresentation.allCases) { style in
                Text(style.title).tag(style)
              }
            }
            .pickerStyle(.segmented)
            Text("Compact is the shipped default. Balanced adds package guidance; Spacious also previews the project structure being created.")
              .font(.caption).foregroundStyle(.secondary)
          }
        }
      }
      if model.scenario.authorsModuleGeometry {
        Section("Shared geometry") {
          slider("Gutter", \.frame.gutter, 4...32)
          slider("Outer inset", \.frame.outerInset, 0...40)
          slider("Content inset", \.frame.contentInset, 0...24)
          slider("Module corner radius", \.frame.cornerRadius, 0...40)
          Text("Shared dimensions belong to every module. Edit the inherited palette in Theme Lab.")
            .font(.caption).foregroundStyle(.secondary)
        }
        Section("Editor chrome") {
          TextField("Fixture project name", text: $model.fixtureProjectTitle)
          TextField("Centered application name", text: $model.preset.toolbarApplicationTitle)
          slider("Application name size", \.toolbarApplicationTitleSize, 10...24)
          Toggle("Show project title", isOn: $model.preset.toolbarShowsProjectTitle)
          Toggle("Show project thumbnail", isOn: $model.preset.toolbarShowsProjectThumbnail)
          if model.preset.toolbarShowsProjectThumbnail {
            slider("Project thumbnail size", \.toolbarProjectThumbnailSize, 18...40)
            slider("Project thumbnail corner radius", \.toolbarProjectThumbnailCornerRadius, 0...16)
          }
          Picker("Project title weight", selection: $model.preset.toolbarTitleWeight) {
            ForEach(ApplicationShellPreset.TitleWeight.allCases, id: \.self) {
              Text($0.rawValue).tag($0)
            }
          }
          slider("Project title size", \.toolbarTitleSize, 10...24)
          slider("Project title width", \.toolbarIdentityWidth, 120...320)
          Text(
            "macOS owns toolbar glass, adaptive blur, shadow and control grouping. Photara supplies semantic groups only."
          )
          .font(.caption).foregroundStyle(.secondary)
        }
        Section("Advanced shared geometry") {
          DisclosureGroup("Pane, header and status metrics") {
            slider("Leading pane", \.leadingIdealWidth, 230...360)
            slider("Trailing pane", \.trailingIdealWidth, 280...440)
            slider("Module title bar height", \.panelHeaderHeight, 28...72)
            slider("Panel header title size", \.panelHeaderTitleSize, 10...22)
            slider("Panel header horizontal inset", \.panelHeaderHorizontalInset, 4...32)
            slider("Elevation", \.frame.elevation, 0...16)
            Toggle("Separate status surface", isOn: $model.preset.frame.separateStatusSurface)
            slider("Status bar height", \.statusBarHeight, 20...52)
            slider("Status text size", \.statusTextSize, 9...20)
            slider("Status horizontal inset", \.statusHorizontalInset, 4...32)
            slider("Status item spacing", \.statusItemSpacing, 4...28)
            slider("Compact breakpoint", \.frame.compactBreakpoint, 800...1400)
          }
        }
      }
      Section("Handoff") {
        Button("Apply to Photara") { model.applyPresetToPhotara() }
        Button("Remove Photara Override") { model.removePhotaraOverride() }
        Button("Export Shell Preset…") {
          do {
            try LabPresetExport.save(
              model.preset.encoded(), filename: "photara-application-presentation-v1.json")
          } catch { self.error = error.localizedDescription }
        }
        Button("Restore Shipped Preset") { model.restoreShippedPreset() }
        Text("Lab edits are saved automatically as a local draft.")
          .font(.caption)
          .foregroundStyle(.secondary)
        Text(model.lastAction).font(.caption).foregroundStyle(.secondary)
      }
    }.formStyle(.grouped).frame(minWidth: 340, minHeight: 560)
      .onChange(of: model.scenario) {
        if model.scenario == .compact { resize(width: 820, height: 720) }
      }
      .alert(
        "Export failed",
        isPresented: Binding(get: { error != nil }, set: { if !$0 { error = nil } })
      ) {
        Button("OK") { error = nil }
      } message: {
        Text(error ?? "")
      }
  }
  private func slider(
    _ title: String, _ key: WritableKeyPath<ApplicationShellPreset, Double>,
    _ range: ClosedRange<Double>
  ) -> some View {
    slider(title, key, range, step: 1)
  }
  private func slider(
    _ title: String,
    _ key: WritableKeyPath<ApplicationShellPreset, Double>,
    _ range: ClosedRange<Double>,
    step: Double
  ) -> some View {
    VStack(alignment: .leading) {
      Text(
        "\(title) · \(model.preset[keyPath: key].formatted(.number.precision(.fractionLength(step < 1 ? 2 : 0))))"
      )
      .font(.caption)
      Slider(
        value: Binding(
          get: { model.preset[keyPath: key] }, set: { model.preset[keyPath: key] = $0 }), in: range,
        step: step
      )
      .accessibilityLabel(title)
    }
  }
  private func resize(width: Double, height: Double) {
    guard let window = NSApp.windows.first(where: { $0.title == "Photara — Shell Preview" }) else {
      return
    }
    window.setContentSize(.init(width: width, height: height))
  }
}

private struct LifecycleControls: View {
    @ObservedObject var model: LibraryLifecycleFixture
    var body: some View {
        Section("Library Lifecycle · review only") {
            Picker("State", selection: Binding(get: { model.scenario }, set: { model.show($0) })) {
                ForEach(LibraryLifecycleScenario.allCases) { Text($0.title).tag($0) }
            }
            Picker("Switcher context", selection: Binding(get: { model.fixtureCase }, set: { model.configure($0) })) {
                ForEach(LibrarySwitcherFixtureCase.allCases) { Text($0.title).tag($0) }
            }
            Text("Synthetic catalog only. No database, network, credentials or package operations. Use Continue to review the native final destructive dialog.")
                .font(.caption).foregroundStyle(.secondary)
        }
    }
}
