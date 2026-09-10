import AppKit
import SwiftUI

struct ApplicationShell<Panel: View>: View {
    let presentation: ApplicationPresentation
    let actions: ApplicationActions
    @EnvironmentObject private var workspace: WorkspaceModel
    @Environment(\.photaraTheme) private var theme
    @ViewBuilder var panel: (WorkspacePanelID) -> Panel

    var body: some View {
        Group {
            if presentation.hasOpenProject {
                VStack(spacing: 0) {
                    ProjectCommandBar(presentation: presentation, actions: actions)
                    Divider()
                    HSplitView {
                        region(.leading, minimumWidth: 230, idealWidth: 280)
                        region(.content, minimumWidth: 420, idealWidth: 680)
                        region(.trailing, minimumWidth: 280, idealWidth: 360)
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    Divider()
                    ProjectStatusBar(presentation: presentation, actions: actions)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                ProjectLauncherView(presentation: presentation, actions: actions)
            }
        }
        .frame(minWidth: 980, minHeight: 620)
        .background(theme?.color(.surfaceCanvas) ?? Color(nsColor: .windowBackgroundColor))
        .toolbar {
            ToolbarItemGroup {
                Button("New", systemImage: "doc.badge.plus") { actions.send(.newProject) }
                Button("Open", systemImage: "folder") { actions.send(.openProject) }
                Button("Close", systemImage: "xmark.square") { actions.send(.closeProject) }
                    .disabled(!presentation.hasOpenProject)
                Button("Import Pair", systemImage: "photo.badge.plus") {
                    actions.send(.importPair)
                }
                .disabled(!presentation.hasOpenProject)
                Button("Add Node", systemImage: "square.grid.2x2") {
                    workspace.requestNodeMenu()
                }
                Button("Save", systemImage: "square.and.arrow.down") {
                    actions.send(.save)
                }
                Button("Evaluate", systemImage: "play.fill") {
                    actions.send(.evaluate)
                }
                .disabled(presentation.isEvaluating)
                Button("Cancel", systemImage: "stop.fill") {
                    actions.send(.cancel)
                }
                .disabled(!presentation.isEvaluating)
                panelsMenu
            }
        }
    }

    @ViewBuilder
    private func region(
        _ region: WorkspaceRegion,
        minimumWidth: CGFloat,
        idealWidth: CGFloat
    ) -> some View {
        let panels = workspace.visiblePanels(in: region)
        if panels.isEmpty {
            ContentUnavailableView("Empty Region", systemImage: "rectangle.dashed")
                .frame(
                    minWidth: minimumWidth,
                    idealWidth: idealWidth,
                    maxWidth: .infinity,
                    maxHeight: .infinity
                )
        } else {
            VStack(spacing: 0) {
                ForEach(panels) { panel in
                    panelView(panel)
                    if panel != panels.last { Divider() }
                }
            }
            .frame(
                minWidth: minimumWidth,
                idealWidth: idealWidth,
                maxWidth: .infinity,
                maxHeight: .infinity
            )
        }
    }

    @ViewBuilder
    private func panelView(_ panel: WorkspacePanelID) -> some View {
        VStack(spacing: 0) {
            PanelHeader(panel: panel)
            Divider()
            self.panel(panel)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .foregroundStyle(theme?.color(.textPrimary) ?? Color.primary)
        .background(theme?.color(.surfacePanel) ?? Color(nsColor: .windowBackgroundColor))
    }

    private var panelsMenu: some View {
        Menu("Panels", systemImage: "rectangle.3.group") {
            ForEach(WorkspacePanelID.allCases) { panel in
                Toggle(panel.title, isOn: Binding(
                    get: { workspace.isVisible(panel) },
                    set: { _ in workspace.toggle(panel) }
                ))
            }
            Divider()
            Button("Restore Layout Authoring") {
                workspace.restoreLayoutAuthoringPreset()
            }
        }
    }
}
