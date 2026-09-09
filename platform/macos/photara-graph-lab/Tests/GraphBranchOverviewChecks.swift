import AppKit
import SwiftUI

extension GraphLabChecks {
    func branchChecks(zoom: Double, style: PhotaraGraphNoodleStyle) async {
        reset(zoom: zoom, style: style)
        await focusCanvas()
        let original = "fixture-assets-input"
        let point = controller.defaultKnot(connectionID: original)
        mouse(.leftMouseDown, screen(point), flags: [.option])
        mouse(.leftMouseUp, screen(point), flags: [.option])
        let before = controller.document
        let route = before.connections[0].routingPointID!
        for ending in ["empty", "output", "same-node", "escape", "focus", "tool"] {
            mouse(.leftMouseDown, screen(point))
            Self.check(controller.wire?.source == assets && controller.wire?.routingPointID == route
                       && controller.wire?.originalConnection == nil && Self.near(controller.wireStart!, point),
                       "Routing point immediately owns a branch preview with its upstream source")
            mouse(.leftMouseDragged, port(layer1))
            if ending == "escape" { key(53, "\u{1b}") }
            if ending == "focus" { await loseAndRestoreFocus() }
            if ending == "tool" { key(16, "y") }
            let end = ending == "output" ? port(result) :
                (ending == "same-node" ? port(assets) : (ending == "empty" ? CGPoint(x: 8, y: 8) : port(layer1)))
            mouse(.leftMouseUp, end)
            if ending == "tool" { key(16, "y", up: true) }
            Self.check(controller.document == before, "Cancelled/invalid branch preserves document: \(ending)")
            idle("branch \(ending)")
        }
        drag(from: screen(point), to: port(layer1))
        Self.check(connected(assets, layer1) && controller.document.connections.count == 2,
                   "Branch creates a distinct output-to-input edge")
        Self.check(controller.document.connections.allSatisfy { $0.routingPointID == route }
                   && controller.document.routingPoints.count == 1,
                   "Branches share one document routing point")
        let branched = controller.document
        drag(from: screen(point), to: port(layer1))
        Self.check(controller.document == branched, "Repeated routing-point drop is idempotent")
        let moved = CGPoint(x: point.x + 7, y: point.y + 12)
        mouse(.leftMouseDown, screen(point), flags: [.option])
        mouse(.leftMouseDragged, screen(moved), flags: [.option])
        Self.check(controller.document == branched && controller.document.connections.allSatisfy {
            Self.near(controller.knot(of: $0)!, moved)
        }, "Shared knot preview moves all branches without persisting")
        mouse(.leftMouseUp, screen(moved), flags: [.option])
        Self.check(Self.near(controller.document.routingPoints[0].position.cgPoint, moved),
                   "Option-drag commits the shared point once")
        // A horizontal slice below the junction intersects only the descending branch.
        cut(CGPoint(x: -210, y: -15), CGPoint(x: 60, y: -15))
        Self.check(controller.document.connections.count == 1 && connected(assets, input)
                   && controller.document.routingPoints.first?.id == route,
                   "Cutting one branch preserves its sibling and shared point")
        Self.check(controller.document.routingPoints.first?.isJunction == true,
                   "Surviving branch retains its junction geometry")
        drag(from: screen(moved), to: port(layer1))
        Self.check(contextMenu("Disconnect", at: port(input)), "Disconnect original routed edge")
        Self.check(connected(assets, layer1) && controller.document.connections.count == 1
                   && controller.document.routingPoints.first?.id == route,
                   "Original edge deletion cannot orphan a surviving branch")
        drag(from: screen(moved), to: port(input))
        cut(CGPoint(x: -50, y: -150), CGPoint(x: -50, y: -40))
        Self.check(controller.document.connections.isEmpty && controller.document.routingPoints.isEmpty,
                   "Cutting the common upstream segment removes every attached edge and orphan routing point")
        await postCut(zoom: zoom)

        reset(zoom: zoom, style: style)
        controller.setKnot(point, connectionID: original)
        drag(from: screen(point), to: port(layer1))
        mouse(.leftMouseDown, screen(point)); mouse(.leftMouseUp, screen(point))
        key(51, "\u{7f}")
        Self.check(controller.document.connections.count == 2 && controller.document.routingPoints.isEmpty
                   && controller.document.connections.allSatisfy { $0.routingPointID == nil },
                   "Deleting shared knot keeps each semantic connection")
        idle("routing branches complete")
    }

