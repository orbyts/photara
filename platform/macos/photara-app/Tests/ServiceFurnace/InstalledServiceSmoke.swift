import Foundation
import Security

@main
struct InstalledServiceSmoke {
    @MainActor static func main() async throws {
        var assertions = 0
        func check(_ value: Bool, _ label: String) { precondition(value, label); assertions += 1 }
        let root = FileManager.default.homeDirectoryForCurrentUser
            .appending(path: ".local/share/photara-development-service/Photara Development Operator.app")
        let executable = root.appending(path: "Contents/MacOS/photara-development-service")
        let service = root.appending(path: "Contents/Resources/photara-service")
        try NativeDevelopmentService.verifyCode(root, identifier: "com.photara.operator.development")
        try NativeDevelopmentService.verifyCode(service, identifier: "photara-service")
        assertions += 2
        func run(_ url: URL, arguments: [String], environment: [String: String]) async throws -> (Data, Data, Int32) {
            let process = Process(), output = Pipe(), error = Pipe()
            process.executableURL = url; process.arguments = arguments; process.environment = environment
            process.standardInput = FileHandle.nullDevice; process.standardOutput = output; process.standardError = error
            try process.run()
            let deadline = ContinuousClock.now.advanced(by: .seconds(8))
            while process.isRunning && ContinuousClock.now < deadline { try await Task.sleep(for: .milliseconds(20)) }
            if process.isRunning { kill(process.processIdentifier, SIGKILL) }
            await Task.detached { process.waitUntilExit() }.value
            return (output.fileHandleForReading.readDataToEndOfFile(), error.fileHandleForReading.readDataToEndOfFile(), process.terminationStatus)
        }
        // Read/validate within the actual signed operator; values never leave it.
        let checked = try await run(executable, arguments: ["check"], environment: ["PATH": "/usr/bin:/bin"])
        check(checked.2 == 0 && checked.0 == Data("development-service:keychain-ready\n".utf8), "Installed operator Keychain access")
        let group = try NativeCredentialStore.declaredAccessGroup(bundleIdentifier: ReleaseConfiguration.current.identity.bundleIdentifier)
        let team = group.split(separator: ".").first!
        let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: "synthetic-denial-probe-" + UUID().uuidString,
            kSecAttrAccessGroup as String: String(team) + ".com.photara.operator.development",
            kSecUseDataProtectionKeychain as String: true, kSecAttrSynchronizable as String: false]
        check(SecItemCopyMatching(query as CFDictionary, nil) == errSecMissingEntitlement, "Desktop cannot query operator access group")
        // This factory cannot launch or stop anything; the existing shared
        // service is observed by the exact production readiness algorithm.
        let observer = try NativeDevelopmentService.observeInstalled(configuration: .current)
        for _ in 0..<3 { try await observer.ensureReady(); assertions += 1 }
        check(observer.launches == 0, "Existing service reused without duplicate launch")
        let transport = try NativePinnedTransport(origin: ReleaseConfiguration.current.environment.apiOrigin, developmentLoopback: true, timeout: 3)
        let live = try await transport.send(path: "/health/live", method: "GET", body: nil, headers: ["Accept": "application/json"])
        check(live.1 == 200 && live.0 == Data("{\"healthy\":true}".utf8), "Exact installed liveness protocol")
        // Run the actual embedded Rust binary in a credential-free child. The
        // production profile refuses a different port before DB/issuer startup.
        let refused = try await run(service, arguments: [], environment: ["PATH": "/usr/bin:/bin", "PHOTARA_RELEASE_CHANNEL": "development", "PORT": "18081"])
        check(refused.2 == 1 && refused.1 == Data("photara-service-startup:invalid-setting:PORT\n".utf8), "Installed binary refuses test-port injection before reading credentials")
        print("installed-service-smoke: \(assertions) assertions passed; signatures, operator Keychain access/isolation, real URLSession health/capabilities, zero launches, isolated fail-closed child lifecycle")
    }
}
