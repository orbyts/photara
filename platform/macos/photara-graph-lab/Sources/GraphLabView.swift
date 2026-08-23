import SwiftUI

private enum GraphLabNodeID: Hashable {
    case source
    case transform
    case composite
}

struct GraphLabView: View {
    @Environment(\.photaraTheme) private var theme
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency
    @Binding var appearance: PhotaraThemeAppearance

    @State private var pattern = PhotaraGraphPattern.lines
    @State private var gridSpacing = 24.0
    @State private var minorOpacity = 0.24
    @State private var minorLineWidth = 0.5
    @State private var minorMarkSize = 1.4
    @State private var majorInterval = 5
    @State private var majorOpacity = 0.58
    @State private var majorLineWidth = 1.0
    @State private var majorMarkSize = 3.0
    @State private var minorColorOverride: Color?
    @State private var majorColorOverride: Color?
    @State private var noodleColorOverride: Color?
    @State private var idleGlassTreatment = PhotaraGraphGlassTreatment.regular
    @State private var selectedGlassTreatment = PhotaraGraphGlassTreatment.clear
    @State private var idleGlassTintOpacity = 0.05
    @State private var selectedGlassTintOpacity = 0.025
    @State private var cornerRadius = 12.0
    @State private var portShape = PhotaraGraphPortShape.round
    @State private var portOffset = 0.0
    @State private var portGlassTreatment = PhotaraGraphGlassTreatment.clear
    @State private var portGlassTintOpacity = 0.18
    @State private var shadowBlur = 5.0
    @State private var shadowOpacity = 0.12
    @State private var shadowOffsetY = 3.0
    @State private var pan = CGSize.zero
    @State private var zoom = 1.0
    @State private var selectedNode = GraphLabNodeID.transform
    @State private var nodeOffsets: [GraphLabNodeID: CGSize] = [:]
    @State private var activeNode: GraphLabNodeID?
    @GestureState private var canvasDrag = CGSize.zero
    @GestureState private var nodeDrag = CGSize.zero

    var body: some View {
        HSplitView {
            canvas
                .frame(minWidth: 700)
            controls
                .frame(minWidth: 285, idealWidth: 310, maxWidth: 340)
        }
        .toolbar {
            Button("Center Scene", systemImage: "scope") { centerScene() }
            Text("Zoom \(Int(zoom * 100))%")
                .font(.caption.monospacedDigit())
            Slider(value: $zoom, in: 0.55...1.8)
                .frame(width: 120)
            Divider()
            Picker("Appearance", selection: $appearance) {
                Text("Light").tag(PhotaraThemeAppearance.light)
                Text("Dark").tag(PhotaraThemeAppearance.dark)
            }
            .pickerStyle(.segmented)
            .frame(width: 150)
        }
    }

