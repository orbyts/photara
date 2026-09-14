import Foundation
import Security

private actor SyntheticAuthHTTP: NativeAuthenticationHTTP {
    var responses: [String: (Data, Int)] = [:]
    var counts: [String: Int] = [:]
    var forms: [Data] = []
    func set(_ path: String, _ bytes: Data, status: Int = 200) { responses[path] = (bytes, status) }
    func count(_ path: String) -> Int { counts[path, default: 0] }
    func lastForm() -> Data? { forms.last }
    func send(path: String, method: String, body: Data?, headers: [String: String]) async throws -> (Data, Int) {
        counts[path, default: 0] += 1
        if let body { forms.append(body) }
        await Task.yield()
        guard let response = responses[path] else { throw NativeAuthenticationError.transportUnavailable }
        return response
    }
}

@main
struct NativeAuth0ClientTests {
    @MainActor static func main() async throws {
        var assertions = 0
        func check(_ value: Bool) { precondition(value, "Synthetic Auth0 invariant failed"); assertions += 1 }
        func rejects(_ action: () async throws -> Void) async {
            do { try await action(); preconditionFailure("Unsafe synthetic Auth0 input accepted") }
            catch { assertions += 1 }
        }
        func json(_ value: Any) throws -> Data { try JSONSerialization.data(withJSONObject: value, options: .sortedKeys) }
        let env = ReleaseConfiguration.current.environment, issuer = env.auth0Issuer.absoluteString
        let now = Date(timeIntervalSince1970: 1_800_000_000)
        guard let privateKey = SecKeyCreateRandomKey([kSecAttrKeyType: kSecAttrKeyTypeRSA, kSecAttrKeySizeInBits: 2048] as CFDictionary, nil),
              let publicKey = SecKeyCopyPublicKey(privateKey),
              let representation = SecKeyCopyExternalRepresentation(publicKey, nil) as Data?
        else { throw NativeAuthenticationError.keychainUnavailable }
        let der = Array(representation)
        var position = 0
        func read(_ tag: UInt8) -> Data {
            precondition(der[position] == tag); position += 1
            var length = Int(der[position]); position += 1
            if length & 128 != 0 {
                let count = length & 127; length = 0
                for _ in 0..<count { length = length * 256 + Int(der[position]); position += 1 }
            }
            let content = Data(der[position..<(position + length)])
            if tag != 0x30 { position += length }
            return content
        }
        _ = read(0x30)
        var modulus = read(2)
        if modulus.first == 0 { modulus.removeFirst() }
        let exponent = read(2)
        let n = AuthenticationEncoding.base64URL(modulus), e = AuthenticationEncoding.base64URL(exponent)
        func jwks(_ keyIDs: [String]) throws -> Data {
            try json(["keys": keyIDs.map { ["kid": $0, "kty": "RSA", "alg": "RS256", "use": "sig", "n": n, "e": e] }])
        }
        let discovery = try json(["issuer": issuer, "jwks_uri": issuer + ".well-known/jwks.json", "token_endpoint": issuer + "oauth/token", "authorization_endpoint": issuer + "authorize"])
        func configured() async throws -> SyntheticAuthHTTP {
            let http = SyntheticAuthHTTP()
            await http.set("/.well-known/openid-configuration", discovery)
            await http.set("/.well-known/jwks.json", try jwks(["key-one"]))
            return http
        }

        let http = try await configured()
        let cache = try NativeJWKSCache(issuer: env.auth0Issuer, http: http)
        async let one = cache.keyData(keyID: "key-one", now: now)
        async let two = cache.keyData(keyID: "key-one", now: now)
        check(try await one == two)
        check(await http.count("/.well-known/openid-configuration") == 1)
        check(await http.count("/.well-known/jwks.json") == 1)
        for _ in 0..<8 { await rejects { _ = try await cache.keyData(keyID: "attacker-key", now: now) } }
        check(await http.count("/.well-known/jwks.json") == 1)
        await http.set("/.well-known/jwks.json", try jwks(["key-one", "key-two"]))
        _ = try await cache.keyData(keyID: "key-two", now: now.addingTimeInterval(31))
        check(await http.count("/.well-known/jwks.json") == 2)
        await http.set("/.well-known/openid-configuration", Data(), status: 503)
        _ = try await cache.keyData(keyID: "key-one", now: now.addingTimeInterval(100))
        await rejects { _ = try await cache.keyData(keyID: "key-one", now: now.addingTimeInterval(3700)) }
        await rejects { _ = try await cache.keyData(keyID: "key-one", now: now.addingTimeInterval(3701)) }

        for (field, bad) in [("issuer", String(issuer.dropLast())), ("jwks_uri", "https://attacker.invalid/jwks"), ("token_endpoint", "https://attacker.invalid/token"), ("authorization_endpoint", "https://attacker.invalid/authorize")] {
            let transport = try await configured()
            var fields = ["issuer": issuer, "jwks_uri": issuer + ".well-known/jwks.json", "token_endpoint": issuer + "oauth/token", "authorization_endpoint": issuer + "authorize"]
            fields[field] = bad
            await transport.set("/.well-known/openid-configuration", try json(fields))
            let invalid = try NativeJWKSCache(issuer: env.auth0Issuer, http: transport)
            await rejects { _ = try await invalid.keyData(keyID: "key-one", now: now) }
            check(await transport.count("/.well-known/jwks.json") == 0)
        }
        for keys in [try jwks(["duplicate", "duplicate"]), try json(["keys": [["kid": "key-one", "kty": "RSA", "n": "AQAB", "e": "AQAB"]]])] {
            let transport = try await configured()
            await transport.set("/.well-known/jwks.json", keys)
            let invalid = try NativeJWKSCache(issuer: env.auth0Issuer, http: transport)
            await rejects { _ = try await invalid.keyData(keyID: "key-one", now: now) }
        }
        let redirect = try await configured()
        await redirect.set("/.well-known/openid-configuration", discovery, status: 302)
        let redirected = try NativeJWKSCache(issuer: env.auth0Issuer, http: redirect)
        await rejects { _ = try await redirected.keyData(keyID: "key-one", now: now) }

        func prepare() throws -> NativePKCEAttempt {
            var attempt = try NativePKCEAttempt(callbackURL: env.callbackURL, nonce: String(repeating: "N", count: 43), generation: 1, now: now)
            _ = try attempt.consume(URL(string: env.callbackURL.absoluteString + "?state=" + attempt.state + "&code=synthetic-code")!, generation: 1, now: now)
            return attempt
        }
        func idToken(_ attempt: NativePKCEAttempt) throws -> String {
            let claims: [String: Any] = ["iss": issuer, "sub": "synthetic-user", "aud": env.nativeClientId, "azp": env.nativeClientId, "nonce": attempt.nonce, "iat": 1_800_000_000, "exp": 1_800_000_600, "auth_time": 1_800_000_000,
                "name": "Synthetic Person", "email": "person@example.invalid", "picture": "https://lh3.googleusercontent.com/synthetic"]
            let parts = try [json(["alg": "RS256", "kid": "key-one"]), json(claims)].map(AuthenticationEncoding.base64URL).joined(separator: ".")
            let signature = SecKeyCreateSignature(privateKey, .rsaSignatureMessagePKCS1v15SHA256, Data(parts.utf8) as CFData, nil)! as Data
            return parts + "." + AuthenticationEncoding.base64URL(signature)
        }
        func response(_ attempt: NativePKCEAttempt) throws -> [String: Any] {
            ["access_token": "synthetic-access", "refresh_token": "synthetic-refresh", "id_token": try idToken(attempt), "token_type": "Bearer", "expires_in": 600, "scope": "openid photara:onboard"]
        }
        let tokenHTTP = try await configured(), attempt = try prepare()
        await tokenHTTP.set("/oauth/token", try json(response(attempt)))
        let client = try NativeAuth0Client(environment: env, http: tokenHTTP)
        let result = try await client.exchange(code: "synthetic-code", attempt: attempt, challengeCreatedAt: now, now: now)
        check(result.identity.subject == "synthetic-user" && result.accessToken == "synthetic-access")
        check(result.profile?.name == "Synthetic Person" && result.profile?.email == "person@example.invalid")
        check(result.profile?.picture == "https://lh3.googleusercontent.com/synthetic" && result.profile?.avatar == nil)
        for picture in ["http://lh3.googleusercontent.com/a", "https://googleusercontent.com.attacker.invalid/a", "file:///tmp/a", "https://user:secret@lh3.googleusercontent.com/a"] {
            check(await NativeAccountProfileCache.avatar(picture) == nil)
        }
        let form = String(data: await tokenHTTP.lastForm()!, encoding: .utf8)!
        check(form.contains("code_verifier=" + attempt.verifier) && form.contains("grant_type=authorization_code") && !form.contains("client_secret"))
        await rejects { _ = try await client.exchange(code: "synthetic-code", attempt: attempt, challengeCreatedAt: now, now: now) }
        check(await tokenHTTP.count("/oauth/token") == 1)
        for (field, bad) in [("token_type", "MAC" as Any), ("expires_in", 601 as Any), ("refresh_token", "" as Any), ("id_token", "not-a-jwt" as Any), ("scope", "openid" as Any)] {
            let fresh = try prepare()
            var badResponse = try response(fresh); badResponse[field] = bad
            await tokenHTTP.set("/oauth/token", try json(badResponse))
            await rejects { _ = try await client.exchange(code: "synthetic-code", attempt: fresh, challengeCreatedAt: now, now: now) }
        }
        let lost = try prepare()
        await tokenHTTP.set("/oauth/token", Data(), status: 503)
        await rejects { _ = try await client.exchange(code: "synthetic-code", attempt: lost, challengeCreatedAt: now, now: now) }
        let before = await tokenHTTP.count("/oauth/token")
        await rejects { _ = try await client.exchange(code: "synthetic-code", attempt: lost, challengeCreatedAt: now, now: now) }
        check(await tokenHTTP.count("/oauth/token") == before)
        await tokenHTTP.set("/oauth/token", try json(["access_token": "synthetic-new-access", "refresh_token": "synthetic-new-refresh", "token_type": "Bearer", "expires_in": 600]))
        let rotated = try await client.refresh("synthetic-old-refresh")
        check(rotated.refreshToken == "synthetic-new-refresh")
        await tokenHTTP.set("/oauth/token", try json(["access_token": "synthetic-new-access", "refresh_token": "synthetic-old-refresh", "token_type": "Bearer", "expires_in": 600]))
        await rejects { _ = try await client.refresh("synthetic-old-refresh") }
        func accessToken(_ claims: [String: Any]) throws -> String {
            let parts = try [json(["alg": "RS256", "kid": "key-one"]), json(claims)].map(AuthenticationEncoding.base64URL).joined(separator: ".")
            let signature = SecKeyCreateSignature(privateKey, .rsaSignatureMessagePKCS1v15SHA256, Data(parts.utf8) as CFData, nil)! as Data
            return parts + "." + AuthenticationEncoding.base64URL(signature)
        }
        let accessClaims: [String: Any] = ["iss": issuer, "sub": "synthetic-user", "aud": [env.auth0Audience, issuer + "userinfo"],
            "azp": env.nativeClientId, "scope": "openid photara:onboard", "iat": 1_800_000_000, "exp": 1_800_000_600]
        check(try await client.verifyAccess(accessToken(accessClaims), now: now).subject == "synthetic-user")
        for (field, bad) in [("aud", [issuer + "userinfo"] as Any), ("aud", [env.auth0Audience, "https://other.invalid/userinfo"] as Any),
            ("aud", [env.auth0Audience, env.auth0Audience] as Any), ("aud", [env.auth0Audience, issuer + "userinfo", "extra"] as Any),
            ("azp", "other-client" as Any), ("iss", "https://other.invalid/" as Any), ("scope", "openid" as Any),
            ("exp", 1_800_000_601 as Any), ("gty", "client-credentials" as Any), ("sub", "" as Any)] {
            var changed = accessClaims; changed[field] = bad
            await rejects { _ = try await client.verifyAccess(accessToken(changed), now: now) }
        }
        await rejects { _ = try await client.verifyAccess(accessToken(accessClaims), now: now.addingTimeInterval(601)) }
        print("native-auth0-client: \(assertions) synthetic network assertions passed")
    }
}
