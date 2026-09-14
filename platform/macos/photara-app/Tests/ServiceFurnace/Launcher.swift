import Foundation
import Security

@main
struct Launcher {
    @MainActor static func main() async throws {
        let configURL = URL(fileURLWithPath: CommandLine.arguments[1])
        let raw = try JSONSerialization.jsonObject(with: Data(contentsOf: configURL)) as! [String: Any]
        let port = raw["port"] as! Int
        precondition(port != 8080)
        let current = ReleaseConfiguration.current.environment
        let environment = ReleaseEnvironment(channel: .development, environmentId: "synthetic-furnace",
            apiOrigin: URL(string: "http://127.0.0.1:\(port)/")!, auth0Issuer: URL(string: "https://synthetic.invalid/")!,
            auth0Audience: "urn:synthetic", nativeClientId: "synthetic", callbackURL: current.callbackURL, logoutURL: current.logoutURL,
            schemaFamily: current.schemaFamily, schemaEpoch: 1, minimumAPI: 3, loggingPolicy: current.loggingPolicy, telemetryEnabled: false)
        let controller = NativeDevelopmentService.furnace(environment: environment,
            operatorURL: URL(fileURLWithPath: CommandLine.arguments[2]),
            lockURL: URL(fileURLWithPath: raw["lock"] as! String), configurationURL: configURL,
            budget: .milliseconds(Int64((raw["budget"] as! Double) * 1000)))
        func emit(_ fields: [String: Any]) {
            let data = try! JSONSerialization.data(withJSONObject: fields, options: .sortedKeys)
            FileHandle.standardOutput.write(data + Data([10]))
        }
        while let command = await Task.detached(operation: { readLine() }).value {
            if command == "stop" {
                await controller.stopFixture()
                emit(["status": "stopped", "forcedStops": controller.forcedStops]); return
            }
            precondition(command == "ensure")
            let begin = ContinuousClock.now
            let task = Task { try await controller.ensureReady() }
            let cancellation = Task {
                while !Task.isCancelled {
                    if FileManager.default.fileExists(atPath: raw["cancel"] as! String) { task.cancel(); return }
                    try? await Task.sleep(for: .milliseconds(20))
                }
            }
            var status = "ready", diagnostic = "none"
            do { try await task.value }
            catch {
                status = task.isCancelled ? "cancelled" : "failed"
                switch error as? NativeAuthenticationError {
                case .signingRequired: diagnostic = "signing"
                case .invalidResponse: diagnostic = "protocol"
                case .transportUnavailable: diagnostic = "unavailable"
                default: diagnostic = "other"
                }
            }
            cancellation.cancel()
            let elapsed = begin.duration(to: .now)
            emit(["status": status, "diagnostic": diagnostic, "childStatus": controller.lastChildStatus, "childDiagnostic": controller.lastChildDiagnostic, "launches": controller.launches, "pid": controller.ownedPID ?? 0,
                  "forcedStops": controller.forcedStops, "seconds": Double(elapsed.components.seconds) + Double(elapsed.components.attoseconds) / 1e18])
        }
        await controller.stopFixture()
    }
}
