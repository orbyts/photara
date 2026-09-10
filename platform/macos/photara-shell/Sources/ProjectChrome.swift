import SwiftUI

struct ProjectStatusBar: View {
    let presentation: ApplicationPresentation
    var preset: ApplicationShellPreset = .shipped
    @EnvironmentObject private var workspace: WorkspaceModel
    @Environment(\.photaraTheme) private var theme
    var body: some View {
        HStack(spacing: 12) {
            if presentation.isDirty { Text("Unsaved changes") }
            if let sync = presentation.syncLabel { Text(sync).foregroundStyle(.secondary) }
            if presentation.diagnosticCount > 0 {
                Button { workspace.show(.diagnostics) } label: {
                    Label("\(presentation.diagnosticCount) diagnostics", systemImage: "exclamationmark.triangle")
                }.buttonStyle(.borderless)
            }
            Spacer(minLength: 8)
            if presentation.isEvaluating {
                ProgressView().controlSize(.mini)
                Text(presentation.progressLabel).lineLimit(1)
            } else if let context = presentation.surfaceContext {
                Text(context).foregroundStyle(.secondary).lineLimit(1)
            }
        }
        .font(.caption).padding(.horizontal, 12).frame(height: preset.statusBarHeight)
        .background(theme?.color(.surfaceElevated) ?? Color(nsColor: .windowBackgroundColor))
    }
}

struct PanelHeader: View {
    @EnvironmentObject private var workspace: WorkspaceModel
    @Environment(\.photaraTheme) private var theme
    let panel: WorkspacePanelID
    var height: Double = 34
    var allowsPlacement = true
    var title: String? = nil

    var body: some View {
        HStack(spacing: 7) {
            Text(title ?? panel.title)
                .font(.subheadline.weight(.semibold))
            Spacer()
            if allowsPlacement {
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
            }
            if allowsPlacement && panel != .graph && panel != .nodeWorkSurface {
            Button {
                workspace.toggle(panel)
            } label: {
                Image(systemName: "xmark")
                    .frame(width: 20, height: 20)
            }
            .buttonStyle(.borderless)
            .help("Hide \(panel.title)")
            }
        }
        .padding(.horizontal, 10)
        .frame(height: height)
        .background(theme?.color(.surfaceElevated) ?? Color(nsColor: .windowBackgroundColor))
    }
}
