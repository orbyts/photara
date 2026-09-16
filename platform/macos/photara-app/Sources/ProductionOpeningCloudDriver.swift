import AppKit
import Foundation

actor NativeProjectCreationJournal {
    let path: String
    init(path: String) { self.path = path }
    private func use<T>(_ action: (PhotaraProjectCreation) throws -> T) throws -> T {
        let journal = try PhotaraProjectCreation.open(path: path, packageExtension: ReleaseConfiguration.current.identity.projectPackageExtension)
        defer { try? journal.close() }
        return try action(journal)
    }
    func prepare(operation: String, title: String, destination: URL, destinationPin: String?) throws -> BridgeProjectCreation {
        try use { try $0.prepare(operationId: operation, title: title, destination: destination.path, destinationPin: destinationPin,
            createdAt: ISO8601DateFormatter.creation.string(from: Date())) }
    }
    func advance(_ operation: String) throws -> BridgeProjectCreation { try use { try $0.advance(operationId: operation) } }
    func operations() throws -> [BridgeProjectCreation] { try use { try $0.operations() } }
    func dispatch(_ operation: String) throws -> Data { try use { try $0.dispatch(operationId: operation) } }
    func receive(_ operation: String, receipt: Data) throws { try use { try $0.receive(operationId: operation, receipt: receipt) } }
    func cancel(_ operation: String) throws { try use { try $0.cancel(operationId: operation) } }
}
private extension ISO8601DateFormatter {
    static var creation: ISO8601DateFormatter {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return formatter
    }
}

private struct OnboardingChallenge: Decodable {
    let schema: String
    let challenge_id: String
    let nonce: String
    let expires_at: String
}

@MainActor
protocol NativeEnrollmentCredentialStore: NativeCredentialPersistence {
    func insert(_ value: NativeStoredCredential, namespace: NativeCredentialNamespace) throws
    func removePrepared(reference: String, configuration: ReleaseConfiguration) throws
    func recoveryNamespace(reference: String, configuration: ReleaseConfiguration) throws -> NativeCredentialNamespace
}
extension NativeCredentialStore: NativeEnrollmentCredentialStore {}

/// Inject only external boundaries. The deterministic harness uses this exact
/// driver, browser owner and real generated Rust journal against temporary files.
@MainActor
struct NativeOpeningDependencies {
    let credentials: () throws -> any NativeEnrollmentCredentialStore
    let ensureService: () async throws -> Void
    let transport: any NativeAuthenticationHTTP
    let supportDirectory: () throws -> URL
    let installation: () -> UUID
    let authenticate: (URL, String) async throws -> URL
    let exchange: (String, NativePKCEAttempt, Date, Date) async throws -> NativeAuthenticatedTokens
    var now: () -> Date = { Date() }
    var refresh: (String) async throws -> NativeRefreshResult = { _ in throw NativeAuthenticationError.invalidCredential }
    var verifyAccess: (String, Date) async throws -> NativeVerifiedIdentity = { _, _ in throw NativeAuthenticationError.invalidCredential }
}

actor NativeOnboardingJournal {
    private let path: String
    private var handle: PhotaraLocalOnboarding?
    init(path: String) { self.path = path }
    private func opened() throws -> PhotaraLocalOnboarding {
        if let handle { return handle }
        let handle = try PhotaraLocalOnboarding.open(path: path)
        self.handle = handle
        return handle
    }
    func localState() throws -> BridgeLocalState { try Task.checkCancellation(); return try opened().localState() }
    func prepare(_ input: BridgePrepareEnrollment) throws -> BridgeEnrollmentIntent {
        try Task.checkCancellation()
        return try opened().prepare(input: input)
    }
    func pending(library: String, environment: String, principal: BridgeEnrollmentPrincipal? = nil) throws -> BridgeEnrollmentIntent? {
        try opened().pending(libraryId: library, environmentId: environment, principal: principal)
    }
    func pendingCredential(library: String, environment: String) throws -> String? {
        try opened().pendingCredential(libraryId: library, environmentId: environment)
    }
    func expiredCredential(library: String, environment: String) throws -> String? {
        try opened().expiredCredential(libraryId: library, environmentId: environment)
    }
    func savedState(library: String, environment: String) throws -> String? {
        try opened().savedState(libraryId: library, environmentId: environment)
    }
    func cachedCredential(library: String, environment: String) throws -> String? {
        try opened().cachedCredential(libraryId: library, environmentId: environment)
    }
    func boundIntent(library: String, environment: String, principal: BridgeEnrollmentPrincipal) throws -> BridgeEnrollmentIntent {
        try opened().boundIntent(libraryId: library, environmentId: environment, principal: principal)
    }
    func abandonExpired(operation: String, principal: BridgeEnrollmentPrincipal, generation: Int64, absence: Data) throws {
        try Task.checkCancellation()
        try opened().abandonExpired(operationId: operation, principal: principal, generation: generation, absenceCanonical: absence)
    }
    func advance(_ principal: BridgeEnrollmentPrincipal?) throws -> Int64 {
        try opened().advanceSession(principal: principal)
    }
    func dispatch(operation: String, principal: BridgeEnrollmentPrincipal, generation: Int64) throws -> BridgeEnrollmentIntent {
        try Task.checkCancellation()
        return try opened().dispatch(operationId: operation, principal: principal, generation: generation)
    }
    func cancelPrepared(operation: String) throws { try opened().cancelPrepared(operationId: operation) }
    func record(operation: String, receipt: Data) throws {
        // Late receipts remain evidence even if the caller has cancelled.
        try opened().recordReceipt(operationId: operation, envelopeCanonical: receipt)
    }
    func apply(operation: String, principal: BridgeEnrollmentPrincipal, generation: Int64, session: Data) throws -> BridgeEnrollmentApplication {
        try Task.checkCancellation()
        return try opened().apply(operationId: operation, principal: principal, generation: generation, sessionCanonical: session)
    }
    func libraryChoice(operation: String, principal: BridgeEnrollmentPrincipal, session: Data) throws -> String {
        try Task.checkCancellation()
        return try opened().libraryChoice(operationId: operation, principal: principal, sessionCanonical: session)
    }
    func select(operation: String, principal: BridgeEnrollmentPrincipal, generation: Int64, session: Data) throws -> BridgeEnrollmentApplication {
        try Task.checkCancellation()
        return try opened().openExistingCloudLibrary(operationId: operation, principal: principal,
            generation: generation, sessionCanonical: session)
    }
    func close() throws { try handle?.close(); handle = nil }
}

