import AppKit
import Foundation
import SwiftUI

private enum GraphLabNodeSurfaceStyle: String, CaseIterable, Identifiable {
    case flat
    case glass

    var id: String { rawValue }
    var title: String { rawValue.capitalized }
}

struct GraphLabView: View {
    private static let preferencesKey = "graph-lab.visual-preferences.v1"
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
    @State private var lightColors = GraphLabAppearanceColors()
    @State private var darkColors = GraphLabAppearanceColors()
    @State private var nodeSurfaceStyle = GraphLabNodeSurfaceStyle.flat
    @State private var idleGlassTreatment = PhotaraGraphGlassTreatment.regular
    @State private var selectedGlassTreatment = PhotaraGraphGlassTreatment.clear
    @State private var idleGlassTintOpacity = 0.05
    @State private var selectedGlassTintOpacity = 0.025
    @State private var lightIdleGlassOpacity = 1.0
    @State private var darkIdleGlassOpacity = 1.0
    @State private var lightSelectedGlassOpacity = 1.0
    @State private var darkSelectedGlassOpacity = 1.0
    @State private var selectedStrokeWidth = 2.0
    @State private var cornerRadius = 12.0
    @State private var portShape = PhotaraGraphPortShape.round
    @State private var portOffset = 0.0
    @State private var portGlassTreatment = PhotaraGraphGlassTreatment.clear
    @State private var portGlassTintOpacity = 0.18
    @State private var portCoreSize = 5.0
    @State private var inactivePortSaturation = 0.18
    @State private var inactivePortShowsStroke = true
    @State private var inactivePortStrokeWidth = 1.0
    @State private var activePortShowsShadow = true
    @State private var activePortShadowOpacity = 0.32
    @State private var activePortShadowBlur = 3.0
    @State private var activePortShadowOffsetY = 1.5
    @State private var lightPortCoreBrightness = 0.0
    @State private var darkPortCoreBrightness = 0.18
    @State private var lightActivePortCoreBrightness = 0.12
    @State private var darkActivePortCoreBrightness = 0.28
    @State private var noodleStyle = PhotaraGraphNoodleStyle.curved
    @State private var controller = PhotaraGraphInteractionController(
        document: GraphLabFixtures.document, selection: .node("transform")
    )
    @State private var nodeShadowBlur = 5.0
    @State private var nodeShadowOffsetY = 3.0
    @State private var lightNodeShadowOpacity = 0.12
    @State private var darkNodeShadowOpacity = 0.18
    @State private var didLoadPreferences = false
    @State private var preferencesStatus: String?

    var body: some View {
        HSplitView {
            canvas
                .frame(minWidth: 700)
            controls
                .frame(minWidth: 285, idealWidth: 310, maxWidth: 340)
        }
        .toolbar {
            Button("Center Scene", systemImage: "scope") { centerScene() }
            Button("Save Preferences", systemImage: "square.and.arrow.down") {
                savePreferences()
            }
            Picker("Appearance", selection: $appearance) {
                Text("Light").tag(PhotaraThemeAppearance.light)
                Text("Dark").tag(PhotaraThemeAppearance.dark)
            }
            .pickerStyle(.segmented)
            .frame(width: 150)
        }
        .onAppear {
            loadPreferencesIfAvailable()
            configureGeometry()
        }
        .onChange(of: portOffset) { configureGeometry() }
        .onChange(of: noodleStyle) { configureGeometry() }
        .onChange(of: appearance) { controller.cancel(resetTool: true) }
        .onChange(of: nodeSurfaceStyle) { controller.cancel() }
    }

    private func configureGeometry() {
        controller.configure(.init(portOffset: portOffset, noodleStyle: noodleStyle))
    }

    private var canvas: some View {
        GraphLabCanvas(
            controller: controller,
            backgroundStyle: PhotaraGraphBackgroundStyle(
                pattern: pattern, spacing: gridSpacing, opacity: minorOpacity,
                markSize: minorMarkSize, lineWidth: minorLineWidth,
                majorInterval: majorInterval, majorOpacity: majorOpacity,
                majorMarkSize: majorMarkSize, majorLineWidth: majorLineWidth
            ),
            backgroundColor: activeColor(\.graphBackground),
            minorColor: activeColor(\.minor),
            majorColor: activeColor(\.major),
            noodleColor: activeColor(\.noodle) ?? theme?.color(.borderFocus) ?? .accentColor,
            nodeContent: specimen
        )
    }

