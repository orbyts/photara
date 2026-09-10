import SwiftUI

/// Resolves package-neutral icon resources into this macOS client's skin.
private enum NativeNodeResources {
    static func symbol(for resourceID: String) -> String {
        switch resourceID {
        case "photara.layout.compose": "rectangle.3.group"
        case "photara.project.assets": "photo.stack"
        case "photara.disk.folder": "folder"
        default: "square.dashed"
        }
    }
}

struct NodeBrandIcon: View {
    @Environment(\.photaraTheme) private var theme

    let resourceID: String
    let themeColorRole: String?
    let accentHex: String?
    let size: CGFloat

    var body: some View {
        let role = themeColorRole.flatMap(PhotaraThemeRole.init(rawValue:))
        let accent = role.flatMap { theme?.color($0) }
            ?? Color(srgbHex: accentHex)
            ?? .accentColor
        Image(systemName: NativeNodeResources.symbol(for: resourceID))
            .font(.system(size: size * 0.56, weight: .medium))
            .foregroundStyle(accent)
            .frame(width: size, height: size)
            .background(accent.opacity(0.12), in: RoundedRectangle(cornerRadius: size * 0.22))
    }
}

private extension Color {
    init?(srgbHex: String?) {
        guard var value = srgbHex?.trimmingCharacters(in: .whitespacesAndNewlines),
              value.hasPrefix("#")
        else { return nil }
        value.removeFirst()
        guard value.count == 6, let rgb = UInt64(value, radix: 16) else { return nil }
        self.init(
            .sRGB,
            red: Double((rgb >> 16) & 0xff) / 255,
            green: Double((rgb >> 8) & 0xff) / 255,
            blue: Double(rgb & 0xff) / 255,
            opacity: 1
        )
    }
}
