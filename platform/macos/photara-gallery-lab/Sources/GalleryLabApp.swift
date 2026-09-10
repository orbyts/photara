import SwiftUI

@main
struct GalleryLabApp: App {
    var body: some Scene { WindowGroup("Photara Gallery Lab") { GalleryLabView() } }
}

struct GalleryLabView: View {
    private enum Scenario: String, CaseIterable, Identifiable {
        case populated = "Populated"
        case noSource = "No source nodes"
        case awaiting = "Source awaiting results"
        case noMatches = "No filter matches"
        var id: String { rawValue }
    }
    private static let draftKey = "photara.gallery-lab.authoring-draft.v1"
    @State private var assets = GalleryFixtures.assets()
    @State private var filter = ""
    @State private var selected: String? = "asset-1"
    @State private var dark = true
    @State private var canAssign = true
    @State private var scenario = Scenario.populated
    @State private var preset: GalleryPreset
    @State private var requested = Set<String>()
    @State private var action = "Select, open, assign, or View a fixture."

    init() {
        let saved = UserDefaults.standard.data(forKey: Self.draftKey)
            .flatMap { try? GalleryPreset.decode($0) }
        _preset = State(initialValue: saved ?? .shipped)
    }

    var body: some View {
        LabAppearance(dark: dark) {
            HSplitView {
                Form {
                    Section("Preview") {
                        Toggle("Dark appearance", isOn: $dark)
                        Toggle("Compatible Layout target", isOn: $canAssign)
                        Picker("Scenario", selection: $scenario) {
                            ForEach(Scenario.allCases) { Text($0.rawValue).tag($0) }
                        }
                        Text("HDR fixtures contain 4× reference-white highlights. Thumbnails use constrainedHigh; View opens the same image using high. Compare on an HDR display.")
                            .font(.caption)
                    }
                    Section("Shipped visual defaults") {
                        LabeledContent("Initial thumbnail size") { Slider(value: $preset.defaultThumbnailSize, in: 76...220) }
                        LabeledContent("Photo gap") { Slider(value: $preset.photoSpacing, in: 0...16) }
                        LabeledContent("Square row gap") { Slider(value: $preset.squareRowSpacing, in: 0...24) }
                        LabeledContent("Selection stroke") { Slider(value: $preset.selectionStrokeWidth, in: 0.5...4) }
                    }
                    if scenario != .populated {
                        Section("Empty state") { emptyStateEditor }
                    }
                    Section("Handoff") {
                        Button("Apply to Photara") { applyToPhotara() }
                        Button("Remove Photara Override") { removeOverride() }
                        Button("Export Gallery Preset…") {
                            do { try LabPresetExport.save(preset.encoded(), filename: "photara-gallery-presentation-v1.json") }
                            catch { action = error.localizedDescription }
                        }
                        Button("Restore Shipped") { preset = .shipped }
                        Text("Lab edits are saved automatically as a local draft.").font(.caption).foregroundStyle(.secondary)
                    }
                    Section("Adapter events") {
                        Text(action).font(.caption.monospaced())
                        Text("\(requested.count) preview requests received").font(.caption)
                    }
                }.formStyle(.grouped).frame(minWidth: 280, idealWidth: 310, maxWidth: 360)
                AssetGalleryView(presentation: galleryPresentation,
                    actions: .init(open: { action = "open(\($0))" }, assign: { action = "assign(\($0))" },
                        requestPreview: { requested.insert($0) },
                        addSourceNode: { action = "addSourceNode()" },
                        runWorkflow: { action = "runWorkflow()" },
                        clearFilter: { action = "clearFilter()" }),
                    preset: preset, filter: $filter, selectedAssetID: $selected)
                    .frame(minWidth: 440)
            }
        }.frame(minWidth: 800, minHeight: 620)
        .onChange(of: scenario) {
            filter = scenario == .noMatches ? "No fixture matches this" : ""
        }
        .onChange(of: preset) {
            if let data = try? preset.encoded() { UserDefaults.standard.set(data, forKey: Self.draftKey) }
        }
    }

    private var galleryPresentation: GalleryPresentation {
        .init(assets: scenario == .populated || scenario == .noMatches ? assets : [],
              canAssign: canAssign, hasSourceNodes: scenario == .awaiting)
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
            switch scenario {
            case .populated, .noSource: preset.noSourceState
            case .awaiting: preset.awaitingAssetsState
            case .noMatches: preset.noMatchesState
            }
        }, set: { value in
            switch scenario {
            case .populated, .noSource: preset.noSourceState = value
            case .awaiting: preset.awaitingAssetsState = value
            case .noMatches: preset.noMatchesState = value
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
        do { try PhotaraGalleryDevelopmentSettings.setOverride(preset); action = "Applied to Photara." }
        catch { action = error.localizedDescription }
    }
    private func removeOverride() {
        do { try PhotaraGalleryDevelopmentSettings.setOverride(nil); action = "Removed Photara Gallery override." }
        catch { action = error.localizedDescription }
    }
}
