import Foundation
import Security
import CryptoKit

/// Structural JSON validation rejects duplicate names before Codable sees them.
/// Decoding alone loses duplicate fields and is insufficient for signed claims.
enum AuthenticationJSON {
    static func validate(_ data: Data, maximum: Int = 16_384) throws {
        guard !data.isEmpty, data.count <= maximum else { throw NativeAuthenticationError.invalidResponse }
        var parser = Parser(bytes: Array(data))
        try parser.value(depth: 0)
        parser.space()
        guard parser.index == parser.bytes.count else { throw NativeAuthenticationError.invalidResponse }
    }

    /// Validates the strict object envelope used by Photara's canonical wire
    /// contracts. Forward slashes remain unescaped, as required by the frozen
    /// canonical codec and emitted by the Rust service.
    static func validateCanonicalObject(
        _ data: Data,
        keys: Set<String>,
        maximum: Int = 16_384
    ) throws {
        try validate(data, maximum: maximum)
        guard let object = try JSONSerialization.jsonObject(with: data) as? [String: Any],
              Set(object.keys) == keys,
              let canonical = try? JSONSerialization.data(
                  withJSONObject: object,
                  options: [.sortedKeys, .withoutEscapingSlashes]
              ),
              canonical == data
        else { throw NativeAuthenticationError.invalidResponse }
    }

    private struct Parser {
        let bytes: [UInt8]
        var index = 0
        mutating func space() { while index < bytes.count && [9, 10, 13, 32].contains(bytes[index]) { index += 1 } }
        mutating func take(_ byte: UInt8) -> Bool {
            space()
            if index < bytes.count && bytes[index] == byte { index += 1; return true }
            return false
        }
        mutating func string() throws -> String {
            space()
            let start = index
            guard take(34) else { throw NativeAuthenticationError.invalidResponse }
            var escaped = false
            while index < bytes.count {
                let byte = bytes[index]
                index += 1
                if byte == 34 && !escaped {
                    return try JSONDecoder().decode(String.self, from: Data(bytes[start..<index]))
                }
                if byte == 92 && !escaped { escaped = true } else { escaped = false }
            }
            throw NativeAuthenticationError.invalidResponse
        }
        mutating func value(depth: Int) throws {
            guard depth <= 16 else { throw NativeAuthenticationError.invalidResponse }
            space()
            guard index < bytes.count else { throw NativeAuthenticationError.invalidResponse }
            if bytes[index] == 34 { _ = try string(); return }
            if take(123) {
                var keys = Set<String>()
                if take(125) { return }
                repeat {
                    let key = try string()
                    guard keys.insert(key).inserted, take(58) else { throw NativeAuthenticationError.invalidResponse }
                    try value(depth: depth + 1)
                    if take(125) { return }
                    guard take(44) else { throw NativeAuthenticationError.invalidResponse }
                } while true
            }
            if take(91) {
                if take(93) { return }
                repeat {
                    try value(depth: depth + 1)
                    if take(93) { return }
                    guard take(44) else { throw NativeAuthenticationError.invalidResponse }
                } while true
            }
            let start = index
            while index < bytes.count && ![9, 10, 13, 32, 44, 93, 125].contains(bytes[index]) { index += 1 }
            guard index > start else { throw NativeAuthenticationError.invalidResponse }
            _ = try JSONSerialization.jsonObject(with: Data(bytes[start..<index]), options: .fragmentsAllowed)
        }
    }
}

/// Output includes only the verified identity. The ID token stays transient and
/// is never usable as an API access token or persisted in the credential store.
struct NativeVerifiedIdentity: Equatable {
    let issuer: String
    let subject: String
}

struct NativeIDTokenVerifier {
    let issuer: String
    let clientID: String

    private struct Header: Decodable { let alg: String; let kid: String; let typ: String? }
    private struct Audience: Decodable {
        let values: [String]
        init(from decoder: Decoder) throws {
            let container = try decoder.singleValueContainer()
            if let single = try? container.decode(String.self) { values = [single] }
            else { values = try container.decode([String].self) }
        }
    }
    private struct Claims: Decodable {
        let iss: String
        let sub: String
        let aud: Audience
        let azp: String?
        let exp: Int64
        let iat: Int64
        let nbf: Int64?
        let nonce: String
        let auth_time: Int64?
        let at_hash: String?
    }

