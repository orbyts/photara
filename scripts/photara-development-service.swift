// Host-only operator helper. Never embed this executable or its secrets in the app.
import Foundation
import Security
import LocalAuthentication
import Darwin

#if PHOTARA_SERVICE_FURNACE
// Only the separate test build has this namespace and cleanup command.
let service = FurnaceOperatorConfiguration.keychainService
let account = "synthetic-runtime-v1"
#else
let service = "com.photara.operator.development.neon-main"
let account = "photara-runtime-v1"
#endif
let keys = ["PHOTARA_DB_API_URL", "PHOTARA_DB_CONTROL_URL", "PHOTARA_DB_AUTH_READ_URL", "PHOTARA_CURSOR_KEY_B64"]

func refuse(_ code: String) -> Never {
    FileHandle.standardError.write(Data("development-service:\(code)\n".utf8))
    exit(1)
}

func query() -> [String: Any] {
    let context = LAContext()
    context.interactionNotAllowed = true
    return [kSecClass as String: kSecClassGenericPassword,
     kSecAttrService as String: service,
     kSecAttrAccount as String: account,
     kSecUseDataProtectionKeychain as String: true,
     kSecAttrAccessGroup as String: operatorAccessGroup,
     kSecAttrSynchronizable as String: false,
     kSecUseAuthenticationContext as String: context]
}

// The OS must authorize this entitlement through a provisioning profile. An
// ad-hoc signature, made-up Team ID or user default is never signing authority.
// The operator is a separate app-like bundle, with its own private access group.
func authorizedAccessGroup() -> String {
    guard let task = SecTaskCreateFromSelf(nil),
          let identifier = SecTaskCopyValueForEntitlement(task, "com.apple.application-identifier" as CFString, nil) as? String,
          let groups = SecTaskCopyValueForEntitlement(task, "keychain-access-groups" as CFString, nil) as? [String],
          let team = SecTaskCopyValueForEntitlement(task, "com.apple.developer.team-identifier" as CFString, nil) as? String,
          !team.isEmpty, identifier == team + ".com.photara.operator.development",
          groups == [identifier]
    else { refuse("signing-required") }
    return identifier
}

func validate(_ data: Data) -> [String: String] {
    guard data.count <= 16_384,
          let values = try? JSONDecoder().decode([String: String].self, from: data),
          Set(values.keys) == Set(keys) else { refuse("invalid-secret-bundle") }
    var users = Set<String>()
    var hosts = Set<String>()
    for (index, key) in keys.prefix(3).enumerated() {
        guard let value = values[key],
              let url = URLComponents(string: value), url.scheme == "postgresql",
              let host = url.host, host.hasSuffix(".neon.tech"),
              url.path == "/neondb", url.port == nil || url.port == 5432,
              url.user == ["photara_dev_api", "photara_dev_control", "photara_dev_auth_read"][index],
              let password = url.password, !password.isEmpty,
              url.queryItems == [URLQueryItem(name: "sslmode", value: "verify-full")]
        else { refuse("invalid-runtime-url") }
        users.insert(url.user!); hosts.insert(host)
    }
    guard users.count == 3, hosts.count == 1,
          let key = values[keys[3]], key.count == 43,
          key.range(of: "^[A-Za-z0-9_-]{43}$", options: .regularExpression) != nil,
          let bytes = Data(base64Encoded: key.replacingOccurrences(of: "-", with: "+").replacingOccurrences(of: "_", with: "/") + "="),
          bytes.count == 32 else { refuse("invalid-cursor-key") }
    return values
}

func readBundle() -> [String: String] {
    var request = query()
    request[kSecReturnData as String] = true
    request[kSecMatchLimit as String] = kSecMatchLimitOne
    var result: CFTypeRef?
    let status = SecItemCopyMatching(request as CFDictionary, &result)
    guard status == errSecSuccess, let data = result as? Data else {
        refuse(status == errSecItemNotFound ? "keychain-item-missing" : "keychain-access-blocked-\(status)")
    }
    return validate(data)
}

