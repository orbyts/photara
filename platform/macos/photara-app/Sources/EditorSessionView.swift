import SwiftUI

/// Production composition root: shared shell + shared features, connected to real host adapters.
struct EditorSessionView: View {
    @EnvironmentObject private var app: AppModel
    @EnvironmentObject private var session: EditorSessionModel
    var shellPreset: ApplicationShellPreset = .shipped

    var body: some View {
        ApplicationShell(presentation: app.applicationPresentation(session),
                         actions: .init(send: app.performApplicationAction),
                         preset: shellPreset,
                         openingCloud: app.openingCloud,
                         workSurface: { ProductionWorkSurfaceRegistry.view(for: $0) }) { panel in
            switch panel {
            case .people, .locations, .scenes, .account: ProductionLibraryModule(panel: panel, library: app.library)
            case .projectInfo: ProductionProjectInfo(library: app.library)
            case .assetGallery: ProductionGalleryView()
            case .graph:
                if let created = app.createdProject { CreatedProjectGraph(project: created) }
                else { ProductionGraphView() }
            case .nodeWorkSurface: EmptyView() // Hosted through the node contribution registry.
            case .inspector: ProductionInspectorView()
            case .diagnostics:
                DiagnosticsView(diagnostics: (app.snapshot?.diagnostics ?? []).map {
                    .init(code: $0.code, message: $0.message)
                })
            }
        }
        .onChange(of: app.projectSetupRequest) { session.show(.projectInfo) }
        .onChange(of: app.createdProject?.projectId) { if app.createdProject != nil { session.show(.graph) } }
        .task { app.restoreCreationOperations() }
        .sheet(isPresented: $app.showsCreateProject) {
            CreateProjectView(libraryName: ReleaseConfiguration.current.identity.defaultLibraryName,
                isCloudLibrary: app.openingCloud.hasCloudLibrary,
                chooseDestination: app.chooseCreationDestination,
                cancel: app.cancelProjectCreation,
                create: app.submitProjectCreation,
                destination: app.creationDestination.path,
                isWorking: app.isCreatingProject,
                recoveryMessage: app.creationMessage,
                hasPendingOperation: app.creationOperation != nil,
                pendingName: app.creationTitle,
                keepForLater: { app.showsCreateProject = false })
            .interactiveDismissDisabled(app.isCreatingProject || app.creationOperation != nil)
        }
        .alert(ReleaseConfiguration.current.identity.displayName, isPresented: Binding(
            get: { app.presentedError != nil }, set: { if !$0 { app.presentedError = nil } }
        )) { Button("OK") { app.presentedError = nil } }
        message: { Text(app.presentedError ?? "Unknown error") }
    }
}

/// UI1's safe saved-project route uses the accepted canvas without attaching an
/// authoring adapter that still writes legacy single-document projects.
private struct CreatedProjectGraph: View {
    let project: BridgeProjectCreation
    @Environment(\.colorScheme) private var colorScheme
    @State private var controller = PhotaraGraphInteractionController(document: .init(nodes: [], connections: []))
    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Graph 1").font(.headline)
                Spacer()
                Label("Saved", systemImage: "checkmark.circle").foregroundStyle(.secondary)
            }.padding(12)
            let preset = PhotaraGraphPresentationPreset.shipped
            let palette = preset.palette(colorScheme)
            PhotaraGraphCanvas(toolRailPlacement: .leftTop, controller: controller,
                backgroundStyle: preset.backgroundStyle, backgroundColor: palette.graphBackground?.color,
                minorColor: palette.minor?.color, majorColor: palette.major?.color,
                noodleColor: palette.noodle?.color ?? .accentColor,
                knifeCursorSize: 24, showsToolRail: false,
                centerScene: {}, addNativeNode: { _ in },
                nodeContent: { _, _, _, _ in EmptyView() }) { EmptyView() }
                .accessibilityLabel("Graph 1, empty saved graph")
                .accessibilityIdentifier("created-project-graph")
        }
        .id(project.graphId)
    }
}
