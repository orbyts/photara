import AppKit
import SwiftUI

struct LayoutCanvasCell: View {
    let presentation: LayoutPresentation
    let actions: LayoutActions
    @EnvironmentObject private var workspace: WorkspaceModel
    let node: NodeInspection
    let frame: LayoutFrameInspection
    let cell: LayoutCellInspection
    let canvasSize: CGSize
    @State private var draftTranslation: CGSize = .zero

    private let scale = CGFloat(1_000_000)

    var body: some View {
        let rect = cell.resolvedRect
        let width = canvasSize.width * CGFloat(rect.width) / scale
        let height = canvasSize.height * CGFloat(rect.height) / scale
        let x = canvasSize.width * CGFloat(rect.x) / scale + width / 2
        let y = canvasSize.height * CGFloat(rect.y) / scale + height / 2
        ZStack {
            if let preview = presentation.previews[cell.cellId]
            {
                let descriptor = preview.metadata
                let image = preview.image
                proxyImage(image)
                    .rotationEffect(rotation(cell.quarterTurn))
                    .offset(draftTranslation)
                VStack {
                    Spacer()
                    HStack {
                        Spacer()
                        Text(descriptor.dynamicRange == .hdr ? "HDR" : "SDR")
                            .font(.caption2.bold())
                            .padding(4)
                            .background(.thinMaterial, in: Capsule())
                    }
                }
                .padding(5)
            } else if let image = presentation.nativeThumbnails[cell.cellId] {
                proxyImage(image)
                    .rotationEffect(rotation(cell.quarterTurn))
                    .offset(draftTranslation)
            } else {
                Image(systemName: cell.assetId == nil ? "plus" : "photo")
                    .font(.title2)
                    .foregroundStyle(.secondary)
            }
        }
        .frame(width: width, height: height)
        .clipped()
        .overlay {
            Rectangle()
                .stroke(
                    workspace.selectedCellID == cell.cellId ? Color.accentColor : .white.opacity(0.8),
                    lineWidth: workspace.selectedCellID == cell.cellId ? 3 : 1
                )
        }
        .contentShape(Rectangle())
        .position(x: x, y: y)
        .onTapGesture {
            workspace.selectedNodeID = node.nodeId
            workspace.selectedFrameID = frame.frameId
            workspace.selectedCellID = cell.cellId
        }
        .gesture(cropGesture(cellSize: CGSize(width: width, height: height)))
    }

    @ViewBuilder
    private func proxyImage(_ image: NSImage) -> some View {
        switch cell.contentMode {
        case .fit:
            PhotaraHDRImageView(
                image: image,
                sizingMode: .fit,
                preferredDynamicRange: .constrainedHigh
            )
        case .fill, .crop:
            PhotaraHDRImageView(
                image: image,
                sizingMode: .fill,
                preferredDynamicRange: .constrainedHigh
            )
        }
    }

    private func cropGesture(cellSize: CGSize) -> some Gesture {
        DragGesture(minimumDistance: 2)
            .onChanged { value in
                guard workspace.selectedCellID == cell.cellId, cell.cropRect != nil else { return }
                draftTranslation = value.translation
            }
            .onEnded { value in
                defer { draftTranslation = .zero }
                guard workspace.selectedCellID == cell.cellId,
                      let crop = cell.cropRect,
                      cellSize.width > 0,
                      cellSize.height > 0
                else { return }
                let dx = Int64((value.translation.width / cellSize.width * scale).rounded())
                let dy = Int64((value.translation.height / cellSize.height * scale).rounded())
                let maximumX = Int64(1_000_000 - crop.width)
                let maximumY = Int64(1_000_000 - crop.height)
                let x = UInt32(max(0, min(maximumX, Int64(crop.x) - dx)))
                let y = UInt32(max(0, min(maximumY, Int64(crop.y) - dy)))
                actions.editCell(
                    node: node.nodeId,
                    frameID: frame.frameId,
                    cellID: cell.cellId,
                    edit: .crop(x: x, y: y, width: crop.width, height: crop.height)
                )
            }
    }

    private func rotation(_ turn: LayoutQuarterTurn) -> Angle {
        switch turn {
        case .zero: .degrees(0)
        case .clockwise90: .degrees(90)
        case .clockwise180: .degrees(180)
        case .clockwise270: .degrees(270)
        }
    }
}
