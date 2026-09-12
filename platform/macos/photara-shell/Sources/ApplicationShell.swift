import AppKit
import SwiftUI

struct ApplicationShell<Panel: View>: View {
    let presentation: ApplicationPresentation
    let actions: ApplicationActions
    var preset: ApplicationShellPreset = .shipped
    @EnvironmentObject private var workspace: WorkspaceModel
    @Environment(\.photaraTheme) private var theme
    @Environment(\.colorScheme) private var colorScheme
    var workSurface: (NodeWorkSurfacePresentation) -> AnyView
    @ViewBuilder var panel: (WorkspacePanelID) -> Panel
    private var availability: ApplicationShellAvailability { .init(presentation: presentation) }

    var body: some View {
        Group {
            if presentation.hasOpenProject || hasLibraryPanels {
                GeometryReader { geometry in
                    VStack(spacing: preset.frame.gutter / 2) {
                        if geometry.size.width < preset.frame.compactBreakpoint {
                            compactWorkspace
                        } else {
                            regularWorkspace
                        }
                        if availability.hasStatus {
                            ProjectStatusBar(presentation: presentation, preset: preset)
                                .clipShape(RoundedRectangle(cornerRadius: preset.frame.separateStatusSurface ? preset.frame.cornerRadius : 0))
                                .padding(.horizontal, preset.frame.gutter / 2)
                        }
                    }.padding(max(0, preset.frame.outerInset - preset.frame.gutter / 2))
                }
            } else {
                ProjectLauncherView(presentation: presentation, actions: actions, preset: preset)
            }
        }
        .frame(minWidth: 760, minHeight: 560)
        .background { canvasBackground }
        .containerBackground(canvasFill, for: .window)
        .background(WindowTitleVisibilityController())
        .toolbar {
            if presentation.hasOpenProject {
                ToolbarItem(placement: .navigation) {
                    projectIdentity
                }
            }
            ToolbarItem(placement: .principal) { applicationIdentity }
            ToolbarItemGroup(placement: .primaryAction) {
                Button("Account", systemImage: "person.crop.circle") { workspace.show(.account) }.help("Account · Library & Sync")
                Button("People", systemImage: "person.2") { workspace.show(.people) }.help("People and Clients")
                Button("Locations", systemImage: "mappin.and.ellipse") { workspace.show(.locations) }.help("Locations")
                Button("Scenes", systemImage: "rectangle.stack") { workspace.show(.scenes) }.help("Scenes")
            }
            if presentation.hasOpenProject {
                ToolbarSpacer(.fixed, placement: .primaryAction)
                ToolbarItemGroup(placement: .primaryAction) {
                    workspaceModeControls
                }
                ToolbarSpacer(.fixed, placement: .primaryAction)
                ToolbarItemGroup(placement: .primaryAction) {
                    projectActionControls
                }
            } else {
                ToolbarSpacer(.fixed, placement: .primaryAction)
                ToolbarItem(placement: .primaryAction) { panelsMenu }
            }
        }
        .onAppear { synchronize() }
        .onChange(of: presentation.projectID) { synchronize() }
        .onChange(of: presentation.nodeIDs) { synchronize() }
        .onChange(of: presentation.workSurfaces) {
            if activeWorkSurface == nil {
                if workspace.mode == .nodeWorkSurface { workspace.activateGraph() }
                workspace.activeWorkspaceNodeID = presentation.workSurfaces.first?.nodeID
            }
        }
        .onChange(of: presentation.hasReviewableResult) {
            if workspace.mode == .review && !presentation.hasReviewableResult { workspace.activateGraph() }
        }
    }

