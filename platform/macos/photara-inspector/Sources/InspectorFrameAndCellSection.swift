import SwiftUI

extension InspectorView {

  @ViewBuilder func frameAndCell(_ node: NodeInspection) -> some View {
    if let frame = selectedFrame, let cell = selectedCell {
      Section("Frame") {
        LabeledContent("Index", value: String(frame.index + 1))
        Picker("Arrangement", selection: arrangementBinding(node, frame)) {
          Text("One").tag(LayoutArrangementChoice.one)
          Text("Horizontal").tag(LayoutArrangementChoice.horizontal)
          Text("Vertical").tag(LayoutArrangementChoice.vertical)
          Text("Grid").tag(LayoutArrangementChoice.grid)
          if frame.arrangement == .custom {
            Text("Custom").tag(LayoutArrangementChoice.custom)
          }
        }
        HStack {
          Button("Add Cell") {
            actions.editStructure(
              node: node.nodeId,
              edit: .insertCell(
                frameId: frame.frameId,
                index: UInt64(frame.cells.count)
              )
            )
          }
          .disabled(frame.arrangement == .one || frame.arrangement == .custom)
          Button("Remove Cell") {
            actions.editStructure(
              node: node.nodeId,
              edit: .removeCell(
                frameId: frame.frameId,
                cellId: cell.cellId
              )
            )
          }
          .disabled(frame.cells.count <= 1)
        }
        HStack {
          Button("Add Frame") {
            actions.editStructure(
              node: node.nodeId,
              edit: .insertFrame(
                index: UInt64(node.layout?.frames.count ?? 0)
              )
            )
          }
          Button("Remove Frame") {
            actions.editStructure(
              node: node.nodeId,
              edit: .removeFrame(frameId: frame.frameId)
            )
          }
          .disabled((node.layout?.frames.count ?? 0) <= 1)
        }
        HStack {
          Button("Move Earlier") {
            actions.editStructure(
              node: node.nodeId,
              edit: .moveFrame(
                frameId: frame.frameId,
                toIndex: frame.index - 1
              )
            )
          }
          .disabled(frame.index == 0)
          Button("Move Later") {
            actions.editStructure(
              node: node.nodeId,
              edit: .moveFrame(
                frameId: frame.frameId,
                toIndex: frame.index + 1
              )
            )
          }
          .disabled(frame.index + 1 >= UInt64(node.layout?.frames.count ?? 0))
        }
      }
      .listRowBackground(inspectorGroupBackground)
      Section("Cell") {
        if let descriptor = presentation.previews[cell.cellId] {
          LabeledContent(
            "Preview range",
            value: descriptor.dynamicRange == .hdr ? "HDR" : "SDR"
          )
          LabeledContent("Color", value: descriptor.colorSpaceId)
        }
        Picker("Mode", selection: contentModeBinding(node, frame, cell)) {
          Text("Fit").tag(LayoutContentChoice.fit)
          Text("Fill").tag(LayoutContentChoice.fill)
          Text("Crop").tag(LayoutContentChoice.crop)
        }
        HStack {
          Text("Focal / alignment")
          Spacer()
          AlignmentPad { x, y in
            let edit: LayoutCellAction =
              cell.contentMode == .fit
              ? .fit(alignmentX: x, alignmentY: y)
              : .fill(focalX: x, focalY: y)
            actions.editCell(
              node: node.nodeId,
              frameID: frame.frameId,
              cellID: cell.cellId,
              edit: edit
            )
          }
        }
        Button("Rotate 90°") {
          actions.editCell(
            node: node.nodeId,
            frameID: frame.frameId,
            cellID: cell.cellId,
            edit: .setQuarterTurn(
              quarterTurn: nextQuarterTurn(after: cell.quarterTurn)
            )
          )
        }
        if let rect = cell.cropRect {
          LabeledContent(
            "Crop",
            value: "\(rect.x), \(rect.y) · \(rect.width) × \(rect.height)"
          )
          .font(.caption.monospaced())
          HStack {
            Button("Crop Tighter") {
              actions.editCell(
                node: node.nodeId,
                frameID: frame.frameId,
                cellID: cell.cellId,
                edit: resizedCrop(rect, factor: 0.8)
              )
            }
            Button("Crop Looser") {
              actions.editCell(
                node: node.nodeId,
                frameID: frame.frameId,
                cellID: cell.cellId,
                edit: resizedCrop(rect, factor: 1.25)
              )
            }
          }
        }
      }
      .listRowBackground(inspectorGroupBackground)
    }
  }
}
