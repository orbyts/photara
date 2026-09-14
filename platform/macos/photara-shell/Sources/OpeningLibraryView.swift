import SwiftUI

enum OpeningLibraryDestination: String, CaseIterable, Hashable {
    case projects
    case people
    case locations
    case locationKinds

    var title: String {
        switch self {
        case .projects: "Projects"
        case .people: "People"
        case .locations: "Locations"
        case .locationKinds: "Location Kinds"
        }
    }

    var symbol: String {
        switch self {
        case .projects: "folder"
        case .people: "person.2"
        case .locations: "mappin.and.ellipse"
        case .locationKinds: "tag"
        }
    }
}

/// The opening Library hierarchy deliberately delegates sidebar rendering to
/// macOS. Photara owns its labels and content, not its material or selection UI.
struct OpeningLibraryView: View {
    private let libraryName = ReleaseConfiguration.current.identity.defaultLibraryName
    let presentation: ApplicationPresentation
    let actions: ApplicationActions
    @State private var selection: OpeningLibraryDestination? = .projects
    @StateObject private var cloud: OpeningCloudModel

    init(presentation: ApplicationPresentation, actions: ApplicationActions, cloud: OpeningCloudModel? = nil) {
        self.presentation = presentation
        self.actions = actions
        _cloud = StateObject(wrappedValue: cloud ?? OpeningCloudModel())
    }

    var body: some View {
        NavigationSplitView {
            sidebar
                .navigationSplitViewColumnWidth(min: 190, ideal: 238, max: 320)
        } detail: {
            destination
        }
        .navigationSplitViewStyle(.balanced)
        .onAppear { cloud.loadSavedState() }
    }

    private var sidebar: some View {
        VStack(spacing: 0) {
            Label(libraryName, systemImage: "books.vertical")
                .font(.headline)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, 16)
                .padding(.top, 10)
                .padding(.bottom, 8)
                .accessibilityIdentifier("opening-library-selector")

            List(selection: $selection) {
                Section("Library") {
                    ForEach(OpeningLibraryDestination.allCases, id: \.self) { destination in
                        Label(destination.title, systemImage: destination.symbol)
                            .tag(Optional(destination))
                    }
                }
            }
            .listStyle(.sidebar)
            .frame(minHeight: 0, maxHeight: .infinity)

            VStack(alignment: .leading, spacing: 8) {
                Label(cloud.hasCloudLibrary ? "Cloud Library" : "Local Library", systemImage: cloud.hasCloudLibrary ? "cloud" : "laptopcomputer")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, alignment: .leading)
                if let message = cloud.message {
                    Text(message)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                        .lineLimit(3)
                        .help(message)
                        .accessibilityIdentifier("opening-cloud-status")
                }
                SidebarAccountControls(cloud: cloud)
            }
            .font(.callout)
            .padding(.horizontal, 16)
            .padding(.top, 10)
            .padding(.bottom, 14)
            .fixedSize(horizontal: false, vertical: true)
        }
    }

    @ViewBuilder private var destination: some View {
        switch selection ?? .projects {
        case .projects:
            projects
        case .people:
            emptyDestination("People", symbol: "person.2", description: "People in \(libraryName) appear here.")
        case .locations:
            emptyDestination("Locations", symbol: "mappin.and.ellipse", description: "Locations in \(libraryName) appear here.")
        case .locationKinds:
            emptyDestination("Location Kinds", symbol: "tag", description: "Location kinds in \(libraryName) appear here.")
        }
    }

    private var projects: some View {
        VStack(spacing: 0) {
            Spacer(minLength: 40)
            VStack(spacing: 22) {
                Image(systemName: "point.3.connected.trianglepath.dotted")
                    .font(.system(size: 72, weight: .ultraLight))
                    .foregroundStyle(.secondary)
                    .accessibilityHidden(true)

                VStack(spacing: 7) {
                    Text(libraryName)
                        .font(.system(size: 38, weight: .bold))
                    Text(cloud.hasCloudLibrary ? "Your cloud library is ready." : "Your local library is ready.")
                        .font(.title3)
                        .foregroundStyle(.secondary)
                }

                VStack(spacing: 10) {
                    Button("Create New Project") { actions.send(.newProject) }
                        .accessibilityIdentifier("opening-create-project")
                        .buttonStyle(.bordered)
                        .controlSize(.large)
                    Button("Browse Projects") {
                        // The typed Projects browser is a later CXT4c fixture.
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.large)
                    Button("Open Project Package…") { actions.send(.openProject) }
                        .accessibilityIdentifier("opening-open-package")
                        .buttonStyle(.bordered)
                        .controlSize(.large)
                }
                .frame(width: 292)

                Label("Projects are saved to \(libraryName)", systemImage: "chevron.down")
                    .labelStyle(.titleAndIcon)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }
            Spacer(minLength: 40)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .foregroundStyle(.primary)
        .background(Color(nsColor: .windowBackgroundColor))
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("opening-projects")
    }

    private func emptyDestination(_ title: String, symbol: String, description: String) -> some View {
        ContentUnavailableView(title, systemImage: symbol, description: Text(description))
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(Color(nsColor: .windowBackgroundColor))
    }
}
