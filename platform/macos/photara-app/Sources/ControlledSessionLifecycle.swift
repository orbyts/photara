import AppKit

@MainActor
final class ControlledSessionApplicationDelegate: NSObject, NSApplicationDelegate {
    private(set) var isTerminationPending = false
    weak var model: DisposableAutosaveProcess?
    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        guard let model else { return .terminateCancel }
        if model.closed { return .terminateNow }
        if isTerminationPending { return .terminateLater }
        isTerminationPending = true
        Task { @MainActor [self] in
            model.close { succeeded in
                self.isTerminationPending = false
                sender.reply(toApplicationShouldTerminate: succeeded)
            }
        }
        return .terminateLater
    }
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
}

