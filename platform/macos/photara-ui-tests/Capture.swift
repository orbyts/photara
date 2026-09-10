import AppKit
import SwiftUI

@MainActor
func require(_ condition: @autoclosure () -> Bool, _ message: String) {
    guard condition() else { fatalError(message) }
}

/// Native hosting snapshot, only in the test executable. No production screenshot mode.
@MainActor
func capture<V: View>(_ view: V, name: String, size: NSSize, directory: URL) async throws {
    let host = NSHostingView(rootView: view)
    let window = NSWindow(contentRect: .init(origin: .zero, size: size),
                          styleMask: [.titled, .resizable], backing: .buffered, defer: false)
    window.isReleasedWhenClosed = false
    window.contentView = host
    window.orderFront(nil)
    try await Task.sleep(for: .milliseconds(180))
    host.layoutSubtreeIfNeeded()
    let rect = host.bounds
    require(rect.width > 0 && rect.height > 0, "Empty native view: \(name)")
    // Canvas and native glass are compositor layers; cacheDisplay omits them.
    let screenshot = Process()
    screenshot.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
    screenshot.arguments = ["-x", "-o", "-l", String(window.windowNumber),
                            directory.appending(path: "\(name).png").path]
    try screenshot.run()
    screenshot.waitUntilExit()
    require(screenshot.terminationStatus == 0, "Native compositor capture failed: \(name)")
    window.close()
}
