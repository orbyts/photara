import AppKit
@preconcurrency import ApplicationServices
import SwiftUI

/// Own the test window from AppKit's launch delegate. A SwiftUI WindowGroup task
/// cannot diagnose a launch that never creates its scene on macOS 27.
@main
@MainActor
enum GraphLabVerificationApp {
  static func main() {
    if ProcessInfo.processInfo.arguments.contains("--launch-focus-competitor") {
      GraphFocusCompetitor.run()
      return
    }
    GraphTestLog.write("PROCESS: pid=\(getpid()), explicit AppKit verification host")
    GraphVerificationResult.writeRecord("process.json", fields: [:])
    UserDefaults.standard.addSuite(named: "com.photara.graph-lab")
    let app = NSApplication.shared
    let delegate = GraphVerificationLifecycle()
    app.setActivationPolicy(.regular)
    app.delegate = delegate
    withExtendedLifetime(delegate) { app.run() }
    // Normal completion exits through GraphVerificationResult. Merely stopping
    // the event loop must never look like a completed behavioral gate.
    GraphTestLog.write("FAIL: verification event loop stopped without a result")
    GraphVerificationResult.recordExitCode(1)
    exit(1)
  }
}

@MainActor
private final class GraphVerificationPresentation: ObservableObject {
  @Published var appearance = PhotaraThemeAppearance.dark
  let theme = try! PhotaraThemeDocument.load(
    from: Bundle.main.url(forResource: "photara-default", withExtension: "json")!)
}

private struct GraphVerificationContent: View {
  @ObservedObject var model: GraphVerificationPresentation
  var body: some View {
    GraphLabView(appearance: $model.appearance)
      .environment(\.photaraTheme, model.theme.resolved(for: model.appearance))
      .preferredColorScheme(model.appearance == .dark ? .dark : .light)
      .frame(minWidth: 1080, minHeight: 700)
  }
}

/// The launcher validates these atomic records against its exact child PID and
/// random run token. An app exit, stale record or windowless process is not success.
@MainActor
enum GraphVerificationResult {
  static func writeRecord(_ name: String, fields: [String: Any]) {
    guard let path = ProcessInfo.processInfo.environment["PHOTARA_GRAPH_EXIT_FILE"] else { return }
    var record = fields
    record["pid"] = getpid()
    record["token"] = ProcessInfo.processInfo.environment["PHOTARA_GRAPH_RUN_TOKEN"] ?? ""
    record["protocol"] = 1
    do {
      let data = try JSONSerialization.data(withJSONObject: record, options: [.sortedKeys])
      try data.write(to: URL(fileURLWithPath: path).deletingLastPathComponent()
        .appendingPathComponent(name), options: .atomic)
    } catch {
      GraphTestLog.write("FAIL: cannot write launch evidence \(name): \(error)")
      exit(1)
    }
  }

  static func recordExitCode(_ code: Int32) {
    guard let path = ProcessInfo.processInfo.environment["PHOTARA_GRAPH_EXIT_FILE"] else { return }
    do { try Data("\(code)\n".utf8).write(to: URL(fileURLWithPath: path), options: .atomic) }
    catch {
      GraphTestLog.write("FAIL: cannot write verification exit status: \(error)")
      exit(1)
    }
  }
}

/// Check permissions in the signed app, where macOS attributes native access.
/// Ordinary preflight is prompt-free. Only the explicit request mode may ask macOS
/// to show its native consent dialogs, and that mode never runs the matrix.
@MainActor
enum GraphVerificationPermissions {
  private static func requestAccessAndExit() -> Never {
    GraphTestLog.write("REQUEST: native permission dialogs only; no tests will run")
    GraphTestLog.write("App: \(Bundle.main.bundleURL.path)")
    GraphTestLog.write("Bundle ID: \(Bundle.main.bundleIdentifier ?? "missing")")
    let options =
      [kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: true] as CFDictionary
    let accessibility = AXIsProcessTrustedWithOptions(options)
    let screenCapture = CGRequestScreenCaptureAccess()
    GraphTestLog.write(
      "REQUEST RETURN: Accessibility=\(accessibility), ScreenCapture=\(screenCapture)")
    GraphTestLog.write(
      "Requests issued. Complete macOS consent manually, then relaunch the same app with --permissions-only. Request return values are not verification results."
    )
    // Zero means only that both public request APIs returned. Permission-only mode
    // must establish fresh access in a new process before the native gate can run.
    GraphVerificationResult.recordExitCode(0)
    exit(0)
  }

