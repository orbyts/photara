import SwiftUI

/// One sheet item carries its immutable action and original Library/revision.
/// Switching presentation state cannot turn a Rename into a Create.
struct LocalLibraryNameRequest: Identifiable {
    enum Mode {
        case create(UUID)
        case rename(DisposableLibraryChoice)
    }
    let id = UUID()
    let mode: Mode
    var title: String {
        switch mode { case .create: "Create Library"; case .rename: "Rename Library" }
    }
    var buttonTitle: String {
        switch mode { case .create: "Create"; case .rename: "Rename" }
    }
    var initialName: String {
        switch mode { case .create: ""; case let .rename(library): library.name }
    }
}

struct LocalLibraryNameSheet: View {
    let request: LocalLibraryNameRequest
    let cancel: () -> Void
    let submit: (LocalLibraryNameRequest.Mode, String) -> Void
    @State private var name: String
    init(request: LocalLibraryNameRequest, cancel: @escaping () -> Void,
         submit: @escaping (LocalLibraryNameRequest.Mode, String) -> Void) {
        self.request = request; self.cancel = cancel; self.submit = submit
        _name = State(initialValue: request.initialName)
    }
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text(request.title).font(.headline)
            TextField("Library name", text: $name).accessibilityIdentifier("local-library-name")
            HStack {
                Button("Cancel", action: cancel).keyboardShortcut(.cancelAction)
                Spacer()
                Button(request.buttonTitle) { submit(request.mode, name) }
                    .keyboardShortcut(.defaultAction)
                    .disabled(!DisposableAutosaveProcess.libraryNameIsValid(name))
            }
        }.padding(24).frame(width: 360)
    }
}
