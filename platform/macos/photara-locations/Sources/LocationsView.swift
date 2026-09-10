import SwiftUI

struct LocationsView: View {
    let presentation: LibraryPresentation
    let actions: LibraryActions
    var body: some View {
        LibraryBrowser(title: "Locations", introduction: "Keep places and their sub-locations together, ready for your next project.", kinds: [.location], presentation: presentation, actions: actions) { draft in
            LocationsEditorFields(draft: draft, items: presentation.items)
        }
    }
}

struct LocationsEditorFields: View {
    @Binding var draft: LibraryDraft
    let items: [LibraryItem]
    var body: some View {
        Group {
            Picker("Within", selection: $draft.parentID) {
                Text("Top-level location").tag("")
                ForEach(items.filter { $0.kind == .location && $0.id != draft.recordID }) {
                    Text($0.name).tag($0.id)
                }
            }
            TextField("Address or description", text: $draft.detail, axis: .vertical).lineLimit(3...5)
            Text("Use Within to nest a studio, room, beach access point or other sub-location.").font(.caption).foregroundStyle(.secondary)
        }
    }
}
