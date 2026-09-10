import SwiftUI

@main
struct PhotaraMacApp: App {
    @StateObject private var app = AppModel()
    @StateObject private var workspace = WorkspaceModel()
    @StateObject private var theme = PhotaraThemeStore()

    var body: some Scene {
        WindowGroup("Photara") {
            ThemedWorkspaceRoot()
                .environmentObject(app)
                .environmentObject(workspace)
                .environmentObject(theme)
        }
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("New Project") { app.newProject() }
                    .keyboardShortcut("n")
                Button("Open Project…") { app.chooseAndOpenProject() }
                    .keyboardShortcut("o")
                Button("Close Project") { app.closeProject() }
                    .keyboardShortcut("w")
            }
            CommandGroup(replacing: .saveItem) {
                Button("Save") { app.save() }
                    .keyboardShortcut("s")
            }
            CommandGroup(replacing: .undoRedo) {
                Button("Undo Layout Edit") { app.undoLayout() }
                    .keyboardShortcut("z")
                Button("Redo Layout Edit") { app.redoLayout() }
                    .keyboardShortcut("z", modifiers: [.command, .shift])
            }
            CommandMenu("Workspace") {
                Button("Restore Default Workspace") {
                    workspace.restoreLayoutAuthoringPreset()
                }
                .keyboardShortcut("0", modifiers: [.command, .option])
                Divider()
                ForEach(ApplicationShellAvailability(presentation: app.applicationPresentation(workspace)).panels.filter { $0 != .graph && $0 != .nodeWorkSurface }) { panel in
                    Toggle(panel.title, isOn: Binding(
                        get: { WorkspaceRegion.allCases.contains {
                            ApplicationShellAvailability(presentation: app.applicationPresentation(workspace))
                                .visiblePanels(in: $0, workspace: workspace).contains(panel)
                        } },
                        set: { visible in if visible { workspace.show(panel) } else { workspace.toggle(panel) } }
                    ))
                }
            }
            CommandMenu("Graph") {
                Button("Add Node…") {
                    workspace.requestNodeMenu()
                }
                .keyboardShortcut(KeyEquivalent("\t"), modifiers: [])
                .disabled(!app.hasOpenProject || !workspace.isVisible(.graph))
            }
        }
    }
}

private struct ThemedWorkspaceRoot: View {
    @Environment(\.colorScheme) private var colorScheme
    @EnvironmentObject private var theme: PhotaraThemeStore

    var body: some View {
        let appearance: PhotaraThemeAppearance = colorScheme == .dark ? .dark : .light
        let resolved = theme.document.resolved(for: appearance)
        WorkspaceView()
            .environment(\.photaraTheme, resolved)
            .tint(resolved.color(.borderFocus))
    }
}
