import AppKit
import SwiftUI

private enum RandomGraphGesture: String, CaseIterable {
    case connect, abandonWire, duplicate, replaceInput, rewire, abandonRewire
    case moveNode, pan, middlePan, wheelZoom, pinchZoom, trackpadPan, sliderMinimum, sliderMaximum
    case addKnot, moveKnot, deleteKnot, deleteConnection, cut, cancelCut
    case escapeWire, focusLoss, switchTool, escapeNode, escapePan, escapeKnot
    case contextConnect, contextDisconnect, contextAddKnot, contextDeleteKnot
    case rejectOutputOutput, rejectInputInput
    case branch, branchCancel, branchInvalid, branchDuplicate, branchFocusLoss, branchEscape, branchToolSwitch, branchDelete, branchCut
    case overviewPolicy, overviewSize, overviewPosition, overviewRounding, resizeWindow
}

extension GraphLabChecks {
    func randomChecks(appearance: Binding<PhotaraThemeAppearance>) async {
        let options = Self.options
        let seeds: [UInt64] = options.seed.map { [$0] } ?? [0x50484f54415241, 0xC0FFEE, 20260909]
        var coverage: [String: Int] = [:]
        for mode in [PhotaraThemeAppearance.light, .dark] where options.appearance == nil || options.appearance == mode.rawValue {
            appearance.wrappedValue = mode
            await Self.settle()
            for style in PhotaraGraphNoodleStyle.allCases where options.style == nil || options.style == style.rawValue {
                for seed in seeds {
                    let counts = await randomSequence(seed: seed, steps: options.steps, style: style, appearance: mode.rawValue)
                    for (key, count) in counts { coverage[key, default: 0] += count }
                }
            }
        }
        GraphTestLog.write("RANDOM COVERAGE: \(coverage.sorted { $0.key < $1.key })")
        if options.seed == nil {
            for action in RandomGraphGesture.allCases {
                Self.check(coverage[action.rawValue, default: 0] > 0, "Random harness exercised \(action.rawValue)")
            }
        }
    }

