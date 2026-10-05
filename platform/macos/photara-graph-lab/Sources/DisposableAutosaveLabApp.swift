import AppKit
import SwiftUI

@MainActor
final class DisposableAutosaveApplicationDelegate: NSObject, NSApplicationDelegate {
    weak var model: DisposableAutosaveProcess?
    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        guard let model, !model.closed else { return .terminateNow }
        Task { @MainActor in
            model.close { succeeded in sender.reply(toApplicationShouldTerminate: succeeded) }
        }
        return .terminateLater
    }
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
}

private struct DisposableWindowCloseGuard: NSViewRepresentable {
    let model: DisposableAutosaveProcess
    func makeCoordinator() -> Coordinator { Coordinator(model) }
    func makeNSView(context: Context) -> NSView { NSView() }
    func updateNSView(_ view: NSView, context: Context) {
        let coordinator = context.coordinator
        DispatchQueue.main.async { [weak view] in view?.window?.delegate = coordinator }
    }
    @MainActor final class Coordinator: NSObject, NSWindowDelegate {
        let model: DisposableAutosaveProcess
        init(_ model: DisposableAutosaveProcess) { self.model = model }
        func windowShouldClose(_ sender: NSWindow) -> Bool {
            if model.closed { return true }
            model.close { succeeded in if succeeded { sender.performClose(nil) } }
            return false
        }
    }
}

/// Explicit separate build entry within Graph Lab. It has no file picker or
/// production app route: only the controller-provisioned disposable Rust host.
@main
struct DisposableAutosaveLabApp: App {
    @NSApplicationDelegateAdaptor(DisposableAutosaveApplicationDelegate.self) private var delegate
    @StateObject private var model = DisposableAutosaveProcess()

    var body: some Scene {
        Window("Disposable Project — Autosave", id: "disposable-autosave") {
            VStack(alignment: .leading, spacing: 16) {
                Text("Disposable project position editor").font(.headline)
                Text("Changes save automatically in this disposable test project.")
                    .foregroundStyle(.secondary)
                if let projection = model.projection {
                    DisposableAutosaveStatusView(projection: projection, retry: model.retry)
                } else { Text("Opening the disposable project…") }
                if let error = model.transportFailure {
                    Text(error).foregroundStyle(.red)
                    Button("Retry original request", action: model.retry).disabled(model.busy)
                }
                if let pending = model.pendingPosition {
                    Text("Pending move: \(pending.title) to x \(pending.x), y \(pending.y). This draft has not been acknowledged.")
                        .accessibilityIdentifier("disposable-pending-position")
                }
                List(model.nodes) { node in
                    HStack {
                        VStack(alignment: .leading) {
                            Text(node.title)
                            Text("x \(node.x) · y \(node.y)").font(.caption).foregroundStyle(.secondary)
                        }
                        Spacer()
                        Button("←") { model.move(node, dx: -16, dy: 0) }.accessibilityLabel("Move \(node.title) left")
                        Button("→") { model.move(node, dx: 16, dy: 0) }.accessibilityLabel("Move \(node.title) right")
                        Button("↑") { model.move(node, dx: 0, dy: -16) }.accessibilityLabel("Move \(node.title) up")
                        Button("↓") { model.move(node, dx: 0, dy: 16) }.accessibilityLabel("Move \(node.title) down")
                    }.disabled(!model.canEdit)
                }
                HStack {
                    Button("Undo") { model.action("undo") }.keyboardShortcut("z").disabled(!model.canEdit || !model.undoAvailable)
                    Button("Redo") { model.action("redo") }.keyboardShortcut("z", modifiers: [.command, .shift]).disabled(!model.canEdit || !model.redoAvailable)
                    Spacer()
                    Button("Save Now") { model.action("complete") }.keyboardShortcut("s").disabled(!model.canEdit)
                    Button("Close") { NSApp.terminate(nil) }
                }
            }
            .padding(20).frame(minWidth: 620, minHeight: 380)
            .background(DisposableWindowCloseGuard(model: model).frame(width: 0, height: 0))
            .task { delegate.model = model; model.start() }
        }
    }
}
