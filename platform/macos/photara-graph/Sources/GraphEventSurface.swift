import AppKit
import SwiftUI

/// A single native responder owns every graph pointer sequence. SwiftUI graph
/// content is render-only; controls outside/above this view keep native routing.
struct PhotaraGraphEventSurface: NSViewRepresentable {
    let controller: PhotaraGraphInteractionController
    func makeNSView(context: Context) -> PhotaraGraphEventView { PhotaraGraphEventView(controller: controller) }
    func updateNSView(_ view: PhotaraGraphEventView, context: Context) { view.refreshCursor() }
    static func dismantleNSView(_ view: PhotaraGraphEventView, coordinator: ()) { view.detach() }
}

final class PhotaraGraphEventView: NSView {
    let controller: PhotaraGraphInteractionController
    private(set) var capturedButton: Int?
    private var tracking: NSTrackingArea?
    private var shortcutMonitor: Any?
    private var lastCursor: NSCursor?
    private var menuActions: [() -> Void] = []
    private lazy var knifeCursor = Self.makeKnifeCursor()

    init(controller: PhotaraGraphInteractionController) {
        self.controller = controller
        super.init(frame: .zero)
        setAccessibilityIdentifier("photara.graph.canvas")
    }
    @available(*, unavailable) required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
    override var isFlipped: Bool { true }
    override var acceptsFirstResponder: Bool { true }
    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        NotificationCenter.default.removeObserver(self)
        if let shortcutMonitor { NSEvent.removeMonitor(shortcutMonitor); self.shortcutMonitor = nil }
        guard let window else { detach(); return }
        NotificationCenter.default.addObserver(self, selector: #selector(lostFocus), name: NSWindow.didResignKeyNotification, object: window)
        NotificationCenter.default.addObserver(self, selector: #selector(lostFocus), name: NSApplication.didResignActiveNotification, object: nil)
        // A tool shortcut may be used over the canvas before its first click,
        // or after using a sidebar slider. Forward only this key to the native
        // responder. This monitor never sees or owns pointer events.
        shortcutMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            guard let self, event.window === self.window, self.window?.firstResponder !== self,
                  !(self.window?.firstResponder is NSTextView),
                  event.modifierFlags.intersection([.command, .control, .option]).isEmpty,
                  event.charactersIgnoringModifiers?.lowercased() == "y",
                  let window = self.window,
                  let content = window.contentView,
                  content.hitTest(content.convert(window.mouseLocationOutsideOfEventStream, from: nil)) === self
            else { return event }
            window.makeFirstResponder(self)
            self.keyDown(with: event)
            return nil
        }
    }
    func detach() {
        controller.cancel(resetTool: true)
        capturedButton = nil
        if let shortcutMonitor { NSEvent.removeMonitor(shortcutMonitor); self.shortcutMonitor = nil }
        NotificationCenter.default.removeObserver(self)
    }
    @objc private func lostFocus() {
        controller.cancel(resetTool: true)
        capturedButton = nil
        lastCursor = nil
        NSCursor.arrow.set()
    }
    override func resignFirstResponder() -> Bool {
        controller.cancel(resetTool: true)
        refreshCursor()
        return true
    }
    override func setFrameSize(_ newSize: NSSize) {
        if frame.size != newSize { controller.cancel(); controller.viewport = newSize }
        super.setFrameSize(newSize)
    }
    private func point(_ event: NSEvent) -> CGPoint { convert(event.locationInWindow, from: nil) }

    override func mouseDown(with event: NSEvent) {
        if event.modifierFlags.contains(.control) { showMenu(event); return }
        begin(event)
    }
    override func otherMouseDown(with event: NSEvent) { if event.buttonNumber == 2 { begin(event) } }
    private func begin(_ event: NSEvent) {
        guard capturedButton == nil else { return }
        window?.makeFirstResponder(self)
        capturedButton = event.buttonNumber
        controller.pointerDown(at: point(event), middle: event.buttonNumber == 2, option: event.modifierFlags.contains(.option))
        refreshCursor()
    }
    override func mouseDragged(with event: NSEvent) { drag(event) }
    override func otherMouseDragged(with event: NSEvent) { drag(event) }
    private func drag(_ event: NSEvent) {
        guard capturedButton == event.buttonNumber else { return }
        controller.pointerDragged(to: point(event))
        refreshCursor()
    }
    override func mouseUp(with event: NSEvent) { end(event) }
    override func otherMouseUp(with event: NSEvent) { end(event) }
    private func end(_ event: NSEvent) {
        guard capturedButton == event.buttonNumber else { return }
        controller.pointerUp(at: point(event))
        capturedButton = nil
        refreshCursor()
    }
    override func keyDown(with event: NSEvent) {
        let command = event.modifierFlags.intersection([.command, .control, .option])
        if event.keyCode == 53 {
            controller.cancel(resetTool: true)
        } else if command.isEmpty && event.charactersIgnoringModifiers?.lowercased() == "y" {
            if !event.isARepeat { controller.setKnifeMode(true) }
        } else if command.isEmpty && (event.keyCode == 51 || event.keyCode == 117) {
            if !controller.deleteSelection() { super.keyDown(with: event) }
        } else { super.keyDown(with: event) }
        refreshCursor()
    }
    override func keyUp(with event: NSEvent) {
        if event.charactersIgnoringModifiers?.lowercased() == "y" { controller.setKnifeMode(false); refreshCursor() }
        else { super.keyUp(with: event) }
    }
    override func cancelOperation(_ sender: Any?) { controller.cancel(resetTool: true); refreshCursor() }
    override func scrollWheel(with event: NSEvent) {
        // Ignore momentum and camera gestures while a pointer sequence owns the
        // canvas. Geometry cannot move under a captured port/node/knife drag.
        guard capturedButton == nil else { return }
        if event.hasPreciseScrollingDeltas {
            controller.scroll(by: CGSize(width: event.scrollingDeltaX, height: event.scrollingDeltaY))
        } else if abs(event.scrollingDeltaY) >= abs(event.scrollingDeltaX), event.scrollingDeltaY != 0 {
            controller.zoom(to: controller.camera.zoom * exp(event.scrollingDeltaY * 0.012), anchor: point(event))
        }
    }
    override func magnify(with event: NSEvent) {
        guard capturedButton == nil else { return }
        controller.zoom(to: controller.camera.zoom * max(0.01, 1 + event.magnification), anchor: point(event))
    }

