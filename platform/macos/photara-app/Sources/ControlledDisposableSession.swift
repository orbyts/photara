import AppKit
import SwiftUI

/// The actor keeps synchronous UniFFI calls off MainActor and in request order.
/// Only Rust's controlled constructor can acquire project authority.
private actor ControlledDisposableBridge: DisposableSessionBridge {
    private var session: ControlledDisposableSession?
    private let environment: [String: String]
    init(environment: [String: String]) { self.environment = environment }

    func execute(_ request: DisposableHostRequest) throws -> DisposableHostResponse {
        if session == nil {
            let config = try ControlledDisposableConfiguration.load(environment)
            session = try ControlledDisposableSession.open(manifestPath: config.manifestPath,
                                                          bindingJson: config.bindingJSON)
        }
        guard let session else { throw CocoaError(.coderInvalidValue) }
        let command: ControlledSessionCommand
        switch request.command {
        case "snapshot": command = .snapshot
        case "revalidate": command = .revalidate
        case "submit": command = .submit
        case "complete": command = .complete
        case "barrier": command = .barrier
        case "undo": command = .undo
        case "redo": command = .redo
        case "retry": command = .retry
        case "close": command = .close
        default: throw CocoaError(.coderInvalidValue)
        }
        let result = try session.execute(request: .init(id: request.id.uuidString.lowercased(),
            command: command, nodeId: request.node_id, x: request.x, y: request.y,
            expectedOwnerEpoch: request.expectedOwnerEpoch?.uuidString.lowercased(),
            expectedAttachmentGeneration: request.expectedAttachmentGeneration))
        guard result.id == request.id.uuidString.lowercased() else { throw CocoaError(.coderInvalidValue) }
        return try .init(id: request.id,
            snapshot: decode(result.snapshotJson, as: DisposableSessionSnapshot.self),
            result: decode(result.resultJson, as: DisposableHostResult.self),
            acknowledgement: decode(result.acknowledgementJson, as: DisposableOperationAcknowledgement.self),
            error: result.error)
    }

    private func decode<T: Decodable>(_ value: String?, as type: T.Type) throws -> T? {
        guard let value else { return nil }
        guard value.utf8.count <= 1_048_576 else { throw CocoaError(.coderInvalidValue) }
        return try JSONDecoder().decode(type, from: Data(value.utf8))
    }
}

@MainActor
func makeControlledDisposableModel() -> DisposableAutosaveProcess {
    DisposableAutosaveProcess(bridge: ControlledDisposableBridge(environment: ProcessInfo.processInfo.environment))
}

struct ControlledDisposableScene: Scene {
    let delegate: ControlledSessionApplicationDelegate
    @ObservedObject var model: DisposableAutosaveProcess

    var body: some Scene {
        Window("Photara — Disposable Project", id: "controlled-project") {
            ControlledDisposableEditor(model: model)
                .task { delegate.model = model; model.start() }
        }
        .windowToolbarStyle(.unified)
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("Close Project") { model.close { if $0 { NSApp.terminate(nil) } } }.keyboardShortcut("w")
            }
            CommandGroup(replacing: .saveItem) {
                Button("Save") { model.action("complete") }.keyboardShortcut("s").disabled(!model.canEdit)
            }
            CommandGroup(replacing: .undoRedo) {
                Button("Undo Move") { model.action("undo") }.keyboardShortcut("z")
                    .disabled(!model.canEdit || !model.undoAvailable)
                Button("Redo Move") { model.action("redo") }.keyboardShortcut("z", modifiers: [.command, .shift])
                    .disabled(!model.canEdit || !model.redoAvailable)
            }
        }
    }
}

private struct ControlledWindowCloseGuard: NSViewRepresentable {
    let model: DisposableAutosaveProcess
    func makeCoordinator() -> Coordinator { Coordinator(model) }
    func makeNSView(context: Context) -> NSView { NSView() }
    func updateNSView(_ view: NSView, context: Context) {
        let coordinator = context.coordinator
        DispatchQueue.main.async { [weak view] in view?.window?.delegate = coordinator }
    }
    @MainActor final class Coordinator: NSObject, NSWindowDelegate {
        let model: DisposableAutosaveProcess
        init(_ model: DisposableAutosaveProcess) { self.model = model }
        func windowShouldClose(_ sender: NSWindow) -> Bool {
            if model.closed { return true }
            model.close { success in if success { sender.performClose(nil) } }
            return false
        }
    }
}

