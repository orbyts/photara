import Foundation
import CryptoKit
import Security
import LocalAuthentication
import AuthenticationServices
import AppKit

/// Fixed diagnostics only: never attach provider errors, callback URLs or tokens.
enum NativeAuthenticationError: Error, Equatable {
    case signingRequired, keychainUnavailable, invalidCredential, randomUnavailable
    case invalidCallback, expiredAttempt, consumedAttempt, cancelled, browserUnavailable
    case invalidResponse, transportUnavailable, staleSession
}

enum AuthenticationEncoding {
    static func base64URL(_ data: Data) -> String {
        data.base64EncodedString().replacingOccurrences(of: "+", with: "-")
            .replacingOccurrences(of: "/", with: "_").replacingOccurrences(of: "=", with: "")
    }

    static func random() throws -> String {
        var bytes = [UInt8](repeating: 0, count: 32)
        guard SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes) == errSecSuccess
        else { throw NativeAuthenticationError.randomUnavailable }
        return base64URL(Data(bytes))
    }

    static func decodeBase64URL(_ value: String) throws -> Data {
        guard !value.isEmpty,
              value.utf8.allSatisfy({ (48...57).contains($0) || (65...90).contains($0) || (97...122).contains($0) || $0 == 45 || $0 == 95 })
        else { throw NativeAuthenticationError.invalidCredential }
        let base64 = value.replacingOccurrences(of: "-", with: "+")
            .replacingOccurrences(of: "_", with: "/")
        guard let data = Data(base64Encoded: base64 + String(repeating: "=", count: (4 - base64.count % 4) % 4)),
              base64URL(data) == value
        else { throw NativeAuthenticationError.invalidCredential }
        return data
    }

    static func equal(_ lhs: String, _ rhs: String) -> Bool {
        let a = Array(lhs.utf8), b = Array(rhs.utf8)
        guard a.count == b.count else { return false }
        var difference: UInt8 = 0
        for index in a.indices { difference |= a[index] ^ b[index] }
        return difference == 0
    }

    static func sha256(_ data: Data) -> String {
        SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
    }
}

/// Opaque durable-coordinate format accepted by the Rust onboarding journal.
/// The value identifies a Keychain namespace; it never contains a credential.
enum NativeCredentialReference {
    static func create() -> String {
        "keychain:\(UUID().uuidString.lowercased())"
    }

    static func validate(_ value: String) throws {
        guard value.hasPrefix("keychain:"),
              let id = UUID(uuidString: String(value.dropFirst(9))),
              id.uuidString != "00000000-0000-0000-0000-000000000000"
        else { throw NativeAuthenticationError.invalidCredential }
    }
}

/// One-use in-memory attempt; this type is intentionally neither Codable nor
/// CustomStringConvertible. Native code consumes it before any token exchange.
struct NativePKCEAttempt {
    let state: String
    let verifier: String
    let nonce: String
    let generation: UInt64
    let startedAt: Date
    let deadline: Date
    let callbackURL: URL
    private(set) var consumed = false

    init(callbackURL: URL, nonce: String, generation: UInt64, now: Date = Date()) throws {
        guard nonce.utf8.count == 43,
              nonce.utf8.allSatisfy({ (48...57).contains($0) || (65...90).contains($0) || (97...122).contains($0) || $0 == 45 || $0 == 95 })
        else { throw NativeAuthenticationError.invalidResponse }
        self.callbackURL = callbackURL
        self.nonce = nonce
        self.generation = generation
        state = try AuthenticationEncoding.random()
        verifier = try AuthenticationEncoding.random()
        startedAt = now
        deadline = now.addingTimeInterval(300)
    }

    var challenge: String { AuthenticationEncoding.base64URL(Data(SHA256.hash(data: Data(verifier.utf8)))) }

