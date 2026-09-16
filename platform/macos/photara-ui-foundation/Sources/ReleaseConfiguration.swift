import Foundation

/// Public identity only. Database credentials never enter native configuration.
struct ProductIdentity: Decodable, Sendable {
    var legacyPreferenceSuites: [String] = []
    var developmentOperatorDirectory: String = ""
    var developmentOperatorProduct: String = ""
    var productName: String = ""
    var executableName: String = ""
    var projectDocumentUTI: String = ""
    var legacyProjectPackageExtensions: [String] = []
    var defaultProjectsDirectory: String = ""
    var journalDirectory: String = "State"
    let displayName: String
    let shortName: String
    let isCodename: Bool
    let bundleIdentifier: String
    let keychainService: String
    let callbackScheme: String
    let projectPackageDisplayType: String
    let projectPackageExtension: String
    let apiAudienceNamespace: String
    let serviceHostname: String?
    let userAgent: String
    let applicationSupportDirectory: String
    let cacheDirectory: String
    let defaultLibraryName: String
    let websiteURL: URL?
    let supportURL: URL?
    let privacyURL: URL?
    let storeURL: URL?
}

enum ReleaseChannel: String, Decodable, Sendable {
    case development, remoteAcceptance, production
}

struct ReleaseEnvironment: Decodable, Sendable {
    let channel: ReleaseChannel
    let environmentId: String
    let apiOrigin: URL
    let auth0Issuer: URL
    let auth0Audience: String
    let nativeClientId: String
    let callbackURL: URL
    let logoutURL: URL
    let schemaFamily: String
    let schemaEpoch: UInt32
    let minimumAPI: UInt32
    let loggingPolicy: String
    let telemetryEnabled: Bool
}

/// `current` is generated from the checked descriptor at build time. No runtime
/// environment variable or preference can switch a distributed app to loopback.
struct ReleaseConfiguration: Decodable, Sendable {
    let identity: ProductIdentity
    let environment: ReleaseEnvironment
}

// Public filename/root policy only. Serialized format IDs are deliberately separate.
extension ProductIdentity {
    func acceptsProjectPackage(_ url: URL) -> Bool {
        let suffix = url.pathExtension.lowercased()
        return suffix == projectPackageExtension || legacyProjectPackageExtensions.contains(suffix)
    }

    func defaultProjectsURL(home: URL = FileManager.default.homeDirectoryForCurrentUser) -> URL {
        home.appending(path: "Pictures").appending(path: defaultProjectsDirectory).appending(path: "Projects")
    }

    func supportURL(base: URL) -> URL { base.appending(path: applicationSupportDirectory) }
    func cacheURL(base: URL) -> URL { base.appending(path: cacheDirectory) }
    func journalURL(support: URL) -> URL { support.appending(path: journalDirectory) }
}
