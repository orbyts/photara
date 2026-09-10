import SwiftUI

struct ProjectLauncherView: View {
    let presentation: ApplicationPresentation
    let actions: ApplicationActions

    var body: some View {
        HStack(spacing: 0) {
            VStack(alignment: .leading, spacing: 22) {
                Spacer()
                Image(systemName: "point.3.connected.trianglepath.dotted")
                    .font(.system(size: 54, weight: .light))
                    .foregroundStyle(.tint)
                VStack(alignment: .leading, spacing: 6) {
                    Text("Photara")
                        .font(.largeTitle.weight(.semibold))
                    Text("Build visual workflows around your creative projects.")
                        .foregroundStyle(.secondary)
                }
                VStack(alignment: .leading, spacing: 10) {
                    Button("Create New Project", systemImage: "doc.badge.plus") {
                        actions.send(.newProject)
                    }
                    .buttonStyle(.borderedProminent)
                    .controlSize(.large)
                    Button("Open Existing Project…", systemImage: "folder") {
                        actions.send(.openProject)
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.large)
                }
                Spacer()
            }
            .frame(minWidth: 340, maxWidth: 440, maxHeight: .infinity, alignment: .leading)
            .padding(44)
            Divider()
            VStack(alignment: .leading, spacing: 12) {
                Text("Recent Projects")
                    .font(.title2.weight(.semibold))
                if presentation.recentProjects.isEmpty {
                    ContentUnavailableView(
                        "No Recent Projects",
                        systemImage: "clock",
                        description: Text("Projects you create or open will appear here.")
                    )
                } else {
                    List(presentation.recentProjects) { recent in
                        Button {
                            actions.send(.openRecent(recent.id))
                        } label: {
                            HStack(spacing: 12) {
                                Image(systemName: "doc.richtext")
                                    .font(.title2)
                                    .foregroundStyle(.tint)
                                VStack(alignment: .leading, spacing: 3) {
                                    Text(recent.title)
                                        .font(.headline)
                                    Text(recent.lastOpened, style: .relative)
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                }
                                Spacer()
                                Image(systemName: "chevron.right")
                                    .foregroundStyle(.tertiary)
                            }
                            .contentShape(Rectangle())
                            .padding(.vertical, 5)
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .padding(32)
        }
        .background(.background)
    }
}