    func authorizationURL(environment: ReleaseEnvironment, switchingAccount: Bool) throws -> URL {
        guard environment.callbackURL == callbackURL,
              var url = URLComponents(url: environment.auth0Issuer.appending(path: "authorize"), resolvingAgainstBaseURL: false)
        else { throw NativeAuthenticationError.invalidResponse }
        url.queryItems = [
            URLQueryItem(name: "response_type", value: "code"),
            URLQueryItem(name: "client_id", value: environment.nativeClientId),
            URLQueryItem(name: "redirect_uri", value: callbackURL.absoluteString),
            URLQueryItem(name: "audience", value: environment.auth0Audience),
            URLQueryItem(name: "scope", value: "openid profile email offline_access photara:onboard"),
            URLQueryItem(name: "connection", value: "google-oauth2"),
            URLQueryItem(name: "code_challenge_method", value: "S256"),
            URLQueryItem(name: "code_challenge", value: challenge),
            URLQueryItem(name: "state", value: state),
            URLQueryItem(name: "nonce", value: nonce),
            URLQueryItem(name: "max_age", value: "0")
        ]
        if switchingAccount { url.queryItems?.append(URLQueryItem(name: "prompt", value: "login")) }
        guard let result = url.url else { throw NativeAuthenticationError.invalidResponse }
        return result
    }

    mutating func consume(_ callback: URL, generation current: UInt64, now: Date = Date()) throws -> String {
        guard !consumed else { throw NativeAuthenticationError.consumedAttempt }
        // Even a malformed callback burns this attempt. It cannot be repaired by
        // a second unsolicited delivery while another callback is exchanging.
        consumed = true
        guard current == generation else { throw NativeAuthenticationError.staleSession }
        guard now >= startedAt, now < deadline else { throw NativeAuthenticationError.expiredAttempt }
        guard callback.absoluteString.utf8.count <= 16_384,
              let actual = URLComponents(url: callback, resolvingAgainstBaseURL: false),
              let expected = URLComponents(url: callbackURL, resolvingAgainstBaseURL: false),
              actual.scheme == expected.scheme, actual.host == expected.host,
              actual.percentEncodedPath == expected.percentEncodedPath,
              actual.port == nil, actual.user == nil, actual.password == nil,
              actual.fragment == nil, let items = actual.queryItems,
              Set(items.map(\.name)).count == items.count,
              items.allSatisfy({ ["state", "code", "error", "error_description", "error_uri"].contains($0.name) }),
              let receivedState = items.first(where: { $0.name == "state" })?.value,
              AuthenticationEncoding.equal(receivedState, state)
        else { throw NativeAuthenticationError.invalidCallback }
        let code = items.first(where: { $0.name == "code" })?.value
        let error = items.first(where: { $0.name == "error" })?.value
        guard (code != nil) != (error != nil) else { throw NativeAuthenticationError.invalidCallback }
        guard let code else { throw NativeAuthenticationError.cancelled }
        guard !code.isEmpty, code.utf8.count <= 4_096,
              !code.unicodeScalars.contains(where: { CharacterSet.controlCharacters.contains($0) }),
              !items.contains(where: { $0.name == "error_description" || $0.name == "error_uri" })
        else { throw NativeAuthenticationError.invalidCallback }
        return code
    }

    mutating func cancel() { consumed = true }
}

/// Each credential namespace is installation/principal/environment specific.
/// Public coordinates are hashed only to avoid identifiers in Keychain labels;
/// that hash never establishes authority.
struct NativeCredentialNamespace {
    let service: String
    let account: String
    let reference: String?

    /// Used only with the exact attributes returned by a private Keychain
    /// reference query. Match against the verified identity before journal use.
    init(recoveryAccount: String, reference: String, configuration: ReleaseConfiguration) throws {
        try NativeCredentialReference.validate(reference)
        guard recoveryAccount.utf8.count == 64,
              recoveryAccount.utf8.allSatisfy({ (48...57).contains($0) || (97...102).contains($0) })
        else { throw NativeAuthenticationError.invalidCredential }
        service = configuration.identity.keychainService + "." + configuration.environment.environmentId
        account = recoveryAccount
        self.reference = reference
    }