    private var canvas: some View {
        GeometryReader { geometry in
            let displayedPan = CGSize(
                width: pan.width + canvasDrag.width,
                height: pan.height + canvasDrag.height
            )
            ZStack {
                PhotaraGraphBackground(
                    pan: displayedPan,
                    zoom: zoom,
                    style: PhotaraGraphBackgroundStyle(
                        pattern: pattern,
                        spacing: gridSpacing,
                        opacity: minorOpacity,
                        markSize: minorMarkSize,
                        lineWidth: minorLineWidth,
                        majorInterval: majorInterval,
                        majorOpacity: majorOpacity,
                        majorMarkSize: majorMarkSize,
                        majorLineWidth: majorLineWidth
                    ),
                    minorColor: minorColorOverride,
                    majorColor: majorColorOverride
                )
                GraphLabNoodleLayer(
                    color: noodleColorOverride ?? theme?.color(.borderFocus) ?? .accentColor,
                    pan: displayedPan,
                    zoom: zoom
                )

                Color.clear
                    .contentShape(Rectangle())
                    .gesture(canvasPanGesture)

                specimen(
                    id: .source,
                    title: "Source",
                    subtitle: "One port",
                    inputs: [],
                    outputs: ["Assets"],
                    worldPosition: CGPoint(x: -190, y: -120),
                    canvasSize: geometry.size,
                    pan: displayedPan
                )
                specimen(
                    id: .transform,
                    title: "Transform",
                    subtitle: "Three port rows",
                    inputs: ["Input", "Mask", "Control"],
                    outputs: ["Result", "Preview", "Metadata"],
                    worldPosition: CGPoint(x: 150, y: -90),
                    canvasSize: geometry.size,
                    pan: displayedPan
                )
                specimen(
                    id: .composite,
                    title: "Composite",
                    subtitle: "Six port rows",
                    inputs: ["Layer 1", "Layer 2", "Mask", "Depth", "Color", "Control"],
                    outputs: ["Image", "Preview"],
                    worldPosition: CGPoint(x: 0, y: 150),
                    canvasSize: geometry.size,
                    pan: displayedPan
                )
            }
            .clipped()
            .overlay(alignment: .bottomLeading) {
                Text("Click or drag a node · drag empty canvas to pan")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .padding(10)
                    .background(.thinMaterial, in: RoundedRectangle(cornerRadius: 8))
                    .padding(14)
            }
        }
    }

