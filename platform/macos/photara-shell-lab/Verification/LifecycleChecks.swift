import AppKit
import SwiftUI

@main
struct LifecycleVerificationApp: App {
    @NSApplicationDelegateAdaptor(LifecycleDelegate.self) private var delegate
    var body: some Scene { Settings { EmptyView() } }
}
@MainActor
final class LifecycleDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        Task { @MainActor in
            do {
                let directory = URL(fileURLWithPath: CommandLine.arguments[1])
                try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
                let model = LibraryLifecycleFixture()
                if CommandLine.arguments.contains("--keyboard-only") {
                    try await capture(LibraryLifecycleLabView(model: model), name: "keyboard-switch", size: .init(width: 900, height: 600), directory: directory,
                        inspect: {
                            try await probe("keyboard")
                            require(model.activeID == "cloud-4D90", "Native menu keyboard selection failed: " + model.activeID)
                        })
                    print("PASS: native keyboard selection changed active Library")
                    exit(0)
                }
                verifySwitcher(model)
                model.configure(.cloud)
                model.show(.removal)
                for invalid in ["", "fieldwork", "Fieldwork ", " Fieldwork", "FieldworK"] {
                    model.confirmation = invalid
                    require(!model.canReviewFinal, "Nonexact confirmation accepted")
                    model.requestFinal()
                    require(!model.finalConfirmation, "Invalid text opened final dialog")
                }
                model.confirmation = "Fieldwork"
                model.confirmRemoval()
                require(model.scenario == .removal, "Retyping bypassed final confirmation")
                model.requestFinal()
                require(model.finalConfirmation, "Exact name cannot reach final dialog")
                model.cancelFinal()
                model.confirmRemoval()
                require(model.scenario == .removal, "Cancelled final dialog retained consent")
                model.requestFinal()
                model.confirmRemoval()
                require(model.scenario == .removing, "Explicit final action failed")
                for state in LibraryLifecycleScenario.allCases where state != .removal {
                    model.show(state); model.confirmation = "Fieldwork"
                    require(!model.canReviewFinal, "Blocked state enables removal")
                }
                model.show(.create); model.name = "\n"
                require(!model.validName, "Empty name accepted")
                model.name = "Fieldwork"; require(model.validName, "Valid name rejected")
                if !CommandLine.arguments.contains("--switcher-only") {
                for dark in [false, true] {
                    let mode = dark ? "dark" : "light"
                    model.show(.ready)
                    try await capture(LibraryLifecycleLabView(model: model).preferredColorScheme(dark ? .dark : .light),
                        name: "libraries-\(mode)", size: .init(width: 900, height: 600), directory: directory)
                    for state in LibraryLifecycleScenario.allCases where state != .ready && state != .removed {
                        model.show(state)
                        try await capture(LibraryLifecycleSheet(model: model).preferredColorScheme(dark ? .dark : .light),
                            name: "\(state.rawValue)-\(mode)", size: .init(width: 528, height: state == .removal || state == .stale ? 620 : 410), directory: directory)
                    }
                    model.show(.removed)
                    try await capture(LibraryLifecycleLabView(model: model).preferredColorScheme(dark ? .dark : .light),
                        name: "receipt-\(mode)", size: .init(width: 900, height: 600), directory: directory)
                    model.show(.removal)
                    try await capture(LibraryLifecycleLabView(model: model).preferredColorScheme(dark ? .dark : .light),
                        name: "attached-removal-\(mode)", size: .init(width: 820, height: 720), directory: directory)
                    model.show(.removal); model.confirmation = "Fieldwork"
                    try await capture(LibraryLifecycleSheet(model: model).preferredColorScheme(dark ? .dark : .light)
                        .task { try? await Task.sleep(for: .milliseconds(100)); model.requestFinal() },
                        name: "final-dialog-\(mode)", size: .init(width: 528, height: 620), directory: directory)
                }
                }
                for context in LibrarySwitcherFixtureCase.allCases {
                    for dark in [false, true] {
                        model.configure(context)
                        FileHandle.standardOutput.write(Data("CASE \(context.rawValue) dark=\(dark)\n".utf8))
                        let mode = dark ? "dark" : "light"
                        try await capture(LibraryLifecycleLabView(model: model).preferredColorScheme(dark ? .dark : .light),
                            name: "switcher-\(context.rawValue)-\(mode)", size: .init(width: 760, height: 560), directory: directory,
                            inspect: {
                                try await probe("inspect")
                                try await probe("menu", path: directory.appending(path: "menu-\(context.rawValue)-\(mode).png").path, expected: model.title(for: model.active!))
                            })
                    }
                }
                model.configure(.cloud)
                try await capture(LibraryLifecycleLabView(model: model), name: "keyboard-switch", size: .init(width: 900, height: 600), directory: directory,
                    inspect: {
                        try await probe("keyboard")
                        require(model.activeID == "cloud-4D90", "Native menu keyboard selection failed")
                    })
                for mode in ["click", "signin", "settings"] {
                    model.configure(mode == "signin" ? .signedOut : .cloud)
                    try await capture(LibraryLifecycleLabView(model: model), name: "native-\(mode)", size: .init(width: 900, height: 600), directory: directory,
                        inspect: {
                            try await probe(mode, expected: model.title(for: model.active!))
                            if mode == "signin" { require(model.notice.contains("Sign In requested"), "Native Google action did not reach supported-flow fixture") }
                            if mode == "settings" { require(model.destination == "Library Settings", "Native settings action failed") }
                        })
                }
                for context in [LibrarySwitcherFixtureCase.unsaved, .inaccessible, .removedTarget] {
                    for dark in [false, true] {
                        model.configure(context); model.choose("cloud-4D90")
                        try await capture(LibraryLifecycleLabView(model: model).preferredColorScheme(dark ? .dark : .light),
                            name: "recovery-\(context.rawValue)-\(dark ? "dark" : "light")", size: .init(width: 760, height: 560), directory: directory)
                    }
                }
                print("PASS: exact-name and final-gate fixture checks; blocked states; name validation; lifecycle and identity-menu native captures. No persistence behavior tested.")
                exit(0)
            } catch {
                FileHandle.standardError.write(Data("\(error)\n".utf8)); exit(1)
            }
        }
    }
}

