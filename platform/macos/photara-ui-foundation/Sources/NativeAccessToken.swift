import Foundation
import Security

/// Resolves a recovered identity only after signature and pinned API-policy
/// validation. Unverified token claims never unlock the bound Rust journal.
enum NativeAccessToken {
    private struct Audience: Decodable {
        let values: [String]
        init(from decoder: Decoder) throws {
            let container = try decoder.singleValueContainer()
            if let value = try? container.decode(String.self) { values = [value] }
            else { values = try container.decode([String].self) }
        }
    }
    private struct Claims: Decodable {
        let iss, sub, azp, scope: String
        let aud: Audience
        let exp, iat: Int64
        let nbf: Int64?
        let gty: String?
    }
    static func verify(_ token: String, environment: ReleaseEnvironment,
                       key: SecKey, keyID: String, now: Date) throws -> NativeVerifiedIdentity {
        do {
            guard try NativeJWTEncoding.keyID(token) == keyID else { throw NativeAuthenticationError.invalidResponse }
            let parts = token.split(separator: ".", omittingEmptySubsequences: false)
            let bytes = try NativeJWTEncoding.decode(String(parts[1]))
            try AuthenticationJSON.validate(bytes)
            let header = try JSONSerialization.jsonObject(with: NativeJWTEncoding.decode(String(parts[0]))) as? [String: Any]
            guard header?["typ"] == nil || header?["typ"] as? String == "JWT",
                  SecKeyVerifySignature(key, .rsaSignatureMessagePKCS1v15SHA256,
                    Data((parts[0] + "." + parts[1]).utf8) as CFData,
                    try NativeJWTEncoding.decode(String(parts[2])) as CFData, nil)
            else { throw NativeAuthenticationError.invalidResponse }
            let claims = try JSONDecoder().decode(Claims.self, from: bytes)
            let seconds = Int64(now.timeIntervalSince1970)
            let audience = Set(claims.aud.values)
            guard claims.iss == environment.auth0Issuer.absoluteString,
                  !claims.sub.isEmpty, claims.sub.utf8.count <= 255,
                  claims.azp == environment.nativeClientId,
                  audience.count == claims.aud.values.count,
                  audience.contains(environment.auth0Audience),
                  audience.isSubset(of: [environment.auth0Audience, environment.auth0Issuer.absoluteString + "userinfo"]),
                  claims.scope.split(separator: " ").contains("photara:onboard"),
                  claims.gty != "client-credentials", claims.iat >= 0,
                  claims.iat <= seconds + 60, claims.exp > seconds,
                  claims.exp > claims.iat, claims.exp - claims.iat <= 600,
                  claims.nbf.map({ $0 <= seconds + 60 && $0 < claims.exp }) ?? true
            else { throw NativeAuthenticationError.invalidResponse }
            return NativeVerifiedIdentity(issuer: claims.iss, subject: claims.sub)
        } catch { throw NativeAuthenticationError.invalidResponse }
    }
}