    private var controls: some View {
        Form {
            Section("Overview") {
                Picker("Visibility", selection: $controller.overviewPolicy) {
                    ForEach(PhotaraGraphOverviewPolicy.allCases) { policy in
                        Text(policy.title).tag(policy)
                    }
                }
                .accessibilityIdentifier("photara.graph.overview-policy")
                Picker("Position", selection: $controller.overviewPosition) {
                    ForEach(PhotaraGraphOverviewPosition.allCases) { position in
                        Text(position.title).tag(position)
                    }
                }
                valueSlider("Corner radius", value: $controller.overviewCornerRadius, range: 0...36, suffix: " pt")
                valueSlider("Size", value: Binding(get: { controller.overviewSizeFraction * 100 },
                    set: { controller.overviewSizeFraction = PhotaraGraphOverviewSizing.fraction($0 / 100) }),
                    range: 10...28, suffix: "%")
                Text("Relative to the graph window, with limits to keep the overview readable and compact.")
                    .font(.caption).foregroundStyle(.secondary)
            }

            Section("Background pattern") {
                ColorPicker(
                    appearance == .dark ? "Dark Graph color" : "Light Graph color",
                    selection: colorBinding(
                        activeColorBinding(\.graphBackground),
                        default: theme?.color(.graphBackground) ?? Color(nsColor: .controlBackgroundColor)
                    )
                )
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
                        appearance == .dark ? "Dark color" : "Light color",
                        selection: colorBinding(
                            activeColorBinding(\.minor),
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
                        appearance == .dark ? "Dark color" : "Light color",
                        selection: colorBinding(
                            activeColorBinding(\.major),
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
                Picker("Surface", selection: $nodeSurfaceStyle) {
                    ForEach(GraphLabNodeSurfaceStyle.allCases) { style in
                        Text(style.title).tag(style)
                    }
                }
                .pickerStyle(.segmented)
                if nodeSurfaceStyle == .flat {
                    ColorPicker(
                        appearance == .dark ? "Dark node color" : "Light node color",
                        selection: colorBinding(
                            activeColorBinding(\.idleNodeFill),
                            default: defaultFlatNodeColor
                        )
                    )
                    ColorPicker(
                        appearance == .dark ? "Dark selected stroke" : "Light selected stroke",
                        selection: colorBinding(
                            activeColorBinding(\.selectedNodeStroke),
                            default: defaultSelectedStrokeColor
                        )
                    )
                    valueSlider("Selected stroke width", value: $selectedStrokeWidth, range: 0...6, suffix: " pt")
                }
                valueSlider("Corner radius", value: $cornerRadius, range: 0...32, suffix: " pt")
            }

            Section("Node contrast") {
                ColorPicker(
                    appearance == .dark ? "Dark title text" : "Light title text",
                    selection: colorBinding(
                        activeColorBinding(\.titleText),
                        default: defaultTitleTextColor
                    )
                )
                ColorPicker(
                    appearance == .dark ? "Dark detail text" : "Light detail text",
                    selection: colorBinding(
                        activeColorBinding(\.detailText),
                        default: defaultDetailTextColor
                    )
                )
            }

            if nodeSurfaceStyle == .glass {
                Section("Node glass") {
                Picker("Unselected", selection: $idleGlassTreatment) {
                    ForEach(PhotaraGraphGlassTreatment.allCases) { treatment in
                        Text(treatment.title).tag(treatment)
                    }
                }
                .pickerStyle(.segmented)
                ColorPicker(
                    appearance == .dark ? "Dark unselected tint color" : "Light unselected tint color",
                    selection: colorBinding(
                        activeColorBinding(\.idleGlassTint),
                        default: theme?.color(.nodeNative) ?? .accentColor
                    )
                )
                valueSlider("Unselected tint", value: $idleGlassTintOpacity, range: 0...0.18)
                valueSlider(
                    appearance == .dark ? "Dark unselected glass opacity" : "Light unselected glass opacity",
                    value: activeIdleGlassOpacityBinding,
                    range: 0.2...1
                )

                Picker("Selected", selection: $selectedGlassTreatment) {
                    ForEach(PhotaraGraphGlassTreatment.allCases) { treatment in
                        Text(treatment.title).tag(treatment)
                    }
                }
                .pickerStyle(.segmented)
                ColorPicker(
                    appearance == .dark ? "Dark selected tint color" : "Light selected tint color",
                    selection: colorBinding(
                        activeColorBinding(\.selectedGlassTint),
                        default: theme?.color(.nodeNative) ?? .accentColor
                    )
                )
                valueSlider("Selected tint", value: $selectedGlassTintOpacity, range: 0...0.18)
                valueSlider(
                    appearance == .dark ? "Dark selected glass opacity" : "Light selected glass opacity",
                    value: activeSelectedGlassOpacityBinding,
                    range: 0.2...1
                )
                Text("Regular and Clear are Apple's public optical treatments. Tint changes glass color; opacity changes only the native glass surface, never its text or ports.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                if reduceTransparency {
                    Text("Reduce Transparency is active: semantic opaque surfaces replace glass.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
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
                ColorPicker(
                    appearance == .dark ? "Dark glass tint color" : "Light glass tint color",
                    selection: colorBinding(
                        activeColorBinding(\.portGlassTint),
                        default: theme?.color(.borderFocus) ?? .accentColor
                    )
                )
                valueSlider("Glass tint", value: $portGlassTintOpacity, range: 0...0.5)
                valueSlider("Inner dot size", value: $portCoreSize, range: 3...10, suffix: " pt")
                Text("Assets and Input are connected; neighboring ports remain disconnected for comparison.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Section("Inactive port dot") {
                valueSlider("Saturation", value: $inactivePortSaturation, range: 0...1)
                valueSlider(
                    appearance == .dark ? "Dark brightness" : "Light brightness",
                    value: activeInactivePortBrightnessBinding,
                    range: -0.25...0.35
                )
                Toggle("Stroke", isOn: $inactivePortShowsStroke)
                if inactivePortShowsStroke {
                    valueSlider("Stroke width", value: $inactivePortStrokeWidth, range: 0.5...3, suffix: " pt")
                }
            }

            Section("Active port dot") {
                valueSlider(
                    appearance == .dark ? "Dark brightness" : "Light brightness",
                    value: activeConnectedPortBrightnessBinding,
                    range: -0.25...0.35
                )
                Text("Dot size stays constant and the active dot remains centered over the glass bead.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Section("Active port shadow") {
                Toggle("Enabled", isOn: $activePortShowsShadow)
                if activePortShowsShadow {
                    valueSlider("Opacity", value: $activePortShadowOpacity, range: 0...0.8)
                    valueSlider("Blur", value: $activePortShadowBlur, range: 0...10, suffix: " pt")
                    valueSlider("Vertical offset", value: $activePortShadowOffsetY, range: -4...8, suffix: " pt")
                }
                Text("The semantic-colored shadow belongs only to a connected glass bead.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Section("Node shadow") {
                valueSlider("Blur", value: $nodeShadowBlur, range: 0...30, suffix: " pt")
                valueSlider(
                    appearance == .dark ? "Dark opacity" : "Light opacity",
                    value: activeNodeShadowOpacityBinding,
                    range: 0...0.5
                )
                valueSlider("Vertical offset", value: $nodeShadowOffsetY, range: -4...18, suffix: " pt")
                Text("Selection uses only the authored stroke; clicking or dragging does not change this shadow.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Section("Noodle") {
                ColorPicker(
                    appearance == .dark ? "Dark noodle color" : "Light noodle color",
                    selection: colorBinding(
                        activeColorBinding(\.noodle),
                        default: theme?.color(.borderFocus) ?? .accentColor
                    )
                )
                Picker("Style", selection: $noodleStyle) {
                    ForEach(PhotaraGraphNoodleStyle.allCases) { style in
                        Text(style.title).tag(style)
                    }
                }
                .pickerStyle(.segmented)
                Button(activeNoodleKnot == nil ? "Add routing knot" : "Remove routing knot") {
                    guard let id = connectionForKnotEditingID else { return }
                    if activeNoodleKnot == nil {
                        setNoodleKnot(defaultNoodleKnotPosition(connectionID: id), connectionID: id)
                    } else {
                        removeNoodleKnot(connectionID: id)
                    }
                }
                .disabled(controller.document.connections.isEmpty)
                Text("The noodle follows both ports live. A routing knot bends the path without changing the connection; drag the knot to branch, or Option-drag to move it.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Section {
                Button("Reset All") { reset() }
                Button("Save Preferences") { savePreferences() }
                if let preferencesStatus {
                    Text(preferencesStatus)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
        }
        .formStyle(.grouped)
    }

    private func activeColor(_ keyPath: KeyPath<GraphLabAppearanceColors, Color?>) -> Color? {
        let colors = appearance == .dark ? darkColors : lightColors
        return colors[keyPath: keyPath]
    }

    private func activeColorBinding(
        _ keyPath: WritableKeyPath<GraphLabAppearanceColors, Color?>
    ) -> Binding<Color?> {
        Binding(
            get: { activeColor(keyPath) },
            set: { color in
                if appearance == .dark {
                    darkColors[keyPath: keyPath] = color
                } else {
                    lightColors[keyPath: keyPath] = color
                }
            }
        )
    }

    private var defaultTitleTextColor: Color {
        appearance == .dark ? .white : (theme?.color(.textPrimary) ?? .primary)
    }

    private var activeIdleGlassOpacityBinding: Binding<Double> {
        Binding(
            get: { appearance == .dark ? darkIdleGlassOpacity : lightIdleGlassOpacity },
            set: { value in
                if appearance == .dark {
                    darkIdleGlassOpacity = value
                } else {
                    lightIdleGlassOpacity = value
                }
            }
        )
    }

    private var activeSelectedGlassOpacityBinding: Binding<Double> {
        Binding(
            get: { appearance == .dark ? darkSelectedGlassOpacity : lightSelectedGlassOpacity },
            set: { value in
                if appearance == .dark {
                    darkSelectedGlassOpacity = value
                } else {
                    lightSelectedGlassOpacity = value
                }
            }
        )
    }

    private var defaultDetailTextColor: Color {
        appearance == .dark ? Color.white.opacity(0.86) : (theme?.color(.textSecondary) ?? .secondary)
    }

    private var defaultFlatNodeColor: Color {
        theme?.color(.graphNode) ?? Color(nsColor: .controlBackgroundColor)
    }

    private var defaultSelectedStrokeColor: Color {
        appearance == .dark ? Color.white.opacity(0.72) : Color.black.opacity(0.42)
    }

    private var activeInactivePortBrightnessBinding: Binding<Double> {
        Binding(
            get: { appearance == .dark ? darkPortCoreBrightness : lightPortCoreBrightness },
            set: { value in
                if appearance == .dark {
                    darkPortCoreBrightness = value
                } else {
                    lightPortCoreBrightness = value
                }
            }
        )
    }

    private var activeConnectedPortBrightnessBinding: Binding<Double> {
        Binding(
            get: { appearance == .dark ? darkActivePortCoreBrightness : lightActivePortCoreBrightness },
            set: { value in
                if appearance == .dark {
                    darkActivePortCoreBrightness = value
                } else {
                    lightActivePortCoreBrightness = value
                }
            }
        )
    }

    private var activeNodeShadowOpacityBinding: Binding<Double> {
        Binding(
            get: { appearance == .dark ? darkNodeShadowOpacity : lightNodeShadowOpacity },
            set: { value in
                if appearance == .dark {
                    darkNodeShadowOpacity = value
                } else {
                    lightNodeShadowOpacity = value
                }
            }
        )
    }

    private func nodeStyle() -> PhotaraGraphNodeStyle {
        return PhotaraGraphNodeStyle(
            cornerRadius: cornerRadius,
            portShape: portShape,
            portOffset: portOffset,
            shadowBlur: nodeShadowBlur,
            shadowOpacity: appearance == .dark ? darkNodeShadowOpacity : lightNodeShadowOpacity,
            shadowOffsetY: nodeShadowOffsetY
        )
    }

    private func specimen(
        node: PhotaraGraphNode,
        isSelected: Bool,
        connectedInputs: Set<Int>,
        connectedOutputs: Set<Int>
    ) -> some View {
        let glassTreatment = isSelected ? selectedGlassTreatment : idleGlassTreatment
        return
            GraphLabNodeSpecimen(
                title: node.title,
                subtitle: node.subtitle,
                presentation: GraphLabFixtures.presentation(for: node.kind),
                inputs: node.ports(.input),
                outputs: node.ports(.output),
                style: nodeStyle(),
                glassTreatment: nodeSurfaceStyle == .glass ? glassTreatment : nil,
                flatFillColor: activeColor(\.idleNodeFill) ?? defaultFlatNodeColor,
                flatStrokeColor: isSelected
                    ? (activeColor(\.selectedNodeStroke) ?? defaultSelectedStrokeColor)
                    : nil,
                flatStrokeWidth: isSelected ? selectedStrokeWidth : 0.5,
                titleColor: activeColor(\.titleText) ?? defaultTitleTextColor,
                detailColor: activeColor(\.detailText) ?? defaultDetailTextColor,
                glassTintColor: isSelected
                    ? (activeColor(\.selectedGlassTint) ?? theme?.color(.nodeNative) ?? .accentColor)
                    : (activeColor(\.idleGlassTint) ?? theme?.color(.nodeNative) ?? .accentColor),
                glassTintOpacity: isSelected ? selectedGlassTintOpacity : idleGlassTintOpacity,
                glassOpacity: isSelected
                    ? (appearance == .dark ? darkSelectedGlassOpacity : lightSelectedGlassOpacity)
                    : (appearance == .dark ? darkIdleGlassOpacity : lightIdleGlassOpacity),
                portGlassTreatment: portGlassTreatment,
                portGlassTintColor: activeColor(\.portGlassTint) ?? theme?.color(.borderFocus) ?? .accentColor,
                portGlassTintOpacity: portGlassTintOpacity,
                inactivePortCoreBrightness: appearance == .dark ? darkPortCoreBrightness : lightPortCoreBrightness,
                activePortCoreBrightness: appearance == .dark
                    ? darkActivePortCoreBrightness
                    : lightActivePortCoreBrightness,
                portCoreSize: portCoreSize,
                inactivePortSaturation: inactivePortSaturation,
                inactivePortShowsStroke: inactivePortShowsStroke,
                inactivePortStrokeWidth: inactivePortStrokeWidth,
                activePortShowsShadow: activePortShowsShadow,
                activePortShadowOpacity: activePortShadowOpacity,
                activePortShadowBlur: activePortShadowBlur,
                activePortShadowOffsetY: activePortShadowOffsetY,
                connectedInputs: connectedInputs,
                connectedOutputs: connectedOutputs
            )
    }

    private var connectionForKnotEditingID: String? {
        switch controller.selection {
        case .noodle(let id), .knot(let id): return id
        default: return controller.document.connections.first?.id
        }
    }
    private var activeNoodleKnot: CGPoint? {
        guard let id = connectionForKnotEditingID else { return nil }
        return controller.document.connections.first(where: { $0.id == id }).flatMap { controller.knot(of: $0) }
    }
    private func setNoodleKnot(_ point: CGPoint, connectionID: String) {
        controller.setKnot(point, connectionID: connectionID)
    }
    private func removeNoodleKnot(connectionID: String) {
        controller.setKnot(nil, connectionID: connectionID)
    }
    private func defaultNoodleKnotPosition(connectionID: String) -> CGPoint {
        controller.defaultKnot(connectionID: connectionID)
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

    private func savePreferences() {
        let savedKnot = controller.document.connections.first.flatMap { controller.document.knot(of: $0) }
        let preferences = GraphLabSavedPreferences(
            appearance: appearance.rawValue,
            pattern: pattern.rawValue,
            gridSpacing: gridSpacing,
            minorOpacity: minorOpacity,
            minorLineWidth: minorLineWidth,
            minorMarkSize: minorMarkSize,
            majorInterval: majorInterval,
            majorOpacity: majorOpacity,
            majorLineWidth: majorLineWidth,
            majorMarkSize: majorMarkSize,
            graphBackgroundColor: nil,
            lightGraphBackgroundColor: nil,
            darkGraphBackgroundColor: nil,
            minorColor: nil,
            majorColor: nil,
            noodleColor: nil,
            lightColors: GraphLabSavedPalette(lightColors),
            darkColors: GraphLabSavedPalette(darkColors),
            noodleStyle: noodleStyle.rawValue,
            overviewPolicy: controller.overviewPolicy.rawValue,
            overviewSizeFraction: controller.overviewSizeFraction,
            overviewPosition: controller.overviewPosition.rawValue,
            overviewCornerRadius: controller.overviewCornerRadius,
            noodleKnotX: savedKnot.map { Double($0.x) },
            noodleKnotY: savedKnot.map { Double($0.y) },
            nodeSurfaceStyle: nodeSurfaceStyle.rawValue,
            idleGlassTreatment: idleGlassTreatment.rawValue,
            selectedGlassTreatment: selectedGlassTreatment.rawValue,
            idleGlassTintColor: nil,
            selectedGlassTintColor: nil,
            idleGlassTintOpacity: idleGlassTintOpacity,
            selectedGlassTintOpacity: selectedGlassTintOpacity,
            lightIdleGlassOpacity: lightIdleGlassOpacity,
            darkIdleGlassOpacity: darkIdleGlassOpacity,
            lightSelectedGlassOpacity: lightSelectedGlassOpacity,
            darkSelectedGlassOpacity: darkSelectedGlassOpacity,
            selectedStrokeWidth: selectedStrokeWidth,
            cornerRadius: cornerRadius,
            portShape: portShape.rawValue,
            portOffset: portOffset,
            portGlassTreatment: portGlassTreatment.rawValue,
            portGlassTintColor: nil,
            portGlassTintOpacity: portGlassTintOpacity,
            showsPortCutouts: nil,
            portCutoutRadius: nil,
            showsPortRecesses: nil,
            portRecessDepth: nil,
            inactivePortCoreScale: nil,
            activePortCoreScale: nil,
            portCoreSize: portCoreSize,
            inactivePortSaturation: inactivePortSaturation,
            inactivePortOpacity: nil,
            inactivePortShowsStroke: inactivePortShowsStroke,
            inactivePortStrokeWidth: inactivePortStrokeWidth,
            activePortShowsGlow: nil,
            activePortGlowOpacity: nil,
            activePortGlowRadius: nil,
            activePortShowsShadow: activePortShowsShadow,
            activePortShadowOpacity: activePortShadowOpacity,
            activePortShadowBlur: activePortShadowBlur,
            activePortShadowOffsetY: activePortShadowOffsetY,
            lightPortCoreBrightness: lightPortCoreBrightness,
            darkPortCoreBrightness: darkPortCoreBrightness,
            lightActivePortCoreBrightness: lightActivePortCoreBrightness,
            darkActivePortCoreBrightness: darkActivePortCoreBrightness,
            shadowBlur: nodeShadowBlur,
            shadowOpacity: lightNodeShadowOpacity,
            shadowOffsetY: nodeShadowOffsetY,
            clearShadowBlur: nil,
            clearShadowOpacity: nil,
            clearShadowOffsetY: nil,
            lightRestingShadowOpacity: lightNodeShadowOpacity,
            darkRestingShadowOpacity: darkNodeShadowOpacity,
            lightLiftedShadowOpacity: nil,
            darkLiftedShadowOpacity: nil
        )

        do {
            let data = try JSONEncoder().encode(preferences)
            UserDefaults.standard.set(data, forKey: Self.preferencesKey)
            preferencesStatus = "Saved for the next Graph Lab launch."
        } catch {
            preferencesStatus = "Could not save preferences."
        }
    }

    private func loadPreferencesIfAvailable() {
        guard !didLoadPreferences else { return }
        didLoadPreferences = true
        guard let data = UserDefaults.standard.data(forKey: Self.preferencesKey),
              let preferences = try? JSONDecoder().decode(GraphLabSavedPreferences.self, from: data)
        else { return }

        appearance = PhotaraThemeAppearance(rawValue: preferences.appearance) ?? appearance
        pattern = PhotaraGraphPattern(rawValue: preferences.pattern) ?? pattern
        gridSpacing = preferences.gridSpacing
        minorOpacity = preferences.minorOpacity
        minorLineWidth = preferences.minorLineWidth
        minorMarkSize = preferences.minorMarkSize
        majorInterval = preferences.majorInterval
        majorOpacity = preferences.majorOpacity
        majorLineWidth = preferences.majorLineWidth
        majorMarkSize = preferences.majorMarkSize
        loadSavedColors(preferences)
        noodleStyle = PhotaraGraphNoodleStyle(rawValue: preferences.noodleStyle ?? "") ?? noodleStyle
        controller.overviewPolicy = PhotaraGraphOverviewPolicy(savedValue: preferences.overviewPolicy)
        controller.overviewSizeFraction = PhotaraGraphOverviewSizing.fraction(preferences.overviewSizeFraction)
        controller.overviewPosition = PhotaraGraphOverviewPosition(savedValue: preferences.overviewPosition)
        controller.overviewCornerRadius = PhotaraGraphOverviewSizing.cornerRadius(preferences.overviewCornerRadius)
        if let x = preferences.noodleKnotX, let y = preferences.noodleKnotY {
            let point = CGPoint(x: x, y: y)
            if let id = controller.document.connections.first?.id {
                controller.setKnot(point, connectionID: id)
                controller.select(.node("transform"))
            }
        }
        nodeSurfaceStyle = GraphLabNodeSurfaceStyle(rawValue: preferences.nodeSurfaceStyle ?? "") ?? nodeSurfaceStyle
        idleGlassTreatment = PhotaraGraphGlassTreatment(rawValue: preferences.idleGlassTreatment) ?? idleGlassTreatment
        selectedGlassTreatment = PhotaraGraphGlassTreatment(rawValue: preferences.selectedGlassTreatment) ?? selectedGlassTreatment
        idleGlassTintOpacity = preferences.idleGlassTintOpacity
        selectedGlassTintOpacity = preferences.selectedGlassTintOpacity
        lightIdleGlassOpacity = preferences.lightIdleGlassOpacity ?? lightIdleGlassOpacity
        darkIdleGlassOpacity = preferences.darkIdleGlassOpacity ?? darkIdleGlassOpacity
        lightSelectedGlassOpacity = preferences.lightSelectedGlassOpacity ?? lightSelectedGlassOpacity
        darkSelectedGlassOpacity = preferences.darkSelectedGlassOpacity ?? darkSelectedGlassOpacity
        selectedStrokeWidth = preferences.selectedStrokeWidth ?? selectedStrokeWidth
        cornerRadius = preferences.cornerRadius
        portShape = PhotaraGraphPortShape(rawValue: preferences.portShape) ?? portShape
        portOffset = preferences.portOffset
        portGlassTreatment = PhotaraGraphGlassTreatment(rawValue: preferences.portGlassTreatment) ?? portGlassTreatment
        portGlassTintOpacity = preferences.portGlassTintOpacity
        portCoreSize = preferences.portCoreSize ?? portCoreSize
        inactivePortSaturation = preferences.inactivePortSaturation ?? inactivePortSaturation
        inactivePortShowsStroke = preferences.inactivePortShowsStroke ?? inactivePortShowsStroke
        inactivePortStrokeWidth = preferences.inactivePortStrokeWidth ?? inactivePortStrokeWidth
        activePortShowsShadow = preferences.activePortShowsShadow
            ?? preferences.activePortShowsGlow
            ?? activePortShowsShadow
        activePortShadowOpacity = preferences.activePortShadowOpacity
            ?? preferences.activePortGlowOpacity
            ?? activePortShadowOpacity
        activePortShadowBlur = preferences.activePortShadowBlur
            ?? preferences.activePortGlowRadius
            ?? activePortShadowBlur
        activePortShadowOffsetY = preferences.activePortShadowOffsetY ?? activePortShadowOffsetY
        lightPortCoreBrightness = preferences.lightPortCoreBrightness ?? lightPortCoreBrightness
        darkPortCoreBrightness = preferences.darkPortCoreBrightness ?? darkPortCoreBrightness
        lightActivePortCoreBrightness = preferences.lightActivePortCoreBrightness ?? lightActivePortCoreBrightness
        darkActivePortCoreBrightness = preferences.darkActivePortCoreBrightness ?? darkActivePortCoreBrightness
        nodeShadowBlur = preferences.shadowBlur
        nodeShadowOffsetY = preferences.shadowOffsetY
        lightNodeShadowOpacity = preferences.lightRestingShadowOpacity ?? preferences.shadowOpacity
        darkNodeShadowOpacity = preferences.darkRestingShadowOpacity ?? preferences.shadowOpacity
        preferencesStatus = "Loaded saved preferences."
    }

    private func loadSavedColors(_ preferences: GraphLabSavedPreferences) {
        if preferences.lightColors != nil || preferences.darkColors != nil {
            lightColors = preferences.lightColors?.colors ?? GraphLabAppearanceColors()
            darkColors = preferences.darkColors?.colors ?? GraphLabAppearanceColors()
            return
        }

        let legacyColors = GraphLabAppearanceColors(
            graphBackground: preferences.graphBackgroundColor?.color,
            minor: preferences.minorColor?.color,
            major: preferences.majorColor?.color,
            noodle: preferences.noodleColor?.color,
            idleGlassTint: preferences.idleGlassTintColor?.color,
            selectedGlassTint: preferences.selectedGlassTintColor?.color,
            portGlassTint: preferences.portGlassTintColor?.color
        )

        if preferences.lightGraphBackgroundColor != nil || preferences.darkGraphBackgroundColor != nil {
            lightColors = legacyColors
            darkColors = legacyColors
            lightColors.graphBackground = preferences.lightGraphBackgroundColor?.color
            darkColors.graphBackground = preferences.darkGraphBackgroundColor?.color
            return
        }

        if appearance == .dark {
            darkColors = legacyColors
        } else {
            lightColors = legacyColors
        }
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
        lightColors = GraphLabAppearanceColors()
        darkColors = GraphLabAppearanceColors()
        noodleStyle = .curved
        controller.overviewPolicy = .whileZooming
        controller.overviewSizeFraction = 0.16
        controller.overviewPosition = .topRight
        controller.overviewCornerRadius = 12
        try? controller.replaceDocument(GraphLabFixtures.document)
        nodeSurfaceStyle = .flat
        idleGlassTreatment = .regular
        selectedGlassTreatment = .clear
        idleGlassTintOpacity = 0.05
        selectedGlassTintOpacity = 0.025
        lightIdleGlassOpacity = 1
        darkIdleGlassOpacity = 1
        lightSelectedGlassOpacity = 1
        darkSelectedGlassOpacity = 1
        selectedStrokeWidth = 2
        cornerRadius = 12
        portShape = .round
        portOffset = 0
        portGlassTreatment = .clear
        portGlassTintOpacity = 0.18
        portCoreSize = 5
        inactivePortSaturation = 0.18
        inactivePortShowsStroke = true
        inactivePortStrokeWidth = 1
        activePortShowsShadow = true
        activePortShadowOpacity = 0.32
        activePortShadowBlur = 3
        activePortShadowOffsetY = 1.5
        lightPortCoreBrightness = 0
        darkPortCoreBrightness = 0.18
        lightActivePortCoreBrightness = 0.12
        darkActivePortCoreBrightness = 0.28
        nodeShadowBlur = 5
        nodeShadowOffsetY = 3
        lightNodeShadowOpacity = 0.12
        darkNodeShadowOpacity = 0.18
        centerScene()
    }

    private func centerScene() {
        controller.center(positions: Dictionary(uniqueKeysWithValues: GraphLabFixtures.document.nodes.map { ($0.id, $0.position) }), selectedNode: "transform")
    }
}

struct GraphLabSavedPreferences: Codable {
    let appearance: String
    let pattern: String
    let gridSpacing: Double
    let minorOpacity: Double
    let minorLineWidth: Double
    let minorMarkSize: Double
    let majorInterval: Int
    let majorOpacity: Double
    let majorLineWidth: Double
    let majorMarkSize: Double
    let graphBackgroundColor: GraphLabSavedColor?
    let lightGraphBackgroundColor: GraphLabSavedColor?
    let darkGraphBackgroundColor: GraphLabSavedColor?
    let minorColor: GraphLabSavedColor?
    let majorColor: GraphLabSavedColor?
    let noodleColor: GraphLabSavedColor?
    let lightColors: GraphLabSavedPalette?
    let darkColors: GraphLabSavedPalette?
    let noodleStyle: String?
    var overviewPolicy: String? = nil
    var overviewSizeFraction: Double? = nil
    var overviewPosition: String? = nil
    var overviewCornerRadius: Double? = nil
    let noodleKnotX: Double?
    let noodleKnotY: Double?
    let nodeSurfaceStyle: String?
    let idleGlassTreatment: String
    let selectedGlassTreatment: String
    let idleGlassTintColor: GraphLabSavedColor?
    let selectedGlassTintColor: GraphLabSavedColor?
    let idleGlassTintOpacity: Double
    let selectedGlassTintOpacity: Double
    let lightIdleGlassOpacity: Double?
    let darkIdleGlassOpacity: Double?
    let lightSelectedGlassOpacity: Double?
    let darkSelectedGlassOpacity: Double?
    let selectedStrokeWidth: Double?
    let cornerRadius: Double
    let portShape: String
    let portOffset: Double
    let portGlassTreatment: String
    let portGlassTintColor: GraphLabSavedColor?
    let portGlassTintOpacity: Double
    let showsPortCutouts: Bool?
    let portCutoutRadius: Double?
    let showsPortRecesses: Bool?
    let portRecessDepth: Double?
    let inactivePortCoreScale: Double?
    let activePortCoreScale: Double?
    let portCoreSize: Double?
    let inactivePortSaturation: Double?
    let inactivePortOpacity: Double?
    let inactivePortShowsStroke: Bool?
    let inactivePortStrokeWidth: Double?
    let activePortShowsGlow: Bool?
    let activePortGlowOpacity: Double?
    let activePortGlowRadius: Double?
    let activePortShowsShadow: Bool?
    let activePortShadowOpacity: Double?
    let activePortShadowBlur: Double?
    let activePortShadowOffsetY: Double?
    let lightPortCoreBrightness: Double?
    let darkPortCoreBrightness: Double?
    let lightActivePortCoreBrightness: Double?
    let darkActivePortCoreBrightness: Double?
    let shadowBlur: Double
    let shadowOpacity: Double
    let shadowOffsetY: Double
    let clearShadowBlur: Double?
    let clearShadowOpacity: Double?
    let clearShadowOffsetY: Double?
    let lightRestingShadowOpacity: Double?
    let darkRestingShadowOpacity: Double?
    let lightLiftedShadowOpacity: Double?
    let darkLiftedShadowOpacity: Double?
}

struct GraphLabAppearanceColors {
    var graphBackground: Color?
    var minor: Color?
    var major: Color?
    var noodle: Color?
    var idleNodeFill: Color?
    var selectedNodeFill: Color?
    var selectedNodeStroke: Color?
    var idleGlassTint: Color?
    var selectedGlassTint: Color?
    var portGlassTint: Color?
    var titleText: Color?
    var detailText: Color?
}

struct GraphLabSavedPalette: Codable {
    let graphBackground: GraphLabSavedColor?
    let minor: GraphLabSavedColor?
    let major: GraphLabSavedColor?
    let noodle: GraphLabSavedColor?
    let idleNodeFill: GraphLabSavedColor?
    let selectedNodeFill: GraphLabSavedColor?
    let selectedNodeStroke: GraphLabSavedColor?
    let idleGlassTint: GraphLabSavedColor?
    let selectedGlassTint: GraphLabSavedColor?
    let portGlassTint: GraphLabSavedColor?
    let titleText: GraphLabSavedColor?
    let detailText: GraphLabSavedColor?

    init(_ colors: GraphLabAppearanceColors) {
        graphBackground = GraphLabSavedColor(colors.graphBackground)
        minor = GraphLabSavedColor(colors.minor)
        major = GraphLabSavedColor(colors.major)
        noodle = GraphLabSavedColor(colors.noodle)
        idleNodeFill = GraphLabSavedColor(colors.idleNodeFill)
        selectedNodeFill = GraphLabSavedColor(colors.selectedNodeFill)
        selectedNodeStroke = GraphLabSavedColor(colors.selectedNodeStroke)
        idleGlassTint = GraphLabSavedColor(colors.idleGlassTint)
        selectedGlassTint = GraphLabSavedColor(colors.selectedGlassTint)
        portGlassTint = GraphLabSavedColor(colors.portGlassTint)
        titleText = GraphLabSavedColor(colors.titleText)
        detailText = GraphLabSavedColor(colors.detailText)
    }

    var colors: GraphLabAppearanceColors {
        GraphLabAppearanceColors(
            graphBackground: graphBackground?.color,
            minor: minor?.color,
            major: major?.color,
            noodle: noodle?.color,
            idleNodeFill: idleNodeFill?.color,
            selectedNodeFill: selectedNodeFill?.color,
            selectedNodeStroke: selectedNodeStroke?.color,
            idleGlassTint: idleGlassTint?.color,
            selectedGlassTint: selectedGlassTint?.color,
            portGlassTint: portGlassTint?.color,
            titleText: titleText?.color,
            detailText: detailText?.color
        )
    }
}

struct GraphLabSavedColor: Codable {
    let red: Double
    let green: Double
    let blue: Double
    let opacity: Double

    init?(_ color: Color?) {
        guard let color,
              let resolved = NSColor(color).usingColorSpace(.sRGB)
        else { return nil }
        red = Double(resolved.redComponent)
        green = Double(resolved.greenComponent)
        blue = Double(resolved.blueComponent)
        opacity = Double(resolved.alphaComponent)
    }

    var color: Color {
        Color(.sRGB, red: red, green: green, blue: blue, opacity: opacity)
    }
}

private struct GraphLabNodeSpecimen: View {
    @Environment(\.photaraTheme) private var theme
    @Environment(\.photaraGraphPresentationZoom) private var zoom
    let title: String
    let subtitle: String
    let presentation: PhotaraGraphNodePresentationMetadata
    let inputs: [PhotaraGraphPortDefinition]
    let outputs: [PhotaraGraphPortDefinition]
    let style: PhotaraGraphNodeStyle
    let glassTreatment: PhotaraGraphGlassTreatment?
    let flatFillColor: Color
    let flatStrokeColor: Color?
    let flatStrokeWidth: Double
    let titleColor: Color
    let detailColor: Color
    let glassTintColor: Color
    let glassTintOpacity: Double
    let glassOpacity: Double
    let portGlassTreatment: PhotaraGraphGlassTreatment
    let portGlassTintColor: Color
    let portGlassTintOpacity: Double
    let inactivePortCoreBrightness: Double
    let activePortCoreBrightness: Double
    let portCoreSize: Double
    let inactivePortSaturation: Double
    let inactivePortShowsStroke: Bool
    let inactivePortStrokeWidth: Double
    let activePortShowsShadow: Bool
    let activePortShadowOpacity: Double
    let activePortShadowBlur: Double
    let activePortShadowOffsetY: Double
    let connectedInputs: Set<Int>
    let connectedOutputs: Set<Int>
    private var width: CGFloat { PhotaraGraphGeometry.nodeWidth * zoom }
    private var rowCount: Int { max(1, max(inputs.count, outputs.count)) }
    private var height: CGFloat {
        (74 + CGFloat(rowCount) * PhotaraGraphGeometry.rowHeight) * zoom
    }

    private var scaledStyle: PhotaraGraphNodeStyle {
        PhotaraGraphNodeStyle(
            cornerRadius: style.cornerRadius * zoom,
            portShape: style.portShape,
            portOffset: style.portOffset * zoom,
            shadowBlur: style.shadowBlur * zoom,
            shadowOpacity: style.shadowOpacity,
            shadowOffsetY: style.shadowOffsetY * zoom
        )
    }

    var body: some View {
        PhotaraGraphNodeSurface(
            style: scaledStyle,
            glassTreatment: glassTreatment,
            glassTint: glassTintColor.opacity(glassTintOpacity),
            glassOpacity: glassOpacity,
            flatFill: flatFillColor,
            flatStroke: flatStrokeColor,
            flatStrokeWidth: flatStrokeWidth * zoom
        ) {
            VStack(spacing: 0) {
                HStack(spacing: 9 * zoom) {
                    PhotaraGraphNodeIcon(
                        resource: presentation.iconResource,
                        color: presentation.category.color,
                        size: 28 * zoom
                    )
                    .frame(width: 32 * zoom, height: 32 * zoom)

                    VStack(alignment: .leading, spacing: 1 * zoom) {
                        Text(title)
                            .font(.system(size: 13 * zoom, weight: .semibold))
                            .foregroundStyle(titleColor)
                        Text(subtitle)
                            .font(.system(size: 11 * zoom))
                            .foregroundStyle(detailColor)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, 14 * zoom)
                .padding(.vertical, 10 * zoom)
                Divider()
                HStack(alignment: .top) {
                    labelColumn(inputs, alignment: .leading)
                    Spacer(minLength: 16 * zoom)
                    labelColumn(outputs, alignment: .trailing)
                }
                .padding(.horizontal, 14 * zoom)
                .padding(.vertical, 8 * zoom)
            }
            .frame(width: width, height: height, alignment: .top)
            .clipShape(RoundedRectangle(cornerRadius: style.cornerRadius * zoom, style: .continuous))
        } ports: {
            GeometryReader { geometry in
                ZStack {
                    GlassEffectContainer(spacing: 0) {
                        ForEach(0..<inputs.count, id: \.self) { index in
                            portShell(label: inputs[index].label, at: index, isConnected: connectedInputs.contains(index))
                                .position(x: -style.portOffset * zoom, y: portY(index))
                        }
                        ForEach(0..<outputs.count, id: \.self) { index in
                            portShell(label: outputs[index].label, at: index, isConnected: connectedOutputs.contains(index))
                                .position(x: geometry.size.width + style.portOffset * zoom, y: portY(index))
                        }
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)

                    ForEach(0..<inputs.count, id: \.self) { index in
                        portCore(
                            label: inputs[index].label,
                            edge: .leading,
                            at: index,
                            isConnected: connectedInputs.contains(index)
                        )
                            .position(x: -style.portOffset * zoom, y: portY(index))
                    }
                    ForEach(0..<outputs.count, id: \.self) { index in
                        portCore(
                            label: outputs[index].label,
                            edge: .trailing,
                            at: index,
                            isConnected: connectedOutputs.contains(index)
                        )
                            .position(x: geometry.size.width + style.portOffset * zoom, y: portY(index))
                    }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .frame(width: width, height: height)
        .contentShape(RoundedRectangle(cornerRadius: style.cornerRadius * zoom, style: .continuous))
    }

    private func labelColumn(_ ports: [PhotaraGraphPortDefinition], alignment: HorizontalAlignment) -> some View {
        VStack(alignment: alignment, spacing: 0) {
            ForEach(ports) { port in
                Text(port.label)
                    .font(.system(size: 10.5 * zoom))
                    .foregroundStyle(detailColor)
                    .frame(height: PhotaraGraphGeometry.rowHeight * zoom)
            }
        }
    }

    @ViewBuilder
    private func portShell(label: String, at index: Int, isConnected: Bool) -> some View {
        Group {
            if isConnected {
                PhotaraGraphPort(
                    shape: style.portShape,
                    width: 14 * zoom,
                    height: 14 * zoom,
                    glassTreatment: portGlassTreatment,
                    glassTint: portGlassTintColor.opacity(portGlassTintOpacity),
                    coreColor: semanticPortColor(for: label),
                    coreBrightness: activePortCoreBrightness,
                    showsCore: false
                )
                .shadow(
                    color: semanticPortColor(for: label).opacity(
                        activePortShowsShadow ? activePortShadowOpacity : 0
                    ),
                    radius: activePortShowsShadow ? activePortShadowBlur * zoom : 0,
                    y: activePortShowsShadow ? activePortShadowOffsetY * zoom : 0
                )
            } else {
                Color.clear
                    .frame(width: (style.portShape == .pill ? 21.7 : 14) * zoom, height: 14 * zoom)
                    .contentShape(Rectangle().inset(by: -6 * zoom))
            }
        }
        .accessibilityHidden(true)
    }

    @ViewBuilder
    private func portCore(
        label: String,
        edge: PhotaraGraphNodeEdge,
        at index: Int,
        isConnected: Bool
    ) -> some View {
        let color = semanticPortColor(for: label)
        Group {
            if isConnected {
                portShape
                    .fill(color)
                    .brightness(activePortCoreBrightness)
                    .frame(width: portCoreWidth * zoom, height: portCoreSize * zoom)
                    .allowedDynamicRange(.standard)
            } else {
                ZStack {
                    portShape.fill(color)
                    if inactivePortShowsStroke {
                        portShape.stroke(color, lineWidth: inactivePortStrokeWidth * zoom)
                    }
                }
                .saturation(inactivePortSaturation)
                .brightness(inactivePortCoreBrightness)
                .frame(width: portCoreWidth * zoom, height: portCoreSize * zoom)
                .allowedDynamicRange(.standard)
            }
        }
        .frame(width: 36 * zoom, height: 22 * zoom)
        .contentShape(Rectangle())
        .accessibilityLabel("\(label) port \(index + 1)")
        .accessibilityValue(isConnected ? "Connected" : "Not connected")
        .accessibilityHint("Drag to create or reconnect a noodle")
    }

    private var portCoreWidth: CGFloat {
        style.portShape == .pill ? portCoreSize * 1.55 : portCoreSize
    }

    private var portShape: AnyShape {
        switch style.portShape {
        case .round: AnyShape(Circle())
        case .pill: AnyShape(Capsule())
        }
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
        (PhotaraGraphGeometry.firstPortY + CGFloat(index) * PhotaraGraphGeometry.rowHeight) * zoom
    }
}
