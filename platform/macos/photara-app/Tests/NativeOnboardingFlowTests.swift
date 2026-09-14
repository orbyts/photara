import Foundation
import AuthenticationServices
import SQLite3

private func canonical(_ value: Any) throws -> Data {
    try JSONSerialization.data(withJSONObject: value, options: [.sortedKeys, .withoutEscapingSlashes])
}
private func object(_ data: Data) throws -> [String: Any] {
    try JSONSerialization.jsonObject(with: data) as! [String: Any]
}
private let syntheticIssuer = "https://synthetic.invalid/"
private let syntheticSubject = "synthetic-person"
private let syntheticConfiguration = ReleaseConfiguration(
    identity: ProductIdentity(displayName: "Synthetic", shortName: "Synthetic", isCodename: true,
        bundleIdentifier: "com.synthetic.desktop", keychainService: "com.synthetic.auth", callbackScheme: "com.synthetic.desktop",
        projectPackageDisplayType: "Synthetic", projectPackageExtension: "synthetic", apiAudienceNamespace: "urn:synthetic",
        serviceHostname: nil, userAgent: "Synthetic/1", applicationSupportDirectory: "Synthetic", cacheDirectory: "Synthetic",
        defaultLibraryName: "My Library", websiteURL: nil, supportURL: nil, privacyURL: nil, storeURL: nil),
    environment: ReleaseEnvironment(channel: .development, environmentId: "synthetic", apiOrigin: URL(string: "http://127.0.0.1:8080/")!,
        auth0Issuer: URL(string: syntheticIssuer)!, auth0Audience: "urn:synthetic", nativeClientId: "synthetic-native",
        callbackURL: URL(string: "com.synthetic.desktop://synthetic.invalid/macos/com.synthetic.desktop/callback")!,
        logoutURL: URL(string: "com.synthetic.desktop://synthetic.invalid/macos/com.synthetic.desktop/callback")!,
        schemaFamily: "photara.service.g2", schemaEpoch: 1, minimumAPI: 3, loggingPolicy: "redacted-status-only", telemetryEnabled: false))

