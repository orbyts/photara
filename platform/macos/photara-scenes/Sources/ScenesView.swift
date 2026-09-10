import SwiftUI

struct ScenesView: View {
    let presentation: LibraryPresentation
    let actions: LibraryActions
    var body: some View {
        LibraryBrowser(title: "Scenes", introduction: "Reusable scene ideas. Each project assignment creates its own occurrence.", kinds: [.scene], presentation: presentation, actions: actions) { draft in
            ScenesEditorFields(draft: draft)
        }
    }
}

struct ScenesEditorFields: View {
    @Binding var draft: LibraryDraft
    var body: some View {
        Group {
            TextField("Tags (comma separated)", text: $draft.labels)
            TextField("Scene description", text: $draft.detail, axis: .vertical).lineLimit(4...6)
        }
    }
}
