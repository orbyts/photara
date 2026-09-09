import AppKit
import SwiftUI

@main
struct GraphLabVerificationApp: App {
    @State private var appearance = PhotaraThemeAppearance.dark
    private let theme = try! PhotaraThemeDocument.load(from: Bundle.main.url(forResource: "photara-default", withExtension: "json")!)
    init() { UserDefaults.standard.addSuite(named: "com.photara.graph-lab") }
    var body: some Scene {
        WindowGroup("Graph Lab Interaction Verification") {
            GraphLabView(appearance: $appearance)
                .environment(\.photaraTheme, theme.resolved(for: appearance))
                .preferredColorScheme(appearance == .dark ? .dark : .light)
                .frame(minWidth: 1080, minHeight: 700)
                .task {
                    await GraphLabChecks.run(appearance: $appearance)
                }
        }
    }
}

@MainActor
final class GraphLabChecks {
    static let options = GraphVerificationOptions()
    static var started = false
    static var assertions = 0
    static var failures: [String] = []
    static func check(_ condition: @autoclosure () -> Bool, _ message: String) {
        assertions += 1
        if !condition() { failures.append(message); GraphTestLog.write("FAIL: \(message)") }
    }
    static func near(_ a: CGPoint, _ b: CGPoint, tolerance: Double = 0.001) -> Bool {
        hypot(a.x - b.x, a.y - b.y) < tolerance
    }
    static func settle() async { try? await Task.sleep(for: .milliseconds(35)) }
    static func findSurface(_ view: NSView) -> PhotaraGraphEventView? {
        if let surface = view as? PhotaraGraphEventView { return surface }
        return view.subviews.lazy.compactMap(findSurface).first
    }
    static func run(appearance: Binding<PhotaraThemeAppearance>) async {
        guard !started else { return }; started = true
        try? await Task.sleep(for: .milliseconds(700))
        guard let window = NSApp.windows.first(where: { $0.contentView.flatMap(findSurface) != nil }),
              let surface = window.contentView.flatMap(findSurface) else {
            GraphTestLog.write("FAIL: Could not locate production event surface"); exit(1)
        }
        window.setContentSize(NSSize(width: 1800, height: 1000))
        window.center()
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
        await settle()
        window.makeFirstResponder(surface)
        let suite = GraphLabChecks(window: window, surface: surface)
        if options.randomOnly {
            await suite.randomChecks(appearance: appearance)
            finish()
        }
        suite.documentChecks()
        for mode in [PhotaraThemeAppearance.light, .dark] {
            appearance.wrappedValue = mode
            await settle()
            for style in PhotaraGraphNoodleStyle.allCases {
                for zoom in [0.55, 1.0, 1.8] {
                    GraphTestLog.write("MATRIX: \(mode.rawValue), \(style.rawValue), \(zoom)")
                    await suite.matrix(zoom: zoom, style: style)
                }
            }
            suite.reset(zoom: 1, style: .curved)
            await settle()
            suite.snapshot("\(mode.rawValue)-baseline")
            suite.drag(from: suite.port(suite.assets), to: suite.port(suite.layer1))
            await settle()
            suite.snapshot("\(mode.rawValue)-fanout")
            suite.controller.setKnot(CGPoint(x: -35, y: -20), connectionID: "fixture-assets-input")
            await settle()
            suite.snapshot("\(mode.rawValue)-routing")
            suite.controller.removeConnections(Set(suite.controller.document.connections.map(\.id)))
            await settle()
            suite.snapshot("\(mode.rawValue)-disconnected")
            suite.mouse(.leftMouseDown, suite.port(suite.assets))
            suite.mouse(.leftMouseDragged, suite.screen(CGPoint(x: -20, y: -20)))
            await settle()
            suite.snapshot("\(mode.rawValue)-wire-preview")
            suite.key(53, "\u{1b}")
            suite.mouse(.leftMouseUp, suite.port(suite.input))
        }
        await suite.lifecycleChecks()
        await suite.nativeCameraChecks()
        await suite.ownershipAndMenuChecks()
        await suite.randomChecks(appearance: appearance)
        suite.fuzzChecks()
        suite.benchmark()
        finish()
    }

    static func finish() -> Never {
        let result = "\(assertions) assertions; \(failures.count) failures\n" + failures.joined(separator: "\n")
        GraphTestLog.write(result)
        try? result.write(toFile: "/tmp/photara-graph-verification/result.txt", atomically: true, encoding: .utf8)
        exit(failures.isEmpty ? 0 : 1)
    }

