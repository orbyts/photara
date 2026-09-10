import Foundation
import SwiftUI

extension AppModel {
    func applicationPresentation(_ workspace: WorkspaceModel) -> ApplicationPresentation {
        return .init(hasOpenProject: hasOpenProject, title: snapshot?.title ?? "Photara Project",
            subtitle: snapshot.map { "rev \($0.projectRevision) · \($0.projectId.prefix(12))" } ?? "Not loaded",
            isDirty: snapshot?.dirty == true, nodeCount: snapshot?.nodes.count ?? 0,
            diagnosticCount: snapshot?.diagnostics.count ?? 0, progressLabel: progressLabel,
            isEvaluating: isEvaluating,
            workSurfaces: (snapshot?.nodes ?? []).compactMap { ProductionWorkSurfaceRegistry.presentation(for: $0) },
            projectID: snapshot?.projectId, nodeIDs: snapshot?.nodes.map(\.nodeId) ?? [],
            hasAssets: !(snapshot?.assets.isEmpty ?? true),
            hasAssetProducingContext: snapshot?.nodes.contains { node in
                node.ports.contains { $0.direction == .output && $0.valueTypeId == "photara.asset-set" }
            } == true,
            hasReviewableResult: snapshot?.assets.contains { galleryProxyImages[$0.assetId] != nil } == true,
            recentProjects: recentProjects.map { .init(id: $0.id, title: $0.title, lastOpened: $0.lastOpened) })
    }
    func performApplicationAction(_ action: ApplicationAction) {
        switch action {
        case .newProject: newProject()
        case .openProject: chooseAndOpenProject()
        case .closeProject: closeProject()
        case .importPair: chooseAndImportTiffPair()
        case .save: save()
        case .evaluate: evaluate()
        case .cancel: cancelEvaluation()
        case .openRecent(let id): if let recent = recentProjects.first(where: { $0.id == id }) { openRecent(recent) }
        }
    }
}

/// A node's optional work-surface contribution is not a universal Layout capability.
/// Additional node surfaces get their own host adapters when implemented.
extension BridgeNodeDto {
    var hasLayoutWorkSurface: Bool {
        hasWorkspace && workspaceContributionId == "photara.layout.workspace" && layout != nil
    }
}

/// Register native node-owned surfaces here. Adding a renderer needs no shell changes.
@MainActor
enum ProductionWorkSurfaceRegistry {
    struct Registration {
        var accepts: (BridgeNodeDto) -> Bool
        var render: (NodeWorkSurfacePresentation) -> AnyView
    }
    private static let registrations: [String: Registration] = [
        "photara.layout.workspace": .init(accepts: { $0.hasLayoutWorkSurface },
            render: { AnyView(ProductionLayoutView(nodeID: $0.nodeID)) })
    ]
    static func presentation(for node: BridgeNodeDto) -> NodeWorkSurfacePresentation? {
        guard node.hasWorkspace, let id = node.workspaceContributionId,
              registrations[id]?.accepts(node) == true else { return nil }
        return .init(nodeID: node.nodeId, contributionID: id, title: node.displayName,
            iconResourceID: node.iconResourceId, themeColorRole: node.themeColorRole, accentHex: node.accentSrgbHex)
    }
    static func view(for surface: NodeWorkSurfacePresentation) -> AnyView {
        guard let registration = registrations[surface.contributionID] else {
            return AnyView(ContentUnavailableView("Work Surface Unavailable", systemImage: "puzzlepiece.extension"))
        }
        return registration.render(surface)
    }
}