/// Only static fixture labels are used as tokens, entirely in memory. Actual
/// command/receipt/session JSON crosses the real Rust ABI and temporary SQLite.
private actor SyntheticService: NativeAuthenticationHTTP {
    var fault: NativeOnboardingPhase?
    var challengeVariant: String?
    var holdPath: String?
    var paths: [String] = []
    var challengeBody: Data?
    var command: Data?
    var receiptFields: [String: String] = [:]
    private var arrived: Set<String> = []
    private var waiters: [String: CheckedContinuation<Void, Never>] = [:]
    private var held: CheckedContinuation<Void, Never>?
    let nonce = AuthenticationEncoding.base64URL(Data(repeating: 7, count: 32))
    func configure(fault: NativeOnboardingPhase? = nil, challengeVariant: String? = nil, hold: String? = nil) {
        self.fault = fault; self.challengeVariant = challengeVariant; holdPath = hold
    }
    func waitFor(_ path: String) async {
        if arrived.contains(path) { return }
        await withCheckedContinuation { waiters[path] = $0 }
    }
    func release() { held?.resume(); held = nil }
    func send(path: String, method: String, body: Data?, headers: [String: String]) async throws -> (Data, Int) {
        paths.append(path)
        arrived.insert(path)
        waiters.removeValue(forKey: path)?.resume()
        if path == holdPath { await withCheckedContinuation { held = $0 } }
        switch path {
        case "/v1/onboarding/capabilities":
            precondition(method == "GET" && body == nil && headers["Authorization"] == nil)
            var fields: [String: Any] = ["schema": "photara.onboarding.v1", "environment_id": "synthetic",
                "service_origin": "http://127.0.0.1:8080/", "minimum_api": "3", "canonical_codec": "photara.canonical-json.v1",
                "max_body_bytes": "65536", "max_token_bytes": "16384", "challenge_ttl_seconds": "300", "authentication_profile": "auth0-rs256-api-userinfo-v1", "providers": ["google"]]
            if fault == .capabilities { fields["service_origin"] = "https://substituted.invalid/" }
            return (try canonical(fields), 200)
        case "/v1/onboarding/challenges":
            precondition(method == "POST" && body != nil && headers["Authorization"] == nil)
            challengeBody = body
            let input = try object(body!)
            precondition(input["action"] as? String == "bootstrap")
            var value = ["schema": "photara.onboarding.v1", "challenge_id": UUID().uuidString.lowercased(),
                "nonce": nonce, "expires_at": String(Int64(Date().addingTimeInterval(300).timeIntervalSince1970 * 1000))]
            if fault == .challenge { value["nonce"] = "bad" }
            switch challengeVariant {
            case "expired": value["expires_at"] = "1"
            case "overlong": value["expires_at"] = String(Int64(Date().addingTimeInterval(3600).timeIntervalSince1970 * 1000))
            case "nondecimal": value["expires_at"] = "tomorrow"
            case "nil-id": value["challenge_id"] = "00000000-0000-0000-0000-000000000000"
            case "noncanonical": return (Data("{\"schema\":\"x\",\"schema\":\"y\"}".utf8), 200)
            default: break
            }
            return (try canonical(value), 200)
        case "/v1/onboarding/bootstrap":
            precondition(method == "POST" && headers["Authorization"] == "Bearer synthetic-access")
            precondition(headers["photara-enrollment-token"] == "synthetic-id-token")
            precondition(headers["photara-device-credential"] != nil)
            command = body
            let input = try object(body!), challenge = try object(challengeBody!)
            precondition(challenge["request_sha256"] as? String == AuthenticationEncoding.sha256(body!))
            precondition(challenge["operation_id"] as? String == input["operation_id"] as? String)
            let credential = try AuthenticationEncoding.decodeBase64URL(headers["photara-device-credential"]!)
            precondition(input["device_credential_sha256"] as? String == AuthenticationEncoding.sha256(credential))
            if fault == .bootstrap { return (try canonical(["schema": "photara.onboarding.v1", "code": "authentication-required"]), 401) }
            var receipt = input as! [String: String]
            receipt.removeValue(forKey: "intent"); receipt.removeValue(forKey: "device_display_name"); receipt.removeValue(forKey: "library_display_name")
            receipt["request_sha256"] = AuthenticationEncoding.sha256(body!)
            receipt["issuer"] = syntheticIssuer; receipt["subject"] = syntheticSubject
            for field in ["account_id", "identity_id", "membership_id", "stream_id", "stream_epoch"] { receipt[field] = UUID().uuidString.lowercased() }
            receipt["outcome"] = "claimed-default"; receipt["default_library_id"] = receipt["requested_library_id"]
            receipt["contract_version"] = "1"; receipt["authorization_generation"] = "1"; receipt["initial_high_water"] = "0"
            receipt["completed_at"] = String(Int64(Date().timeIntervalSince1970 * 1000))
            receiptFields = receipt
            if fault == .receipt { receipt["subject"] = "wrong-principal" }
            return (try canonical(["receipt": receipt, "receipt_sha256": AuthenticationEncoding.sha256(try canonical(receipt))]), 200)
        case "/v1/session":
            precondition(method == "GET" && headers["photara-device-id"] == receiptFields["device_id"])
            if fault == .session { return (Data(), 503) }
            var session: [String: String] = [:]
            for field in ["schema", "issuer", "subject", "account_id", "identity_id", "device_id", "default_library_id", "membership_id", "authorization_generation"] {
                session[field] = receiptFields[field]
            }
            for field in ["account_state", "identity_state", "device_state", "credential_state"] { session[field] = "active" }
            session["membership_role"] = "owner"; session["membership_revision"] = "1"; session["credential_revision"] = "1"
            session["observed_at"] = String(Int64(Date().timeIntervalSince1970 * 1000))
            if fault == .reconciliation { session["subject"] = "wrong-principal" }
            return (try canonical(session), 200)
        default:
            if path.hasPrefix("/v1/onboarding/operations/") { return (try canonical(["schema": "photara.onboarding.v1", "code": "authentication-required"]), 401) }
            preconditionFailure("Unexpected synthetic HTTP boundary")
        }
    }
}

