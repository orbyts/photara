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
                         workSurface: { ProductionWorkSurfaceRegistry.view(for: $0) }) { panel in
            switch panel {
            case .people, .locations, .scenes, .account: ProductionLibraryModule(panel: panel, library: app.library)
            case .projectInfo: ProductionProjectInfo(library: app.library)
            case .assetGallery: ProductionGalleryView()
            case .graph: ProductionGraphView()
            case .nodeWorkSurface: EmptyView() // Hosted through the node contribution registry.
            case .inspector: ProductionInspectorView()
            case .diagnostics:
                DiagnosticsView(diagnostics: (app.snapshot?.diagnostics ?? []).map {
                    .init(code: $0.code, message: $0.message)
                })
            }
        }
        .onChange(of: app.projectSetupRequest) { session.show(.projectInfo) }
        .alert("Photara", isPresented: Binding(
            get: { app.presentedError != nil }, set: { if !$0 { app.presentedError = nil } }
        )) { Button("OK") { app.presentedError = nil } }
        message: { Text(app.presentedError ?? "Unknown error") }
    }
}