    let window: NSWindow
    let surface: PhotaraGraphEventView
    var controller: PhotaraGraphInteractionController { surface.controller }
    let assets = PhotaraGraphPortID(node: "source", key: "out.Assets")
    let input = PhotaraGraphPortID(node: "transform", key: "in.Input")
    let mask = PhotaraGraphPortID(node: "transform", key: "in.Mask")
    let layer1 = PhotaraGraphPortID(node: "composite", key: "in.Layer 1")
    let layer2 = PhotaraGraphPortID(node: "composite", key: "in.Layer 2")
    let result = PhotaraGraphPortID(node: "transform", key: "out.Result")
    init(window: NSWindow, surface: PhotaraGraphEventView) { self.window = window; self.surface = surface }
    func screen(_ world: CGPoint) -> CGPoint { controller.camera.screen(world, in: controller.viewport) }
    func port(_ id: PhotaraGraphPortID) -> CGPoint { screen(controller.portPoint(id)!) }
    func reset(zoom: Double, style: PhotaraGraphNoodleStyle) {
        try! controller.replaceDocument(GraphLabFixtures.document)
        controller.center(positions: [:], selectedNode: "transform")
        controller.configure(.init(portOffset: 0, noodleStyle: style))
        controller.zoom(to: zoom, anchor: CGPoint(x: controller.viewport.width / 2, y: controller.viewport.height / 2))
        window.makeFirstResponder(surface)
    }
    func mouse(_ type: NSEvent.EventType, _ point: CGPoint, flags: NSEvent.ModifierFlags = []) {
        let location = surface.convert(point, to: nil)
        let event = NSEvent.mouseEvent(with: type, location: location, modifierFlags: flags,
                                      timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: window.windowNumber,
                                      context: nil, eventNumber: 1, clickCount: 1, pressure: type == .leftMouseUp ? 0 : 1)!
        if type == .otherMouseDown || type == .otherMouseDragged || type == .otherMouseUp || type == .rightMouseDown || type == .rightMouseUp {
            let cg = event.cgEvent!
            cg.setIntegerValueField(.mouseEventButtonNumber, value: (type == .rightMouseDown || type == .rightMouseUp) ? 1 : 2)
            let middle = NSEvent(cgEvent: cg)!
            window.sendEvent(middle)
        } else { window.sendEvent(event) }
    }
    func key(_ code: UInt16, _ chars: String, up: Bool = false) {
        let event = NSEvent.keyEvent(with: up ? .keyUp : .keyDown, location: .zero, modifierFlags: [],
                                    timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: window.windowNumber,
                                    context: nil, characters: chars, charactersIgnoringModifiers: chars,
                                    isARepeat: false, keyCode: code)!
        window.sendEvent(event)
    }
    func drag(from start: CGPoint, to end: CGPoint, middle: Bool = false) {
        mouse(middle ? .otherMouseDown : .leftMouseDown, start)
        for step in 1...8 {
            let t = Double(step) / 8
            mouse(middle ? .otherMouseDragged : .leftMouseDragged, CGPoint(x: start.x + (end.x - start.x) * t, y: start.y + (end.y - start.y) * t))
        }
        mouse(middle ? .otherMouseUp : .leftMouseUp, end)
    }
    func focusCanvas() async {
        NSApp.activate(ignoringOtherApps: true)
        window.makeKeyAndOrderFront(nil)
        for _ in 0..<100 {
            if NSApp.isActive && window.isKeyWindow { break }
            try? await Task.sleep(for: .milliseconds(10))
        }
        window.makeKeyAndOrderFront(nil)
        window.makeFirstResponder(surface)
        Self.check(NSApp.isActive && window.isKeyWindow, "Verification window acquired native input focus")
    }

    func loseAndRestoreFocus() async {
        let other = NSWindow(contentRect: NSRect(x: 120, y: 120, width: 140, height: 100), styleMask: [.titled], backing: .buffered, defer: false)
        other.makeKeyAndOrderFront(nil)
        for _ in 0..<100 {
            if other.isKeyWindow && !window.isKeyWindow { break }
            try? await Task.sleep(for: .milliseconds(10))
        }
        Self.check(other.isKeyWindow && !window.isKeyWindow, "Native focus-loss precondition occurred")
        Self.check(controller.interaction == .idle && surface.capturedButton == nil, "Native focus-loss notification cancels capture")
        other.orderOut(nil)
        await focusCanvas()
    }