  static func requireAccess() async {
    if ProcessInfo.processInfo.arguments.contains("--request-permissions") {
      requestAccessAndExit()
    }
    let accessibility = AXIsProcessTrusted()
    let screenCapture = CGPreflightScreenCaptureAccess()
    GraphTestLog.write(
      "PERMISSIONS: Accessibility=\(accessibility), ScreenCapture=\(screenCapture)")
    if ProcessInfo.processInfo.arguments.contains("--probe-permissions") {
      let operational = await GraphPermissionProbe.run()
      GraphTestLog.write(
        "PROBE RESULT: operations=\(operational), authorization=\(accessibility && screenCapture); no matrix run"
      )
      GraphVerificationResult.recordExitCode(operational && accessibility && screenCapture ? 0 : 1)
      exit(operational && accessibility && screenCapture ? 0 : 1)
    }
    guard accessibility && screenCapture else {
      GraphTestLog.write(
        "BLOCKED: Enable Graph Lab Verification in System Settings > Privacy & Security > Accessibility and Screen & System Audio Recording, then relaunch."
      )
      GraphTestLog.write("App: \(Bundle.main.bundleURL.path)")
      GraphTestLog.write("Bundle ID: \(Bundle.main.bundleIdentifier ?? "missing")")
      GraphVerificationResult.recordExitCode(1)
      exit(1)
    }
    guard await GraphPermissionProbe.run() else {
      GraphTestLog.write("BLOCKED: native capability probe failed before the behavioral matrix")
      GraphVerificationResult.recordExitCode(1)
      exit(1)
    }
    if ProcessInfo.processInfo.arguments.contains("--permissions-only") {
      GraphTestLog.write("PASS: native permissions; behavioral matrix was not run")
      GraphVerificationResult.recordExitCode(0)
      exit(0)
    }
  }
}

/// Readiness depends on the identified native window and production event
/// surface, not an arbitrary delay or whichever NSApp window happens to be first.
@MainActor
final class GraphVerificationLifecycle: NSObject, NSApplicationDelegate {
  static private(set) var window: NSWindow?
  private let presentation = GraphVerificationPresentation()

  func applicationDidFinishLaunching(_ notification: Notification) {
    GraphTestLog.write("LIFECYCLE: didFinishLaunching=true, isRunning=\(NSApp.isRunning), pid=\(getpid())")
    let arguments = ProcessInfo.processInfo.arguments
    let furnace = arguments.contains("--launch-furnace") || arguments.contains("--launch-furnace-focus")
    let fault = ProcessInfo.processInfo.environment["PHOTARA_GRAPH_HOST_FAULT"]
    if let fault, !fault.isEmpty {
      guard furnace && ["windowless", "stalled", "run-stalled"].contains(fault) else {
        GraphTestLog.write("FAIL: host fault injection requires explicit launch furnace mode")
        GraphVerificationResult.recordExitCode(1)
        exit(1)
      }
      GraphTestLog.write("FURNACE FAULT: \(fault); no behavioral matrix")
      if fault == "windowless" { return }
      if fault == "stalled" {
        // Deliberately block the main thread. Only the external process watchdog
        // can bound this failure; an in-app Task/Timer would never fire.
        Thread.sleep(forTimeInterval: 1200)
        return
      }
    }
    let window = NSWindow(
      contentRect: NSRect(x: 0, y: 0, width: 1080, height: 700),
      styleMask: [.titled, .closable, .miniaturizable, .resizable],
      backing: .buffered, defer: false)
    window.title = "Graph Lab Interaction Verification"
    window.identifier = NSUserInterfaceItemIdentifier("photara.graph.verification." +
      (ProcessInfo.processInfo.environment["PHOTARA_GRAPH_RUN_TOKEN"] ?? String(getpid())))
    window.isRestorable = false
    window.isReleasedWhenClosed = false
    window.contentView = NSHostingView(rootView: GraphVerificationContent(model: presentation))
    Self.window = window
    window.center()
    window.makeKeyAndOrderFront(nil)
    NSApp.activate(ignoringOtherApps: true)
    Task { await start(window: window, furnace: furnace, fault: fault) }
  }

