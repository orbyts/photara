import SwiftUI

/// A Lab-only optical stress scene. The Graph backdrop is deliberately a sibling
/// below one glass container so SwiftUI can sample it; no production Graph view
/// or node implementation participates in this prototype.
struct GlassTestScene: View {
    let theme: PhotaraResolvedTheme

    @Environment(\.accessibilityReduceTransparency) private var systemReducesTransparency
    @State private var forcesFallback = false
    @State private var sceneMode = GlassLabSceneMode.composition
    @State private var gridStyle = GlassLabGridStyle.squares
    @State private var glassVariant = GlassLabVariant.clear
    @State private var showsControls = true
    @State private var parameters = GlassLabParameters()
    @State private var nodeOffset: CGSize = .zero
    @GestureState private var dragOffset: CGSize = .zero
    @GestureState private var isDragging = false

    private var usesFallback: Bool {
        systemReducesTransparency || forcesFallback
    }

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()
            HStack(spacing: 0) {
                GeometryReader { geometry in
                    let size = geometry.size
                    ZStack {
                        GlassGraphBackdrop(
                            theme: theme,
                            style: gridStyle,
                            parameters: parameters
                        )
                        GlassSamplingContent(theme: theme)
                        if sceneMode == .composition {
                            glassSpecimens(in: size)
                            specimenLabels(in: size)
                        } else {
                            controlVerificationSpecimens(in: size)
                        }
                    }
                    .clipped()
                }
                if showsControls {
                    Divider()
                    GlassLabControls(
                        parameters: $parameters,
                        theme: theme
                    )
                    .frame(width: 286)
                }
            }
        }
        .background(theme.color(.graphBackground))
        .foregroundStyle(theme.color(.textPrimary))
    }

    private var header: some View {
        HStack(spacing: 14) {
            VStack(alignment: .leading, spacing: 2) {
                Text(sceneMode == .composition
                    ? "Liquid Glass optical test"
                    : "Control verification")
                    .font(.headline)
                Text(sceneMode == .composition
                    ? "Drag the center node across the grid, noodles, and spectral targets."
                    : "Compare native, composed, and fallback rendering over one backdrop.")
                    .font(.caption)
                    .foregroundStyle(theme.color(.textSecondary))
            }
            Spacer()
            Text(usesFallback ? "Fallback active" : "Native custom glass")
                .font(.caption2.weight(.semibold))
                .foregroundStyle(
                    usesFallback
                        ? theme.color(.statusTextWarning)
                        : theme.color(.statusTextSuccess)
                )
                .padding(.horizontal, 8)
                .padding(.vertical, 4)
                .background(theme.color(.surfaceControl), in: Capsule())
            Picker("Scene", selection: $sceneMode) {
                Text("Composition").tag(GlassLabSceneMode.composition)
                Text("Verify Controls").tag(GlassLabSceneMode.verification)
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            .frame(width: 190)
            Picker("Grid", selection: $gridStyle) {
                Text("Squares").tag(GlassLabGridStyle.squares)
                Text("Dots").tag(GlassLabGridStyle.dots)
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            .frame(width: 130)
            Picker("Glass", selection: $glassVariant) {
                Text("Clear").tag(GlassLabVariant.clear)
                Text("Regular").tag(GlassLabVariant.regular)
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            .frame(width: 140)
            Toggle(isOn: $showsControls) {
                Label("Controls", systemImage: "slider.horizontal.3")
            }
            .toggleStyle(.button)
            .labelStyle(.iconOnly)
            .help("Show or hide live glass-authoring controls")
            if sceneMode == .composition {
                Toggle("Opaque fallback", isOn: $forcesFallback)
                    .toggleStyle(.switch)
                    .controlSize(.small)
                    .help("Preview the Reduce Transparency and performance fallback")
            }
        }
        .padding(.horizontal, 16)
        .frame(height: 54)
        .background(theme.color(.surfaceElevated))
    }

    @ViewBuilder
    private func glassSpecimens(in size: CGSize) -> some View {
        let accent = theme.color(.nodeNative)
        GlassEffectContainer(spacing: 6) {
            ZStack {
                OpticalNodeSpecimen(
                    title: "Idle",
                    subtitle: "photara.disk.folder",
                    icon: "folder",
                    state: .idle,
                    accent: accent,
                    theme: theme,
                    usesFallback: usesFallback,
                    glassVariant: glassVariant,
                    parameters: parameters,
                    interactive: false
                )
                .frame(width: 226, height: 112)
                .position(x: 155, y: 118)

                OpticalNodeSpecimen(
                    title: "Selected",
                    subtitle: "photara.layout.compose",
                    icon: "rectangle.3.group",
                    state: .selected,
                    accent: theme.color(.nodeCreative),
                    theme: theme,
                    usesFallback: usesFallback,
                    glassVariant: glassVariant,
                    parameters: parameters,
                    interactive: false
                )
                .frame(width: 226, height: 112)
                .position(x: max(155, size.width - 155), y: 118)

                OpticalNodeSpecimen(
                    title: "Pressed",
                    subtitle: "temporary gesture state",
                    icon: "hand.draw",
                    state: .pressed,
                    accent: theme.color(.nodeAutomation),
                    theme: theme,
                    usesFallback: usesFallback,
                    glassVariant: glassVariant,
                    parameters: parameters,
                    interactive: false
                )
                .frame(width: 226, height: 112)
                .position(x: max(155, size.width - 155), y: max(150, size.height - 112))

                OpticalNodeSpecimen(
                    title: "Draggable Layout",
                    subtitle: isDragging
                        ? "pressed · clearer · elevated"
                        : (glassVariant == .clear ? "clear optical glass" : "regular optical glass"),
                    icon: "rectangle.3.group",
                    state: isDragging ? .pressed : .idle,
                    accent: accent,
                    theme: theme,
                    usesFallback: usesFallback,
                    glassVariant: glassVariant,
                    parameters: parameters,
                    interactive: parameters.interactive
                )
                .frame(width: 380, height: 184)
                .position(x: size.width * 0.50, y: size.height * 0.53)
                .offset(
                    x: nodeOffset.width + dragOffset.width,
                    y: nodeOffset.height + dragOffset.height
                )
                .highPriorityGesture(
                    DragGesture(minimumDistance: 0)
                        .updating($dragOffset) { value, state, _ in
                            state = value.translation
                        }
                        .updating($isDragging) { _, state, _ in
                            state = true
                        }
                        .onEnded { value in
                            nodeOffset.width += value.translation.width
                            nodeOffset.height += value.translation.height
                        }
                )
                NativeGlassReference(
                    theme: theme,
                    usesFallback: usesFallback,
                    glassVariant: glassVariant,
                    parameters: parameters
                )
                .position(x: 150, y: max(150, size.height - 92))
            }
            // `position` is expressed in scene coordinates. Without an explicit
            // scene-sized layout container SwiftUI centers this intrinsically
            // sized ZStack, so the glass specimens no longer line up with the
            // sampling targets and noodles drawn by the backdrop sibling.
            .frame(width: size.width, height: size.height)
        }
    }

    @ViewBuilder
    private func specimenLabels(in size: CGSize) -> some View {
        Group {
            specimenLabel("IDLE", at: CGPoint(x: 155, y: 194))
            specimenLabel("SELECTED", at: CGPoint(x: max(155, size.width - 155), y: 194))
            specimenLabel(
                "PRESSED / DRAGGING",
                at: CGPoint(x: max(155, size.width - 155), y: max(222, size.height - 36))
            )
        }
        .allowsHitTesting(false)
    }

    private func specimenLabel(_ label: String, at point: CGPoint) -> some View {
        Text(label)
            .font(.system(size: 9, weight: .bold, design: .monospaced))
            .tracking(1.2)
            .foregroundStyle(theme.color(.textSecondary))
            .position(point)
    }

    private func controlVerificationSpecimens(in size: CGSize) -> some View {
        GlassEffectContainer(spacing: 6) {
            HStack(alignment: .top, spacing: 22) {
                verificationColumn(
                    title: "1 · APPLE BASELINE",
                    detail: "Public glass API only",
                    content: NativeGlassVerificationPlate(
                        theme: theme,
                        glassVariant: glassVariant,
                        parameters: parameters
                    )
                )
                verificationColumn(
                    title: "2 · PHOTARA COMPOSITION",
                    detail: "Native glass + Photara effects",
                    content: OpticalNodeSpecimen(
                        title: "Photara Native",
                        subtitle: "live controls",
                        icon: "rectangle.3.group",
                        state: .idle,
                        accent: theme.color(.nodeNative),
                        theme: theme,
                        usesFallback: false,
                        glassVariant: glassVariant,
                        parameters: parameters,
                        interactive: parameters.interactive
                    )
                    .frame(width: 260, height: 132)
                )
                verificationColumn(
                    title: "3 · OPAQUE FALLBACK",
                    detail: "graph.node changes this sample only",
                    content: OpticalNodeSpecimen(
                        title: "Fallback",
                        subtitle: "Reduce Transparency",
                        icon: "rectangle.3.group",
                        state: .idle,
                        accent: theme.color(.nodeNative),
                        theme: theme,
                        usesFallback: true,
                        glassVariant: glassVariant,
                        parameters: parameters,
                        interactive: false
                    )
                    .frame(width: 260, height: 132)
                )
            }
            .frame(maxWidth: .infinity)
            .padding(.horizontal, 24)
            .position(x: size.width / 2, y: min(size.height / 2, 270))
        }
        .frame(width: size.width, height: size.height)
    }

    private func verificationColumn<Content: View>(
        title: String,
        detail: String,
        content: Content
    ) -> some View {
        VStack(spacing: 12) {
            VStack(spacing: 3) {
                Text(title)
                    .font(.system(size: 10, weight: .bold, design: .monospaced))
                    .tracking(0.8)
                Text(detail)
                    .font(.caption2)
                    .foregroundStyle(theme.color(.textSecondary))
            }
            content
        }
        .frame(width: 280)
    }
}

private enum GlassLabSceneMode {
    case composition
    case verification
}

private enum GlassLabGridStyle {
    case squares
    case dots
}

private enum GlassLabVariant {
    case clear
    case regular
}

private struct GlassLabParameters {
    var tintOpacity = 0.0
    var interactive = true
    var cornerRadius = 24.0
    var edgeIntensity = 1.0
    var labelDimming = 0.24
    var shadowOpacity = 0.20
    var shadowBlur = 14.0
    var shadowOffset = 7.0
    var dragLift = 6.0
    var gridSpacing = 20.0
    var minorGridOpacity = 0.16
    var majorGridOpacity = 0.42
    var portDiameter = 18.0
}

private struct GlassLabControls: View {
    @Binding var parameters: GlassLabParameters
    let theme: PhotaraResolvedTheme

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                controlSection("Native SwiftUI glass") {
                    parameterSlider("Tint", target: "Samples 1–2", value: $parameters.tintOpacity, range: 0...0.18)
                    parameterSlider("Corner radius", target: "Samples 1–3", value: $parameters.cornerRadius, range: 8...48)
                    Toggle("Interactive response", isOn: $parameters.interactive)
                        .toggleStyle(.switch)
                        .controlSize(.small)
                        .help("Apple interaction response on the draggable native node")
                }

                controlSection("Photara composition") {
                    parameterSlider("Edge response", target: "Samples 2–3", value: $parameters.edgeIntensity, range: 0...1.5)
                    parameterSlider("Label dimming", target: "Samples 2–3", value: $parameters.labelDimming, range: 0...0.45)
                    parameterSlider("Shadow opacity", target: "Samples 2–3", value: $parameters.shadowOpacity, range: 0...0.45)
                    parameterSlider("Shadow blur", target: "Samples 2–3", value: $parameters.shadowBlur, range: 0...30)
                    parameterSlider("Shadow offset", target: "Samples 2–3", value: $parameters.shadowOffset, range: 0...20)
                    parameterSlider("Drag lift", target: "Dragging only", value: $parameters.dragLift, range: 0...16)
                    parameterSlider("Grid spacing", target: "Backdrop", value: $parameters.gridSpacing, range: 12...48)
                    parameterSlider("Minor grid", target: "Backdrop", value: $parameters.minorGridOpacity, range: 0...0.35)
                    parameterSlider("Major grid", target: "Backdrop", value: $parameters.majorGridOpacity, range: 0...0.70)
                    parameterSlider("Port diameter", target: "Samples 2–3", value: $parameters.portDiameter, range: 12...28)
                }

                Text("Apple controls refraction, blur, specular response, dispersion, and reflectivity. The controls above expose the public glass API plus Photara’s surrounding composition—not private optical physics.")
                    .font(.caption)
                    .foregroundStyle(theme.color(.textSecondary))
                    .fixedSize(horizontal: false, vertical: true)

                Button("Reset") {
                    parameters = GlassLabParameters()
                }
            }
            .padding(14)
        }
        .background(theme.color(.surfacePanel))
    }

    private func controlSection<Content: View>(
        _ title: String,
        @ViewBuilder content: () -> Content
    ) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(title)
                .font(.headline)
            content()
        }
    }

    private func parameterSlider(
        _ title: String,
        target: String,
        value: Binding<Double>,
        range: ClosedRange<Double>
    ) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(title)
                Text(target)
                    .font(.system(size: 8, weight: .semibold))
                    .foregroundStyle(theme.color(.textSecondary))
                    .padding(.horizontal, 4)
                    .padding(.vertical, 2)
                    .background(theme.color(.surfaceControl), in: Capsule())
                Spacer()
                Text(value.wrappedValue.formatted(.number.precision(.fractionLength(2))))
                    .font(.system(.caption, design: .monospaced))
                    .foregroundStyle(theme.color(.textSecondary))
            }
            HStack(spacing: 6) {
                Button("Min") { value.wrappedValue = range.lowerBound }
                    .buttonStyle(.borderless)
                    .font(.caption2)
                Slider(value: value, in: range)
                Button("Max") { value.wrappedValue = range.upperBound }
                    .buttonStyle(.borderless)
                    .font(.caption2)
            }
        }
        .font(.caption)
    }
}