    init(configuration: ReleaseConfiguration, installation: UUID, issuer: String, subject: String, reference: String? = nil) throws {
        guard issuer == configuration.environment.auth0Issuer.absoluteString,
              !subject.isEmpty, subject.utf8.count <= 255,
              installation != UUID(uuid: (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0))
        else { throw NativeAuthenticationError.invalidCredential }
        service = configuration.identity.keychainService + "." + configuration.environment.environmentId
        if let reference {
            try NativeCredentialReference.validate(reference)
        }
        self.reference = reference
        // Length-delimited JSON array avoids concatenation collisions.
        var coordinates = [configuration.environment.nativeClientId, issuer, subject, installation.uuidString]
        if let reference { coordinates.append(reference) }
        let bytes = try JSONEncoder().encode(coordinates)
        account = AuthenticationEncoding.sha256(bytes)
    }
}

/// Access tokens and ID tokens are deliberately absent. A pending rotating
/// refresh token cannot be retried after an ambiguous result or process crash.
struct NativeStoredCredential: Codable {
    var refreshToken: String?
    var refreshInFlight: Bool
    let deviceCredential: String

    func validate() throws {
        guard deviceCredential.utf8.count == 43,
              deviceCredential.utf8.allSatisfy({ (48...57).contains($0) || (65...90).contains($0) || (97...122).contains($0) || $0 == 45 || $0 == 95 }),
              refreshToken.map({ !$0.isEmpty && $0.utf8.count <= 16_384 }) ?? true
        else { throw NativeAuthenticationError.invalidCredential }
    }
}

/// No default/file-Keychain fallback. Entitlements are checked before browser
/// or network dispatch; SecItem additionally enforces the provisioning profile.
struct NativeCredentialStore {
    let accessGroup: String

    init(bundleIdentifier: String) throws {
        let identifier = try Self.declaredAccessGroup(bundleIdentifier: bundleIdentifier)
        accessGroup = identifier
        let probe: [String: Any] = [kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: "com.photara.signing-probe." + UUID().uuidString,
            kSecAttrAccessGroup as String: identifier,
            kSecUseDataProtectionKeychain as String: true,
            kSecAttrSynchronizable as String: false]
        // Let securityd validate the entitlement, rather than trusting a string
        // read from a forged/ad-hoc signature. This creates no Keychain item.
        guard SecItemCopyMatching(probe as CFDictionary, nil) == errSecItemNotFound
        else { throw NativeAuthenticationError.signingRequired }
    }

    /// Metadata only: opening a local Library must not contact Keychain. This
    /// may explain unavailable UI, but cannot authorize credential operations.
    static func declaredAccessGroup(bundleIdentifier: String) throws -> String {
        guard let task = SecTaskCreateFromSelf(nil),
              let team = SecTaskCopyValueForEntitlement(task, "com.apple.developer.team-identifier" as CFString, nil) as? String,
              let identifier = SecTaskCopyValueForEntitlement(task, "com.apple.application-identifier" as CFString, nil) as? String,
              let groups = SecTaskCopyValueForEntitlement(task, "keychain-access-groups" as CFString, nil) as? [String],
              !team.isEmpty, identifier == team + "." + bundleIdentifier, groups == [identifier]
        else { throw NativeAuthenticationError.signingRequired }
        return identifier
    }

    private func query(_ namespace: NativeCredentialNamespace) -> [String: Any] {
        let context = LAContext()
        context.interactionNotAllowed = true
        return [kSecClass as String: kSecClassGenericPassword,
                kSecAttrService as String: namespace.service,
                kSecAttrAccount as String: namespace.account,
                kSecAttrAccessGroup as String: accessGroup,
                kSecUseDataProtectionKeychain as String: true,
                kSecAttrSynchronizable as String: false,
                kSecUseAuthenticationContext as String: context]
    }

