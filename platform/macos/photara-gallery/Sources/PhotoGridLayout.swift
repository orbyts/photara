import AppKit
import SwiftUI

struct GalleryAspectRatioKey: LayoutValueKey {
    static let defaultValue: CGFloat = 1
}

/// A compact justified photo layout. Unknown items begin square; when a native
/// or project proxy supplies dimensions, the affected rows adopt their actual
/// aspect ratios without ever drawing outside their assigned frames.
struct PhotoGridLayout: Layout {
    let spacing: CGFloat
    let targetRowHeight: CGFloat

    func sizeThatFits(
        proposal: ProposedViewSize,
        subviews: Subviews,
        cache: inout ()
    ) -> CGSize {
        let width = max(proposal.width ?? 480, 1)
        let result = layout(width: width, subviews: subviews)
        return CGSize(width: width, height: result.height)
    }

    func placeSubviews(
        in bounds: CGRect,
        proposal: ProposedViewSize,
        subviews: Subviews,
        cache: inout ()
    ) {
        let result = layout(width: max(bounds.width, 1), subviews: subviews)
        for (index, frame) in result.frames.enumerated() {
            subviews[index].place(
                at: CGPoint(x: bounds.minX + frame.minX, y: bounds.minY + frame.minY),
                anchor: .topLeading,
                proposal: ProposedViewSize(width: frame.width, height: frame.height)
            )
        }
    }

    private func layout(width: CGFloat, subviews: Subviews) -> (frames: [CGRect], height: CGFloat) {
        guard !subviews.isEmpty else { return ([], 0) }
        var rows: [[Int]] = []
        var row: [Int] = []
        var ratioSum: CGFloat = 0

        for index in subviews.indices {
            let ratio = normalized(subviews[index][GalleryAspectRatioKey.self])
            row.append(index)
            ratioSum += ratio
            let occupied = ratioSum * targetRowHeight
                + spacing * CGFloat(max(row.count - 1, 0))
            if occupied >= width {
                rows.append(row)
                row = []
                ratioSum = 0
            }
        }
        if !row.isEmpty { rows.append(row) }

        var frames = Array(repeating: CGRect.zero, count: subviews.count)
        var y: CGFloat = 0
        for (rowIndex, indices) in rows.enumerated() {
            let sum = indices.reduce(CGFloat.zero) {
                $0 + normalized(subviews[$1][GalleryAspectRatioKey.self])
            }
            let available = max(width - spacing * CGFloat(max(indices.count - 1, 0)), 1)
            let isLastIncompleteRow = rowIndex == rows.count - 1
                && sum * targetRowHeight < available
            let height = isLastIncompleteRow ? targetRowHeight : available / max(sum, 0.01)
            var x: CGFloat = 0
            for index in indices {
                let itemWidth = height * normalized(subviews[index][GalleryAspectRatioKey.self])
                frames[index] = CGRect(x: x, y: y, width: itemWidth, height: height)
                x += itemWidth + spacing
            }
            y += height + spacing
        }
        return (frames, max(y - spacing, 0))
    }

    private func normalized(_ ratio: CGFloat) -> CGFloat {
        guard ratio.isFinite else { return 1 }
        return min(max(ratio, 0.35), 4)
    }
}