private struct GlassGraphBackdrop: View {
    let theme: PhotaraResolvedTheme
    let style: GlassLabGridStyle
    let parameters: GlassLabParameters

    var body: some View {
        Canvas { context, size in
            let minor = CGFloat(parameters.gridSpacing)
            let majorEvery = 4
            switch style {
            case .squares:
                for index in 0...Int(size.width / minor) + 1 {
                    let x = CGFloat(index) * minor
                    var path = Path()
                    path.move(to: CGPoint(x: x, y: 0))
                    path.addLine(to: CGPoint(x: x, y: size.height))
                    context.stroke(
                        path,
                        with: .color(theme.color(.graphGrid).opacity(
                            index % majorEvery == 0
                                ? parameters.majorGridOpacity
                                : parameters.minorGridOpacity
                        )),
                        lineWidth: index % majorEvery == 0 ? 1 : 0.5
                    )
                }
                for index in 0...Int(size.height / minor) + 1 {
                    let y = CGFloat(index) * minor
                    var path = Path()
                    path.move(to: CGPoint(x: 0, y: y))
                    path.addLine(to: CGPoint(x: size.width, y: y))
                    context.stroke(
                        path,
                        with: .color(theme.color(.graphGrid).opacity(
                            index % majorEvery == 0
                                ? parameters.majorGridOpacity
                                : parameters.minorGridOpacity
                        )),
                        lineWidth: index % majorEvery == 0 ? 1 : 0.5
                    )
                }
            case .dots:
                for xIndex in 1...Int(size.width / minor) {
                    for yIndex in 1...Int(size.height / minor) {
                        let isMajor = xIndex % majorEvery == 0 && yIndex % majorEvery == 0
                        let diameter: CGFloat = isMajor ? 3 : 1.5
                        let dot = CGRect(
                            x: CGFloat(xIndex) * minor - diameter / 2,
                            y: CGFloat(yIndex) * minor - diameter / 2,
                            width: diameter,
                            height: diameter
                        )
                        context.fill(
                            Path(ellipseIn: dot),
                            with: .color(theme.color(.graphGrid).opacity(
                                isMajor
                                    ? min(1, parameters.majorGridOpacity * 1.6)
                                    : min(1, parameters.minorGridOpacity * 2.4)
                            ))
                        )
                    }
                }
            }
        }
        .background(theme.color(.graphBackground))
    }
}

