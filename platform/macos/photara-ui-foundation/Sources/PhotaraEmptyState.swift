import Foundation
import SwiftUI

/// Portable, authorable empty-state content and geometry shared by feature modules.
struct PhotaraEmptyStatePreset: Codable, Equatable {
    var icon: String
    var title: String
    var message: String
    var actionTitle: String
    var iconSize: Double
    var titleSize: Double
    var messageSize: Double
    var spacing: Double
    var verticalOffset: Double
    var maximumTextWidth: Double

    var isValid: Bool {
        !icon.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && !title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && !message.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && (20...72).contains(iconSize)
            && (13...32).contains(titleSize)
            && (10...22).contains(messageSize)
            && (4...32).contains(spacing)
            && (-240...240).contains(verticalOffset)
            && (160...520).contains(maximumTextWidth)
    }
}

struct PhotaraEmptyStateView: View {
    let preset: PhotaraEmptyStatePreset
    var action: (() -> Void)? = nil
    @Environment(\.photaraTheme) private var theme

    var body: some View {
        VStack(spacing: preset.spacing) {
            Image(systemName: preset.icon)
                .font(.system(size: preset.iconSize, weight: .regular))
                .foregroundStyle(theme?.color(.textDisabled) ?? Color.secondary)
            Text(preset.title)
                .font(.system(size: preset.titleSize, weight: .semibold))
                .multilineTextAlignment(.center)
            Text(preset.message)
                .font(.system(size: preset.messageSize))
                .foregroundStyle(theme?.color(.textSecondary) ?? Color.secondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: preset.maximumTextWidth)
            if let action, !preset.actionTitle.isEmpty {
                Button(preset.actionTitle, action: action)
                    .buttonStyle(.borderedProminent)
                    .controlSize(.regular)
                    .padding(.top, 4)
            }
        }
        .padding(24)
        .offset(y: preset.verticalOffset)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
