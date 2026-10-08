import Foundation

/// Executes the real generated FFI checksum checks and controlled constructor.
/// The deliberately invalid scope must refuse before any project effects.
@main
struct ControlledBridgeAdmissionSmoke {
    static func main() {
        do {
            _ = try ControlledDisposableSession.open(
                manifestPath: "/not-an-approved-disposable/manifest.json", bindingJson: "{}")
            preconditionFailure("Unregistered scope unexpectedly admitted")
        } catch ControlledSessionError.AdmissionRefused {
            print("PASS: exact generated Swift/library ABI and typed unregistered-scope refusal")
        } catch {
            preconditionFailure("Unexpected controlled admission result: \(error)")
        }
        do {
            _ = try ControlledDisposableWorkspace.open(
                manifestPath: "/not-an-approved-disposable/manifest.json", bindingJson: "{}", targetBindingsJson: "{}")
            preconditionFailure("Unregistered activation scope unexpectedly admitted")
        } catch ControlledSessionError.AdmissionRefused {
            print("PASS: typed unregistered workspace-scope refusal")
        } catch {
            preconditionFailure("Unexpected controlled workspace result: \(error)")
        }
    }
}
