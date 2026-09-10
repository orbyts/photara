import SwiftUI

struct InspectorView: View {
    let presentation: InspectorPresentation
    let actions: InspectorActions
    var section: InspectorSection = .all
    var preset: InspectorPreset = .shipped
    @Environment(\.photaraTheme) var theme

    var selectedNode: NodeInspection? { presentation.node }
    var selectedFrame: LayoutFrameInspection? { presentation.selectedFrame }
    var selectedCell: LayoutCellInspection? { presentation.selectedCell }

    var body: some View {
        if let node = selectedNode, presentation.hasEditableSettings {
            Form {
                if section == .all || section == .identity { identity(node) }
                if section == .all || section == .disk { disk(node) }
                if section == .all || section == .inputs { inputs(node) }
                if section == .all || section == .parameters { parameters(node) }
                if section == .all || section == .frameAndCell { frameAndCell(node) }
                if section == .all || section == .outputs { outputs(node) }
                if section == .all || section == .evaluation { evaluation(node) }
                if section == .all || section == .diagnostics { diagnostics(node) }
            }
            .disabled(!presentation.actionsEnabled)
            .formStyle(.grouped)
            .scrollContentBackground(.hidden)
            .background(theme?.color(.surfacePanel) ?? Color(nsColor: .windowBackgroundColor))
        } else if selectedNode != nil {
            PhotaraEmptyStateView(preset: preset.noSettingsState)
        } else if presentation.showsGraph {
            PhotaraEmptyStateView(preset: preset.noSelectionState)
        } else {
            PhotaraEmptyStateView(preset: preset.graphHiddenState, action: actions.showGraph)
        }
    }

    var inspectorGroupBackground: Color {
        theme?.color(.surfaceElevated) ?? Color(nsColor: .controlBackgroundColor)
    }

    func canvasDescription(_ node: NodeInspection) -> String {
        guard let canvas = node.layout?.canvas else { return "Unknown" }
        return "\(canvas.widthPixels) × \(canvas.heightPixels)"
    }

    func inputPorts(_ node: NodeInspection) -> [PortInspection] {
        node.ports.filter { $0.direction == .input }
    }

    func outputPorts(_ node: NodeInspection) -> [PortInspection] {
        node.ports.filter { $0.direction == .output }
    }

    func statusColor(_ status: String) -> Color {
        switch status {
        case "Ready": theme?.color(.statusTextSuccess) ?? .green
        case "Error": theme?.color(.statusTextError) ?? .red
        default: theme?.color(.statusTextWarning) ?? .orange
        }
    }

    func arrangementBinding(
        _ node: NodeInspection,
        _ frame: LayoutFrameInspection
    ) -> Binding<LayoutArrangementChoice> {
        Binding(
            get: { LayoutArrangementChoice(frame.arrangement) },
            set: { choice in
                actions.editStructure(
                    node: node.nodeId,
                    edit: .setFrameArrangement(
                        frameId: frame.frameId,
                        arrangement: choice.action
                    )
                )
            }
        )
    }

    func contentModeBinding(
        _ node: NodeInspection,
        _ frame: LayoutFrameInspection,
        _ cell: LayoutCellInspection
    ) -> Binding<LayoutContentChoice> {
        Binding(
            get: { LayoutContentChoice(cell.contentMode) },
            set: { choice in
                let edit: LayoutCellAction = switch choice {
                case .fit: .fit(alignmentX: 500_000, alignmentY: 500_000)
                case .fill: .fill(focalX: 500_000, focalY: 500_000)
                case .crop: .crop(x: 100_000, y: 100_000, width: 800_000, height: 800_000)
                }
                actions.editCell(
                    node: node.nodeId,
                    frameID: frame.frameId,
                    cellID: cell.cellId,
                    edit: edit
                )
            }
        )
    }

    func resizedCrop(
        _ rect: LayoutNormalizedRect,
        factor: Double
    ) -> LayoutCellAction {
        let width = UInt32(max(100_000, min(1_000_000, Double(rect.width) * factor)))
        let height = UInt32(max(100_000, min(1_000_000, Double(rect.height) * factor)))
        let centerX = Int64(rect.x) + Int64(rect.width) / 2
        let centerY = Int64(rect.y) + Int64(rect.height) / 2
        let x = UInt32(max(0, min(Int64(1_000_000 - width), centerX - Int64(width) / 2)))
        let y = UInt32(max(0, min(Int64(1_000_000 - height), centerY - Int64(height) / 2)))
        return .crop(x: x, y: y, width: width, height: height)
    }
}
