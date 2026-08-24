import AppKit
import Foundation
import SwiftUI

private enum GraphLabNodeID: Hashable {
    case source
    case transform
    case composite
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
    @State private var idleGlassTreatment = PhotaraGraphGlassTreatment.regular
    @State private var selectedGlassTreatment = PhotaraGraphGlassTreatment.clear
    @State private var idleGlassTintOpacity = 0.05
    @State private var selectedGlassTintOpacity = 0.025
    @State private var cornerRadius = 12.0
    @State private var portShape = PhotaraGraphPortShape.round
    @State private var portOffset = 0.0
    @State private var portGlassTreatment = PhotaraGraphGlassTreatment.clear
    @State private var portGlassTintOpacity = 0.18
    @State private var lightPortCoreBrightness = 0.0
    @State private var darkPortCoreBrightness = 0.18
    @State private var restingShadowBlur = 5.0
    @State private var restingShadowOffsetY = 3.0
    @State private var liftedShadowBlur = 12.0
    @State private var liftedShadowOffsetY = 7.0
    @State private var lightRestingShadowOpacity = 0.12
    @State private var darkRestingShadowOpacity = 0.18
    @State private var lightLiftedShadowOpacity = 0.18
    @State private var darkLiftedShadowOpacity = 0.28
    @State private var pan = CGSize.zero
    @State private var zoom = 1.0
    @State private var selectedNode = GraphLabNodeID.transform
    @State private var nodeOffsets: [GraphLabNodeID: CGSize] = [:]
    @State private var didLoadPreferences = false
    @State private var preferencesStatus: String?
    @GestureState private var canvasDrag = CGSize.zero

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
        .onAppear { loadPreferencesIfAvailable() }
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
                    backgroundColor: activeColor(\.graphBackground),
                    minorColor: activeColor(\.minor),
                    majorColor: activeColor(\.major)
                )
                GraphLabNoodleLayer(
                    color: activeColor(\.noodle) ?? theme?.color(.borderFocus) ?? .accentColor,
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
                    canvasSize: geometry.size,
                    pan: displayedPan
                )
                specimen(
                    id: .transform,
                    title: "Transform",
                    subtitle: "Three port rows",
                    inputs: ["Input", "Mask", "Control"],
                    outputs: ["Result", "Preview", "Metadata"],
                    canvasSize: geometry.size,
                    pan: displayedPan
                )
                specimen(
                    id: .composite,
                    title: "Composite",
                    subtitle: "Six port rows",
                    inputs: ["Layer 1", "Layer 2", "Mask", "Depth", "Color", "Control"],
                    outputs: ["Image", "Preview"],
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
                valueSlider(
                    appearance == .dark ? "Dark port brightness" : "Light port brightness",
                    value: activePortCoreBrightnessBinding,
                    range: 0...0.35
                )
                Text("Port brightness preserves each semantic port hue while raising contrast.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Section("Native glass") {
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
                ColorPicker(
                    appearance == .dark ? "Dark glass tint color" : "Light glass tint color",
                    selection: colorBinding(
                        activeColorBinding(\.portGlassTint),
                        default: theme?.color(.borderFocus) ?? .accentColor
                    )
                )
                valueSlider("Glass tint", value: $portGlassTintOpacity, range: 0...0.5)
            }

            Section("Resting shadow") {
                valueSlider("Blur", value: $restingShadowBlur, range: 0...30, suffix: " pt")
                valueSlider(
                    appearance == .dark ? "Dark opacity" : "Light opacity",
                    value: activeRestingShadowOpacityBinding,
                    range: 0...0.5
                )
                valueSlider("Vertical offset", value: $restingShadowOffsetY, range: -4...18, suffix: " pt")
            }

            Section("Lifted shadow") {
                valueSlider("Blur", value: $liftedShadowBlur, range: 0...40, suffix: " pt")
                valueSlider(
                    appearance == .dark ? "Dark opacity" : "Light opacity",
                    value: activeLiftedShadowOpacityBinding,
                    range: 0...0.5
                )
                valueSlider("Vertical offset", value: $liftedShadowOffsetY, range: -4...24, suffix: " pt")
                Text("Applied only while a node is held or dragged; release returns it to the resting shadow.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Section("Backdrop specimen") {
                ColorPicker(
                    appearance == .dark ? "Dark noodle color" : "Light noodle color",
                    selection: colorBinding(
                        activeColorBinding(\.noodle),
                        default: theme?.color(.borderFocus) ?? .accentColor
                    )
                )
                Text("The bright noodle remains fixed in Graph space beneath Transform.")
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

    private var defaultDetailTextColor: Color {
        appearance == .dark ? Color.white.opacity(0.86) : (theme?.color(.textSecondary) ?? .secondary)
    }

    private var activePortCoreBrightnessBinding: Binding<Double> {
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

    private var activeRestingShadowOpacityBinding: Binding<Double> {
        Binding(
            get: { appearance == .dark ? darkRestingShadowOpacity : lightRestingShadowOpacity },
            set: { value in
                if appearance == .dark {
                    darkRestingShadowOpacity = value
                } else {
                    lightRestingShadowOpacity = value
                }
            }
        )
    }

    private var activeLiftedShadowOpacityBinding: Binding<Double> {
        Binding(
            get: { appearance == .dark ? darkLiftedShadowOpacity : lightLiftedShadowOpacity },
            set: { value in
                if appearance == .dark {
                    darkLiftedShadowOpacity = value
                } else {
                    lightLiftedShadowOpacity = value
                }
            }
        )
    }

    private func nodeStyle(isLifted: Bool) -> PhotaraGraphNodeStyle {
        return PhotaraGraphNodeStyle(
            cornerRadius: cornerRadius,
            portShape: portShape,
            portOffset: portOffset,
            shadowBlur: isLifted ? liftedShadowBlur : restingShadowBlur,
            shadowOpacity: isLifted
                ? (appearance == .dark ? darkLiftedShadowOpacity : lightLiftedShadowOpacity)
                : (appearance == .dark ? darkRestingShadowOpacity : lightRestingShadowOpacity),
            shadowOffsetY: isLifted ? liftedShadowOffsetY : restingShadowOffsetY
        )
    }

    private func specimen(
        id: GraphLabNodeID,
        title: String,
        subtitle: String,
        inputs: [String],
        outputs: [String],
        canvasSize: CGSize,
        pan: CGSize
    ) -> some View {
        let storedOffset = nodeOffsets[id] ?? .zero
        let worldPosition = baseWorldPosition(for: id)
        let position = screenPosition(
            CGPoint(
                x: worldPosition.x + storedOffset.width,
                y: worldPosition.y + storedOffset.height
            ),
            canvasSize: canvasSize,
            pan: pan
        )
        let glassTreatment = selectedNode == id ? selectedGlassTreatment : idleGlassTreatment
        return GraphLabDraggableNode(
            zoom: zoom,
            storedOffset: storedOffset,
            position: position,
            constrainedOffset: { collisionConstrainedOffset(for: id, proposed: $0) },
            onBegan: {
                if selectedNode != id { selectedNode = id }
            },
            onEnded: { nodeOffsets[id] = $0 }
        ) { isLifted in
            GraphLabNodeSpecimen(
                title: title,
                subtitle: subtitle,
                inputs: inputs,
                outputs: outputs,
                style: nodeStyle(isLifted: isLifted),
                glassTreatment: glassTreatment,
                titleColor: activeColor(\.titleText) ?? defaultTitleTextColor,
                detailColor: activeColor(\.detailText) ?? defaultDetailTextColor,
                glassTintColor: selectedNode == id
                    ? (activeColor(\.selectedGlassTint) ?? theme?.color(.nodeNative) ?? .accentColor)
                    : (activeColor(\.idleGlassTint) ?? theme?.color(.nodeNative) ?? .accentColor),
                glassTintOpacity: selectedNode == id ? selectedGlassTintOpacity : idleGlassTintOpacity,
                portGlassTreatment: portGlassTreatment,
                portGlassTintColor: activeColor(\.portGlassTint) ?? theme?.color(.borderFocus) ?? .accentColor,
                portGlassTintOpacity: portGlassTintOpacity,
                portCoreBrightness: appearance == .dark ? darkPortCoreBrightness : lightPortCoreBrightness
            )
        }
    }

    private var canvasPanGesture: some Gesture {
        DragGesture()
            .updating($canvasDrag) { value, state, _ in state = value.translation }
            .onEnded {
                pan.width += $0.translation.width
                pan.height += $0.translation.height
            }
    }

    private func collisionConstrainedOffset(for id: GraphLabNodeID, proposed: CGSize) -> CGSize {
        let base = baseWorldPosition(for: id)
        let storedOffset = nodeOffsets[id] ?? .zero
        let start = CGPoint(x: base.x + storedOffset.width, y: base.y + storedOffset.height)
        let target = CGPoint(x: base.x + proposed.width, y: base.y + proposed.height)
        let size = nodeSize(for: id)
        let halfWidth = size.width / 2
        let halfHeight = size.height / 2
        let otherIDs = [GraphLabNodeID.source, .transform, .composite].filter { $0 != id }
        let obstacles = otherIDs.map { otherID -> CGRect in
            let otherOffset = nodeOffsets[otherID] ?? .zero
            let otherBase = baseWorldPosition(for: otherID)
            let otherCenter = CGPoint(
                x: otherBase.x + otherOffset.width,
                y: otherBase.y + otherOffset.height
            )
            let otherSize = nodeSize(for: otherID)
            return CGRect(
                x: otherCenter.x - otherSize.width / 2,
                y: otherCenter.y - otherSize.height / 2,
                width: otherSize.width,
                height: otherSize.height
            ).insetBy(dx: -10, dy: -10)
        }

        var center = start
        let deltaX = target.x - start.x
        var constrainedX = target.x

        for obstacle in obstacles {
            let overlapsVertically = center.y + halfHeight > obstacle.minY
                && center.y - halfHeight < obstacle.maxY
            guard overlapsVertically else { continue }

            if deltaX > 0 {
                let boundary = obstacle.minX - halfWidth
                if start.x <= boundary && constrainedX > boundary {
                    constrainedX = min(constrainedX, boundary)
                }
            } else if deltaX < 0 {
                let boundary = obstacle.maxX + halfWidth
                if start.x >= boundary && constrainedX < boundary {
                    constrainedX = max(constrainedX, boundary)
                }
            }
        }
        center.x = constrainedX

        let deltaY = target.y - start.y
        var constrainedY = target.y

        for obstacle in obstacles {
            let overlapsHorizontally = center.x + halfWidth > obstacle.minX
                && center.x - halfWidth < obstacle.maxX
            guard overlapsHorizontally else { continue }

            if deltaY > 0 {
                let boundary = obstacle.minY - halfHeight
                if start.y <= boundary && constrainedY > boundary {
                    constrainedY = min(constrainedY, boundary)
                }
            } else if deltaY < 0 {
                let boundary = obstacle.maxY + halfHeight
                if start.y >= boundary && constrainedY < boundary {
                    constrainedY = max(constrainedY, boundary)
                }
            }
        }
        center.y = constrainedY

        return CGSize(width: center.x - base.x, height: center.y - base.y)
    }

    private func baseWorldPosition(for id: GraphLabNodeID) -> CGPoint {
        switch id {
        case .source: CGPoint(x: -190, y: -120)
        case .transform: CGPoint(x: 150, y: -90)
        case .composite: CGPoint(x: 0, y: 150)
        }
    }

    private func nodeSize(for id: GraphLabNodeID) -> CGSize {
        switch id {
        case .source: CGSize(width: 224, height: 99)
        case .transform: CGSize(width: 224, height: 149)
        case .composite: CGSize(width: 224, height: 224)
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

    private func savePreferences() {
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
            idleGlassTreatment: idleGlassTreatment.rawValue,
            selectedGlassTreatment: selectedGlassTreatment.rawValue,
            idleGlassTintColor: nil,
            selectedGlassTintColor: nil,
            idleGlassTintOpacity: idleGlassTintOpacity,
            selectedGlassTintOpacity: selectedGlassTintOpacity,
            cornerRadius: cornerRadius,
            portShape: portShape.rawValue,
            portOffset: portOffset,
            portGlassTreatment: portGlassTreatment.rawValue,
            portGlassTintColor: nil,
            portGlassTintOpacity: portGlassTintOpacity,
            lightPortCoreBrightness: lightPortCoreBrightness,
            darkPortCoreBrightness: darkPortCoreBrightness,
            shadowBlur: restingShadowBlur,
            shadowOpacity: lightRestingShadowOpacity,
            shadowOffsetY: restingShadowOffsetY,
            clearShadowBlur: liftedShadowBlur,
            clearShadowOpacity: lightLiftedShadowOpacity,
            clearShadowOffsetY: liftedShadowOffsetY,
            lightRestingShadowOpacity: lightRestingShadowOpacity,
            darkRestingShadowOpacity: darkRestingShadowOpacity,
            lightLiftedShadowOpacity: lightLiftedShadowOpacity,
            darkLiftedShadowOpacity: darkLiftedShadowOpacity
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
        idleGlassTreatment = PhotaraGraphGlassTreatment(rawValue: preferences.idleGlassTreatment) ?? idleGlassTreatment
        selectedGlassTreatment = PhotaraGraphGlassTreatment(rawValue: preferences.selectedGlassTreatment) ?? selectedGlassTreatment
        idleGlassTintOpacity = preferences.idleGlassTintOpacity
        selectedGlassTintOpacity = preferences.selectedGlassTintOpacity
        cornerRadius = preferences.cornerRadius
        portShape = PhotaraGraphPortShape(rawValue: preferences.portShape) ?? portShape
        portOffset = preferences.portOffset
        portGlassTreatment = PhotaraGraphGlassTreatment(rawValue: preferences.portGlassTreatment) ?? portGlassTreatment
        portGlassTintOpacity = preferences.portGlassTintOpacity
        lightPortCoreBrightness = preferences.lightPortCoreBrightness ?? lightPortCoreBrightness
        darkPortCoreBrightness = preferences.darkPortCoreBrightness ?? darkPortCoreBrightness
        restingShadowBlur = preferences.shadowBlur
        restingShadowOffsetY = preferences.shadowOffsetY
        liftedShadowBlur = preferences.clearShadowBlur ?? preferences.shadowBlur
        liftedShadowOffsetY = preferences.clearShadowOffsetY ?? preferences.shadowOffsetY
        lightRestingShadowOpacity = preferences.lightRestingShadowOpacity ?? preferences.shadowOpacity
        darkRestingShadowOpacity = preferences.darkRestingShadowOpacity ?? preferences.shadowOpacity
        lightLiftedShadowOpacity = preferences.lightLiftedShadowOpacity
            ?? preferences.clearShadowOpacity
            ?? preferences.shadowOpacity
        darkLiftedShadowOpacity = preferences.darkLiftedShadowOpacity
            ?? preferences.clearShadowOpacity
            ?? preferences.shadowOpacity
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
        idleGlassTreatment = .regular
        selectedGlassTreatment = .clear
        idleGlassTintOpacity = 0.05
        selectedGlassTintOpacity = 0.025
        cornerRadius = 12
        portShape = .round
        portOffset = 0
        portGlassTreatment = .clear
        portGlassTintOpacity = 0.18
        lightPortCoreBrightness = 0
        darkPortCoreBrightness = 0.18
        restingShadowBlur = 5
        restingShadowOffsetY = 3
        liftedShadowBlur = 12
        liftedShadowOffsetY = 7
        lightRestingShadowOpacity = 0.12
        darkRestingShadowOpacity = 0.18
        lightLiftedShadowOpacity = 0.18
        darkLiftedShadowOpacity = 0.28
        centerScene()
    }

    private func centerScene() {
        pan = .zero
        zoom = 1
        selectedNode = .transform
        nodeOffsets = [:]
    }
}

private struct GraphLabSavedPreferences: Codable {
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
    let idleGlassTreatment: String
    let selectedGlassTreatment: String
    let idleGlassTintColor: GraphLabSavedColor?
    let selectedGlassTintColor: GraphLabSavedColor?
    let idleGlassTintOpacity: Double
    let selectedGlassTintOpacity: Double
    let cornerRadius: Double
    let portShape: String
    let portOffset: Double
    let portGlassTreatment: String
    let portGlassTintColor: GraphLabSavedColor?
    let portGlassTintOpacity: Double
    let lightPortCoreBrightness: Double?
    let darkPortCoreBrightness: Double?
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

private struct GraphLabAppearanceColors {
    var graphBackground: Color?
    var minor: Color?
    var major: Color?
    var noodle: Color?
    var idleGlassTint: Color?
    var selectedGlassTint: Color?
    var portGlassTint: Color?
    var titleText: Color?
    var detailText: Color?
}

private struct GraphLabSavedPalette: Codable {
    let graphBackground: GraphLabSavedColor?
    let minor: GraphLabSavedColor?
    let major: GraphLabSavedColor?
    let noodle: GraphLabSavedColor?
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
            idleGlassTint: idleGlassTint?.color,
            selectedGlassTint: selectedGlassTint?.color,
            portGlassTint: portGlassTint?.color,
            titleText: titleText?.color,
            detailText: detailText?.color
        )
    }
}

private struct GraphLabSavedColor: Codable {
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

private struct GraphLabNodeDragState {
    var translation = CGSize.zero
    var isActive = false
}

private struct GraphLabDraggableNode<Content: View>: View {
    let zoom: CGFloat
    let storedOffset: CGSize
    let position: CGPoint
    let constrainedOffset: (CGSize) -> CGSize
    let onBegan: () -> Void
    let onEnded: (CGSize) -> Void
    @ViewBuilder let content: (Bool) -> Content

    @GestureState private var dragState = GraphLabNodeDragState()

    var body: some View {
        let proposedOffset = CGSize(
            width: storedOffset.width + dragState.translation.width / zoom,
            height: storedOffset.height + dragState.translation.height / zoom
        )
        let allowedOffset = constrainedOffset(proposedOffset)
        let liveTranslation = CGSize(
            width: (allowedOffset.width - storedOffset.width) * zoom,
            height: (allowedOffset.height - storedOffset.height) * zoom
        )

        content(dragState.isActive)
            .scaleEffect(zoom)
            .position(position)
            .offset(liveTranslation)
            .transaction { transaction in
                transaction.animation = nil
            }
            .gesture(dragGesture)
    }

    private var dragGesture: some Gesture {
        DragGesture(minimumDistance: 0)
            .onChanged { _ in onBegan() }
            .updating($dragState) { value, state, _ in
                state.translation = value.translation
                state.isActive = true
            }
            .onEnded { value in
                let proposedOffset = CGSize(
                    width: storedOffset.width + value.translation.width / zoom,
                    height: storedOffset.height + value.translation.height / zoom
                )
                onEnded(constrainedOffset(proposedOffset))
            }
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
    let glassTreatment: PhotaraGraphGlassTreatment
    let titleColor: Color
    let detailColor: Color
    let glassTintColor: Color
    let glassTintOpacity: Double
    let portGlassTreatment: PhotaraGraphGlassTreatment
    let portGlassTintColor: Color
    let portGlassTintOpacity: Double
    let portCoreBrightness: Double

    private let width = 224.0
    private var rowCount: Int { max(1, max(inputs.count, outputs.count)) }
    private var height: CGFloat { 62 + CGFloat(rowCount) * 25 + 12 }

    var body: some View {
        PhotaraGraphNodeSurface(
            style: style,
            glassTreatment: glassTreatment,
            glassTint: glassTintColor.opacity(glassTintOpacity)
        ) {
            VStack(spacing: 0) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(title)
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(titleColor)
                    Text(subtitle)
                        .font(.caption)
                        .foregroundStyle(detailColor)
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
                ZStack {
                    GlassEffectContainer(spacing: 0) {
                        ForEach(0..<inputs.count, id: \.self) { index in
                            portShell(label: inputs[index], at: index)
                                .position(x: -style.portOffset, y: portY(index))
                        }
                        ForEach(0..<outputs.count, id: \.self) { index in
                            portShell(label: outputs[index], at: index)
                                .position(x: geometry.size.width + style.portOffset, y: portY(index))
                        }
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)

                    ForEach(0..<inputs.count, id: \.self) { index in
                        portCore(label: inputs[index], at: index)
                            .position(x: -style.portOffset, y: portY(index))
                    }
                    ForEach(0..<outputs.count, id: \.self) { index in
                        portCore(label: outputs[index], at: index)
                            .position(x: geometry.size.width + style.portOffset, y: portY(index))
                    }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .frame(width: width, height: height)
        .contentShape(RoundedRectangle(cornerRadius: style.cornerRadius, style: .continuous))
    }

    private func labelColumn(_ labels: [String], alignment: HorizontalAlignment) -> some View {
        VStack(alignment: alignment, spacing: 0) {
            ForEach(labels, id: \.self) { label in
                Text(label)
                    .font(.system(size: 10.5))
                    .foregroundStyle(detailColor)
                    .frame(height: 25)
            }
        }
    }

    private func portShell(label: String, at index: Int) -> some View {
        PhotaraGraphPort(
            shape: style.portShape,
            width: 14,
            height: 14,
            glassTreatment: portGlassTreatment,
            glassTint: portGlassTintColor.opacity(portGlassTintOpacity),
            coreColor: semanticPortColor(for: label),
            coreBrightness: portCoreBrightness,
            showsCore: false
        )
            .accessibilityLabel("\(label) port \(index + 1)")
    }

    private func portCore(label: String, at index: Int) -> some View {
        PhotaraGraphPortCore(
            shape: style.portShape,
            width: 14,
            height: 14,
            color: semanticPortColor(for: label),
            brightness: portCoreBrightness
        )
        .allowsHitTesting(false)
        .accessibilityHidden(true)
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
