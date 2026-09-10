import SwiftUI

struct ProjectLauncherView: View {
    let presentation: ApplicationPresentation
    let actions: ApplicationActions
    var preset: ApplicationShellPreset = .shipped
    @EnvironmentObject private var workspace: WorkspaceModel
    private var showsRecentProjects: Bool { workspace.showsRecentProjects }

    private let cardColumns = [
        GridItem(.adaptive(minimum: 220, maximum: 320), spacing: 16)
    ]

    var body: some View {
        ZStack {
            launcherBackdrop

            VStack(spacing: preset.launcherSpacing) {
                Spacer(minLength: 44)
                hero

                if showsRecentProjects {
                    recentProjects
                        .transition(.move(edge: .bottom).combined(with: .opacity))
                }

                Spacer(minLength: 32)
            }
            .padding(.horizontal, 44)
            .padding(.vertical, preset.contentInset)
        }
        .background(.background)
        .animation(.snappy(duration: 0.28), value: showsRecentProjects)
    }

    private var hero: some View {
        VStack(spacing: preset.heroSpacing) {
            ZStack {
                RoundedRectangle(cornerRadius: 28, style: .continuous)
                    .fill(Color.accentColor.opacity(0.11))
                RoundedRectangle(cornerRadius: 28, style: .continuous)
                    .stroke(Color.accentColor.opacity(0.2), lineWidth: 1)
                Image(systemName: "point.3.connected.trianglepath.dotted")
                    .font(.system(size: preset.heroIconSize, weight: .light))
                    .foregroundStyle(.tint)
            }
            .frame(width: preset.heroSize, height: preset.heroSize)
            .shadow(color: .black.opacity(0.08), radius: 22, y: 10)

            VStack(spacing: 7) {
                Text("Photara")
                    .font(.system(size: preset.launcherTitleSize, weight: preset.launcherTitleWeight.fontWeight, design: .rounded))
                Text("Build visual workflows around your creative projects.")
                    .font(.title3)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
            }

            HStack(spacing: 12) {
                Button("Create Project", systemImage: "doc.badge.plus") {
                    actions.send(.newProject)
                }
                .buttonStyle(.borderedProminent)
                .controlSize(.large)

                Button("Open Project…", systemImage: "folder") {
                    actions.send(.openProject)
                }
                .buttonStyle(.bordered)
                .controlSize(.large)

                Button {
                    workspace.showsRecentProjects.toggle()
                } label: {
                    Label(
                        showsRecentProjects ? "Hide Recents" : "Recent Projects",
                        systemImage: "clock.arrow.circlepath"
                    )
                }
                .buttonStyle(.bordered)
                .controlSize(.large)
            }

            if !showsRecentProjects, !presentation.recentProjects.isEmpty {
                Text("\(presentation.recentProjects.count) recent \(presentation.recentProjects.count == 1 ? "project" : "projects")")
                    .font(.caption)
                    .foregroundStyle(.tertiary)
            }
        }
        .frame(maxWidth: 720)
    }

    private var recentProjects: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Text("Recent Projects")
                    .font(.title2.weight(.semibold))
                Spacer()
                Text("Stored on this Mac")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            if presentation.recentProjects.isEmpty {
                ContentUnavailableView(
                    "No Recent Projects",
                    systemImage: "clock",
                    description: Text("Projects you save or open will appear here.")
                )
                .frame(maxWidth: .infinity, minHeight: 170)
                .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 18, style: .continuous))
            } else {
                ScrollView {
                    LazyVGrid(columns: cardColumns, alignment: .leading, spacing: 16) {
                        ForEach(presentation.recentProjects) { recent in
                            recentProjectCard(recent)
                        }
                    }
                    .padding(1)
                }
                .frame(maxHeight: 250)
            }
        }
        .frame(maxWidth: 980)
    }

    private func recentProjectCard(_ recent: ApplicationPresentation.Recent) -> some View {
        Button {
            actions.send(.openRecent(recent.id))
        } label: {
            HStack(spacing: 13) {
                ProjectIdentityIcon(projectID: recent.id, title: recent.title)

                VStack(alignment: .leading, spacing: 4) {
                    Text(recent.title)
                        .font(.headline)
                        .lineLimit(1)
                    Text(recent.lastOpened, style: .relative)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer(minLength: 4)
                Image(systemName: "chevron.right")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.tertiary)
            }
            .padding(14)
            .frame(maxWidth: .infinity, minHeight: 76, alignment: .leading)
            .contentShape(Rectangle())
            .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
            .overlay {
                RoundedRectangle(cornerRadius: 16, style: .continuous)
                    .stroke(Color(nsColor: .separatorColor).opacity(0.45), lineWidth: 1)
            }
        }
        .buttonStyle(.plain)
    }

    private var launcherBackdrop: some View {
        GeometryReader { geometry in
            ZStack {
                Circle()
                    .fill(Color.accentColor.opacity(0.07))
                    .frame(width: geometry.size.width * 0.58)
                    .blur(radius: 70)
                    .offset(x: geometry.size.width * 0.28, y: -geometry.size.height * 0.3)
                Circle()
                    .fill(Color.secondary.opacity(0.035))
                    .frame(width: geometry.size.width * 0.42)
                    .blur(radius: 80)
                    .offset(x: -geometry.size.width * 0.34, y: geometry.size.height * 0.32)
            }
        }
        .allowsHitTesting(false)
    }
}

private struct ProjectIdentityIcon: View {
    let projectID: String
    let title: String

    @Environment(\.photaraTheme) private var theme
    private static let identityRoles: [PhotaraThemeRole] = [
        .nodeIO, .nodeIntegration, .nodeTransform, .nodeCreative, .nodeAutomation, .nodeCompute
    ]
    private var color: Color {
        let fingerprint = projectID.unicodeScalars.reduce(UInt64(0)) {
            ($0 &* 31) &+ UInt64($1.value)
        }
        return theme?.color(Self.identityRoles[Int(fingerprint % UInt64(Self.identityRoles.count))]) ?? .accentColor
    }

    private var initials: String {
        let words = title.split(whereSeparator: { !$0.isLetter && !$0.isNumber })
        let letters = words.prefix(2).compactMap(\.first)
        return letters.isEmpty ? "P" : String(letters).uppercased()
    }

    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 12, style: .continuous)
                .fill(color.gradient)
            Text(initials)
                .font(.system(size: 15, weight: .bold, design: .rounded))
                .foregroundStyle(.white)
        }
        .frame(width: 48, height: 48)
        .accessibilityHidden(true)
    }
}
