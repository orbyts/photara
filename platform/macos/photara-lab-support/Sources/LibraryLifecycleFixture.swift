import SwiftUI
import AppKit

/// Review-only states. This file is compiled into labs, never the production app.
enum LibraryLifecycleScenario: String, CaseIterable, Identifiable {
    case ready, create, rename, switchLibrary, removal, offline, denied, protectedLibrary
    case pending, stale, removing, unknown, reconciling, removed
    var id: String { rawValue }
    var title: String {
        switch self {
        case .ready: "Libraries"
        case .create: "Create Library"
        case .rename: "Rename Library"
        case .switchLibrary: "Switch · unsaved work"
        case .removal: "Remove · impact review"
        case .offline: "Remove · offline"
        case .denied: "Remove · owner required"
        case .protectedLibrary: "Remove · default / last Library"
        case .pending: "Remove · pending operations"
        case .stale: "Remove · changed since review"
        case .removing: "Remove · submitting"
        case .unknown: "Remove · response lost"
        case .reconciling: "Remove · local cleanup pending"
        case .removed: "Remove · receipt"
        }
    }
}

enum LibrarySwitcherFixtureCase: String, CaseIterable, Identifiable {
    case cloud, localOnly, duplicateNames, offline, signedOut, unsigned, unsaved, inaccessible, removedTarget
    var id: String { rawValue }
    var title: String {
        switch self {
        case .cloud: "Cloud account · multiple Libraries"
        case .localOnly: "True local-only Library"
        case .duplicateNames: "Duplicate names"
        case .offline: "Offline cloud projection"
        case .signedOut: "Signed out · Google sign-in available"
        case .unsigned: "Unsigned build · local identity"
        case .unsaved: "Unsaved project"
        case .inaccessible: "Target access denied"
        case .removedTarget: "Target removed remotely"
        }
    }
}

struct SwitcherLibrary: Identifiable, Equatable {
    enum Authority: Equatable { case account(String), local }
    let id: String
    let name: String
    let authority: Authority
    var cached = true
    var accessible = true
    var removed = false
}

