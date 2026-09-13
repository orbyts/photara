import AppKit
import SwiftUI

/// The visual authoring surface is deliberately independent from Inspector
/// placement. It consumes immutable resolved DTOs and shared proxy references.
struct LayoutAuthoringSurfaceView: View {
    let presentation: LayoutPresentation
    let actions: LayoutActions
    @EnvironmentObject private var session: EditorSessionModel

    private var selectedNode: NodeInspection? { presentation.node }

    private func selectedFrame(in node: NodeInspection) -> LayoutFrameInspection? {
        let frames = node.layout?.frames ?? []
        return frames.first { $0.frameId == session.selectedFrameID } ?? frames.first
    }

    var body: some View {
        if let node = selectedNode, let layout = node.layout,
           let frame = selectedFrame(in: node)
        {
            VStack(spacing: 0) {
                HStack {
                    Picker("Frame", selection: Binding(
                        get: { session.selectedFrameID ?? frame.frameId },
                        set: { value in
                            session.selectedNodeID = node.nodeId
                            session.selectedFrameID = value
                            session.selectedCellID = nil
                        }
                    )) {
                        ForEach(layout.frames, id: \.frameId) { candidate in
                            Text("Frame \(candidate.index + 1)").tag(candidate.frameId)
                        }
                    }
                    .frame(maxWidth: 220)
                    Spacer()
                    Text("\(layout.canvas.widthPixels) × \(layout.canvas.heightPixels)")
                        .font(.caption.monospacedDigit())
                        .foregroundStyle(.secondary)
                }
                .padding(8)
                Divider()
                LayoutCanvasView(presentation: presentation, actions: actions, node: node, frame: frame)
                    .padding(20)
            }
            .task(id: layout.authoredStateDigest) {
                if !layout.frames.contains(where: { $0.frameId == session.selectedFrameID }) {
                    session.selectedFrameID = layout.frames.first?.frameId
                }
                let currentFrame = selectedFrame(in: node)
                if currentFrame?.cells.contains(where: {
                    $0.cellId == session.selectedCellID
                }) != true {
                    session.selectedCellID = currentFrame?.cells.first?.cellId
                }
                actions.requestPreviews(node.nodeId)
            }
        } else {
            ContentUnavailableView(
                "No Layout Selected",
                systemImage: "rectangle.3.group",
                description: Text("Add or select a Layout node to author it visually.")
            )
        }
    }
}
