import SwiftUI

@main struct ScenesLabApp: App {
    @StateObject private var model = LibraryLabModel()
    var body: some Scene {
        Window("Photara Scenes Lab", id: "scenes") {
            LibraryLabHost(title: "Scenes", model: model) {
                ScenesView(presentation: model.presentation, actions: .init(send: model.send, chooseThumbnail: LibraryLabModel.chooseThumbnail))
            }
        }.defaultSize(width: 850, height: 740)
    }
}
