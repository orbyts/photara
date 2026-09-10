import SwiftUI

@main struct PeopleLabApp: App {
    @StateObject private var model = LibraryLabModel()
    var body: some Scene {
        Window("Photara People Lab", id: "people") {
            LibraryLabHost(title: "People", model: model) {
                PeopleView(presentation: model.presentation, actions: .init(send: model.send, chooseThumbnail: LibraryLabModel.chooseThumbnail))
            }
        }.defaultSize(width: 850, height: 740)
    }
}
