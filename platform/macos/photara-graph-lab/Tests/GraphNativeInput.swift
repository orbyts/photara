import AppKit

/// Native NSEvent construction used only by the verification executable.
/// The behavioral matrix still dispatches through NSWindow and the production
/// responder. It never substitutes controller calls or relaxed oracle tolerances.
@MainActor
enum GraphNativeInput {
  struct DeliveryError: Error, CustomStringConvertible {
    let description: String
  }

  /// Resolve pending SwiftUI/AppKit layout before fixing a synthetic event's
  /// window coordinates. A lazy layout during dispatch can otherwise move the
  /// canvas origin by half a point between the harness and native responder.
  /// This drains native layout only; it does not read or change Graph state.
  static func location(_ point: CGPoint, in view: NSView) -> CGPoint {
    view.window?.contentView?.layoutSubtreeIfNeeded()
    view.window?.displayIfNeeded()
    return view.convert(point, to: nil)
  }

  /// NSView.hitTest takes a point in the receiver's superview coordinates,
  /// unlike the receiver-local coordinates used by most view drawing APIs.
  static func hitTest(_ point: CGPoint, in view: NSView, through receiver: NSView?) -> NSView? {
    guard let receiver else { return nil }
    receiver.window?.contentView?.layoutSubtreeIfNeeded()
    receiver.window?.displayIfNeeded()
    return receiver.hitTest(view.convert(point, to: receiver.superview))
  }

  static func mouse(
    _ type: NSEvent.EventType, location: CGPoint,
    flags: NSEvent.ModifierFlags, window: NSWindow
  ) throws -> NSEvent {
    guard
      let original = NSEvent.mouseEvent(
        with: type, location: location, modifierFlags: flags,
        timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: window.windowNumber,
        context: nil, eventNumber: 1, clickCount: 1, pressure: type == .leftMouseUp ? 0 : 1)
    else { throw DeliveryError(description: "Native mouse factory returned nil") }
    let button: Int
    switch type {
    case .rightMouseDown, .rightMouseDragged, .rightMouseUp: button = 1
    case .otherMouseDown, .otherMouseDragged, .otherMouseUp: button = 2
    default: button = 0
    }
    var event = original
    if button != 0 {
      guard let cg = original.cgEvent else {
        throw DeliveryError(description: "Native mouse event has no CGEvent")
      }
      cg.setIntegerValueField(.mouseEventButtonNumber, value: Int64(button))
      guard let wrapped = NSEvent(cgEvent: cg) else {
        throw DeliveryError(description: "Native mouse bridge returned nil")
      }
      event = wrapped
      // macOS 27 already supplies the correct window-local point. Writing
      // cg.location even to the same value invalidates that association.
      // Older AppKit can retain an origin after resize; normalize only an
      // actually incorrect point, and still reject any failed conversion.
      if !near(event.locationInWindow, location) {
        let global = window.convertPoint(toScreen: location)
        cg.location = CGPoint(x: global.x, y: NSScreen.screens.first!.frame.height - global.y)
        guard let bridged = NSEvent(cgEvent: cg) else {
          throw DeliveryError(description: "Legacy mouse bridge returned nil")
        }
        let error = CGPoint(
          x: location.x - bridged.locationInWindow.x,
          y: location.y - bridged.locationInWindow.y)
        cg.location = CGPoint(x: cg.location.x + error.x, y: cg.location.y - error.y)
        guard let corrected = NSEvent(cgEvent: cg) else {
          throw DeliveryError(description: "Legacy coordinate normalization returned nil")
        }
        event = corrected
      }
    }
    guard near(event.locationInWindow, location), event.window === window,
      event.windowNumber == window.windowNumber, event.buttonNumber == button,
      event.type == type,
      event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        == flags.intersection(.deviceIndependentFlagsMask)
    else {
      throw DeliveryError(
        description:
          "expected \(location), actual \(event.locationInWindow), window \(event.windowNumber)/\(window.windowNumber), button \(event.buttonNumber)/\(button), type \(event.type.rawValue)/\(type.rawValue)"
      )
    }
    return event
  }

  private static func near(_ a: CGPoint, _ b: CGPoint) -> Bool {
    hypot(a.x - b.x, a.y - b.y) <= 0.001
  }

