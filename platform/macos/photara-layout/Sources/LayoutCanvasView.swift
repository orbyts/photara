import AppKit
import SwiftUI

struct LayoutCanvasView: View {
    let presentation: LayoutPresentation
    let actions: LayoutActions
    @EnvironmentObject private var workspace: WorkspaceModel
    @Environment(\.photaraTheme) private var theme
    let node: NodeInspection
    let frame: LayoutFrameInspection

    private var canvas: LayoutInspection.Canvas { node.layout!.canvas }

    var body: some View {
        GeometryReader { available in
            let fitted = fittedCanvas(in: available.size)
            ZStack {
                theme?.color(.workspaceSurround) ?? Color.black.opacity(0.06)
                ZStack {
                    Color.white
                    ForEach(frame.cells, id: \.cellId) { cell in
                        LayoutCanvasCell(
                            presentation: presentation, actions: actions, node: node,
                            frame: frame,
                            cell: cell,
                            canvasSize: fitted
                        )
                    }
                }
                .frame(width: fitted.width, height: fitted.height)
                .shadow(color: .black.opacity(0.18), radius: 12, y: 5)
            }
        }
    }

    private func fittedCanvas(in available: CGSize) -> CGSize {
        let width = CGFloat(canvas.widthPixels)
        let height = CGFloat(canvas.heightPixels)
        let scale = min(available.width / width, available.height / height)
        return CGSize(width: width * scale, height: height * scale)
    }
}