    /// SwiftUI's native slider consults physical button state while tracking.
    /// Use public HID event posting when Accessibility permission is available,
    /// rather than pretending synchronous NSEvent calls test that tracking loop.
    func nativeControlDrag(from start: CGPoint, to end: CGPoint) async {
        guard AXIsProcessTrusted() else {
            Self.check(false, "Native slider drag requires Accessibility permission for the verification process")
            return
        }
        let oldPointer = CGEvent(source: nil)!.location
        await focusCanvas()
        await Self.settle()
        let source = CGEventSource(stateID: .hidSystemState)
        for step in 0...10 {
            let t = Double(step) / 10
            let point = CGPoint(x: start.x + (end.x - start.x) * t, y: start.y + (end.y - start.y) * t)
            let native = window.convertPoint(toScreen: surface.convert(point, to: nil))
            let global = CGPoint(x: native.x, y: NSScreen.screens.first!.frame.height - native.y)
            let type: CGEventType = step == 0 ? .leftMouseDown : (step == 10 ? .leftMouseUp : .leftMouseDragged)
            let event = CGEvent(mouseEventSource: source, mouseType: type, mouseCursorPosition: global, mouseButton: .left)!
            event.setIntegerValueField(.mouseEventClickState, value: 1)
            event.post(tap: .cghidEventTap)
            try? await Task.sleep(for: .milliseconds(20))
        }
        CGEvent(mouseEventSource: source, mouseType: .mouseMoved, mouseCursorPosition: oldPointer, mouseButton: .left)?.post(tap: .cghidEventTap)
        await Self.settle()
    }

    func idle(_ label: String) {
        Self.check(controller.interaction == .idle && controller.wire == nil && surface.capturedButton == nil, "\(label): clean release")
        Self.check((try? controller.document.validate()) != nil, "\(label): valid document")
    }
    func connected(_ source: PhotaraGraphPortID, _ destination: PhotaraGraphPortID) -> Bool {
        controller.document.connections.contains { $0.source == source && $0.destination == destination }
    }
    func cut(_ a: CGPoint, _ b: CGPoint) {
        key(16, "y")
        Self.check(controller.knifeMode, "Y activates knife through first responder")
        drag(from: screen(a), to: screen(b))
        key(16, "y", up: true)
        idle("cut")
    }

