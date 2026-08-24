import SwiftUI

/// Reusable, presentation-only Graph appearance values. These do not enter a
/// Photara project, graph digest, or node package contract.
enum PhotaraGraphPattern: String, CaseIterable, Identifiable {
    case dots
    case lines
    case crosses
    case none

    var id: String { rawValue }

    var title: String {
        switch self {
        case .dots: "Dots"
        case .lines: "Lines"
        case .crosses: "Crosses"
        case .none: "None"
        }
    }
}

struct PhotaraGraphBackgroundStyle: Equatable {
    var pattern: PhotaraGraphPattern = .dots
    var spacing: CGFloat = 24
    var opacity: Double = 0.45
    var markSize: CGFloat = 1.5
    var lineWidth: CGFloat = 0.5
    var majorInterval: Int = 0
    var majorOpacity: Double = 0
    var majorMarkSize: CGFloat = 3
    var majorLineWidth: CGFloat = 1

    static let production = Self()
}

struct PhotaraGraphBackground: View {
    @Environment(\.photaraTheme) private var theme

    let pan: CGSize
    let zoom: CGFloat
    var style: PhotaraGraphBackgroundStyle = .production
    var backgroundColor: Color?
    var minorColor: Color?
    var majorColor: Color?

    var body: some View {
        ZStack {
            (backgroundColor
                ?? theme?.color(.graphBackground)
                ?? Color(nsColor: .controlBackgroundColor))

            Canvas { context, size in
                guard style.pattern != .none else { return }
                let spacing = style.spacing * zoom
                guard spacing >= 2 else { return }
                let transformedOrigin = CGPoint(
                    x: size.width / 2 * (1 - zoom) + pan.width,
                    y: size.height / 2 * (1 - zoom) + pan.height
                )
                let minorStart = CGPoint(
                    x: phase(transformedOrigin.x, spacing: spacing),
                    y: phase(transformedOrigin.y, spacing: spacing)
                )
                let minorFade = min(1, max(0, (spacing - 4) / 8))
                let resolvedMinorColor = minorColor ?? theme?.color(.graphGrid) ?? .secondary
                let resolvedMajorColor = majorColor ?? theme?.color(.borderStrong) ?? .primary

                drawPattern(
                    context: &context,
                    size: size,
                    start: minorStart,
                    spacing: spacing,
                    color: resolvedMinorColor.opacity(style.opacity * minorFade),
                    markSize: style.markSize * zoom,
                    lineWidth: style.lineWidth
                )

                if style.majorInterval > 1, style.majorOpacity > 0 {
                    let majorSpacing = spacing * CGFloat(style.majorInterval)
                    let majorStart = CGPoint(
                        x: phase(transformedOrigin.x, spacing: majorSpacing),
                        y: phase(transformedOrigin.y, spacing: majorSpacing)
                    )
                    drawPattern(
                        context: &context,
                        size: size,
                        start: majorStart,
                        spacing: majorSpacing,
                        color: resolvedMajorColor.opacity(style.majorOpacity),
                        markSize: style.majorMarkSize * zoom,
                        lineWidth: style.majorLineWidth
                    )
                }
            }
        }
        .allowsHitTesting(false)
    }

    private func drawPattern(
        context: inout GraphicsContext,
        size: CGSize,
        start: CGPoint,
        spacing: CGFloat,
        color: Color,
        markSize: CGFloat,
        lineWidth: CGFloat
    ) {
        switch style.pattern {
        case .dots:
            let diameter = min(8, max(0.7, markSize))
            for x in stride(from: start.x, through: size.width, by: spacing) {
                for y in stride(from: start.y, through: size.height, by: spacing) {
                    context.fill(
                        Path(ellipseIn: CGRect(
                            x: x - diameter / 2,
                            y: y - diameter / 2,
                            width: diameter,
                            height: diameter
                        )),
                        with: .color(color)
                    )
                }
            }
        case .lines:
            var path = Path()
            for x in stride(from: start.x, through: size.width, by: spacing) {
                path.move(to: CGPoint(x: x, y: 0))
                path.addLine(to: CGPoint(x: x, y: size.height))
            }
            for y in stride(from: start.y, through: size.height, by: spacing) {
                path.move(to: CGPoint(x: 0, y: y))
                path.addLine(to: CGPoint(x: size.width, y: y))
            }
            context.stroke(path, with: .color(color), lineWidth: lineWidth)
        case .crosses:
            let arm = min(8, max(1, markSize * 1.8))
            var path = Path()
            for x in stride(from: start.x, through: size.width, by: spacing) {
                for y in stride(from: start.y, through: size.height, by: spacing) {
                    path.move(to: CGPoint(x: x - arm, y: y))
                    path.addLine(to: CGPoint(x: x + arm, y: y))
                    path.move(to: CGPoint(x: x, y: y - arm))
                    path.addLine(to: CGPoint(x: x, y: y + arm))
                }
            }
            context.stroke(path, with: .color(color), lineWidth: lineWidth)
        case .none:
            break
        }
    }

    private func phase(_ value: CGFloat, spacing: CGFloat) -> CGFloat {
        let remainder = value.truncatingRemainder(dividingBy: spacing)
        return remainder >= 0 ? remainder : remainder + spacing
    }
}

