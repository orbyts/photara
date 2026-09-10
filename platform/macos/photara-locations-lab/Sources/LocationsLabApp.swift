import SwiftUI

@main struct LocationsLabApp: App {
    @StateObject private var model = LibraryLabModel()
    var body: some Scene {
        Window("Photara Locations Lab", id: "locations") {
            LibraryLabHost(title: "Locations", model: model) {
                LocationsView(presentation: model.presentation, actions: .init(send: model.send, chooseThumbnail: LibraryLabModel.chooseThumbnail))
            }
        }.defaultSize(width: 850, height: 740)
    }
}
