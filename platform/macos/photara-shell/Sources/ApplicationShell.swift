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
                            // Compact retains every disclosed pane without squeezing the Graph.
                            VSplitView {
                                primaryRegion
                                if hasSidePanels {
                                    HSplitView { region(.leading, expanded: true); region(.trailing, expanded: true) }
                                }
                            }
                        } else {
                            HSplitView { region(.leading); primaryRegion; region(.trailing) }
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
        .toolbar {
            ToolbarItemGroup(placement: .primaryAction) {
                Button("Account", systemImage: "person.crop.circle") { workspace.show(.account) }.help("Account · Library & Sync")
                Button("People", systemImage: "person.2") { workspace.show(.people) }.help("People and Clients")
                Button("Locations", systemImage: "mappin.and.ellipse") { workspace.show(.locations) }.help("Locations")
                Button("Scenes", systemImage: "rectangle.stack") { workspace.show(.scenes) }.help("Scenes")
            }
            if presentation.hasOpenProject {
                ToolbarItem(placement: .navigation) {
                    if preset.toolbarShowsProjectTitle {
                        Text(presentation.title)
                            .font(.system(size: preset.toolbarTitleSize,
                                          weight: preset.toolbarTitleWeight.fontWeight))
                            .foregroundStyle(theme?.color(.textPrimary) ?? Color.primary)
                            .lineLimit(1)
                            .frame(maxWidth: preset.toolbarIdentityWidth, alignment: .leading)
                            .help(presentation.title)
                    }
                }
                ToolbarItem(placement: .principal) {
                    if !presentation.workSurfaces.isEmpty || presentation.hasReviewableResult {
                        HStack(spacing: 8) {
                            Button { workspace.activateGraph() } label: {
                                Image(systemName: WorkspaceMode.graph.symbol)
                            }.help("Graph").accessibilityLabel("Graph")
                            ForEach(presentation.workSurfaces) { surface in
                                Button {
                                    workspace.selectedNodeID = surface.nodeID
                                    workspace.activateWorkspace(for: surface.nodeID)
                                } label: {
                                    NodeBrandIcon(resourceID: surface.iconResourceID,
                                        themeColorRole: surface.themeColorRole, accentHex: surface.accentHex, size: 26)
                                        .overlay {
                                            if workspace.mode == .nodeWorkSurface && workspace.activeWorkspaceNodeID == surface.nodeID {
                                                RoundedRectangle(cornerRadius: 6).stroke(.tint, lineWidth: 1.5)
                                            }
                                        }
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
                        }.buttonStyle(.borderless)
                    }
                }
                ToolbarItemGroup(placement: .primaryAction) {
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
            } else {
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
        preset.frame.usesThemeFills ? (theme?.color(.surfacePanel) ?? Color(nsColor: .controlBackgroundColor)) : preset.frame.surfaceFill.color(colorScheme)
    }
    @ViewBuilder private var canvasBackground: some View {
        let fill = preset.frame.usesThemeFills ? (theme?.color(.surfaceCanvas) ?? Color(nsColor: .windowBackgroundColor)) : preset.frame.canvasFill.color(colorScheme)
        switch preset.frame.canvasMaterial {
        case .theme: fill
        case .ultraThin: Rectangle().fill(.ultraThinMaterial).overlay(fill.opacity(0.35))
        case .thin: Rectangle().fill(.thinMaterial).overlay(fill.opacity(0.35))
        case .regular: Rectangle().fill(.regularMaterial).overlay(fill.opacity(0.35))
        case .thick: Rectangle().fill(.thickMaterial).overlay(fill.opacity(0.35))
        }
    }
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
    @ViewBuilder private func region(_ region: WorkspaceRegion, expanded: Bool = false) -> some View {
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
                } else {
                    VSplitView { ForEach(panels) { id in panelView(id) } }
                }
            }
            .frame(minWidth: region == .content ? 320 : 230,
                   idealWidth: region == .leading ? preset.leadingIdealWidth : region == .trailing ? preset.trailingIdealWidth : 680,
                   maxWidth: expanded || region == .content || panels.contains(.graph) || panels.contains(.nodeWorkSurface) ? .infinity : region == .leading ? preset.leadingIdealWidth : preset.trailingIdealWidth,
                   minHeight: 160, maxHeight: .infinity)
        }
    }
    private func panelView(_ id: WorkspacePanelID) -> some View {
        VStack(spacing: 0) {
            Group {
                PanelHeader(panel: id, height: preset.panelHeaderHeight,
                    titleSize: preset.panelHeaderTitleSize,
                    horizontalPadding: preset.panelHeaderHorizontalInset,
                    elevated: preset.frame.elevatedHeader,
                    allowsPlacement: true,
                    title: id == .nodeWorkSurface ? activeWorkSurface?.title : nil)
                PhotaraShellDivider(thickness: preset.dividerThickness)
            }
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
        .overlay {
            RoundedRectangle(cornerRadius: preset.frame.cornerRadius)
                .strokeBorder(workspace.focusedPanel == id ? (theme?.color(.borderFocus) ?? Color.accentColor) : (theme?.color(.borderSubtle) ?? Color.gray.opacity(0.3)),
                    lineWidth: workspace.focusedPanel == id ? preset.frame.activeEmphasis : preset.frame.borderWidth)
                .allowsHitTesting(false)
        }
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

struct PhotaraShellDivider: View {
    @Environment(\.photaraTheme) private var theme
    let thickness: Double
    var body: some View {
        Rectangle()
            .fill(theme?.color(.borderSubtle) ?? Color(nsColor: .separatorColor))
            .frame(height: thickness)
            .accessibilityHidden(true)
    }
}