    func matrix(zoom: Double, style: PhotaraGraphNoodleStyle) async {
        reset(zoom: zoom, style: style)
        Self.check(controller.viewport.width > 1000, "Canvas receives actual native viewport")
        // Every visible port and its row-bounded invisible target picks itself.
        for node in controller.document.nodes {
            for p in node.ports {
                let id = PhotaraGraphPortID(node: node.id, key: p.id)
                let center = controller.portPoint(id)!
                Self.check(controller.hitTest(screen(center)) == .port(id), "Port center \(id)")
                Self.check(controller.hitTest(screen(CGPoint(x: center.x + 8, y: center.y + 9))) == .port(id), "Expanded port target \(id)")
            }
        }
        let beforeInvalid = controller.document
        drag(from: port(assets), to: port(result))
        Self.check(controller.document == beforeInvalid, "Output-to-output drop is invalid")
        drag(from: port(mask), to: port(layer1))
        Self.check(controller.document == beforeInvalid, "Unconnected input-to-input drag creates no edge")
        drag(from: port(input), to: port(result))
        Self.check(controller.document == beforeInvalid, "Input rewire dropped on output preserves original")
        drag(from: port(result), to: port(mask))
        Self.check(controller.document == beforeInvalid, "Same-node output-to-input drop is invalid")
        idle("invalid direction drops")
        controller.removeConnections(Set(controller.document.connections.map(\.id)))
        mouse(.leftMouseDown, port(assets))
        Self.check(controller.wire?.source == assets, "Output pulls immediately on down")
        Self.check(controller.activePorts(for: controller.document.nodes[0], direction: .output) == [0], "Only active preview bead")
        mouse(.leftMouseDragged, port(input))
        // Release elsewhere without a final drag tests stale-hover fallback.
        mouse(.leftMouseUp, screen(CGPoint(x: -350, y: 280)))
        Self.check(controller.document.connections.isEmpty, "Empty release after valid hover cancels")
        idle("invalid new wire")
        drag(from: port(assets), to: port(input))
        Self.check(connected(assets, input), "Create Disk Folder to Rotate")
        drag(from: port(assets), to: port(layer1))
        Self.check(controller.document.connections.count == 2, "Output fanout")
        drag(from: port(assets), to: port(input))
        Self.check(controller.document.connections.count == 2, "Identical pair is idempotent")
        drag(from: port(result), to: port(layer1))
        Self.check(connected(result, layer1) && !connected(assets, layer1), "Replace occupied input")
        let original = controller.document
        drag(from: port(input), to: screen(CGPoint(x: -350, y: 280)))
        Self.check(controller.document == original, "Invalid rewire preserves original exactly")
        drag(from: port(input), to: port(mask))
        Self.check(connected(assets, mask) && !connected(assets, input), "Connected input rewires")
        idle("connections")
        await Self.settle()

        // Move every node with attached wires through the same code path and
        // verify port geometry at each frame before persistence on release.
        for node in controller.document.nodes {
            let startWorld = node.position.cgPoint
            let start = screen(startWorld)
            let beforePorts = node.ports.map { controller.portPoint(.init(node: node.id, key: $0.id))! }
            mouse(.leftMouseDown, start)
            mouse(.leftMouseDragged, CGPoint(x: start.x - 12 * zoom, y: start.y - 8 * zoom))
            let delta = controller.position(of: node)
            Self.check(Self.near(delta, CGPoint(x: startWorld.x - 12, y: startWorld.y - 8)), "Move \(node.title)")
            for (index, p) in node.ports.enumerated() {
                let expected = CGPoint(x: beforePorts[index].x - 12, y: beforePorts[index].y - 8)
                Self.check(Self.near(controller.portPoint(.init(node: node.id, key: p.id))!, expected), "Live endpoint glued to \(node.title)")
            }
            Self.check(controller.document.nodes.first { $0.id == node.id }!.position.cgPoint == startWorld, "Node preview excluded from document")
            mouse(.leftMouseUp, CGPoint(x: start.x - 12 * zoom, y: start.y - 8 * zoom))
            idle("move \(node.title)")
        }

        reset(zoom: zoom, style: style)
        // Baseline curve is nearly horizontal at y=-94; slice its middle.
        cut(CGPoint(x: -20, y: -150), CGPoint(x: -20, y: -30))
        Self.check(controller.document.connections.isEmpty, "Cut baseline noodle")
        Self.check(controller.activePorts(for: controller.document.nodes[0], direction: .output).isEmpty, "Cut removes output bead")
        await postCut(zoom: zoom)
        reset(zoom: zoom, style: style)
        drag(from: port(assets), to: port(mask))
        drag(from: port(result), to: port(layer1)) // Should survive left-side cut.
        cut(CGPoint(x: -20, y: -155), CGPoint(x: -20, y: -45))
        Self.check(controller.document.connections.count == 1 && connected(result, layer1), "Slice cuts every crossed edge and preserves other edge")
        await postCut(zoom: zoom)

        reset(zoom: zoom, style: style)
        let midpoint = controller.defaultKnot(connectionID: "fixture-assets-input")
        mouse(.leftMouseDown, screen(midpoint), flags: [.option])
        mouse(.leftMouseUp, screen(midpoint), flags: [.option])
        Self.check(controller.document.connections[0].knot != nil, "Option-click adds routing knot")
        let moved = CGPoint(x: midpoint.x, y: midpoint.y + 35)
        drag(from: screen(midpoint), to: screen(moved))
        Self.check(Self.near(controller.document.connections[0].knot!.cgPoint, moved), "Routing knot moves and persists")
        key(51, "\u{7f}")
        Self.check(controller.document.connections[0].knot == nil, "Delete selected knot")
        controller.setKnot(moved, connectionID: "fixture-assets-input")
        cut(CGPoint(x: moved.x - 20, y: moved.y), CGPoint(x: moved.x + 20, y: moved.y))
        Self.check(controller.document.connections.isEmpty, "Cut removes connection and its knot")
        await postCut(zoom: zoom)
        idle("matrix complete")
    }

    func postCut(zoom: Double) async {
        let empty = screen(CGPoint(x: -340, y: 180))
        let before = controller.camera.pan
        drag(from: empty, to: CGPoint(x: empty.x + 8, y: empty.y + 5))
        Self.check(controller.camera.pan != before, "Pan immediately after cut")
        let anchor = screen(CGPoint(x: -150, y: -150))
        let world = controller.camera.world(anchor, in: controller.viewport)
        controller.zoom(to: zoom == 1.8 ? 1.6 : zoom + 0.1, anchor: anchor)
        Self.check(Self.near(controller.camera.world(anchor, in: controller.viewport), world), "Anchored zoom after cut")
        let node = controller.document.nodes[0]
        drag(from: screen(node.position.cgPoint), to: screen(CGPoint(x: node.position.x - 6, y: node.position.y)))
        Self.check(controller.document.nodes[0].position != node.position, "Node moves immediately after cut")
        drag(from: port(assets), to: port(input))
        Self.check(connected(assets, input), "Same output creates immediately after cut")
        drag(from: port(input), to: port(mask))
        Self.check(connected(assets, mask), "Input rewires immediately after cut")
        idle("post-cut chain")
        await Self.settle()
    }

