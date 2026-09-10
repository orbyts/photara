import AppKit
import SwiftUI

struct ApplicationShell<Panel: View>: View {
    let presentation: ApplicationPresentation
    let actions: ApplicationActions
    var preset: ApplicationShellPreset = .shipped
    @EnvironmentObject private var workspace: WorkspaceModel
    @Environment(\.photaraTheme) private var theme
    var workSurface: (NodeWorkSurfacePresentation) -> AnyView
    @ViewBuilder var panel: (WorkspacePanelID) -> Panel
    private var availability: ApplicationShellAvailability { .init(presentation: presentation) }

    var body: some View {
        Group {
            if presentation.hasOpenProject {
                GeometryReader { geometry in
                    VStack(spacing: 0) {
                        if geometry.size.width < 1100 {
                            // Compact retains every disclosed pane without squeezing the Graph.
                            VSplitView {
                                region(.content)
                                if hasSidePanels {
                                    HSplitView { region(.leading, expanded: true); region(.trailing, expanded: true) }
                                }
                            }
                        } else {
                            HSplitView { region(.leading); region(.content); region(.trailing) }
                        }
                        if availability.hasStatus {
                            Divider()
                            ProjectStatusBar(presentation: presentation, preset: preset)
                        }
                    }
                }
            } else {
                ProjectLauncherView(presentation: presentation, actions: actions, preset: preset)
            }
        }
        .frame(minWidth: 760, minHeight: 560)
        .background(theme?.color(.surfaceCanvas) ?? Color(nsColor: .windowBackgroundColor))
        .toolbar {
            if presentation.hasOpenProject {
                ToolbarItem(placement: .navigation) {
                    Text(presentation.title).font(.headline).lineLimit(1)
                        .frame(maxWidth: preset.toolbarIdentityWidth, alignment: .leading)
                        .help(presentation.title)
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
            }
        }
        .onAppear { synchronize() }
        .onChange(of: presentation.projectID) { synchronize() }
        .onChange(of: presentation.nodeIDs) { synchronize() }
        .onChange(of: presentation.workSurfaces) {
            if workspace.mode == .nodeWorkSurface && activeWorkSurface == nil { workspace.activateGraph() }
        }
        .onChange(of: presentation.hasReviewableResult) {
            if workspace.mode == .review && !presentation.hasReviewableResult { workspace.activateGraph() }
        }
    }

    private var hasSidePanels: Bool {
        !displayedPanels(in: .leading).isEmpty || !displayedPanels(in: .trailing).isEmpty
    }
    private func synchronize() {
        workspace.synchronizeProject(id: presentation.projectID, nodeIDs: presentation.nodeIDs)
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
            VSplitView {
                ForEach(panels) { id in panelView(id) }
            }
            .frame(minWidth: region == .content ? 320 : 230,
                   idealWidth: region == .leading ? preset.leadingIdealWidth : region == .trailing ? preset.trailingIdealWidth : 680,
                   maxWidth: expanded || region == .content || panels.contains(.graph) || panels.contains(.nodeWorkSurface) ? .infinity : region == .leading ? preset.leadingIdealWidth : preset.trailingIdealWidth,
                   minHeight: 160, maxHeight: .infinity)
        }
    }
    private func panelView(_ id: WorkspacePanelID) -> some View {
        VStack(spacing: 0) {
            if id != .graph || presentation.nodeCount > 0 {
                PanelHeader(panel: id, height: preset.panelHeaderHeight, allowsPlacement: workspace.mode != .review || id != .assetGallery, title: id == .nodeWorkSurface ? activeWorkSurface?.title : nil)
                Divider()
            }
            hostedPanel(id).frame(maxWidth: .infinity, maxHeight: .infinity).overlay {
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
        .background(theme?.color(.surfacePanel) ?? Color(nsColor: .windowBackgroundColor))
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
                if id != .graph && id != .nodeWorkSurface {
                    Toggle(id.title, isOn: Binding(
                        get: { WorkspaceRegion.allCases.contains { availability.visiblePanels(in: $0, workspace: workspace).contains(id) } },
                        set: { visible in if visible { workspace.show(id) } else { workspace.toggle(id) } }))
                }
            }
            Divider()
            Button("Import Pair…", systemImage: "photo.badge.plus") { actions.send(.importPair) }
            Button("Close Project") { actions.send(.closeProject) }
            Button("Restore Workspace") { workspace.restoreLayoutAuthoringPreset() }
        }
    }
}