    func read(_ namespace: NativeCredentialNamespace) throws -> NativeStoredCredential? {
        var request = query(namespace)
        request[kSecReturnData as String] = true
        request[kSecMatchLimit as String] = kSecMatchLimitOne
        var result: CFTypeRef?
        let status = SecItemCopyMatching(request as CFDictionary, &result)
        if status == errSecItemNotFound { return nil }
        guard status == errSecSuccess, let data = result as? Data, data.count <= 32_768
        else { throw NativeAuthenticationError.keychainUnavailable }
        let value = try JSONDecoder().decode(NativeStoredCredential.self, from: data)
        try value.validate()
        return value
    }

    func recoveryNamespace(reference: String, configuration: ReleaseConfiguration) throws -> NativeCredentialNamespace {
        try NativeCredentialReference.validate(reference)
        let context = LAContext()
        context.interactionNotAllowed = true
        let request: [String: Any] = [kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: configuration.identity.keychainService + "." + configuration.environment.environmentId,
            kSecAttrGeneric as String: Data(reference.utf8), kSecAttrAccessGroup as String: accessGroup,
            kSecUseDataProtectionKeychain as String: true, kSecAttrSynchronizable as String: false,
            kSecUseAuthenticationContext as String: context, kSecReturnAttributes as String: true,
            kSecMatchLimit as String: kSecMatchLimitAll]
        var result: CFTypeRef?
        guard SecItemCopyMatching(request as CFDictionary, &result) == errSecSuccess,
              let matches = result as? [[String: Any]], matches.count == 1,
              let account = matches[0][kSecAttrAccount as String] as? String
        else { throw NativeAuthenticationError.keychainUnavailable }
        return try NativeCredentialNamespace(recoveryAccount: account, reference: reference, configuration: configuration)
    }

    func insert(_ value: NativeStoredCredential, namespace: NativeCredentialNamespace) throws {
        try value.validate()
        var request = query(namespace)
        request[kSecValueData as String] = try JSONEncoder().encode(value)
        request[kSecAttrAccessible as String] = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        if let reference = namespace.reference {
            request[kSecAttrGeneric as String] = Data(reference.utf8)
        }
        guard SecItemAdd(request as CFDictionary, nil) == errSecSuccess
        else { throw NativeAuthenticationError.keychainUnavailable }
    }

    /// A single SecItemUpdate atomically replaces the refresh marker and token.
    /// Caller must hold the installation's cross-process lock across refresh.
    func replace(_ value: NativeStoredCredential, namespace: NativeCredentialNamespace) throws {
        try value.validate()
        let changes: [String: Any] = [kSecValueData as String: try JSONEncoder().encode(value)]
        guard SecItemUpdate(query(namespace) as CFDictionary, changes as CFDictionary) == errSecSuccess
        else { throw NativeAuthenticationError.keychainUnavailable }
    }

    /// Removes only this installation/principal/environment credential. This
    /// is used for pre-dispatch rollback and explicit logout; absence is safe.
    func remove(_ namespace: NativeCredentialNamespace) throws {
        let status = SecItemDelete(query(namespace) as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound
        else { throw NativeAuthenticationError.keychainUnavailable }
    }

    /// A prepared journal has no principal yet. Its unique public reference
    /// tags only that attempt's newly inserted item, permitting crash cleanup
    /// without enumerating accounts or deleting another session's credential.
    func removePrepared(reference: String, configuration: ReleaseConfiguration) throws {
        try NativeCredentialReference.validate(reference)
        let context = LAContext()
        context.interactionNotAllowed = true
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: configuration.identity.keychainService + "." + configuration.environment.environmentId,
            kSecAttrGeneric as String: Data(reference.utf8),
            kSecAttrAccessGroup as String: accessGroup,
            kSecUseDataProtectionKeychain as String: true,
            kSecAttrSynchronizable as String: false,
            kSecUseAuthenticationContext as String: context
        ]
        let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound
        else { throw NativeAuthenticationError.keychainUnavailable }
    }
}

