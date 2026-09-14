// One-use, signed migration receiver. The legacy recovery helper injects the
// existing runtime bundle into this process; no secret is printed or written
// to a file. This executable stores it in the modern Data Protection Keychain.
import Foundation
import Security
import LocalAuthentication
import Darwin

let service = "com.photara.operator.development.neon-main"
let account = "photara-runtime-v1"
let keys = ["PHOTARA_DB_API_URL", "PHOTARA_DB_CONTROL_URL", "PHOTARA_DB_AUTH_READ_URL", "PHOTARA_CURSOR_KEY_B64"]

func refuse(_ code: String) -> Never {
    FileHandle.standardError.write(Data("development-migration:\(code)\n".utf8))
    exit(1)
}

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

func query(accessGroup: String) -> [String: Any] {
    let context = LAContext()
    context.interactionNotAllowed = true
    return [kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecUseDataProtectionKeychain as String: true,
            kSecAttrAccessGroup as String: accessGroup,
            kSecAttrSynchronizable as String: false,
            kSecUseAuthenticationContext as String: context]
}

func validate(_ values: [String: String]) -> [String: String] {
    guard Set(values.keys) == Set(keys) else { refuse("invalid-secret-bundle") }
    var users = Set<String>()
    var hosts = Set<String>()
    for (index, key) in keys.prefix(3).enumerated() {
        guard let value = values[key], value.utf8.count <= 4_096,
              let url = URLComponents(string: value), url.scheme == "postgresql",
              let host = url.host, host.hasSuffix(".neon.tech"),
              url.path == "/neondb", url.port == nil || url.port == 5432,
              url.user == ["photara_dev_api", "photara_dev_control", "photara_dev_auth_read"][index],
              let password = url.password, !password.isEmpty,
              url.queryItems == [URLQueryItem(name: "sslmode", value: "verify-full")]
        else { refuse("invalid-runtime-url") }
        users.insert(url.user!)
        hosts.insert(host)
    }
    guard users.count == 3, hosts.count == 1,
          let key = values[keys[3]], key.count == 43,
          key.range(of: "^[A-Za-z0-9_-]{43}$", options: .regularExpression) != nil,
          let bytes = Data(base64Encoded: key.replacingOccurrences(of: "-", with: "+").replacingOccurrences(of: "_", with: "/") + "="),
          bytes.count == 32 else { refuse("invalid-cursor-key") }
    return values
}

func readBundle(accessGroup: String) -> [String: String]? {
    var request = query(accessGroup: accessGroup)
    request[kSecReturnData as String] = true
    request[kSecMatchLimit as String] = kSecMatchLimitOne
    var result: CFTypeRef?
    let status = SecItemCopyMatching(request as CFDictionary, &result)
    if status == errSecItemNotFound { return nil }
    guard status == errSecSuccess, let data = result as? Data,
          data.count <= 16_384,
          let values = try? JSONDecoder().decode([String: String].self, from: data)
    else { refuse("modern-keychain-read-blocked-\(status)") }
    return validate(values)
}

var coreLimit = rlimit(rlim_cur: 0, rlim_max: 0)
guard setrlimit(RLIMIT_CORE, &coreLimit) == 0 else { refuse("core-dump-policy-refused") }
guard CommandLine.arguments.count == 1,
      ProcessInfo.processInfo.environment["PHOTARA_RELEASE_CHANNEL"] == "development"
else { refuse("unexpected-invocation") }

let accessGroup = authorizedAccessGroup()
let environment = ProcessInfo.processInfo.environment
var incoming: [String: String] = [:]
for key in keys {
    guard let value = environment[key] else { refuse("missing-runtime-value") }
    incoming[key] = value
}
incoming = validate(incoming)

if let existing = readBundle(accessGroup: accessGroup) {
    guard existing == incoming else { refuse("modern-item-conflict-no-overwrite") }
    print("development-migration:already-stored-and-readable")
    exit(0)
}

let encoder = JSONEncoder()
encoder.outputFormatting = [.sortedKeys]
guard let data = try? encoder.encode(incoming), data.count <= 16_384 else {
    refuse("secret-encoding-failed")
}
var request = query(accessGroup: accessGroup)
request[kSecValueData as String] = data
request[kSecAttrLabel as String] = "Photara development service — Neon main"
request[kSecAttrAccessible as String] = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
let status = SecItemAdd(request as CFDictionary, nil)
guard status == errSecSuccess else {
    refuse(status == errSecDuplicateItem ? "modern-item-raced-no-overwrite" : "modern-keychain-store-blocked-\(status)")
}
guard readBundle(accessGroup: accessGroup) == incoming else {
    refuse("modern-keychain-readback-mismatch")
}
print("development-migration:modern-keychain-stored-and-readable")
