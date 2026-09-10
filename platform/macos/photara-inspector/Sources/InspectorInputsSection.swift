import SwiftUI

extension InspectorView {

  @ViewBuilder func inputs(_ node: NodeInspection) -> some View {
    if !inputPorts(node).isEmpty {
      Section("Inputs") {
        ForEach(inputPorts(node), id: \.portId) { port in
          InspectorPortView(port: port)
        }
      }
      .listRowBackground(inspectorGroupBackground)
    }
  }
}