    private var controls: some View {
        Form {
            Section("Background pattern") {
                Picker("Pattern", selection: $pattern) {
                    ForEach(PhotaraGraphPattern.allCases) { value in
                        Text(value.title).tag(value)
                    }
                }
                if pattern != .none {
                    valueSlider("Minor spacing", value: $gridSpacing, range: 12...48, suffix: " pt")
                    Stepper("Major every \(majorInterval) cells", value: $majorInterval, in: 2...12)
                }
            }

            if pattern != .none {
                Section("Minor") {
                    ColorPicker(
                        "Color",
                        selection: colorBinding(
                            $minorColorOverride,
                            default: theme?.color(.graphGrid) ?? .secondary
                        )
                    )
                    valueSlider("Opacity", value: $minorOpacity, range: 0...0.9)
                    if pattern == .lines {
                        valueSlider("Line width", value: $minorLineWidth, range: 0.25...2.5, suffix: " pt")
                    } else {
                        valueSlider("Mark size", value: $minorMarkSize, range: 0.6...5, suffix: " pt")
                    }
                }

                Section("Major") {
                    ColorPicker(
                        "Color",
                        selection: colorBinding(
                            $majorColorOverride,
                            default: theme?.color(.borderStrong) ?? .primary
                        )
                    )
                    valueSlider("Opacity", value: $majorOpacity, range: 0...1)
                    if pattern == .lines {
                        valueSlider("Line width", value: $majorLineWidth, range: 0.5...4, suffix: " pt")
                    } else {
                        valueSlider("Mark size", value: $majorMarkSize, range: 1...8, suffix: " pt")
                    }
                }
            }

            Section("Node") {
                valueSlider("Corner radius", value: $cornerRadius, range: 0...32, suffix: " pt")
            }

            Section("Native glass") {
                Picker("Unselected", selection: $idleGlassTreatment) {
                    ForEach(PhotaraGraphGlassTreatment.allCases) { treatment in
                        Text(treatment.title).tag(treatment)
                    }
                }
                .pickerStyle(.segmented)
                valueSlider("Unselected tint", value: $idleGlassTintOpacity, range: 0...0.18)

                Picker("Selected", selection: $selectedGlassTreatment) {
                    ForEach(PhotaraGraphGlassTreatment.allCases) { treatment in
                        Text(treatment.title).tag(treatment)
                    }
                }
                .pickerStyle(.segmented)
                valueSlider("Selected tint", value: $selectedGlassTintOpacity, range: 0...0.18)
                Text("Regular and Clear are Apple's public optical treatments. Tint changes native glass color, not blur strength.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                if reduceTransparency {
                    Text("Reduce Transparency is active: semantic opaque surfaces replace glass.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }

            Section("Ports") {
                Picker("Shape", selection: $portShape) {
                    ForEach(PhotaraGraphPortShape.allCases) { value in
                        Text(value.title).tag(value)
                    }
                }
                .pickerStyle(.segmented)
                valueSlider("Position offset", value: $portOffset, range: -8...16, suffix: " pt")
                Picker("Glass bead", selection: $portGlassTreatment) {
                    ForEach(PhotaraGraphGlassTreatment.allCases) { treatment in
                        Text(treatment.title).tag(treatment)
                    }
                }
                .pickerStyle(.segmented)
                valueSlider("Glass tint", value: $portGlassTintOpacity, range: 0...0.5)
            }

            Section("Shadow") {
                valueSlider("Blur", value: $shadowBlur, range: 0...30, suffix: " pt")
                valueSlider("Opacity", value: $shadowOpacity, range: 0...0.5)
                valueSlider("Vertical offset", value: $shadowOffsetY, range: -4...18, suffix: " pt")
            }

            Section("Backdrop specimen") {
                ColorPicker(
                    "Noodle color",
                    selection: colorBinding(
                        $noodleColorOverride,
                        default: theme?.color(.borderFocus) ?? .accentColor
                    )
                )
                Text("The bright noodle remains fixed in Graph space beneath Transform.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Section {
                Button("Reset All") { reset() }
            }
        }
        .formStyle(.grouped)
    }

    private var nodeStyle: PhotaraGraphNodeStyle {
        PhotaraGraphNodeStyle(
            cornerRadius: cornerRadius,
            portShape: portShape,
            portOffset: portOffset,
            shadowBlur: shadowBlur,
            shadowOpacity: shadowOpacity,
            shadowOffsetY: shadowOffsetY
        )
    }

    private func specimen(
        id: GraphLabNodeID,
        title: String,
        subtitle: String,
        inputs: [String],
        outputs: [String],
        worldPosition: CGPoint,
        canvasSize: CGSize,
        pan: CGSize
    ) -> some View {
        let storedOffset = nodeOffsets[id] ?? .zero
        let liveDrag = activeNode == id ? nodeDrag : .zero
        let position = screenPosition(
            CGPoint(
                x: worldPosition.x + storedOffset.width + liveDrag.width / zoom,
                y: worldPosition.y + storedOffset.height + liveDrag.height / zoom
            ),
            canvasSize: canvasSize,
            pan: pan
        )
        return GraphLabNodeSpecimen(
            title: title,
            subtitle: subtitle,
            inputs: inputs,
            outputs: outputs,
            style: nodeStyle,
            isSelected: selectedNode == id,
            glassTreatment: selectedNode == id ? selectedGlassTreatment : idleGlassTreatment,
            glassTintOpacity: selectedNode == id ? selectedGlassTintOpacity : idleGlassTintOpacity,
            portGlassTreatment: portGlassTreatment,
            portGlassTintOpacity: portGlassTintOpacity
        )
        .scaleEffect(zoom)
        .position(position)
        .gesture(nodeGesture(for: id))
    }

    private var canvasPanGesture: some Gesture {
        DragGesture()
            .updating($canvasDrag) { value, state, _ in state = value.translation }
            .onEnded {
                pan.width += $0.translation.width
                pan.height += $0.translation.height
            }
    }

    private func nodeGesture(for id: GraphLabNodeID) -> some Gesture {
        DragGesture(minimumDistance: 0)
            .onChanged { _ in
                selectedNode = id
                activeNode = id
            }
            .updating($nodeDrag) { value, state, _ in state = value.translation }
            .onEnded { value in
                var offset = nodeOffsets[id] ?? .zero
                offset.width += value.translation.width / zoom
                offset.height += value.translation.height / zoom
                nodeOffsets[id] = offset
                activeNode = nil
            }
    }

    private func screenPosition(_ world: CGPoint, canvasSize: CGSize, pan: CGSize) -> CGPoint {
        CGPoint(
            x: canvasSize.width / 2 + world.x * zoom + pan.width,
            y: canvasSize.height / 2 + world.y * zoom + pan.height
        )
    }

    private func valueSlider(
        _ title: String,
        value: Binding<Double>,
        range: ClosedRange<Double>,
        suffix: String = ""
    ) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(title)
                Spacer()
                Text(value.wrappedValue.formatted(.number.precision(.fractionLength(2))) + suffix)
                    .foregroundStyle(.secondary)
                    .monospacedDigit()
            }
            Slider(value: value, in: range)
        }
    }

    private func colorBinding(_ storage: Binding<Color?>, default defaultColor: Color) -> Binding<Color> {
        Binding(
            get: { storage.wrappedValue ?? defaultColor },
            set: { storage.wrappedValue = $0 }
        )
    }

    private func reset() {
        pattern = .lines
        gridSpacing = 24
        minorOpacity = 0.24
        minorLineWidth = 0.5
        minorMarkSize = 1.4
        majorInterval = 5
        majorOpacity = 0.58
        majorLineWidth = 1
        majorMarkSize = 3
        minorColorOverride = nil
        majorColorOverride = nil
        noodleColorOverride = nil
        idleGlassTreatment = .regular
        selectedGlassTreatment = .clear
        idleGlassTintOpacity = 0.05
        selectedGlassTintOpacity = 0.025
        cornerRadius = 12
        portShape = .round
        portOffset = 0
        portGlassTreatment = .clear
        portGlassTintOpacity = 0.18
        shadowBlur = 5
        shadowOpacity = 0.12
        shadowOffsetY = 3
        centerScene()
    }

    private func centerScene() {
        pan = .zero
        zoom = 1
        selectedNode = .transform
        nodeOffsets = [:]
        activeNode = nil
    }
}

private struct GraphLabNoodleLayer: View {
    let color: Color
    let pan: CGSize
    let zoom: CGFloat

