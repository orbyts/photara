import Foundation
import Security

protocol NativeAuthenticationHTTP: Sendable {
    func send(path: String, method: String, body: Data?, headers: [String: String]) async throws -> (Data, Int)
}
extension NativePinnedTransport: NativeAuthenticationHTTP {}

enum NativeJWTEncoding {
    static func decode(_ value: String) throws -> Data {
        guard !value.isEmpty, value.utf8.allSatisfy({ (48...57).contains($0) || (65...90).contains($0) || (97...122).contains($0) || $0 == 45 || $0 == 95 })
        else { throw NativeAuthenticationError.invalidResponse }
        let base64 = value.replacingOccurrences(of: "-", with: "+").replacingOccurrences(of: "_", with: "/")
        guard let data = Data(base64Encoded: base64 + String(repeating: "=", count: (4 - base64.count % 4) % 4)),
              AuthenticationEncoding.base64URL(data) == value else { throw NativeAuthenticationError.invalidResponse }
        return data
    }

    static func keyID(_ token: String) throws -> String {
        guard token.utf8.count <= 16_384 else { throw NativeAuthenticationError.invalidResponse }
        let parts = token.split(separator: ".", omittingEmptySubsequences: false)
        guard parts.count == 3 else { throw NativeAuthenticationError.invalidResponse }
        let bytes = try decode(String(parts[0]))
        try AuthenticationJSON.validate(bytes)
        guard let header = try JSONSerialization.jsonObject(with: bytes) as? [String: Any],
              Set(header.keys).isSubset(of: ["alg", "kid", "typ"]),
              header["alg"] as? String == "RS256",
              let keyID = header["kid"] as? String, !keyID.isEmpty, keyID.utf8.count <= 255
        else { throw NativeAuthenticationError.invalidResponse }
        return keyID
    }

    static func rsaPublicKey(modulus: String, exponent: String) throws -> Data {
        let n = try decode(modulus), e = try decode(exponent)
        guard let first = n.first, first != 0, (2048...4096).contains(n.count * 8 - first.leadingZeroBitCount),
              e == Data([1, 0, 1]) else { throw NativeAuthenticationError.invalidResponse }
        func tlv(_ tag: UInt8, _ content: Data) -> Data {
            var length: [UInt8] = []
            var count = content.count
            repeat { length.insert(UInt8(count & 255), at: 0); count >>= 8 } while count > 0
            if content.count >= 128 { length.insert(0x80 | UInt8(length.count), at: 0) }
            return Data([tag] + length) + content
        }
        func integer(_ value: Data) -> Data {
            tlv(2, value.first! & 0x80 != 0 ? Data([0]) + value : value)
        }
        return tlv(0x30, integer(n) + integer(e))
    }

    static func key(_ data: Data) throws -> SecKey {
        guard let key = SecKeyCreateWithData(data as CFData,
            [kSecAttrKeyType: kSecAttrKeyTypeRSA, kSecAttrKeyClass: kSecAttrKeyClassPublic] as CFDictionary, nil)
        else { throw NativeAuthenticationError.invalidResponse }
        return key
    }
}

