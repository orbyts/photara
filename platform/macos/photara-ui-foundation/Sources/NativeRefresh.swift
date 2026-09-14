import Foundation
import Darwin

@MainActor protocol NativeCredentialPersistence {
    func read(_ namespace: NativeCredentialNamespace) throws -> NativeStoredCredential?
    func replace(_ value: NativeStoredCredential, namespace: NativeCredentialNamespace) throws
}
extension NativeCredentialStore: NativeCredentialPersistence {}

struct NativeRefreshResult {
    let accessToken: String
    let refreshToken: String
}

/// A lock file contains no credential or identity. Every app process must use the
/// same host-local installation lock URL. Keep it out of synchronized storage.
enum NativeRefresh {
    @MainActor
    static func perform(store: any NativeCredentialPersistence,
                        namespace: NativeCredentialNamespace, lockURL: URL,
                        exchange: (String) async throws -> NativeRefreshResult) async throws -> NativeRefreshResult {
        guard lockURL.isFileURL else { throw NativeAuthenticationError.keychainUnavailable }
        let descriptor = open(lockURL.path, O_RDWR | O_CREAT | O_NOFOLLOW | O_CLOEXEC, 0o600)
        guard descriptor >= 0 else { throw NativeAuthenticationError.keychainUnavailable }
        defer { close(descriptor) }
        var info = stat()
        guard fstat(descriptor, &info) == 0, info.st_uid == getuid(),
              (info.st_mode & S_IFMT) == S_IFREG, (info.st_mode & 0o077) == 0,
              info.st_nlink == 1, flock(descriptor, LOCK_EX | LOCK_NB) == 0
        else { throw NativeAuthenticationError.keychainUnavailable }
        defer { flock(descriptor, LOCK_UN) }
        guard var credential = try store.read(namespace), !credential.refreshInFlight,
              let oldRefreshToken = credential.refreshToken
        else { throw NativeAuthenticationError.invalidCredential }
        try Task.checkCancellation()
        credential.refreshInFlight = true
        try store.replace(credential, namespace: namespace)
        // Any failure from this point intentionally leaves the durable marker.
        // A new browser login is required; neither crash nor retry replays old RT.
        let result = try await exchange(oldRefreshToken)
        guard !result.accessToken.isEmpty, result.accessToken.utf8.count <= 16_384,
              !result.refreshToken.isEmpty, result.refreshToken.utf8.count <= 16_384,
              result.refreshToken != oldRefreshToken
        else { throw NativeAuthenticationError.invalidResponse }
        credential.refreshToken = result.refreshToken
        credential.refreshInFlight = false
        try store.replace(credential, namespace: namespace)
        // Cancellation after exchange does not discard the durable replacement.
        // It only prevents publishing access credentials into a cancelled session.
        try Task.checkCancellation()
        return result
    }
}
