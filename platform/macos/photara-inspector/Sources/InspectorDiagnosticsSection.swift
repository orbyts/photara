import SwiftUI

extension InspectorView {

  @ViewBuilder func diagnostics(_ node: NodeInspection) -> some View {
    if !node.diagnostics.isEmpty {
      Section("Diagnostics") {
        ForEach(node.diagnostics, id: \.code) { diagnostic in
          VStack(alignment: .leading, spacing: 2) {
            Text(diagnostic.message)
            Text(diagnostic.code)
              .font(.caption.monospaced())
              .foregroundStyle(.secondary)
          }
        }
      }
      .listRowBackground(inspectorGroupBackground)
    }
  }
}
