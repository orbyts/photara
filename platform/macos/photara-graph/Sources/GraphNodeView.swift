import AppKit
import SwiftUI

struct PhotaraGraphNodeView: View {
    @Environment(\.photaraTheme) private var theme
    @Environment(\.photaraGraphPresentationZoom) private var zoom
    @Environment(\.colorScheme) private var colorScheme
    let node: PhotaraGraphNode
    let presentation: PhotaraGraphNodePresentationMetadata
    let selected: Bool
    let connectedInputs: Set<Int>
    let connectedOutputs: Set<Int>
    var preset = PhotaraGraphPresentationPreset.shipped

    private var inputs: [PhotaraGraphPortDefinition] { node.ports(.input) }
    private var outputs: [PhotaraGraphPortDefinition] { node.ports(.output) }
    private var palette: PhotaraGraphPalettePreset { preset.palette(colorScheme) }
    private var width: CGFloat { PhotaraGraphGeometry.nodeWidth * zoom }
    private var height: CGFloat { (74 + CGFloat(max(1, max(inputs.count, outputs.count))) * PhotaraGraphGeometry.rowHeight) * zoom }
    private var style: PhotaraGraphNodeStyle { .init(cornerRadius: preset.cornerRadius * zoom, portShape: .round,
        portOffset: preset.portOffset * zoom, shadowBlur: preset.nodeShadowBlur * zoom,
        shadowOpacity: colorScheme == .dark ? preset.darkNodeShadowOpacity : preset.lightNodeShadowOpacity,
        shadowOffsetY: preset.nodeShadowOffsetY * zoom) }

    var body: some View {
        PhotaraGraphNodeSurface(style: style,
            flatFill: (selected ? (palette.selectedNodeFill ?? palette.idleNodeFill) : palette.idleNodeFill)?.color
                ?? theme?.color(.graphNode),
            flatStroke: selected ? (palette.selectedNodeStroke?.color ?? theme?.color(.borderFocus))
                : (theme?.color(.borderStrong) ?? .secondary).opacity(0.5),
            flatStrokeWidth: (selected ? preset.selectedStrokeWidth : 0.5) * zoom) {
            VStack(spacing: 0) {
                HStack(spacing: 9 * zoom) {
                    PhotaraGraphNodeIcon(resource: presentation.iconResource,
                        color: presentation.iconColor, size: 28 * zoom)
                        .frame(width: 32 * zoom, height: 32 * zoom)
                    VStack(alignment: .leading, spacing: zoom) {
                        Text(node.title).font(.system(size: 13 * zoom, weight: .semibold))
                            .foregroundStyle(palette.titleText?.color ?? .primary)
                        Text(node.subtitle).font(.system(size: 11 * zoom))
                            .foregroundStyle(palette.detailText?.color ?? .secondary)
                    }
                }.frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, 14 * zoom).padding(.vertical, 10 * zoom)
                Divider()
                HStack(alignment: .top) {
                    labels(inputs, alignment: .leading)
                    Spacer(minLength: 16 * zoom)
                    labels(outputs, alignment: .trailing)
                }.padding(.horizontal, 14 * zoom).padding(.vertical, 8 * zoom)
            }.frame(width: width, height: height, alignment: .top)
                .clipShape(RoundedRectangle(cornerRadius: preset.cornerRadius * zoom, style: .continuous))
        } ports: {
            GeometryReader { geometry in
                ZStack {
                    GlassEffectContainer(spacing: 0) {
                        ForEach(Array(inputs.enumerated()), id: \.element.id) { index, port in
                            shell(port, connected: connectedInputs.contains(index)).position(x: 0, y: portY(index))
                        }
                        ForEach(Array(outputs.enumerated()), id: \.element.id) { index, port in
                            shell(port, connected: connectedOutputs.contains(index)).position(x: geometry.size.width, y: portY(index))
                        }
                    }
                    ForEach(Array(inputs.enumerated()), id: \.element.id) { index, port in
                        core(port, connected: connectedInputs.contains(index)).position(x: 0, y: portY(index))
                    }
                    ForEach(Array(outputs.enumerated()), id: \.element.id) { index, port in
                        core(port, connected: connectedOutputs.contains(index)).position(x: geometry.size.width, y: portY(index))
                    }
                }
            }
        }.frame(width: width, height: height).allowsHitTesting(false)
    }

    private func labels(_ ports: [PhotaraGraphPortDefinition], alignment: HorizontalAlignment) -> some View {
        VStack(alignment: alignment, spacing: 0) { ForEach(ports) { port in
            Text(port.label).font(.system(size: 10.5 * zoom))
                .foregroundStyle(palette.detailText?.color ?? .secondary)
                .frame(height: PhotaraGraphGeometry.rowHeight * zoom)
        }}
    }
    @ViewBuilder private func shell(_ port: PhotaraGraphPortDefinition, connected: Bool) -> some View {
        if connected {
            PhotaraGraphPort(shape: .round, width: 14 * zoom, height: 14 * zoom,
                glassTreatment: .clear, glassTint: (palette.portGlassTint?.color ?? .secondary).opacity(preset.portGlassTintOpacity),
                coreColor: portColor(port), showsCore: false)
                .shadow(color: portColor(port).opacity(preset.activePortShowsShadow ? preset.activePortShadowOpacity : 0),
                    radius: preset.activePortShadowBlur * zoom, y: preset.activePortShadowOffsetY * zoom)
        } else { Color.clear.frame(width: 14 * zoom, height: 14 * zoom) }
    }
    private func core(_ port: PhotaraGraphPortDefinition, connected: Bool) -> some View {
        Circle().fill(portColor(port)).saturation(connected ? 1 : preset.inactivePortSaturation)
            .brightness(connected ? (colorScheme == .dark ? preset.darkActivePortCoreBrightness : preset.lightActivePortCoreBrightness)
                : (colorScheme == .dark ? preset.darkPortCoreBrightness : preset.lightPortCoreBrightness))
            .overlay { if !connected && preset.inactivePortShowsStroke { Circle().stroke(portColor(port), lineWidth: preset.inactivePortStrokeWidth * zoom) } }
            .frame(width: preset.portCoreSize * zoom, height: preset.portCoreSize * zoom)
            .frame(width: 36 * zoom, height: 22 * zoom)
            .accessibilityLabel("\(port.label) \(port.direction.rawValue) port")
            .accessibilityValue(connected ? "Connected" : "Not connected")
    }
    private func portColor(_ port: PhotaraGraphPortDefinition) -> Color {
        let type = port.dataType.lowercased()
        if type.contains("control") { return theme?.color(.nodeAutomation) ?? .purple }
        if type.contains("mask") || type.contains("depth") { return theme?.color(.nodeCompute) ?? .orange }
        if type.contains("metadata") { return theme?.color(.nodeIntegration) ?? .green }
        if type.contains("preview") { return theme?.color(.nodeCreative) ?? .pink }
        return theme?.color(.nodeIO) ?? .blue
    }
    private func portY(_ index: Int) -> CGFloat { (PhotaraGraphGeometry.firstPortY + CGFloat(index) * PhotaraGraphGeometry.rowHeight) * zoom }
}
