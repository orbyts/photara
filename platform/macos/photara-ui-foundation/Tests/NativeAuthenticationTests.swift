import Foundation
import CryptoKit
import Security

@main
struct NativeAuthenticationTests {
    @MainActor static func main() async throws {
        var assertions = 0
        func check(_ condition: Bool) {
            precondition(condition, "Synthetic authentication invariant failed")
            assertions += 1
        }
        func rejects(_ body: () throws -> Void) {
            do { try body(); preconditionFailure("Unsafe synthetic input was accepted") }
            catch { assertions += 1 }
        }
        let now = Date(timeIntervalSince1970: 1_800_000_000)
        let callback = URL(string: "com.photara.synthetic://auth/callback")!
        let nonce = String(repeating: "N", count: 43)
        func attempt() throws -> NativePKCEAttempt {
            try NativePKCEAttempt(callbackURL: callback, nonce: nonce, generation: 7, now: now)
        }
        func response(_ attempt: NativePKCEAttempt, suffix: String = "") -> URL {
            URL(string: callback.absoluteString + "?state=" + attempt.state + "&code=synthetic-code" + suffix)!
        }
        var valid = try attempt()
        check(valid.state.count == 43 && valid.verifier.count == 43 && valid.challenge.count == 43)
        check(valid.state != valid.verifier && valid.state != nonce)
        let randomCredential = try AuthenticationEncoding.random()
        check(try AuthenticationEncoding.decodeBase64URL(randomCredential).count == 32)
        rejects { _ = try AuthenticationEncoding.decodeBase64URL("not+pinned") }
        let credentialReference = NativeCredentialReference.create()
        check(credentialReference.hasPrefix("keychain:") && UUID(uuidString: String(credentialReference.dropFirst("keychain:".count))) != nil)
        let callbackReachedMainActor = await withCheckedContinuation { continuation in
            let handler = NativeAuthenticationCallback.handler { _, _ in
                continuation.resume(returning: Thread.isMainThread)
            }
            DispatchQueue.global().async { handler(nil, nil) }
        }
        check(callbackReachedMainActor)
        check(try valid.consume(response(valid), generation: 7, now: now) == "synthetic-code")
        rejects { _ = try valid.consume(response(valid), generation: 7, now: now) }
        var cancelled = try attempt()
        cancelled.cancel()
        rejects { _ = try cancelled.consume(response(cancelled), generation: 7, now: now) }
        for suffix in ["&state=duplicate", "&code=duplicate", "&error=denied", "&untrusted=value", "&error_description=unexpected", "#fragment"] {
            var a = try attempt()
            rejects { _ = try a.consume(response(a, suffix: suffix), generation: 7, now: now) }
            check(a.consumed)
        }
        for url in ["com.photara.synthetic://wrong/callback", "com.photara.synthetic://auth/other", "another://auth/callback", "com.photara.synthetic://user@auth/callback", "com.photara.synthetic://auth:443/callback"] {
            var a = try attempt()
            rejects { _ = try a.consume(URL(string: url + "?state=" + a.state + "&code=x")!, generation: 7, now: now) }
        }
        var wrongState = try attempt()
        rejects { _ = try wrongState.consume(URL(string: callback.absoluteString + "?state=wrong&code=x")!, generation: 7, now: now) }
        var stale = try attempt()
        rejects { _ = try stale.consume(response(stale), generation: 8, now: now) }
        var expired = try attempt()
        rejects { _ = try expired.consume(response(expired), generation: 7, now: now.addingTimeInterval(300)) }
        var future = try attempt()
        rejects { _ = try future.consume(response(future), generation: 7, now: now.addingTimeInterval(-1)) }

        for malformed in ["{\"iss\":1,\"iss\":2}", "{\"nested\":{\"x\":1,\"x\":2}}", "{\"iss\":1,\"\\u0069ss\":2}", "{\"x\":true} false", "[1,]", "{\"x\":1,}"] {
            rejects { try AuthenticationJSON.validate(Data(malformed.utf8)) }
        }
        try AuthenticationJSON.validate(Data("{\"nested\":[true,false,null,123,\"a\\\"b\"],\"distinct\":{\"a\":1}}".utf8))
        assertions += 1
        let canonicalCapability = Data("{\"schema\":\"photara.onboarding.v1\",\"service_origin\":\"http://127.0.0.1:8080/\"}".utf8)
        try AuthenticationJSON.validateCanonicalObject(
            canonicalCapability,
            keys: ["schema", "service_origin"]
        )
        assertions += 1
        rejects {
            try AuthenticationJSON.validateCanonicalObject(
                Data(#"{"schema":"photara.onboarding.v1","service_origin":"http:\/\/127.0.0.1:8080\/"}"#.utf8),
                keys: ["schema", "service_origin"]
            )
        }

        // In-memory synthetic RSA key: kSecAttrIsPermanent is never enabled.
        guard let privateKey = SecKeyCreateRandomKey([kSecAttrKeyType: kSecAttrKeyTypeRSA, kSecAttrKeySizeInBits: 2048] as CFDictionary, nil)
        else { throw NativeAuthenticationError.keychainUnavailable }
        let publicKey = SecKeyCopyPublicKey(privateKey)!
        let verifier = NativeIDTokenVerifier(issuer: "https://synthetic.invalid/", clientID: "synthetic-native")
        let a = try attempt()
        let access = "synthetic-api-token"
        let hash = AuthenticationEncoding.base64URL(Data(SHA256.hash(data: Data(access.utf8))).prefix(16))
        let claims: [String: Any] = ["iss": "https://synthetic.invalid/", "sub": "synthetic-subject", "aud": "synthetic-native", "azp": "synthetic-native", "iat": 1_800_000_000, "exp": 1_800_000_600, "nonce": nonce, "auth_time": 1_800_000_000, "at_hash": hash]
        func token(_ claims: [String: Any], header: [String: Any] = ["alg": "RS256", "kid": "synthetic-key", "typ": "JWT"]) throws -> String {
            let encoded = try [header, claims].map { AuthenticationEncoding.base64URL(try JSONSerialization.data(withJSONObject: $0, options: .sortedKeys)) }.joined(separator: ".")
            let signature = SecKeyCreateSignature(privateKey, .rsaSignatureMessagePKCS1v15SHA256, Data(encoded.utf8) as CFData, nil)! as Data
            return encoded + "." + AuthenticationEncoding.base64URL(signature)
        }
        func verify(_ token: String, accessToken: String = access, keyID: String = "synthetic-key") throws -> NativeVerifiedIdentity {
            try verifier.verify(token, accessToken: accessToken, attempt: a, key: publicKey, expectedKeyID: keyID, now: now, enrollmentChallengeCreatedAt: now)
        }
        check(try verify(token(claims)) == NativeVerifiedIdentity(issuer: "https://synthetic.invalid/", subject: "synthetic-subject"))
        for (key, value) in [("iss", "https://synthetic.invalid"), ("aud", "synthetic-native-attacker"), ("azp", "other-client"), ("nonce", "wrong"), ("sub", ""), ("at_hash", "wrong")] {
            var bad = claims; bad[key] = value
            rejects { _ = try verify(token(bad)) }
        }
        for (key, value) in [("exp", 1_799_999_000), ("iat", 1_800_100_000), ("nbf", 1_800_100_000), ("auth_time", 1_799_990_000)] {
            var bad = claims; bad[key] = value
            rejects { _ = try verify(token(bad)) }
        }
        for header in [["alg": "none", "kid": "synthetic-key"], ["alg": "HS256", "kid": "synthetic-key"], ["alg": "RS256", "kid": "synthetic-key", "jku": "https://attacker.invalid/keys"]] {
            rejects { _ = try verify(token(claims, header: header)) }
        }
        rejects { _ = try verify(token(claims), accessToken: "substituted") }
        rejects { _ = try verify(token(claims), keyID: "unknown") }
        rejects { _ = try verify(String(repeating: "x", count: 16_385)) }

        let identity = NativeVerifiedIdentity(issuer: "https://synthetic.invalid/", subject: "synthetic-subject")
        let operation = UUID()
        var machine = NativeOnboardingStateMachine()
        _ = try machine.begin(signingAvailable: false, emptyLocalLibrary: true, operation: operation)
        check(machine.state == .signingRequired && machine.operationID == nil)
        _ = try machine.begin(signingAvailable: true, emptyLocalLibrary: false, operation: operation)
        check(machine.state == .localContentEnrollmentRequired)
        let generation = try machine.begin(signingAvailable: true, emptyLocalLibrary: true, operation: operation)
        try machine.authenticated(identity, generation: generation)
        try machine.dispatchRecorded(generation: generation)
        machine.cancel()
        check(machine.state == .outcomeUnknown)
        check(!machine.receiptRecorded(operation: operation, identity: identity, generation: generation))
        rejects { _ = try machine.begin(signingAvailable: true, emptyLocalLibrary: true, operation: UUID()) }
        let retry = try machine.begin(signingAvailable: true, emptyLocalLibrary: true, operation: operation)
        rejects { try machine.authenticated(NativeVerifiedIdentity(issuer: identity.issuer, subject: "other-account"), generation: retry) }
        try machine.authenticated(identity, generation: retry)
        check(machine.receiptRecorded(operation: operation, identity: identity, generation: retry))
        try machine.reconciled(generation: retry, sameLibrary: true, transactionApplied: false)
        check(machine.state == .reconciliationRequired && !machine.hasCloudBinding)

        var success = NativeOnboardingStateMachine()
        let g = try success.begin(signingAvailable: true, emptyLocalLibrary: true, operation: operation)
        try success.authenticated(identity, generation: g)
        try success.dispatchRecorded(generation: g)
        check(success.receiptRecorded(operation: operation, identity: identity, generation: g))
        try success.reconciled(generation: g, sameLibrary: true, transactionApplied: true)
        check(success.state == .cloudReady)
        success.networkUnavailable()
        check(success.state == .cloudOffline && success.hasCloudBinding)
        success.logout()
        check(success.state == .signedOutCached && success.hasCloudBinding)
        var expiredMachine = NativeOnboardingStateMachine()
        let oldGeneration = try expiredMachine.begin(signingAvailable: true, emptyLocalLibrary: true, operation: operation)
        try expiredMachine.authenticated(identity, generation: oldGeneration)
        try expiredMachine.dispatchRecorded(generation: oldGeneration)
        expiredMachine.networkUnavailable()
        rejects { try expiredMachine.abandonedExpired(operation: UUID(), generation: oldGeneration) }
        try expiredMachine.abandonedExpired(operation: operation, generation: oldGeneration)
        check(expiredMachine.state == .freshSignInRequired && !expiredMachine.submitted)
        check(!expiredMachine.receiptRecorded(operation: operation, identity: identity, generation: oldGeneration))
        let newGeneration = try expiredMachine.begin(signingAvailable: true, emptyLocalLibrary: true, operation: UUID())
        check(newGeneration > oldGeneration)
        check(!expiredMachine.receiptRecorded(operation: operation, identity: identity, generation: oldGeneration))
        rejects { try expiredMachine.authenticated(identity, generation: oldGeneration) }
        try expiredMachine.authenticated(identity, generation: newGeneration)
        for origin in ["http://example.com", "http://localhost:8080", "http://127.0.0.1:8081", "https://user@example.com", "https://example.com/?redirect=x"] {
            rejects { _ = try NativePinnedTransport(origin: URL(string: origin)!, developmentLoopback: true) }
        }
        _ = try NativePinnedTransport(origin: URL(string: "http://127.0.0.1:8080")!, developmentLoopback: true)
        _ = try NativePinnedTransport(origin: URL(string: "http://127.0.0.1:8080/")!, developmentLoopback: true)
        rejects { _ = try NativePinnedTransport(origin: URL(string: "http://127.0.0.1:8080")!, developmentLoopback: false) }

        // Test storage is an in-memory protocol implementation. Only the empty
        // flock file goes to a uniquely created temporary directory.
        final class MemoryCredentials: NativeCredentialPersistence {
            var value = NativeStoredCredential(refreshToken: "synthetic-old", refreshInFlight: false, deviceCredential: String(repeating: "D", count: 43))
            var writes = 0
            var failAtWrite: Int?
            func read(_ namespace: NativeCredentialNamespace) throws -> NativeStoredCredential? { value }
            func replace(_ newValue: NativeStoredCredential, namespace: NativeCredentialNamespace) throws {
                writes += 1
                if writes == failAtWrite { throw NativeAuthenticationError.keychainUnavailable }
                value = newValue
            }
        }
        let namespace = try NativeCredentialNamespace(configuration: .current, installation: UUID(), issuer: ReleaseConfiguration.current.environment.auth0Issuer.absoluteString, subject: "synthetic-subject")
        let testDirectory = FileManager.default.temporaryDirectory.appending(path: "photara-refresh-synthetic-" + UUID().uuidString)
        try FileManager.default.createDirectory(at: testDirectory, withIntermediateDirectories: false, attributes: [.posixPermissions: 0o700])
        defer { try? FileManager.default.removeItem(at: testDirectory) }
        let lock = testDirectory.appending(path: "refresh.lock")
        let storage = MemoryCredentials()
        let refreshed = try await NativeRefresh.perform(store: storage, namespace: namespace, lockURL: lock) { old in
            check(old == "synthetic-old" && storage.value.refreshInFlight && storage.writes == 1)
            // A second process/open file description cannot run a concurrent
            // refresh while the first holds this exact installation lock.
            do {
                _ = try await NativeRefresh.perform(store: storage, namespace: namespace, lockURL: lock) { _ in
                    preconditionFailure("Concurrent refresh must not exchange")
                }
                preconditionFailure("Concurrent refresh must not acquire lock")
            } catch { assertions += 1 }
            return NativeRefreshResult(accessToken: "synthetic-access", refreshToken: "synthetic-new")
        }
        check(refreshed.accessToken == "synthetic-access" && storage.value.refreshToken == "synthetic-new" && !storage.value.refreshInFlight && storage.writes == 2)
        for failingWrite in [0, 1, 2] {
            let failed = MemoryCredentials()
            if failingWrite > 0 { failed.failAtWrite = failingWrite }
            var exchanges = 0
            do {
                _ = try await NativeRefresh.perform(store: failed, namespace: namespace, lockURL: lock) { _ in
                    exchanges += 1
                    if failingWrite == 0 { throw NativeAuthenticationError.transportUnavailable }
                    return NativeRefreshResult(accessToken: "synthetic-access", refreshToken: "synthetic-new")
                }
                preconditionFailure("Injected failure must prevent access publication")
            } catch { assertions += 1 }
            check(exchanges == (failingWrite == 1 ? 0 : 1))
            check(failed.value.refreshInFlight == (failingWrite != 1))
            if failingWrite != 1 {
                do {
                    _ = try await NativeRefresh.perform(store: failed, namespace: namespace, lockURL: lock) { _ in
                        preconditionFailure("Ambiguous rotating refresh token must never be replayed")
                    }
                    preconditionFailure("Pending marker must require reauthentication")
                } catch { assertions += 1 }
            }
        }
        print("native-authentication: \(assertions) synthetic security assertions passed")
    }
}