private struct GlassSamplingContent: View {
    let theme: PhotaraResolvedTheme

    var body: some View {
        GeometryReader { geometry in
            let size = geometry.size
            ZStack {
                RoundedRectangle(cornerRadius: 30)
                    .fill(
                        LinearGradient(
                            colors: [
                                theme.color(.nodeIO),
                                theme.color(.nodeCreative),
                                theme.color(.statusTextError),
                                theme.color(.statusTextWarning),
                            ],
                            startPoint: .leading,
                            endPoint: .trailing
                        )
                    )
                    .frame(width: min(520, size.width * 0.62), height: 28)
                    .position(x: size.width * 0.50, y: size.height * 0.53)

                Circle()
                    .fill(theme.color(.nodeIO))
                    .frame(width: 78, height: 78)
                    .position(x: size.width * 0.37, y: size.height * 0.53)

                Circle()
                    .fill(theme.color(.statusTextWarning))
                    .frame(width: 78, height: 78)
                    .position(x: size.width * 0.63, y: size.height * 0.53)

                RoundedRectangle(cornerRadius: 24, style: .continuous)
                    .fill(
                        LinearGradient(
                            colors: [
                                theme.color(.nodeIO),
                                theme.color(.nodeCreative),
                                theme.color(.statusTextWarning),
                            ],
                            startPoint: .topLeading,
                            endPoint: .bottomTrailing
                        )
                    )
                    .frame(width: 220, height: 64)
                    .position(x: 150, y: max(150, size.height - 92))

                Canvas { context, _ in
                    var primary = Path()
                    primary.move(to: CGPoint(x: -20, y: size.height * 0.68))
                    primary.addCurve(
                        to: CGPoint(x: size.width + 20, y: size.height * 0.36),
                        control1: CGPoint(x: size.width * 0.24, y: size.height * 0.30),
                        control2: CGPoint(x: size.width * 0.72, y: size.height * 0.78)
                    )
                    context.stroke(
                        primary,
                        with: .linearGradient(
                            Gradient(colors: [
                                theme.color(.nodeIO),
                                theme.color(.nodeCreative),
                                theme.color(.statusTextError),
                            ]),
                            startPoint: CGPoint(x: 0, y: size.height * 0.5),
                            endPoint: CGPoint(x: size.width, y: size.height * 0.5)
                        ),
                        style: StrokeStyle(lineWidth: 7, lineCap: .round)
                    )

                    var secondary = Path()
                    secondary.move(to: CGPoint(x: size.width * 0.18, y: -20))
                    secondary.addCurve(
                        to: CGPoint(x: size.width * 0.78, y: size.height + 20),
                        control1: CGPoint(x: size.width * 0.72, y: size.height * 0.12),
                        control2: CGPoint(x: size.width * 0.24, y: size.height * 0.82)
                    )
                    context.stroke(
                        secondary,
                        with: .color(theme.color(.textPrimary).opacity(0.34)),
                        style: StrokeStyle(lineWidth: 3, lineCap: .round, dash: [12, 8])
                    )
                }
            }
            .frame(width: size.width, height: size.height)
        }
        .allowsHitTesting(false)
    }
}

