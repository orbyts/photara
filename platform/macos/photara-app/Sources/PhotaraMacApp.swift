import SwiftUI

@main
struct PhotaraMacApp: App {
    @StateObject private var app = AppModel()
    @StateObject private var workspace = WorkspaceModel()

    var body: some Scene {
        WindowGroup("Photara") {
            WorkspaceView()
                .environmentObject(app)
                .environmentObject(workspace)
        }
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("New Project") { app.newProject() }
                    .keyboardShortcut("n")
                Button("Reopen Last Project") { app.reopenLastProject() }
                    .keyboardShortcut("o")
                Button("Close Project") { app.closeProject() }
                    .keyboardShortcut("w")
            }
            CommandGroup(replacing: .saveItem) {
                Button("Save") { app.save() }
                    .keyboardShortcut("s")
            }
        }
    }
}
