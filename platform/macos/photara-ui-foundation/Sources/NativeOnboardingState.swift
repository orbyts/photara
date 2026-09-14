import Foundation

/// Presentation consumes this state; durable identities/intents/receipts remain
/// Rust-owned. The reducer never grants authority or marks an HTTP receipt as
/// applied. Only a successful local transaction can issue `reconciled`.
enum NativeOnboardingState: Equatable {
    case localReady
    case freshSignInRequired
    case signingRequired
    case authenticating
    case bootstrapPending
    case outcomeUnknown
    case reconciling
    case reconciliationRequired
    case libraryChoiceRequired
    case cloudReady
    case cloudOffline
    case authRequired
    case signedOutCached
    case accessDisabled
    case protocolIncompatible
    case localContentEnrollmentRequired
}

struct NativeOnboardingStateMachine {
    private(set) var state = NativeOnboardingState.localReady
    private(set) var generation: UInt64 = 0
    private(set) var operationID: UUID?
    private(set) var principal: NativeVerifiedIdentity?
    private(set) var hasCloudBinding = false
    private(set) var submitted = false

    mutating func begin(signingAvailable: Bool, emptyLocalLibrary: Bool, operation: UUID) throws -> UInt64 {
        guard ![.authenticating, .bootstrapPending, .reconciling].contains(state),
              operationID == nil || operationID == operation
        else { throw NativeAuthenticationError.staleSession }
        generation += 1
        guard signingAvailable else { state = .signingRequired; return generation }
        guard emptyLocalLibrary || hasCloudBinding else { state = .localContentEnrollmentRequired; return generation }
        operationID = operation
        state = .authenticating
        return generation
    }

    mutating func authenticated(_ identity: NativeVerifiedIdentity, generation expected: UInt64) throws {
        guard generation == expected, state == .authenticating,
              principal == nil || principal == identity
        else { throw NativeAuthenticationError.staleSession }
        principal = identity
        state = .bootstrapPending
    }

    /// Call only after Rust durably commits the exact intent and marks dispatch.
    mutating func dispatchRecorded(generation expected: UInt64) throws {
        guard generation == expected, state == .bootstrapPending else { throw NativeAuthenticationError.staleSession }
        submitted = true
    }

    /// Caller must persist a submitted receipt under its original identity even
    /// if this returns false after cancel/logout. It must never change selection.
    mutating func receiptRecorded(operation: UUID, identity: NativeVerifiedIdentity, generation expected: UInt64) -> Bool {
        guard generation == expected, operationID == operation, principal == identity,
              submitted, [.bootstrapPending, .outcomeUnknown].contains(state) else { return false }
        state = .reconciling
        return true
    }

    mutating func reconciled(generation expected: UInt64, sameLibrary: Bool, transactionApplied: Bool) throws {
        guard generation == expected, state == .reconciling else { throw NativeAuthenticationError.staleSession }
        if !sameLibrary { state = .libraryChoiceRequired; return }
        guard transactionApplied else { state = .reconciliationRequired; return }
        hasCloudBinding = true
        submitted = false
        state = .cloudReady
    }

    mutating func cancel() {
        generation += 1
        state = submitted ? .outcomeUnknown : (hasCloudBinding ? .authRequired : .localReady)
    }

    /// Presentation follows a successfully persisted Rust terminal disposition.
    /// Original intent and any late receipt stay in that journal.
    mutating func abandonedExpired(operation: UUID, generation expected: UInt64) throws {
        guard generation == expected, operationID == operation, submitted,
              state == .outcomeUnknown, !hasCloudBinding
        else { throw NativeAuthenticationError.staleSession }
        generation += 1
        operationID = nil
        submitted = false
        state = .freshSignInRequired
    }

    mutating func networkUnavailable() {
        state = submitted ? .outcomeUnknown : (hasCloudBinding ? .cloudOffline : .localReady)
    }

    mutating func logout() {
        generation += 1
        state = hasCloudBinding ? .signedOutCached : (submitted ? .outcomeUnknown : .localReady)
        // Keep the principal and operation: account switch cannot retarget an
        // existing journal. A separately selected journal gets another reducer.
    }