private enum OpticalNodeState {
    case idle
    case selected
    case pressed
}

/// Same geometry as the verification nodes, with only Apple's public glass
/// modifier. This is the control specimen for distinguishing framework behavior
/// from Photara's edge, shadow, label, and port composition.
private struct NativeGlassVerificationPlate: View {
    let theme: PhotaraResolvedTheme
    let glassVariant: GlassLabVariant
    let parameters: GlassLabParameters

    var body: some View {
        let shape = RoundedRectangle(
            cornerRadius: CGFloat(parameters.cornerRadius),
            style: .continuous
        )
        let base: Glass = glassVariant == .clear ? .clear : .regular

        VStack(spacing: 5) {
            Label("Native Glass", systemImage: "sparkles")
                .font(.headline)
            Text("No Photara overlay")
                .font(.caption.monospaced())
                .foregroundStyle(theme.color(.textSecondary))
        }
        .frame(width: 242, height: 122)
        .glassEffect(
            base
                .tint(parameters.tintOpacity > 0
                    ? theme.color(.nodeNative).opacity(parameters.tintOpacity)
                    : nil)
                .interactive(parameters.interactive),
            in: shape
        )
    }
}

/// Intentionally mirrors Apple's smallest custom SwiftUI recipe so the Lab can
/// distinguish framework behavior from Photara's node composition.
private struct NativeGlassReference: View {
    let theme: PhotaraResolvedTheme
    let usesFallback: Bool
    let glassVariant: GlassLabVariant
    let parameters: GlassLabParameters

