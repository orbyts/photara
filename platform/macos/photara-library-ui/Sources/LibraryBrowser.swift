import SwiftUI

/// Shared discovery/session chrome. Feature modules own their fields and vocabulary.
struct LibraryBrowser<Fields: View>: View {
    let title: String
    let introduction: String
    let kinds: [LibraryKind]
    let presentation: LibraryPresentation
    let actions: LibraryActions
    @ViewBuilder var fields: (Binding<LibraryDraft>) -> Fields
    @State private var search = ""
    @State private var selectedID: String?
    @State private var category: LibraryKind?
    @State private var draft: LibraryDraft?
    @State private var deleting: LibraryItem?
    @Environment(\.photaraTheme) private var theme

    private var items: [LibraryItem] {
        presentation.items.filter { kinds.contains($0.kind) && (category == nil || category == $0.kind)
            && (search.isEmpty || $0.searchText.localizedCaseInsensitiveContains(search)) }
    }
    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                TextField("Search \(title.lowercased())", text: $search).textFieldStyle(.roundedBorder)
                Menu { ForEach(kinds) { kind in Button("New \(kind.title)") { draft = .init(kind: kind) } } }
                    label: { Label("New", systemImage: "plus") }
                    .disabled(presentation.phase != .ready)
                Button { actions.send(.reload) } label: { Image(systemName: "arrow.clockwise") }.help("Reload Library")
            }
            if kinds.count > 1 {
                Picker("Category", selection: $category) {
                    Text("All").tag(Optional<LibraryKind>.none)
                    ForEach(kinds) { Text($0 == .client ? "Clients" : "People").tag(Optional($0)) }
                }.pickerStyle(.segmented)
            }
            ScrollView {
                VStack(alignment: .leading, spacing: 10) {
                    Text(introduction).font(.callout).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
                    switch presentation.phase {
                    case .loading:
                        ProgressView("Loading \(title.lowercased())…").frame(maxWidth: .infinity).padding(30)
                    case .failed(let message):
                        ContentUnavailableView {
                            Label("Couldn’t load \(title.lowercased())", systemImage: "exclamationmark.triangle")
                        } description: { Text(message) } actions: { Button("Try Again") { actions.send(.reload) } }
                    case .ready:
                        if items.isEmpty {
                            ContentUnavailableView {
                                Label(search.isEmpty ? "Your \(title.lowercased()) library" : "No matches", systemImage: kinds[0].symbol)
                            } description: {
                                Text(search.isEmpty ? "Create a reusable record, then assign it to any project." : "Try a different name, alias or label.")
                            } actions: {
                                if search.isEmpty { Button("New \(kinds[0].title)") { draft = .init(kind: kinds[0]) } }
                                else { Button("Clear Search") { search = "" } }
                            }
                        } else {
                            LazyVStack(spacing: 6) { ForEach(items) { item in row(item) } }
                        }
                    }
                }.frame(maxWidth: .infinity, alignment: .leading)
            }
            Text("On This Mac · \(items.count) shown").font(.caption).foregroundStyle(.secondary)
        }.padding(12)
        .sheet(item: $draft) { value in
            LibraryEditor(draft: value, actions: actions, fields: fields)
        }
        .confirmationDialog("Delete this Library record? Project snapshots will remain available.", isPresented: Binding(get: { deleting != nil }, set: { if !$0 { deleting = nil } })) {
            if let deleting { Button("Delete \(deleting.name)", role: .destructive) { actions.send(.delete(deleting)); self.deleting = nil } }
        }
    }
    private func row(_ item: LibraryItem) -> some View {
        HStack(alignment: .top, spacing: 10) {
            LibraryThumbnail(data: item.thumbnailData, name: item.name, kind: item.kind)
            VStack(alignment: .leading, spacing: 4) {
                Text(item.name).font(.headline)
                if let parent = presentation.items.first(where: { $0.id == item.parentID }) {
                    Label(parent.name, systemImage: "arrow.turn.down.right").font(.caption).foregroundStyle(.secondary)
                }
                if !item.labels.isEmpty { Text(item.labels.joined(separator: " · ")).font(.caption).foregroundStyle(.secondary) }
                if !item.detail.isEmpty { Text(item.detail).font(.caption).foregroundStyle(.secondary).lineLimit(3) }
            }.frame(maxWidth: .infinity, alignment: .leading)
            Menu {
                Button("Edit…") { draft = .init(item) }
                Button("Delete…", role: .destructive) { deleting = item }
            } label: { Image(systemName: "ellipsis") }.menuStyle(.borderlessButton).frame(width: 24)
        }.padding(12)
        .background(selectedID == item.id ? (theme?.color(.borderFocus) ?? Color.accentColor).opacity(0.10) : Color.primary.opacity(0.035), in: RoundedRectangle(cornerRadius: 10))
        .contentShape(Rectangle())
        .onTapGesture(count: 2) { draft = .init(item) }
        .onTapGesture { selectedID = item.id; actions.send(.select(item.id)) }
        .accessibilityElement(children: .contain)
    }
}
struct LibraryEditor<Fields: View>: View {
    @State var draft: LibraryDraft
    let actions: LibraryActions
    @ViewBuilder var fields: (Binding<LibraryDraft>) -> Fields
    @Environment(\.dismiss) private var dismiss
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("\(draft.recordID == nil ? "New" : "Edit") \(draft.kind.title)").font(.title2.weight(.semibold))
            Form {
                HStack {
                    LibraryThumbnail(data: draft.thumbnailSource.flatMap { try? Data(contentsOf: $0) }, name: draft.name, kind: draft.kind)
                    Button("Choose Thumbnail…") {
                        if let url = actions.chooseThumbnail() { draft.thumbnailSource = url }
                    }
                    if draft.thumbnailDigest != nil || draft.thumbnailSource != nil {
                        Button("Remove") { draft.thumbnailSource = nil; draft.thumbnailDigest = nil }
                    }
                }
                TextField("Name", text: $draft.name)
                TextField("Aliases (comma separated)", text: $draft.aliases)
                fields($draft)
            }.formStyle(.grouped)
            HStack {
                Button("Cancel") { dismiss() }.keyboardShortcut(.cancelAction)
                Spacer()
                Button("Save") { actions.send(.save(draft)); dismiss() }
                    .keyboardShortcut(.defaultAction)
                    .disabled(draft.name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        }.padding(20).frame(width: 440, height: 490)
    }
}

/// Every record reserves thumbnail geometry; missing media has an identity-specific fallback.
struct LibraryThumbnail: View {
    let data: Data?
    let name: String
    let kind: LibraryKind
    var body: some View {
        Group {
            if let data, let image = NSImage(data: data) {
                Image(nsImage: image).resizable().scaledToFill()
            } else {
                ZStack {
                    Color.accentColor.opacity(0.10)
                    VStack(spacing: 2) {
                        Image(systemName: kind.symbol).font(.system(size: 18))
                        Text(name.split(separator: " ").prefix(2).compactMap { $0.first.map(String.init) }.joined())
                            .font(.system(size: 10, weight: .semibold))
                    }.foregroundStyle(.secondary)
                }
            }
        }.frame(width: 52, height: 52).clipShape(RoundedRectangle(cornerRadius: kind == .person ? 26 : 10))
            .accessibilityLabel("Thumbnail for \(name)")
    }
}
