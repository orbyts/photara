import Foundation
import Security
import SQLite3

private func json(_ value: Any) throws -> Data { try JSONSerialization.data(withJSONObject: value, options: [.sortedKeys, .withoutEscapingSlashes]) }

/// Only synthetic credentials are persisted in this disposable process fixture.
/// Production uses the separately tested Data Protection Keychain implementation.
@MainActor private final class FixtureCredentials: NativeEnrollmentCredentialStore {
    struct Item: Codable { let account: String; var value: NativeStoredCredential }
    let path: URL
    var items: [String: Item]
    init(_ root: URL) throws {
        path = root.appending(path: "synthetic-credentials.json")
        items = FileManager.default.fileExists(atPath: path.path) ? try JSONDecoder().decode([String: Item].self, from: Data(contentsOf: path)) : [:]
    }
    func save() throws { try JSONEncoder().encode(items).write(to: path, options: .atomic) }
    func insert(_ value: NativeStoredCredential, namespace: NativeCredentialNamespace) throws {
        precondition(items[namespace.reference!] == nil)
        items[namespace.reference!] = Item(account: namespace.account, value: value); try save()
    }
    func read(_ namespace: NativeCredentialNamespace) throws -> NativeStoredCredential? {
        guard let item = items[namespace.reference!], item.account == namespace.account else { return nil }
        return item.value
    }
    func replace(_ value: NativeStoredCredential, namespace: NativeCredentialNamespace) throws {
        precondition(items[namespace.reference!]?.account == namespace.account)
        items[namespace.reference!]!.value = value; try save()
    }
    func removePrepared(reference: String, configuration: ReleaseConfiguration) throws { items.removeValue(forKey: reference); try save() }
    func recoveryNamespace(reference: String, configuration: ReleaseConfiguration) throws -> NativeCredentialNamespace {
        try NativeCredentialNamespace(recoveryAccount: items[reference]!.account, reference: reference, configuration: configuration)
    }
}