// Never query the old file-based item. It stays untouched for a separately
// reviewed migration after signing is established. Modern calls fail closed.
let operatorAccessGroup = authorizedAccessGroup()
var coreLimit = rlimit(rlim_cur: 0, rlim_max: 0)
guard setrlimit(RLIMIT_CORE, &coreLimit) == 0 else { refuse("core-dump-policy-refused") }
let arguments = CommandLine.arguments
guard arguments.count >= 2 else { refuse("expected-store-check-or-run") }
switch arguments[1] {
case "store":
    guard arguments.count == 2, isatty(STDIN_FILENO) == 0 else { refuse("secret-input-must-be-private-pipe") }
    let data = FileHandle.standardInput.readData(ofLength: 16_385)
    _ = validate(data)
    var request = query()
    request[kSecValueData as String] = data
    request[kSecAttrLabel as String] = "Photara development service — Neon main"
    request[kSecAttrAccessible as String] = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
    let status = SecItemAdd(request as CFDictionary, nil)
    guard status == errSecSuccess else {
        refuse(status == errSecDuplicateItem ? "keychain-item-already-exists-no-overwrite" : "keychain-store-blocked-\(status)")
    }
    _ = readBundle()
    print("development-service:keychain-stored-and-readable")
case "check":
    guard arguments.count == 2 else { refuse("unexpected-argument") }
    _ = readBundle()
    print("development-service:keychain-ready")
case "run":
    guard arguments.count == 3 else { refuse("expected-absolute-service-binary") }
    let path = arguments[2]
    // Never deliver the operator bundle to an arbitrary caller-supplied binary.
    let embedded = Bundle.main.bundleURL.appending(path: "Contents/Resources/photara-service")
        .resolvingSymlinksInPath().standardizedFileURL
    guard URL(fileURLWithPath: path).resolvingSymlinksInPath().standardizedFileURL == embedded
    else { refuse("service-path-not-embedded") }
    var info = stat()
    guard path.hasPrefix("/"), URL(fileURLWithPath: path).lastPathComponent == "photara-service",
          lstat(path, &info) == 0, (info.st_mode & S_IFMT) == S_IFREG,
          info.st_uid == getuid(), (info.st_mode & 0o022) == 0,
          access(path, X_OK) == 0 else { refuse("unsafe-service-executable") }
    guard ProcessInfo.processInfo.environment["PHOTARA_DB_MIGRATION_URL"] == nil else {
        refuse("migration-owner-setting-forbidden")
    }
    var environment = readBundle()
    environment["PHOTARA_RELEASE_CHANNEL"] = "development"
    // Keep the native process environment minimal. No owner URL, tracing, proxy,
    // loader injection or user-selected profile reaches the runtime child.
    environment["PATH"] = "/usr/bin:/bin"
    #if PHOTARA_SERVICE_FURNACE
    guard let config = ProcessInfo.processInfo.environment["PHOTARA_FURNACE_CONFIG"], config.hasPrefix("/private/tmp/")
    else { refuse("furnace-config-required") }
    environment["PHOTARA_FURNACE_CONFIG"] = config
    #endif
    let argv = [strdup(path), nil]
    let envp = environment.sorted { $0.key < $1.key }.map { strdup("\($0.key)=\($0.value)") } + [nil]
    execve(path, argv, envp)
    refuse("service-exec-failed")
#if PHOTARA_SERVICE_FURNACE
case "erase-fixture":
    guard arguments.count == 2 else { refuse("unexpected-argument") }
    let status = SecItemDelete(query() as CFDictionary)
    guard status == errSecSuccess || status == errSecItemNotFound else { refuse("furnace-cleanup-failed") }
    print("development-service:furnace-item-removed")
#endif
default:
    refuse("unknown-command")
}
