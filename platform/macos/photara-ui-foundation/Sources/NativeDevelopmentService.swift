import Foundation
import Security
import Darwin

/// Exact public protocol, shared by the startup gate and the enrollment driver.
enum NativeServiceCapabilities {
    private struct Value: Decodable {
        let schema, environment_id, service_origin, minimum_api, canonical_codec: String
        let max_body_bytes, max_token_bytes, challenge_ttl_seconds, authentication_profile: String
        let providers: [String]
    }
    static func validate(_ data: Data, environment: ReleaseEnvironment) throws {
        try AuthenticationJSON.validateCanonicalObject(data, keys: [
            "schema", "environment_id", "service_origin", "minimum_api", "canonical_codec",
            "max_body_bytes", "max_token_bytes", "challenge_ttl_seconds", "authentication_profile", "providers"], maximum: 65_536)
        let value = try JSONDecoder().decode(Value.self, from: data)
        guard value.schema == "photara.onboarding.v1", value.environment_id == environment.environmentId,
              value.service_origin == environment.apiOrigin.absoluteString, value.minimum_api == String(environment.minimumAPI),
              value.canonical_codec == "photara.canonical-json.v1", value.max_body_bytes == "65536",
              value.max_token_bytes == "16384", value.challenge_ttl_seconds == "300", value.providers == ["google"],
              value.authentication_profile == "auth0-rs256-api-userinfo-v1"
        else { throw NativeAuthenticationError.invalidResponse }
    }
}

/// No stdout/stderr from the operator is surfaced to UI. Only bounded, public
/// HTTP observations select readiness; child exit alone never decides it.
@MainActor
final class NativeDevelopmentService {
    private enum Observation { case absent, starting, ready, incompatible }
    private let environment: ReleaseEnvironment
    private let executable: URL
    private let service: URL
    private let lockURL: URL
    private let budget: Duration
    private let makeTransport: (TimeInterval) throws -> NativePinnedTransport
    private let childEnvironment: [String: String]
    private var owned: Process?
    private var launchAllowed = true
    private(set) var launches = 0
    private(set) var forcedStops = 0
    #if PHOTARA_SERVICE_FURNACE
    private var fixtureError: Pipe?
    private(set) var lastChildStatus: Int32 = 0
    private(set) var lastChildDiagnostic = "none"
    #endif

    private init(environment: ReleaseEnvironment, executable: URL, service: URL, lockURL: URL,
                 budget: Duration, childEnvironment: [String: String],
                 makeTransport: @escaping (TimeInterval) throws -> NativePinnedTransport) {
        self.environment = environment; self.executable = executable; self.service = service
        self.lockURL = lockURL; self.budget = budget; self.childEnvironment = childEnvironment
        self.makeTransport = makeTransport
    }