@MainActor
final class ProductionOpeningCloudDriver: OpeningCloudDriver {
    private let configuration: ReleaseConfiguration
    private let injected: NativeOpeningDependencies?
    private let supportDirectoryOverride: URL?
    private let browser = NativeAuthenticationBrowser()
    private var generation: UInt64 = 0
    private var developmentService: NativeDevelopmentService?
    private var active = false
    private var libraryPicker: NativeLibraryPicker?

    /// UI1 explicitly requests service access. Passive Opening still reads no
    /// Keychain and starts no service. Refresh/device secrets stay native-only.
    func projectCreation(_ creation: BridgeProjectCreation, journal: NativeProjectCreationJournal) async throws {
        let deps = try dependencies()
        guard creation.environment == configuration.environment.environmentId,
              let issuer = creation.issuer, let subject = creation.subject else {
            throw NativeAuthenticationError.invalidCredential
        }
        let support = try deps.supportDirectory()
        let onboarding = NativeOnboardingJournal(path: configuration.identity.journalURL(support: support).appending(path: "photara-local-v2.sqlite").path)
        let cachedReference = try await onboarding.cachedCredential(library: creation.libraryId,
            environment: configuration.environment.environmentId)
        try await onboarding.close()
        guard let reference = cachedReference else {
            throw NativeAuthenticationError.invalidCredential
        }
        let store = try deps.credentials()
        let namespace = try store.recoveryNamespace(reference: reference, configuration: configuration)
        let refreshed = try await NativeRefresh.perform(store: store, namespace: namespace,
            lockURL: configuration.identity.journalURL(support: support).appending(path: "refresh.lock"), exchange: deps.refresh)
        let identity = try await deps.verifyAccess(refreshed.accessToken, deps.now())
        let expected = try NativeCredentialNamespace(configuration: configuration, installation: deps.installation(),
            issuer: issuer, subject: subject, reference: reference)
        guard identity.issuer == issuer, identity.subject == subject,
              namespace.account == expected.account, namespace.service == expected.service,
              let credential = try store.read(expected) else { throw NativeAuthenticationError.invalidCredential }
        try await deps.ensureService()
        try await verifyCapabilities(deps.transport)
        let headers = ["Authorization": "Bearer " + refreshed.accessToken,
            "photara-device-credential": credential.deviceCredential,
            "photara-device-id": creation.deviceId, "Accept": "application/json"]
        let command = try await journal.dispatch(creation.operationId)
        let response = try await deps.transport.send(path: "/v1/projects/create", method: "POST", body: command, headers: headers)
        guard response.1 == 200 else {
            throw NativeOnboardingFailure.response(phase: .service, data: response.0, status: response.1)
        }
        try await journal.receive(creation.operationId, receipt: response.0)
    }

    func setLibraryPicker(_ picker: @escaping NativeLibraryPicker) { libraryPicker = picker }

    init(configuration: ReleaseConfiguration = .current, dependencies: NativeOpeningDependencies? = nil, supportDirectoryOverride: URL? = nil) {
        self.configuration = configuration
        injected = dependencies
        self.supportDirectoryOverride = supportDirectoryOverride
    }

    var availability: OpeningCloudAvailability {
        if injected != nil { return .available }
        // Opening a local Library inspects public metadata only.
        return (try? NativeCredentialStore.declaredAccessGroup(bundleIdentifier: configuration.identity.bundleIdentifier)) == nil
            ? .signingRequired : .available
    }