    private var surfaceFill: Color {
        theme?.color(.surfacePanel) ?? Color(nsColor: .controlBackgroundColor)
    }
    private var canvasFill: Color {
        theme?.color(.surfaceCanvas) ?? Color(nsColor: .windowBackgroundColor)
    }
    private var applicationIdentity: some View {
        Text(preset.toolbarApplicationTitle)
            .font(.system(size: preset.toolbarApplicationTitleSize, weight: .semibold))
            .foregroundStyle(theme?.color(.textPrimary) ?? Color.primary)
            .lineLimit(1)
            .accessibilityIdentifier("application-title")
    }
    private var projectIdentity: some View {
        HStack(spacing: 8) {
            if preset.toolbarShowsProjectThumbnail {
                projectThumbnail
            }
            if preset.toolbarShowsProjectTitle {
                Text(presentation.title)
                    .font(.system(size: preset.toolbarTitleSize,
                                  weight: preset.toolbarTitleWeight.fontWeight))
                    .foregroundStyle(theme?.color(.textPrimary) ?? Color.primary)
                    .lineLimit(1)
            }
        }
        .frame(maxWidth: preset.toolbarIdentityWidth, alignment: .leading)
        .help(presentation.title)
        .accessibilityIdentifier("project-identity")
    }
    @ViewBuilder private var projectThumbnail: some View {
        let size = preset.toolbarProjectThumbnailSize
        if let url = presentation.projectThumbnailURL,
           let image = NSImage(contentsOf: url) {
            Image(nsImage: image)
                .resizable()
                .scaledToFill()
                .frame(width: size, height: size)
                .clipShape(RoundedRectangle(cornerRadius: preset.toolbarProjectThumbnailCornerRadius))
        } else {
            RoundedRectangle(cornerRadius: preset.toolbarProjectThumbnailCornerRadius)
                .fill(surfaceFill)
                .frame(width: size, height: size)
                .overlay {
                    Image(systemName: "photo")
                        .font(.system(size: size * 0.48, weight: .medium))
                        .foregroundStyle(theme?.color(.textSecondary) ?? Color.secondary)
                }
        }
    }
    @ViewBuilder private var workspaceModeControls: some View {
        Button { workspace.activateGraph() } label: {
            Image(systemName: WorkspaceMode.graph.symbol)
        }.help("Graph").accessibilityLabel("Graph")
        ForEach(presentation.workSurfaces) { surface in
            Button {
                workspace.selectedNodeID = surface.nodeID
                workspace.activateWorkspace(for: surface.nodeID)
            } label: {
                NodeBrandIcon(resourceID: surface.iconResourceID,
                    themeColorRole: surface.themeColorRole, accentHex: surface.accentHex, size: 24)
            }
            .help("\(surface.title) · \(surface.nodeID.prefix(8))")
            .accessibilityLabel("\(surface.title) work surface")
            .accessibilityIdentifier("node-work-surface-\(surface.nodeID)")
        }
        if presentation.hasReviewableResult {
            Button { workspace.activateReview() } label: {
                Image(systemName: WorkspaceMode.review.symbol)
            }.help("Review").accessibilityLabel("Review")
        }
    }
    @ViewBuilder private var projectActionControls: some View {
        Button("Add Node", systemImage: "plus") { workspace.requestNodeMenu() }
        if presentation.isEvaluating {
            Button("Cancel", systemImage: "stop.fill") { actions.send(.cancel) }
        } else if presentation.nodeCount > 0 {
            Button("Run", systemImage: "play.fill") { actions.send(.evaluate) }
        }
        Button("Save", systemImage: "square.and.arrow.down") { actions.send(.save) }
            .disabled(!presentation.isDirty)
        panelsMenu
    }
    private var canvasBackground: some View { canvasFill }
    @ViewBuilder private var primaryRegion: some View {
        if !presentation.hasOpenProject && displayedPanels(in: .content).isEmpty {
            ProjectLauncherView(presentation: presentation, actions: actions, preset: preset)
        } else if displayedPanels(in: .content).isEmpty && workspace.mode != .review {
            VStack(spacing: 12) {
                Text("Make room for your work").font(.title2)
                Text("Open a module from Workspace, or restore the default workspace.").foregroundStyle(.secondary)
                Button("Restore Workspace") { workspace.restoreLayoutAuthoringPreset() }
            }.frame(maxWidth: .infinity, maxHeight: .infinity)
        } else { region(.content) }
    }
    private var hasLibraryPanels: Bool {
        WorkspaceRegion.allCases.contains { !displayedPanels(in: $0).isEmpty }
    }
    private var hasSidePanels: Bool {
        !displayedPanels(in: .leading).isEmpty || !displayedPanels(in: .trailing).isEmpty
    }
    private var hasLeadingPanels: Bool { !displayedPanels(in: .leading).isEmpty }
    private var hasTrailingPanels: Bool { !displayedPanels(in: .trailing).isEmpty }

