import AppKit
@preconcurrency import ApplicationServices
import SwiftUI

/// Keep the original SwiftUI window hierarchy while explicitly presenting the
/// verification scene. The runner supplies the normal LaunchServices open event;
/// a restored windowless app must not skip its test task.
@main
struct GraphLabVerificationApp: App {
  @NSApplicationDelegateAdaptor(GraphVerificationLifecycle.self) private var lifecycle
  @State private var appearance = PhotaraThemeAppearance.dark
  private let theme = try! PhotaraThemeDocument.load(
    from: Bundle.main.url(forResource: "photara-default", withExtension: "json")!)
  init() { UserDefaults.standard.addSuite(named: "com.photara.graph-lab") }
  var body: some Scene {
    WindowGroup("Graph Lab Interaction Verification") {
      GraphLabView(appearance: $appearance)
        .environment(\.photaraTheme, theme.resolved(for: appearance))
        .preferredColorScheme(appearance == .dark ? .dark : .light)
        .frame(minWidth: 1080, minHeight: 700)
        .task {
          guard await GraphVerificationLifecycle.waitUntilReady() else { return }
          await GraphVerificationPermissions.requireAccess()
          await GraphLabChecks.run(appearance: $appearance)
        }
    }
    .defaultLaunchBehavior(.presented)
    .restorationBehavior(.disabled)
  }
}

/// LaunchServices does not propagate an app's exit status through `open -W`.
/// The runner accepts a result only after the actual verification finish path
/// writes it atomically. A crash or early startup failure leaves no success file.
@MainActor
enum GraphVerificationResult {
  static func recordExitCode(_ code: Int32) {
    guard let path = ProcessInfo.processInfo.environment["PHOTARA_GRAPH_EXIT_FILE"] else { return }
    try? Data("\(code)\n".utf8).write(to: URL(fileURLWithPath: path), options: .atomic)
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

/// `.task` may start before the application launch notification. Observe the real
/// AppKit lifecycle, then yield another turn before calling any permission API.
@MainActor
final class GraphVerificationLifecycle: NSObject, NSApplicationDelegate {
  static var finishedLaunching = false
  static var started = false

  func applicationDidFinishLaunching(_ notification: Notification) {
    Self.finishedLaunching = true
  }

  static func waitUntilReady() async -> Bool {
    guard !started else { return false }
    started = true
    for _ in 0..<500 {
      if finishedLaunching && NSApp.isRunning { break }
      try? await Task.sleep(for: .milliseconds(10))
    }
    guard finishedLaunching && NSApp.isRunning else {
      GraphTestLog.write(
        "BLOCKED: applicationDidFinishLaunching/main event loop not established in 5 seconds")
      GraphVerificationResult.recordExitCode(1)
      exit(1)
    }
    await Task.yield()
    GraphTestLog.write("LIFECYCLE: didFinishLaunching=true, isRunning=true, pid=\(getpid())")
    return true
  }
}