    private func dependencies() throws -> NativeOpeningDependencies {
        if let injected { return injected }
        let provider = try NativeAuth0Client(environment: configuration.environment)
        return NativeOpeningDependencies(
            credentials: { try NativeCredentialStore(bundleIdentifier: self.configuration.identity.bundleIdentifier) },
            ensureService: { try await self.ensureService() },
            transport: try NativePinnedTransport(origin: configuration.environment.apiOrigin,
                developmentLoopback: configuration.environment.channel == .development),
            supportDirectory: {
                if let root = self.supportDirectoryOverride { return root }
                return try FileManager.default.url(for: .applicationSupportDirectory, in: .userDomainMask,
                    appropriateFor: nil, create: true).appending(path: self.configuration.identity.applicationSupportDirectory)
            },
            installation: { self.installationIdentifier() },
            authenticate: { url, scheme in
                guard let window = NSApp.keyWindow ?? NSApp.windows.first(where: { $0.isVisible })
                else { throw NativeAuthenticationError.browserUnavailable }
                return try await self.browser.authenticate(url: url, callbackScheme: scheme, window: window, ephemeral: false)
            },
            exchange: { code, attempt, started, now in
                try await NativeAuth0Client(environment: self.configuration.environment).exchange(
                    code: code, attempt: attempt, challengeCreatedAt: started, now: now)
            },
            refresh: { try await provider.refresh($0) },
            verifyAccess: { try await provider.verifyAccess($0, now: $1) }
        )
    }

    func savedState() async throws -> NativeOnboardingState? {
        guard availability == .available else { return nil }
        let deps = try dependencies()
        let journal = NativeOnboardingJournal(path: try configuration.identity.journalURL(support: deps.supportDirectory()).appending(path: "photara-local-v2.sqlite").path)
        do {
            let local = try await journal.localState()
            let state: NativeOnboardingState
            if let saved = try await journal.savedState(library: local.libraryId, environment: configuration.environment.environmentId) {
                switch saved {
                case "cached": state = .cloudOffline
                case "signed-out": state = .signedOutCached
                default: state = .accessDisabled
                }
            }
            else if try await journal.pendingCredential(library: local.libraryId, environment: configuration.environment.environmentId) != nil { state = .outcomeUnknown }
            else if try await journal.expiredCredential(library: local.libraryId, environment: configuration.environment.environmentId) != nil { state = .freshSignInRequired }
            else { state = .localReady }
            try await journal.close()
            return state
        } catch { try? await journal.close(); throw error }
    }

    func recoverIfNeeded(progress: @escaping @MainActor (NativeOnboardingState) -> Void) async throws -> NativeOnboardingState? {
        guard availability == .available else { return nil }
        let deps = try dependencies()
        let journal = NativeOnboardingJournal(path: try configuration.identity.journalURL(support: deps.supportDirectory()).appending(path: "photara-local-v2.sqlite").path)
        let reference: String?
        let expired: String?
        do {
            let local = try await journal.localState()
            reference = try await journal.pendingCredential(library: local.libraryId, environment: configuration.environment.environmentId)
            expired = try await journal.expiredCredential(library: local.libraryId, environment: configuration.environment.environmentId)
            try await journal.close()
        } catch { try? await journal.close(); throw error }
        guard reference != nil else { return expired == nil ? nil : .freshSignInRequired }
        progress(.reconciling)
        return try await perform(retryExpired: false, progress: progress)
    }

    func signIn(progress: @escaping @MainActor (NativeOnboardingState) -> Void) async throws -> NativeOnboardingState {
        try await perform(retryExpired: true, progress: progress)
    }

    func accountProfile(refreshAvatar: Bool) async throws -> NativeAccountProfile? {
        guard availability == .available else { return nil }
        let support = try dependencies().supportDirectory()
        let journal = NativeOnboardingJournal(path: configuration.identity.journalURL(support: support).appending(path: "photara-local-v2.sqlite").path)
        let local: BridgeLocalState
        let saved: String?
        do {
            local = try await journal.localState()
            saved = try await journal.savedState(library: local.libraryId, environment: configuration.environment.environmentId)
            try await journal.close()
        } catch { try? await journal.close(); throw error }
        guard saved == "cached" else { return nil }
        let url = NativeAccountProfileCache.url(support: support, environment: configuration.environment.environmentId, library: local.libraryId)
        guard var profile = NativeAccountProfileCache.read(url) else { return nil }
        if refreshAvatar, profile.avatar == nil {
            profile.avatar = await NativeAccountProfileCache.avatar(profile.picture)
            try? NativeAccountProfileCache.write(profile, to: url)
        }
        return profile
    }

    func signOut() async throws -> NativeOnboardingState {
        guard !active else {
            throw NativeOnboardingFailure(phase: .cleanup, reason: .busy, submitted: false, receiptRecorded: false)
        }
        active = true
        defer { active = false }
        let deps = try dependencies()
        let stateDirectory = try configuration.identity.journalURL(support: deps.supportDirectory())
        let attemptLock = try NativeOnboardingLock(url: stateDirectory.appending(path: "onboarding.lock"))
        let refreshLock = try NativeOnboardingLock(url: stateDirectory.appending(path: "refresh.lock"))
        defer { _fixLifetime(attemptLock); _fixLifetime(refreshLock) }
        let journal = NativeOnboardingJournal(path: stateDirectory.appending(path: "photara-local-v2.sqlite").path)
        var signedOut = false
        do {
            let local = try await journal.localState()
            guard let reference = try await journal.cachedCredential(library: local.libraryId, environment: configuration.environment.environmentId)
            else { throw NativeAuthenticationError.invalidCredential }
            // Revoke local session authority durably before touching Keychain.
            // Keep the device credential and library binding for the next login.
            _ = try await journal.advance(nil)
            signedOut = true
            let store = try deps.credentials()
            let namespace = try store.recoveryNamespace(reference: reference, configuration: configuration)
            try clearRefreshToken(store, namespace: namespace)
            try await journal.close()
            return .signedOutCached
        } catch {
            try? await journal.close()
            if signedOut {
                throw NativeOnboardingFailure(phase: .cleanup, reason: .failed, submitted: false,
                    receiptRecorded: false, stateOverride: .signedOutCached)
            }
            throw error
        }
    }