    var body: some View {
        let label = Label("Apple recipe", systemImage: "sparkles")
            .font(.headline)
            .padding(.horizontal, 18)
            .padding(.vertical, 12)

        if usesFallback {
            label.background(theme.color(.graphNode), in: Capsule())
        } else {
            let base: Glass = glassVariant == .clear ? .clear : .regular
            label.glassEffect(
                base
                    .tint(parameters.tintOpacity > 0
                        ? theme.color(.nodeNative).opacity(parameters.tintOpacity)
                        : nil)
                    .interactive(parameters.interactive)
            )
        }
    }
}

private struct OpticalNodeSpecimen: View {
    let title: String
    let subtitle: String
    let icon: String
    let state: OpticalNodeState
    let accent: Color
    let theme: PhotaraResolvedTheme
    let usesFallback: Bool
    let glassVariant: GlassLabVariant
    let parameters: GlassLabParameters
    let interactive: Bool

    private var cornerRadius: CGFloat {
        CGFloat(parameters.cornerRadius)
    }

    private var shadowStrength: Double {
        switch state {
        case .idle: 0.72
        case .selected: 0.86
        case .pressed: 1.0
        }
    }

    var body: some View {
        GeometryReader { geometry in
            let bodySize = CGSize(
                width: geometry.size.width - 18,
                height: geometry.size.height - 10
            )
            ZStack {
                opticalBody(size: bodySize)
                    .shadow(
                        color: Color.black.opacity(parameters.shadowOpacity * shadowStrength),
                        radius: CGFloat(parameters.shadowBlur),
                        y: CGFloat(parameters.shadowOffset)
                    )
                OpticalPortBead(
                    core: theme.color(.nodeIO),
                    theme: theme,
                    usesFallback: usesFallback,
                    glassVariant: glassVariant,
                    parameters: parameters,
                    interactive: interactive
                )
                .position(x: 9, y: geometry.size.height * 0.66)
                OpticalPortBead(
                    core: theme.color(.nodeCreative),
                    theme: theme,
                    usesFallback: usesFallback,
                    glassVariant: glassVariant,
                    parameters: parameters,
                    interactive: interactive
                )
                .position(x: geometry.size.width - 9, y: geometry.size.height * 0.66)
            }
            .offset(y: state == .pressed ? -CGFloat(parameters.dragLift) : 0)
            .animation(.snappy(duration: 0.22), value: state == .pressed)
        }
        .contentShape(Rectangle())
    }