    func lifecycleChecks() async {
        for cancellation in ["escape", "focus", "tool", "resize"] {
            for gesture in ["wire", "rewire", "node", "pan", "middle", "knot", "knife"] {
                reset(zoom: 1, style: .curved)
                var start = port(assets)
                if gesture == "rewire" { start = port(input) }
                if gesture == "node" { start = screen(controller.document.nodes[0].position.cgPoint) }
                if gesture == "pan" || gesture == "middle" { start = screen(CGPoint(x: -350, y: 260)) }
                if gesture == "knot" {
                    let knot = controller.defaultKnot(connectionID: "fixture-assets-input")
                    controller.setKnot(knot, connectionID: "fixture-assets-input")
                    start = screen(knot)
                }
                if gesture == "knife" { key(16, "y"); start = screen(CGPoint(x: -20, y: -150)) }
                let original = controller.document
                let camera = controller.camera
                let down: NSEvent.EventType = gesture == "middle" ? .otherMouseDown : .leftMouseDown
                let move: NSEvent.EventType = gesture == "middle" ? .otherMouseDragged : .leftMouseDragged
                let up: NSEvent.EventType = gesture == "middle" ? .otherMouseUp : .leftMouseUp
                mouse(down, start)
                let end = CGPoint(x: start.x + 25, y: start.y + 80)
                mouse(move, end)
                Self.check(controller.interaction != .idle, "\(gesture) started before \(cancellation)")
                switch cancellation {
                case "escape": key(53, "\u{1b}")
                case "focus":
                    let other = NSWindow(contentRect: NSRect(x: 100, y: 100, width: 120, height: 120), styleMask: [.titled], backing: .buffered, defer: false)
                    other.makeKeyAndOrderFront(nil)
                    await Self.settle()
                    other.orderOut(nil)
                    window.makeKeyAndOrderFront(nil)
                    window.makeFirstResponder(surface)
                case "tool": controller.setKnifeMode(!controller.knifeMode)
                default: surface.setFrameSize(NSSize(width: surface.frame.width - 1, height: surface.frame.height))
                }
                Self.check(controller.interaction == .idle && controller.wire == nil, "\(cancellation) cancels \(gesture)")
                Self.check(controller.document == original && controller.camera == camera, "\(cancellation) rolls back \(gesture)")
                // Tail events after cancellation cannot restart any gesture.
                mouse(move, port(input)); mouse(up, port(input))
                Self.check(controller.document == original && controller.interaction == .idle, "Cancelled \(gesture) cannot resurrect from tail events")
                controller.setKnifeMode(false)
                drag(from: port(assets), to: port(mask))
                Self.check(connected(assets, mask), "Next wire after \(cancellation)/\(gesture)")
                idle("lifecycle")
            }
        }
        // Y release while slicing cancels. Y activation while wiring consumes
        // the remaining old sequence, then the next knife down works.
        reset(zoom: 1, style: .curved)
        mouse(.leftMouseDown, port(assets)); mouse(.leftMouseDragged, port(mask))
        key(16, "y")
        mouse(.leftMouseDragged, port(input)); mouse(.leftMouseUp, port(input))
        Self.check(controller.document.connections.count == 1 && controller.wire == nil, "Tool switch during wire has one owner")
        mouse(.leftMouseDown, screen(CGPoint(x: -20, y: -150)))
        mouse(.leftMouseDragged, screen(CGPoint(x: -20, y: -30)))
        key(16, "y", up: true)
        mouse(.leftMouseUp, screen(CGPoint(x: -20, y: -30)))
        Self.check(controller.document.connections.count == 1, "Early Y release cancels slice transaction")
        idle("early Y release")
    }