/// One actor/cache per native issuer. Concurrent misses share a single refresh;
/// unknown key IDs get at most one network refresh in a 30-second window. An
/// unavailable/expired cache never admits a token. Token claims choose no URL.
actor NativeJWKSCache {
    private let issuer: String
    private let http: any NativeAuthenticationHTTP
    private var keys: [String: Data] = [:]
    private var expiresAt = Date.distantPast
    private var lastRefreshAt = Date.distantPast
    private var inFlight: (UUID, Task<[String: Data], Error>)?

    init(issuer: URL, http: any NativeAuthenticationHTTP) throws {
        guard let components = URLComponents(url: issuer, resolvingAgainstBaseURL: false),
              components.scheme == "https", components.host != nil, components.path == "/",
              components.user == nil, components.password == nil, components.port == nil,
              components.query == nil, components.fragment == nil
        else { throw NativeAuthenticationError.invalidResponse }
        self.issuer = issuer.absoluteString
        self.http = http
    }

    func keyData(keyID: String, now: Date) async throws -> Data {
        guard !keyID.isEmpty, keyID.utf8.count <= 255 else { throw NativeAuthenticationError.invalidResponse }
        if now < expiresAt, let key = keys[keyID] { return key }
        if let flight = inFlight {
            let fetched = try await flight.1.value
            guard let key = fetched[keyID] else { throw NativeAuthenticationError.invalidResponse }
            return key
        }
        guard now.timeIntervalSince(lastRefreshAt) >= 30 else { throw NativeAuthenticationError.invalidResponse }
        lastRefreshAt = now
        let identifier = UUID(), issuer = self.issuer, http = self.http
        let task = Task { try await Self.fetch(issuer: issuer, http: http) }
        inFlight = (identifier, task)
        defer { if inFlight?.0 == identifier { inFlight = nil } }
        let fetched = try await task.value
        keys = fetched
        expiresAt = now.addingTimeInterval(3600)
        guard let key = keys[keyID] else { throw NativeAuthenticationError.invalidResponse }
        return key
    }

    private static func fetch(issuer: String, http: any NativeAuthenticationHTTP) async throws -> [String: Data] {
        do { return try await fetchChecked(issuer: issuer, http: http) }
        catch { throw NativeAuthenticationError.invalidResponse }
    }

    private static func fetchChecked(issuer: String, http: any NativeAuthenticationHTTP) async throws -> [String: Data] {
        struct Discovery: Decodable { let issuer: String; let jwks_uri: String; let token_endpoint: String; let authorization_endpoint: String }
        struct JWK: Decodable { let kty: String; let kid: String?; let alg: String?; let use: String?; let key_ops: [String]?; let n: String?; let e: String? }
        struct JWKS: Decodable { let keys: [JWK] }
        let (discoveryData, discoveryStatus) = try await http.send(path: "/.well-known/openid-configuration", method: "GET", body: nil, headers: ["Accept": "application/json"])
        guard discoveryStatus == 200 else { throw NativeAuthenticationError.invalidResponse }
        try AuthenticationJSON.validate(discoveryData, maximum: 65_536)
        let discovery = try JSONDecoder().decode(Discovery.self, from: discoveryData)
        guard discovery.issuer == issuer, discovery.jwks_uri == issuer + ".well-known/jwks.json",
              discovery.token_endpoint == issuer + "oauth/token", discovery.authorization_endpoint == issuer + "authorize"
        else { throw NativeAuthenticationError.invalidResponse }
        let (keyData, keyStatus) = try await http.send(path: "/.well-known/jwks.json", method: "GET", body: nil, headers: ["Accept": "application/json"])
        guard keyStatus == 200 else { throw NativeAuthenticationError.invalidResponse }
        try AuthenticationJSON.validate(keyData, maximum: 65_536)
        let jwks = try JSONDecoder().decode(JWKS.self, from: keyData)
        guard !jwks.keys.isEmpty, jwks.keys.count <= 32 else { throw NativeAuthenticationError.invalidResponse }
        var result: [String: Data] = [:], seen = Set<String>()
        for key in jwks.keys {
            guard let keyID = key.kid, !keyID.isEmpty, keyID.utf8.count <= 255, seen.insert(keyID).inserted
            else { throw NativeAuthenticationError.invalidResponse }
            guard key.kty == "RSA", key.alg == nil || key.alg == "RS256", key.use == nil || key.use == "sig",
                  key.key_ops == nil || key.key_ops == ["verify"] else { continue }
            guard let n = key.n, let e = key.e else { throw NativeAuthenticationError.invalidResponse }
            let data = try NativeJWTEncoding.rsaPublicKey(modulus: n, exponent: e)
            _ = try NativeJWTEncoding.key(data)
            result[keyID] = data
        }
        guard !result.isEmpty else { throw NativeAuthenticationError.invalidResponse }
        return result
    }
}

/// Transient result; access and ID tokens are intentionally not Codable.
struct NativeAuthenticatedTokens {
    let identity: NativeVerifiedIdentity
    let accessToken: String
    let refreshToken: String
    let enrollmentIDToken: String
    let expiresAt: Date
    var profile: NativeAccountProfile? = nil
}