    func overviewSnapshotCheck(_ appearance: String) async {
        reset(zoom: 1.8, style: .curved)
        controller.overviewPolicy = .always
        let id = controller.document.connections[0].id
        controller.setKnot(CGPoint(x: -20, y: -65), connectionID: id)
        drag(from: screen(CGPoint(x: -20, y: -65)), to: port(layer1))
        controller.scroll(by: CGSize(width: 190, height: 90))
        try? await Task.sleep(for: .milliseconds(250))
        snapshot("\(appearance)-overview-branches")
        controller.overviewSizeFraction = 0.28
        await Self.settle()
        snapshot("\(appearance)-overview-large")
        for (index, position) in PhotaraGraphOverviewPosition.allCases.enumerated() {
            controller.overviewPosition = position
            controller.overviewCornerRadius = Double(index) * 12
            controller.overviewSizeFraction = 0.10 + Double(index) * 0.06
            await Self.settle()
            snapshot("\(appearance)-overview-\(position.rawValue)")
        }
        controller.overviewSizeFraction = 0.16
        controller.overviewCornerRadius = 12
        controller.overviewPosition = .topRight
        controller.overviewPolicy = .whileZooming
    }

    func overviewChecks() async {
        reset(zoom: 1, style: .curved)
        Self.check(!controller.overviewVisible, "Default overview is hidden at rest")
        controller.zoom(to: 1.2, anchor: CGPoint(x: 100, y: 100))
        Self.check(controller.overviewVisible, "Zoom in shows overview immediately")
        try? await Task.sleep(for: .milliseconds(400))
        controller.zoom(to: 0.8, anchor: CGPoint(x: 100, y: 100))
        try? await Task.sleep(for: .milliseconds(400))
        Self.check(controller.overviewVisible, "Zoom out renews idle dismissal without flicker")
        try? await Task.sleep(for: .milliseconds(350))
        Self.check(!controller.overviewVisible, "Overview dismisses after zoom idle")
        controller.beginZoomGesture()
        try? await Task.sleep(for: .milliseconds(720))
        Self.check(controller.overviewVisible, "Held zoom gesture stays visible through a pause")
        controller.endZoomGesture()
        try? await Task.sleep(for: .milliseconds(720))
        Self.check(!controller.overviewVisible, "Ending zoom gesture schedules dismissal")
        for policy in PhotaraGraphOverviewPolicy.allCases {
            controller.overviewPolicy = policy
            controller.zoom(to: 1.3, anchor: .zero)
            Self.check(controller.overviewVisible == (policy != .never), "Overview policy while zooming: \(policy)")
            controller.cancel(resetTool: true)
            Self.check(controller.overviewVisible == (policy == .always), "Overview policy after cancellation: \(policy)")
        }
        controller.overviewPolicy = .whileZooming
        controller.zoom(to: 1.4, anchor: .zero)
        await loseAndRestoreFocus()
        Self.check(!controller.overviewVisible && !controller.zoomActive, "Focus loss clears zoom overview")
        controller.zoom(to: 1.2, anchor: .zero)
        key(16, "y")
        Self.check(!controller.overviewVisible, "Tool change clears zoom overview")
        key(16, "y", up: true)
        controller.overviewPolicy = .always
        for position in PhotaraGraphOverviewPosition.allCases {
            controller.overviewPosition = position
            await Self.settle()
            let left = position == .topLeft || position == .bottomLeft
            let point = CGPoint(x: left ? 50 : controller.viewport.width - 50,
                                y: position.isBottom ? controller.viewport.height - 90 : 50)
            Self.check(window.contentView?.hitTest(surface.convert(point, to: window.contentView)) === surface,
                       "Overview passes pointer input through at \(position.title)")
        }
        controller.overviewPosition = .topRight
        let hitPoint = CGPoint(x: controller.viewport.width - 50, y: 50)
        await Self.settle()
        Self.check(window.contentView?.hitTest(surface.convert(hitPoint, to: window.contentView)) === surface,
                   "Overview never intercepts native pointer input")
        drag(from: hitPoint, to: CGPoint(x: hitPoint.x - 20, y: hitPoint.y + 10))
        for zoom in [0.55, 1.0, 1.8] {
            controller.zoom(to: zoom, anchor: .zero)
            let view = controller.overviewSnapshot
            let expectedOrigin = CGPoint(x: (-controller.viewport.width / 2 - controller.camera.pan.width) / zoom,
                                         y: (-controller.viewport.height / 2 - controller.camera.pan.height) / zoom)
            Self.check(Self.near(view.viewport.origin, expectedOrigin)
                       && abs(view.viewport.width - controller.viewport.width / zoom) < 0.00001,
                       "Overview viewport independently matches camera")
            for fraction in [0.10, 0.16, 0.28] {
                let size = PhotaraGraphOverviewSizing.size(in: controller.viewport, fraction: fraction)
                let transform = view.transform(in: size)
                let visible = view.viewport.applying(transform)
                let graph = view.graphBounds.applying(transform)
                let frame = CGRect(origin: .zero, size: size)
                Self.check(frame.contains(visible) && frame.contains(graph), "Overview fits graph plus entire viewport")
                Self.check(abs(transform.a - transform.d) < 0.00001, "Overview preserves aspect ratio")
                let sample = CGPoint(x: -89, y: 133)
                Self.check(Self.near(sample.applying(transform).applying(transform.inverted()), sample), "Overview map roundtrip")
            }
        }
        let node = controller.document.nodes[0]
        let originalBounds = controller.overviewSnapshot.graphBounds
        mouse(.leftMouseDown, screen(node.position.cgPoint))
        mouse(.leftMouseDragged, screen(CGPoint(x: node.position.x - 80, y: node.position.y)))
        Self.check(controller.overviewSnapshot.graphBounds.minX < originalBounds.minX,
                   "Visible overview follows live node preview")
        key(53, "\u{1b}"); mouse(.leftMouseUp, .zero)
        controller.setKnot(CGPoint(x: -800, y: -500), connectionID: controller.document.connections[0].id)
        Self.check(controller.overviewSnapshot.graphBounds.minX <= -804.5,
                   "Overview includes routing geometry outside nodes")
        for (windowWidth, expectedWidth) in [(700.0, 144.0), (1400, 224), (4000, 360)] {
            let size = PhotaraGraphOverviewSizing.size(in: CGSize(width: windowWidth, height: 900), fraction: 0.16)
            Self.check(abs(size.width - expectedWidth) < 0.001, "Responsive overview size \(windowWidth)")
        }
        for windowSize in [CGSize(width: 1100, height: 1400), CGSize(width: 1800, height: 800), CGSize(width: 1400, height: 1400)] {
            window.setContentSize(windowSize)
            window.center()
            await Self.settle()
            let viewport = surface.bounds.size
            Self.check(controller.viewport == viewport, "Native window resize delivers canvas geometry")
            for fraction in [0.10, 0.16, 0.28] {
                let size = PhotaraGraphOverviewSizing.size(in: controller.viewport, fraction: fraction)
                Self.check(abs(size.width / size.height - viewport.width / viewport.height) < 0.00001,
                           "Overview aspect ratio follows live graph viewport resize")
                let map = controller.overviewSnapshot
                Self.check(abs(map.viewport.width / map.viewport.height - viewport.width / viewport.height) < 0.00001,
                           "Viewport outline follows resized graph aspect ratio")
            }
        }
        window.setContentSize(NSSize(width: 1801, height: 1000))
        window.setContentSize(NSSize(width: 1800, height: 1000))
        await Self.settle()
        reset(zoom: 1, style: .curved)
    }

