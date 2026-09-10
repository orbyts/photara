import SwiftUI

struct ProjectAssignment: Identifiable, Equatable, Sendable {
    let id: String
    let recordID: String
    let ownerID: String
    let revision: UInt64
    let name: String
    let kind: LibraryKind
    let relationship: String
    let date: String
    let notes: String
}
struct ProjectInfoPresentation: Sendable {
    let title: String
    let revision: UInt64
    let assignments: [ProjectAssignment]
    let library: LibraryPresentation
    let phase: LibraryPhase
    let hasProject: Bool
}
enum ProjectInfoAction {
    case createAndAssign(LibraryDraft)
    case assign(recordID: String, relationship: String)
    case edit(assignmentID: String, date: String, notes: String)
    case remove(String), reload, undo
}
struct ProjectInfoActions {
    let send: (ProjectInfoAction) -> Void
    var chooseThumbnail: () -> URL? = { nil }
}
