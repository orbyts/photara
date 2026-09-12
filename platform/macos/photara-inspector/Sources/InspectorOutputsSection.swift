import SwiftUI

extension InspectorView {

  @ViewBuilder func outputs(_ node: NodeInspection) -> some View {
    if !outputPorts(node).isEmpty {
      Section("Outputs") {
        ForEach(outputPorts(node), id: \.portId) { port in
          InspectorPortView(port: port)
        }
      }
    }
  }
}
