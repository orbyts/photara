import SwiftUI

/// The same semantic specimen is used by Theme Lab and native raster verification.
/// It illustrates a local composition root; it never computes a fourth surface.
struct ThemeLadderSpecimen: View {
  let document: PhotaraThemeDocument

  var body: some View {
    HStack(alignment: .top, spacing: 0) {
      ForEach(PhotaraThemeAppearance.allCases) { appearance in
        specimen(appearance)
          .environment(\.colorScheme, appearance == .dark ? .dark : .light)
      }
    }
    .accessibilityIdentifier("theme-ladder-specimen")
  }

  private func specimen(_ appearance: PhotaraThemeAppearance) -> some View {
    let palette = document.resolved(for: appearance)
    return VStack(alignment: .leading, spacing: 18) {
      Text(appearance.rawValue.capitalized).font(.title2.bold())
      label(.foundation, palette: palette)
      VStack(alignment: .leading, spacing: 18) {
        label(.primary, palette: palette)
        VStack(alignment: .leading, spacing: 12) {
          label(.inset, palette: palette)
          Text("A nested work area").font(.callout)
            .foregroundStyle(palette.color(.textSecondary))
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(18)
        .background(palette.surface(.inset), in: RoundedRectangle(cornerRadius: 10))
      }
      .padding(18)
      .background(palette.surface(.primary), in: RoundedRectangle(cornerRadius: 14))
      HStack(spacing: 10) {
        RoundedRectangle(cornerRadius: 4)
          .fill(palette.color(.editorSurround)).frame(width: 40, height: 40)
        VStack(alignment: .leading, spacing: 3) {
          Text("Photograph reference").font(.caption.bold())
          Text(palette.rgba(.editorSurround)?.hex ?? "")
            .font(.caption.monospaced()).foregroundStyle(palette.color(.textSecondary))
        }
      }
      Text("New composition root → Foundation")
        .font(.caption).foregroundStyle(palette.color(.textSecondary))
    }
    .foregroundStyle(palette.color(.textPrimary))
    .padding(24)
    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    .background(palette.surface(.foundation))
  }

  private func label(_ level: PhotaraSurfaceLevel, palette: PhotaraResolvedTheme) -> some View {
    HStack {
      Text(level.rawValue).font(.headline)
      Spacer(minLength: 12)
      Text(palette.rgba(level.role)?.hex ?? "")
        .font(.caption.monospaced()).foregroundStyle(palette.color(.textSecondary))
    }
  }
}
