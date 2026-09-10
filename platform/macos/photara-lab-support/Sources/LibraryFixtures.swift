import SwiftUI
import AppKit

enum LibraryFixtureState: String, CaseIterable, Identifiable {
    case populated, empty, loading, error
    var id: String { rawValue }
}
@MainActor enum LibraryFixtures {
    private static let baseItems: [LibraryItem] = [
        .init(id: "person-1", ownerID: "studio", revision: 1, kind: .person, name: "Maya Chen", aliases: ["Maya"], labels: ["model"], parentID: nil, detail: ""),
        .init(id: "person-2", ownerID: "studio", revision: 2, kind: .person, name: "Alex Rivera", aliases: [], labels: ["stylist", "art director"], parentID: nil, detail: ""),
        .init(id: "client-1", ownerID: "studio", revision: 1, kind: .client, name: "North Coast Studio", aliases: ["North Coast"], labels: ["organization"], parentID: nil, detail: ""),
        .init(id: "location-1", ownerID: "studio", revision: 1, kind: .location, name: "Ocean Beach", aliases: ["OB"], labels: [], parentID: nil, detail: "San Francisco, California"),
        .init(id: "location-2", ownerID: "studio", revision: 1, kind: .location, name: "North Dunes", aliases: [], labels: [], parentID: "location-1", detail: "Access from the northern end of the promenade."),
        .init(id: "scene-1", ownerID: "studio", revision: 3, kind: .scene, name: "Beach at Blue Hour", aliases: ["Coastal evening"], labels: ["natural light", "coast"], parentID: nil, detail: "Quiet portraits with open sky, cool shadows and the waterline in the distance."),
        .init(id: "scene-2", ownerID: "studio", revision: 1, kind: .scene, name: "Window Light", aliases: [], labels: ["interior"], parentID: nil, detail: "Soft daylight and a restrained background.")
    ]
    static let items: [LibraryItem] = baseItems.enumerated().map { index, item in
        let image = NSImage(size: .init(width: 104, height: 104), flipped: false) { rect in
            NSColor(calibratedHue: CGFloat(index) / 8, saturation: 0.24, brightness: 0.80, alpha: 1).setFill()
            rect.fill()
            NSColor.white.withAlphaComponent(0.35).setFill()
            NSBezierPath(ovalIn: .init(x: 40, y: 62, width: 42, height: 42)).fill()
            let symbol = NSImage(systemSymbolName: item.kind.symbol, accessibilityDescription: nil)!
            symbol.draw(in: .init(x: 24, y: 24, width: 56, height: 56))
            return true
        }
        let data = image.tiffRepresentation.flatMap(NSBitmapImageRep.init(data:))?.representation(using: .png, properties: [:])
        return LibraryItem(id: item.id, ownerID: item.ownerID, revision: item.revision, kind: item.kind, name: item.name,
            aliases: item.aliases, labels: item.labels, parentID: item.parentID, detail: item.detail, thumbnailData: data)
    }
    static func presentation(_ state: LibraryFixtureState) -> LibraryPresentation {
        switch state {
        case .populated: .init(items: items, phase: .ready)
        case .empty: .init(items: [], phase: .ready)
        case .loading: .loading
        case .error: .init(items: [], phase: .failed("The local Library could not be read. Try again after storage becomes available."))
        }
    }
    static let assignments: [ProjectAssignment] = [
        .init(id: "assignment-1", recordID: "person-1", ownerID: "studio", revision: 1, name: "Maya Chen", kind: .person, relationship: "model", date: "", notes: ""),
        .init(id: "assignment-2", recordID: "client-1", ownerID: "studio", revision: 1, name: "North Coast Studio", kind: .client, relationship: "commissioning client", date: "", notes: ""),
        .init(id: "assignment-3", recordID: "location-2", ownerID: "studio", revision: 1, name: "North Dunes", kind: .location, relationship: "shoot location", date: "", notes: ""),
        .init(id: "occurrence-1", recordID: "scene-1", ownerID: "studio", revision: 2, name: "Beach Shoot", kind: .scene, relationship: "opening portraits", date: "2026-09-12 · sunset", notes: "Meet at the promenade."),
        .init(id: "occurrence-2", recordID: "scene-1", ownerID: "studio", revision: 3, name: "Beach at Blue Hour", kind: .scene, relationship: "closing portraits", date: "2026-09-13", notes: "Second occurrence of the same reusable scene.")
    ]
}
@MainActor final class LibraryLabModel: ObservableObject {
    @Published var state: LibraryFixtureState = .populated { didSet { reset() } }
    @Published var dark = false
    @Published var presentation = LibraryFixtures.presentation(.populated)
    @Published var assignments = LibraryFixtures.assignments
    @Published var lastAction = "Deterministic fixtures · no database or network"
    private var sequence = 0
    private var history: [[ProjectAssignment]] = []
    var projectInfo: ProjectInfoPresentation {
        .init(title: "Coastal Studies", revision: 1, assignments: assignments, library: presentation, phase: presentation.phase, hasProject: true)
    }
    static func chooseThumbnail() -> URL? {
        let panel = NSOpenPanel(); panel.allowedContentTypes = [.image]; panel.allowsMultipleSelection = false
        return panel.runModal() == .OK ? panel.url : nil
    }
    func reset() {
        presentation = LibraryFixtures.presentation(state)
        assignments = state == .populated ? LibraryFixtures.assignments : []
        sequence = 0; history = []
    }
    func send(_ action: LibraryAction) {
        lastAction = String(describing: action)
        switch action {
        case .reload: state = .populated
        case .select: break
        case .delete(let item): presentation = .init(items: presentation.items.filter { $0.id != item.id }, phase: .ready)
        case .save(let draft):
            sequence += 1
            let item = LibraryItem(id: draft.recordID ?? "created-\(sequence)", ownerID: "studio", revision: (draft.expectedRevision ?? 0) + 1,
                kind: draft.kind, name: draft.name, aliases: LibraryDraft.values(draft.aliases), labels: LibraryDraft.values(draft.labels), parentID: draft.parentID.isEmpty ? nil : draft.parentID, detail: draft.detail, thumbnailData: draft.thumbnailSource.flatMap { try? Data(contentsOf: $0) } ?? presentation.items.first { $0.id == draft.recordID }?.thumbnailData)
            presentation = .init(items: presentation.items.filter { $0.id != item.id } + [item], phase: .ready)
        }
    }
    func projectAction(_ action: ProjectInfoAction) {
        lastAction = String(describing: action)
        if case .undo = action { if let previous = history.popLast() { assignments = previous }; return }
        history.append(assignments)
        switch action {
        case .createAndAssign(let draft):
            send(.save(draft))
            if let item = presentation.items.last { projectAction(.assign(recordID: item.id, relationship: draft.kind.rawValue)) }
        case .assign(let id, let relationship):
            guard let item = presentation.items.first(where: { $0.id == id }) else { return }
            sequence += 1
            assignments.append(.init(id: "new-occurrence-\(sequence)", recordID: item.id, ownerID: item.ownerID, revision: item.revision, name: item.name,
                kind: item.kind, relationship: relationship, date: "", notes: ""))
        case .remove(let id): assignments.removeAll { $0.id == id }
        case .edit(let id, let date, let notes):
            assignments = assignments.map { a in a.id == id ? .init(id: a.id, recordID: a.recordID, ownerID: a.ownerID, revision: a.revision, name: a.name, kind: a.kind, relationship: a.relationship, date: date, notes: notes) : a }
        case .reload: state = .populated
        case .undo: break
        }
    }
}
struct LibraryLabHost<Content: View>: View {
    let title: String
    @ObservedObject var model: LibraryLabModel
    @ViewBuilder var content: () -> Content
    var body: some View {
        HSplitView {
            Form {
                Picker("State", selection: $model.state) { ForEach(LibraryFixtureState.allCases) { Text($0.rawValue.capitalized).tag($0) } }
                Toggle("Dark appearance", isOn: $model.dark)
                Button("Reset Fixtures") { model.reset() }
                Text(model.lastAction).font(.caption).textSelection(.enabled)
            }.formStyle(.grouped).frame(minWidth: 220, idealWidth: 250, maxWidth: 320)
            LabAppearance(dark: model.dark) {
                VStack(alignment: .leading, spacing: 0) {
                    Text(title).font(.headline).padding(14)
                    Divider()
                    content()
                }.background(Color(nsColor: .controlBackgroundColor), in: RoundedRectangle(cornerRadius: 16)).padding(14)
            }.frame(minWidth: 320, maxWidth: .infinity, maxHeight: .infinity)
        }.frame(minWidth: 600, minHeight: 560)
    }
}