    @ViewBuilder
    private func opticalBody(size: CGSize) -> some View {
        let shape = RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
        nodeContent(size: size)
            .modifier(
                OpticalGlassSurface(
                    shape: shape,
                    state: state,
                    accent: accent,
                    theme: theme,
                    usesFallback: usesFallback,
                    glassVariant: glassVariant,
                    parameters: parameters,
                    interactive: interactive
                )
            )
    }

    private func nodeContent(size: CGSize) -> some View {
        VStack(spacing: 0) {
            HStack(spacing: 10) {
                HStack(spacing: 8) {
                    Image(systemName: icon)
                        .font(.system(size: 18, weight: .medium))
                        .foregroundStyle(accent)
                        .frame(width: 30, height: 30)
                    VStack(alignment: .leading, spacing: 1) {
                        Text(title)
                            .font(.system(size: 14, weight: .semibold))
                        Text(subtitle)
                            .font(.system(size: 9.5, design: .monospaced))
                            .foregroundStyle(theme.color(.textSecondary))
                    }
                }
                .padding(.horizontal, 7)
                .padding(.vertical, 4)
                .background(
                    theme.color(.graphBackground).opacity(parameters.labelDimming),
                    in: RoundedRectangle(cornerRadius: 10, style: .continuous)
                )
                Spacer(minLength: 4)
                Circle()
                    .fill(theme.color(.statusTextSuccess))
                    .frame(width: 7, height: 7)
            }
            Spacer(minLength: 4)
            HStack {
                Text("Assets")
                Spacer()
                Text("Layout")
            }
            .font(.system(size: 10, weight: .medium))
            .foregroundStyle(theme.color(.textSecondary))
        }
        .padding(.horizontal, 22)
        .padding(.vertical, 15)
        .frame(width: size.width, height: size.height)
        .foregroundStyle(theme.color(.textPrimary))
        .shadow(color: theme.color(.graphBackground).opacity(0.48), radius: 1, y: 1)
    }
}