    /// key and expectedKeyID must come from fresh, pinned issuer JWKS. This
    /// function never chooses trust from token-supplied URLs or embedded keys.
    func verify(_ token: String, accessToken: String, attempt: NativePKCEAttempt,
                key: SecKey, expectedKeyID: String, now: Date,
                enrollmentChallengeCreatedAt: Date?) throws -> NativeVerifiedIdentity {
        do {
            guard token.utf8.count <= 16_384 else { throw NativeAuthenticationError.invalidResponse }
            let segments = token.split(separator: ".", omittingEmptySubsequences: false)
            guard segments.count == 3 else { throw NativeAuthenticationError.invalidResponse }
            func decode(_ input: Substring) throws -> Data {
                guard !input.isEmpty, input.utf8.allSatisfy({ (48...57).contains($0) || (65...90).contains($0) || (97...122).contains($0) || $0 == 45 || $0 == 95 })
                else { throw NativeAuthenticationError.invalidResponse }
                let text = input.replacingOccurrences(of: "-", with: "+").replacingOccurrences(of: "_", with: "/")
                guard let data = Data(base64Encoded: text + String(repeating: "=", count: (4 - text.count % 4) % 4)),
                      AuthenticationEncoding.base64URL(data) == input
                else { throw NativeAuthenticationError.invalidResponse }
                return data
            }
            let headerBytes = try decode(segments[0]), claimsBytes = try decode(segments[1])
            try AuthenticationJSON.validate(headerBytes)
            try AuthenticationJSON.validate(claimsBytes)
            let header = try JSONDecoder().decode(Header.self, from: headerBytes)
            guard let headerObject = try JSONSerialization.jsonObject(with: headerBytes) as? [String: Any],
                  Set(headerObject.keys).isSubset(of: ["alg", "kid", "typ"]),
                  header.alg == "RS256", !header.kid.isEmpty, header.kid.utf8.count <= 255,
                  header.kid == expectedKeyID, header.typ == nil || header.typ == "JWT",
                  SecKeyIsAlgorithmSupported(key, .verify, .rsaSignatureMessagePKCS1v15SHA256)
            else { throw NativeAuthenticationError.invalidResponse }
            let message = Data((segments[0] + "." + segments[1]).utf8)
            guard SecKeyVerifySignature(key, .rsaSignatureMessagePKCS1v15SHA256, message as CFData,
                                       try decode(segments[2]) as CFData, nil)
            else { throw NativeAuthenticationError.invalidResponse }
            let claims = try JSONDecoder().decode(Claims.self, from: claimsBytes)
            let seconds = Int64(now.timeIntervalSince1970)
            guard claims.iss == issuer, !claims.sub.isEmpty, claims.sub.utf8.count <= 255,
                  !claims.aud.values.isEmpty, claims.aud.values.allSatisfy({ $0 == clientID }),
                  Set(claims.aud.values).count == claims.aud.values.count,
                  claims.azp == nil || claims.azp == clientID,
                  claims.aud.values.count == 1 || claims.azp == clientID,
                  claims.exp > seconds - 60, claims.iat <= seconds + 60,
                  claims.iat >= 0, claims.exp > claims.iat,
                  claims.nbf.map({ $0 <= seconds + 60 && $0 < claims.exp }) ?? true,
                  AuthenticationEncoding.equal(claims.nonce, attempt.nonce)
            else { throw NativeAuthenticationError.invalidResponse }
            if let created = enrollmentChallengeCreatedAt {
                guard let authenticated = claims.auth_time,
                      authenticated >= Int64(created.timeIntervalSince1970) - 60,
                      authenticated >= seconds - 300, authenticated <= seconds + 60
                else { throw NativeAuthenticationError.invalidResponse }
            }
            if let accessHash = claims.at_hash {
                let digest = Data(SHA256.hash(data: Data(accessToken.utf8))).prefix(16)
                guard AuthenticationEncoding.equal(accessHash, AuthenticationEncoding.base64URL(Data(digest)))
                else { throw NativeAuthenticationError.invalidResponse }
            }
            return NativeVerifiedIdentity(issuer: claims.iss, subject: claims.sub)
        } catch { throw NativeAuthenticationError.invalidResponse }
    }
}
