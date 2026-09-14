import Foundation

/// Signed, isolated Data Protection Keychain acceptance. Uses random public
/// references and synthetic credential labels; no user item is enumerated.
@main
struct NativeOnboardingKeychainTests {
    static func main() throws {
        let configuration = ReleaseConfiguration.current
        let store = try NativeCredentialStore(bundleIdentifier: configuration.identity.bundleIdentifier)
        let installation = UUID(), subject = "synthetic-probe-" + UUID().uuidString
        let first = try NativeCredentialNamespace(configuration: configuration, installation: installation,
            issuer: configuration.environment.auth0Issuer.absoluteString, subject: subject, reference: NativeCredentialReference.create())
        let second = try NativeCredentialNamespace(configuration: configuration, installation: installation,
            issuer: configuration.environment.auth0Issuer.absoluteString, subject: subject, reference: NativeCredentialReference.create())
        defer {
            try? store.remove(first)
            try? store.remove(second)
        }
        let value = NativeStoredCredential(refreshToken: "synthetic-refresh", refreshInFlight: false,
            deviceCredential: try AuthenticationEncoding.random())
        try store.insert(value, namespace: first)
        try store.insert(value, namespace: second)
        let reopened = try NativeCredentialStore(bundleIdentifier: configuration.identity.bundleIdentifier)
        let before = try reopened.read(first)
        precondition(before?.refreshToken == "synthetic-refresh")
        let recovered = try reopened.recoveryNamespace(reference: first.reference!, configuration: configuration)
        precondition(recovered.account == first.account && recovered.service == first.service)
        let recoveredValue = try reopened.read(recovered)
        precondition(recoveredValue?.deviceCredential == value.deviceCredential)
        try reopened.removePrepared(reference: first.reference!, configuration: configuration)
        let removed = try reopened.read(first), retained = try reopened.read(second)
        precondition(removed == nil && retained?.refreshToken == "synthetic-refresh")
        var replacement = value
        replacement.refreshInFlight = true
        try reopened.replace(replacement, namespace: second)
        let updated = try reopened.read(second)
        precondition(updated?.refreshInFlight == true)
        replacement.refreshToken = nil
        replacement.refreshInFlight = false
        try reopened.replace(replacement, namespace: second)
        let signedOut = try reopened.read(second)
        precondition(signedOut?.refreshToken == nil && signedOut?.deviceCredential == value.deviceCredential)
        let signedOutNamespace = try reopened.recoveryNamespace(reference: second.reference!, configuration: configuration)
        precondition(signedOutNamespace.account == second.account)
        try reopened.removePrepared(reference: first.reference!, configuration: configuration)
        try reopened.removePrepared(reference: second.reference!, configuration: configuration)
        let final = try reopened.read(second)
        precondition(final == nil)
        print("native-onboarding-keychain: 8 signed Data Protection assertions passed; probe items removed")
    }
}
