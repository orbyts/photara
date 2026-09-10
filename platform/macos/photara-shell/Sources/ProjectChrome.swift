import SwiftUI

struct ProjectCommandBar: View {
    let presentation: ApplicationPresentation
    let actions: ApplicationActions
    @EnvironmentObject private var workspace: WorkspaceModel
    @Environment(\.photaraTheme) private var theme

    private var projectInitials: String {
        let words = (presentation.title)
            .split(separator: " ")
            .prefix(2)
        return words.compactMap(\.first).map(String.init).joined().uppercased()
    }

    var body: some View {
        HStack(spacing: 12) {
            HStack(spacing: 9) {
                Text(projectInitials)
                    .font(.caption.weight(.bold))
                    .foregroundStyle(.white)
                    .frame(width: 32, height: 32)
                    .background(.tint, in: RoundedRectangle(cornerRadius: 7))
                VStack(alignment: .leading, spacing: 1) {
                    HStack(spacing: 4) {
                        Text(presentation.title)
                            .font(.subheadline.weight(.semibold))
                        if presentation.isDirty {
                            Circle()
                                .fill(.orange)
                                .frame(width: 5, height: 5)
                                .help("Unsaved changes")
                        }
                    }
                    Text(projectSubtitle)
                        .font(.caption2.monospaced())
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
            }

            Divider().frame(height: 28)

            Button("New", systemImage: "plus") { actions.send(.newProject) }
                .buttonStyle(.bordered)
            Button("Open", systemImage: "folder") { actions.send(.openProject) }
                .buttonStyle(.bordered)

            Spacer(minLength: 12)

            HStack(spacing: 2) {
                modeButton(.graph) { workspace.activateGraph() }
                modeButton(.layout) {
                    if let nodeID = selectedLayoutNodeID {
                        workspace.selectedNodeID = nodeID
                        workspace.activateWorkspace(for: nodeID)
                    }
                }
                modeButton(.review) { workspace.activateReview() }
                    .disabled(true)
                    .help("Review workspace is planned after Layout authoring")
            }
            .padding(3)
            .background(.quaternary.opacity(0.55), in: RoundedRectangle(cornerRadius: 8))

            Spacer(minLength: 12)

            Button("Run", systemImage: "play.fill") { actions.send(.evaluate) }
                .buttonStyle(.borderedProminent)
                .disabled(presentation.isEvaluating)
            Button("Save", systemImage: "square.and.arrow.down") { actions.send(.save) }
                .buttonStyle(.bordered)
        }
        .controlSize(.small)
        .padding(.horizontal, 12)
        .frame(height: 54)
        .background(theme?.color(.surfaceElevated) ?? Color(nsColor: .windowBackgroundColor))
    }

    private var selectedLayoutNodeID: String? { presentation.layoutNodeID }
    private var projectSubtitle: String { presentation.subtitle }

    private func modeButton(
        _ mode: WorkspaceMode,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            Label(mode.title, systemImage: mode.symbol)
                .font(.caption.weight(workspace.mode == mode ? .semibold : .regular))
                .padding(.horizontal, 11)
                .frame(height: 26)
                .background {
                    if workspace.mode == mode {
                        RoundedRectangle(cornerRadius: 6)
                            .fill(Color.accentColor.opacity(0.18))
                    }
                }
        }
        .buttonStyle(.plain)
        .foregroundStyle(workspace.mode == mode ? Color.accentColor : Color.secondary)
    }
}

struct ProjectStatusBar: View {
    let presentation: ApplicationPresentation
    let actions: ApplicationActions
    @Environment(\.photaraTheme) private var theme

    var body: some View {
        HStack(spacing: 9) {
            Circle()
                .fill(presentation.hasOpenProject ? Color.green : Color.secondary)
                .frame(width: 7, height: 7)
            Text("Project Loaded")
            Divider().frame(height: 15)
            Text(presentation.title)
                .foregroundStyle(.secondary)
            Divider().frame(height: 15)
            Text("\(presentation.nodeCount) nodes")
            Text(diagnosticSummary)
                .foregroundStyle(hasErrors ? Color.orange : Color.secondary)
            Spacer()
            if presentation.isDirty {
                Text("Unsaved")
                    .foregroundStyle(.orange)
            }
            Divider().frame(height: 15)
            Text(presentation.progressLabel)
                .foregroundStyle(.secondary)
        }
        .font(.caption)
        .padding(.horizontal, 12)
        .frame(height: 28)
        .background(theme?.color(.surfaceElevated) ?? Color(nsColor: .windowBackgroundColor))
    }

    private var hasErrors: Bool {
        presentation.diagnosticCount > 0
    }

    private var diagnosticSummary: String {
        let count = presentation.diagnosticCount
        return count == 0 ? "No errors" : "\(count) diagnostic\(count == 1 ? "" : "s")"
    }
}

struct PanelHeader: View {
    @EnvironmentObject private var workspace: WorkspaceModel
    @Environment(\.photaraTheme) private var theme
    let panel: WorkspacePanelID

    var body: some View {
        HStack(spacing: 7) {
            Text(panel.title)
                .font(.subheadline.weight(.semibold))
            Spacer()
            Menu {
                ForEach(WorkspaceRegion.allCases, id: \.self) { region in
                    Button(region.rawValue.capitalized) {
                        workspace.move(panel, to: region)
                    }
                }
            } label: {
                Image(systemName: "pin")
                    .frame(width: 20, height: 20)
            }
            .menuStyle(.borderlessButton)
            .help("Move \(panel.title)")
            Button {
                workspace.toggle(panel)
            } label: {
                Image(systemName: "xmark")
                    .frame(width: 20, height: 20)
            }
            .buttonStyle(.borderless)
            .help("Hide \(panel.title)")
        }
        .padding(.horizontal, 10)
        .frame(height: 34)
        .background(theme?.color(.surfaceElevated) ?? Color(nsColor: .windowBackgroundColor))
    }
}
