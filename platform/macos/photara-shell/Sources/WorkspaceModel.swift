import Foundation
import SwiftUI

enum WorkspacePanelID: String, CaseIterable, Codable, Identifiable, Sendable {
    case assetGallery
    case graph
    case layoutAuthoring
    case inspector
    case diagnostics

    var id: String { rawValue }

    var title: String {
        switch self {
        case .assetGallery: "Assets"
        case .graph: "Graph"
        case .layoutAuthoring: "Layout Workspace"
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
    case layout
    case review

    var title: String { rawValue.capitalized }

    var symbol: String {
        switch self {
        case .graph: "point.3.connected.trianglepath.dotted"
        case .layout: "rectangle.3.group"
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
    @Published var selectedNodeID: String?
    @Published var selectedAssetID: String?
    @Published var selectedFrameID: String?
    @Published var selectedCellID: String?
    @Published var activeWorkspaceNodeID: String?
    @Published var galleryFilter = ""
    @Published private(set) var mode: WorkspaceMode = .graph
    @Published private(set) var nodeMenuRequest: UInt64 = 0

    private static let persistenceKey = "photara.workspace.layout-authoring.v1"
    private let defaults: UserDefaults

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
        if let data = defaults.data(forKey: Self.persistenceKey),
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
        update(panel) { $0.isVisible.toggle() }
    }

    func show(_ panel: WorkspacePanelID) {
        update(panel) { $0.isVisible = true }
    }

    func activateWorkspace(for nodeID: String) {
        activeWorkspaceNodeID = nodeID
        mode = .layout
        setPrimarySurface(.layoutAuthoring)
    }

    func activateGraph() {
        mode = .graph
        setPrimarySurface(.graph)
    }

    func activateReview() {
        mode = .review
    }

    func requestNodeMenu() {
        nodeMenuRequest &+= 1
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

    private func setPrimarySurface(_ activePanel: WorkspacePanelID) {
        for index in placements.indices where [
            WorkspacePanelID.graph,
            WorkspacePanelID.layoutAuthoring,
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
        guard let data = try? JSONEncoder().encode(placements) else { return }
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