    override func updateTrackingAreas() {
        if let tracking { removeTrackingArea(tracking) }
        tracking = NSTrackingArea(rect: .zero, options: [.activeInKeyWindow, .inVisibleRect, .mouseEnteredAndExited, .cursorUpdate], owner: self)
        addTrackingArea(tracking!)
        super.updateTrackingAreas()
    }
    private var cursor: NSCursor { controller.isPanning ? .closedHand : (controller.knifeMode ? knifeCursor : .arrow) }
    override func resetCursorRects() { addCursorRect(bounds, cursor: cursor) }
    override func cursorUpdate(with event: NSEvent) { cursor.set() }
    override func mouseEntered(with event: NSEvent) { cursor.set() }
    override func mouseExited(with event: NSEvent) { if capturedButton == nil { NSCursor.arrow.set() } }
    func refreshCursor() {
        let next = cursor
        guard next !== lastCursor else { return }
        lastCursor = next
        window?.invalidateCursorRects(for: self)
        if capturedButton != nil || bounds.contains(convert(window?.mouseLocationOutsideOfEventStream ?? .zero, from: nil)) { next.set() }
    }

    override func rightMouseDown(with event: NSEvent) { showMenu(event) }
    private func showMenu(_ event: NSEvent) {
        guard capturedButton == nil else { return }
        controller.cancel(resetTool: true)
        window?.makeFirstResponder(self)
        let hit = controller.hitTest(point(event))
        controller.select(hit)
        let menu = NSMenu()
        menuActions = []
        func add(_ title: String, _ action: @escaping () -> Void) {
            let item = NSMenuItem(title: title, action: #selector(performMenuAction(_:)), keyEquivalent: "")
            item.target = self
            item.tag = menuActions.count
            menuActions.append(action)
            menu.addItem(item)
        }
        switch hit {
        case .noodle(let id):
            if controller.document.connections.first(where: { $0.id == id })?.knot == nil {
                let point = controller.camera.world(point(event), in: controller.viewport)
                add("Add Routing Knot") { [controller] in controller.setKnot(point, connectionID: id) }
            }
            add("Disconnect") { [controller] in controller.removeConnections([id]) }
        case .knot(let id):
            add("Delete Knot") { [controller] in controller.setKnot(nil, connectionID: id) }
            add("Disconnect") { [controller] in controller.removeConnections([id]) }
        case .port(let port):
            if controller.document.connections.contains(where: { $0.source == port || $0.destination == port }) {
                add("Disconnect") { [controller] in controller.disconnect(port: port) }
            }
            if controller.document.port(port)?.direction == .input {
                for node in controller.document.nodes {
                    for output in node.ports(.output) {
                        let source = PhotaraGraphPortID(node: node.id, key: output.id)
                        if controller.document.canConnect(source, port) {
                            add("Connect \(node.title) / \(output.label) Here") { [controller] in controller.connect(source, to: port) }
                        }
                    }
                }
            }
        default: break
        }
        if !menu.items.isEmpty { NSMenu.popUpContextMenu(menu, with: event, for: self) }
        menuActions.removeAll()
        refreshCursor()
    }
    @objc private func performMenuAction(_ item: NSMenuItem) {
        if menuActions.indices.contains(item.tag) { menuActions[item.tag]() }
    }

    private static func makeKnifeCursor() -> NSCursor {
        let image = NSImage(size: NSSize(width: 24, height: 24), flipped: false) { _ in
            let blade = NSBezierPath()
            blade.move(to: NSPoint(x: 9, y: 16))
            blade.line(to: NSPoint(x: 15, y: 16))
            blade.curve(to: NSPoint(x: 12, y: 2), controlPoint1: NSPoint(x: 15, y: 10), controlPoint2: NSPoint(x: 13, y: 5))
            blade.curve(to: NSPoint(x: 9, y: 16), controlPoint1: NSPoint(x: 11, y: 5), controlPoint2: NSPoint(x: 9, y: 10))
            blade.close()
            NSColor.white.setStroke(); blade.lineWidth = 2; blade.stroke()
            NSColor.black.setFill(); blade.fill()
            let handle = NSBezierPath(roundedRect: NSRect(x: 7, y: 16, width: 10, height: 6), xRadius: 2, yRadius: 2)
            NSColor.white.setStroke(); handle.lineWidth = 2; handle.stroke()
            NSColor.black.setFill(); handle.fill()
            return true
        }
        image.accessibilityDescription = "Knife cursor for cutting noodles"
        return NSCursor(image: image, hotSpot: NSPoint(x: 12, y: 21))
    }
}
