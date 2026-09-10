import SwiftUI

extension InspectorView {

  @ViewBuilder func parameters(_ node: NodeInspection) -> some View {
    if node.layout != nil {
      Section("Parameters") {
        LabeledContent("Canvas", value: canvasDescription(node))
        if let layout = node.layout {
          LabeledContent("Frames", value: String(layout.frames.count))
          LabeledContent("Authored digest", value: layout.authoredStateDigest)
            .font(.caption.monospaced())
        }
      }
      .listRowBackground(inspectorGroupBackground)
    }
  }
}
