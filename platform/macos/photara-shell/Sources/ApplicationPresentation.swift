import Foundation

struct ApplicationPresentation {
    struct Recent: Identifiable { var id: String; var title: String; var lastOpened: Date }
    var hasOpenProject: Bool
    var title: String
    var subtitle: String
    var isDirty = false
    var nodeCount = 0
    var diagnosticCount = 0
    var progressLabel = "Idle"
    var isEvaluating = false
    var layoutNodeID: String?
    var recentProjects: [Recent] = []
}
enum ApplicationAction { case newProject, openProject, closeProject, importPair, save, evaluate, cancel, openRecent(String) }
struct ApplicationActions { var send: (ApplicationAction) -> Void }
