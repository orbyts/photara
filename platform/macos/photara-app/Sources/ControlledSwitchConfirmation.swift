import AppKit
import SwiftUI

/// Display data only; the Rust preparation token remains in the session model.
struct ControlledSwitchConfirmation: Identifiable, Equatable, Sendable {
    let id: UUID
    let sourceTitle: String
    let targetTitle: String
}

/// The model creates this presentation only after its current Saved barrier has
/// verified. No suppression preference; Escape cancels, Return switches.
struct ControlledSwitchConfirmationPresenter: NSViewRepresentable {
    let confirmation: ControlledSwitchConfirmation?
    let respond: (UUID, Bool) -> Void

    func makeCoordinator() -> Coordinator { Coordinator() }
    func makeNSView(context: Context) -> NSView { NSView() }
    func updateNSView(_ view: NSView, context: Context) {
        let coordinator = context.coordinator
        DispatchQueue.main.async { [weak view] in
            guard let window = view?.window else { return }
            coordinator.update(confirmation, window: window, respond: respond)
        }
    }

    @MainActor static func makeAlert(_ value: ControlledSwitchConfirmation) -> NSAlert {
        let alert = NSAlert()
        alert.messageText = "Switch to “\(value.targetTitle)”?"
        alert.informativeText = "Photara will close “\(value.sourceTitle)” and open “\(value.targetTitle).” Changes to “\(value.sourceTitle)” have been saved automatically."
        alert.addButton(withTitle: "Cancel").keyEquivalent = "\u{1b}"
        let confirm = alert.addButton(withTitle: "Switch Project")
        confirm.keyEquivalent = "\r"
        alert.window.defaultButtonCell = confirm.cell as? NSButtonCell
        return alert
    }

    @MainActor final class Coordinator {
        private var presented: UUID?
        private var alert: NSAlert?
        func update(_ value: ControlledSwitchConfirmation?, window: NSWindow,
                    respond: @escaping (UUID, Bool) -> Void) {
            guard let value else {
                if let alert, window.attachedSheet === alert.window {
                    window.endSheet(alert.window, returnCode: .abort)
                }
                return
            }
            guard presented == nil, window.attachedSheet == nil else { return }
            let alert = ControlledSwitchConfirmationPresenter.makeAlert(value)
            presented = value.id
            self.alert = alert
            alert.beginSheetModal(for: window) { [weak self] response in
                guard let self, self.presented == value.id else { return }
                self.presented = nil
                self.alert = nil
                respond(value.id, response == .alertSecondButtonReturn)
            }
        }
    }
}