@MainActor
final class LibraryLifecycleFixture: ObservableObject {
    @Published var scenario: LibraryLifecycleScenario = .ready
    @Published var name = ""
    @Published var confirmation = ""
    @Published var cloud = true
    @Published var finalConfirmation = false
    private var finalArmed = false
    @Published var fixtureCase: LibrarySwitcherFixtureCase = .cloud
    @Published var activeID = "cloud-7C21"
    @Published var destination: String? = "Projects"
    @Published var pendingTarget: String?
    @Published var libraries: [SwitcherLibrary] = []
    @Published var signedIn = true
    @Published var online = true
    @Published var unsavedProject = false
    @Published var selectionGeneration = 0
    let accountID = "suhail@example.com"
    init() { configure(.cloud) }
    var active: SwitcherLibrary? { libraries.first { $0.id == activeID } }
    var authorityLabel: String {
        guard let active else { return "On This Mac" }
        switch active.authority {
        case .local: return "On This Mac"
        case .account(let account): return account + (online ? " · Cloud" : " · Cloud · Offline")
        }
    }
    var menuLibraries: [SwitcherLibrary] {
        var seen = Set<String>()
        return libraries.filter {
            guard seen.insert($0.id).inserted else { return false }
            if case .account = $0.authority { return signedIn }
            return true
        }
    }
    func title(for item: SwitcherLibrary) -> String {
        let duplicates = menuLibraries.filter { $0.name == item.name && $0.authority == item.authority }
        let title = duplicates.count > 1 ? "\(item.name) · \(item.id.suffix(4))" : item.name
        return !online && item.authority != .local && !item.cached ? title + " · Not downloaded" : title
    }
    func configure(_ fixture: LibrarySwitcherFixtureCase) {
        fixtureCase = fixture; signedIn = ![.signedOut, .unsigned].contains(fixture); online = fixture != .offline
        unsavedProject = fixture == .unsaved; pendingTarget = nil; destination = "Projects"
        activeID = [.signedOut, .unsigned, .localOnly].contains(fixture) ? "local-1001" : "cloud-7C21"
        libraries = [
            .init(id: "cloud-7C21", name: "Fieldwork", authority: .account(accountID)),
            .init(id: "cloud-4D90", name: fixture == .duplicateNames ? "Fieldwork" : "Studio", authority: .account(accountID)),
            .init(id: "local-1001", name: "Personal", authority: .local),
            .init(id: "cloud-7C21", name: "Fieldwork", authority: .account(accountID)) // same cached projection
        ]
        if fixture == .offline { libraries[1].cached = false }
        if fixture == .inaccessible { libraries[1].accessible = false }
        if fixture == .removedTarget { libraries[1].removed = true }
        cloud = active?.authority != .local
        show(.ready)
    }
    func choose(_ id: String) {
        guard id != activeID else { return }
        pendingTarget = id
        if unsavedProject { show(.switchLibrary) } else { finishSwitch() }
    }
    func finishSwitch() {
        guard let target = pendingTarget, let item = menuLibraries.first(where: { $0.id == target }) else {
            notice = "This Library is no longer available. Choose another Library."; pendingTarget = nil; return
        }
        guard item.accessible && !item.removed && (online || item.cached || item.authority == .local) else {
            show(.ready)
            notice = item.removed ? "\(item.name) was removed. \(libraryName) is still open." : "\(item.name) is unavailable. \(libraryName) is still open."
            if item.removed { libraries.removeAll { $0.id == target } }
            pendingTarget = nil; return
        }
        // Fixture checks completed: only this point changes the persisted-selection analogue.
        activeID = target; selectionGeneration += 1; cloud = item.authority != .local
        unsavedProject = false; pendingTarget = nil; destination = "Projects"; show(.ready)
    }
    func cancelSwitch() { pendingTarget = nil; show(.ready) }
    func signIn() { notice = fixtureCase == .unsigned ? "Open signed Photara to sign in with Google." : "Sign In requested through the existing account flow (fixture only)." }
    func signOut() { configure(.signedOut); notice = "Signed-out local fixture. No credentials changed." }
    @Published var notice = ""
    var libraryName: String { active?.name ?? "Library" }
    static let diskPromise = "Files remain on disk. Project packages (.photara), source photographs, archives, and NAS or cloud objects are not deleted."
    var exactName: Bool { Array(confirmation.utf8) == Array(libraryName.utf8) }
    var canReviewFinal: Bool { scenario == .removal && exactName }
    var validName: Bool {
        let value = name.trimmingCharacters(in: .whitespacesAndNewlines)
        return !value.isEmpty && value.utf8.count <= 128 && !value.unicodeScalars.contains(where: { CharacterSet.controlCharacters.contains($0) })
    }
    func show(_ state: LibraryLifecycleScenario) {
        if ![.ready, .switchLibrary, .create].contains(state) { destination = "Library Settings" }
        scenario = state; confirmation = ""; finalConfirmation = false; finalArmed = false
        name = state == .rename ? libraryName : ""
        notice = ""
    }
    func requestFinal() { if canReviewFinal { finalArmed = true; finalConfirmation = true } }
    func cancelFinal() { finalArmed = false; finalConfirmation = false }
    func confirmRemoval() {
        guard finalArmed && canReviewFinal else { return }
        finalConfirmation = false; finalArmed = false; scenario = .removing
    }
}

