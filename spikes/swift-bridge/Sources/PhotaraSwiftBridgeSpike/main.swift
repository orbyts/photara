import Foundation

enum SpikeError: Error, CustomStringConvertible {
    case invalidUsage
    case invalidJSON
    case missing(String)
    case processFailed(Int32)

    var description: String {
        switch self {
        case .invalidUsage:
            return "usage: photara-swift-bridge-spike <path-to-swift_bridge_server>"
        case .invalidJSON:
            return "bridge emitted invalid JSON"
        case .missing(let expectation):
            return "bridge contract was not observed: \(expectation)"
        case .processFailed(let status):
            return "bridge server exited with status \(status)"
        }
    }
}

struct Observations {
    var applied = false
    var rejected = false
    var portableProject = false
    var portableGraph = false
    var progress = false
    var cancelledProgress = false
    var cancellationError = false
    var complete = false
}

@main
struct PhotaraSwiftBridgeSpike {
    static func main() throws {
        guard CommandLine.arguments.count == 2 else {
            throw SpikeError.invalidUsage
        }

        let process = Process()
        let input = Pipe()
        let output = Pipe()
        process.executableURL = URL(fileURLWithPath: CommandLine.arguments[1])
        process.standardInput = input
        process.standardOutput = output
        process.standardError = FileHandle.standardError
        try process.run()

        let commandID = UUID().uuidString.lowercased()
        let requestID = UUID().uuidString.lowercased()
        let evaluationID = UUID().uuidString.lowercased()
        var observations = Observations()
        var cancellationSent = false

        while let line = try readLine(from: output.fileHandleForReading) {
            guard
                let data = line.data(using: .utf8),
                let event = try JSONSerialization.jsonObject(with: data) as? [String: Any],
                let kind = event["event"] as? String,
                let payload = event["payload"] as? [String: Any]
            else {
                throw SpikeError.invalidJSON
            }

            switch kind {
            case "ready":
                guard payload["api_version"] as? Int == 1 else {
                    throw SpikeError.missing("application API version")
                }
                try send([
                    "operation": "start",
                    "command_id": commandID,
                    "request_id": requestID,
                    "evaluation_id": evaluationID,
                ], to: input.fileHandleForWriting)

            case "command-response":
                guard payload["command_id"] as? String == commandID else {
                    throw SpikeError.missing("correlated command identity")
                }
                if payload["status"] as? String == "applied" {
                    observations.applied = payload["snapshot"] is [String: Any]
                } else if payload["status"] as? String == "rejected" {
                    let error = payload["error"] as? [String: Any]
                    observations.rejected = error?["code"] as? String == "revision-conflict"
                }

            case "portable-documents":
                let project = payload["project"] as? [String: Any]
                let graph = payload["node_graph"] as? [String: Any]
                observations.portableProject =
                    project?["project_id"] is String && project?["graph"] is [String: Any]
                observations.portableGraph =
                    graph?["project_id"] == nil && graph?["graph"] is [String: Any]

            case "evaluation-progress":
                guard
                    payload["request_id"] as? String == requestID,
                    payload["evaluation_id"] as? String == evaluationID
                else {
                    throw SpikeError.missing("correlated progress identity")
                }
                observations.progress = true
                let phase = payload["phase"] as? String
                if phase == "evaluating" && !cancellationSent {
                    cancellationSent = true
                    try send([
                        "operation": "cancel",
                        "request_id": requestID,
                        "evaluation_id": evaluationID,
                    ], to: input.fileHandleForWriting)
                }
                if phase == "cancelled" {
                    observations.cancelledProgress = true
                }

            case "evaluation-error":
                observations.cancellationError =
                    payload["code"] as? String == "evaluation-cancelled"

            case "complete":
                observations.complete = payload["cancelled"] as? Bool == true

            default:
                throw SpikeError.missing("known event kind")
            }
        }

        input.fileHandleForWriting.closeFile()
        process.waitUntilExit()
        guard process.terminationStatus == 0 else {
            throw SpikeError.processFailed(process.terminationStatus)
        }

        let required: [(Bool, String)] = [
            (observations.applied, "applied command response"),
            (observations.rejected, "structured revision conflict"),
            (observations.portableProject, "portable project document"),
            (observations.portableGraph, "standalone node graph document"),
            (observations.progress, "evaluation progress"),
            (observations.cancelledProgress, "cancelled progress"),
            (observations.cancellationError, "structured cancellation error"),
            (observations.complete, "completion acknowledgement"),
        ]
        if let missing = required.first(where: { !$0.0 }) {
            throw SpikeError.missing(missing.1)
        }
        print("Swift bridge spike passed on \(ProcessInfo.processInfo.operatingSystemVersionString)")
    }

    static func send(_ object: [String: Any], to handle: FileHandle) throws {
        var data = try JSONSerialization.data(withJSONObject: object, options: [.sortedKeys])
        data.append(0x0a)
        try handle.write(contentsOf: data)
    }

    static func readLine(from handle: FileHandle) throws -> String? {
        var data = Data()
        while true {
            guard let byte = try handle.read(upToCount: 1), !byte.isEmpty else {
                return data.isEmpty ? nil : String(data: data, encoding: .utf8)
            }
            if byte[byte.startIndex] == 0x0a {
                return String(data: data, encoding: .utf8)
            }
            data.append(byte)
        }
    }
}