    func nativeCameraChecks() async {
        reset(zoom: 1, style: .curved)
        let start = screen(CGPoint(x: -300, y: 250))
        let original = controller.camera.pan
        mouse(.otherMouseDown, start)
        Self.check(controller.isPanning && surface.capturedButton == 2, "Middle button captured")
        mouse(.otherMouseDragged, CGPoint(x: start.x + 45, y: start.y + 20))
        mouse(.otherMouseUp, CGPoint(x: start.x + 45, y: start.y + 20))
        Self.check(controller.camera.pan == CGSize(width: original.width + 45, height: original.height + 20), "Native middle pan")
        idle("middle")
        // CG scroll events retain AppKit's precise-vs-wheel semantics. Their
        // coordinates are native global screen coordinates, converted by NSEvent.
        let anchor = screen(CGPoint(x: -200, y: -200))
        let global = window.convertPoint(toScreen: surface.convert(anchor, to: nil))
        let screenHeight = NSScreen.screens.first!.frame.height
        for unit in [CGScrollEventUnit.line, .pixel] {
            let before = controller.camera
            let event = CGEvent(scrollWheelEvent2Source: nil, units: unit, wheelCount: 2, wheel1: 4, wheel2: 2, wheel3: 0)!
            event.location = CGPoint(x: global.x, y: screenHeight - global.y)
            let native = NSEvent(cgEvent: event)!
            surface.scrollWheel(with: native)
            if unit == .line { Self.check(controller.camera.zoom != before.zoom, "Native wheel zoom") }
            else { Self.check(controller.camera.pan != before.pan && controller.camera.zoom == before.zoom, "Precise trackpad scroll pans") }
        }
        // NSEvent has no public magnify-event constructor. A typed NSEvent
        // test double exercises the production responder without private APIs.
        let pinch = MagnifyEvent(anchor: surface.convert(anchor, to: nil), amount: 0.12)
        let beforeZoom = controller.camera.zoom
        surface.magnify(with: pinch)
        Self.check(controller.camera.zoom > beforeZoom, "Native magnify responder")
        // Native SwiftUI slider: hit routing must bypass the event surface.
        await Self.settle()
        let sliderPoint = CGPoint(x: controller.viewport.width / 2 + 66, y: controller.viewport.height - 34)
        let hit = window.contentView?.hitTest(surface.convert(sliderPoint, to: window.contentView))
        Self.check(hit !== surface, "Zoom control gets native pointer priority")
        let beforeSlider = controller.camera.zoom
        await nativeControlDrag(from: sliderPoint, to: CGPoint(x: sliderPoint.x - 50, y: sliderPoint.y))
        await Self.settle()
        Self.check(controller.camera.zoom != beforeSlider && surface.capturedButton == nil, "Native zoom slider changes camera without canvas capture")
        window.makeFirstResponder(surface)
    }

    func ownershipAndMenuChecks() async {
        reset(zoom: 1, style: .curved)
        let originalCamera = controller.camera
        // A disconnected input owns its press even though there is no wire to
        // rewire. Noodle presses also cannot fall through to canvas panning.
        drag(from: port(mask), to: screen(CGPoint(x: -300, y: 200)))
        Self.check(controller.camera == originalCamera && controller.wire == nil, "Disconnected input never pans")
        let middle = controller.defaultKnot(connectionID: "fixture-assets-input")
        drag(from: screen(middle), to: screen(CGPoint(x: middle.x + 35, y: middle.y + 50)))
        Self.check(controller.camera == originalCamera, "Noodle drag never pans")
        mouse(.leftMouseDown, port(assets))
        let active = controller.interaction
        mouse(.otherMouseDown, screen(CGPoint(x: -300, y: 200)))
        mouse(.otherMouseDragged, screen(CGPoint(x: -200, y: 200)))
        mouse(.otherMouseUp, screen(CGPoint(x: -200, y: 200)))
        Self.check(controller.interaction == active && controller.camera == originalCamera, "Second button cannot steal wire ownership")
        let secondaryMenu = MenuDriver()
        secondaryMenu.expected = "Disconnect"
        secondaryMenu.install()
        mouse(.rightMouseDown, port(input))
        mouse(.rightMouseUp, port(input))
        secondaryMenu.uninstall()
        Self.check(secondaryMenu.observedTitles.isEmpty && controller.interaction == active
                   && controller.document.connections.count == 1 && surface.capturedButton == 0,
                   "Right button cannot open a menu or steal an active wire")
        surface.magnify(with: MagnifyEvent(anchor: .zero, amount: 0.2))
        Self.check(controller.camera == originalCamera, "Pinch cannot move geometry during captured pointer sequence")
        mouse(.leftMouseUp, CGPoint(x: -50, y: -50))
        Self.check(controller.document.connections.count == 1, "Release outside canvas cancels new wire")
        idle("outside release")
        // Merge an already-connected identical destination by native input drag.
        drag(from: port(assets), to: port(mask))
        drag(from: port(input), to: port(mask))
        Self.check(controller.document.connections.count == 1 && connected(assets, mask), "Native rewire to identical pair merges")
        reset(zoom: 1, style: .curved)
        let driver = MenuDriver()
        driver.install()
        func menu(_ title: String, at point: CGPoint, control: Bool = false) {
            driver.expected = title
            driver.performed = false
            mouse(control ? .leftMouseDown : .rightMouseDown, point, flags: control ? [.control] : [])
            Self.check(driver.performed, "Native context menu: \(title)")
            // Matching release is harmless and cannot create a second gesture.
            mouse(control ? .leftMouseUp : .rightMouseUp, point)
        }
        menu("Add Routing Knot", at: screen(middle))
        Self.check(controller.document.connections[0].knot != nil, "Native menu adds knot")
        menu("Delete Knot", at: screen(middle), control: true)
        Self.check(controller.document.connections[0].knot == nil, "Control-click menu removes knot")
        menu("Disconnect", at: port(input))
        Self.check(controller.document.connections.isEmpty, "Port menu disconnects")
        menu("Connect Disk Folder / Assets Here", at: port(input))
        Self.check(connected(assets, input), "Port menu uses compatible sources")
        menu("Add Routing Knot", at: screen(middle))
        let id = controller.document.connections[0].id
        controller.select(.noodle(id))
        key(51, "\u{7f}")
        Self.check(controller.document.connections.isEmpty, "Delete connection removes its routing state")
        driver.uninstall()
        var extended = GraphLabFixtures.document
        extended.nodes.append(.init(id: "future-node", kind: "future-kind", title: "Future", subtitle: "Data-defined node",
            ports: [.init(id: "input-a", direction: .input, dataType: "graph-value", label: "Repeated label"),
                    .init(id: "input-b", direction: .input, dataType: "graph-value", label: "Repeated label"),
                    .init(id: "input-wrong", direction: .input, dataType: "other", label: "Other type")],
            position: .init(x: 450, y: 100)))
        try! controller.replaceDocument(extended)
        await Self.settle()
        let incompatible = PhotaraGraphPortID(node: "future-node", key: "input-wrong")
        let beforeIncompatible = controller.document
        drag(from: port(assets), to: port(incompatible))
        Self.check(controller.document == beforeIncompatible, "Native drop rejects incompatible port types")
        let futurePort = PhotaraGraphPortID(node: "future-node", key: "input-b")
        drag(from: port(assets), to: port(futurePort))
        Self.check(connected(assets, futurePort), "Unknown node and repeated labels inherit native port behavior")
        drag(from: screen(CGPoint(x: 450, y: 100)), to: screen(CGPoint(x: 470, y: 105)))
        Self.check(Self.near(controller.document.nodes.last!.position.cgPoint, CGPoint(x: 470, y: 105)), "Unknown node inherits native dragging")
        idle("menus")
        await Self.settle()
    }

