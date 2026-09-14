// Test-only process. No database, provider or token endpoint exists here.
import Foundation
import Darwin

struct Configuration: Decodable {
    let port: UInt16
    let events: String
    let readyFile: String?
    let bindFile: String?
    let delayHealth: Double?
    let firstHealthDelay: Double?
    let wrongCapabilities: Bool?
    let wrongHealth: Bool?
    let exitBeforeBind: Bool?
    let ignoreTerm: Bool?
}
let configPath = ProcessInfo.processInfo.environment["PHOTARA_FURNACE_CONFIG"]!
let config = try JSONDecoder().decode(Configuration.self, from: Data(contentsOf: URL(fileURLWithPath: configPath)))
precondition(config.port > 1024 && config.port != 8080)
// The real signed operator must strip unrelated inherited values. These are
// inert fixture credentials, never production values, and never printed.
let environment = ProcessInfo.processInfo.environment
precondition(environment["PHOTARA_DB_API_URL"]?.contains("synthetic-furnace") == true)
precondition(environment["PHOTARA_FORBIDDEN_INHERITANCE"] == nil)
precondition(environment["DYLD_INSERT_LIBRARIES"] == nil)
func event(_ name: String) {
    let fd = open(config.events, O_WRONLY | O_CREAT | O_APPEND, 0o600)
    guard fd >= 0 else { exit(90) }
    let data = Data("\(getpid()) \(name)\n".utf8)
    data.withUnsafeBytes { bytes in _ = write(fd, bytes.baseAddress, bytes.count) }
    close(fd)
}
func waitFor(_ path: String?) {
    if let path { while !FileManager.default.fileExists(atPath: path) { usleep(10_000) } }
}
event("started")
if config.ignoreTerm == true { signal(SIGTERM, SIG_IGN) }
signal(SIGPIPE, SIG_IGN)
if config.exitBeforeBind == true { event("exit-before-bind"); exit(12) }
waitFor(config.bindFile)
let listener = socket(AF_INET, SOCK_STREAM, 0)
precondition(listener >= 0)
var reuse: Int32 = 1
setsockopt(listener, SOL_SOCKET, SO_REUSEADDR, &reuse, socklen_t(MemoryLayout.size(ofValue: reuse)))
var address = sockaddr_in()
address.sin_len = UInt8(MemoryLayout<sockaddr_in>.size)
address.sin_family = sa_family_t(AF_INET)
address.sin_port = config.port.bigEndian
address.sin_addr.s_addr = inet_addr("127.0.0.1")
let bound = withUnsafePointer(to: &address) {
    $0.withMemoryRebound(to: sockaddr.self, capacity: 1) { bind(listener, $0, socklen_t(MemoryLayout<sockaddr_in>.size)) }
}
if bound != 0 { event("exit-bind-race"); exit(98) }
precondition(listen(listener, 64) == 0)
event("bound")
final class RequestCounter: @unchecked Sendable {
    private let lock = NSLock()
    private var count = 0
    func next() -> Int { lock.lock(); defer { lock.unlock() }; count += 1; return count }
}
let healthRequests = RequestCounter()
func serve(_ client: Int32) {
    defer { close(client) }
    var timeout = timeval(tv_sec: 3, tv_usec: 0)
    setsockopt(client, SOL_SOCKET, SO_RCVTIMEO, &timeout, socklen_t(MemoryLayout<timeval>.size))
    var buffer = [UInt8](repeating: 0, count: 8192)
    let count = recv(client, &buffer, buffer.count, 0)
    guard count > 0 else { return }
    let request = String(decoding: buffer.prefix(count), as: UTF8.self)
    precondition(!request.lowercased().contains("authorization:") && !request.lowercased().contains("photara-device-credential:"))
    let path = request.split(separator: " ").dropFirst().first.map(String.init) ?? ""
    var status = 200
    let body: Data
    switch path {
    case "/health/ready":
        event("health")
        let delay = healthRequests.next() == 1 ? (config.firstHealthDelay ?? config.delayHealth) : config.delayHealth
        if let delay { usleep(useconds_t(delay * 1_000_000)) }
        if let ready = config.readyFile, !FileManager.default.fileExists(atPath: ready) {
            status = 503; body = Data("{\"error\":\"unavailable\"}".utf8)
        } else { body = Data((config.wrongHealth == true ? "{\"ready\":false}" : "{\"ready\":true}").utf8) }
    case "/health/live": body = Data("{\"healthy\":true}".utf8)
    case "/v1/onboarding/capabilities":
        event("capabilities")
        body = try! JSONSerialization.data(withJSONObject: [
            "schema": "photara.onboarding.v1", "environment_id": config.wrongCapabilities == true ? "stale" : "synthetic-furnace",
            "service_origin": "http://127.0.0.1:\(config.port)/", "minimum_api": "3", "canonical_codec": "photara.canonical-json.v1",
            "max_body_bytes": "65536", "max_token_bytes": "16384", "challenge_ttl_seconds": "300", "authentication_profile": "auth0-rs256-api-userinfo-v1", "providers": ["google"]
        ], options: [.sortedKeys, .withoutEscapingSlashes])
    default: status = 404; body = Data()
    }
    var response = Data("HTTP/1.1 \(status) Fixture\r\nContent-Type: application/json\r\nContent-Length: \(body.count)\r\nConnection: close\r\n\r\n".utf8)
    response.append(body)
    response.withUnsafeBytes { raw in
        var offset = 0
        while offset < raw.count {
            let count = send(client, raw.baseAddress!.advanced(by: offset), raw.count - offset, 0)
            if count <= 0 { break }; offset += count
        }
    }
}
while true {
    let client = accept(listener, nil, nil)
    if client >= 0 { DispatchQueue.global().async { serve(client) } }
}
