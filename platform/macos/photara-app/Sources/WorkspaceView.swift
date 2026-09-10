import SwiftUI

/// Production composition root: shared shell + shared features, connected to real host adapters.
struct WorkspaceView: View {
    @EnvironmentObject private var app: AppModel
    @EnvironmentObject private var workspace: WorkspaceModel

    var body: some View {
        ApplicationShell(presentation: app.applicationPresentation(workspace),
                         actions: .init(send: app.performApplicationAction)) { panel in
            switch panel {
            case .assetGallery: ProductionGalleryView()
            case .graph: ProductionGraphView()
            case .layoutAuthoring: ProductionLayoutView()
            case .inspector: ProductionInspectorView()
            case .diagnostics:
                DiagnosticsView(diagnostics: (app.snapshot?.diagnostics ?? []).map {
                    .init(code: $0.code, message: $0.message)
                })
            }
        }
        .alert("Photara", isPresented: Binding(
            get: { app.presentedError != nil }, set: { if !$0 { app.presentedError = nil } }
        )) { Button("OK") { app.presentedError = nil } }
        message: { Text(app.presentedError ?? "Unknown error") }
        .task(id: app.snapshot?.graph.digest) {
            let nodes = app.snapshot?.nodes ?? []
            if workspace.selectedNodeID.flatMap({ selected in nodes.first { $0.nodeId == selected } }) == nil {
                workspace.selectedNodeID = nodes.first { $0.layout != nil }?.nodeId ?? nodes.first?.nodeId
            }
        }
    }
}
