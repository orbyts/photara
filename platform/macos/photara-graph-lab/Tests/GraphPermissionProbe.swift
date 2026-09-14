import AppKit
@preconcurrency import ApplicationServices
import ScreenCaptureKit

/// Exercise only a disposable window owned by this verifier. These are capability
/// diagnostics, never replacements for the original Graph behavioral assertions.
@MainActor
enum GraphPermissionProbe {
  static func run() async -> Bool {
    let watchdog = Task {
      try? await Task.sleep(for: .seconds(15))
      guard !Task.isCancelled else { return }
      GraphTestLog.write("PROBE: timed out after 15 seconds; no capability pass")
      GraphVerificationResult.recordExitCode(1)
      exit(1)
    }
    defer { watchdog.cancel() }
    let previous = NSApp.keyWindow
    let window = NSWindow(
      contentRect: NSRect(x: 120, y: 160, width: 320, height: 220),
      styleMask: [.titled], backing: .buffered, defer: false)
    window.title = "Graph permission probe"
    window.isReleasedWhenClosed = false
    let view = PermissionProbeView(frame: NSRect(x: 0, y: 0, width: 320, height: 220))
    let button = NSButton(title: "AX probe", target: view, action: #selector(view.pressed))
    button.frame = NSRect(x: 90, y: 150, width: 140, height: 32)
    button.bezelStyle = .rounded
    view.addSubview(button)
    window.contentView = view
    window.center()
    window.makeKeyAndOrderFront(nil)
    NSApp.activate(ignoringOtherApps: true)
    window.makeFirstResponder(view)
    defer {
      window.orderOut(nil)
      previous?.makeKeyAndOrderFront(nil)
    }
    for _ in 0..<100 {
      if NSApp.isActive && window.isKeyWindow { break }
      try? await Task.sleep(for: .milliseconds(10))
    }
    guard NSApp.isActive && window.isKeyWindow else {
      GraphTestLog.write("PROBE: disposable window could not acquire focus")
      return false
    }
    window.displayIfNeeded()
    try? await Task.sleep(for: .milliseconds(150))

    // This is the same public HID-posting route used by the unchanged native
    // slider tests, with a nonce proving receipt rather than a method call.
    let oldPointer = CGEvent(source: nil)?.location
    let source = CGEventSource(stateID: .hidSystemState)
    let positions = [CGPoint(x: 45, y: 40), CGPoint(x: 75, y: 40), CGPoint(x: 105, y: 40)]
    let types: [CGEventType] = [.leftMouseDown, .leftMouseDragged, .leftMouseUp]
    for (point, type) in zip(positions, types) {
      guard NSApp.isActive && window.isKeyWindow else { break }
      let native = window.convertPoint(toScreen: view.convert(point, to: nil))
      let global = CGPoint(x: native.x, y: NSScreen.screens[0].frame.height - native.y)
      let event = CGEvent(
        mouseEventSource: source, mouseType: type, mouseCursorPosition: global, mouseButton: .left)
      event?.setIntegerValueField(.eventSourceUserData, value: view.nonce)
      event?.setIntegerValueField(.mouseEventClickState, value: 1)
      event?.post(tap: .cghidEventTap)
      try? await Task.sleep(for: .milliseconds(40))
    }
    if let oldPointer {
      CGEvent(
        mouseEventSource: source, mouseType: .mouseMoved,
        mouseCursorPosition: oldPointer, mouseButton: .left)?.post(tap: .cghidEventTap)
    }
    try? await Task.sleep(for: .milliseconds(100))
    let hid = view.received == [.leftMouseDown, .leftMouseDragged, .leftMouseUp]
    GraphTestLog.write(
      "PROBE HID: received=\(view.received.map(\.rawValue)), passed=\(hid), postPreflight=\(CGPreflightPostEventAccess())"
    )

    // The AX return code is diagnostic: a nonzero AXError is still an API failure,
    // even if our own target observed a callback. Self-targeted AX dispatch does
    // not establish system input authorization and is not used by Graph gestures.
    // The gate uses the public authorization checks plus actual HID receipt.
    let native = window.convertPoint(toScreen: CGPoint(x: button.frame.midX, y: button.frame.midY))
    let axPoint = CGPoint(x: native.x, y: NSScreen.screens[0].frame.height - native.y)
    let pid = getpid()
    let axCodes = await Task.detached {
      let application = AXUIElementCreateApplication(pid)
      AXUIElementSetMessagingTimeout(application, 2)
      var element: AXUIElement?
      let lookup = AXUIElementCopyElementAtPosition(
        application, Float(axPoint.x), Float(axPoint.y), &element)
      let press = element.map { AXUIElementPerformAction($0, kAXPressAction as CFString) }
      return (lookup.rawValue, press?.rawValue)
    }.value
    try? await Task.sleep(for: .milliseconds(100))
    let ax = axCodes.0 == 0 && axCodes.1 == 0 && view.presses == 1
    GraphTestLog.write(
      "PROBE AX: lookup=\(axCodes.0), press=\(String(describing: axCodes.1)), observedActions=\(view.presses), apiSucceeded=\(ax), diagnosticOnly=true"
    )

    let directory = URL(
      fileURLWithPath: ProcessInfo.processInfo.environment["PHOTARA_GRAPH_EXIT_FILE"]
        ?? "/tmp/photara-graph-verification/result"
    )
    .deletingLastPathComponent()
    var screenKit = false
    do {
      let content = try await SCShareableContent.excludingDesktopWindows(
        true, onScreenWindowsOnly: true)
      guard
        let target = content.windows.first(where: { $0.windowID == CGWindowID(window.windowNumber) }
        )
      else {
        throw ProbeFailure.windowMissing
      }
      let filter = SCContentFilter(desktopIndependentWindow: target)
      let configuration = SCStreamConfiguration()
      configuration.width = 640
      configuration.height = 480
      configuration.showsCursor = false
      configuration.capturesAudio = false
      let image = try await SCScreenshotManager.captureImage(
        contentFilter: filter, configuration: configuration)
      screenKit = calibration(image)
      let png = NSBitmapImageRep(cgImage: image).representation(using: .png, properties: [:])
      try png?.write(to: directory.appendingPathComponent("probe-screencapturekit.png"))
      GraphTestLog.write(
        "PROBE ScreenCaptureKit: \(image.width)x\(image.height), calibration=\(screenKit)")
    } catch {
      let error = error as NSError
      GraphTestLog.write(
        "PROBE ScreenCaptureKit: domain=\(error.domain), code=\(error.code), \(error.localizedDescription)"
      )
    }

    // Exercise the exact compositor subprocess route retained by Graph snapshots.
    let output = directory.appendingPathComponent("probe-cgwindow.png")
    let capture = Process()
    capture.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
    capture.arguments = ["-x", "-l", String(window.windowNumber), output.path]
    var cgWindow = false
    do {
      try capture.run()
      for _ in 0..<200 {
        if !capture.isRunning { break }
        try? await Task.sleep(for: .milliseconds(10))
      }
      if capture.isRunning {
        capture.terminate()
        GraphTestLog.write("PROBE CGWindow: capture exceeded 2 seconds")
      } else {
        if capture.terminationStatus == 0,
          let image = NSImage(contentsOf: output)?.cgImage(
            forProposedRect: nil, context: nil, hints: nil)
        {
          cgWindow = calibration(image)
        }
        GraphTestLog.write(
          "PROBE CGWindow: exit=\(capture.terminationStatus), calibration=\(cgWindow)")
      }
    } catch {
      GraphTestLog.write("PROBE CGWindow: \(error.localizedDescription)")
    }
    return hid && CGPreflightPostEventAccess() && screenKit && cgWindow
  }

  /// Normalize the tagged image into sRGB, then compare spatially uniform patches
  /// and relative hue/contrast. Display profiles may lift nominally-zero channels;
  /// neither channel endpoints nor exact authored RGB bytes are invariants.
  static func calibration(_ image: CGImage) -> Bool {
    guard image.width >= 200, image.height >= 150,
      let space = CGColorSpace(name: CGColorSpace.sRGB),
      let context = CGContext(
        data: nil, width: image.width, height: image.height, bitsPerComponent: 8,
        bytesPerRow: image.width * 4, space: space,
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
          | CGBitmapInfo.byteOrder32Big.rawValue)
    else { return false }
    context.draw(image, in: CGRect(x: 0, y: 0, width: image.width, height: image.height))
    guard let data = context.data?.assumingMemoryBound(to: UInt8.self) else { return false }
    func patch(_ fractions: [Double]) -> [Double]? {
      var samples: [[Double]] = []
      for x in fractions {
        for y in [0.3, 0.5, 0.7] {
          let offset =
            Int(y * Double(image.height)) * context.bytesPerRow + Int(x * Double(image.width)) * 4
          guard data[offset + 3] >= 250 else { return nil }
          samples.append((0..<3).map { Double(data[offset + $0]) / 255 })
        }
      }
      let mean = (0..<3).map { channel in
        samples.reduce(0) { $0 + $1[channel] } / Double(samples.count)
      }
      guard
        samples.allSatisfy({ sample in (0..<3).allSatisfy { abs(sample[$0] - mean[$0]) < 0.08 } })
      else { return nil }
      return mean
    }
    // These patches avoid native title chrome, rounded corners and the AX button.
    guard let left = patch([0.15, 0.20, 0.25]), let right = patch([0.75, 0.80, 0.85]) else {
      return false
    }
    let magentaChroma = min(left[0], left[2]) - left[1]
    let cyanChroma = min(right[1], right[2]) - right[0]
    return magentaChroma > 0.15 && cyanChroma > 0.15
      && left[0] - right[0] > 0.15 && right[1] - left[1] > 0.15
  }
}

private enum ProbeFailure: Error { case windowMissing }

@MainActor
private final class PermissionProbeView: NSView {
  let nonce = Int64.random(in: 1...Int64.max)
  var received: [NSEvent.EventType] = []
  var presses = 0
  override var acceptsFirstResponder: Bool { true }
  override func draw(_ dirtyRect: NSRect) {
    NSColor.magenta.setFill()
    NSRect(x: 0, y: 0, width: bounds.width / 2, height: bounds.height).fill()
    NSColor.cyan.setFill()
    NSRect(x: bounds.width / 2, y: 0, width: bounds.width / 2, height: bounds.height).fill()
  }
  // Same-process AX dispatch can invoke a target on its IPC caller's thread.
  // Record the observed callback on the main actor without asserting its origin.
  @objc nonisolated func pressed(_ sender: NSButton) {
    Task { @MainActor in self.presses += 1 }
  }
  private func record(_ event: NSEvent) {
    if event.cgEvent?.getIntegerValueField(.eventSourceUserData) == nonce {
      received.append(event.type)
    }
  }
  override func mouseDown(with event: NSEvent) { record(event) }
  override func mouseDragged(with event: NSEvent) { record(event) }
  override func mouseUp(with event: NSEvent) { record(event) }
}