    var body: some View {
        GeometryReader { geometry in
            Canvas { context, _ in
                let start = screenPoint(CGPoint(x: -360, y: -90), in: geometry.size)
                let end = screenPoint(CGPoint(x: 390, y: -90), in: geometry.size)
                var primary = Path()
                primary.move(to: start)
                primary.addCurve(
                    to: end,
                    control1: CGPoint(x: start.x + 190 * zoom, y: start.y - 54 * zoom),
                    control2: CGPoint(x: end.x - 190 * zoom, y: end.y + 54 * zoom)
                )
                context.stroke(
                    primary,
                    with: .color(color.opacity(0.92)),
                    lineWidth: max(1.5, 2.4 * zoom)
                )

                let secondaryStart = screenPoint(CGPoint(x: -300, y: 210), in: geometry.size)
                let secondaryEnd = screenPoint(CGPoint(x: 310, y: -190), in: geometry.size)
                var secondary = Path()
                secondary.move(to: secondaryStart)
                secondary.addCurve(
                    to: secondaryEnd,
                    control1: CGPoint(x: secondaryStart.x + 150 * zoom, y: secondaryStart.y - 20 * zoom),
                    control2: CGPoint(x: secondaryEnd.x - 140 * zoom, y: secondaryEnd.y + 100 * zoom)
                )
                context.stroke(
                    secondary,
                    with: .color(color.opacity(0.36)),
                    lineWidth: max(1, 1.3 * zoom)
                )
            }
        }
        .allowsHitTesting(false)
    }

