import SwiftUI

struct PeopleView: View {
    let presentation: LibraryPresentation
    let actions: LibraryActions
    var body: some View {
        LibraryBrowser(title: "People", introduction: "People and clients you work with, across all your projects.", kinds: [.person, .client], presentation: presentation, actions: actions) { draft in
            PeopleEditorFields(draft: draft)
        }
    }
}

struct PeopleEditorFields: View {
    @Binding var draft: LibraryDraft
    var body: some View {
        Group {
            if draft.kind == .client {
                Picker("Client category", selection: $draft.labels) {
                    Text("Organization").tag("organization")
                    Text("Individual").tag("individual")
                }
            } else {
                TextField("Roles (model, stylist, photographer…)", text: $draft.labels)
                Text("A person is a collaborator or subject. Your account is managed in Library & Sync.").font(.caption).foregroundStyle(.secondary)
            }
        }
    }
}
