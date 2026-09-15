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
  // Give compact fixtures their requested canvas within SwiftUI. Keep the host
  // directly attached to NSWindow so real SwiftUI toolbars remain installed.
  let host = NSHostingView(rootView: view.frame(width: size.width, height: size.height))
  host.sizingOptions = []
  let window = NSWindow(
    contentRect: .init(origin: .zero, size: size),
    styleMask: [.titled, .resizable], backing: .buffered, defer: false)
  window.isReleasedWhenClosed = false
  window.contentView = host
  window.makeKeyAndOrderFront(nil)
  NSApp.activate(ignoringOtherApps: true)
  try await Task.sleep(for: .milliseconds(180))
  // SwiftUI may install native navigation/toolbar constraints after the first
  // layout. Wait for the requested content size to survive successive native
  // layout turns. On macOS 27, setContentSize can exclude a different titlebar
  // inset than the mounted host (e.g. 240 requested becomes 230 actual). Adjust
  // the window frame by the measured host delta, preserving native chrome and
  // the exact requested fixture size instead of accepting a smaller snapshot.
  window.setContentSize(size)
  var stableLayouts = 0
  for _ in 0..<20 {
    host.layoutSubtreeIfNeeded()
    window.displayIfNeeded()
    try await Task.sleep(for: .milliseconds(40))
    host.layoutSubtreeIfNeeded()
    if abs(host.bounds.width - size.width) < 1 && abs(host.bounds.height - size.height) < 1 {
      stableLayouts += 1
      if stableLayouts == 2 { break }
    } else {
      stableLayouts = 0
      var frame = window.frame
      frame.size.width += size.width - host.bounds.width
      frame.size.height += size.height - host.bounds.height
      window.setFrame(frame, display: true)
    }
  }
  require(
    stableLayouts == 2,
    "Rendered view escaped the requested fixture size: \(name); requested \(size), host \(host.bounds), window \(window.frame), content \(window.contentLayoutRect), screen \(String(describing: window.screen?.visibleFrame))")
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