/// Every bootstrap, operation lookup and session request reaches the real Rust
/// router. This shim can lose a real response or configure a test-only storage
/// checkpoint; it never manufactures a receipt, lookup result or bootstrap reply.
private actor FixtureTransport: NativeAuthenticationHTTP {
    let base: NativePinnedTransport
    let directory: URL
    var mode: String
    var posts = 0, lookups = 0, requests = 0
    var original: Data?
    var replacements: [Data] = []
    init(origin: URL, directory: URL, mode: String) throws {
        base = try NativePinnedTransport(furnaceOrigin: origin, timeout: 3)
        self.directory = directory; self.mode = mode
        let file = directory.appending(path: "original-command.json")
        if FileManager.default.fileExists(atPath: file.path) { original = try Data(contentsOf: file) }
    }
    func fault(_ phase: Int) async throws {
        let response = try await base.send(path: "/__furnace/fault", method: "POST", body: Data(String(phase).utf8), headers: ["Content-Type": "application/json"])
        precondition(response.1 == 200)
    }
    func recoverMode() async throws { mode = "recover"; try await fault(0) }
    func originalSession() async throws -> Data {
        var headers = try JSONDecoder().decode([String: String].self, from: Data(contentsOf: directory.appending(path: "synthetic-original-headers.json")))
        let command = try JSONSerialization.jsonObject(with: original!) as! [String: Any]
        headers["photara-device-id"] = command["device_id"] as? String
        let result = try await base.send(path: "/v1/session", method: "GET", body: nil, headers: headers)
        precondition(result.1 == 200)
        return result.0
    }
    func lateOriginal() async throws -> Data {
        let headers = try JSONDecoder().decode([String: String].self, from: Data(contentsOf: directory.appending(path: "synthetic-original-headers.json")))
        let result = try await base.send(path: "/v1/onboarding/bootstrap", method: "POST", body: original, headers: headers)
        precondition(result.1 == 200, "Late original converges on existing ownership")
        return result.0
    }
    func send(path: String, method: String, body: Data?, headers: [String: String]) async throws -> (Data, Int) {
        requests += 1
        var headers = headers
        if mode == "unauthorized", path != "/v1/onboarding/capabilities", path != "/v1/onboarding/challenges" {
            headers["Authorization"] = "Bearer synthetic-invalid-token"
        }
        if path.hasPrefix("/v1/onboarding/operations/") { lookups += 1 }
        if path == "/v1/onboarding/bootstrap" {
            posts += 1
            if let original, original != body {
                precondition(mode.hasPrefix("expired-fresh"), "Only explicit terminal replacement may change operation")
                let old = try JSONSerialization.jsonObject(with: original) as! [String: Any]
                let fresh = try JSONSerialization.jsonObject(with: body!) as! [String: Any]
                precondition(old["operation_id"] as! String != fresh["operation_id"] as! String)
                for field in ["requested_library_id", "device_id", "device_credential_sha256", "environment_id"] {
                    precondition(old[field] as! String == fresh[field] as! String, "Replacement preserves ownership coordinates")
                }
                if let first = replacements.first { precondition(first == body, "Replacement replay is exact") }
                replacements.append(body!)
                if mode == "expired-fresh-late-before", posts == 1 { _ = try await lateOriginal() }
            } else if original == nil {
                original = body; try body!.write(to: directory.appending(path: "original-command.json"))
                try JSONEncoder().encode(headers).write(to: directory.appending(path: "synthetic-original-headers.json"))
            }
            if mode == "commit-503" { try await fault(8) }
            if mode == "commit-timeout" { try await fault(10) }
            if mode == "rollback-503" || mode == "expired-start" || (mode == "not-found-replay" && posts == 1) { try await fault(7) }
        }
        if path == "/v1/session", mode == "expired-fresh-crash-receipt" { exit(75) }
        let response = try await base.send(path: path, method: method, body: body, headers: headers)
        if path == "/v1/onboarding/capabilities", mode == "stale-capabilities" {
            var object = try JSONSerialization.jsonObject(with: response.0) as! [String: Any]
            object.removeValue(forKey: "authentication_profile")
            return (try json(object), response.1)
        }
        if path == "/v1/onboarding/bootstrap", posts == 1 {
            if mode == "commit-503" || mode == "not-found-replay" { try await fault(0) }
            if mode == "crash-commit" || mode == "expired-fresh-crash-commit" { precondition(response.1 == 200); exit(73) }
            if mode == "lookup-503" { precondition(response.1 == 200); try await fault(9); throw NativeAuthenticationError.transportUnavailable }
            if mode == "lost-reply" { precondition(response.1 == 200); throw NativeAuthenticationError.transportUnavailable }
        }
        if path == "/v1/projects/create", mode == "ui1-crash" {
            precondition(response.1 == 200, "Real creation committed before simulated crash")
            exit(76)
        }
        return response
    }
    func token(subject: String, nonce: String) async throws -> (String, String) {
        let response = try await base.send(path: "/__furnace/token", method: "POST", body: json(["subject": subject, "nonce": nonce]), headers: ["Content-Type": "application/json"])
        precondition(response.1 == 200)
        let fields = try JSONSerialization.jsonObject(with: response.0) as! [String: String]
        return (fields["access"]!, fields["proof"]!)
    }
}

