import SwiftUI

enum PhotaraSplitSizing {
    case first(Double)
    case second(Double)
    case fraction(Double)
}

/// A native-feeling split whose hit target lives in the surface gutter. The
/// resize indicator is intentionally absent at rest and fades after dragging.
struct PhotaraTransientSplit<First: View, Second: View>: View {
    let axis: Axis
    let sizing: PhotaraSplitSizing
    let minimumFirst: Double
    let minimumSecond: Double
    let gutter: Double
    @ViewBuilder var first: () -> First
    @ViewBuilder var second: () -> Second

    @Environment(\.photaraTheme) private var theme
    @State private var storedFirstLength: Double?
    @State private var dragStart: Double = 0
    @State private var isDragging = false

    var body: some View {
        GeometryReader { geometry in
            let available = axis == .horizontal ? geometry.size.width : geometry.size.height
            let firstLength = resolvedFirstLength(available: available)
            Group {
                if axis == .horizontal {
                    HStack(spacing: 0) {
                        first().frame(width: firstLength)
                        handle(available: available, current: firstLength)
                        second().frame(maxWidth: .infinity)
                    }
                } else {
                    VStack(spacing: 0) {
                        first().frame(height: firstLength)
                        handle(available: available, current: firstLength)
                        second().frame(maxHeight: .infinity)
                    }
                }
            }
        }
    }

    private func resolvedFirstLength(available: Double) -> Double {
        let proposed = storedFirstLength ?? {
            switch sizing {
            case let .first(value): value
            case let .second(value): available - value
            case let .fraction(value): available * value
            }
        }()
        return min(max(proposed, minimumFirst), max(minimumFirst, available - minimumSecond))
    }

    private func handle(available: Double, current: Double) -> some View {
        Color.clear
            .frame(width: axis == .horizontal ? 0 : nil,
                   height: axis == .vertical ? 0 : nil)
            .overlay {
                Rectangle()
                    .fill(theme?.color(.borderFocus) ?? Color.accentColor)
                    .frame(width: axis == .horizontal ? 1 : max(10, gutter),
                           height: axis == .vertical ? 1 : max(10, gutter))
                    .opacity(isDragging ? 0.72 : 0)
                    .animation(.easeOut(duration: isDragging ? 0.08 : 0.42), value: isDragging)
                    .frame(width: axis == .horizontal ? max(10, gutter) : nil,
                           height: axis == .vertical ? max(10, gutter) : nil)
                    .contentShape(Rectangle())
                    .gesture(
                        DragGesture(minimumDistance: 1)
                            .onChanged { value in
                                if !isDragging {
                                    dragStart = current
                                    isDragging = true
                                }
                                let translation = axis == .horizontal
                                    ? value.translation.width : value.translation.height
                                storedFirstLength = min(
                                    max(dragStart + translation, minimumFirst),
                                    max(minimumFirst, available - minimumSecond)
                                )
                            }
                            .onEnded { _ in
                                withAnimation(.easeOut(duration: 0.42)) { isDragging = false }
                            }
                    )
            }
            .zIndex(20)
            .accessibilityLabel(axis == .horizontal ? "Resize columns" : "Resize rows")
    }
}
