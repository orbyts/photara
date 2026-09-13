import Foundation

struct ApplicationPresentation {
    struct Recent: Identifiable { var id: String; var title: String; var lastOpened: Date }
    var hasOpenProject: Bool
    var title: String
    var subtitle: String
    var projectThumbnailURL: URL? = nil
    var isDirty = false
    var nodeCount = 0
    var diagnosticCount = 0
    var progressLabel = "Idle"
    var isEvaluating = false
    var workSurfaces: [NodeWorkSurfacePresentation] = []
    var projectID: String?
    var nodeIDs: [String] = []
    var hasAssets = false
    var hasAssetProducingContext = false
    var hasReviewableResult = false
    var syncLabel: String?
    var surfaceContext: String?
    var recentProjects: [Recent] = []
}
enum ApplicationAction { case newProject, openProject, closeProject, importPair, save, evaluate, cancel, openRecent(String) }
struct ApplicationActions { var send: (ApplicationAction) -> Void }

/// Capability availability is separate from the user's placement/visibility preference.
struct ApplicationShellAvailability {
    let presentation: ApplicationPresentation
    var panels: [EditorPanelID] {
        guard presentation.hasOpenProject else { return [.people, .locations, .scenes, .account] }
        var result: [EditorPanelID] = [.graph, .assetGallery, .diagnostics, .inspector, .people, .locations, .scenes, .projectInfo, .account]
        if !presentation.workSurfaces.isEmpty { result.append(.nodeWorkSurface) }
        return result
    }
    var modes: [EditorMode] {
        guard presentation.hasOpenProject else { return [] }
        var result: [EditorMode] = [.graph]
        if !presentation.workSurfaces.isEmpty { result.append(.nodeWorkSurface) }
        if presentation.hasReviewableResult { result.append(.review) }
        return result
    }
    @MainActor func visiblePanels(in region: EditorRegion, session: EditorSessionModel) -> [EditorPanelID] {
        session.visiblePanels(in: region).filter { panel in
            guard panels.contains(panel) else { return false }
            switch panel {
            case .inspector: return session.inspectorActivated
            case .assetGallery:
                return presentation.hasAssets || presentation.hasAssetProducingContext || session.galleryExplicitlyOpened
            case .diagnostics: return presentation.diagnosticCount > 0 || session.diagnosticsExplicitlyOpened
            default: return true
            }
        }
    }
    var hasStatus: Bool {
        presentation.isDirty || presentation.syncLabel != nil || presentation.diagnosticCount > 0
            || presentation.isEvaluating || presentation.surfaceContext != nil
    }
}

/// A node-owned, host-supported contribution. The shell knows neither its fields nor its renderer.
struct NodeWorkSurfacePresentation: Identifiable, Equatable {
    var nodeID: String
    var contributionID: String
    var title: String
    var iconResourceID: String
    var themeColorRole: String?
    var accentHex: String?
    var id: String { nodeID }
}
