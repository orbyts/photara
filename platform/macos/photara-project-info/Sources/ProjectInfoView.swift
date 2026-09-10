import SwiftUI

struct ProjectInfoView: View {
    let presentation: ProjectInfoPresentation
    let actions: ProjectInfoActions
    @State private var editing: ProjectAssignment?
    @State private var selectedRecord = ""
    @State private var relationship = ""
    @State private var assigning = false
    @State private var search = ""
    @State private var creating: LibraryDraft?
    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text(presentation.title).font(.headline).lineLimit(1)
                Spacer()
                Menu {
                    Button("Undo Last Assignment Edit") { actions.send(.undo) }
                    Button("Reload") { actions.send(.reload) }
                } label: { Image(systemName: "ellipsis") }.frame(width: 25)
            }
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    switch presentation.phase {
                    case .loading: ProgressView("Loading project context…").padding(24)
                    case .failed(let message):
                        ContentUnavailableView { Label("Project Info unavailable", systemImage: "exclamationmark.triangle") }
                            description: { Text(message) } actions: { Button("Try Again") { actions.send(.reload) } }
                    case .ready:
                        if !presentation.hasProject {
                            ContentUnavailableView("Open a project", systemImage: "folder", description: Text("Assign people, clients, locations and scenes to a project."))
                        } else {
                            Text("Project assignments keep a snapshot of each Library record. Scene assignments are unique occurrences.")
                                .font(.callout).foregroundStyle(.secondary)
                            ForEach(LibraryKind.allCases) { kind in
                                VStack(alignment: .leading, spacing: 8) {
                                    Text(sectionTitle(kind)).font(.headline)
                                    let assignments = presentation.assignments.filter { $0.kind == kind }
                                    if assignments.isEmpty { Text("None assigned").font(.callout).foregroundStyle(.secondary) }
                                    ForEach(assignments) { row($0) }
                                }
                            }
                        }
                    }
                }.frame(maxWidth: .infinity, alignment: .leading)
            }
            Button("Assign from Library…", systemImage: "link.badge.plus") { assigning = true }
                .disabled(!presentation.hasProject || presentation.phase != .ready || presentation.library.phase != .ready)
        }.padding(14)
        .sheet(isPresented: $assigning) { assignmentPicker }
        .sheet(item: $editing) { assignment in
            ProjectOccurrenceEditor(assignment: assignment, actions: actions)
        }
    }
    private func sectionTitle(_ kind: LibraryKind) -> String {
        switch kind { case .person: "People"; case .client: "Clients"; case .location: "Locations"; case .scene: "Scene occurrences" }
    }
    private func row(_ assignment: ProjectAssignment) -> some View {
        let current = presentation.library.items.first { $0.id == assignment.recordID && $0.ownerID == assignment.ownerID }
        return HStack(alignment: .top) {
            LibraryThumbnail(data: current?.thumbnailData, name: assignment.name, kind: assignment.kind)
            VStack(alignment: .leading, spacing: 4) {
                Text(assignment.name).font(.body.weight(.medium))
                Text(assignment.relationship).font(.caption).foregroundStyle(.secondary)
                if !assignment.date.isEmpty { Text(assignment.date).font(.caption) }
                if !assignment.notes.isEmpty { Text(assignment.notes).font(.caption).lineLimit(3) }
                if presentation.library.phase == .ready && current == nil {
                    Text("Saved snapshot · Library record unavailable").font(.caption2).foregroundStyle(.secondary)
                } else if let current, current.revision != assignment.revision {
                    Text("Saved snapshot · now \(current.name)").font(.caption2).foregroundStyle(.secondary)
                }
            }.frame(maxWidth: .infinity, alignment: .leading)
            Menu {
                Button("Edit Occurrence Details…") { editing = assignment }
                Button("Remove Assignment") { actions.send(.remove(assignment.id)) }
            } label: { Image(systemName: "ellipsis") }.frame(width: 24)
        }.padding(10).background(Color.primary.opacity(0.035), in: RoundedRectangle(cornerRadius: 10))
    }
    private var assignmentPicker: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Assign from Library").font(.title2.weight(.semibold))
            TextField("Search people, clients, locations and scenes", text: $search).textFieldStyle(.roundedBorder)
            if presentation.library.items.isEmpty {
                Text("Create a person, client, location or scene in its Library module first.").foregroundStyle(.secondary)
            } else {
                ScrollView {
                    LazyVStack(spacing: 6) {
                        ForEach(presentation.library.items.filter { search.isEmpty || $0.searchText.localizedCaseInsensitiveContains(search) }) { item in
                            Button { selectedRecord = item.id } label: {
                                HStack {
                                    LibraryThumbnail(data: item.thumbnailData, name: item.name, kind: item.kind)
                                    VStack(alignment: .leading) { Text(item.name); Text(item.kind.title).font(.caption).foregroundStyle(.secondary) }
                                    Spacer()
                                    if selectedRecord == item.id { Image(systemName: "checkmark.circle.fill") }
                                }.padding(6).background(selectedRecord == item.id ? Color.accentColor.opacity(0.1) : Color.clear, in: RoundedRectangle(cornerRadius: 8))
                            }.buttonStyle(.plain)
                        }
                    }
                }.frame(height: 240)
                TextField("Project role", text: $relationship)
                Text("For example: model, commissioning client, shoot location or scene.").font(.caption).foregroundStyle(.secondary)
            }
            Menu("Create New…", systemImage: "plus") {
                ForEach(LibraryKind.allCases) { kind in
                    Button(kind.title) { var draft = LibraryDraft(kind: kind); draft.name = search; creating = draft }
                }
            }
            HStack {
                Button("Cancel") { assigning = false }.keyboardShortcut(.cancelAction)
                Spacer()
                Button("Assign") { actions.send(.assign(recordID: selectedRecord, relationship: relationship)); assigning = false }
                    .disabled(selectedRecord.isEmpty || relationship.trimmingCharacters(in: .whitespaces).isEmpty)
                    .keyboardShortcut(.defaultAction)
            }
        }.padding(24).frame(width: 460)
        .sheet(item: $creating) { draft in
            LibraryEditor(draft: draft, actions: .init(send: { action in
                if case .save(let value) = action { actions.send(.createAndAssign(value)); assigning = false }
            }, chooseThumbnail: actions.chooseThumbnail)) { binding in
                switch binding.wrappedValue.kind {
                case .person, .client: PeopleEditorFields(draft: binding)
                case .location: LocationsEditorFields(draft: binding, items: presentation.library.items)
                case .scene: ScenesEditorFields(draft: binding)
                }
            }
        }
        .onChange(of: selectedRecord) {
            if let item = presentation.library.items.first(where: { $0.id == selectedRecord }) {
                relationship = item.kind == .person ? (item.labels.first ?? "collaborator") : item.kind.title.lowercased()
            }
        }
    }
}
private struct ProjectOccurrenceEditor: View {
    let assignment: ProjectAssignment
    let actions: ProjectInfoActions
    @State private var date = ""
    @State private var notes = ""
    @Environment(\.dismiss) private var dismiss
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text(assignment.name).font(.title2.weight(.semibold))
            Text("Details belong to this project occurrence.").foregroundStyle(.secondary)
            TextField("Date or schedule", text: $date)
            TextField("Notes", text: $notes, axis: .vertical).lineLimit(4...8)
            HStack {
                Button("Cancel") { dismiss() }.keyboardShortcut(.cancelAction)
                Spacer()
                Button("Save") { actions.send(.edit(assignmentID: assignment.id, date: date, notes: notes)); dismiss() }.keyboardShortcut(.defaultAction)
            }
        }.padding(24).frame(width: 420)
            .onAppear { date = assignment.date; notes = assignment.notes }
    }
}