@main struct NativeServiceFlow {
    @MainActor static func main() async {
        do { try await run() }
        catch let failure as NativeOnboardingFailure { print("native-service-furnace: unexpected sanitized failure phase=\(failure.phase.rawValue) status=\(failure.httpStatus ?? 0)"); exit(1) }
        catch { print("native-service-furnace: unexpected failure; details suppressed"); exit(1) }
    }
    @MainActor static func run() async throws {
        let directory = URL(fileURLWithPath: CommandLine.arguments[1]), mode = CommandLine.arguments[2]
        precondition(directory.path.hasPrefix("/private/tmp/photara-native-service-case-"))
        let fields = try JSONSerialization.jsonObject(with: Data(contentsOf: directory.appending(path: "configuration.json"))) as! [String: Any]
        let now = Date(timeIntervalSince1970: Double(fields["now_ms"] as! Int64) / 1000)
        let origin = URL(string: fields["origin"] as! String)!, subject = fields["subject"] as! String
        let key = try NativeJWTEncoding.key(Data(base64Encoded: fields["public_key"] as! String)!)
        let env = ReleaseEnvironment(channel: .development, environmentId: "synthetic", apiOrigin: origin,
            auth0Issuer: URL(string: "https://issuer.invalid/")!, auth0Audience: "synthetic-api", nativeClientId: "synthetic-native",
            callbackURL: URL(string: "com.synthetic.desktop://issuer.invalid/macos/com.synthetic.desktop/callback")!,
            logoutURL: URL(string: "com.synthetic.desktop://issuer.invalid/macos/com.synthetic.desktop/callback")!,
            schemaFamily: "photara.service.g2", schemaEpoch: 1, minimumAPI: 3, loggingPolicy: "redacted-status-only", telemetryEnabled: false)
        let identity = ProductIdentity(displayName: "Synthetic", shortName: "Synthetic", isCodename: true,
            bundleIdentifier: "com.synthetic.desktop", keychainService: "com.synthetic.auth", callbackScheme: "com.synthetic.desktop",
            projectPackageDisplayType: "Synthetic", projectPackageExtension: "synthetic", apiAudienceNamespace: "synthetic-api",
            serviceHostname: nil, userAgent: "Synthetic/1", applicationSupportDirectory: "Synthetic", cacheDirectory: "Synthetic", defaultLibraryName: "My Library",
            websiteURL: nil, supportURL: nil, privacyURL: nil, storeURL: nil)
        let configuration = ReleaseConfiguration(identity: identity, environment: env)
        let installationFile = directory.appending(path: "installation.txt")
        let installation: UUID
        if FileManager.default.fileExists(atPath: installationFile.path) { installation = UUID(uuidString: try String(contentsOf: installationFile, encoding: .utf8))! }
        else { installation = UUID(); try installation.uuidString.write(to: installationFile, atomically: true, encoding: .utf8) }
        let credentials = try FixtureCredentials(directory)
        let transport = try FixtureTransport(origin: origin, directory: directory, mode: mode)
        var browsers = 0, exchanges = 0, refreshes = 0, assertions = 0, credentialOpens = 0
        var releaseBrowser = false
        var loginSubject = subject
        func check(_ value: Bool, _ label: String) { precondition(value, label); assertions += 1 }
        func driver() -> ProductionOpeningCloudDriver {
            ProductionOpeningCloudDriver(configuration: configuration, dependencies: NativeOpeningDependencies(
                credentials: { credentialOpens += 1; return credentials }, ensureService: {}, transport: transport, supportDirectory: { directory }, installation: { installation },
                authenticate: { url, _ in
                    browsers += 1
                    if mode == "expired-prepared" { exit(74) }
                    if mode == "expired-cancel" { throw NativeAuthenticationError.cancelled }
                    if mode.hasPrefix("expired-fresh") { while !releaseBrowser { await Task.yield() } }
                    let values = URLComponents(url: url, resolvingAgainstBaseURL: false)!.queryItems!
                    let state = values.first { $0.name == "state" }!.value!
                    return URL(string: env.callbackURL.absoluteString + "?code=synthetic-code&state=" + state)!
                }, exchange: { _, attempt, started, _ in
                    exchanges += 1
                    let tokens = try await transport.token(subject: loginSubject, nonce: attempt.nonce)
                    let verified = try NativeIDTokenVerifier(issuer: env.auth0Issuer.absoluteString, clientID: env.nativeClientId)
                        .verify(tokens.1, accessToken: tokens.0, attempt: attempt, key: key, expectedKeyID: "furnace", now: now, enrollmentChallengeCreatedAt: started)
                    return NativeAuthenticatedTokens(identity: verified, accessToken: tokens.0, refreshToken: "synthetic-refresh-0", enrollmentIDToken: tokens.1, expiresAt: now.addingTimeInterval(600))
                }, now: { now }, refresh: { _ in
                    refreshes += 1
                    let tokens = try await transport.token(subject: subject, nonce: "not-an-enrollment-proof")
                    return NativeRefreshResult(accessToken: tokens.0, refreshToken: "synthetic-refresh-" + UUID().uuidString)
                }, verifyAccess: { token, _ in try NativeAccessToken.verify(token, environment: env, key: key, keyID: "furnace", now: now) }))
        }
        func pending() async throws -> BridgeEnrollmentIntent? {
            let journal = NativeOnboardingJournal(path: directory.appending(path: "State/photara-local-v2.sqlite").path)
            let local = try await journal.localState()
            let result = try await journal.pending(library: local.libraryId, environment: "synthetic", principal: .init(issuer: env.auth0Issuer.absoluteString, subject: subject))
            try await journal.close(); return result
        }
        let passive = try await driver().savedState()
        check(browsers == 0 && exchanges == 0 && refreshes == 0 && credentialOpens == 0, "Launch reads only local metadata")
        check(await transport.requests == 0, "Launch makes no service requests")
        if mode.hasPrefix("ui1-") {
            let journal = NativeProjectCreationJournal(path: directory.appending(path: "State/photara-local-v2.sqlite").path)
            let operation: BridgeProjectCreation
            if mode == "ui1-crash" {
                check(try await driver().signIn { _ in } == .cloudReady, "UI1 signs into disposable Library")
                let destination = directory.appending(path: "Projects")
                try FileManager.default.createDirectory(at: destination, withIntermediateDirectories: true)
                let prepared = try await journal.prepare(operation: UUID().uuidString.lowercased(), title: "Furnace Project", destination: destination, destinationPin: try inspectCreationDestination(path: destination.path))
                operation = try await journal.advance(prepared.operationId)
                check(operation.state == "staged", "Cloud package waits privately for service")
            } else {
                operation = try await journal.operations().first!
                check(operation.state == "dispatching", "Restart retains unknown service outcome")
                check(browsers == 0, "Creation restart opens no browser")
            }
            try await driver().projectCreation(operation, journal: journal)
            let completed = try await journal.advance(operation.operationId)
            check(completed.state == "complete", "Real service, package and SQLite complete")
            let replay = try await journal.advance(operation.operationId)
            check(replay.projectId == completed.projectId && replay.graphId == completed.graphId && replay.commitSha256 == completed.commitSha256, "Exact Project, Graph and commit survive replay")
            check(try await journal.operations().count == 1, "One retained creation")
            try json(["project":completed.projectId,"graph":completed.graphId,"library":completed.libraryId,"commit_sha256":completed.commitSha256,"package":completed.packagePath,"device":completed.deviceId]).write(to: directory.appending(path: "creation-result.json"))
            print("native-service-furnace: \(mode): \(assertions) Swift assertions passed")
            return
        }
        if mode.hasPrefix("accounts") {
            if mode == "accounts" {
                check(passive == .localReady, "First account launch offers sign in")
                let current = driver()
                var offered = 0
                current.setLibraryPicker { choices in
                    check(choices.count == 1, "Real service offers one cloud default")
                    offered += 1
                    return choices[0].id
                }
                check(try await current.signIn { _ in } == .cloudReady && offered == 1, "First sign in requires library selection")
                let before = await transport.requests
                check(try await current.signOut() == .signedOutCached, "Explicit offline logout")
                check(await transport.requests == before, "Logout makes no service mutations")
                check(credentials.items.count == 1 && credentials.items.values.first!.value.refreshToken == nil, "Only refresh token cleared")
            } else if mode == "accounts-rejoin" {
                check(passive == .signedOutCached, "Logout survives a real process restart")
                let original = try JSONSerialization.jsonObject(with: await transport.original!) as! [String: Any]
                let reference = credentials.items.keys.first!, device = credentials.items[reference]!.value.deviceCredential
                loginSubject = "synthetic-wrong-" + subject
                do { _ = try await driver().signIn { _ in }; preconditionFailure("Wrong account reopened library") }
                catch let failure as NativeOnboardingFailure { check(failure.reason == .differentAccount && failure.state == .signedOutCached, "Verified different account rejected") }
                loginSubject = subject
                let cancelled = driver()
                cancelled.setLibraryPicker { _ in throw NativeAuthenticationError.cancelled }
                do { _ = try await cancelled.signIn { _ in }; preconditionFailure("Cancelled picker reopened library") }
                catch let failure as NativeOnboardingFailure { check(failure.state == .signedOutCached, "Cancelled picker leaves library signed out") }
                check(credentials.items[reference]!.value.refreshToken == nil, "No usable refresh token after cancellation")
                let current = driver()
                var selections = 0
                current.setLibraryPicker { choices in
                    check(choices.count == 1 && choices[0].id == original["requested_library_id"] as! String, "Picker points to same real library")
                    selections += 1
                    return choices[0].id
                }
                for _ in 0..<2 {
                    check(try await current.signIn { _ in } == .cloudReady, "Same account reopens selected library")
                    if selections == 1 { check(try await current.signOut() == .signedOutCached, "Repeated logout") }
                }
                check(selections == 2, "Every deliberate login chooses a library")
                check(credentials.items.count == 1 && credentials.items[reference]!.value.deviceCredential == device, "Stable device credential across cycles")
                let posts = await transport.posts, lookups = await transport.lookups
                check(posts == 0 && lookups == 0, "Rejoin never creates another enrollment")
                check(try await pending() == nil && refreshes == 0, "Rejoin uses deliberate authentication")
            } else {
                check(passive == .cloudOffline, "Selected library survives process restart quietly")
                _ = try await driver().accountProfile(refreshAvatar: false)
                check(browsers == 0 && exchanges == 0 && refreshes == 0 && credentialOpens == 0, "Returning launch has no authentication work")
                check(await transport.requests == 0, "Returning profile reads only local display cache")
            }
        } else if mode == "expired-retire" {
            check(passive == .outcomeUnknown, "Restart offers deliberate recovery")
            try await transport.recoverMode()
            check(try await driver().recoverIfNeeded { _ in } == .freshSignInRequired, "Confirmed absence and lost proof retires attempt")
            check(try await pending() == nil, "Terminal operation releases pending slot")
            check(try await driver().savedState() == .freshSignInRequired, "Terminal state survives another driver")
            check(browsers == 0 && exchanges == 0 && credentials.items.count == 1, "Terminal retains credential without a browser")
        } else if mode == "expired-prepared" || mode == "expired-cancel" {
            check(passive == .freshSignInRequired, "Restart exposes explicit sign-in again")
            let reference = credentials.items.keys.first!
            do { _ = try await driver().signIn { _ in }; preconditionFailure("Expected cancellation or process exit") }
            catch { check(mode == "expired-cancel", "Expected cancellation") }
            check(credentials.items[reference] != nil && credentials.items.count == 1, "Cancelled replacement retains original credential")
            check(try await pending() == nil, "Cancelled prepared replacement releases slot")
        } else if mode.hasPrefix("expired-fresh") {
            check(passive == (mode == "expired-fresh-inline" ? .outcomeUnknown : .freshSignInRequired), "Restart stays passive")
            let current = driver()
            let first = Task { try await current.signIn { _ in } }
            while browsers == 0 { await Task.yield() }
            do { _ = try await current.signIn { _ in }; preconditionFailure("Concurrent click accepted") }
            catch let error as NativeOnboardingFailure { check(error.reason == .busy, "Concurrent click cannot create another authorization") }
            releaseBrowser = true
            check(try await first.value == .cloudReady, "Fresh proof binds original library")
            check(browsers == 1 && exchanges == 1 && refreshes == (mode == "expired-fresh-inline" ? 1 : 0), "Exactly one fresh authorization")
            check(credentials.items.count == 1, "Original credential reference retained")
            check(try await driver().savedState() == .cloudOffline, "Returning launch quietly retains session")
            let original = try JSONSerialization.jsonObject(with: await transport.original!) as! [String: Any]
            let operation = original["operation_id"] as! String
            let journal = NativeOnboardingJournal(path: directory.appending(path: "State/photara-local-v2.sqlite").path)
            let principal = BridgeEnrollmentPrincipal(issuer: env.auth0Issuer.absoluteString, subject: subject)
            if mode.contains("late-") {
                let late = try await transport.lateOriginal()
                try await journal.record(operation: operation, receipt: late)
                let generation = try await journal.advance(principal)
                do { _ = try await journal.apply(operation: operation, principal: principal, generation: generation, session: try await transport.originalSession()); preconditionFailure("Terminal receipt changed selection") }
                catch { check(true, "Late terminal receipt cannot become active") }
            }
            let local = try await journal.localState()
            check(local.libraryId.lowercased() == (original["requested_library_id"] as! String).lowercased(), "No second local library")
            try await journal.close()
            check(try await pending() == nil, "Replacement applied without reviving original")
        } else {
        let failureModes = ["rollback-503", "lookup-503", "unauthorized", "stale-capabilities", "expired-start"]
        if failureModes.contains(mode) {
            do { _ = try await driver().signIn { _ in }; preconditionFailure("Expected real boundary failure") }
            catch let error as NativeOnboardingFailure {
                let phase: NativeOnboardingPhase = (mode == "rollback-503" || mode == "expired-start") ? .operationReplay : mode == "lookup-503" ? .operationLookup : mode == "unauthorized" ? .bootstrap : .capabilities
                check(error.phase == phase, "Correct real HTTP failure phase")
                if mode != "stale-capabilities" { check(error.httpStatus == (mode == "unauthorized" ? 401 : 503), "Retained HTTP status") }
            }
            if mode == "stale-capabilities" { check(browsers == 0 && credentials.items.isEmpty, "Stale service blocks before authentication") }
            else {
                let before = try await pending()!
                check(before.state == "unknown", "Unknown evidence survives real failure")
                check(credentials.items.count == 1, "Credential retained")
                if mode == "expired-start" {
                    print("native-service-furnace: \(mode): \(assertions) Swift assertions passed"); return
                }
                try await transport.recoverMode()
                if mode == "lookup-503" {
                    check(try await driver().recoverIfNeeded { _ in } == .cloudReady, "Explicit recovery after transient lookup failure")
                } else {
                    check(try await driver().recoverIfNeeded { _ in } == .freshSignInRequired, "Confirmed absent operation is terminal")
                    check(try await pending() == nil, "Terminal operation releases pending slot")
                    check(await transport.original == before.commandCanonical, "Absent replay keeps exact canonical operation")
                }
                check(browsers == 1 && exchanges == 1 && refreshes == 1, "Recovery uses retained session without browser")
            }
        } else {
            let result: NativeOnboardingState?
            if mode == "recover" { result = try await driver().recoverIfNeeded { _ in } }
            else { result = try await driver().signIn { _ in } }
            check(result == .cloudReady, "Real service to local cloud binding")
            check(try await pending() == nil, "Original operation applied")
            if mode == "recover" { check(browsers == 0 && exchanges == 0 && refreshes == 1, "Restart uses retained credential only") }
            if mode != "normal" { check(await transport.lookups >= 1, "Recovery crosses real operation HTTP route") }
            check(await transport.posts == (mode == "recover" ? 0 : mode == "not-found-replay" ? 2 : 1), "Bounded exact replay count")
        }
        }
        print("native-service-furnace: \(mode): \(assertions) Swift assertions passed")
    }
}