struct LibraryLifecycleLabView: View {
    @ObservedObject var model: LibraryLifecycleFixture
    var body: some View {
        NavigationSplitView {
            List(selection: $model.destination) {
                Section(model.libraryName) {
                    Label("Projects", systemImage: "folder").tag("Projects")
                    Label("People", systemImage: "person.2").tag("People")
                    Label("Locations", systemImage: "mappin.and.ellipse").tag("Locations")
                    Label("Location Kinds", systemImage: "tag").tag("Location Kinds")
                }
            }
            .navigationSplitViewColumnWidth(min: 220, ideal: 260, max: 320)
            .safeAreaInset(edge: .bottom) {
                HStack { LibrarySwitcherMenu(model: model).fixedSize(); Spacer(minLength: 0) }.padding()
            }
        } detail: {
            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    Text(model.scenario == .removed ? "Library removed" : (model.destination ?? "Projects")).font(.largeTitle)
                    Text(model.libraryName).font(.title2)
                    if model.destination == "Library Settings" {
                        Text(model.authorityLabel).foregroundStyle(.secondary)
                        if model.scenario == .removed {
                            Label("Catalog removal confirmed", systemImage: "checkmark.circle")
                            Text(LibraryLifecycleFixture.diskPromise)
                            Text("Receipt LL-DEMO-104 · Local catalog reconciled").font(.caption).foregroundStyle(.secondary)
                        } else {
                            Text("12 projects · Library catalog")
                            Text("Manage this Library’s name and catalog. Packages keep their existing names and locations.").foregroundStyle(.secondary)
                            HStack {
                                Button("Rename…") { model.show(.rename) }
                                Button("Remove Library…", role: .destructive) { model.show(.removal) }
                            }
                        }
                    } else {
                        Text("Your projects appear here.").foregroundStyle(.secondary)
                    }
                    if !model.notice.isEmpty { Text(model.notice).accessibilityIdentifier("lifecycle.switch-status") }
                }
                .padding(28).frame(maxWidth: .infinity, alignment: .leading)
            }
            .navigationTitle(model.destination ?? "Projects")
        }
        .sheet(isPresented: Binding(get: { model.scenario != .ready && model.scenario != .removed },
                                   set: { if !$0 { model.cancelSwitch() } })) {
            LibraryLifecycleSheet(model: model)
        }
    }
}

// SwiftUI Menu does not expose a preferred opening edge. Use the system
// menu positioning API to anchor this bottom-sidebar menu above its identity row.
@MainActor
final class LibrarySwitcherButton: NSButton {
    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        target = self
        action = #selector(openLibraries)
    }
    required init?(coder: NSCoder) { fatalError("init(coder:) is unavailable") }
    override var acceptsFirstResponder: Bool { true }
    override func keyDown(with event: NSEvent) {
        if event.keyCode == 49 || event.keyCode == 36 { performClick(nil) }
        else { super.keyDown(with: event) }
    }
    @objc private func openLibraries() {
        DispatchQueue.main.async { [weak self] in self?.presentLibraries() }
    }
    private func presentLibraries() {
        guard let menu, let window else { return }
        let anchor = window.convertToScreen(convert(bounds, to: nil))
        let top = NSPoint(x: anchor.minX, y: anchor.maxY + menu.size.height)
        let point = convert(window.convertPoint(fromScreen: top), from: nil)
        menu.popUp(positioning: nil, at: point, in: self)
    }
}

struct LibrarySwitcherMenu: NSViewRepresentable {
    @ObservedObject var model: LibraryLifecycleFixture

