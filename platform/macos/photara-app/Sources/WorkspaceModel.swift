import Foundation
import SwiftUI

enum WorkspacePanelID: String, CaseIterable, Codable, Identifiable, Sendable {
    case assetGallery
    case graph
    case inspector
    case diagnostics

    var id: String { rawValue }

    var title: String {
        switch self {
        case .assetGallery: "Assets"
        case .graph: "Graph"
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
    @Published var galleryFilter = ""

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
        PanelPlacement(id: .assetGallery, region: .leading, order: 0, isVisible: true),
        PanelPlacement(id: .graph, region: .content, order: 0, isVisible: true),
        PanelPlacement(id: .inspector, region: .trailing, order: 0, isVisible: true),
        PanelPlacement(id: .diagnostics, region: .trailing, order: 1, isVisible: false),
    ]
}