private struct OpticalGlassSurface<S: InsettableShape>: ViewModifier {
    let shape: S
    let state: OpticalNodeState
    let accent: Color
    let theme: PhotaraResolvedTheme
    let usesFallback: Bool
    let glassVariant: GlassLabVariant
    let parameters: GlassLabParameters
    let interactive: Bool

    func body(content: Content) -> some View {
        if usesFallback {
            content
                .background(theme.color(.graphNode).opacity(0.96), in: shape)
                .overlay { perimeter }
        } else {
            let base: Glass = glassVariant == .clear ? .clear : .regular
            let stateTintScale = state == .pressed ? 0.35 : (state == .selected ? 1 : 0.65)
            content
                .glassEffect(
                    base
                        .tint(parameters.tintOpacity > 0
                            ? accent.opacity(parameters.tintOpacity * stateTintScale)
                            : nil)
                        .interactive(interactive),
                    in: shape
                )
                .overlay { perimeter }
        }
    }

    private var perimeter: some View {
        shape
            .strokeBorder(
                AngularGradient(
                    colors: [
                        .white.opacity(min(1, (state == .pressed ? 0.88 : 0.56) * parameters.edgeIntensity)),
                        accent.opacity(min(1, (state == .selected ? 0.82 : 0.24) * parameters.edgeIntensity)),
                        .white.opacity(min(1, 0.08 * parameters.edgeIntensity)),
                        theme.color(.nodeCreative).opacity(min(1, (state == .selected ? 0.52 : 0.12) * parameters.edgeIntensity)),
                        .white.opacity(min(1, (state == .pressed ? 0.72 : 0.34) * parameters.edgeIntensity)),
                    ],
                    center: .center
                ),
                lineWidth: state == .selected ? 1.7 : 1
            )
            .allowsHitTesting(false)
    }
}

private struct OpticalPortBead: View {
    let core: Color
    let theme: PhotaraResolvedTheme
    let usesFallback: Bool
    let glassVariant: GlassLabVariant
    let parameters: GlassLabParameters
    let interactive: Bool

    var body: some View {
        ZStack {
            Circle()
                .stroke(.white.opacity(0.66), lineWidth: 0.8)
            Circle()
                .fill(core)
                .frame(
                    width: max(4, CGFloat(parameters.portDiameter) * 0.28),
                    height: max(4, CGFloat(parameters.portDiameter) * 0.28)
                )
                .shadow(color: core.opacity(0.72), radius: 2)
        }
        .frame(
            width: CGFloat(parameters.portDiameter),
            height: CGFloat(parameters.portDiameter)
        )
        .modifier(
            OpticalPortSurface(
                theme: theme,
                usesFallback: usesFallback,
                glassVariant: glassVariant,
                parameters: parameters,
                interactive: interactive
            )
        )
    }
}

private struct OpticalPortSurface: ViewModifier {
    let theme: PhotaraResolvedTheme
    let usesFallback: Bool
    let glassVariant: GlassLabVariant
    let parameters: GlassLabParameters
    let interactive: Bool

    func body(content: Content) -> some View {
        if usesFallback {
            content.background(theme.color(.graphNode), in: Circle())
        } else {
            let base: Glass = glassVariant == .clear ? .clear : .regular
            content.glassEffect(
                base
                    .tint(parameters.tintOpacity > 0
                        ? theme.color(.nodeNative).opacity(parameters.tintOpacity * 0.7)
                        : nil)
                    .interactive(interactive),
                in: Circle()
            )
        }
    }
}