@MainActor
func probe(_ mode: String, path: String? = nil, expected: String? = nil) async throws {
    let process = Process()
    process.executableURL = Bundle.main.executableURL!.deletingLastPathComponent().appending(path: "SwitcherAccessibilityProbe")
    process.arguments = [String(ProcessInfo.processInfo.processIdentifier), mode] + [path ?? "", expected ?? ""]
    try process.run()
    while process.isRunning { try await Task.sleep(for: .milliseconds(20)) }
    require(process.terminationStatus == 0, "External switcher probe failed")
}

@MainActor
func verifySwitcher(_ model: LibraryLifecycleFixture) {
    for from in LibrarySwitcherFixtureCase.allCases {
        for to in LibrarySwitcherFixtureCase.allCases {
            model.configure(from); model.choose("cloud-4D90"); model.configure(to)
            require(model.pendingTarget == nil && model.scenario == .ready, "Switcher fixture transition leaked pending state")
        }
    }
    model.configure(.cloud)
    require(model.menuLibraries.count == 3, "Cached cloud projection duplicated")
    model.choose("cloud-4D90")
    require(model.activeID == "cloud-4D90" && model.scenario == .ready, "Ordinary switch should have no Save gate")
    model.choose("local-1001"); require(model.authorityLabel == "On This Mac", "Local authority mislabeled")
    model.configure(.duplicateNames)
    require(model.title(for: model.menuLibraries[0]) != model.title(for: model.menuLibraries[1]), "Duplicate names not disambiguated")
    model.configure(.offline); model.choose("cloud-4D90")
    require(model.activeID == "cloud-7C21", "Uncached offline target selected")
    model.configure(.signedOut)
    require(model.menuLibraries.allSatisfy { $0.authority == .local }, "Signed-out cloud Library exposed")
    model.signIn(); require(model.notice.contains("Sign In requested"), "Supported sign-in action missing")
    model.configure(.unsaved); model.choose("cloud-4D90")
    require(model.activeID == "cloud-7C21" && model.scenario == .switchLibrary && model.destination == "Projects", "Unsaved project bypassed")
    model.cancelSwitch(); require(model.activeID == "cloud-7C21" && model.pendingTarget == nil && model.destination == "Projects", "Cancel changed selection")
    model.choose("cloud-4D90"); model.finishSwitch(); require(model.activeID == "cloud-4D90", "Resolved switch failed")
    for fixture in [LibrarySwitcherFixtureCase.inaccessible, .removedTarget] {
        model.configure(fixture); model.choose("cloud-4D90")
        require(model.activeID == "cloud-7C21" && !model.notice.isEmpty, "Stale target replaced active Library")
    }
    print("PASS: 81 switcher fixture transitions; cloud/local/duplicates/offline/signed-out/unsaved/stale target semantics")
}