    func contextMenu(_ title: String, at point: CGPoint, control: Bool = false) -> Bool {
        let driver = MenuDriver()
        driver.expected = title
        driver.install()
        defer { driver.uninstall() }
        mouse(control ? .leftMouseDown : .rightMouseDown, point, flags: control ? [.control] : [])
        mouse(control ? .leftMouseUp : .rightMouseUp, point)
        if !driver.performed {
            GraphTestLog.write("MENU DELIVERY: expected=\(title), hit=\(controller.hitTest(point)), menus=\(driver.observedTitles), focus=\(window.isKeyWindow)/\(NSApp.isActive), control=\(control)")
        }
        return driver.performed
    }

    func documentChecks() {
        let document = GraphLabFixtures.document
        let encoded = try! JSONEncoder().encode(document)
        let decoded = try! JSONDecoder().decode(PhotaraGraphDocument.self, from: encoded)
        Self.check(decoded == document, "DTO JSON roundtrip")
        var invalid = document
        invalid.connections.append(document.connections[0])
        Self.check((try? invalid.validate()) == nil, "Reject duplicate DTO")
        invalid = document
        invalid.connections[0].destination = .init(node: "missing", key: "missing")
        Self.check((try? invalid.validate()) == nil, "Reject dangling DTO")
        for (source, destination) in [(assets, result), (input, mask), (input, assets), (result, mask)] {
            var rejected = document
            Self.check(rejected.connect(from: source, to: destination) == nil && rejected == document, "Document rejects invalid direction/self-node pair")
            rejected.connections = [.init(id: "invalid-edge", source: source, destination: destination)]
            Self.check((try? rejected.validate()) == nil, "DTO rejects invalid direction/self-node pair")
        }
        var doc = document
        let originalID = doc.connections[0].id
        let second = doc.connect(from: assets, to: mask)!
        _ = doc.connect(from: assets, to: mask, replacing: originalID)
        Self.check(doc.connections.count == 1 && doc.connections[0].id == originalID && !doc.connections.contains { $0.id == second }, "Rewire merges identical pair atomically")
        var extra = document.nodes[1]
        extra = .init(id: "future", kind: "unregistered-kind", title: "Future", subtitle: "Seven rows", ports: extra.ports + [.init(id: "in.extra", direction: .input, dataType: "other", label: "Extra")], position: .init(x: 450, y: 100))
        var extended = document; extended.nodes.append(extra)
        let model = PhotaraGraphInteractionController(document: extended)
        model.viewport = CGSize(width: 1800, height: 1000)
        let futurePort = PhotaraGraphPortID(node: "future", key: "in.Input")
        Self.check(model.portPoint(futurePort) != nil && model.document.canConnect(assets, futurePort), "New node inherits geometry and compatibility")
        Self.check(!model.document.canConnect(assets, .init(node: "future", key: "in.extra")), "Data type compatibility enforced")
    }