  private func start(window: NSWindow, furnace: Bool, fault: String?) async {
    var stableLayouts = 0
    for _ in 0..<500 {
      window.contentView?.layoutSubtreeIfNeeded()
      window.displayIfNeeded()
      if NSApp.isRunning, window.isVisible,
        let surface = window.contentView.flatMap(GraphLabChecks.findSurface),
        surface.window === window, surface.bounds.width > 0, surface.bounds.height > 0 {
        stableLayouts += 1
        if stableLayouts == 3 { break }
      } else { stableLayouts = 0 }
      try? await Task.sleep(for: .milliseconds(20))
    }
    guard stableLayouts == 3, let surface = window.contentView.flatMap(GraphLabChecks.findSurface) else {
      GraphTestLog.write("FAIL: identified verification window/event surface not ready in 10 seconds; visible=\(window.isVisible), windows=\(NSApp.windows.count)")
      GraphVerificationResult.recordExitCode(1)
      exit(1)
    }
    GraphTestLog.write("READY: window=\(window.windowNumber), identifier=\(window.identifier!.rawValue), surface=\(surface.bounds)")
    GraphVerificationResult.writeRecord("ready.json", fields: [
      "window_number": window.windowNumber,
      "window_identifier": window.identifier!.rawValue,
      "event_surface": true,
      "lifecycle_started": true,
      "native_toolbar": window.toolbar?.items.isEmpty == false,
    ])
    if fault == "run-stalled" {
      try? await Task.sleep(for: .seconds(1200))
      return
    }
    if furnace {
      GraphTestLog.write("FURNACE: real-process launch/focus only; no behavioral matrix")
      GraphLabChecks.check(window.toolbar?.items.isEmpty == false, "Explicit test host retains populated native toolbar")
      let suite = GraphLabChecks(window: window, surface: surface)
      await suite.focusCanvas()
      if ProcessInfo.processInfo.arguments.contains("--launch-furnace-focus") {
        await suite.focusFurnace()
      }
      GraphLabChecks.finish()
    }
    await GraphVerificationPermissions.requireAccess()
    await GraphLabChecks.run(appearance: Binding(
      get: { self.presentation.appearance }, set: { self.presentation.appearance = $0 }))
  }
}

/// A separate native process provides actual foreground interruption for the
/// furnace without activating or changing any user-owned application.
@MainActor
private final class GraphFocusCompetitor: NSObject, NSApplicationDelegate {
  private var window: NSWindow?
  static func run() {
    let app = NSApplication.shared
    let delegate = GraphFocusCompetitor()
    app.setActivationPolicy(.regular)
    app.delegate = delegate
    withExtendedLifetime(delegate) { app.run() }
  }
  func applicationDidFinishLaunching(_ notification: Notification) {
    let window = NSWindow(contentRect: NSRect(x: 140, y: 140, width: 280, height: 160),
      styleMask: [.titled, .closable], backing: .buffered, defer: false)
    window.title = "Graph focus furnace interruption"
    window.isReleasedWhenClosed = false
    self.window = window
    Task {
      for _ in 0..<100 {
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
        if NSApp.isActive && window.isKeyWindow { break }
        try? await Task.sleep(for: .milliseconds(20))
      }
      if let path = ProcessInfo.processInfo.environment["PHOTARA_GRAPH_COMPETITOR_READY"] {
        let record: [String: Any] = ["pid": getpid(), "window_number": window.windowNumber,
          "active": NSApp.isActive, "key": window.isKeyWindow]
        if let data = try? JSONSerialization.data(withJSONObject: record, options: [.sortedKeys]) {
          try? data.write(to: URL(fileURLWithPath: path), options: .atomic)
        }
      }
    }
    Timer.scheduledTimer(withTimeInterval: 20, repeats: false) { _ in exit(1) }
  }
}
