import Foundation

enum LibraryKind: String, CaseIterable, Identifiable, Sendable {
    case person, client, location, scene
    var id: String { rawValue }
    var title: String { rawValue.capitalized }
    var symbol: String {
        switch self {
        case .person: "person.crop.circle"
        case .client: "building.2.crop.circle"
        case .location: "mappin.and.ellipse"
        case .scene: "rectangle.stack"
        }
    }
}
struct LibraryItem: Identifiable, Equatable, Sendable {
    let id: String
    let ownerID: String
    let revision: UInt64
    let kind: LibraryKind
    let name: String
    let aliases: [String]
    let labels: [String]
    let parentID: String?
    let detail: String
    let thumbnailDigest: String?
    let thumbnailData: Data?
    init(id: String, ownerID: String, revision: UInt64, kind: LibraryKind, name: String, aliases: [String], labels: [String], parentID: String?, detail: String, thumbnailDigest: String? = nil, thumbnailData: Data? = nil) {
        self.id = id; self.ownerID = ownerID; self.revision = revision; self.kind = kind; self.name = name
        self.aliases = aliases; self.labels = labels; self.parentID = parentID; self.detail = detail
        self.thumbnailDigest = thumbnailDigest; self.thumbnailData = thumbnailData
    }
    var searchText: String { ([name, detail] + aliases + labels).joined(separator: " ") }
}
enum LibraryPhase: Equatable, Sendable { case loading, ready, failed(String) }
struct LibraryPresentation: Sendable {
    let items: [LibraryItem]
    let phase: LibraryPhase
    static let loading = Self(items: [], phase: .loading)
}
/// Disposable session state; identity and optimistic revision come from the observed value.
struct LibraryDraft: Identifiable, Sendable {
    var id: String { recordID ?? "new-\(kind.rawValue)" }
    let recordID: String?
    let expectedRevision: UInt64?
    let kind: LibraryKind
    var name: String
    var aliases: String
    var labels: String
    var parentID: String
    var detail: String
    var thumbnailDigest: String?
    var thumbnailSource: URL?
    init(kind: LibraryKind) {
        recordID = nil; expectedRevision = nil; self.kind = kind
        name = ""; aliases = ""; labels = kind == .client ? "organization" : ""
        parentID = ""; detail = ""; thumbnailDigest = nil; thumbnailSource = nil
    }
    init(_ item: LibraryItem) {
        recordID = item.id; expectedRevision = item.revision; kind = item.kind
        name = item.name; aliases = item.aliases.joined(separator: ", ")
        labels = item.labels.joined(separator: ", "); parentID = item.parentID ?? ""; detail = item.detail
        thumbnailDigest = item.thumbnailDigest; thumbnailSource = nil
    }
    static func values(_ text: String) -> [String] {
        text.split(separator: ",").map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }.filter { !$0.isEmpty }
    }
}
enum LibraryAction { case reload, save(LibraryDraft), delete(LibraryItem), select(String) }
struct LibraryActions {
    let send: (LibraryAction) -> Void
    var chooseThumbnail: () -> URL? = { nil }
}
