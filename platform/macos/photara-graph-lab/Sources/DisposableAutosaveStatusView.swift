import SwiftUI

/// Controlled-session presentation; no simulated Saved state.
/// The shared adapter owns the projection and exact retained-request retry.
struct DisposableAutosaveStatusView: View {
    let projection: DisposableAutosaveProjection
    let retry: () -> Void

    var body: some View {
        HStack {
            Text(projection.status.label)
                .accessibilityIdentifier("disposable-autosave-status")
            if let revision = projection.snapshot?.accepted.revision {
                Text("Revision \(revision)").foregroundStyle(.secondary)
            }
            if case .failed(let message) = projection.status {
                Text(message).foregroundStyle(.secondary)
                Button("Retry", action: retry)
            }
        }
        .accessibilityElement(children: .contain)
    }
}