/// AuthenticationServices is permitted to invoke its completion on an XPC
/// callback queue. Manufacture that callback outside the main actor, then make
/// the actor hop explicit before touching UI-owned state.
enum NativeAuthenticationCallback {
    nonisolated static func handler(
        deliver: @escaping @MainActor @Sendable (URL?, Error?) -> Void
    ) -> @Sendable (URL?, Error?) -> Void {
        { callback, error in
            Task { @MainActor in deliver(callback, error) }
        }
    }
}

/// Owns exactly one system browser session. The caller receives only a callback
/// and must consume it with its attempt before beginning token exchange.
@MainActor
final class NativeAuthenticationBrowser: NSObject, ASWebAuthenticationPresentationContextProviding {
    typealias SessionFactory = @MainActor (URL, String, Bool, any ASWebAuthenticationPresentationContextProviding, @escaping @Sendable (URL?, Error?) -> Void) -> any NativeAuthenticationSession
    private let makeSession: SessionFactory
    private var session: (any NativeAuthenticationSession)?
    private weak var window: NSWindow?
    private var continuation: CheckedContinuation<URL, Error>?
    private var attemptID: UUID?

    init(makeSession: @escaping SessionFactory = { url, scheme, ephemeral, anchor, completion in
        let session = ASWebAuthenticationSession(url: url, callback: .customScheme(scheme), completionHandler: completion)
        session.presentationContextProvider = anchor
        session.prefersEphemeralWebBrowserSession = ephemeral
        return session
    }) {
        self.makeSession = makeSession
        super.init()
    }

    func presentationAnchor(for session: ASWebAuthenticationSession) -> ASPresentationAnchor { window ?? NSWindow() }

    func authenticate(url: URL, callbackScheme: String, window: NSWindow?, ephemeral: Bool) async throws -> URL {
        guard session == nil else { throw NativeAuthenticationError.browserUnavailable }
        try Task.checkCancellation()
        let identifier = UUID()
        attemptID = identifier
        self.window = window
        return try await withTaskCancellationHandler {
            try await withCheckedThrowingContinuation { continuation in
                self.continuation = continuation
                let completion = NativeAuthenticationCallback.handler { [weak self] callback, error in
                    guard let self, self.attemptID == identifier, let continuation = self.continuation else { return }
                    self.attemptID = nil
                    self.continuation = nil
                    self.session = nil
                    if let callback, error == nil { continuation.resume(returning: callback) }
                    else if let error = error as? ASWebAuthenticationSessionError, error.code == .canceledLogin {
                        continuation.resume(throwing: NativeAuthenticationError.cancelled)
                    } else { continuation.resume(throwing: NativeAuthenticationError.browserUnavailable) }
                }
                let session = makeSession(url, callbackScheme, ephemeral, self, completion)
                self.session = session
                if !session.start() {
                    self.attemptID = nil
                    self.session = nil
                    self.continuation = nil
                    continuation.resume(throwing: NativeAuthenticationError.browserUnavailable)
                }
            }
        } onCancel: { Task { @MainActor in self.cancel(identifier: identifier) } }
    }

    private func cancel(identifier: UUID) {
        guard attemptID == identifier else { return }
        cancel()
    }

    func cancel() {
        attemptID = nil
        let pending = continuation
        continuation = nil
        session?.cancel()
        session = nil
        pending?.resume(throwing: NativeAuthenticationError.cancelled)
    }
}

@MainActor
protocol NativeAuthenticationSession: AnyObject {
    func start() -> Bool
    func cancel()
}
extension ASWebAuthenticationSession: NativeAuthenticationSession {}