    final class Coordinator: NSObject {
        var actions: [Int: () -> Void] = [:]
        @objc func invoke(_ sender: NSMenuItem) { actions[sender.tag]?() }
    }
    func makeCoordinator() -> Coordinator { Coordinator() }
    func makeNSView(context: Context) -> LibrarySwitcherButton {
        let button = LibrarySwitcherButton(frame: .zero)
        button.isBordered = false
        button.imagePosition = .imageLeft
        button.font = .systemFont(ofSize: NSFont.smallSystemFontSize)
        button.setAccessibilityIdentifier("lifecycle.library-switcher")
        button.setContentHuggingPriority(.required, for: .horizontal)
        return button
    }
    func updateNSView(_ button: LibrarySwitcherButton, context: Context) {
        let identity = model.signedIn ? "Suhail" : "On This Mac"
        button.title = identity
        button.image = NSImage(systemSymbolName: "person.crop.circle", accessibilityDescription: nil)
        button.toolTip = "Libraries and account"
        button.setAccessibilityLabel(identity + ", Libraries and account")
        let menu = NSMenu()
        menu.autoenablesItems = false
        context.coordinator.actions.removeAll()
        func action(_ title: String, enabled: Bool = true, checked: Bool = false, perform: @escaping () -> Void) {
            let item = NSMenuItem(title: title, action: #selector(Coordinator.invoke(_:)), keyEquivalent: "")
            item.target = context.coordinator
            item.tag = context.coordinator.actions.count
            context.coordinator.actions[item.tag] = perform
            item.isEnabled = enabled
            item.state = checked ? .on : .off
            menu.addItem(item)
        }
        menu.addItem(.sectionHeader(title: model.libraryName + " · " + model.authorityLabel))
        if model.signedIn {
            menu.addItem(.sectionHeader(title: model.accountID))
            for item in model.menuLibraries.filter({ $0.authority == .account(model.accountID) }) {
                action(model.title(for: item), enabled: model.online || item.cached, checked: item.id == model.activeID) { model.choose(item.id) }
            }
        }
        menu.addItem(.sectionHeader(title: "On This Mac"))
        for item in model.menuLibraries.filter({ $0.authority == .local }) {
            action(model.title(for: item), checked: item.id == model.activeID) { model.choose(item.id) }
        }
        menu.addItem(.separator())
        action("New Library…") { model.show(.create) }
        action("Library Settings…") { model.destination = "Library Settings" }
        menu.addItem(.separator())
        action("Account Settings…") { model.notice = "Account Settings requested (fixture only)." }
        if model.signedIn {
            action("Sign Out") { model.signOut() }
        } else {
            action("Sign in with Google", enabled: model.fixtureCase != .unsigned) { model.signIn() }
            if model.fixtureCase == .unsigned { menu.addItem(.sectionHeader(title: "Google sign-in requires signed Photara.")) }
        }
        button.menu = menu
        button.title = identity
        button.image = NSImage(systemSymbolName: "person.crop.circle", accessibilityDescription: nil)
    }
}

struct LibraryLifecycleSheet: View {
    @ObservedObject var model: LibraryLifecycleFixture
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text(heading).font(.title2).bold()
            if model.scenario == .create || model.scenario == .rename {
                TextField("Library name", text: $model.name).accessibilityIdentifier("lifecycle.name")
                if model.scenario == .create {
                    Picker("Store catalog", selection: $model.cloud) {
                        Text("Cloud").tag(true).disabled(!model.signedIn); Text("On This Mac").tag(false)
                    }
                    Text(model.cloud ? "Available on your signed-in devices. Creating requires a connection." : "Available on this Mac. This does not upload or convert another Library.")
                } else {
                    Text("Only the Library display name changes. Package names and paths stay the same.")
                }
                if !model.validName { Text(model.name.isEmpty ? "Enter a Library name." : "Choose a shorter name without line breaks or special control characters.").font(.caption).foregroundStyle(.secondary) }
                HStack {
                    Spacer(); cancel
                    Button(model.scenario == .create ? "Create Library" : "Save Name") {
                        let name = model.name.trimmingCharacters(in: .whitespacesAndNewlines)
                        model.show(.ready); model.notice = "Fixture accepted “\(name)”. No catalog was changed."
                    }.disabled(!model.validName).keyboardShortcut(.defaultAction)
                }
            } else if model.scenario == .switchLibrary {
                Text("Save or close the current project before switching Libraries. Its package remains where it is.")
                Text("Unsaved work in Coastal Studies").foregroundStyle(.secondary)
                HStack {
                    Spacer(); cancel
                    Button("Save and Switch") { model.finishSwitch() }
                }
            } else {
                removalContent
            }
        }
        .padding(24).frame(width: 480).fixedSize(horizontal: false, vertical: true)
        .interactiveDismissDisabled([.removing, .unknown, .reconciling].contains(model.scenario))
        .confirmationDialog("Permanently remove “\(model.libraryName)”?", isPresented: $model.finalConfirmation, titleVisibility: .visible) {
            Button("Remove Library", role: .destructive) { model.confirmRemoval() }
            Button("Cancel", role: .cancel) { model.cancelFinal() }
        } message: {
            Text("Remove the Library’s database catalog and references for all members. This cannot be undone. Files remain on disk.")
        }
    }
    private var heading: String {
        switch model.scenario {
        case .create: "Create Library"
        case .rename: "Rename Library"
        case .switchLibrary: "Switch Library?"
        case .unknown: "Checking removal status"
        case .reconciling: "Removed in the cloud"
        case .removing: "Removing Library…"
        default: "Remove “\(model.libraryName)”?"
        }
    }
    private var cancel: some View { Button("Cancel") { model.cancelSwitch() }.keyboardShortcut(.cancelAction) }
    @ViewBuilder private var removalContent: some View {
        if [.removal, .stale].contains(model.scenario) {
            Text("This permanently removes the database catalog and references for everyone in this Library.")
                .fixedSize(horizontal: false, vertical: true)
            Grid(alignment: .leading, horizontalSpacing: 32, verticalSpacing: 7) {
                row("Project records", "12"); row("Members / invitations", "3 / 2")
                row("People / locations / other typed records", "24 / 8 / 6")
                row("Grants / streams / media references", "15 / 13 / 48")
                row("Storage references / this Mac’s bindings", "4 / 12")
                row("Pending operations", "0 awaiting completion")
            }.font(.callout)
            Text("Other devices may have unsent work. Their catalog access ends when they reconnect.").font(.caption).foregroundStyle(.secondary)
            Text(LibraryLifecycleFixture.diskPromise).fixedSize(horizontal: false, vertical: true)
            if model.scenario == .stale {
                Label("The Library changed. Review fresh counts and type its name again.", systemImage: "arrow.clockwise")
                Button("Refresh Impact") { model.show(.removal) }
            } else {
                Text("Type \(model.libraryName) exactly to continue.")
                TextField("Library name", text: $model.confirmation).accessibilityIdentifier("lifecycle.exact-name")
                HStack { Spacer(); cancel
                    Button("Continue…") { model.requestFinal() }.disabled(!model.canReviewFinal)
                }
            }
        } else {
            Label(statusText, systemImage: statusSymbol).fixedSize(horizontal: false, vertical: true)
            Text(LibraryLifecycleFixture.diskPromise).font(.callout).foregroundStyle(.secondary)
            if [.removing, .unknown, .reconciling].contains(model.scenario) {
                ProgressView().controlSize(.small)
                Text("Operation LL-DEMO-104 · Keep this receipt identifier").font(.caption)
                if model.scenario == .unknown {
                    Button("Check Status") { model.show(.reconciling) }
                    Text("A lost reply does not mean removal failed. Checking reuses the same operation.").font(.caption)
                }
                if model.scenario == .reconciling {
                    Button("Retry Local Reconciliation") { model.show(.removed) }
                }
                Button("Close") { model.show(.ready); model.notice = "Removal status remains pending in this fixture." }
            } else { HStack { Spacer(); cancel } }
        }
    }
    private func row(_ label: String, _ count: String) -> some View { GridRow { Text(label); Text(count).monospacedDigit() } }
    private var statusSymbol: String {
        [.unknown, .removing, .reconciling].contains(model.scenario) ? "arrow.triangle.2.circlepath" : "exclamationmark.circle"
    }
    private var statusText: String {
        switch model.scenario {
        case .offline: "Connect to review current ownership and impact. Cloud removal cannot be queued offline."
        case .denied: "Only a current Library owner can remove this Library. Your access has changed."
        case .protectedLibrary: "This is a default or last Library. Keep it in this release; it can still be renamed."
        case .pending: "Resolve pending project creation or catalog operations before removing this Library."
        case .unknown: "The server may have completed removal. Check the existing operation before trying anything else."
        case .reconciling: "Cloud removal is confirmed. This Mac must finish clearing its catalog projection."
        default: "The removal request has been submitted. Closing this window does not cancel it."
        }
    }
}