    func preferenceChecks() {
        let key = "graph-lab.visual-preferences.v1"
        guard let data = UserDefaults.standard.data(forKey: key),
              var preferences = try? JSONDecoder().decode(GraphLabSavedPreferences.self, from: data),
              let original = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            Self.check(false, "Load current saved preferences for migration verification"); return
        }
        Self.check(PhotaraGraphOverviewPolicy(savedValue: nil) == .whileZooming
                   && PhotaraGraphOverviewPolicy(savedValue: "unknown") == .whileZooming,
                   "Old/unknown overview preference defaults compatibly")
        Self.check(PhotaraGraphOverviewSizing.fraction(nil) == 0.16 && PhotaraGraphOverviewSizing.fraction(.nan) == 0.16,
                   "Missing/invalid overview size uses readable default")
        Self.check(PhotaraGraphOverviewPosition(savedValue: nil) == .topRight
                   && PhotaraGraphOverviewPosition(savedValue: "future") == .topRight
                   && PhotaraGraphOverviewSizing.cornerRadius(nil) == 12
                   && PhotaraGraphOverviewSizing.cornerRadius(.nan) == 12,
                   "Old/unknown overview position and rounding migrate compatibly")
        for policy in PhotaraGraphOverviewPolicy.allCases {
          for position in PhotaraGraphOverviewPosition.allCases {
            preferences.overviewPosition = position.rawValue
            preferences.overviewCornerRadius = 27
            preferences.overviewPolicy = policy.rawValue
            preferences.overviewSizeFraction = 0.23
            let encoded = try! JSONEncoder().encode(preferences)
            let decoded = try! JSONDecoder().decode(GraphLabSavedPreferences.self, from: encoded)
            let values = try! JSONSerialization.jsonObject(with: encoded) as! [String: Any]
            Self.check(PhotaraGraphOverviewPolicy(savedValue: decoded.overviewPolicy) == policy
                       && decoded.overviewSizeFraction == 0.23
                       && PhotaraGraphOverviewPosition(savedValue: decoded.overviewPosition) == position
                       && decoded.overviewCornerRadius == 27, "Overview policy/size/position/rounding preference roundtrip")
            for (field, value) in original where !["overviewPolicy", "overviewSizeFraction", "overviewPosition", "overviewCornerRadius"].contains(field) {
                Self.check(NSDictionary(dictionary: [field: value]).isEqual(to: [field: values[field] ?? NSNull()]),
                           "Preference migration retains authored field \(field)")
            }
        }
          }
        var legacy = try! JSONSerialization.jsonObject(with: JSONEncoder().encode(GraphLabFixtures.document)) as! [String: Any]
        legacy.removeValue(forKey: "routingPoints")
        var edges = legacy["connections"] as! [[String: Any]]
        edges[0]["knot"] = ["x": -20.0, "y": -60.0]
        legacy["connections"] = edges
        let migrated = try! JSONDecoder().decode(PhotaraGraphDocument.self, from: JSONSerialization.data(withJSONObject: legacy))
        Self.check(migrated.routingPoints.count == 1 && migrated.knot(of: migrated.connections[0]) == .init(x: -20, y: -60),
                   "Legacy connection-owned knot migrates into shared routing DTO")
        let roundtrip = try! JSONDecoder().decode(PhotaraGraphDocument.self, from: JSONEncoder().encode(migrated))
        Self.check(roundtrip == migrated, "Shared routing DTO preserves stable identity through JSON")
        var invalid = migrated
        invalid.routingPoints[0] = .init(id: invalid.routingPoints[0].id, source: result, position: .zero)
        Self.check((try? invalid.validate()) == nil, "DTO rejects routing source inconsistent with attached edge")
        invalid = migrated
        invalid.connections.removeAll()
        Self.check((try? invalid.validate()) == nil, "DTO rejects orphan routing point")
    }
}
