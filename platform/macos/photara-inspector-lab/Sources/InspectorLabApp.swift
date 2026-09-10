import SwiftUI

@main
struct InspectorLabApp: App {
    var body: some Scene { WindowGroup("Photara Inspector Lab") { InspectorLabView() } }
}

struct InspectorLabView: View {
    private static let draftKey = "photara.inspector-lab.authoring-draft.v1"
    @State private var fixture = InspectorFixture.disk
    @State private var presentation = InspectorFixture.disk.presentation
    @State private var section = InspectorSection.all
    @State private var dark = true
    @State private var action = "Actions are recorded by this fixture adapter."
    @State private var preset: InspectorPreset

    init() {
        let saved = UserDefaults.standard.data(forKey: Self.draftKey)
            .flatMap { try? InspectorPreset.decode($0) }
        _preset = State(initialValue: saved ?? .shipped)
    }

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
                    if [.noSelection, .graphHidden, .noSettings].contains(fixture) {
                        Section("Empty state") { emptyStateEditor }
                    }
                    Section("Handoff") {
                        Button("Apply to Photara") { applyToPhotara() }
                        Button("Remove Photara Override") { removeOverride() }
                        Button("Export Inspector Preset…") {
                            do { try LabPresetExport.save(preset.encoded(), filename: "photara-inspector-presentation-v1.json") }
                            catch { action = error.localizedDescription }
                        }
                        Button("Restore Shipped") { preset = .shipped }
                        Text("Lab edits are saved automatically as a local draft.").font(.caption).foregroundStyle(.secondary)
                    }
                    Text(action).font(.caption.monospaced())
                }.formStyle(.grouped).frame(minWidth: 260, idealWidth: 290, maxWidth: 340)
                InspectorView(presentation: presentation, actions: actions, section: section, preset: preset)
                    .frame(minWidth: 230, idealWidth: 360, maxWidth: .infinity)
            }
        }
        .frame(minWidth: 620, minHeight: 680)
        .onChange(of: fixture) { presentation = fixture.presentation }
        .onChange(of: preset) {
            if let data = try? preset.encoded() { UserDefaults.standard.set(data, forKey: Self.draftKey) }
        }
    }

    @ViewBuilder private var emptyStateEditor: some View {
        let state = emptyStateBinding
        TextField("SF Symbol", text: state.icon)
        TextField("Title", text: state.title)
        TextField("Description", text: state.message, axis: .vertical)
        TextField("Action title", text: state.actionTitle)
        valueSlider("Icon size", state.iconSize, 20...72)
        valueSlider("Title size", state.titleSize, 13...32)
        valueSlider("Description size", state.messageSize, 10...22)
        valueSlider("Spacing", state.spacing, 4...32)
        valueSlider("Vertical position", state.verticalOffset, -240...240)
        valueSlider("Maximum text width", state.maximumTextWidth, 160...520)
    }

    private var emptyStateBinding: Binding<PhotaraEmptyStatePreset> {
        Binding(get: {
            switch fixture {
            case .graphHidden: preset.graphHiddenState
            case .noSettings: preset.noSettingsState
            default: preset.noSelectionState
            }
        }, set: { value in
            switch fixture {
            case .graphHidden: preset.graphHiddenState = value
            case .noSettings: preset.noSettingsState = value
            default: preset.noSelectionState = value
            }
        })
    }

    private func valueSlider(_ title: String, _ value: Binding<Double>, _ range: ClosedRange<Double>) -> some View {
        VStack(alignment: .leading) {
            Text("\(title) · \(Int(value.wrappedValue))").font(.caption)
            Slider(value: value, in: range, step: 1)
        }
    }

    private func applyToPhotara() {
        do { try PhotaraInspectorDevelopmentSettings.setOverride(preset); action = "Applied to Photara." }
        catch { action = error.localizedDescription }
    }
    private func removeOverride() {
        do { try PhotaraInspectorDevelopmentSettings.setOverride(nil); action = "Removed Photara Inspector override." }
        catch { action = error.localizedDescription }
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
            }, showGraph: { action = "showGraph()" })
    }
}
