import Foundation
import SwiftUI
import AppKit
import ImageIO
import UniformTypeIdentifiers

/// Serial background access prevents database work from blocking native view updates.
actor LocalLibrarySession {
    private var handle: PhotaraLibrary?
    let path: String
    let owner: String
    init(path: String, owner: String) { self.path = path; self.owner = owner }
    func library() throws -> PhotaraLibrary {
        if let handle { return handle }
        let created = try PhotaraLibrary.local(path: path, ownerId: owner)
        handle = created
        return created
    }
    func load() throws -> [LibraryItem] {
        let handle = try library()
        return try handle.search(text: "").map { value in
            let data = value.thumbnailDigest.flatMap { try? handle.thumbnailPath(digest: $0) }.flatMap { try? Data(contentsOf: URL(fileURLWithPath: $0)) }
            return LibraryItem(value, thumbnailData: data)
        }
    }
    func save(_ draft: LibraryDraft, deleted: Bool = false) throws -> BridgeLibraryRecordDto {
        var digest = draft.thumbnailDigest
        if let source = draft.thumbnailSource {
            let temporary = FileManager.default.temporaryDirectory.appending(path: "photara-library-\(UUID().uuidString).png")
            defer { try? FileManager.default.removeItem(at: temporary) }
            guard let imageSource = CGImageSourceCreateWithURL(source as CFURL, nil),
                  let image = CGImageSourceCreateThumbnailAtIndex(imageSource, 0, [
                    kCGImageSourceCreateThumbnailFromImageAlways: true,
                    kCGImageSourceThumbnailMaxPixelSize: 256,
                    kCGImageSourceCreateThumbnailWithTransform: true
                  ] as CFDictionary),
                  let destination = CGImageDestinationCreateWithURL(temporary as CFURL, UTType.png.identifier as CFString, 1, nil)
            else { throw CocoaError(.fileReadCorruptFile) }
            CGImageDestinationAddImage(destination, image, nil)
            guard CGImageDestinationFinalize(destination) else { throw CocoaError(.fileWriteUnknown) }
            digest = try library().importThumbnail(sourcePath: temporary.path)
        }
        return try library().edit(edit: .init(recordId: draft.recordID, expectedRevision: draft.expectedRevision,
            kind: draft.kind.bridgeKind, displayName: draft.name, aliases: LibraryDraft.values(draft.aliases),
            labels: LibraryDraft.values(draft.labels), parentId: draft.parentID.isEmpty ? nil : draft.parentID,
            detail: draft.detail, deleted: deleted, thumbnailDigest: digest))
    }
    func edit(_ draft: LibraryDraft, deleted: Bool = false) throws -> [LibraryItem] {
        _ = try save(draft, deleted: deleted)
        return try load()
    }
}
@MainActor final class LibraryModel: ObservableObject {
    @Published private(set) var presentation = LibraryPresentation.loading
    @Published var error: String?
    let session: LocalLibrarySession
    init(defaults: UserDefaults, supportRoot: URL?) {
        let key = "photara.library.owner.v1"
        let owner = defaults.string(forKey: key) ?? UUID().uuidString.lowercased()
        defaults.set(owner, forKey: key)
        let root = supportRoot ?? FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0].appending(path: "Photara")
        session = LocalLibrarySession(path: root.appending(path: "Library/library.sqlite").path, owner: owner)
    }
    static func chooseThumbnail() -> URL? {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.image]; panel.allowsMultipleSelection = false; panel.canChooseDirectories = false
        panel.message = "Choose a thumbnail for this Library record"
        return panel.runModal() == .OK ? panel.url : nil
    }
    func load() async {
        do { presentation = .init(items: try await session.load(), phase: .ready) }
        catch { presentation = .init(items: presentation.items, phase: .failed(error.localizedDescription)) }
    }
    func send(_ action: LibraryAction) {
        Task {
            do {
                switch action {
                case .reload: await load()
                case .select: break
                case .save(let draft): presentation = .init(items: try await session.edit(draft), phase: .ready)
                case .delete(let item): presentation = .init(items: try await session.edit(.init(item), deleted: true), phase: .ready)
                }
            } catch { self.error = error.localizedDescription }
        }
    }
}
extension LibraryKind {
    var bridgeKind: BridgeLibraryKind {
        switch self { case .person: .person; case .client: .client; case .location: .location; case .scene: .scene }
    }
    init(_ value: BridgeLibraryKind) {
        switch value { case .person: self = .person; case .client: self = .client; case .location: self = .location; case .scene: self = .scene }
    }
}
extension LibraryItem {
    init(_ value: BridgeLibraryRecordDto, thumbnailData: Data? = nil) {
        self.init(id: value.recordId, ownerID: value.ownerId, revision: value.revision, kind: .init(value.kind), name: value.displayName,
            aliases: value.aliases, labels: value.labels, parentID: value.parentId, detail: value.detail, thumbnailDigest: value.thumbnailDigest, thumbnailData: thumbnailData)
    }
}
struct ProductionLibraryModule: View {
    let panel: EditorPanelID
    @ObservedObject var library: LibraryModel
    var body: some View {
        Group {
            switch panel {
            case .people: PeopleView(presentation: library.presentation, actions: .init(send: library.send, chooseThumbnail: LibraryModel.chooseThumbnail))
            case .locations: LocationsView(presentation: library.presentation, actions: .init(send: library.send, chooseThumbnail: LibraryModel.chooseThumbnail))
            case .scenes: ScenesView(presentation: library.presentation, actions: .init(send: library.send, chooseThumbnail: LibraryModel.chooseThumbnail))
            default: LibrarySyncView()
            }
        }.task { await library.load() }
            .alert("Library", isPresented: Binding(get: { library.error != nil }, set: { if !$0 { library.error = nil } })) {
                Button("OK") { library.error = nil }
            } message: { Text(library.error ?? "") }
    }
}
struct ProductionProjectInfo: View {
    @EnvironmentObject private var app: AppModel
    @ObservedObject var library: LibraryModel
    @State private var context: BridgeProjectInfoDto?
    @State private var phase: LibraryPhase = .loading
    var body: some View {
        ProjectInfoView(presentation: .init(title: app.snapshot?.title ?? "Project Info", revision: context?.revision ?? 0,
            assignments: (context?.assignments ?? []).map { .init(id: $0.assignmentId, recordID: $0.recordId,
                ownerID: $0.ownerId, revision: $0.recordRevision, name: $0.displayName, kind: .init($0.kind),
                relationship: $0.relationship, date: $0.date, notes: $0.notes) },
            library: library.presentation, phase: phase, hasProject: app.hasOpenProject), actions: .init(send: send, chooseThumbnail: LibraryModel.chooseThumbnail))
            .task(id: app.snapshot?.projectId) {
                context = nil; phase = .loading
                await library.load()
                if !Task.isCancelled { reload() }
            }
    }
    private func reload() {
        do { context = try app.project?.libraryContext(); phase = .ready }
        catch { phase = .failed(error.localizedDescription) }
    }
    private func send(_ action: ProjectInfoAction) {
        guard let project = app.project else { return }
        let expected = context?.revision ?? 0
        let projectID = app.snapshot?.projectId
        Task {
            do {
                let result: BridgeProjectInfoDto
                switch action {
                case .createAndAssign(let draft):
                    let created = try await library.session.save(draft)
                    await library.load()
                    result = try project.assignLibraryRecord(library: await library.session.library(), recordId: created.recordId, expectedRevision: expected,
                        relationship: draft.kind == .person ? (LibraryDraft.values(draft.labels).first ?? "collaborator") : draft.kind.rawValue)
                case .reload: await library.load(); reload(); return
                case .undo: result = try project.undoLibraryAssignment(expectedRevision: expected)
                case .assign(let id, let relationship):
                    result = try project.assignLibraryRecord(library: await library.session.library(), recordId: id, expectedRevision: expected, relationship: relationship)
                case .edit(let id, let date, let notes):
                    result = try project.editLibraryAssignment(assignmentId: id, expectedRevision: expected, date: date, notes: notes, remove: false)
                case .remove(let id):
                    result = try project.editLibraryAssignment(assignmentId: id, expectedRevision: expected, date: "", notes: "", remove: true)
                }
                guard app.snapshot?.projectId == projectID else { return }
                context = result; phase = .ready
                app.refreshAfterLibraryEdit()
            } catch { app.presentedError = error.localizedDescription }
        }
    }
}
