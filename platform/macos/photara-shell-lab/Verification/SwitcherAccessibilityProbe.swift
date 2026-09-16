import AppKit
import ApplicationServices

@main
struct SwitcherAccessibilityProbe {
    static func value(_ element: AXUIElement, _ name: String) -> CFTypeRef? {
        var result: CFTypeRef?
        guard AXUIElementCopyAttributeValue(element, name as CFString, &result) == .success else { return nil }
        return result
    }
    static func tree(_ element: AXUIElement, depth: Int = 0) -> [AXUIElement] {
        guard depth < 30, value(element, kAXRoleAttribute) as? String != kAXMenuBarRole else { return [] }
        return [element] + ((value(element, kAXChildrenAttribute) as? [AXUIElement]) ?? []).flatMap { tree($0, depth: depth + 1) }
    }
    static func check(_ condition: Bool, _ message: String) {
        if !condition { print("FAIL: \(message)"); exit(1) }
    }
    static func key(_ pid: pid_t, code: CGKeyCode, text: String? = nil) {
        for down in [true, false] {
            let event = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: down)!
            event.flags = []
            if let text {
                let chars = Array(text.utf16)
                event.keyboardSetUnicodeString(stringLength: chars.count, unicodeString: chars)
            }
            check(NSWorkspace.shared.frontmostApplication?.processIdentifier == pid, "Synthetic host lost foreground before keyboard input")
            event.post(tap: .cghidEventTap)
        }
        Thread.sleep(forTimeInterval: 0.1)
    }
    static func main() throws {
        let pid = pid_t(CommandLine.arguments[1])!
        let mode = CommandLine.arguments[2]
        print("CAPABILITIES AX=\(AXIsProcessTrusted()) post=\(CGPreflightPostEventAccess())")
        let app = AXUIElementCreateApplication(pid)
        let initial = tree(app)
        guard let trigger = initial.first(where: { value($0, kAXIdentifierAttribute) as? String == "lifecycle.library-switcher" }) else {
            print(initial.map { "\(value($0,kAXRoleAttribute) as? String ?? "") | \(value($0,kAXDescriptionAttribute) as? String ?? "")" }.joined(separator: "\n"))
            fatalError("Missing native switcher")
        }
        let description = [kAXTitleAttribute, kAXDescriptionAttribute, kAXHelpAttribute, kAXValueAttribute].compactMap { value(trigger, $0) as? String }.joined(separator: " | ")
        print("TRIGGER role=\(value(trigger,kAXRoleAttribute) as? String ?? "") labels=\(description)")
        check(description.contains("Libraries and account") && (description.contains("Suhail") || description.contains("On This Mac")), "Trigger must expose identity and menu purpose")
        if mode == "inspect" { print("PASS: native switcher accessible label: \(description)"); return }
        check(NSWorkspace.shared.frontmostApplication?.processIdentifier == pid, "Synthetic host lost foreground before menu activation")
        var result = AXError.success
        if mode != "keyboard" {
            var point = CGPoint.zero; var size = CGSize.zero
            guard let position = value(trigger, kAXPositionAttribute), let extent = value(trigger, kAXSizeAttribute) else { fatalError("Missing native trigger bounds") }
            check(AXValueGetValue(unsafeBitCast(position, to: AXValue.self), .cgPoint, &point), "Missing trigger position")
            check(AXValueGetValue(unsafeBitCast(extent, to: AXValue.self), .cgSize, &size), "Missing trigger size")
            point.x += size.width / 2; point.y += size.height / 2
            check(NSWorkspace.shared.frontmostApplication?.processIdentifier == pid, "Synthetic host lost foreground before left click")
            for type in [CGEventType.leftMouseDown, .leftMouseUp] {
                let event = CGEvent(mouseEventSource: nil, mouseType: type, mouseCursorPosition: point, mouseButton: .left)!
                event.flags = []; event.post(tap: .cghidEventTap); Thread.sleep(forTimeInterval: 0.05)
            }
        } else if mode == "keyboard" {
            result = AXUIElementSetAttributeValue(trigger, kAXFocusedAttribute as CFString, kCFBooleanTrue)
            check(result == .success, "Cannot focus identity menu for keyboard activation")
            key(pid, code: 49, text: " ")
        } else {
            result = AXUIElementPerformAction(trigger, kAXPressAction as CFString)
        }
        // Observe the result of one activation; never retry the action.
        var items: [AXUIElement] = []
        for _ in 0..<60 {
            items = tree(app).filter { value($0, kAXRoleAttribute) as? String == kAXMenuItemRole }
            if !items.isEmpty { break }
            Thread.sleep(forTimeInterval: 0.05)
        }
        let titles = items.compactMap { value($0, kAXTitleAttribute) as? String }
        print("MENU press=\(result.rawValue) items=\(titles)")
        check(!items.isEmpty, "Native menu did not open")
        check(!titles.contains("Add Account…") && !titles.contains("Save"), "Unavailable commands exposed")
        check(titles.contains("New Library…") && titles.contains("Library Settings…") && titles.contains("Account Settings…"), "Missing settings sections")
        let checked = items.filter { !(value($0, kAXMenuItemMarkCharAttribute) as? String ?? "").isEmpty }
        print("CHECKED \(checked.compactMap { value($0, kAXTitleAttribute) as? String })")
        check(checked.count == 1, "Expected exactly one native current checkmark")
        if CommandLine.arguments.count > 4 && !CommandLine.arguments[4].isEmpty {
            check(value(checked[0], kAXTitleAttribute) as? String == CommandLine.arguments[4], "Wrong Library has the current checkmark")
        }
        let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
        guard let popup = windows.first(where: { ($0[kCGWindowOwnerPID as String] as? Int32) == pid && ($0[kCGWindowLayer as String] as? Int ?? 0) > 0 }),
              let bounds = popup[kCGWindowBounds as String] as? [String: Any],
              let menuFrame = CGRect(dictionaryRepresentation: bounds as CFDictionary),
              let position = value(trigger, kAXPositionAttribute) else { fatalError("Missing menu placement geometry") }
        var triggerOrigin = CGPoint.zero
        check(AXValueGetValue(unsafeBitCast(position, to: AXValue.self), .cgPoint, &triggerOrigin), "Missing trigger geometry")
        print("PLACEMENT menuBottom=\(menuFrame.maxY) triggerTop=\(triggerOrigin.y)")
        check(menuFrame.maxY <= triggerOrigin.y + 2, "Menu must open above the identity trigger")
        if CommandLine.arguments.count > 3 && !CommandLine.arguments[3].isEmpty {
            let infos = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
            let owned = infos.filter { ($0[kCGWindowOwnerPID as String] as? Int32) == pid }
            print("OWNED WINDOWS \(owned.map { [$0[kCGWindowNumber as String] ?? 0, $0[kCGWindowLayer as String] ?? 0] })")
            if let menu = owned.first(where: { ($0[kCGWindowLayer as String] as? Int ?? 0) > 0 }), let id = menu[kCGWindowNumber as String] as? Int {
                let process = Process(); process.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
                process.arguments = ["-x", "-o", "-l", String(id), CommandLine.arguments[3]]
                try process.run(); process.waitUntilExit(); check(process.terminationStatus == 0, "Menu compositor capture failed")
            } else { print("LIMITATION: no separate owned compositor menu window") }
        }
        if mode == "signin" || mode == "settings" {
            let title = mode == "signin" ? "Sign in with Google" : "Library Settings…"
            guard let item = items.first(where: { value($0, kAXTitleAttribute) as? String == title }) else { fatalError("Missing native action") }
            check(value(item, kAXEnabledAttribute) as? Bool == true, "Action is unavailable")
            check(AXUIElementPerformAction(item, kAXPressAction as CFString) == .success, "Native action failed")
        } else if mode == "keyboard" {
            key(pid, code: 125)
            key(pid, code: 125)
            Thread.sleep(forTimeInterval: 0.3)
            print("SELECTED after Down: \(tree(app).filter { value($0,kAXSelectedAttribute) as? Bool == true }.compactMap { value($0,kAXTitleAttribute) as? String })")
            key(pid, code: 36, text: "\r")
        } else { key(pid, code: 53, text: "\u{1b}") }
        Thread.sleep(forTimeInterval: 0.2)
        print("AFTER ACTION CHECKED \(tree(app).filter { !(value($0,kAXMenuItemMarkCharAttribute) as? String ?? "").isEmpty }.compactMap { value($0,kAXTitleAttribute) as? String })")
        Thread.sleep(forTimeInterval: 0.7)
        let remaining = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
        check(!remaining.contains { ($0[kCGWindowOwnerPID as String] as? Int32) == pid && ($0[kCGWindowLayer as String] as? Int ?? 0) == 101 }, "Native menu window did not dismiss after keyboard action")
        print("PASS: native menu accessibility, checkmark, \(mode == "keyboard" ? "Space/Down/Return delivered" : mode == "click" ? "left-click/Escape delivered" : mode == "signin" || mode == "settings" ? "native action delivered" : "Escape delivered")")
    }
}