    @ViewBuilder private var regularWorkspace: some View {
        if hasLeadingPanels && hasTrailingPanels {
            PhotaraTransientSplit(axis: .horizontal, sizing: .first(preset.leadingIdealWidth),
                                  minimumFirst: 230, minimumSecond: 600, gutter: preset.frame.gutter) {
                region(.leading)
            } second: {
                PhotaraTransientSplit(axis: .horizontal, sizing: .second(preset.trailingIdealWidth),
                                      minimumFirst: 320, minimumSecond: 280, gutter: preset.frame.gutter) {
                    primaryRegion
                } second: {
                    region(.trailing)
                }
            }
        } else if hasLeadingPanels {
            PhotaraTransientSplit(axis: .horizontal, sizing: .first(preset.leadingIdealWidth),
                                  minimumFirst: 230, minimumSecond: 320, gutter: preset.frame.gutter) {
                region(.leading)
            } second: {
                primaryRegion
            }
        } else if hasTrailingPanels {
            PhotaraTransientSplit(axis: .horizontal, sizing: .second(preset.trailingIdealWidth),
                                  minimumFirst: 320, minimumSecond: 280, gutter: preset.frame.gutter) {
                primaryRegion
            } second: {
                region(.trailing)
            }
        } else {
            primaryRegion
        }
    }

    @ViewBuilder private var compactWorkspace: some View {
        if hasSidePanels {
            PhotaraTransientSplit(axis: .vertical, sizing: .fraction(0.58),
                                  minimumFirst: 240, minimumSecond: 160, gutter: preset.frame.gutter) {
                primaryRegion
            } second: {
                compactSideRegions
            }
        } else {
            primaryRegion
        }
    }