    private func clearRefreshToken(_ store: any NativeEnrollmentCredentialStore, namespace: NativeCredentialNamespace) throws {
        guard var credential = try store.read(namespace) else { throw NativeAuthenticationError.invalidCredential }
        credential.refreshToken = nil
        credential.refreshInFlight = false
        try store.replace(credential, namespace: namespace)
    }

    private func cacheProfile(_ profile: NativeAccountProfile?, support: URL, library: String) {
        // Replace an old optional profile even when this login has no claims.
        let profile = profile ?? NativeAccountProfile(name: nil, email: nil, picture: nil)
        try? NativeAccountProfileCache.write(profile, to: NativeAccountProfileCache.url(support: support,
            environment: configuration.environment.environmentId, library: library))
    }

    private func reconnect(journal: NativeOnboardingJournal, local: BridgeLocalState, reference: String,
                           store: any NativeEnrollmentCredentialStore, deps: NativeOpeningDependencies,
                           support: URL, progress: @escaping @MainActor (NativeOnboardingState) -> Void) async throws -> NativeOnboardingState {
        let refreshLock = try NativeOnboardingLock(url: configuration.identity.journalURL(support: support).appending(path: "refresh.lock"))
        defer { _fixLifetime(refreshLock) }
        var phase = NativeOnboardingPhase.credentialStorage
        var namespace: NativeCredentialNamespace?
        do {
            _ = try await journal.advance(nil)
            let retainedNamespace = try store.recoveryNamespace(reference: reference, configuration: configuration)
            namespace = retainedNamespace
            // Also finishes a previous interrupted logout before opening Google.
            try clearRefreshToken(store, namespace: retainedNamespace)
            guard let credential = try store.read(retainedNamespace) else { throw NativeAuthenticationError.invalidCredential }
            phase = .service
            try await deps.ensureService()
            phase = .capabilities
            try await verifyCapabilities(deps.transport)
            generation += 1
            let currentGeneration = generation
            let started = deps.now()
            var attempt = try NativePKCEAttempt(callbackURL: configuration.environment.callbackURL,
                nonce: AuthenticationEncoding.random(), generation: currentGeneration, now: started)
            let url = try attempt.authorizationURL(environment: configuration.environment, switchingAccount: true)
            try Task.checkCancellation()
            phase = .browser
            progress(.authenticating)
            let callback = try await deps.authenticate(url, configuration.identity.callbackScheme)
            try Task.checkCancellation()
            phase = .callback
            let code = try attempt.consume(callback, generation: currentGeneration, now: deps.now())
            phase = .tokenExchange
            let tokens = try await deps.exchange(code, attempt, started, deps.now())
            try Task.checkCancellation()
            guard deps.now() < tokens.expiresAt else { throw NativeAuthenticationError.expiredAttempt }
            let expected = try NativeCredentialNamespace(configuration: configuration, installation: deps.installation(),
                issuer: tokens.identity.issuer, subject: tokens.identity.subject, reference: reference)
            guard retainedNamespace.account == expected.account, retainedNamespace.service == expected.service else {
                throw NativeOnboardingFailure(phase: phase, reason: .differentAccount, submitted: false,
                    receiptRecorded: false, stateOverride: .signedOutCached)
            }
            let principal = BridgeEnrollmentPrincipal(issuer: tokens.identity.issuer, subject: tokens.identity.subject)
            let bound = try await journal.boundIntent(library: local.libraryId,
                environment: configuration.environment.environmentId, principal: principal)
            let command = try JSONSerialization.jsonObject(with: bound.commandCanonical) as? [String: Any]
            guard command?["device_credential_sha256"] as? String == AuthenticationEncoding.sha256(
                try AuthenticationEncoding.decodeBase64URL(credential.deviceCredential)) else { throw NativeAuthenticationError.invalidCredential }
            phase = .session
            progress(.reconciling)
            let headers = [
                "Authorization": "Bearer " + tokens.accessToken, "photara-device-id": bound.deviceId,
                "photara-device-credential": credential.deviceCredential, "Accept": "application/json"]
            var session = try await deps.transport.send(path: "/v1/session", method: "GET", body: nil, headers: headers)
            guard session.1 == 200 else { throw NativeOnboardingFailure.response(phase: phase, data: session.0, status: session.1) }
            if let libraryPicker {
                let id = try await journal.libraryChoice(operation: bound.operationId, principal: principal, session: session.0)
                let selected = try await libraryPicker([NativeLibraryChoice(id: id, name: configuration.identity.defaultLibraryName)])
                try Task.checkCancellation()
                guard selected == id, deps.now() < tokens.expiresAt else { throw NativeAuthenticationError.expiredAttempt }
                // Selection can take minutes. Recheck access after the click.
                session = try await deps.transport.send(path: "/v1/session", method: "GET", body: nil, headers: headers)
                guard session.1 == 200 else { throw NativeOnboardingFailure.response(phase: phase, data: session.0, status: session.1) }
            }
            try Task.checkCancellation()
            phase = .credentialStorage
            try store.replace(NativeStoredCredential(refreshToken: tokens.refreshToken, refreshInFlight: false,
                deviceCredential: credential.deviceCredential), namespace: expected)
            let durableGeneration = try await journal.advance(principal)
            phase = .reconciliation
            let result = try await journal.apply(operation: bound.operationId, principal: principal,
                generation: durableGeneration, session: session.0)
            guard case .applied = result else { throw NativeAuthenticationError.invalidResponse }
            cacheProfile(tokens.profile, support: support, library: local.libraryId)
            try await journal.close()
            return .cloudReady
        } catch {
            _ = try? await journal.advance(nil)
            if let namespace { try? clearRefreshToken(store, namespace: namespace) }
            let cancelled = Task.isCancelled || error is CancellationError || (error as? NativeAuthenticationError) == .cancelled
            var failure = (error as? NativeOnboardingFailure) ?? NativeOnboardingFailure(phase: phase,
                reason: cancelled ? .cancelled : .failed, submitted: false, receiptRecorded: false)
            failure.stateOverride = .signedOutCached
            throw failure
        }
    }

