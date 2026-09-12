import SwiftUI

struct ProjectStatusBar: View {
    let presentation: ApplicationPresentation
    var preset: ApplicationShellPreset = .shipped
    @EnvironmentObject private var workspace: WorkspaceModel
    @Environment(\.photaraTheme) private var theme
    var body: some View {
        HStack(spacing: preset.statusItemSpacing) {
            if presentation.isDirty {
                Text("Unsaved changes").foregroundStyle(theme?.color(.statusTextWarning) ?? Color.orange)
            }
            if let sync = presentation.syncLabel {
                Text(sync).foregroundStyle(sync.localizedCaseInsensitiveContains("sync")
                    ? (theme?.color(.statusTextRunning) ?? Color.blue)
                    : (theme?.color(.statusTextSuccess) ?? Color.green))
            }
            if presentation.diagnosticCount > 0 {
                Button { workspace.show(.diagnostics) } label: {
                    Label("\(presentation.diagnosticCount) diagnostics", systemImage: "exclamationmark.triangle")
                }.buttonStyle(.borderless)
            }
            Spacer(minLength: 8)
            if presentation.isEvaluating {
                ProgressView().controlSize(.mini)
                Text(presentation.progressLabel)
                    .foregroundStyle(theme?.color(.statusTextRunning) ?? Color.blue)
                    .lineLimit(1)
            } else if let context = presentation.surfaceContext {
                Text(context).foregroundStyle(theme?.color(.statusTextNeutral) ?? Color.secondary).lineLimit(1)
            }
        }
        .font(.system(size: preset.statusTextSize))
        .foregroundStyle(theme?.color(.statusTextNeutral) ?? Color.secondary)
        .padding(.horizontal, preset.statusHorizontalInset).frame(height: preset.statusBarHeight)
        .background(theme?.color(.surfaceElevated) ?? Color(nsColor: .windowBackgroundColor))
    }
}

struct PanelHeader: View {
    @EnvironmentObject private var workspace: WorkspaceModel
    @Environment(\.photaraTheme) private var theme
    let panel: WorkspacePanelID
    var height: Double = 34
    var titleSize: Double = 13
    var horizontalPadding: Double = 10
    var elevated = true
    var activeFill: Color? = nil
    var allowsPlacement = true
    var title: String? = nil

    var body: some View {
        HStack(spacing: 7) {
            Label(title ?? panel.title, systemImage: panel.symbol)
                .font(.system(size: titleSize, weight: .semibold))
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
            if allowsPlacement {
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
        .padding(.horizontal, horizontalPadding)
        .frame(height: height)
        .background {
            if let activeFill {
                activeFill
            } else if elevated {
                theme?.color(.surfaceElevated) ?? Color(nsColor: .windowBackgroundColor)
            } else {
                Color.clear
            }
        }
        .contentShape(Rectangle())
        .onTapGesture { workspace.focusedPanel = panel }
    }
}
