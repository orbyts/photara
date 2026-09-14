import SwiftUI

struct ThemeLabView: View {
  @EnvironmentObject private var model: ThemeLabModel
  @State private var showsOpening = false
  @StateObject private var session = EditorSessionModel(persists: false)

  var body: some View {
    NavigationSplitView {
      editor.navigationSplitViewColumnWidth(min: 420, ideal: 460, max: 560)
    } detail: {
      if showsOpening {
        ApplicationShell(
          presentation: .init(
            hasOpenProject: false, title: "", subtitle: "",
            isDirty: false, nodeCount: 0, diagnosticCount: 0, progressLabel: "", isEvaluating: false
          ),
          actions: .init(send: { model.message = "Preview action: \($0)" }),
          workSurface: { _ in AnyView(EmptyView()) }, panel: { _ in EmptyView() }
        )
        .environmentObject(session)
        .environment(\.photaraTheme, model.resolved)
        .preferredColorScheme(model.appearance == .dark ? .dark : .light)
      } else {
        ThemeLadderSpecimen(document: model.document)
      }
    }
    .toolbar {
      Toggle("Production Opening", isOn: $showsOpening).toggleStyle(.button)
      if showsOpening {
        Picker("Preview appearance", selection: $model.appearance) {
          ForEach(PhotaraThemeAppearance.allCases) { Text($0.rawValue.capitalized).tag($0) }
        }.pickerStyle(.segmented).frame(width: 150)
      }
      Button("Open", systemImage: "folder") { model.open() }
      Button("Save", systemImage: "square.and.arrow.down") { model.save() }
    }
  }

  private var editor: some View {
    Form {
      Section("Shared adaptive palette") {
        Text(model.document.displayName).font(.headline)
        Text(
          "Author each role once, with paired Light and Dark values. Native macOS materials and controls remain system-owned."
        )
        .font(.caption).foregroundStyle(.secondary)
      }
      ForEach(groups, id: \.name) { group in
        Section(group.name) {
          ForEach(group.roles) { role in
            VStack(alignment: .leading, spacing: 6) {
              Text(role.authoringLabel).font(.callout)
              HStack(spacing: 16) {
                ForEach(PhotaraThemeAppearance.allCases) { appearance in
                  VStack(alignment: .leading, spacing: 4) {
                    Text(appearance.rawValue.capitalized).font(.caption).foregroundStyle(.secondary)
                    HStack(spacing: 4) {
                      ColorPicker(
                        role.authoringLabel + " " + appearance.rawValue,
                        selection: model.colorBinding(role, appearance: appearance),
                        supportsOpacity: false
                      )
                      .labelsHidden()
                      TextField("#RRGGBB", text: model.hexBinding(role, appearance: appearance))
                        .font(.caption.monospaced())
                        .accessibilityLabel(
                          role.authoringLabel + " " + appearance.rawValue + " hex")
                    }
                  }
                }
              }
            }
          }
        }
      }
      Section("Inherited compatibility roles") {
        ForEach(PhotaraThemeRole.aliases) { role in
          LabeledContent(role.rawValue, value: role.canonical.authoringLabel)
            .font(.caption)
        }
      }
      Section("Validation") {
        if model.warnings.isEmpty {
          Label("Neutral ladder and text contrast pass", systemImage: "checkmark.circle")
        } else {
          ForEach(model.warnings, id: \.self) { Text($0).font(.caption) }
        }
      }
      Section("Development preview") {
        Button("Apply Theme to Photara and Labs") { model.apply() }
        Button("Remove Theme Override") { model.removeOverride() }
        Text(model.message).font(.caption).foregroundStyle(.secondary)
      }
    }.formStyle(.grouped)
  }

  private var groups: [(name: String, roles: [PhotaraThemeRole])] {
    [
      ("Neutral ladder", PhotaraSurfaceLevel.allCases.map(\.role)),
      ("Photograph reference", [.editorSurround]),
      ("Text", [.textPrimary, .textSecondary, .textDisabled]),
      ("Borders and custom focus", [.borderSubtle, .borderStrong, .borderFocus]),
      ("Custom selection", [.selectionBackground, .selectionForeground]),
      ("Semantic status", PhotaraThemeRole.authored.filter { $0.rawValue.hasPrefix("status.") }),
      ("Category affordances", PhotaraThemeRole.authored.filter { $0.rawValue.hasPrefix("node.") }),
    ]
  }
}