    private func randomSequence(seed: UInt64, steps: Int, style: PhotaraGraphNoodleStyle, appearance: String) async -> [String: Int] {
        window.setContentSize(NSSize(width: 1800, height: 1000))
        window.center()
        await Self.settle()
        reset(zoom: 1, style: style)
        await Self.settle()
        var oracle = GraphGestureOracle(viewport: controller.viewport)
        oracle.curved = style == .curved
        var rng = GraphSeededRandom(state: seed)
        var coverage: [String: Int] = [:]
        var expectedPolicy = PhotaraGraphOverviewPolicy.whileZooming
        var expectedOverviewSize = controller.overviewSizeFraction
        var expectedPosition = controller.overviewPosition
        var expectedRounding = controller.overviewCornerRadius
        let prefix = "seed=\(seed) \(appearance)/\(style.rawValue)"
        GraphTestLog.write("RANDOM: \(prefix), \(steps) actions")
        var trace = ["Replay: verify-interactions.sh --random-only --seed \(seed) --steps \(steps) --appearance \(appearance) --style \(style.rawValue)"]
        let url = URL(fileURLWithPath: "/tmp/photara-graph-verification/random-\(seed)-\(appearance)-\(style.rawValue).log")
        // A successful replay must not leave an older failure snapshot beside
        // its new passing trace.
        try? FileManager.default.removeItem(at: url.appendingPathExtension("actual.json"))
        defer { try? (trace.joined(separator: "\n") + "\n").write(to: url, atomically: true, encoding: .utf8) }
        func p(_ port: PhotaraGraphPortID) -> CGPoint { oracle.screen(oracle.point(port)) }
        func verify(_ step: String) -> Bool {
            var errors = oracle.mismatches(document: controller.document, actualZoom: controller.camera.zoom, actualPan: controller.camera.pan)
            if controller.overviewPolicy != expectedPolicy || controller.overviewSizeFraction != expectedOverviewSize
                || controller.overviewPosition != expectedPosition || controller.overviewCornerRadius != expectedRounding {
                errors.append("overview preferences changed unexpectedly")
            }
            if expectedPolicy == .always && !controller.overviewVisible { errors.append("always overview disappeared") }
            if expectedPolicy == .never && controller.overviewVisible { errors.append("disabled overview visible") }
            if controller.interaction != .idle || controller.wire != nil || !controller.hiddenConnections.isEmpty || surface.capturedButton != nil || controller.knifeMode {
                errors.append("transient gesture/tool/capture survived completion")
            }
            for node in oracle.nodes {
                for direction in [PhotaraGraphPortDirection.input, .output] {
                    let rows = node.ports.filter { $0.direction == direction }
                    let active = Set(rows.enumerated().compactMap { index, port -> Int? in
                        let id = PhotaraGraphPortID(node: node.id, key: port.id)
                        return oracle.edges.contains { direction == .input ? $0.destination == id : $0.source == id } ? index : nil
                    })
                    if active != controller.activePorts(for: node, direction: direction) { errors.append("stuck/missing bead on \(node.id)/\(direction)") }
                }
                for port in node.ports {
                    let id = PhotaraGraphPortID(node: node.id, key: port.id)
                    if !Self.near(controller.portPoint(id)!, oracle.point(id)) { errors.append("endpoint geometry for \(id)") }
                }
            }
            Self.check(errors.isEmpty, "Random \(prefix), \(step): \(errors.joined(separator: "; "))")
            if !errors.isEmpty {
                trace.append("FAIL: \(errors)")
                if let data = try? JSONEncoder().encode(controller.document) { try? data.write(to: url.appendingPathExtension("actual.json")) }
            }
            return errors.isEmpty
        }
        func ensureEdge() -> Bool {
            if oracle.edges.isEmpty {
                trace.append("  prerequisite: connect Assets -> Input")
                drag(from: p(assets), to: p(input))
                oracle.connect(assets, input)
                return verify("prerequisite connection")
            }
            return true
        }
        func revealGraph() -> Bool {
            // Zooming and scrolling may put a future target outside the window
            // or beneath the floating slider. Pan it into view just as a user
            // must before clicking. Never synthesize a down on an invisible port.
            var points = oracle.nodes.flatMap { node -> [CGPoint] in
                let rows = max(1, max(node.ports.filter { $0.direction == .input }.count,
                                      node.ports.filter { $0.direction == .output }.count))
                let halfHeight = (74 + Double(rows) * 25) / 2
                return [CGPoint(x: node.position.x - 112, y: node.position.y - halfHeight),
                        CGPoint(x: node.position.x + 112, y: node.position.y + halfHeight)]
            }
            points += oracle.edges.compactMap(\.knot)
            let screens = points.map(oracle.screen)
            func delta(_ values: [CGFloat], lower: CGFloat, upper: CGFloat) -> CGFloat {
                let minimum = values.min()!, maximum = values.max()!
                if minimum < lower { return lower - minimum }
                if maximum > upper { return upper - maximum }
                return 0
            }
            let dx = delta(screens.map(\.x), lower: 24, upper: oracle.viewport.width - 24)
            let dy = delta(screens.map(\.y), lower: 24, upper: oracle.viewport.height - 78)
            guard abs(dx) > 0.001 || abs(dy) > 0.001 else { return true }
            trace.append("  prerequisite: middle-pan visible targets by (\(dx), \(dy))")
            drag(from: CGPoint(x: 20, y: 65), to: CGPoint(x: 20 + dx, y: 65 + dy), middle: true)
            oracle.pan.x += dx; oracle.pan.y += dy
            return verify("prerequisite reveal targets")
        }
        func chooseEdge() -> GraphGestureOracle.Edge { oracle.edges[rng.index(oracle.edges.count)] }
        func wheel(_ delta: Int32, precise: Bool, anchor: CGPoint) -> NSEvent {
            let event = CGEvent(scrollWheelEvent2Source: nil, units: precise ? .pixel : .line,
                                wheelCount: 2, wheel1: delta, wheel2: precise ? 2 : 0, wheel3: 0)!
            let nativePoint = window.convertPoint(toScreen: surface.convert(anchor, to: nil))
            event.location = CGPoint(x: nativePoint.x, y: NSScreen.screens.first!.frame.height - nativePoint.y)
            return NSEvent(cgEvent: event)!
        }
        // Begin with a shuffled complete action vocabulary, then freely mix it.
        var vocabulary = RandomGraphGesture.allCases
        for i in vocabulary.indices.reversed() where i > 0 { vocabulary.swapAt(i, rng.index(i + 1)) }
        guard verify("initial") else { return coverage }
        for index in 0..<steps {
            let action = index < vocabulary.count ? vocabulary[index] : RandomGraphGesture.allCases[rng.index(RandomGraphGesture.allCases.count)]
            trace.append("\(index): \(action.rawValue)")
            await focusCanvas()
            guard revealGraph() else { return coverage }
            let failuresBeforeStep = Self.failures.count
            var performed = true
            switch action {
            case .resizeWindow:
                let sizes = [NSSize(width: 1600, height: 1000), NSSize(width: 1800, height: 1100),
                             NSSize(width: 2000, height: 1200), NSSize(width: 1800, height: 1000)]
                let size = sizes[rng.index(sizes.count)]
                window.setContentSize(size)
                window.center()
                await Self.settle()
                // Native layout dimensions are an external input to the model,
                // just like the initial viewport. Never read the controller's
                // camera or document to manufacture the expected result.
                oracle.viewport = surface.bounds.size
                let preview = PhotaraGraphOverviewSizing.size(in: oracle.viewport, fraction: expectedOverviewSize)
                Self.check(controller.viewport == oracle.viewport
                           && abs(preview.width / preview.height - oracle.viewport.width / oracle.viewport.height) < 0.00001,
                           "Random native window resize updates viewport and overview aspect")
                trace.append("  native window \(size), canvas \(oracle.viewport)")
            case .overviewPolicy:
                expectedPolicy = PhotaraGraphOverviewPolicy.allCases[rng.index(3)]
                controller.cancel()
                controller.overviewPolicy = expectedPolicy
                trace.append("  overview policy \(expectedPolicy.rawValue)")
                Self.check(controller.overviewVisible == (expectedPolicy == .always), "Random overview policy at rest")
            case .overviewPosition:
                expectedPosition = PhotaraGraphOverviewPosition.allCases[rng.index(4)]
                controller.overviewPosition = expectedPosition
                trace.append("  overview position \(expectedPosition.rawValue)")
            case .overviewRounding:
                expectedRounding = rng.value(0, 36)
                controller.overviewCornerRadius = expectedRounding
                trace.append("  overview rounding \(expectedRounding)")
            case .overviewSize:
                expectedOverviewSize = rng.value(0.10, 0.28)
                controller.overviewSizeFraction = expectedOverviewSize
                trace.append("  overview fraction \(expectedOverviewSize)")
                let size = PhotaraGraphOverviewSizing.size(in: oracle.viewport, fraction: expectedOverviewSize)
                Self.check(abs(size.width - min(360, max(144, oracle.viewport.width * expectedOverviewSize))) < 0.00001,
                           "Random overview responds to relative size setting")
            case .branch, .branchCancel, .branchInvalid, .branchDuplicate, .branchFocusLoss, .branchEscape, .branchToolSwitch, .branchDelete, .branchCut:
                guard ensureEdge() else { return coverage }
                if !oracle.edges.contains(where: { $0.knot != nil }),
                   let edge = oracle.edges.first(where: { oracle.selectablePoint($0) != nil }),
                   let point = oracle.selectablePoint(edge) {
                    trace.append("  prerequisite: routing knot on \(edge.destination) at \(point)")
                    mouse(.leftMouseDown, oracle.screen(point), flags: [.option])
                    mouse(.leftMouseUp, oracle.screen(point), flags: [.option])
                    oracle.setKnot(point, edgeID: edge.id!)
                    guard verify("prerequisite branch point") else { return coverage }
                }
                let choices = oracle.edges.filter { edge in
                    guard let point = edge.knot, !oracle.nodeContains(point), !oracle.portContains(point) else { return false }
                    return !oracle.edges.contains { other in
                        other.group != edge.group && (other.knot.map { hypot($0.x - point.x, $0.y - point.y) < 13 } ?? false)
                    }
                }
                guard !choices.isEmpty else { performed = false; break }
                let edge = choices[rng.index(choices.count)], point = edge.knot!
                let inputs = oracle.inputs.filter { $0.node != edge.source.node }
                let destination = action == .branchDuplicate ? edge.destination : inputs[rng.index(inputs.count)]
                trace.append("  route \(edge.group!) at \(point), upstream \(edge.source), destination \(destination)")
                mouse(.leftMouseDown, oracle.screen(point))
                mouse(.leftMouseDragged, p(destination))
                Self.check(controller.wire?.source == edge.source && Self.near(controller.wireStart ?? .zero, point),
                           "Random branch owns its shared source and origin")
                if action == .branchFocusLoss { await loseAndRestoreFocus() }
                if action == .branchEscape { key(53, "\u{1b}") }
                if action == .branchToolSwitch { key(16, "y") }
                var end = p(destination)
                if action == .branchCancel { end = CGPoint(x: 8, y: 8) }
                if action == .branchInvalid { end = p(oracle.outputs[rng.index(oracle.outputs.count)]) }
                mouse(.leftMouseUp, end)
                if action == .branchToolSwitch { key(16, "y", up: true) }
                if [.branch, .branchDuplicate, .branchDelete, .branchCut].contains(action) {
                    oracle.branch(edge, to: destination)
                    guard verify("branch creation") else { return coverage }
                }
                if action == .branchDelete {
                    Self.check(contextMenu("Disconnect", at: p(destination)), "Random branch deletion menu")
                    oracle.edges.removeAll { $0.destination == destination }
                }
                if action == .branchCut {
                    var slice: (Double, Set<String>)?
                    for _ in 0..<40 {
                        let samples = oracle.polyline(chooseEdge())
                        let x = Double(samples[rng.index(samples.count)].x)
                        if let ids = oracle.cutIDs(atX: x), !ids.isEmpty { slice = (x, ids); break }
                    }
                    if let (x, ids) = slice {
                        trace.append("  branch cut x=\(x), \(ids.count) crossed edges")
                        let sx = oracle.screen(CGPoint(x: x, y: 0)).x
                        key(16, "y")
                        drag(from: CGPoint(x: sx, y: 5), to: CGPoint(x: sx, y: oracle.viewport.height - 3))
                        key(16, "y", up: true)
                        oracle.edges.removeAll { $0.id.map(ids.contains) ?? false }
                    } else { performed = false }
                }
            case .connect, .duplicate, .replaceInput, .contextConnect:
                var source = oracle.outputs[rng.index(oracle.outputs.count)]
                let candidates = oracle.inputs.filter { $0.node != source.node }
                var destination = candidates[rng.index(candidates.count)]
                if action == .duplicate {
                    guard ensureEdge() else { return coverage }
                    let edge = chooseEdge(); source = edge.source; destination = edge.destination
                } else if action == .replaceInput {
                    guard ensureEdge() else { return coverage }
                    destination = chooseEdge().destination
                    let sources = oracle.outputs.filter { $0.node != destination.node }
                    source = sources[rng.index(sources.count)]
                }
                trace.append("  \(source) -> \(destination)")
                if action == .contextConnect {
                    let node = oracle.nodes.first { $0.id == source.node }!
                    let label = node.ports.first { $0.id == source.key }!.label
                    Self.check(contextMenu("Connect \(node.title) / \(label) Here", at: p(destination), control: rng.index(2) == 0), "Random native connect menu")
                } else { drag(from: p(source), to: p(destination)) }
                oracle.connect(source, destination)
            case .abandonWire, .escapeWire, .switchTool, .focusLoss:
                let source = oracle.outputs[rng.index(oracle.outputs.count)]
                let choices = oracle.inputs.filter { $0.node != source.node }
                let hover = choices[rng.index(choices.count)]
                trace.append("  \(source), hover \(hover)")
                mouse(.leftMouseDown, p(source)); mouse(.leftMouseDragged, p(hover))
                Self.check(controller.wire?.source == source, "Random \(prefix)/\(index): immediate wire ownership")
                if action == .escapeWire { key(53, "\u{1b}") }
                if action == .switchTool { key(16, "y") }
                if action == .focusLoss {
                    await loseAndRestoreFocus()
                }
                // Abandonment releases elsewhere *without* a final drag event.
                // Cancellations get a late valid hover/up to detect resurrection.
                if action == .abandonWire { mouse(.leftMouseUp, CGPoint(x: 15, y: 15)) }
                else { mouse(.leftMouseDragged, p(hover)); mouse(.leftMouseUp, p(hover)) }
                if action == .switchTool { key(16, "y", up: true) }
            case .rejectOutputOutput:
                let source = oracle.outputs[rng.index(oracle.outputs.count)]
                let outputs = oracle.outputs.filter { $0.node != source.node }
                let destination = outputs[rng.index(outputs.count)]
                trace.append("  invalid output pair \(source) -> \(destination)")
                drag(from: p(source), to: p(destination))
            case .rejectInputInput:
                // Connected-input dragging is a valid rewire of an existing
                // output->input edge. Use an unconnected input for this attempt.
                let source = oracle.inputs[rng.index(oracle.inputs.count)]
                if oracle.edges.contains(where: { $0.destination == source }) {
                    Self.check(contextMenu("Disconnect", at: p(source)), "Prepare unconnected input for invalid-pair attempt")
                    oracle.edges.removeAll { $0.destination == source }
                    guard verify("prerequisite disconnected input") else { return coverage }
                }
                let inputs = oracle.inputs.filter { $0.node != source.node }
                let destination = inputs[rng.index(inputs.count)]
                trace.append("  invalid input pair \(source) -> \(destination)")
                drag(from: p(source), to: p(destination))
            case .rewire, .abandonRewire:
                guard ensureEdge() else { return coverage }
                let edge = chooseEdge()
                let choices = oracle.inputs.filter { $0.node != edge.source.node }
                let destination = choices[rng.index(choices.count)]
                trace.append("  rewire \(edge.destination) -> \(destination)")
                mouse(.leftMouseDown, p(edge.destination))
                mouse(.leftMouseDragged, p(destination))
                mouse(.leftMouseUp, action == .abandonRewire ? CGPoint(x: 15, y: 15) : p(destination))
                if action == .rewire { oracle.rewire(edge, to: destination) }
            case .moveNode, .escapeNode:
                let nodeIndex = rng.index(oracle.nodes.count)
                let node = oracle.nodes[nodeIndex]
                let original = GraphLabFixtures.document.nodes.first { $0.id == node.id }!.position
                let target = CGPoint(x: original.x + rng.value(-30, 30), y: original.y)
                trace.append("  \(node.id) -> \(target)")
                mouse(.leftMouseDown, oracle.screen(node.position.cgPoint))
                mouse(.leftMouseDragged, oracle.screen(target))
                Self.check(Self.near(controller.document.nodes.first { $0.id == node.id }!.position.cgPoint, node.position.cgPoint), "Random \(prefix)/\(index): node preview is not persistent")
                if action == .escapeNode { key(53, "\u{1b}") }
                else { oracle.nodes[nodeIndex].position = .init(target) }
                mouse(.leftMouseUp, oracle.screen(target))
            case .pan, .middlePan, .escapePan:
                let delta = CGPoint(x: rng.value(-8, 8) - oracle.pan.x, y: rng.value(-8, 8) - oracle.pan.y)
                let start = CGPoint(x: 20, y: 65), end = CGPoint(x: 20 + delta.x, y: 65 + delta.y)
                trace.append("  delta \(delta)")
                let middle = action == .middlePan
                mouse(middle ? .otherMouseDown : .leftMouseDown, start)
                mouse(middle ? .otherMouseDragged : .leftMouseDragged, end)
                if action == .escapePan { key(53, "\u{1b}") }
                else { oracle.pan.x += delta.x; oracle.pan.y += delta.y }
                mouse(middle ? .otherMouseUp : .leftMouseUp, end)
            case .wheelZoom, .pinchZoom:
                let anchor = oracle.screen(CGPoint(x: rng.value(-4, 4), y: rng.value(-4, 4)))
                let amount = rng.value(-0.16, 0.16)
                trace.append("  anchor \(anchor), amount \(amount)")
                if action == .wheelZoom {
                    let native = wheel(amount > 0 ? 8 : -8, precise: false, anchor: anchor)
                    let actualAnchor = surface.convert(native.locationInWindow, from: nil)
                    surface.scrollWheel(with: native)
                    oracle.setZoom(oracle.zoom * exp(native.scrollingDeltaY * 0.012), at: actualAnchor)
                } else {
                    let event = MagnifyEvent(anchor: surface.convert(anchor, to: nil), amount: amount)
                    surface.magnify(with: event)
                    oracle.setZoom(oracle.zoom * (1 + amount), at: anchor)
                }
            case .trackpadPan:
                let native = wheel(oracle.pan.y > 0 ? -3 : 3, precise: true, anchor: CGPoint(x: 30, y: 80))
                surface.scrollWheel(with: native)
                oracle.pan.x += native.scrollingDeltaX
                oracle.pan.y += native.scrollingDeltaY
            case .sliderMinimum, .sliderMaximum:
                let center = oracle.viewport.width / 2
                let start = CGPoint(x: center + 60, y: oracle.viewport.height - 34)
                let end = CGPoint(x: center + (action == .sliderMinimum ? -75 : 155), y: start.y)
                await nativeControlDrag(from: start, to: end)
                oracle.setZoom(action == .sliderMinimum ? 0.55 : 1.8, at: CGPoint(x: center, y: oracle.viewport.height / 2))
                await Self.settle()
            case .contextDisconnect:
                guard ensureEdge() else { return coverage }
                let edge = chooseEdge()
                let port = rng.index(2) == 0 ? edge.source : edge.destination
                Self.check(contextMenu("Disconnect", at: p(port), control: rng.index(2) == 0), "Random native disconnect menu")
                oracle.edges.removeAll { $0.source == port || $0.destination == port }
            case .addKnot, .deleteConnection, .contextAddKnot:
                guard ensureEdge() else { return coverage }
                let candidates = oracle.edges.compactMap { edge -> (GraphGestureOracle.Edge, CGPoint)? in
                    guard action == .deleteConnection || edge.knot == nil, let point = oracle.selectablePoint(edge) else { return nil }
                    return (edge, point)
                }
                if candidates.isEmpty { performed = false; break }
                let (edge, point) = candidates[rng.index(candidates.count)]
                trace.append("  edge to \(edge.destination), at \(point)")
                if action == .contextAddKnot {
                    Self.check(contextMenu("Add Routing Knot", at: oracle.screen(point), control: rng.index(2) == 0), "Random native routing menu")
                } else {
                    let flags: NSEvent.ModifierFlags = action == .addKnot ? [.option] : []
                    mouse(.leftMouseDown, oracle.screen(point), flags: flags)
                    mouse(.leftMouseUp, oracle.screen(point), flags: flags)
                }
                if action == .addKnot || action == .contextAddKnot {
                    oracle.setKnot(point, edgeID: edge.id!)
                } else {
                    key(51, "\u{7f}")
                    oracle.edges.removeAll { $0.id == edge.id }
                }
            case .moveKnot, .deleteKnot, .escapeKnot, .contextDeleteKnot:
                guard ensureEdge() else { return coverage }
                if !oracle.edges.contains(where: { $0.knot != nil }),
                   let edge = oracle.edges.first(where: { oracle.selectablePoint($0) != nil }),
                   let point = oracle.selectablePoint(edge) {
                    trace.append("  prerequisite: routing knot on \(edge.destination) at \(point)")
                    mouse(.leftMouseDown, oracle.screen(point), flags: [.option])
                    mouse(.leftMouseUp, oracle.screen(point), flags: [.option])
                    oracle.setKnot(point, edgeID: edge.id!)
                    guard verify("prerequisite knot") else { return coverage }
                }
                let choices = oracle.edges.filter { edge in
                    guard let k = edge.knot, !oracle.nodeContains(k), !oracle.portContains(k) else { return false }
                    return !oracle.edges.contains { other in other.id != edge.id && other.group != edge.group && (other.knot.map { hypot($0.x - k.x, $0.y - k.y) < 13 } ?? false) }
                }
                if choices.isEmpty { performed = false; break }
                let edge = choices[rng.index(choices.count)], start = edge.knot!
                let target = CGPoint(x: min(300, max(-300, start.x + rng.value(-15, 15))), y: min(200, max(-150, start.y + rng.value(-25, 25))))
                if action == .contextDeleteKnot {
                    Self.check(contextMenu("Delete Knot", at: oracle.screen(start), control: rng.index(2) == 0), "Random native delete-knot menu")
                    oracle.setKnot(nil, edgeID: edge.id!)
                    break
                }
                mouse(.leftMouseDown, oracle.screen(start), flags: [.option])
                if action == .deleteKnot {
                    mouse(.leftMouseUp, oracle.screen(start)); key(51, "\u{7f}")
                    oracle.setKnot(nil, edgeID: edge.id!)
                } else {
                    mouse(.leftMouseDragged, oracle.screen(target))
                    if action == .escapeKnot { key(53, "\u{1b}") }
                    else { oracle.setKnot(target, edgeID: edge.id!) }
                    mouse(.leftMouseUp, oracle.screen(target))
                }
                trace.append("  knot \(start) -> \(target)")
            case .cut, .cancelCut:
                guard ensureEdge() else { return coverage }
                var slice: (Double, Set<String>)?
                for _ in 0..<40 {
                    let edge = chooseEdge(), points = oracle.polyline(edge)
                    let x = points[points.count / 2].x + rng.value(-25, 25)
                    if let ids = oracle.cutIDs(atX: x), !ids.isEmpty { slice = (x, ids); break }
                }
                guard let (x, ids) = slice else { performed = false; break }
                trace.append("  slice x=\(x), expected \(ids.count) edges")
                let screenX = oracle.screen(CGPoint(x: x, y: 0)).x
                key(16, "y")
                mouse(.leftMouseDown, CGPoint(x: screenX, y: 5))
                mouse(.leftMouseDragged, CGPoint(x: screenX, y: oracle.viewport.height - 3))
                if action == .cancelCut { key(53, "\u{1b}") }
                else { oracle.edges.removeAll { $0.id.map(ids.contains) ?? false } }
                mouse(.leftMouseUp, CGPoint(x: screenX, y: oracle.viewport.height - 3))
                key(16, "y", up: true)
            }
            if performed { coverage[action.rawValue, default: 0] += 1 }
            else { trace.append("  no eligible visible target; no-op") }
            guard verify("step \(index) \(action.rawValue)") else { return coverage }
            if Self.failures.count != failuresBeforeStep {
                trace.append("FAIL during gesture: \(Self.failures.dropFirst(failuresBeforeStep))")
                return coverage
            }
            trace.append("  OK: \(oracle.edges.count) edges, zoom \(oracle.zoom), pan \(oracle.pan)")
            // Let SwiftUI render between actions; this is a real native window,
            // not a tight controller-only loop that bypasses view updates.
            try? await Task.sleep(for: .milliseconds(3))
        }
        return coverage
    }
}
