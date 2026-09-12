import SwiftUI

extension InspectorView {

  @ViewBuilder func identity(_ node: NodeInspection) -> some View {
    Section {
      HStack(spacing: 12) {
        NodeBrandIcon(
          resourceID: node.iconResourceId,
          themeColorRole: node.themeColorRole,
          accentHex: node.accentSrgbHex,
          size: 30
        )
        VStack(alignment: .leading, spacing: 2) {
          Text(node.displayName).font(.headline)
          Text(node.status)
            .font(.caption)
            .foregroundStyle(statusColor(node.status))
        }
      }
      LabeledContent("Definition", value: node.definitionId)
        .font(.caption)
      LabeledContent("Package", value: "\(node.packageId) \(node.packageVersion)")
        .font(.caption)
      LabeledContent("Node ID", value: node.nodeId)
        .font(.caption.monospaced())
    }
  }
}
