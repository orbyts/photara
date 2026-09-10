import SwiftUI

@main
struct InspectorLabApp: App {
    var body: some Scene { WindowGroup("Photara Inspector Lab") { InspectorLabView() } }
}

struct InspectorLabView: View {
    @State private var fixture = InspectorFixture.disk
    @State private var presentation = InspectorFixture.disk.presentation
    @State private var section = InspectorSection.all
    @State private var dark = true
    @State private var action = "Actions are recorded by this fixture adapter."

    var body: some View {
        LabAppearance(dark: dark) {
            HSplitView {
                Form {
                    Picker("Fixture", selection: $fixture) {
                        ForEach(InspectorFixture.allCases) { Text($0.rawValue).tag($0) }
                    }
                    Picker("Section", selection: $section) {
                        ForEach(InspectorSection.allCases) { Text($0.rawValue).tag($0) }
                    }
                    Toggle("Dark appearance", isOn: $dark)
                    Toggle("Actions enabled", isOn: $presentation.actionsEnabled)
                    Picker("Evaluation", selection: $presentation.progressLabel) {
                        if !["Idle", "Running", "Completed", "Cancelled", "Failed", "Blocked"].contains(presentation.progressLabel) {
                            Text(presentation.progressLabel).tag(presentation.progressLabel)
                        }
                        ForEach(["Idle", "Running", "Completed", "Cancelled", "Failed", "Blocked"], id: \.self) { Text($0).tag($0) }
                    }
                    if let frames = presentation.node?.layout?.frames {
                        Picker("Frame", selection: $presentation.selectedFrameID) {
                            ForEach(frames, id: \.frameId) { Text("Frame \($0.index + 1)").tag(Optional($0.frameId)) }
                        }
                        if let cells = presentation.selectedFrame?.cells {
                            Picker("Cell", selection: $presentation.selectedCellID) {
                                ForEach(cells, id: \.cellId) { Text($0.cellId).tag(Optional($0.cellId)) }
                            }
                        }
                    }
                    Text("Drag the divider to inspect production behavior at narrow widths. Section spacing and colors come from shared code and theme roles.").font(.caption)
                    Text(action).font(.caption.monospaced())
                }.formStyle(.grouped).frame(minWidth: 260, idealWidth: 290, maxWidth: 340)
                InspectorView(presentation: presentation, actions: actions, section: section)
                    .frame(minWidth: 230, idealWidth: 360, maxWidth: .infinity)
            }
        }
        .frame(minWidth: 620, minHeight: 680)
        .onChange(of: fixture) { presentation = fixture.presentation }
    }

    private var actions: InspectorActions {
        .init(chooseFolder: { action = "chooseFolder(\($0))" },
            scanDisk: { action = "scanDisk(\($0))"; presentation.isScanning.toggle() },
            connectDisk: { action = "connectDisk(\($0))" },
            structure: { id, edit in action = "structure(\(id), \(edit))" },
            cell: { id, frame, cell, edit in
                action = "cell(\(id), \(frame), \(cell), \(edit))"
                guard let f = presentation.node?.layout?.frames.firstIndex(where: { $0.frameId == frame }),
                      let c = presentation.node?.layout?.frames[f].cells.firstIndex(where: { $0.cellId == cell }) else { return }
                switch edit {
                case .fit: presentation.node?.layout?.frames[f].cells[c].contentMode = .fit
                    presentation.node?.layout?.frames[f].cells[c].cropRect = nil
                case .fill: presentation.node?.layout?.frames[f].cells[c].contentMode = .fill
                    presentation.node?.layout?.frames[f].cells[c].cropRect = nil
                case .crop(let x, let y, let w, let h):
                    presentation.node?.layout?.frames[f].cells[c].contentMode = .crop
                    presentation.node?.layout?.frames[f].cells[c].cropRect = .init(x: x, y: y, width: w, height: h)
                case .setQuarterTurn(let turn): presentation.node?.layout?.frames[f].cells[c].quarterTurn = turn
                }
            })
    }
}
