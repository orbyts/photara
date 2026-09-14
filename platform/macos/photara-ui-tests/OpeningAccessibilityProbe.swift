import AppKit
import ApplicationServices

/// Runs outside the host process: SwiftUI publishes its accessibility tree lazily
/// to external clients, not to NSHostingView.accessibilityChildren().
@main
struct OpeningAccessibilityProbe {
  static func value(_ element: AXUIElement, _ attribute: String) -> CFTypeRef? {
    var result: CFTypeRef?
    guard AXUIElementCopyAttributeValue(element, attribute as CFString, &result) == .success else {
      return nil
    }
    return result
  }

  static func tree(_ element: AXUIElement, depth: Int = 0) -> [AXUIElement] {
    guard depth < 30 else { return [] }
    let children = value(element, kAXChildrenAttribute) as? [AXUIElement] ?? []
    return [element] + children.flatMap { tree($0, depth: depth + 1) }
  }

  static func main() {
    let application = AXUIElementCreateApplication(pid_t(CommandLine.arguments[1])!)
    let mode = CommandLine.arguments[2]
    let windows = value(application, kAXWindowsAttribute) as? [AXUIElement] ?? []
    let elements = windows.flatMap { tree($0) }
    if mode.hasPrefix("draft-") {
      verifyDraft(application, mode: mode)
      return
    }
    let identifiers = elements.compactMap { value($0, kAXIdentifierAttribute) as? String }
    guard
      let create = elements.first(where: {
        value($0, kAXIdentifierAttribute) as? String == "opening-create-project"
      }),
      let open = elements.first(where: {
        value($0, kAXIdentifierAttribute) as? String == "opening-open-package"
      })
    else {
      // Keep the failure diagnostic restricted to the synthetic UI host.
      print(
        "Missing Opening actions; trusted=\(AXIsProcessTrusted()); identifiers=\(identifiers); elements=\(elements.count)"
      )
      for element in elements {
        print(
          [
            kAXRoleAttribute, kAXIdentifierAttribute, kAXTitleAttribute, kAXDescriptionAttribute,
            kAXValueAttribute,
          ].map { String(describing: value(element, $0)) }.joined(separator: " | "))
      }
      exit(1)
    }
    for button in [create, open] {
      guard value(button, kAXRoleAttribute) as? String == kAXButtonRole,
        value(button, kAXEnabledAttribute) as? Bool == true
      else {
        print("Opening action is not an enabled native button")
        exit(1)
      }
    }
    let labels = elements.flatMap { element in
      [kAXTitleAttribute, kAXDescriptionAttribute, kAXValueAttribute].compactMap {
        value(element, $0) as? String
      }
    }
    guard ["People", "Locations", "Location Kinds"].allSatisfy({ labels.contains($0) }) else {
      print("Missing native sidebar labels: \(labels)")
      exit(1)
    }
    if mode == "create" || mode == "open" {
      guard
        AXUIElementPerformAction(mode == "create" ? create : open, kAXPressAction as CFString)
          == .success
      else {
        print("Native Create action failed")
        exit(1)
      }
    }
    print("PASS: external Opening accessibility \(mode)")
  }

  static func verifyDraft(_ application: AXUIElement, mode: String) {
    func element(_ identifier: String) -> AXUIElement {
      let windows = value(application, kAXWindowsAttribute) as? [AXUIElement] ?? []
      let elements = windows.flatMap { tree($0) }
      guard
        let result = elements.first(where: {
          value($0, kAXIdentifierAttribute) as? String == identifier
        })
      else { fatalError("Missing Create Project element: \(identifier)") }
      return result
    }
    func press(_ identifier: String) {
      let button = element(identifier)
      guard value(button, kAXRoleAttribute) as? String == kAXButtonRole,
        value(button, kAXEnabledAttribute) as? Bool == true,
        AXUIElementPerformAction(button, kAXPressAction as CFString) == .success
      else { fatalError("Cannot press native Create Project button: \(identifier)") }
    }
    let confirm = element("create-project-confirm")
    guard value(confirm, kAXEnabledAttribute) as? Bool == false else {
      fatalError("Empty Create Project draft enabled confirmation")
    }
    press("create-project-choose")
    if mode == "draft-cancel" {
      press("create-project-cancel")
    } else {
      let field = element("create-project-name")
      // Type through the native responder chain. A direct AX value write can
      // change the AppKit field editor without notifying SwiftUI's binding.
      guard
        value(field, kAXRoleAttribute) as? String == kAXTextFieldRole,
        AXUIElementSetAttributeValue(field, kAXFocusedAttribute as CFString, kCFBooleanTrue)
          == .success
      else {
        fatalError("Cannot focus native project name")
      }
      let pid = pid_t(CommandLine.arguments[1])!
      let characters = Array("  Coastal Studies  ".utf16)
      for down in [true, false] {
        guard let event = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: down) else {
          fatalError("Cannot create native fixture key event")
        }
        event.keyboardSetUnicodeString(stringLength: characters.count, unicodeString: characters)
        event.postToPid(pid)
      }
      var packageLabels: [String] = []
      for _ in 0..<40 {
        let package = element("create-project-package-name")
        packageLabels = tree(package).flatMap { item in
          [kAXTitleAttribute, kAXDescriptionAttribute, kAXValueAttribute].compactMap {
            value(item, $0) as? String
          }
        }
        if value(element("create-project-confirm"), kAXEnabledAttribute) as? Bool == true,
          packageLabels.contains(where: { $0.contains("Coastal Studies.photara") })
        {
          break
        }
        Thread.sleep(forTimeInterval: 0.025)
      }
      guard packageLabels.contains(where: { $0.contains("Coastal Studies.photara") }) else {
        fatalError(
          "Package preview did not follow trimmed draft: \(packageLabels); name=\(String(describing: value(field, kAXValueAttribute))); enabled=\(String(describing: value(element("create-project-confirm"), kAXEnabledAttribute)))"
        )
      }
      press("create-project-confirm")
    }
    print("PASS: external Create Project accessibility \(mode)")
  }
}
