import SwiftUI

extension InspectorView {

  @ViewBuilder func evaluation(_ node: NodeInspection) -> some View {
    Section("Evaluation") {
      LabeledContent("State", value: node.status)
      LabeledContent(
        "Graph revision",
        value: String(presentation.graphRevision)
      )
      LabeledContent("Progress", value: presentation.progressLabel)
    }
    .listRowBackground(inspectorGroupBackground)
  }
}
