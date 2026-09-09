import AppKit

/// Cached vector tool artwork shared by graph chrome and native cursors.
/// Loading belongs to the reusable presentation layer, never Graph Lab state.
enum PhotaraGraphToolIconStore {
    static let knife: NSImage = {
        guard let url = Bundle.main.url(forResource: "tool-knife", withExtension: "svg", subdirectory: "ToolIcons"),
              let image = NSImage(contentsOf: url) else {
            return NSImage(systemSymbolName: "scissors", accessibilityDescription: "Knife") ?? NSImage()
        }
        image.isTemplate = false
        image.accessibilityDescription = "Knife"
        return image
    }()
}