@MainActor
private final class SyntheticCredentials: NativeEnrollmentCredentialStore {
    var items: [String: NativeStoredCredential] = [:]
    var namespaces: [String: NativeCredentialNamespace] = [:]
    var failInsert = false, failRemove = false, failReplace = false
    var insertions = 0, removals = 0
    var onInsert: (() -> Void)?
    func insert(_ value: NativeStoredCredential, namespace: NativeCredentialNamespace) throws {
        if failInsert { throw NativeAuthenticationError.keychainUnavailable }
        precondition(namespace.reference != nil && items[namespace.reference!] == nil)
        items[namespace.reference!] = value
        namespaces[namespace.reference!] = namespace
        insertions += 1
        onInsert?()
    }
    func read(_ namespace: NativeCredentialNamespace) throws -> NativeStoredCredential? { items[namespace.reference!] }
    func replace(_ value: NativeStoredCredential, namespace: NativeCredentialNamespace) throws {
        if failReplace { throw NativeAuthenticationError.keychainUnavailable }
        items[namespace.reference!] = value
    }
    func recoveryNamespace(reference: String, configuration: ReleaseConfiguration) throws -> NativeCredentialNamespace {
        guard let namespace = namespaces[reference] else { throw NativeAuthenticationError.invalidCredential }
        return namespace
    }
    func removePrepared(reference: String, configuration: ReleaseConfiguration) throws {
        if failRemove { throw NativeAuthenticationError.keychainUnavailable }
        items.removeValue(forKey: reference)
        removals += 1
    }
}

@MainActor
private final class SyntheticBrowserSession: NativeAuthenticationSession {
    let completion: @Sendable (URL?, Error?) -> Void
    let callback: URL
    var startResult = true, hold = false, fail = false
    var cancellations = 0
    init(url: URL, completion: @escaping @Sendable (URL?, Error?) -> Void) {
        self.completion = completion
        let items = URLComponents(url: url, resolvingAgainstBaseURL: false)!.queryItems!
        let values = Dictionary(uniqueKeysWithValues: items.map { ($0.name, $0.value!) })
        precondition(values["response_type"] == "code" && values["code_challenge_method"] == "S256")
        precondition(values["client_id"] == "synthetic-native" && values["connection"] == "google-oauth2")
        precondition(values["scope"] == "openid profile email offline_access photara:onboard" && values["max_age"] == "0")
        precondition(values["code_challenge"]?.count == 43 && values["nonce"]?.count == 43)
        precondition(values["redirect_uri"] == syntheticConfiguration.environment.callbackURL.absoluteString)
        callback = URL(string: values["redirect_uri"]! + "?state=" + values["state"]! + "&code=synthetic-code")!
    }
    func start() -> Bool { if startResult && !hold { complete() }; return startResult }
    func cancel() { cancellations += 1 }
    func complete() {
        let callback = callback, completion = completion, fail = fail
        // Same arbitrary executor boundary used by SafariLaunchAgent. All owner
        // and continuation handling runs through NativeAuthenticationCallback.
        DispatchQueue.global().async {
            if fail { completion(nil, NSError(domain: "synthetic-secret-must-not-escape", code: 999)) }
            else { completion(callback, nil) }
        }
    }
}

