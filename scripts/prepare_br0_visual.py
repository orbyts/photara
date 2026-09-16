#!/usr/bin/env python3
"""Prepare/build (never launch) a source-only, fail-closed BR0 GUI fixture.

Production has no runtime path/identity override. All injections below affect only
an isolated source snapshot. GUI actions and quit are performed separately.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import uuid

from verify_brand_readiness import ROOT, synthetic_descriptor


def replace_body(source, marker, body):
    start = source.index(marker)
    opening = source.index("{", start)
    depth = 1
    end = opening + 1
    while depth:
        depth += (source[end] == "{") - (source[end] == "}")
        end += 1
    return source[:opening + 1] + "\n" + body + "\n" + source[end - 1:]


def prepare(output):
    assert str(output).startswith("/private/tmp/") and not output.exists()
    output.mkdir(parents=True)
    snapshot = output / "source"
    snapshot.mkdir()
    for name in subprocess.check_output(["git", "ls-files", "--cached", "--others", "--exclude-standard"], cwd=ROOT, text=True).splitlines():
        src = ROOT / name
        if src.is_file():
            dst = snapshot / name
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, dst)
    nonce = uuid.uuid4().hex
    bundle = "org.example.juniper.br0.visual." + nonce
    descriptor = synthetic_descriptor()
    identity = descriptor["identity"]
    identity.update(bundleIdentifier=bundle, callbackScheme=bundle,
                    projectDocumentUTI=bundle + ".project", keychainService=bundle + ".disabled",
                    defaultLibraryName="Juniper Visual Library", legacyPreferenceSuites=[])
    environment = descriptor["environments"]["development"]
    environment["callbackURL"] = environment["logoutURL"] = f"{bundle}://identity.example.org/macos/{bundle}/callback"
    (snapshot / "config/product-identity.json").write_text(json.dumps(descriptor, indent=2) + "\n")
    data = output / "disposable"
    data.mkdir()
    (data / "isolation-marker").write_text(nonce)
    support = data / "ApplicationSupport"
    support.mkdir()
    # A disposable sentinel proves the sandbox blocks reads outside the data root.
    denied = output / "read-denied-canary"
    denied.write_text("synthetic canary only")
    profile = f'''(version 1)
(allow default)
(deny network-inbound network-outbound (local ip) (remote ip))
(deny file-read* file-write* (subpath "/Users") (subpath "/Volumes"))
(deny file-read* (literal {json.dumps(str(denied))}))
(deny file-write* (require-all
    (require-not (subpath {json.dumps(str(data))}))
    (require-not (subpath {json.dumps(str(data).replace("/private/tmp/", "/tmp/", 1))}))
    (require-not (subpath {json.dumps(os.environ['TMPDIR'].rstrip('/'))}))
    (require-not (subpath {json.dumps(os.path.realpath(os.environ["TMPDIR"]))}))
    (require-not (literal "/dev/null"))))
(deny mach-lookup (global-name "com.apple.securityd") (global-name "com.apple.secd"))
'''
    (output / "isolation.sb").write_text(profile)
    sources = snapshot / "platform/macos/photara-app/Sources"
    driver = sources / "ProductionOpeningCloudDriver.swift"
    text = driver.read_text()
    start = text.index("@MainActor\nfinal class ProductionOpeningCloudDriver")
    # This final class is the last declaration in the current source; retain the
    # real local journal wrappers above it, replace only cloud/service behavior.
    text = text[:start] + '''@MainActor
final class ProductionOpeningCloudDriver: OpeningCloudDriver {
    init(configuration: ReleaseConfiguration = .current, dependencies: NativeOpeningDependencies? = nil, supportDirectoryOverride: URL? = nil) {}
    var availability: OpeningCloudAvailability { .integrationRequired }
    func savedState() async throws -> NativeOnboardingState? { .localReady }
    func signIn(progress: @escaping @MainActor (NativeOnboardingState) -> Void) async throws -> NativeOnboardingState { throw NativeAuthenticationError.signingRequired }
    func projectCreation(_ creation: BridgeProjectCreation, journal: NativeProjectCreationJournal) async throws { throw NativeAuthenticationError.signingRequired }
}
'''
    driver.write_text(text)
    # Disable shared developer overrides, not just HOME-based lookup.
    for name, marker in [
        ("photara-theme/Sources/PhotaraTheme.swift", "static var overrideURL:"),
        ("photara-shell/Sources/ApplicationShellPreset.swift", "static var overridePreset:"),
        ("photara-gallery/Sources/GalleryPreset.swift", "static var overridePreset:"),
        ("photara-inspector/Sources/InspectorPreset.swift", "static var overridePreset:"),
    ]:
        p = snapshot / "platform/macos" / name
        p.write_text(replace_body(p.read_text(), marker, "        nil"))
    app = sources / "PhotaraMacApp.swift"
    text = app.read_text().replace("AppModel()", "BR0VisualIsolation.makeApp()")
    text = text.replace("EditorSessionModel()", "EditorSessionModel(defaults: BR0VisualIsolation.defaults)")
    text = text.replace("PhotaraThemeStore()", "PhotaraThemeStore(usesDevelopmentOverride: false)")
    text = text.replace(".environmentObject(theme)", ".environmentObject(theme)\n                .defaultAppStorage(BR0VisualIsolation.defaults)")
    app.write_text(text)
    helper = '''import AppKit
import Foundation
import Darwin

// All app-model/session/AppStorage reads and writes use this disposable plist.
// No superclass preference lookup is delegated to a real app domain.
final class BR0DisposableDefaults: UserDefaults, @unchecked Sendable {
    private let file: URL
    private let mutex = NSRecursiveLock()
    private var values: [String: Any]
    init(file: URL, suite: String) {
        self.file = file
        values = (try? Data(contentsOf: file)).flatMap {
            (try? PropertyListSerialization.propertyList(from: $0, format: nil)) as? [String: Any]
        } ?? [:]
        super.init(suiteName: suite + ".preferences")!
    }
    override func object(forKey key: String) -> Any? { mutex.lock(); defer { mutex.unlock() }; return values[key] }
    override func set(_ value: Any?, forKey key: String) {
        mutex.lock(); defer { mutex.unlock() }
        values[key] = value
        try! PropertyListSerialization.data(fromPropertyList: values, format: .binary, options: 0).write(to: file, options: .atomic)
        NotificationCenter.default.post(name: UserDefaults.didChangeNotification, object: self)
    }
    override func set(_ value: Int, forKey key: String) { set(NSNumber(value: value), forKey: key) }
    override func set(_ value: Bool, forKey key: String) { set(NSNumber(value: value), forKey: key) }
    override func set(_ value: Double, forKey key: String) { set(NSNumber(value: value), forKey: key) }
    override func set(_ value: Float, forKey key: String) { set(NSNumber(value: value), forKey: key) }
    override func removeObject(forKey key: String) { set(nil, forKey: key) }
    override func string(forKey key: String) -> String? { object(forKey: key) as? String }
    override func data(forKey key: String) -> Data? { object(forKey: key) as? Data }
    override func array(forKey key: String) -> [Any]? { object(forKey: key) as? [Any] }
    override func stringArray(forKey key: String) -> [String]? { object(forKey: key) as? [String] }
    override func dictionary(forKey key: String) -> [String: Any]? { object(forKey: key) as? [String: Any] }
    override func integer(forKey key: String) -> Int { (object(forKey: key) as? NSNumber)?.intValue ?? 0 }
    override func bool(forKey key: String) -> Bool { (object(forKey: key) as? NSNumber)?.boolValue ?? false }
    override func double(forKey key: String) -> Double { (object(forKey: key) as? NSNumber)?.doubleValue ?? 0 }
    override func float(forKey key: String) -> Float { (object(forKey: key) as? NSNumber)?.floatValue ?? 0 }
    override func register(defaults registration: [String: Any]) {
        for (key, value) in registration where object(forKey: key) == nil { set(value, forKey: key) }
    }
    override func dictionaryRepresentation() -> [String: Any] { mutex.lock(); defer { mutex.unlock() }; return values }
    override func synchronize() -> Bool { true }
}

@MainActor
enum BR0VisualIsolation {
    static let root = URL(fileURLWithPath: ROOT_PATH)
    static let support = root.appending(path: "ApplicationSupport")
    static let defaults = BR0DisposableDefaults(file: root.appending(path: "Preferences.plist"), suite: BUNDLE_ID)
    static func makeApp() -> AppModel {
        precondition(Bundle.main.bundleIdentifier == BUNDLE_ID)
        precondition(ReleaseConfiguration.current.identity.legacyPreferenceSuites.isEmpty)
        precondition((try? String(contentsOf: root.appending(path: "isolation-marker"), encoding: .utf8)) == NONCE)
        // SDK 27 marks direct sandbox declarations unavailable. This isolated
        // test host resolves the system runtime entry point, fails closed if it
        // is absent, and never ships this compatibility seam in production.
        typealias ApplySandbox = @convention(c) (UnsafePointer<CChar>, UInt64, UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>) -> Int32
        guard let library = dlopen("/usr/lib/libsandbox.1.dylib", RTLD_NOW),
              let symbol = dlsym(library, "sandbox_init") else { fatalError("No runtime sandbox available") }
        let apply = unsafeBitCast(symbol, to: ApplySandbox.self)
        var error: UnsafeMutablePointer<CChar>?
        precondition(PROFILE.withCString { apply($0, 0, &error) } == 0)
        precondition((try? Data(contentsOf: URL(fileURLWithPath: CANARY))) == nil)
        let proof: [String: String] = ["bundle": BUNDLE_ID, "executable": Bundle.main.executableURL!.path,
            "support": support.path, "cache": ReleaseConfiguration.current.identity.cacheURL(base: support).path,
            "journal": ReleaseConfiguration.current.identity.journalURL(support: support).path,
            "database": ReleaseConfiguration.current.identity.journalURL(support: support).appending(path: "photara-local-v2.sqlite").path,
            "defaultsSuite": BUNDLE_ID + ".preferences", "network": "sandbox-denied", "liveHomeAndVolumes": "sandbox-denied",
            "keychain": "securityd/secd denied; cloud driver absent", "outsideReadCanary": "denied"]
        try! JSONSerialization.data(withJSONObject: proof, options: [.prettyPrinted, .sortedKeys])
            .write(to: root.appending(path: "startup-proof.json"), options: .atomic)
        return AppModel(defaults: defaults, supportRootOverride: support)
    }
}
'''
    for token, value in [("ROOT_PATH", str(data)), ("BUNDLE_ID", bundle), ("NONCE", nonce), ("PROFILE", profile), ("CANARY", str(denied))]:
        helper = helper.replace(token, json.dumps(value))
    (sources / "AppModelBR0Isolation.swift").write_text(helper)
    # The injected helper/profile is the only gate through which app stores open.
    assert "BR0VisualIsolation.makeApp()" in app.read_text()
    assert "NativeCredentialStore(" not in driver.read_text()
    assert "NativeDevelopmentService.installed" not in driver.read_text()
    evidence = {"bundle": bundle, "app": str(output / "build/Juniper Studio.app"),
                "root": str(data), "profile": str(output / "isolation.sb"),
                "defaultsSuite": bundle + ".preferences", "syntheticSourceOnly": True}
    (output / "visual-fixture.json").write_text(json.dumps(evidence, indent=2) + "\n")
    return snapshot



if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--build", action="store_true")
    args = parser.parse_args()
    snapshot = prepare(args.output.resolve())
    if args.build:
        env = dict(os.environ, PHOTARA_APP_BUILD_ROOT=str(args.output / "build"),
                   PHOTARA_APP_RUST_TARGET=str(args.output / "target"),
                   CARGO_NET_OFFLINE="true", PHOTARA_RELEASE_CHANNEL="development")
        env.pop("PHOTARA_MACOS_PROVISIONING_PROFILE", None)
        subprocess.run([str(snapshot / "platform/macos/photara-app/build-app.sh")], cwd=snapshot, env=env, check=True)
    print(args.output / "visual-fixture.json")
