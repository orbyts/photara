import SwiftUI

struct ThemeLabView: View {
    @EnvironmentObject private var model: ThemeLabModel
    @EnvironmentObject private var previewApp: AppModel
    @EnvironmentObject private var previewWorkspace: WorkspaceModel
    @State private var showsRoleDetails = true

    var body: some View {
        NavigationSplitView {
            slotEditor
                .navigationSplitViewColumnWidth(min: 300, ideal: 340, max: 420)
        } detail: {
            productionPreview
        }
        .toolbar {
            Picker("Appearance", selection: $model.appearance) {
                Text("Light").tag(PhotaraThemeAppearance.light)
                Text("Dark").tag(PhotaraThemeAppearance.dark)
            }
            .pickerStyle(.segmented)
            .frame(width: 160)
            Toggle(isOn: $showsRoleDetails) {
                Label("Mappings", systemImage: "info.circle")
            }
            .toggleStyle(.button)
            .help("Show the current production consumer for each semantic color role")
            Button("Open", systemImage: "folder") { model.open() }
            Button("Save", systemImage: "square.and.arrow.down") { model.save() }
        }
    }

    private var slotEditor: some View {
        VStack(spacing: 0) {
            VStack(alignment: .leading, spacing: 4) {
                Text(model.document.displayName)
                    .font(.headline)
                Text("\(model.document.id) · paired sRGB")
                    .font(.caption.monospaced())
                    .foregroundStyle(.secondary)
                if !model.message.isEmpty {
                    Text(model.message)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding()
            Divider()
            List {
                ForEach(slotGroups, id: \.name) { group in
                    Section(group.name) {
                        ForEach(group.roles) { role in
                            VStack(alignment: .leading, spacing: 4) {
                                HStack {
                                    ColorPicker("", selection: model.colorBinding(role))
                                        .labelsHidden()
                                    Text(role.rawValue)
                                        .font(.caption.monospaced())
                                    Spacer()
                                    TextField("#RRGGBB", text: model.hexBinding(role))
                                        .font(.caption.monospaced())
                                        .textFieldStyle(.plain)
                                        .frame(width: 76)
                                }
                                if showsRoleDetails {
                                    HStack(alignment: .firstTextBaseline, spacing: 5) {
                                        Text(role.themeLabConsumer)
                                            .font(.caption2)
                                            .foregroundStyle(.secondary)
                                            .fixedSize(horizontal: false, vertical: true)
                                        Spacer(minLength: 4)
                                        Text(role.themeLabCoverage.title)
                                            .font(.system(size: 8, weight: .semibold))
                                            .foregroundStyle(.secondary)
                                            .padding(.horizontal, 4)
                                            .padding(.vertical, 2)
                                            .background(.quaternary, in: Capsule())
                                    }
                                }
                            }
                        }
                    }
                }
                if !model.warnings.isEmpty {
                    Section("Contrast warnings") {
                        ForEach(model.warnings, id: \.self) { warning in
                            Label(warning, systemImage: "exclamationmark.triangle")
                                .font(.caption)
                        }
                    }
                }
            }
        }
    }

    private var productionPreview: some View {
        let theme = model.resolved
        return WorkspaceView()
            .environment(\.photaraTheme, theme)
            .tint(theme.color(.borderFocus))
            .preferredColorScheme(model.appearance == .dark ? .dark : .light)
            .task {
                prepareLayoutFixture()
            }
    }

    private func prepareLayoutFixture() {
        if !previewApp.hasOpenProject {
            previewApp.newProject()
        }
        if previewApp.snapshot?.nodes.contains(where: { $0.layout != nil }) != true,
           let definition = previewApp.nodeDefinitions.first(where: {
               $0.definitionId == "photara.layout.compose"
           })
        {
            previewApp.addNode(definition)
        }
        guard let layout = previewApp.snapshot?.nodes.first(where: { $0.layout != nil }) else {
            return
        }
        previewWorkspace.selectedNodeID = layout.nodeId
        previewWorkspace.selectedFrameID = layout.layout?.frames.first?.frameId
        previewWorkspace.selectedCellID = layout.layout?.frames.first?.cells.first?.cellId
    }

    private var slotGroups: [(name: String, roles: [PhotaraThemeRole])] {
        let order = [
            "surface", "text", "border", "selection", "graph", "gallery",
            "workspace", "status", "node",
        ]
        return order.map { prefix in
            (
                prefix.capitalized,
                PhotaraThemeRole.allCases.filter {
                    $0.rawValue.hasPrefix("\(prefix).")
                }
            )
        }
    }
}

private enum ThemeLabCoverage {
    case live
    case partial
    case reserved

    var title: String {
        switch self {
        case .live: "LIVE"
        case .partial: "PARTIAL"
        case .reserved: "RESERVED"
        }
    }
}

private extension PhotaraThemeRole {
    var themeLabConsumer: String {
        switch self {
        case .surfaceCanvas: "Application and workspace base behind panels"
        case .surfacePanel: "Inspector and ordinary workspace panel backgrounds"
        case .surfaceElevated: "Inspector groups, panel headers, and command/status bars"
        case .surfaceControl: "Control and passive input fills"
        case .textPrimary: "Primary labels and content text"
        case .textSecondary: "Secondary labels, metadata, and supporting text"
        case .textDisabled: "Unavailable actions and disabled text"
        case .borderSubtle: "Dividers and low-emphasis boundaries"
        case .borderStrong: "Strong boundaries and unselected Graph nodes"
        case .borderFocus: "Keyboard focus, active controls, and selected Gallery cells"
        case .selectionBackground: "Selected Gallery and list item fill"
        case .selectionForeground: "Content displayed on selection fill"
        case .graphBackground: "Graph canvas"
        case .graphGrid: "Graph canvas dot grid"
        case .graphNode: "Graph node Reduce Transparency fallback and overview body"
        case .graphNodeSelected: "Selected Graph node outline"
        case .galleryBackground: "Assets Gallery panel"
        case .galleryCell: "Square Gallery cells and unloaded thumbnail wells"
        case .workspaceSurround: "Neutral surround outside a node authoring canvas"
        case .statusTextNeutral: "Idle and neutral runtime status"
        case .statusTextRunning: "Running and progress status"
        case .statusTextSuccess: "Ready and successful status"
        case .statusTextWarning: "Warning and unresolved status"
        case .statusTextError: "Error and failed status"
        case .statusTextCancelled: "Cancelled runtime status"
        case .nodeNative: "Built-in native nodes such as Layout and Disk"
        case .nodeIO: "Future input/output provider nodes"
        case .nodeTransform: "Future transformation nodes"
        case .nodeCreative: "Future creative-authoring nodes"
        case .nodeAutomation: "Future automation and scripting nodes"
        case .nodeIntegration: "Future external application and cloud integrations"
        case .nodeCompute: "Future compute and ML nodes"
        }
    }

    var themeLabCoverage: ThemeLabCoverage {
        switch self {
        case .surfaceCanvas, .surfacePanel, .surfaceElevated,
             .borderFocus, .selectionBackground, .selectionForeground,
             .graphBackground, .graphGrid, .graphNodeSelected,
             .galleryBackground, .galleryCell, .statusTextSuccess,
             .statusTextWarning, .statusTextError, .nodeNative:
            .live
        case .textPrimary, .textSecondary, .textDisabled, .borderSubtle,
             .borderStrong, .graphNode, .statusTextNeutral, .statusTextRunning,
             .statusTextCancelled:
            .partial
        case .workspaceSurround:
            .live
        case .surfaceControl, .nodeIO, .nodeTransform, .nodeCreative,
             .nodeAutomation, .nodeIntegration, .nodeCompute:
            .reserved
        }
    }
}
