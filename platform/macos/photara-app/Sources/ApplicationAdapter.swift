import Foundation

extension AppModel {
    func applicationPresentation(_ workspace: WorkspaceModel) -> ApplicationPresentation {
        let layouts = snapshot?.nodes.filter { $0.layout != nil } ?? []
        return .init(hasOpenProject: hasOpenProject, title: snapshot?.title ?? "Photara Project",
            subtitle: snapshot.map { "rev \($0.projectRevision) · \($0.projectId.prefix(12))" } ?? "Not loaded",
            isDirty: snapshot?.dirty == true, nodeCount: snapshot?.nodes.count ?? 0,
            diagnosticCount: snapshot?.diagnostics.count ?? 0, progressLabel: progressLabel,
            isEvaluating: isEvaluating,
            layoutNodeID: layouts.first { $0.nodeId == workspace.selectedNodeID }?.nodeId ?? layouts.first?.nodeId,
            recentProjects: recentProjects.map { .init(id: $0.id, title: $0.title, lastOpened: $0.lastOpened) })
    }
    func performApplicationAction(_ action: ApplicationAction) {
        switch action {
        case .newProject: newProject()
        case .openProject: chooseAndOpenProject()
        case .closeProject: closeProject()
        case .importPair: chooseAndImportTiffPair()
        case .save: save()
        case .evaluate: evaluate()
        case .cancel: cancelEvaluation()
        case .openRecent(let id): if let recent = recentProjects.first(where: { $0.id == id }) { openRecent(recent) }
        }
    }
}
