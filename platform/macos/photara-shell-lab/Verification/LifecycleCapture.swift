import AppKit
import SwiftUI

@MainActor
func require(_ condition: @autoclosure () -> Bool, _ message: String) {
    guard condition() else { fatalError(message) }
}

/// Native full-size-content host; SwiftUI owns safe-area and toolbar layout.
/// Sizes request a window content rectangle, not a fixed root-view geometry.
@MainActor
func capture<V: View>(_ view: V, name: String, size: NSSize, directory: URL, inspect: (@MainActor () async throws -> Void)? = nil) async throws {
    let host = NSHostingView(rootView: view)
    host.sizingOptions = []
    let window = NSWindow(contentRect: .init(origin: .zero, size: size),
        styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView],
        backing: .buffered, defer: false)
    window.isReleasedWhenClosed = false
    window.contentView = host
    window.center()
    window.makeKeyAndOrderFront(nil)
    NSApp.activate(ignoringOtherApps: true)
    try await Task.sleep(for: .milliseconds(600))
    host.layoutSubtreeIfNeeded()
    window.displayIfNeeded()
    require(host.bounds.width >= 480 && host.bounds.height >= 180, "Empty or collapsed native host: \(name)")
    require(window.isVisible, "Native host is not visible: \(name)")
    let process = Process()
    process.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
    process.arguments = ["-x", "-o", "-l", String(window.windowNumber), directory.appending(path: "\(name).png").path]
    try process.run()
    while process.isRunning { try await Task.sleep(for: .milliseconds(20)) }
    require(process.terminationStatus == 0, "Compositor capture failed: \(name)")
    print("CAPTURE \(name) window=\(window.frame.size) host=\(host.bounds.size) content=\(window.contentLayoutRect.size)")
    try await inspect?()
    window.close()
}