    @ViewBuilder private var compactSideRegions: some View {
        if hasLeadingPanels && hasTrailingPanels {
            PhotaraTransientSplit(axis: .horizontal, sizing: .fraction(0.5),
                                  minimumFirst: 230, minimumSecond: 230, gutter: preset.frame.gutter) {
                region(.leading)
            } second: {
                region(.trailing)
            }
        } else if hasLeadingPanels {
            region(.leading)
        } else {
            region(.trailing)
        }
    }
    private func synchronize() {
        workspace.synchronizeProject(id: presentation.projectID, nodeIDs: presentation.nodeIDs)
        if workspace.activeWorkspaceNodeID == nil { workspace.activeWorkspaceNodeID = presentation.workSurfaces.first?.nodeID }
    }
    private var activeWorkSurface: NodeWorkSurfacePresentation? {
        presentation.workSurfaces.first { $0.nodeID == workspace.activeWorkspaceNodeID }
    }
    private func displayedPanels(in region: WorkspaceRegion) -> [WorkspacePanelID] {
        availability.visiblePanels(in: region, workspace: workspace)
            .filter { workspace.mode != .review || $0 != .assetGallery }
    }
    @ViewBuilder private func region(_ region: WorkspaceRegion) -> some View {
        let panels = displayedPanels(in: region)
        if workspace.mode == .review && region == .content {
            panelView(.assetGallery).frame(minWidth: 320, maxWidth: .infinity, minHeight: 160, maxHeight: .infinity)
        } else if !panels.isEmpty {
            Group {
                if panels.count > 2 {
                    ScrollViewReader { proxy in
                        ScrollView {
                            VStack(spacing: 0) {
                                ForEach(panels) { id in panelView(id).frame(height: 360).id(id) }
                            }
                        }
                        .onChange(of: workspace.focusedPanel) {
                            if let focused = workspace.focusedPanel { proxy.scrollTo(focused, anchor: .top) }
                        }
                    }
                } else if panels.count == 2 {
                    PhotaraTransientSplit(axis: .vertical, sizing: .fraction(0.5),
                                          minimumFirst: 160, minimumSecond: 160, gutter: preset.frame.gutter) {
                        panelView(panels[0])
                    } second: {
                        panelView(panels[1])
                    }
                } else if let panel = panels.first {
                    panelView(panel)
                }
            }
            .frame(minWidth: region == .content ? 320 : 230,
                   idealWidth: region == .leading ? preset.leadingIdealWidth : region == .trailing ? preset.trailingIdealWidth : 680,
                   maxWidth: .infinity,
                   minHeight: 160, maxHeight: .infinity)
        }
    }
    private func panelView(_ id: WorkspacePanelID) -> some View {
        let isFocused = workspace.focusedPanel == id
        return VStack(spacing: 0) {
            PanelHeader(panel: id, height: preset.panelHeaderHeight,
                titleSize: preset.panelHeaderTitleSize,
                horizontalPadding: preset.panelHeaderHorizontalInset,
                elevated: false,
                activeFill: isFocused
                    ? (theme?.color(.selectionBackground) ?? Color.accentColor).opacity(0.12)
                    : Color.clear,
                allowsPlacement: true,
                title: id == .nodeWorkSurface ? activeWorkSurface?.title : nil)
            hostedPanel(id).padding(preset.frame.contentInset).frame(maxWidth: .infinity, maxHeight: .infinity).overlay {
                if id == .graph && presentation.nodeCount == 0 {
                    VStack(spacing: 12) {
                        Text("Build your workflow").font(.title2.weight(.semibold))
                        Text("Add your first node to begin automating this project.")
                            .foregroundStyle(.secondary).multilineTextAlignment(.center)
                        Button("Add Node", systemImage: "plus") { workspace.requestNodeMenu() }
                            .buttonStyle(.borderedProminent)
                    }.padding(24).background(.regularMaterial, in: RoundedRectangle(cornerRadius: 18))
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .foregroundStyle(theme?.color(.textPrimary) ?? Color.primary)
        .background(surfaceFill)
        .clipShape(RoundedRectangle(cornerRadius: preset.frame.cornerRadius))
        .shadow(color: .black.opacity(colorScheme == .dark ? 0.18 : 0.07), radius: preset.frame.elevation, y: preset.frame.elevation / 2)
        .padding(preset.frame.gutter / 2)
        .accessibilityIdentifier("workspace-module-\(id.rawValue)")
    }
    @ViewBuilder private func hostedPanel(_ id: WorkspacePanelID) -> some View {
        if id == .nodeWorkSurface {
            if let surface = activeWorkSurface { workSurface(surface) }
        } else {
            panel(id)
        }
    }
    private var panelsMenu: some View {
        Menu("Workspace", systemImage: "rectangle.3.group") {
            ForEach(availability.panels) { id in
                Group {
                    Toggle(isOn: Binding(
                        get: { WorkspaceRegion.allCases.contains { availability.visiblePanels(in: $0, workspace: workspace).contains(id) } },
                        set: { visible in if visible {
                            if id == .nodeWorkSurface, workspace.activeWorkspaceNodeID == nil { workspace.activeWorkspaceNodeID = presentation.workSurfaces.first?.nodeID }
                            workspace.show(id)
                        } else { workspace.toggle(id) } })) { Label(id.title, systemImage: id.symbol) }
                }
            }
            Divider()
            if presentation.hasOpenProject {
            Button("Import Pair…", systemImage: "photo.badge.plus") { actions.send(.importPair) }
            Button("Close Project") { actions.send(.closeProject) }
            }
            Button("Restore Workspace") { workspace.restoreLayoutAuthoringPreset() }
        }
    }
}

private struct WindowTitleVisibilityController: NSViewRepresentable {
    func makeNSView(context: Context) -> WindowTitleObserverView { WindowTitleObserverView() }
    func updateNSView(_ nsView: WindowTitleObserverView, context: Context) { nsView.apply() }
}

private final class WindowTitleObserverView: NSView {
    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        apply()
    }

    func apply() {
        window?.titleVisibility = .hidden
    }
}