actor NativeAuth0Client {
    private let environment: ReleaseEnvironment
    private let http: any NativeAuthenticationHTTP
    private let jwks: NativeJWKSCache
    private var exchangedAttempts: [String: Date] = [:]

    init(environment: ReleaseEnvironment) throws {
        let http = try NativePinnedTransport(origin: environment.auth0Issuer, developmentLoopback: false)
        self.environment = environment
        self.http = http
        jwks = try NativeJWKSCache(issuer: environment.auth0Issuer, http: http)
    }

    init(environment: ReleaseEnvironment, http: any NativeAuthenticationHTTP) throws {
        self.environment = environment
        self.http = http
        jwks = try NativeJWKSCache(issuer: environment.auth0Issuer, http: http)
    }

    private struct TokenResponse: Decodable {
        let access_token: String
        let refresh_token: String?
        let id_token: String?
        let token_type: String
        let expires_in: Int
        let scope: String?
    }

    private func token(_ fields: [String: String]) async throws -> TokenResponse {
        var allowed = CharacterSet.alphanumerics
        allowed.insert(charactersIn: "-._~")
        let form = fields.sorted { $0.key < $1.key }.map {
            $0.key.addingPercentEncoding(withAllowedCharacters: allowed)! + "=" + $0.value.addingPercentEncoding(withAllowedCharacters: allowed)!
        }.joined(separator: "&")
        do {
            let (data, status) = try await http.send(path: "/oauth/token", method: "POST", body: Data(form.utf8),
                headers: ["Content-Type": "application/x-www-form-urlencoded", "Accept": "application/json"])
            guard status == 200 else { throw NativeAuthenticationError.invalidResponse }
            try AuthenticationJSON.validate(data, maximum: 65_536)
            let response = try JSONDecoder().decode(TokenResponse.self, from: data)
            guard response.token_type == "Bearer", !response.access_token.isEmpty,
                  response.access_token.utf8.count <= 16_384, (1...600).contains(response.expires_in),
                  response.scope.map({ Set($0.split(separator: " ")).contains("photara:onboard") }) ?? true
            else { throw NativeAuthenticationError.invalidResponse }
            return response
        } catch is CancellationError { throw NativeAuthenticationError.cancelled }
        catch { throw NativeAuthenticationError.invalidResponse }
    }

    func exchange(code: String, attempt: NativePKCEAttempt, challengeCreatedAt: Date, now: Date) async throws -> NativeAuthenticatedTokens {
        guard attempt.consumed, now >= attempt.startedAt, now < attempt.deadline,
              attempt.callbackURL == environment.callbackURL,
              !code.isEmpty, code.utf8.count <= 4096
        else { throw NativeAuthenticationError.invalidResponse }
        try Task.checkCancellation()
        exchangedAttempts = exchangedAttempts.filter { $0.value > now }
        guard exchangedAttempts[attempt.state] == nil, exchangedAttempts.count < 32
        else { throw NativeAuthenticationError.consumedAttempt }
        // Persist only in process memory. Even an ambiguous/lost exchange result
        // consumes this code: retries must start a fresh browser attempt.
        exchangedAttempts[attempt.state] = attempt.deadline
        let response = try await token(["grant_type": "authorization_code", "client_id": environment.nativeClientId,
            "code": code, "redirect_uri": environment.callbackURL.absoluteString, "code_verifier": attempt.verifier])
        guard let idToken = response.id_token, let refreshToken = response.refresh_token,
              !refreshToken.isEmpty, refreshToken.utf8.count <= 16_384
        else { throw NativeAuthenticationError.invalidResponse }
        let keyID = try NativeJWTEncoding.keyID(idToken)
        let keyData = try await jwks.keyData(keyID: keyID, now: now)
        let identity = try NativeIDTokenVerifier(issuer: environment.auth0Issuer.absoluteString, clientID: environment.nativeClientId)
            .verify(idToken, accessToken: response.access_token, attempt: attempt, key: NativeJWTEncoding.key(keyData),
                    expectedKeyID: keyID, now: now, enrollmentChallengeCreatedAt: challengeCreatedAt)
        try Task.checkCancellation()
        return NativeAuthenticatedTokens(identity: identity, accessToken: response.access_token,
            refreshToken: refreshToken, enrollmentIDToken: idToken, expiresAt: now.addingTimeInterval(Double(response.expires_in)),
            profile: NativeAccountProfile.fromVerifiedIDToken(idToken))
    }

    /// Must be called only within NativeRefresh.perform's durable marker/lock.
    func verifyAccess(_ token: String, now: Date) async throws -> NativeVerifiedIdentity {
        let identifier = try NativeJWTEncoding.keyID(token)
        let data = try await jwks.keyData(keyID: identifier, now: now)
        return try NativeAccessToken.verify(token, environment: environment,
            key: NativeJWTEncoding.key(data), keyID: identifier, now: now)
    }

    /// Must be called only within NativeRefresh.perform's durable marker/lock.
    /// Refresh ID tokens are ignored; no refresh response may switch identity.
    func refresh(_ refreshToken: String) async throws -> NativeRefreshResult {
        guard !refreshToken.isEmpty, refreshToken.utf8.count <= 16_384 else { throw NativeAuthenticationError.invalidCredential }
        let response = try await token(["grant_type": "refresh_token", "client_id": environment.nativeClientId, "refresh_token": refreshToken])
        guard let rotated = response.refresh_token, !rotated.isEmpty, rotated.utf8.count <= 16_384,
              rotated != refreshToken else { throw NativeAuthenticationError.invalidResponse }
        return NativeRefreshResult(accessToken: response.access_token, refreshToken: rotated)
    }
}
