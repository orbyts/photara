import SwiftUI

@main
struct PhotaraMacApp: App {
    private let configuration = ReleaseConfiguration.current
    @StateObject private var app = AppModel()
    @StateObject private var session = EditorSessionModel()
    @StateObject private var theme = PhotaraThemeStore()

    var body: some Scene {
        WindowGroup(configuration.identity.displayName) {
            ThemedEditorRoot()
                .environmentObject(app)
                .environmentObject(session)
                .environmentObject(theme)
                .onOpenURL { url in
                    if url.isFileURL && url.pathExtension == configuration.identity.projectPackageExtension {
                        app.openCreatedPackage(url.path)
                    }
                }
                .onAppear {
                    // Public channel only; never log tokens or service settings.
                    print("release-channel:\(configuration.environment.channel.rawValue)")
                }
        }
        .windowToolbarStyle(.unified)
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
                    .disabled(app.project == nil)
            }
            CommandGroup(replacing: .undoRedo) {
                Button("Undo Layout Edit") { app.undoLayout() }
                    .keyboardShortcut("z")
                    .disabled(app.project == nil)
                Button("Redo Layout Edit") { app.redoLayout() }
                    .keyboardShortcut("z", modifiers: [.command, .shift])
                    .disabled(app.project == nil)
            }
            CommandMenu("Editor") {
                Button("Restore Default Editor") {
                    session.restoreLayoutAuthoringPreset()
                }
                .keyboardShortcut("0", modifiers: [.command, .option])
                Divider()
                ForEach(ApplicationShellAvailability(presentation: app.applicationPresentation(session)).panels) { panel in
                    Toggle(isOn: Binding(
                        get: { EditorRegion.allCases.contains {
                            ApplicationShellAvailability(presentation: app.applicationPresentation(session))
                                .visiblePanels(in: $0, session: session).contains(panel)
                        } },
                        set: { visible in if visible {
                            if panel == .nodeWorkSurface, session.activeWorkSurfaceNodeID == nil { session.activeWorkSurfaceNodeID = app.applicationPresentation(session).workSurfaces.first?.nodeID }
                            session.show(panel)
                        } else { session.toggle(panel) } }
                    )) { Label(panel.title, systemImage: panel.symbol) }
                }
            }
            CommandMenu("Graph") {
                Button("Add Node…") {
                    session.requestNodeMenu()
                }
                .keyboardShortcut(KeyEquivalent("\t"), modifiers: [])
                .disabled(app.project == nil || !session.isVisible(.graph))
            }
        }
        Settings { LibrarySyncView().frame(width: 460, height: 500) }
    }
}

private struct ThemedEditorRoot: View {
    @Environment(\.colorScheme) private var colorScheme
    @EnvironmentObject private var theme: PhotaraThemeStore
    @State private var shellPreset = ApplicationShellPreset.developmentOrShipped

    private var brandedShellPreset: ApplicationShellPreset {
        var preset = shellPreset
        preset.toolbarApplicationTitle = ReleaseConfiguration.current.identity.displayName
        return preset
    }

    var body: some View {
        let appearance: PhotaraThemeAppearance = colorScheme == .dark ? .dark : .light
        let resolved = theme.document.resolved(for: appearance)
        EditorSessionView(shellPreset: brandedShellPreset)
            .environment(\.photaraTheme, resolved)
            .task {
                while !Task.isCancelled {
                    let latest = ApplicationShellPreset.developmentOrShipped
                    if latest != shellPreset { shellPreset = latest }
                    try? await Task.sleep(for: .milliseconds(500))
                }
            }
    }
}