    func fuzzChecks() {
        var seed: UInt64 = 0x50686f74617261
        func random(_ upper: Double) -> Double {
            seed = seed &* 6364136223846793005 &+ 1
            return Double(seed >> 32) / Double(UInt32.max) * upper
        }
        reset(zoom: 1, style: .curved)
        let model = controller
        for _ in 0..<600 {
            let node = model.document.nodes[Int(random(2.999))]
            let start = model.camera.screen(node.position.cgPoint, in: model.viewport)
            model.pointerDown(at: start)
            for _ in 0..<3 {
                model.pointerDragged(to: CGPoint(x: random(1800), y: random(1000)))
                for a in model.document.nodes {
                    for b in model.document.nodes where a.id < b.id {
                        let aRect = PhotaraGraphGeometry.rect(of: a, at: model.position(of: a)).insetBy(dx: 0.01, dy: 0.01)
                        let bRect = PhotaraGraphGeometry.rect(of: b, at: model.position(of: b))
                        if aRect.intersects(bRect) {
                            GraphTestLog.write("COLLISION: \(a.id) \(aRect) / \(b.id) \(bRect), interaction \(model.interaction)")
                        }
                        Self.check(!aRect.intersects(bRect), "Swept node collision never overlaps")
                    }
                }
            }
            if random(1) < 0.3 { model.cancel() }
            model.pointerUp(at: CGPoint(x: random(1800), y: random(1000)))
            Self.check(model.interaction == .idle && (try? model.document.validate()) != nil, "Random sequence remains valid")
        }
    }
    func benchmark() {
        reset(zoom: 1, style: .curved)
        let start = screen(controller.document.nodes[0].position.cgPoint)
        controller.pointerDown(at: start)
        let begin = ContinuousClock.now
        for i in 0..<10000 {
            controller.pointerDragged(to: CGPoint(x: start.x - Double(i % 100), y: start.y - Double(i % 40)))
            for edge in controller.document.connections { _ = controller.path(edge) }
        }
        let duration = begin.duration(to: .now)
        controller.cancel()
        GraphTestLog.write("BENCHMARK: 10000 drag updates + attached path derivation: \(duration)")
    }
    func snapshot(_ name: String) {
        // SwiftUI Canvas and native glass are GPU layers; cacheDisplay omits
        // them. Capture the compositor's actual window instead.
        let capture = Process()
        capture.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
        capture.arguments = ["-x", "-l", String(window.windowNumber), "/tmp/photara-graph-verification/\(name).png"]
        try? capture.run()
        capture.waitUntilExit()
        Self.check(capture.terminationStatus == 0, "Native compositor snapshot \(name)")
    }

}

final class MagnifyEvent: NSEvent {
    let anchor: NSPoint
    let amount: CGFloat
    init(anchor: NSPoint, amount: CGFloat) { self.anchor = anchor; self.amount = amount; super.init() }
    required init?(coder: NSCoder) { fatalError("Not used") }
    override var type: NSEvent.EventType { .magnify }
    override var locationInWindow: NSPoint { anchor }
    override var magnification: CGFloat { amount }
}

@MainActor
private final class MenuDriver: NSObject {
    var expected = ""
    var performed = false
    var observedTitles: [[String]] = []
    func install() {
        NotificationCenter.default.addObserver(self, selector: #selector(opened(_:)), name: NSMenu.didBeginTrackingNotification, object: nil)
    }
    func uninstall() { NotificationCenter.default.removeObserver(self) }
    @objc private func opened(_ notification: Notification) {
        guard let menu = notification.object as? NSMenu else { return }
        observedTitles.append(menu.items.map(\.title))
        let timer = Timer(timeInterval: 0.04, target: self, selector: #selector(choose(_:)), userInfo: menu, repeats: false)
        RunLoop.main.add(timer, forMode: .eventTracking)
    }
    @objc private func choose(_ timer: Timer) {
        guard let menu = timer.userInfo as? NSMenu else { return }
        if let index = menu.items.firstIndex(where: { $0.title == expected }) {
            menu.performActionForItem(at: index)
            performed = true
        }
        menu.cancelTracking()
    }
}

enum GraphTestLog {
    static func write(_ message: String) {
        FileHandle.standardOutput.write(Data((message + "\n").utf8))
    }
}
