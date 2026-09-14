import AppKit
import SwiftUI

@MainActor
func require(_ condition: @autoclosure () -> Bool, _ message: String) {
  guard condition() else { fatalError(message) }
}

/// Native hosting snapshot, only in the test executable. No production screenshot mode.
@MainActor
func capture<V: View>(
  _ view: V, name: String, size: NSSize, directory: URL,
  inspect: (@MainActor (NSWindow, NSView) async throws -> Void)? = nil
) async throws {
  let host = NSHostingView(rootView: view)
  host.sizingOptions = []
  let window = NSWindow(
    contentRect: .init(origin: .zero, size: size),
    styleMask: [.titled, .resizable], backing: .buffered, defer: false)
  window.isReleasedWhenClosed = false
  window.contentView = host
  window.makeKeyAndOrderFront(nil)
  NSApp.activate(ignoringOtherApps: true)
  try await Task.sleep(for: .milliseconds(180))
  host.layoutSubtreeIfNeeded()
  do {
    window.setContentSize(size)
    host.layoutSubtreeIfNeeded()
    require(
      abs(host.bounds.height - size.height) < 1,
      "Rendered view escaped the requested fixture size: \(name)")
  }
  let rect = host.bounds
  require(rect.width > 0 && rect.height > 0, "Empty native view: \(name)")
  // Canvas and native glass are compositor layers; cacheDisplay omits them.
  var captured = false
  for _ in 0..<3 {
    window.orderFront(nil)
    let screenshot = Process()
    screenshot.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
    screenshot.arguments = [
      "-x", "-o", "-l", String(window.windowNumber),
      directory.appending(path: "\(name).png").path,
    ]
    try screenshot.run()
    while screenshot.isRunning { try await Task.sleep(for: .milliseconds(20)) }
    if screenshot.terminationStatus == 0 {
      captured = true
      break
    }
    try await Task.sleep(for: .milliseconds(180))
  }
  require(captured, "Native compositor capture failed: \(name)")
  try await inspect?(window, host)
  window.close()
}
