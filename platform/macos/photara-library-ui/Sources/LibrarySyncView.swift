import SwiftUI

struct LibrarySyncView: View {
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                Label("Library & Sync", systemImage: "externaldrive").font(.title2.weight(.semibold))
                Text("Your Library is available offline on this Mac. A future account can sync it across devices.").foregroundStyle(.secondary)
                mode("On This Mac", symbol: "checkmark.circle.fill", status: "Active", detail: "People, clients, locations and scenes are stored locally on this Mac.")
                mode("Photara Cloud", symbol: "cloud", status: "Planned · unavailable", detail: "Account sign-in and cloud sync are not connected in this build.")
                mode("iCloud", symbol: "icloud", status: "Planned · unavailable", detail: "A future sync option for your Apple devices.")
                Text("An account identifies who owns a Library. People are collaborators and subjects you add to it.").font(.callout).foregroundStyle(.secondary)
            }.padding(20).frame(maxWidth: .infinity, alignment: .leading)
        }
    }
    private func mode(_ name: String, symbol: String, status: String, detail: String) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Label(name, systemImage: symbol).font(.headline)
            Text(status).font(.caption.weight(.medium)).foregroundStyle(name == "On This Mac" ? Color.accentColor : Color.secondary)
            Text(detail).font(.callout).foregroundStyle(.secondary)
        }
    }
}
