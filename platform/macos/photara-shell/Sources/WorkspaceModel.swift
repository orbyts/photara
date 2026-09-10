import Foundation
import SwiftUI

enum WorkspacePanelID: String, CaseIterable, Codable, Identifiable, Sendable {
    case assetGallery
    case graph
    case nodeWorkSurface = "layoutAuthoring"
    static var layoutAuthoring: Self { .nodeWorkSurface } // Existing client preference/API compatibility.
    case inspector
    case diagnostics

    var id: String { rawValue }

    var title: String {
        switch self {
        case .assetGallery: "Assets"
        case .graph: "Graph"
        case .nodeWorkSurface: "Work Surface"
        case .inspector: "Inspector"
        case .diagnostics: "Diagnostics"
        }
    }
}

enum WorkspaceRegion: String, CaseIterable, Codable, Sendable {
    case leading
    case content
    case trailing
}

enum WorkspaceMode: String, CaseIterable, Sendable {
    case graph
    case nodeWorkSurface = "layout"
    static var layout: Self { .nodeWorkSurface } // Compatibility with the first Layout-only host.
    case review

    var title: String { self == .nodeWorkSurface ? "Work Surface" : rawValue.capitalized }

    var symbol: String {
        switch self {
        case .graph: "point.3.connected.trianglepath.dotted"
        case .nodeWorkSurface: "rectangle.3.group"
        case .review: "checkmark.bubble"
        }
    }
}

struct PanelPlacement: Codable, Equatable, Identifiable, Sendable {
    var id: WorkspacePanelID
    var region: WorkspaceRegion
    var order: Int
    var isVisible: Bool
}

@MainActor
final class WorkspaceModel: ObservableObject {
    @Published private(set) var placements: [PanelPlacement]
    @Published var selectedNodeID: String? {
        didSet { if selectedNodeID != nil { inspectorActivated = true } }
    }
    @Published private(set) var inspectorActivated = false
    @Published private(set) var galleryExplicitlyOpened = false
    @Published private(set) var diagnosticsExplicitlyOpened = false
    @Published var showsRecentProjects = false
    private var presentedProjectID: String?
    private var knownNodeIDs: Set<String> = []
    @Published var selectedAssetID: String?
    @Published var selectedFrameID: String?
    @Published var selectedCellID: String?
    @Published var activeWorkspaceNodeID: String?
    @Published var galleryFilter = ""
    @Published private(set) var mode: WorkspaceMode = .graph
    private var nodeMenuPending = false
    @Published private(set) var nodeMenuRequest: UInt64 = 0

    private static let persistenceKey = "photara.workspace.layout-authoring.v1"
    private let defaults: UserDefaults
    private let persists: Bool

    init(defaults: UserDefaults = .standard, persists: Bool = true) {
        self.persists = persists
        self.defaults = defaults
        if persists, let data = defaults.data(forKey: Self.persistenceKey),
           let saved = try? JSONDecoder().decode([PanelPlacement].self, from: data),
           Set(saved.map(\.id)) == Set(WorkspacePanelID.allCases)
        {
            placements = saved
        } else {
            placements = Self.layoutAuthoringPreset
        }
    }

    func visiblePanels(in region: WorkspaceRegion) -> [WorkspacePanelID] {
        placements
            .filter { $0.region == region && $0.isVisible }
            .sorted { $0.order < $1.order }
            .map(\.id)
    }

    func isVisible(_ panel: WorkspacePanelID) -> Bool {
        placements.first { $0.id == panel }?.isVisible == true
    }

    func toggle(_ panel: WorkspacePanelID) {
        if panel == .assetGallery && mode == .review { activateGraph() }
        if panel == .assetGallery { galleryExplicitlyOpened = !isVisible(panel) || !galleryExplicitlyOpened }
        if panel == .inspector { inspectorActivated = true }
        if panel == .diagnostics { diagnosticsExplicitlyOpened = true }
        update(panel) { $0.isVisible.toggle() }
    }

    func show(_ panel: WorkspacePanelID) {
        if panel == .assetGallery { galleryExplicitlyOpened = true }
        if panel == .inspector { inspectorActivated = true }
        if panel == .diagnostics { diagnosticsExplicitlyOpened = true }
        update(panel) { $0.isVisible = true }
    }

    func activateWorkspace(for nodeID: String) {
        activeWorkspaceNodeID = nodeID
        mode = .nodeWorkSurface
        setPrimarySurface(.nodeWorkSurface)
    }

    func activateGraph() {
        mode = .graph
        setPrimarySurface(.graph)
    }

    func activateReview() {
        setPrimarySurface(.graph)
        mode = .review
        show(.assetGallery)
    }

    func requestNodeMenu() {
        nodeMenuPending = true
        activateGraph()
        nodeMenuRequest &+= 1
    }

    func consumeNodeMenuRequest() -> Bool {
        let pending = nodeMenuPending
        nodeMenuPending = false
        return pending
    }

    func move(_ panel: WorkspacePanelID, to region: WorkspaceRegion) {
        let nextOrder = placements
            .filter { $0.region == region }
            .map(\.order)
            .max()
            .map { $0 + 1 } ?? 0
        update(panel) {
            $0.region = region
            $0.order = nextOrder
            $0.isVisible = true
        }
    }

    func restoreLayoutAuthoringPreset() {
        placements = Self.layoutAuthoringPreset
        mode = .graph
        persist()
    }

    /// Session-only disclosure. This never enters the document or user defaults.
    func synchronizeProject(id: String?, nodeIDs: [String]) {
        let ids = Set(nodeIDs)
        if presentedProjectID != id {
            presentedProjectID = id
            knownNodeIDs = []
            nodeMenuPending = false
            selectedNodeID = nil
            inspectorActivated = false
            galleryExplicitlyOpened = false
            diagnosticsExplicitlyOpened = false
            selectedAssetID = nil; selectedFrameID = nil; selectedCellID = nil
            activeWorkspaceNodeID = nil
            activateGraph()
        }
        let added = ids.subtracting(knownNodeIDs)
        if let first = nodeIDs.first(where: { added.contains($0) }) {
            selectedNodeID = first
            show(.inspector)
        } else if let selectedNodeID, !ids.contains(selectedNodeID) {
            self.selectedNodeID = nil
        }
        knownNodeIDs = ids
    }

    private func setPrimarySurface(_ activePanel: WorkspacePanelID) {
        for index in placements.indices where [
            WorkspacePanelID.graph,
            WorkspacePanelID.nodeWorkSurface,
        ].contains(placements[index].id) {
            placements[index].isVisible = placements[index].id == activePanel
        }
        persist()
    }

    private func update(_ panel: WorkspacePanelID, mutation: (inout PanelPlacement) -> Void) {
        guard let index = placements.firstIndex(where: { $0.id == panel }) else { return }
        mutation(&placements[index])
        persist()
    }

    private func persist() {
        guard persists, let data = try? JSONEncoder().encode(placements) else { return }
        defaults.set(data, forKey: Self.persistenceKey)
    }

    private static let layoutAuthoringPreset: [PanelPlacement] = [
        PanelPlacement(id: .inspector, region: .leading, order: 0, isVisible: true),
        PanelPlacement(id: .graph, region: .content, order: 0, isVisible: true),
        PanelPlacement(id: .layoutAuthoring, region: .content, order: 1, isVisible: false),
        PanelPlacement(id: .assetGallery, region: .trailing, order: 0, isVisible: true),
        PanelPlacement(id: .diagnostics, region: .trailing, order: 1, isVisible: false),
    ]
}