    private func screenPoint(_ world: CGPoint, in size: CGSize) -> CGPoint {
        CGPoint(
            x: size.width / 2 + world.x * zoom + pan.width,
            y: size.height / 2 + world.y * zoom + pan.height
        )
    }
}

private struct GraphLabNodeSpecimen: View {
    @Environment(\.photaraTheme) private var theme
    let title: String
    let subtitle: String
    let inputs: [String]
    let outputs: [String]
    let style: PhotaraGraphNodeStyle
    let isSelected: Bool
    let glassTreatment: PhotaraGraphGlassTreatment
    let glassTintOpacity: Double
    let portGlassTreatment: PhotaraGraphGlassTreatment
    let portGlassTintOpacity: Double

    private let width = 224.0
    private var rowCount: Int { max(1, max(inputs.count, outputs.count)) }
    private var height: CGFloat { 62 + CGFloat(rowCount) * 25 + 12 }

    var body: some View {
        PhotaraGraphNodeSurface(
            style: style,
            glassTreatment: glassTreatment,
            glassTint: (theme?.color(.nodeNative) ?? .accentColor).opacity(glassTintOpacity)
        ) {
            VStack(spacing: 0) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(title)
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(theme?.color(.textPrimary) ?? .primary)
                    Text(subtitle)
                        .font(.caption)
                        .foregroundStyle(theme?.color(.textSecondary) ?? .secondary)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, 14)
                .padding(.vertical, 10)
                Divider()
                HStack(alignment: .top) {
                    labelColumn(inputs, alignment: .leading)
                    Spacer(minLength: 16)
                    labelColumn(outputs, alignment: .trailing)
                }
                .padding(.horizontal, 14)
                .padding(.vertical, 8)
            }
            .frame(width: width, height: height, alignment: .top)
            .clipShape(RoundedRectangle(cornerRadius: style.cornerRadius, style: .continuous))
        } ports: {
            GeometryReader { geometry in
                ForEach(0..<inputs.count, id: \.self) { index in
                    port(label: inputs[index], at: index)
                        .position(x: -style.portOffset, y: portY(index))
                }
                ForEach(0..<outputs.count, id: \.self) { index in
                    port(label: outputs[index], at: index)
                        .position(x: geometry.size.width + style.portOffset, y: portY(index))
                }
            }
        }
        .frame(width: width, height: height)
        .background {
            RoundedRectangle(cornerRadius: style.cornerRadius, style: .continuous)
                .stroke(
                    isSelected
                        ? (theme?.color(.graphNodeSelected) ?? .accentColor).opacity(0.38)
                        : Color.clear,
                    lineWidth: 1.5
                )
                .padding(-0.5)
        }
        .contentShape(RoundedRectangle(cornerRadius: style.cornerRadius, style: .continuous))
    }

    private func labelColumn(_ labels: [String], alignment: HorizontalAlignment) -> some View {
        VStack(alignment: alignment, spacing: 0) {
            ForEach(labels, id: \.self) { label in
                Text(label)
                    .font(.system(size: 10.5))
                    .foregroundStyle(theme?.color(.textSecondary) ?? .secondary)
                    .frame(height: 25)
            }
        }
    }

    private func port(label: String, at index: Int) -> some View {
        PhotaraGraphPort(
            shape: style.portShape,
            width: 14,
            height: 14,
            glassTreatment: portGlassTreatment,
            glassTint: (theme?.color(.borderFocus) ?? .accentColor).opacity(portGlassTintOpacity),
            coreColor: semanticPortColor(for: label)
        )
            .accessibilityLabel("\(label) port \(index + 1)")
    }

    private func semanticPortColor(for label: String) -> Color {
        switch label {
        case "Control":
            theme?.color(.nodeAutomation) ?? .purple
        case "Mask", "Depth":
            theme?.color(.nodeCompute) ?? .orange
        case "Metadata":
            theme?.color(.nodeIntegration) ?? .green
        case "Preview":
            theme?.color(.nodeCreative) ?? .pink
        default:
            theme?.color(.nodeIO) ?? .blue
        }
    }

    private func portY(_ index: Int) -> CGFloat {
        62 + 12.5 + CGFloat(index) * 25
    }
}
