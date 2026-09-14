import Foundation

@main
struct ReleaseConfigurationTests {
    static func main() {
        let configuration = ReleaseConfiguration.current
        precondition(configuration.identity.displayName == "Photara")
        precondition(configuration.identity.bundleIdentifier == "com.photara.desktop")
        precondition(configuration.identity.projectPackageExtension == "photara")
        precondition(configuration.environment.channel == .development)
        precondition(configuration.environment.apiOrigin.absoluteString == "http://127.0.0.1:8080/")
        precondition(configuration.environment.nativeClientId == "CJ3hH2CUSkEk0vSbdvHMJLEqD4gYWGkW")
        precondition(configuration.environment.callbackURL.scheme == configuration.identity.callbackScheme)
        precondition(configuration.environment.minimumAPI == 3)
        precondition(!configuration.environment.telemetryEnabled)
        print("typed native release configuration: passed")
    }
}
