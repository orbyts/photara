import SwiftUI

@main
struct GalleryLabApp: App {
    var body: some Scene { WindowGroup("Photara Gallery Lab") { GalleryLabView() } }
}

struct GalleryLabView: View {
    @State private var assets = GalleryFixtures.assets()
    @State private var filter = ""
    @State private var selected: String? = "asset-1"
    @State private var dark = true
    @State private var canAssign = true
    @State private var empty = false
    @State private var preset = GalleryPreset.shipped
    @State private var requested = Set<String>()
    @State private var action = "Select, open, assign, or View a fixture."

    var body: some View {
        LabAppearance(dark: dark) {
            HSplitView {
                Form {
                    Section("Preview") {
                        Toggle("Dark appearance", isOn: $dark)
                        Toggle("Compatible Layout target", isOn: $canAssign)
                        Toggle("Empty Gallery", isOn: $empty)
                        Text("HDR fixtures contain 4× reference-white highlights. Thumbnails use constrainedHigh; View opens the same image using high. Compare on an HDR display.")
                            .font(.caption)
                    }
                    Section("Shipped visual defaults") {
                        LabeledContent("Initial thumbnail size") { Slider(value: $preset.defaultThumbnailSize, in: 76...220) }
                        LabeledContent("Photo gap") { Slider(value: $preset.photoSpacing, in: 0...16) }
                        LabeledContent("Square row gap") { Slider(value: $preset.squareRowSpacing, in: 0...24) }
                        LabeledContent("Selection stroke") { Slider(value: $preset.selectionStrokeWidth, in: 0.5...4) }
                        Button("Restore Shipped") { preset = .shipped }
                        Button("Export Shipped Preset…") {
                            do { try LabPresetExport.save(preset.encoded(), filename: "photara-gallery-presentation-v1.json") }
                            catch { action = error.localizedDescription }
                        }
                    }
                    Section("Adapter events") {
                        Text(action).font(.caption.monospaced())
                        Text("\(requested.count) preview requests received").font(.caption)
                    }
                }.formStyle(.grouped).frame(minWidth: 280, idealWidth: 310, maxWidth: 360)
                AssetGalleryView(presentation: .init(assets: empty ? [] : assets, canAssign: canAssign),
                    actions: .init(open: { action = "open(\($0))" }, assign: { action = "assign(\($0))" },
                        requestPreview: { requested.insert($0) }), preset: preset, filter: $filter, selectedAssetID: $selected)
                    .frame(minWidth: 440)
            }
        }.frame(minWidth: 800, minHeight: 620)
    }
}