enum PhotaraGraphPortShape: String, CaseIterable, Identifiable {
    case round
    case pill

    var id: String { rawValue }
    var title: String { rawValue.capitalized }
}

struct PhotaraGraphNodeStyle: Equatable {
    var cornerRadius: CGFloat = 12
    var portShape: PhotaraGraphPortShape = .round
    var portOffset: CGFloat = 0
    var shadowBlur: CGFloat = 5
    var shadowOpacity: Double = 0.12
    var shadowOffsetY: CGFloat = 3
}

enum PhotaraGraphGlassTreatment: String, CaseIterable, Identifiable {
    case regular
    case clear

    var id: String { rawValue }
    var title: String { rawValue.capitalized }
}

/// Shared node surface/chrome. Content and semantic port rows are supplied by
/// the production adapter or by deterministic Graph Lab fixtures.
struct PhotaraGraphNodeSurface<Content: View, Ports: View>: View {
    @Environment(\.photaraTheme) private var theme
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency

    let style: PhotaraGraphNodeStyle
    let glassTreatment: PhotaraGraphGlassTreatment?
    let glassTint: Color
    let content: Content
    let ports: Ports

    init(
        style: PhotaraGraphNodeStyle,
        glassTreatment: PhotaraGraphGlassTreatment? = nil,
        glassTint: Color = .clear,
        @ViewBuilder content: () -> Content,
        @ViewBuilder ports: () -> Ports
    ) {
        self.style = style
        self.glassTreatment = glassTreatment
        self.glassTint = glassTint
        self.content = content()
        self.ports = ports()
    }

    var body: some View {
        ZStack {
            nodeSurface
            content.allowedDynamicRange(.standard)
        }
            .overlay { ports }
            .shadow(
                color: .black.opacity(style.shadowOpacity),
                radius: style.shadowBlur,
                y: style.shadowOffsetY
            )
    }

    @ViewBuilder
    private var nodeSurface: some View {
        let shape = RoundedRectangle(cornerRadius: style.cornerRadius, style: .continuous)
        if let glassTreatment, !reduceTransparency {
            let glass: Glass = glassTreatment == .clear ? .clear : .regular
            Color.clear
                .glassEffect(glass.tint(glassTint), in: shape)
        } else {
            shape
                .fill(theme?.color(.graphNode) ?? Color(nsColor: .controlBackgroundColor))
                .overlay {
                    shape.stroke((theme?.color(.borderStrong) ?? .secondary).opacity(0.5))
                }
        }
    }
}

struct PhotaraGraphPort: View {
    @Environment(\.photaraTheme) private var theme
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency

    let shape: PhotaraGraphPortShape
    var width: CGFloat = 12
    var height: CGFloat = 12
    var glassTreatment: PhotaraGraphGlassTreatment?
    var glassTint: Color?
    var coreColor: Color?
    var coreBrightness: Double = 0
    var showsCore = true

    var body: some View {
        Group {
            if let glassTreatment, !reduceTransparency {
                let glass: Glass = glassTreatment == .clear ? .clear : .regular
                ZStack {
                    Color.clear
                        .glassEffect(
                            glass.tint(glassTint ?? theme?.color(.borderFocus) ?? .accentColor),
                            in: portShape
                        )
                    if showsCore {
                        PhotaraGraphPortCore(
                            shape: shape,
                            width: width,
                            height: height,
                            color: coreColor ?? theme?.color(.borderFocus) ?? .accentColor,
                            brightness: coreBrightness
                        )
                    }
                }
            } else {
                portShape
                    .fill(
                        showsCore
                            ? (coreColor ?? theme?.color(.borderFocus) ?? .accentColor)
                            : (theme?.color(.graphNode) ?? Color(nsColor: .controlBackgroundColor))
                    )
                    .overlay {
                        portShape.stroke(Color.white.opacity(0.65), lineWidth: 0.75)
                    }
                    .allowedDynamicRange(.standard)
                    .overlay {
                        if showsCore {
                            PhotaraGraphPortCore(
                                shape: shape,
                                width: width,
                                height: height,
                                color: coreColor ?? theme?.color(.borderFocus) ?? .accentColor,
                                brightness: coreBrightness
                            )
                        }
                    }
            }
        }
        .frame(width: shape == .pill ? width * 1.55 : width, height: height)
        .contentShape(Rectangle().inset(by: -6))
    }

    private var portShape: AnyShape {
        switch shape {
        case .round: AnyShape(Circle())
        case .pill: AnyShape(Capsule())
        }
    }
}

struct PhotaraGraphPortCore: View {
    let shape: PhotaraGraphPortShape
    var width: CGFloat = 12
    var height: CGFloat = 12
    let color: Color
    var brightness: Double = 0

    var body: some View {
        portShape
            .fill(color)
            .brightness(brightness)
            .padding(min(width, height) * 0.32)
            .frame(width: shape == .pill ? width * 1.55 : width, height: height)
            .allowedDynamicRange(.standard)
    }

    private var portShape: AnyShape {
        switch shape {
        case .round:
            AnyShape(Circle())
        case .pill:
            AnyShape(Capsule())
        }
    }
}