    private func perform(retryExpired: Bool, progress: @escaping @MainActor (NativeOnboardingState) -> Void) async throws -> NativeOnboardingState {
        guard !active else {
            throw NativeOnboardingFailure(phase: .journalRecovery, reason: .busy, submitted: false, receiptRecorded: false)
        }
        active = true
        defer { active = false }
        let before = generation
        let result = try await attempt(progress: progress)
        // A deliberate click may first retire a previous failed operation, then
        // start one fresh authorization. Never start two browsers for one click.
        if retryExpired, result == .freshSignInRequired, generation == before {
            return try await attempt(progress: progress)
        }
        return result
    }

    private func attempt(progress: @escaping @MainActor (NativeOnboardingState) -> Void) async throws -> NativeOnboardingState {
        var phase = NativeOnboardingPhase.signing
        var journal: NativeOnboardingJournal?
        var intent: BridgeEnrollmentIntent?
        var store: (any NativeEnrollmentCredentialStore)?
        var submitted = false
        let receiptRecorded = false
        var attemptLock: NativeOnboardingLock?
        // ARC must retain the cross-process lock through awaited cleanup/close.
        defer { _fixLifetime(attemptLock) }
        do {
            try Task.checkCancellation()
            let deps = try dependencies()
            store = try deps.credentials()
            phase = .journalRecovery
            let support = try deps.supportDirectory()
            let stateDirectory = configuration.identity.journalURL(support: support)
            try FileManager.default.createDirectory(at: stateDirectory, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
            attemptLock = try NativeOnboardingLock(url: stateDirectory.appending(path: "onboarding.lock"))
            let localJournal = NativeOnboardingJournal(path: stateDirectory.appending(path: "photara-local-v2.sqlite").path)
            journal = localJournal
            let local = try await localJournal.localState()
            if let reference = try await localJournal.cachedCredential(library: local.libraryId, environment: configuration.environment.environmentId) {
                return try await reconnect(journal: localJournal, local: local, reference: reference,
                    store: store!, deps: deps, support: support, progress: progress)
            }
            // The journal reveals only an opaque private Keychain locator until
            // a recovered access token independently verifies the same identity.
            if let reference = try await localJournal.pendingCredential(library: local.libraryId, environment: configuration.environment.environmentId) {
                submitted = true
                phase = .service
                try await deps.ensureService()
                phase = .capabilities
                try await verifyCapabilities(deps.transport)
                phase = .sessionRefresh
                let namespace = try store!.recoveryNamespace(reference: reference, configuration: configuration)
                let refreshed = try await NativeRefresh.perform(store: store!, namespace: namespace,
                    lockURL: stateDirectory.appending(path: "refresh.lock"), exchange: deps.refresh)
                let identity = try await deps.verifyAccess(refreshed.accessToken, deps.now())
                let expected = try NativeCredentialNamespace(configuration: configuration, installation: deps.installation(),
                    issuer: identity.issuer, subject: identity.subject, reference: reference)
                guard namespace.account == expected.account, namespace.service == expected.service,
                      let credential = try store!.read(expected)
                else { throw NativeAuthenticationError.invalidCredential }
                let principal = BridgeEnrollmentPrincipal(issuer: identity.issuer, subject: identity.subject)
                guard let retained = try await localJournal.pending(library: local.libraryId,
                    environment: configuration.environment.environmentId, principal: principal), retained.credentialReference == reference
                else { throw NativeAuthenticationError.invalidCredential }
                intent = retained
                let durableGeneration = try await localJournal.advance(principal)
                return try await finishEnrollment(journal: localJournal, intent: retained, principal: principal,
                    generation: durableGeneration, deps: deps, access: refreshed.accessToken,
                    credential: credential.deviceCredential, initial: nil, proof: nil, progress: progress)
            }
            let abandoned: BridgeEnrollmentIntent?
            do { abandoned = try await localJournal.pending(library: local.libraryId, environment: configuration.environment.environmentId) }
            catch {
                throw NativeOnboardingFailure(phase: .journalRecovery, reason: .retainedOperation, submitted: false, receiptRecorded: false)
            }
            if let abandoned {
                guard abandoned.state == "prepared", abandoned.principal == nil else {
                    throw NativeOnboardingFailure(phase: .journalRecovery, reason: .retainedOperation, submitted: false, receiptRecorded: false)
                }
                if abandoned.replacesOperationId == nil {
                    try store!.removePrepared(reference: abandoned.credentialReference, configuration: configuration)
                }
                try await localJournal.cancelPrepared(operation: abandoned.operationId)
            }
            try Task.checkCancellation()
            phase = .service
            try await deps.ensureService()
            try Task.checkCancellation()
            phase = .capabilities
            try await verifyCapabilities(deps.transport)
            try Task.checkCancellation()
            phase = .preparation
            generation += 1
            let currentGeneration = generation
            let retryReference = try await localJournal.expiredCredential(library: local.libraryId, environment: configuration.environment.environmentId)
            let retryNamespace: NativeCredentialNamespace?
            let deviceCredential: String
            if let retryReference {
                let namespace = try store!.recoveryNamespace(reference: retryReference, configuration: configuration)
                guard let retained = try store!.read(namespace) else { throw NativeAuthenticationError.invalidCredential }
                retryNamespace = namespace
                deviceCredential = retained.deviceCredential
            } else {
                retryNamespace = nil
                deviceCredential = try AuthenticationEncoding.random()
            }
            let prepared = try await localJournal.prepare(.init(
                operationId: UUID().uuidString.lowercased(), libraryId: local.libraryId,
                localPrincipalId: local.principalId, environmentId: configuration.environment.environmentId,
                issuer: configuration.environment.auth0Issuer.absoluteString,
                credentialReference: retryReference ?? NativeCredentialReference.create(),
                deviceCommitmentSha256: AuthenticationEncoding.sha256(try AuthenticationEncoding.decodeBase64URL(deviceCredential)),
                deviceDisplayName: "Mac"
            ))
            intent = prepared
            try Task.checkCancellation()
            phase = .challenge
            let challengeStartedAt = deps.now()
            let (challengeData, challengeStatus) = try await deps.transport.send(
                path: "/v1/onboarding/challenges", method: "POST", body: prepared.challengeCanonical,
                headers: ["Content-Type": "application/json", "Accept": "application/json"])
            guard challengeStatus == 200 else { throw NativeAuthenticationError.invalidResponse }
            let challenge: OnboardingChallenge = try decodeCanonical(challengeData,
                keys: ["schema", "challenge_id", "nonce", "expires_at"])
            guard challenge.schema == "photara.onboarding.v1",
                  let challengeID = UUID(uuidString: challenge.challenge_id),
                  challengeID.uuidString != "00000000-0000-0000-0000-000000000000",
                  try AuthenticationEncoding.decodeBase64URL(challenge.nonce).count == 32,
                  let expiresMilliseconds = Int64(challenge.expires_at), String(expiresMilliseconds) == challenge.expires_at
            else { throw NativeAuthenticationError.invalidResponse }
            let challengeDeadline = Date(timeIntervalSince1970: Double(expiresMilliseconds) / 1000)
            guard challengeDeadline > deps.now(), challengeDeadline <= challengeStartedAt.addingTimeInterval(360)
            else { throw NativeAuthenticationError.expiredAttempt }
            var attempt = try NativePKCEAttempt(callbackURL: configuration.environment.callbackURL,
                nonce: challenge.nonce, generation: currentGeneration, now: deps.now())
            let authorizationURL = try attempt.authorizationURL(environment: configuration.environment, switchingAccount: false)
            guard configuration.identity.callbackScheme == configuration.environment.callbackURL.scheme
            else { throw NativeAuthenticationError.invalidResponse }
            try Task.checkCancellation()
            phase = .browser
            progress(.authenticating)
            let callback = try await deps.authenticate(authorizationURL, configuration.identity.callbackScheme)
            try Task.checkCancellation()
            phase = .callback
            guard deps.now() < challengeDeadline else { throw NativeAuthenticationError.expiredAttempt }
            let code = try attempt.consume(callback, generation: currentGeneration, now: deps.now())
            phase = .tokenExchange
            let tokens = try await deps.exchange(code, attempt, challengeStartedAt, deps.now())
            try Task.checkCancellation()
            guard deps.now() < challengeDeadline, deps.now() < tokens.expiresAt else { throw NativeAuthenticationError.expiredAttempt }
            let principal = BridgeEnrollmentPrincipal(issuer: tokens.identity.issuer, subject: tokens.identity.subject)
            phase = .credentialStorage
            let namespace = try NativeCredentialNamespace(configuration: configuration, installation: deps.installation(),
                issuer: tokens.identity.issuer, subject: tokens.identity.subject, reference: prepared.credentialReference)
            let storedTokens = NativeStoredCredential(refreshToken: tokens.refreshToken, refreshInFlight: false, deviceCredential: deviceCredential)
            if let retryNamespace {
                // Fresh authorization may replace this installation's retained
                // token only for exactly the same verified identity namespace.
                guard namespace.account == retryNamespace.account, namespace.service == retryNamespace.service
                else { throw NativeAuthenticationError.invalidCredential }
                try store!.replace(storedTokens, namespace: namespace)
            } else { try store!.insert(storedTokens, namespace: namespace) }
            phase = .dispatch
            try Task.checkCancellation()
            let durableGeneration = try await localJournal.advance(principal)
            try Task.checkCancellation()
            // An ambiguous journal result is treated as submitted until Rust
            // positively confirms the operation is still unbound/prepared.
            submitted = true
            do {
                _ = try await localJournal.dispatch(operation: prepared.operationId, principal: principal, generation: durableGeneration)
            } catch {
                if let pending = try? await localJournal.pending(library: local.libraryId, environment: configuration.environment.environmentId),
                   pending.operationId == prepared.operationId, pending.state == "prepared" { submitted = false }
                throw error
            }
            progress(.bootstrapPending)
            try Task.checkCancellation()
            phase = .bootstrap
            let headers = ["Authorization": "Bearer \(tokens.accessToken)",
                "photara-device-credential": deviceCredential, "photara-challenge-id": challenge.challenge_id,
                "photara-enrollment-token": tokens.enrollmentIDToken,
                "Content-Type": "application/json", "Accept": "application/json"]
            var response: (Data, Int)?
            do {
                response = try await deps.transport.send(path: "/v1/onboarding/bootstrap", method: "POST",
                    body: prepared.commandCanonical, headers: headers)
            } catch { try Task.checkCancellation() }
            let result = try await finishEnrollment(journal: localJournal, intent: prepared, principal: principal,
                generation: durableGeneration, deps: deps, access: tokens.accessToken, credential: deviceCredential,
                initial: response, proof: (headers, challengeDeadline), progress: progress)
            if result == .cloudReady { cacheProfile(tokens.profile, support: support, library: local.libraryId) }
            return result
        } catch {
            let cancelled = Task.isCancelled || error is CancellationError || (error as? NativeAuthenticationError) == .cancelled
            var failure = (error as? NativeOnboardingFailure) ?? NativeOnboardingFailure(
                phase: phase, reason: cancelled ? .cancelled : .failed, submitted: submitted, receiptRecorded: receiptRecorded)
            if let journal {
                if !submitted, let intent, let store {
                    do {
                        // Remove first: a failed Keychain cleanup leaves the
                        // prepared journal discoverable for the next attempt.
                        if intent.replacesOperationId == nil {
                            try store.removePrepared(reference: intent.credentialReference, configuration: configuration)
                        }
                        try await journal.cancelPrepared(operation: intent.operationId)
                    } catch {
                        failure = NativeOnboardingFailure(phase: .cleanup, reason: .failed, submitted: false, receiptRecorded: false)
                    }
                }
                if submitted && cancelled { _ = try? await journal.advance(nil) }
                try? await journal.close()
            }
            throw failure
        }
    }

    private func finishEnrollment(journal: NativeOnboardingJournal, intent: BridgeEnrollmentIntent,
                                  principal: BridgeEnrollmentPrincipal, generation: Int64, deps: NativeOpeningDependencies,
                                  access: String, credential: String, initial: (Data, Int)?,
                                  proof: ([String: String], Date)?,
                                  progress: @escaping @MainActor (NativeOnboardingState) -> Void) async throws -> NativeOnboardingState {
        var phase = NativeOnboardingPhase.receipt
        var recorded = false
        do {
            // Check the exact durable device commitment before disclosing its
            // credential to the pinned service. Rust already validates command bytes.
            let command = try JSONSerialization.jsonObject(with: intent.commandCanonical) as? [String: Any]
            guard command?["device_credential_sha256"] as? String == AuthenticationEncoding.sha256(try AuthenticationEncoding.decodeBase64URL(credential))
            else { throw NativeAuthenticationError.invalidCredential }
            let headers = ["Authorization": "Bearer " + access, "photara-device-credential": credential,
                "Accept": "application/json", "Content-Type": "application/json"]
            var receipt: Data
            if let initial, initial.1 == 200 {
                receipt = initial.0
            } else {
                try Task.checkCancellation()
                phase = .operationLookup
                progress(.outcomeUnknown)
                let lookup = try await deps.transport.send(path: "/v1/onboarding/operations/" + intent.operationId,
                    method: "GET", body: nil, headers: headers)
                if lookup.1 == 200 { receipt = lookup.0 }
                else if lookup.1 == 404, NativeServiceFailureCode.decode(lookup.0) == .operationNotFound {
                    phase = .operationReplay
                    try Task.checkCancellation()
                    // One exact replay also covers a commit racing the lookup.
                    // A fresh original proof permits first commit; without it
                    // the service permits only replay of an existing receipt.
                    let freshProof = proof.flatMap { deps.now() < $0.1 ? $0.0 : nil }
                    let replay = try await deps.transport.send(path: "/v1/onboarding/bootstrap", method: "POST",
                        body: intent.commandCanonical, headers: freshProof ?? headers)
                    if replay.1 == 200 { receipt = replay.0 }
                    else if replay.1 == 403, freshProof == nil, NativeServiceFailureCode.decode(replay.0) == .accessDenied {
                        // A racing commit wins over terminal disposition.
                        phase = .operationLookup
                        let confirmed = try await deps.transport.send(path: "/v1/onboarding/operations/" + intent.operationId,
                            method: "GET", body: nil, headers: headers)
                        if confirmed.1 == 200 { receipt = confirmed.0 }
                        else if confirmed.1 == 404, NativeServiceFailureCode.decode(confirmed.0) == .operationNotFound {
                            try await journal.abandonExpired(operation: intent.operationId, principal: principal,
                                generation: generation, absence: confirmed.0)
                            try await journal.close()
                            return .freshSignInRequired
                        } else { throw NativeOnboardingFailure.response(phase: phase, data: confirmed.0, status: confirmed.1) }
                    } else { throw NativeOnboardingFailure.response(phase: phase, data: replay.0, status: replay.1) }
                } else {
                    // Preserve the original rejected bootstrap diagnostic when
                    // its authentication failure also prevents the lookup.
                    if let initial, (400...499).contains(initial.1), lookup.1 == initial.1 {
                        throw NativeOnboardingFailure.response(phase: .bootstrap, data: initial.0, status: initial.1)
                    }
                    throw NativeOnboardingFailure.response(phase: phase, data: lookup.0, status: lookup.1)
                }
            }
            phase = .receipt
            try await journal.record(operation: intent.operationId, receipt: receipt)
            recorded = true
            try Task.checkCancellation()
            progress(.reconciling)
            phase = .session
            var sessionHeaders = headers
            sessionHeaders["photara-device-id"] = intent.deviceId
            var session = try await deps.transport.send(path: "/v1/session", method: "GET", body: nil, headers: sessionHeaders)
            guard session.1 == 200 else {
                throw NativeOnboardingFailure.response(phase: phase, data: session.0, status: session.1, receiptRecorded: true)
            }
            if let libraryPicker {
                let id = try await journal.libraryChoice(operation: intent.operationId, principal: principal, session: session.0)
                let selected = try await libraryPicker([NativeLibraryChoice(id: id, name: configuration.identity.defaultLibraryName)])
                try Task.checkCancellation()
                guard selected == id else { throw NativeAuthenticationError.invalidResponse }
                session = try await deps.transport.send(path: "/v1/session", method: "GET", body: nil, headers: sessionHeaders)
                guard session.1 == 200 else {
                    throw NativeOnboardingFailure.response(phase: phase, data: session.0, status: session.1, receiptRecorded: true)
                }
            }
            phase = .reconciliation
            let application: BridgeEnrollmentApplication
            if libraryPicker != nil {
                application = try await journal.select(operation: intent.operationId, principal: principal,
                    generation: generation, session: session.0)
            } else {
                application = try await journal.apply(operation: intent.operationId, principal: principal,
                    generation: generation, session: session.0)
            }
            try await journal.close()
            switch application {
            case .applied: return .cloudReady
            case .libraryChoiceRequired: return .libraryChoiceRequired
            case .reconciliationRequired: return .reconciliationRequired
            }
        } catch {
            if let failure = error as? NativeOnboardingFailure { throw failure }
            throw NativeOnboardingFailure(phase: phase, reason: Task.isCancelled ? .cancelled : .failed,
                submitted: true, receiptRecorded: recorded)
        }
    }

    private func installationIdentifier() -> UUID {
        let key = "photara.installation-id.v1"
        if let text = UserDefaults.standard.string(forKey: key), let value = UUID(uuidString: text) { return value }
        let value = UUID()
        UserDefaults.standard.set(value.uuidString.lowercased(), forKey: key)
        return value
    }

    private func verifyCapabilities(_ transport: any NativeAuthenticationHTTP) async throws {
        let (data, status) = try await transport.send(path: "/v1/onboarding/capabilities", method: "GET", body: nil,
            headers: ["Accept": "application/json"])
        guard status == 200 else { throw NativeAuthenticationError.invalidResponse }
        try NativeServiceCapabilities.validate(data, environment: configuration.environment)
    }

    private func decodeCanonical<T: Decodable>(_ data: Data, keys: Set<String>) throws -> T {
        try AuthenticationJSON.validateCanonicalObject(data, keys: keys, maximum: 65_536)
        return try JSONDecoder().decode(T.self, from: data)
    }

    private func ensureService() async throws {
        guard configuration.environment.channel == .development else { return }
        if developmentService == nil {
            developmentService = try NativeDevelopmentService.installed(configuration: configuration)
        }
        try await developmentService!.ensureReady()
    }
}