private struct ControlledDisposableEditor: View {
    @ObservedObject var model: DisposableAutosaveProcess
    @Environment(\.colorScheme) private var colorScheme
    @State private var controller = PhotaraGraphInteractionController(document: .init(nodes: [], connections: []))
    @State private var didCenter = false

    private var document: PhotaraGraphDocument {
        .init(nodes: model.nodes.map { node in
            .init(id: node.id, kind: "controlled-position", title: node.title,
                  subtitle: "x \(node.x) · y \(node.y)", ports: [],
                  position: .init(x: Double(node.x) / 1_000, y: Double(node.y) / 1_000))
        }, connections: [])
    }

    var body: some View {
        let preset = PhotaraGraphPresentationPreset.shipped
        let palette = preset.palette(colorScheme)
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text("Graph").font(.headline)
                Spacer()
                if model.checkingStorage { Text("Checking project…") }
                if let projection = model.projection {
                    DisposableAutosaveStatusView(projection: projection, retry: model.retry)
                } else { Text("Opening the disposable project…") }
            }.padding(12)
            Text("Move nodes to try autosave and undo. Other editing commands are unavailable in this disposable project.")
                .font(.caption).foregroundStyle(.secondary).padding(.horizontal, 12)
            if let error = model.transportFailure {
                HStack {
                    Text(error).foregroundStyle(.red)
                    Button("Retry", action: model.retry).disabled(model.busy)
                }.padding(12)
            }
            if let pending = model.pendingPosition {
                Text("Pending move: \(pending.title) to x \(pending.x), y \(pending.y). This draft has not been acknowledged.")
                    .padding(12).accessibilityIdentifier("controlled-pending-position")
            }
            PhotaraGraphCanvas(toolRailPlacement: .leftTop, controller: controller,
                backgroundStyle: preset.backgroundStyle, backgroundColor: palette.graphBackground?.color,
                minorColor: palette.minor?.color, majorColor: palette.major?.color,
                noodleColor: palette.noodle?.color ?? .accentColor, knifeCursorSize: 24,
                showsToolRail: true, centerScene: { controller.center(positions: [:], selectedNode: nil) },
                addNativeNode: { _ in },
                nodeContent: { node, selected, inputs, outputs in
                    PhotaraGraphNodeView(node: node,
                        presentation: .init(category: .utilitiesControl, iconResource: "node-metadata"),
                        selected: selected, connectedInputs: inputs, connectedOutputs: outputs, preset: preset)
                }) { EmptyView() }
                .allowsHitTesting(model.canEdit)
                .accessibilityIdentifier("controlled-session-graph")
            HStack {
                Button("Undo") { model.action("undo") }.disabled(!model.canEdit || !model.undoAvailable)
                Button("Redo") { model.action("redo") }.disabled(!model.canEdit || !model.redoAvailable)
                Spacer()
                Button("Save Now") { model.action("complete") }.disabled(!model.canEdit)
                Button("Close") { model.close { if $0 { NSApp.terminate(nil) } } }
            }.padding(12)
        }
        .frame(minWidth: 820, minHeight: 540)
        .background(ControlledWindowCloseGuard(model: model).frame(width: 0, height: 0))
        .onAppear {
            controller.configure(.init(portOffset: preset.portOffset, noodleStyle: .curved))
            controller.commitMutation = { mutation in
                if case let .moveNode(id, position) = mutation,
                   let node = model.nodes.first(where: { $0.id == id }),
                   position.x.isFinite, position.y.isFinite,
                   let x = Int64(exactly: (position.x * 1_000).rounded()),
                   let y = Int64(exactly: (position.y * 1_000).rounded()) {
                    model.move(node, x: x, y: y)
                }
                // Canvas retains acknowledged coordinates while the visible
                // pending draft waits for the shared authority's response.
                return document
            }
            synchronize()
        }
        .onChange(of: model.nodes) { synchronize() }
        .onReceive(NSWorkspace.shared.notificationCenter.publisher(for: NSWorkspace.willSleepNotification)) { _ in
            model.prepareForSleep()
        }
        .onReceive(NSWorkspace.shared.notificationCenter.publisher(for: NSWorkspace.didWakeNotification)) { _ in
            model.revalidateAfterWake()
        }
    }

    private func synchronize() {
        try? controller.synchronizeDocument(document)
        if !didCenter, !model.nodes.isEmpty {
            controller.center(positions: [:], selectedNode: nil)
            didCenter = true
        }
    }
}
