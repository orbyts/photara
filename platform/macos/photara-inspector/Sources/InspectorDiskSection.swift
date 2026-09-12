import SwiftUI

extension InspectorView {

  @ViewBuilder func disk(_ node: NodeInspection) -> some View {
    if let disk = node.disk {
      Section("Folder Source") {
        LabeledContent("Accepted Assets", value: String(disk.acceptedAssetCount))
        LabeledContent("Scan", value: disk.recursive ? "Recursive" : "Top Level")
        LabeledContent("Portable Binding", value: disk.folderBindingId)
          .font(.caption.monospaced())
          .lineLimit(1)
        Button("Choose or Rebind Folder", systemImage: "folder.badge.plus") {
          actions.chooseFolder(node.nodeId)
        }
        Button("Scan Folder", systemImage: "arrow.clockwise") {
          actions.scanDisk(node.nodeId)
        }
        .disabled(presentation.isScanning)
        if presentation.isScanning {
          HStack {
            ProgressView()
              .controlSize(.small)
            Text("Fingerprinting changed files…")
              .font(.caption)
              .foregroundStyle(.secondary)
          }
        }
        Button("Connect to Available Layout", systemImage: "point.3.connected.trianglepath.dotted")
        {
          actions.connectDisk(node.nodeId)
        }
      }
    }
  }
}