    static func installed(configuration: ReleaseConfiguration) throws -> NativeDevelopmentService {
        guard configuration.environment.channel == .development,
              configuration.environment.apiOrigin == ReleaseConfiguration.current.environment.apiOrigin
        else { throw NativeAuthenticationError.invalidResponse }
        let root = FileManager.default.homeDirectoryForCurrentUser
            .appending(path: ".local/share/photara-development-service/Photara Development Operator.app")
        let support = try FileManager.default.url(for: .applicationSupportDirectory, in: .userDomainMask,
            appropriateFor: nil, create: true).appending(path: configuration.identity.applicationSupportDirectory + "/State")
        try FileManager.default.createDirectory(at: support, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
        return NativeDevelopmentService(environment: configuration.environment,
            executable: root.appending(path: "Contents/MacOS/photara-development-service"),
            service: root.appending(path: "Contents/Resources/photara-service"),
            lockURL: support.appending(path: "development-service.lock"), budget: .seconds(25),
            childEnvironment: ["PATH": "/usr/bin:/bin"],
            makeTransport: { try NativePinnedTransport(origin: configuration.environment.apiOrigin, developmentLoopback: true, timeout: $0) })
    }

    #if PHOTARA_SERVICE_FURNACE
    static func observeInstalled(configuration: ReleaseConfiguration) throws -> NativeDevelopmentService {
        let controller = try installed(configuration: configuration)
        controller.launchAllowed = false
        return controller
    }
    /// Not compiled into the app. Tests use actual signed subprocesses, sockets
    /// and URLSession; only public fixture coordinates/configuration differ.
    static func furnace(environment: ReleaseEnvironment, operatorURL: URL, lockURL: URL,
                        configurationURL: URL, budget: Duration) -> NativeDevelopmentService {
        NativeDevelopmentService(environment: environment,
            executable: operatorURL.appending(path: "Contents/MacOS/photara-development-service"),
            service: operatorURL.appending(path: "Contents/Resources/photara-service"), lockURL: lockURL, budget: budget,
            childEnvironment: ["PATH": "/usr/bin:/bin", "PHOTARA_FURNACE_CONFIG": configurationURL.path],
            makeTransport: { try NativePinnedTransport(furnaceOrigin: environment.apiOrigin, timeout: $0) })
    }
    func stopFixture() async { await stopOwned() }
    var ownedPID: Int32? { owned?.processIdentifier }
    #endif

    func ensureReady() async throws {
        let deadline = ContinuousClock.now.advanced(by: budget)
        var launchLock: NativeOnboardingLock?
        var attemptedLaunch = false
        var launchedHere = false
        defer { _fixLifetime(launchLock) }
        do {
            while ContinuousClock.now < deadline {
                try Task.checkCancellation()
                let remaining = ContinuousClock.now.duration(to: deadline)
                let seconds = Double(remaining.components.seconds) + Double(remaining.components.attoseconds) / 1e18
                let observation = await probe(timeout: min(3, max(0.05, seconds / 2)))
                try Task.checkCancellation()
                switch observation {
                case .ready: return
                case .incompatible: throw NativeAuthenticationError.invalidResponse
                case .starting: break // A responding service must not be duplicated.
                case .absent:
                    if !launchAllowed { break }
                    if launchLock == nil {
                        do {
                            launchLock = try NativeOnboardingLock(url: lockURL)
                            continue // Always recheck after winning the cross-process lock.
                        } catch let error as NativeOnboardingFailure where error.reason == .busy {
                            // Another launcher owns startup; continue observing it.
                        }
                    } else if !attemptedLaunch && owned?.isRunning != true {
                        try Task.checkCancellation()
                        try verifyInstalledBinaries()
                        try Task.checkCancellation()
                        let process = Process()
                        process.executableURL = executable
                        process.arguments = ["run", service.path]
                        process.environment = childEnvironment
                        process.standardInput = FileHandle.nullDevice
                        process.standardOutput = FileHandle.nullDevice
                        process.standardError = FileHandle.nullDevice
                        #if PHOTARA_SERVICE_FURNACE
                        let error = Pipe()
                        fixtureError = error
                        process.standardError = error
                        #endif
                        try process.run()
                        owned = process; launches += 1; attemptedLaunch = true; launchedHere = true
                    }
                }
                // Keep checking even when our child exits: another process may
                // have won bind and still be warming up. Never relaunch in a loop.
                try await Task.sleep(for: .milliseconds(100))
            }
            throw NativeAuthenticationError.transportUnavailable
        } catch {
            // Only this call's new child is eligible for startup rollback.
            // A reused shared service is never terminated by this controller.
            if launchedHere { await stopOwned() }
            throw error
        }
    }

    private func probe(timeout: TimeInterval) async -> Observation {
        guard let transport = try? makeTransport(timeout) else { return .incompatible }
        // Parallel requests share one observation window. A readiness timeout
        // alongside valid capabilities means "starting", not "absent".
        async let health = try? transport.send(path: "/health/ready", method: "GET", body: nil, headers: ["Accept": "application/json"])
        async let capability = try? transport.send(path: "/v1/onboarding/capabilities", method: "GET", body: nil, headers: ["Accept": "application/json"])
        let (ready, document) = await (health, capability)
        if let document {
            guard document.1 == 200,
                  (try? NativeServiceCapabilities.validate(document.0, environment: environment)) != nil
            else { return .incompatible }
            guard let ready else { return .starting }
            if ready.1 == 503 { return .starting }
            return ready.1 == 200 && ready.0 == Data("{\"ready\":true}".utf8) ? .ready : .incompatible
        }
        return ready == nil ? .absent : .starting
    }

    private func verifyInstalledBinaries() throws {
        // Validate the installed bundle and exact embedded executable before
        // launching a process permitted to read the private operator Keychain.
        guard FileManager.default.isExecutableFile(atPath: executable.path),
              FileManager.default.isExecutableFile(atPath: service.path) else { throw NativeAuthenticationError.transportUnavailable }
        let bundle = executable.deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        try Self.verifyCode(bundle, identifier: "com.photara.operator.development")
        try Self.verifyCode(service, identifier: "photara-service")
    }

    static func verifyCode(_ url: URL, identifier: String) throws {
        // The team's exact identifier is obtained from the desktop's public
        // provisioning entitlement; signatures from another team are refused.
        guard let task = SecTaskCreateFromSelf(nil),
              let team = SecTaskCopyValueForEntitlement(task, "com.apple.developer.team-identifier" as CFString, nil) as? String,
              team.range(of: "^[A-Z0-9]{10}$", options: .regularExpression) != nil
        else { throw NativeAuthenticationError.signingRequired }
        var code: SecStaticCode?, requirement: SecRequirement?
        let text = "anchor apple generic and identifier \"\(identifier)\" and certificate leaf[subject.OU] = \"\(team)\""
        guard SecStaticCodeCreateWithPath(url as CFURL, [], &code) == errSecSuccess, let code,
              SecRequirementCreateWithString(text as CFString, [], &requirement) == errSecSuccess,
              SecStaticCodeCheckValidity(code, SecCSFlags(rawValue: kSecCSStrictValidate), requirement) == errSecSuccess
        else { throw NativeAuthenticationError.signingRequired }
    }

    private func stopOwned() async {
        guard let process = owned else { return }
        if process.isRunning {
            process.terminate()
            // Unstructured cleanup deliberately does not inherit task cancellation.
            await Task.detached {
                let limit = ContinuousClock.now.advanced(by: .seconds(1))
                while process.isRunning && ContinuousClock.now < limit {
                    try? await Task.sleep(for: .milliseconds(20))
                }
            }.value
            if process.isRunning { kill(process.processIdentifier, SIGKILL); forcedStops += 1 }
        }
        // Foundation's termination observation reaps our direct exec child.
        await Task.detached { process.waitUntilExit() }.value
        #if PHOTARA_SERVICE_FURNACE
        lastChildStatus = process.terminationStatus
        if let fixtureError {
            let data = fixtureError.fileHandleForReading.readDataToEndOfFile()
            let text = String(decoding: data, as: UTF8.self)
            // Explicit fixture diagnostic allowlist, never arbitrary stderr.
            for code in ["signing-required", "service-path-not-embedded", "unsafe-service-executable",
                         "keychain-item-missing", "keychain-access-blocked--34018", "furnace-config-required", "service-exec-failed"] {
                if text.contains("development-service:" + code + "\n") { lastChildDiagnostic = code }
            }
        }
        fixtureError = nil
        #endif
        owned = nil
    }
}