  /// Fail before any Graph matrix if event construction or actual NSWindow →
  /// NSView delivery changes. Fractional points catch premature rounding.
  static func preflight(check: (Bool, String) -> Void) -> Bool {
    let recorder = CoordinateRecorder(frame: .init(x: 0, y: 0, width: 640, height: 420))
    let window = NSWindow(
      contentRect: recorder.frame, styleMask: [.titled, .resizable],
      backing: .buffered, defer: false)
    window.isReleasedWhenClosed = false
    window.title = "Graph native input preflight"
    window.contentView = recorder
    defer { window.close() }
    var passed = true
    var deliveries = 0
    let frames = [
      CGRect(x: 100, y: 150, width: 640, height: 420),
      CGRect(x: 340, y: 260, width: 820, height: 580),
      CGRect(x: 70, y: 90, width: 700, height: 460),
    ]
    let groups: [[NSEvent.EventType]] = [
      [.leftMouseDown, .leftMouseDragged, .leftMouseUp],
      [.rightMouseDown, .rightMouseDragged, .rightMouseUp],
      [.otherMouseDown, .otherMouseDragged, .otherMouseUp],
    ]
    for (frameIndex, frame) in frames.enumerated() {
      window.setFrameOrigin(frame.origin)
      window.setContentSize(frame.size)
      window.makeKeyAndOrderFront(nil)
      window.contentView?.layoutSubtreeIfNeeded()
      for (pointIndex, point) in [
        CGPoint(x: 42.125, y: 55.875),
        CGPoint(x: 271.375, y: 192.625), CGPoint(x: 503.75, y: 314.25),
      ].enumerated() {
        for (button, group) in groups.enumerated() {
          let flags: NSEvent.ModifierFlags =
            pointIndex == 0 ? [] : (pointIndex == 1 ? [.option] : [.control])
          for type in group {
            let label =
              "Coordinate preflight frame \(frameIndex), point \(pointIndex), type \(type.rawValue)"
            do {
              let local = recorder.convert(point, to: nil)
              let event = try mouse(type, location: local, flags: flags, window: window)
              recorder.received = nil
              window.sendEvent(event)
              let actual = recorder.received
              let valid =
                actual?.type == type && actual?.buttonNumber == button
                && actual?.window === window
                && actual.map { near(recorder.convert($0.locationInWindow, from: nil), point) }
                  == true
              check(
                valid,
                label + " did not reach the native responder with exact coordinates/button/window")
              passed = passed && valid
              deliveries += 1
            } catch {
              check(false, label + ": \(error)")
              passed = false
            }
          }
        }
      }
    }
    // Independent target geometry: the source point (150,75) lands at the
    // center of target (180,95) in a translated, flipped receiver. Passing
    // that receiver-local point directly to hitTest must miss this target.
    let parent = NSView(frame: .init(x: 0, y: 0, width: 500, height: 400))
    let receiver = CoordinateRecorder(frame: .init(x: 80, y: 60, width: 300, height: 200))
    let source = CoordinateRecorder(frame: .init(x: 30, y: 20, width: 200, height: 150))
    let target = NSView(frame: .init(x: 150, y: 80, width: 60, height: 30))
    parent.addSubview(receiver)
    receiver.addSubview(source)
    receiver.addSubview(target)
    window.contentView = parent
    let point = CGPoint(x: 150, y: 75)
    let resolvesTarget = hitTest(point, in: source, through: receiver) === target
    let rejectsLocal = receiver.hitTest(source.convert(point, to: receiver)) !== target
    check(resolvesTarget, "Native hit-test resolves the independent target through translated/flipped coordinates")
    check(rejectsLocal, "Native hit-test fixture rejects receiver-local coordinates")
    passed = passed && resolvesTarget && rejectsLocal
    if passed {
      GraphTestLog.write(
        "PREFLIGHT: \(deliveries) exact native coordinate/button/window deliveries and 2 independent native hit-test checks pass after move/resize"
      )
    }
    return passed
  }
}

private final class CoordinateRecorder: NSView {
  var received: NSEvent?
  override var isFlipped: Bool { true }
  override var acceptsFirstResponder: Bool { true }
  override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }
  override func mouseDown(with event: NSEvent) { received = event }
  override func mouseDragged(with event: NSEvent) { received = event }
  override func mouseUp(with event: NSEvent) { received = event }
  override func rightMouseDown(with event: NSEvent) { received = event }
  override func rightMouseDragged(with event: NSEvent) { received = event }
  override func rightMouseUp(with event: NSEvent) { received = event }
  override func otherMouseDown(with event: NSEvent) { received = event }
  override func otherMouseDragged(with event: NSEvent) { received = event }
  override func otherMouseUp(with event: NSEvent) { received = event }
}
