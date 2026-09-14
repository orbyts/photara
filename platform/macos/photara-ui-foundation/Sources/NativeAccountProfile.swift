import Foundation
import ImageIO
import UniformTypeIdentifiers

/// Presentation only. Account/library authority always comes from the verified
/// issuer/subject and the Rust journal, never these optional profile claims.
struct NativeAccountProfile: Codable, Equatable, Sendable {
    let name: String?
    let email: String?
    let picture: String?
    var avatar: Data? = nil

    static func fromVerifiedIDToken(_ token: String) -> Self? {
        let parts = token.split(separator: ".")
        guard parts.count == 3,
              let data = try? AuthenticationEncoding.decodeBase64URL(String(parts[1])),
              let claims = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return nil }
        func text(_ key: String, maximum: Int) -> String? {
            guard let value = claims[key] as? String, !value.isEmpty, value.utf8.count <= maximum,
                  value.unicodeScalars.allSatisfy({ !CharacterSet.controlCharacters.contains($0) }) else { return nil }
            return value
        }
        let result = Self(name: text("name", maximum: 256), email: text("email", maximum: 320),
                          picture: text("picture", maximum: 2048))
        return result.name == nil && result.email == nil && result.picture == nil ? nil : result
    }
}

enum NativeAccountProfileCache {
    static func url(support: URL, environment: String, library: String) -> URL {
        let key = AuthenticationEncoding.sha256(Data((environment + "\n" + library).utf8))
        return support.appending(path: "State/Profiles/" + key + ".json")
    }

    static func read(_ url: URL) -> NativeAccountProfile? {
        guard let size = try? url.resourceValues(forKeys: [.fileSizeKey]).fileSize, size <= 400_000,
              let data = try? Data(contentsOf: url), data.count <= 400_000 else { return nil }
        return try? JSONDecoder().decode(NativeAccountProfile.self, from: data)
    }

    static func write(_ profile: NativeAccountProfile, to url: URL) throws {
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true,
                                               attributes: [.posixPermissions: 0o700])
        try JSONEncoder().encode(profile).write(to: url, options: [.atomic])
        try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: url.path)
    }

    /// Optional Google image fetch, after deliberate sign-in only. No cookies,
    /// credentials, redirects, or unbounded downloads enter this presentation path.
    static func avatar(_ picture: String?) async -> Data? {
        guard let picture, let url = URL(string: picture), url.scheme == "https",
              let host = url.host?.lowercased(), host.hasSuffix(".googleusercontent.com"),
              url.user == nil, url.password == nil, url.port == nil || url.port == 443 else { return nil }
        let config = URLSessionConfiguration.ephemeral
        config.httpCookieStorage = nil
        config.httpShouldSetCookies = false
        config.urlCredentialStorage = nil
        config.timeoutIntervalForRequest = 8
        config.timeoutIntervalForResource = 10
        let session = URLSession(configuration: config, delegate: AvatarRedirectBlocker(), delegateQueue: nil)
        defer { session.invalidateAndCancel() }
        do {
            let (bytes, response) = try await session.bytes(from: url)
            guard let response = response as? HTTPURLResponse, response.statusCode == 200,
                  response.expectedContentLength <= 262_144 else { return nil }
            var data = Data()
            for try await byte in bytes {
                try Task.checkCancellation()
                guard data.count < 262_144 else { return nil }
                data.append(byte)
            }
            guard let source = CGImageSourceCreateWithData(data as CFData, nil),
                  let thumbnail = CGImageSourceCreateThumbnailAtIndex(source, 0, [
                    kCGImageSourceCreateThumbnailFromImageAlways: true,
                    kCGImageSourceThumbnailMaxPixelSize: 96,
                    kCGImageSourceCreateThumbnailWithTransform: true
                  ] as CFDictionary) else { return nil }
            let output = NSMutableData()
            guard let destination = CGImageDestinationCreateWithData(output, UTType.png.identifier as CFString, 1, nil)
            else { return nil }
            CGImageDestinationAddImage(destination, thumbnail, nil)
            guard CGImageDestinationFinalize(destination) else { return nil }
            return output as Data
        } catch { return nil }
    }
}

private final class AvatarRedirectBlocker: NSObject, URLSessionTaskDelegate, Sendable {
    func urlSession(_ session: URLSession, task: URLSessionTask,
                    willPerformHTTPRedirection response: HTTPURLResponse, newRequest request: URLRequest,
                    completionHandler: @escaping @Sendable (URLRequest?) -> Void) { completionHandler(nil) }
}