    mutating func denyAccess() { generation += 1; state = .accessDisabled }
    mutating func incompatibleProtocol() { generation += 1; state = .protocolIncompatible }
}

/// All network calls are bounded and pinned to the checked origin. Redirects
/// are refused before credentials can be forwarded; caches/cookies are disabled.
final class NativePinnedTransport: NSObject, URLSessionTaskDelegate, @unchecked Sendable {
    private let origin: URL
    private let timeout: TimeInterval

    convenience init(origin: URL, developmentLoopback: Bool, timeout: TimeInterval = 15) throws {
        try self.init(origin: origin, developmentLoopback: developmentLoopback, timeout: timeout, permittedPort: 8080)
    }

    #if PHOTARA_SERVICE_FURNACE
    /// This initializer is absent from shipped/app builds, including development.
    convenience init(furnaceOrigin: URL, timeout: TimeInterval) throws {
        guard furnaceOrigin.host == "127.0.0.1", let port = furnaceOrigin.port, port > 1024
        else { throw NativeAuthenticationError.invalidResponse }
        try self.init(origin: furnaceOrigin, developmentLoopback: true, timeout: timeout, permittedPort: port)
    }
    #endif

    private init(origin: URL, developmentLoopback: Bool, timeout: TimeInterval, permittedPort: Int) throws {
        guard let components = URLComponents(url: origin, resolvingAgainstBaseURL: false),
              components.user == nil, components.password == nil,
              components.query == nil, components.fragment == nil,
              components.path.isEmpty || components.path == "/",
              components.scheme == "https" || (developmentLoopback && components.scheme == "http" && components.host == "127.0.0.1" && components.port == permittedPort),
              timeout.isFinite, timeout > 0, timeout <= 15
        else { throw NativeAuthenticationError.invalidResponse }
        self.origin = origin
        self.timeout = timeout
    }

    func urlSession(_ session: URLSession, task: URLSessionTask,
                    willPerformHTTPRedirection response: HTTPURLResponse,
                    newRequest request: URLRequest,
                    completionHandler: @escaping @Sendable (URLRequest?) -> Void) { completionHandler(nil) }

    /// Paths and headers are internal typed-adapter output, never provider input.
    func send(path: String, method: String, body: Data?, headers: [String: String]) async throws -> (Data, Int) {
        guard path.hasPrefix("/"), !path.hasPrefix("//"), !path.contains("?"), !path.contains("#"),
              ["GET", "POST"].contains(method), body.map({ $0.count <= 65_536 }) ?? true,
              let url = URL(string: path, relativeTo: origin)?.absoluteURL,
              url.scheme == origin.scheme, url.host == origin.host, url.port == origin.port
        else { throw NativeAuthenticationError.invalidResponse }
        let configuration = URLSessionConfiguration.ephemeral
        configuration.urlCache = nil
        configuration.httpCookieStorage = nil
        configuration.httpShouldSetCookies = false
        configuration.urlCredentialStorage = nil
        configuration.timeoutIntervalForRequest = timeout
        configuration.timeoutIntervalForResource = timeout * 2
        let session = URLSession(configuration: configuration, delegate: self, delegateQueue: nil)
        defer { session.invalidateAndCancel() }
        var request = URLRequest(url: url, cachePolicy: .reloadIgnoringLocalCacheData, timeoutInterval: timeout)
        request.httpMethod = method
        request.httpBody = body
        request.allHTTPHeaderFields = headers
        do {
            let (stream, response) = try await session.bytes(for: request)
            guard let response = response as? HTTPURLResponse,
                  response.url == url, !(300..<400).contains(response.statusCode),
                  response.expectedContentLength <= 65_536
            else { throw NativeAuthenticationError.invalidResponse }
            var data = Data()
            for try await byte in stream {
                guard data.count < 65_536 else { throw NativeAuthenticationError.invalidResponse }
                data.append(byte)
            }
            return (data, response.statusCode)
        } catch is CancellationError { throw NativeAuthenticationError.cancelled }
        catch let error as NativeAuthenticationError { throw error }
        catch { throw NativeAuthenticationError.transportUnavailable }
    }
}
