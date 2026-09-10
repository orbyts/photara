import SwiftUI


struct InspectorPortView: View {
    let port: PortInspection

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            HStack {
                Circle()
                    .fill(Color.accentColor)
                    .frame(width: 7, height: 7)
                Text(port.portId.capitalized).font(.headline)
                Spacer()
                Text(port.valueTypeId)
                    .font(.caption.monospaced())
                    .foregroundStyle(.secondary)
            }
            if let connected = port.connectedNodeName {
                LabeledContent(
                    port.direction == .input ? "Source" : "Consumer",
                    value: connected
                )
            } else {
                LabeledContent("Connection", value: "Unconnected")
                    .foregroundStyle(.secondary)
            }
            ForEach(Array(port.summary.enumerated()), id: \.offset) { _, field in
                LabeledContent(field.label, value: field.value)
            }
        }
        .padding(.vertical, 3)
    }
}

enum LayoutArrangementChoice: Hashable {
    case one
    case horizontal
    case vertical
    case grid
    case custom

    init(_ value: LayoutArrangement) {
        switch value {
        case .one: self = .one
        case .horizontalStack: self = .horizontal
        case .verticalStack: self = .vertical
        case .uniformGrid: self = .grid
        case .custom: self = .custom
        }
    }

    var action: LayoutArrangementAction {
        switch self {
        case .one: .one
        case .horizontal: .horizontalStack
        case .vertical: .verticalStack
        case .grid: .uniformGrid(columns: 2)
        case .custom: .custom
        }
    }
}

enum LayoutContentChoice: Hashable {
    case fit
    case fill
    case crop

    init(_ value: LayoutContentMode) {
        switch value {
        case .fit: self = .fit
        case .fill: self = .fill
        case .crop: self = .crop
        }
    }
}

func nextQuarterTurn(after value: LayoutQuarterTurn) -> LayoutQuarterTurn {
    switch value {
    case .zero: .clockwise90
    case .clockwise90: .clockwise180
    case .clockwise180: .clockwise270
    case .clockwise270: .zero
    }
}

struct AlignmentPad: View {
    let select: (UInt32, UInt32) -> Void

    private let values: [UInt32] = [0, 500_000, 1_000_000]

    var body: some View {
        Grid(horizontalSpacing: 3, verticalSpacing: 3) {
            ForEach(values, id: \.self) { y in
                GridRow {
                    ForEach(values, id: \.self) { x in
                        Button {
                            select(x, y)
                        } label: {
                            Circle().frame(width: 5, height: 5)
                        }
                        .buttonStyle(.bordered)
                        .controlSize(.mini)
                    }
                }
            }
        }
    }
}
