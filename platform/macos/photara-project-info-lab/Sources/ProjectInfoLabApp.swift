import SwiftUI

@main struct ProjectInfoLabApp: App {
    @StateObject private var model = LibraryLabModel()
    var body: some Scene {
        Window("Photara Project Info Lab", id: "project-info") {
            LibraryLabHost(title: "Project Info", model: model) {
                ProjectInfoView(presentation: model.projectInfo, actions: .init(send: model.projectAction, chooseThumbnail: LibraryLabModel.chooseThumbnail))
            }
        }.defaultSize(width: 850, height: 740)
    }
}
