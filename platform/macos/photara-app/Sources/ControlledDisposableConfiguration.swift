import Foundation

/// Presentation input checks only. Rust must independently validate controller
/// provenance, mounted identity, registration and exclusive ownership.
struct ControlledDisposableConfiguration: Sendable {
    let manifestPath: String
    let bindingJSON: String

    static func load(_ environment: [String: String]) throws -> Self {
        guard let manifest = environment["PHOTARA_PS2_REMOUNT_MANIFEST"],
              manifest.utf8.count <= 4096,
              manifest.hasPrefix("/private/tmp/photara-ps2-remount-"),
              manifest.hasSuffix("/manifest.json"),
              !manifest.contains("/../"), !manifest.contains("/./"),
              let binding = environment["PHOTARA_PS2_REMOUNT_BINDING"],
              !binding.isEmpty, binding.utf8.count <= 65_536,
              let data = binding.data(using: .utf8),
              (try? JSONSerialization.jsonObject(with: data)) is [String: Any]
        else { throw ConfigurationError.missingOrInvalid }
        return .init(manifestPath: manifest, bindingJSON: binding)
    }

    enum ConfigurationError: LocalizedError {
        case missingOrInvalid
        var errorDescription: String? {
            "This build opens only a registered disposable project. Launch it through the controlled image launcher."
        }
    }
}
