import Foundation

// Disposable CLI host: no AppModel startup, Keychain, service or UI application.
@main
struct BrandReadiness {
    static func main() throws {
        let args = CommandLine.arguments
        precondition(args.count == 3)
        let root = URL(fileURLWithPath: args[2])
        precondition(root.path.hasPrefix("/private/tmp/"))
        let identity = ReleaseConfiguration.current.identity
        let support = identity.supportURL(base: root.appending(path: "support"))
        let cache = identity.cacheURL(base: root.appending(path: "cache"))
        let journalRoot = identity.journalURL(support: support)
        precondition(support.path.hasPrefix(root.path + "/"))
        precondition(cache.path.hasPrefix(root.path + "/"))
        precondition(journalRoot.path.hasPrefix(support.path + "/"))
        precondition(identity.defaultProjectsURL(home: root).path.hasPrefix(root.path + "/Pictures/"))
        precondition(identity.acceptsProjectPackage(URL(fileURLWithPath: "Legacy.photara")))
        precondition(!identity.acceptsProjectPackage(URL(fileURLWithPath: "Wrong.zip")))
        precondition(ReleaseConfiguration.current.environment.callbackURL.scheme == identity.callbackScheme)
        let legacy = args[1].hasSuffix("legacy")
        let suffix = legacy ? "photara" : identity.projectPackageExtension
        let pending = args[1].contains("pending")
        let destination = root.appending(path: pending ? "pending" : (legacy ? "legacy" : "current"))
        try FileManager.default.createDirectory(at: destination, withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: journalRoot, withIntermediateDirectories: true)
        let db = journalRoot.appending(path: "fixture.sqlite")
        // Reopening uses today's configuration even when the persisted operation
        // was created with an older extension. Its stored policy wins on replay.
        let journal = try PhotaraProjectCreation.open(path: db.path,
            packageExtension: args[1].hasPrefix("create") ? suffix : identity.projectPackageExtension)
        defer { try? journal.close() }
        let receipt = root.appending(path: pending ? "pending.json" : (legacy ? "legacy.json" : "current.json"))
        if args[1].hasPrefix("create") {
            let operation = UUID().uuidString.lowercased()
            _ = try journal.prepare(operationId: operation, title: "Synthetic Project",
                destination: destination.path, destinationPin: nil,
                createdAt: "2026-09-16T12:00:00.000Z")
            if pending {
                try JSONSerialization.data(withJSONObject: ["operation": operation]).write(to: receipt)
                print("brand readiness legacy pending intent: prepared")
                return
            }
            let saved = try journal.advance(operationId: operation)
            precondition(saved.state == "complete")
            precondition(URL(fileURLWithPath: saved.packagePath).pathExtension == suffix)
            precondition(identity.acceptsProjectPackage(URL(fileURLWithPath: saved.packagePath)))
            let data = try JSONSerialization.data(withJSONObject: [
                "operation": operation, "path": saved.packagePath,
                "project": saved.projectId, "graph": saved.graphId, "digest": saved.commitSha256], options: [.sortedKeys])
            try data.write(to: receipt, options: .atomic)
        } else {
            let record = try JSONSerialization.jsonObject(with: Data(contentsOf: receipt)) as! [String: String]
            if pending {
                do {
                    _ = try journal.advance(operationId: record["operation"]!)
                    preconditionFailure("Prior extension policy must not publish after cutover")
                } catch {
                    precondition(!FileManager.default.fileExists(atPath: destination.appending(path: "Synthetic Project.photara").path))
                    print("brand readiness prior-policy incomplete publication: safely refused")
                    return
                }
            }
            let saved = try journal.advance(operationId: record["operation"]!)
            precondition(saved.packagePath == record["path"] && saved.projectId == record["project"])
            precondition(saved.graphId == record["graph"] && saved.commitSha256 == record["digest"])
            precondition(saved.state == "complete")
            precondition(identity.acceptsProjectPackage(URL(fileURLWithPath: saved.packagePath)))
        }
        print("brand readiness \(args[1]): passed (initial durable save; package editing remains read-only)")
    }
}