@MainActor
private final class Fixture {
    let directory: URL
    let service = SyntheticService()
    let credentials = SyntheticCredentials()
    var browserSession: SyntheticBrowserSession?
    var browserStarted: CheckedContinuation<Void, Never>?
    var holdBrowser = false, failBrowserStart = false, failBrowserCompletion = false, invalidCallback = false
    var failExchange = false, failSigning = false, failService = false
    var exchanges = 0
    var subject = syntheticSubject
    var holdExchange = false, changeLocalBeforeDispatch = false
    var exchangeArrived: CheckedContinuation<Void, Never>?
    var exchangeRelease: CheckedContinuation<Void, Never>?
    let installation = UUID()
    lazy var browser = NativeAuthenticationBrowser { [unowned self] url, scheme, ephemeral, _, callback in
        precondition(scheme == syntheticConfiguration.identity.callbackScheme && !ephemeral)
        let session = SyntheticBrowserSession(url: url, completion: callback)
        session.hold = self.holdBrowser; session.startResult = !self.failBrowserStart; session.fail = self.failBrowserCompletion
        self.browserSession = session
        self.browserStarted?.resume(); self.browserStarted = nil
        return session
    }
    init() throws {
        directory = URL(fileURLWithPath: "/private/tmp").appending(path: "photara-onboarding-synthetic-" + UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    }
    func dispose() throws { try FileManager.default.removeItem(at: directory) }
    func waitForBrowser() async {
        if browserSession != nil { return }
        await withCheckedContinuation { browserStarted = $0 }
    }
    func waitForExchange() async {
        if exchanges > 0 { return }
        await withCheckedContinuation { exchangeArrived = $0 }
    }
    func driver() -> ProductionOpeningCloudDriver {
        ProductionOpeningCloudDriver(configuration: syntheticConfiguration, dependencies: NativeOpeningDependencies(
            credentials: { if self.failSigning { throw NativeAuthenticationError.signingRequired }; return self.credentials },
            ensureService: { if self.failService { throw NativeAuthenticationError.transportUnavailable } },
            transport: service, supportDirectory: { self.directory }, installation: { self.installation },
            authenticate: { url, scheme in
                let callback = try await self.browser.authenticate(url: url, callbackScheme: scheme, window: nil, ephemeral: false)
                if self.invalidCallback { return URL(string: callback.absoluteString + "&state=duplicate")! }
                return callback
            },
            exchange: { code, attempt, _, now in
                self.exchanges += 1
                self.exchangeArrived?.resume(); self.exchangeArrived = nil
                if self.holdExchange { await withCheckedContinuation { self.exchangeRelease = $0 } }
                if self.changeLocalBeforeDispatch {
                    // A separate local writer changes the prepared revision.
                    // Dispatch must rerun the Rust preflight and reject it.
                    var database: OpaquePointer?
                    let path = self.directory.appending(path: "State/photara-local-v2.sqlite").path
                    precondition(sqlite3_open_v2(path, &database, SQLITE_OPEN_READWRITE, nil) == SQLITE_OK)
                    defer { sqlite3_close(database) }
                    precondition(sqlite3_exec(database, "UPDATE libraries SET display_name='Changed locally',local_revision=local_revision+1", nil, nil, nil) == SQLITE_OK)
                }
                precondition(attempt.consumed && code == "synthetic-code")
                precondition(attempt.challenge == AuthenticationEncoding.base64URL(Data(CryptoKit.SHA256.hash(data: Data(attempt.verifier.utf8)))))
                if self.failExchange { throw NSError(domain: "synthetic-provider-secret", code: 999) }
                return NativeAuthenticatedTokens(identity: NativeVerifiedIdentity(issuer: syntheticIssuer, subject: self.subject),
                    accessToken: "synthetic-access", refreshToken: "synthetic-refresh", enrollmentIDToken: "synthetic-id-token", expiresAt: now.addingTimeInterval(600),
                    profile: NativeAccountProfile(name: "Synthetic Person", email: "person@example.invalid", picture: nil))
            }))
    }
    func journal() -> NativeOnboardingJournal { NativeOnboardingJournal(path: directory.appending(path: "State/photara-local-v2.sqlite").path) }
    func pending(principal: Bool = false) async throws -> BridgeEnrollmentIntent? {
        let journal = journal(), local = try await journal.localState()
        let result = try await journal.pending(library: local.libraryId, environment: "synthetic",
            principal: principal ? BridgeEnrollmentPrincipal(issuer: syntheticIssuer, subject: syntheticSubject) : nil)
        try await journal.close()
        return result
    }
    func prepareAbandoned(submitted: Bool = false) async throws -> BridgeEnrollmentIntent {
        let journal = journal(), local = try await journal.localState()
        let intent = try await journal.prepare(.init(operationId: UUID().uuidString.lowercased(), libraryId: local.libraryId,
            localPrincipalId: local.principalId, environmentId: "synthetic", issuer: syntheticIssuer,
            credentialReference: NativeCredentialReference.create(), deviceCommitmentSha256: String(repeating: "a", count: 64), deviceDisplayName: "Synthetic"))
        if submitted {
            let who = BridgeEnrollmentPrincipal(issuer: syntheticIssuer, subject: syntheticSubject)
            let generation = try await journal.advance(who)
            _ = try await journal.dispatch(operation: intent.operationId, principal: who, generation: generation)
        }
        try await journal.close()
        return intent
    }
}

import CryptoKit

@main
struct NativeOnboardingFlowTests {
    @MainActor static func main() async throws {
        var assertions = 0
        func check(_ value: Bool, _ label: String) { precondition(value, label); assertions += 1 }
        func failure(_ driver: ProductionOpeningCloudDriver) async -> NativeOnboardingFailure {
            do { _ = try await driver.signIn { _ in }; preconditionFailure("Expected synthetic failure") }
            catch let error as NativeOnboardingFailure { return error }
            catch { preconditionFailure("Unsanitized failure escaped") }
        }
        let happy = try Fixture()
        let abandoned = try await happy.prepareAbandoned()
        happy.credentials.items[abandoned.credentialReference] = NativeStoredCredential(refreshToken: "synthetic-orphan", refreshInFlight: false, deviceCredential: String(repeating: "D", count: 43))
        let oldJournal = happy.journal()
        let oldLocal = try await oldJournal.localState()
        try await oldJournal.close()
        var states: [NativeOnboardingState] = []
        let success = try await happy.driver().signIn { states.append($0) }
        check(success == .cloudReady && states == [.authenticating, .bootstrapPending, .reconciling], "Complete production flow")
        check(happy.exchanges == 1 && happy.credentials.items.count == 1 && happy.credentials.items[abandoned.credentialReference] == nil, "Crash credential recovery")
        let finalJournal = happy.journal(), local = try await finalJournal.localState()
        check(local.libraryId == oldLocal.libraryId && local.authorityMode == "cloud-member", "Same library real Rust application")
        try await finalJournal.close()
        check(try await happy.pending(principal: true) == nil, "No unresolved success")
        let database = try Data(contentsOf: happy.directory.appending(path: "State/photara-local-v2.sqlite"))
        check(database.range(of: Data("synthetic-refresh".utf8)) == nil && database.range(of: Data("synthetic-access".utf8)) == nil && database.range(of: Data("synthetic-id-token".utf8)) == nil, "No tokens in journal")
        let originalCommand = await happy.service.command
        let reference = happy.credentials.items.keys.first!
        let device = happy.credentials.items[reference]!.deviceCredential
        let pathsBeforeLogout = await happy.service.paths
        check(try await happy.driver().accountProfile(refreshAvatar: false)?.name == "Synthetic Person", "Verified display profile cached")
        check(try await happy.driver().signOut() == .signedOutCached, "Explicit sign out")
        check(await happy.service.paths == pathsBeforeLogout, "Logout works offline without cloud requests")
        check(happy.credentials.items[reference]?.refreshToken == nil && happy.credentials.items[reference]?.deviceCredential == device, "Logout clears refresh but retains device")
        check(try await happy.driver().savedState() == .signedOutCached, "Signed out survives driver restart")
        check(try await happy.driver().accountProfile(refreshAvatar: false) == nil, "Signed out hides profile")
        happy.subject = "different-person"
        check(await failure(happy.driver()).reason == .differentAccount, "Another verified account cannot retarget library")
        check(happy.credentials.items[reference]?.refreshToken == nil, "Wrong account cannot replace retained credentials")
        happy.subject = syntheticSubject
        let returning = happy.driver()
        var choose: CheckedContinuation<String, Error>?
        var choices: [NativeLibraryChoice] = []
        returning.setLibraryPicker { available in
            choices = available
            return try await withCheckedThrowingContinuation { choose = $0 }
        }
        let rejoin = Task { try await returning.signIn { _ in } }
        while choose == nil { await Task.yield() }
        check(choices.count == 1 && choices[0].id == local.libraryId, "Picker offers exact existing library")
        check(try await happy.driver().savedState() == .signedOutCached, "Library does not reopen before selection")
        choose!.resume(returning: choices[0].id); choose = nil
        check(try await rejoin.value == .cloudReady, "Same Google identity opens selected library")
        check(await happy.service.command == originalCommand && happy.credentials.insertions == 1, "Rejoin creates no enrollment/device")
        check(happy.credentials.items.count == 1 && happy.credentials.items[reference]?.deviceCredential == device, "Device identity survives rejoin")
        check(try await happy.driver().savedState() == .cloudOffline, "Returning launch restores quiet session")
        // A cleanup failure must never resurrect local session authority.
        happy.credentials.failReplace = true
        do { _ = try await happy.driver().signOut(); preconditionFailure("Expected cleanup failure") }
        catch let error as NativeOnboardingFailure { check(error.state == .signedOutCached, "Failed Keychain cleanup retains durable logout") }
        happy.credentials.failReplace = false
        let cancelledPicker = happy.driver()
        cancelledPicker.setLibraryPicker { _ in throw NativeAuthenticationError.cancelled }
        check(await failure(cancelledPicker).state == .signedOutCached, "Cancelled library choice remains signed out")
        check(happy.credentials.items[reference]?.refreshToken == nil, "Retry cleans old refresh token before browser")
        let invalidPicker = happy.driver()
        invalidPicker.setLibraryPicker { _ in UUID().uuidString }
        check(await failure(invalidPicker).state == .signedOutCached, "Unknown library cannot be selected")
        check(try await happy.driver().signIn { _ in } == .cloudReady, "Repeated logout and login recovers same library")
        try happy.dispose()

        for phase in [NativeOnboardingPhase.signing, .service, .capabilities, .challenge, .browser, .callback, .tokenExchange, .credentialStorage, .bootstrap, .receipt, .session, .reconciliation] {
            let fixture = try Fixture()
            fixture.failSigning = phase == .signing; fixture.failService = phase == .service
            fixture.failBrowserStart = phase == .browser; fixture.invalidCallback = phase == .callback
            fixture.failExchange = phase == .tokenExchange; fixture.credentials.failInsert = phase == .credentialStorage
            await fixture.service.configure(fault: phase)
            let error = await failure(fixture.driver())
            check(error.phase == phase && !error.message.contains("synthetic"), "Safe phase diagnostic: " + phase.rawValue)
            let submitted = [.bootstrap, .receipt, .session, .reconciliation].contains(phase)
            check(error.submitted == submitted, "Submission classification")
            check(try await fixture.pending(principal: submitted) == nil || submitted, "Awaited pre-dispatch cleanup")
            check(fixture.credentials.items.count == (submitted ? 1 : 0), "Credential retention matches journal")
            if submitted {
                let before = try await fixture.pending(principal: true)!
                let next = await failure(fixture.driver())
                check(next.phase == .sessionRefresh, "Submitted recovery refuses fresh operation")
                check(try await fixture.pending(principal: true)?.commandCanonical == before.commandCanonical, "Submitted canonical evidence preserved")
            }
            try fixture.dispose()
        }
        for variant in ["expired", "overlong", "nondecimal", "nil-id", "noncanonical"] {
            let fixture = try Fixture()
            await fixture.service.configure(challengeVariant: variant)
            check(await failure(fixture.driver()).phase == .challenge, "Challenge bound: " + variant)
            check(fixture.browserSession == nil && fixture.exchanges == 0, "Invalid challenge never opens browser")
            try fixture.dispose()
        }
        let browserFailure = try Fixture(); browserFailure.failBrowserCompletion = true
        let browserError = await failure(browserFailure.driver())
        check(browserError.phase == .browser && browserError.reason == .failed, "Browser failure is not mislabeled cancellation")
        try browserFailure.dispose()

        let unknown = try Fixture(), retained = try await unknown.prepareAbandoned(submitted: true)
        check(await failure(unknown.driver()).phase == .sessionRefresh, "Restart preserves unknown")
        check(try await unknown.pending(principal: true)?.commandCanonical == retained.commandCanonical, "Unknown bytes unchanged")
        check(await unknown.service.paths == ["/v1/onboarding/capabilities"] && unknown.browserSession == nil, "Unknown blocks external side effects")
        try unknown.dispose()

        let cancellation = try Fixture(); cancellation.holdBrowser = true
        let driver = cancellation.driver()
        let task = Task { try await driver.signIn { _ in } }
        await cancellation.waitForBrowser()
        let oldSession = cancellation.browserSession!
        check(await failure(cancellation.driver()).reason == .busy, "Cross-driver lock excludes active recovery")
        task.cancel()
        do { _ = try await task.value; preconditionFailure("Cancelled browser succeeded") }
        catch let error as NativeOnboardingFailure { check(error.reason == .cancelled, "Browser cancellation classified") }
        check(oldSession.cancellations == 1 && cancellation.exchanges == 0, "Browser cancellation reaches session")
        check(try await cancellation.pending() == nil, "Cancellation cleanup complete before return")
        cancellation.browserSession = nil
        let retry = Task { try await driver.signIn { _ in } }
        await cancellation.waitForBrowser()
        oldSession.complete(); oldSession.complete()
        // Both old callbacks precede a sentinel on MainActor; then deliver the
        // active session. The old attempt must never resume the new one.
        await withCheckedContinuation { continuation in
            let handler = NativeAuthenticationCallback.handler { _, _ in continuation.resume() }
            DispatchQueue.global().async { handler(nil, nil) }
        }
        check(cancellation.exchanges == 0, "Late duplicate callbacks ignored")
        cancellation.browserSession!.complete()
        check(try await retry.value == .cloudReady && cancellation.exchanges == 1, "Retry completes after old callbacks")
        try cancellation.dispose()

        for path in ["/v1/onboarding/capabilities", "/v1/onboarding/challenges", "/v1/onboarding/bootstrap", "/v1/session"] {
            let fixture = try Fixture()
            await fixture.service.configure(hold: path)
            let task = Task { try await fixture.driver().signIn { _ in } }
            await fixture.service.waitFor(path)
            task.cancel()
            await fixture.service.release() // deliberately cancellation-uncooperative boundary
            do { _ = try await task.value; preconditionFailure("Cancelled HTTP succeeded") }
            catch let error as NativeOnboardingFailure {
                check(error.reason == .cancelled, "HTTP cancellation classified")
                let submitted = path == "/v1/onboarding/bootstrap" || path == "/v1/session"
                check(error.submitted == submitted, "HTTP cancellation dispatch boundary")
                if submitted {
                    check(try await fixture.pending(principal: true)?.state == "receipt-ready", "Late receipt retained without application")
                    let journal = fixture.journal()
                    check(try await journal.localState().authorityMode == "local-only", "Cancellation cannot apply cloud authority")
                    try await journal.close()
                } else { check(try await fixture.pending() == nil && fixture.exchanges == 0, "Pre-browser cancellation has no further side effects") }
            }
            try fixture.dispose()
        }
        let exchangeCancellation = try Fixture(); exchangeCancellation.holdExchange = true
        let exchangeTask = Task { try await exchangeCancellation.driver().signIn { _ in } }
        await exchangeCancellation.waitForExchange()
        exchangeTask.cancel()
        exchangeCancellation.exchangeRelease?.resume(); exchangeCancellation.exchangeRelease = nil
        do { _ = try await exchangeTask.value; preconditionFailure("Cancelled token exchange succeeded") }
        catch let error as NativeOnboardingFailure { check(error.reason == .cancelled && !error.submitted, "Token cancellation blocks storage/dispatch") }
        check(try await exchangeCancellation.pending() == nil && exchangeCancellation.credentials.insertions == 0, "No credentials or prepared intent after cancelled exchange")
        try exchangeCancellation.dispose()

        let insertCancellation = try Fixture()
        var insertTask: Task<NativeOnboardingState, Error>?
        insertCancellation.credentials.onInsert = { insertTask?.cancel() }
        insertTask = Task { try await insertCancellation.driver().signIn { _ in } }
        do { _ = try await insertTask!.value; preconditionFailure("Cancelled storage succeeded") }
        catch let error as NativeOnboardingFailure { check(error.reason == .cancelled && !error.submitted, "Cancellation after insert blocks dispatch") }
        check(try await insertCancellation.pending() == nil && insertCancellation.credentials.items.isEmpty, "Inserted credential rolls back on cancellation")
        insertCancellation.credentials.onInsert = nil
        try insertCancellation.dispose()

        let changed = try Fixture(); changed.changeLocalBeforeDispatch = true
        let changedError = await failure(changed.driver())
        check(changedError.phase == .dispatch && !changedError.submitted, "Dispatch failure confirms prepared journal before cleanup")
        check(try await changed.pending() == nil && changed.credentials.items.isEmpty, "Local mutation does not strand prepared attempt")
        check(await changed.service.command == nil, "Failed dispatch never calls bootstrap")
        try changed.dispose()

        let cleanup = try Fixture(); cleanup.failExchange = true; cleanup.credentials.failRemove = true
        check(await failure(cleanup.driver()).phase == .cleanup, "Cleanup failure is visible")
        check(try await cleanup.pending()?.state == "prepared", "Failed cleanup keeps recovery coordinate")
        cleanup.credentials.failRemove = false; cleanup.failExchange = false
        check(try await cleanup.driver().signIn { _ in } == .cloudReady, "Retry recovers failed cleanup")
        try cleanup.dispose()
        print("native-onboarding-flow: \(assertions) production-boundary assertions passed (real Rust journal; synthetic external boundaries)")
    }
}
