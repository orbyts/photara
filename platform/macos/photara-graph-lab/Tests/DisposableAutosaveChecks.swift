import Foundation

// Projection tests use synthetic DTOs only; they prove no storage durability.
@main
struct DisposableAutosaveChecks {
    static func main() {
        let project = UUID(), incarnation = UUID(), owner = UUID(), attachment = UUID()
        let binding = DisposableAutosaveBinding(projectID: project, incarnationID: incarnation,
            ownerEpoch: owner, attachmentID: attachment, attachmentGeneration: 1,
            principalSHA256: "principal")
        let operation = UUID()
        func coordinate(_ revision: UInt64, _ digest: String, operationID: UUID? = nil) -> DisposableAcceptedCoordinate {
            .init(revision: revision, authoredDigest: digest, coordinateSHA256: digest,
                  mutation: operationID.map { .init(recordID: $0, checksum: "frame") },
                  acceptedFrameSHA256: operationID == nil ? nil : "accepted",
                  operationID: operationID)
        }
        let old = coordinate(1, "old"), new = coordinate(2, "new", operationID: operation)
        func saved(_ target: DisposableAcceptedCoordinate) -> DisposableSavedEvidence {
            .init(target: target, headSHA256: "head", commitSHA256: "commit",
                  checkpointReceipt: .init(recordID: UUID(), checksum: "receipt"))
        }
        func snapshot(_ sequence: UInt64, accepted: DisposableAcceptedCoordinate,
                      savedTarget: DisposableAcceptedCoordinate? = nil,
                      frozen: Bool = false, failure: String? = nil,
                      scope: DisposableAutosaveBinding? = nil) -> DisposableSessionSnapshot {
            .init(binding: scope ?? binding, eventSequence: sequence, accepted: accepted,
                  saved: savedTarget.map(saved), frozen: frozen, closed: false, failure: failure)
        }
        func ack(_ accepted: DisposableAcceptedCoordinate) -> DisposableOperationAcknowledgement {
            .init(binding: binding, operationID: operation, accepted: accepted, operationReceiptSHA256: "original-receipt")
        }
        var model = DisposableAutosaveProjection(binding: binding)
        precondition(model.status == .unavailable && !model.canSubmit)
        precondition(model.receive(snapshot(1, accepted: old, savedTarget: old)))
        precondition(model.status == .saved)
        precondition(model.begin(operation) && model.status == .saving)
        precondition(!model.begin(UUID()))
        precondition(model.receive(snapshot(2, accepted: old, savedTarget: old)))
        precondition(model.status == .saving && model.pendingOperationID == operation)
        precondition(!model.receive(snapshot(3, accepted: old, savedTarget: old), acknowledgement: ack(old)))
        model.submissionFailed(operation, message: "Unknown append result")
        precondition(model.status == .failed("Unknown append result") && !model.canSubmit)
        precondition(model.pendingOperationID == operation)
        let accepted = snapshot(3, accepted: new, savedTarget: old)
        precondition(model.receive(accepted)) // observation can arrive before its request callback
        precondition(model.status == .failed("Unknown append result"))
        precondition(model.receive(accepted, acknowledgement: ack(new)))
        precondition(model.status == .saving && model.pendingOperationID == nil)
        precondition(!model.receive(snapshot(2, accepted: old, savedTarget: old)))
        precondition(model.status == .saving)
        let replaced = DisposableAutosaveBinding(projectID: project, incarnationID: incarnation,
            ownerEpoch: owner, attachmentID: attachment, attachmentGeneration: 2, principalSHA256: "principal")
        precondition(!model.receive(snapshot(4, accepted: old, savedTarget: old, scope: replaced)))
        precondition(model.receive(snapshot(4, accepted: new, savedTarget: coordinate(2, "wrong"))))
        precondition(model.status == .saving)
        precondition(model.receive(snapshot(5, accepted: new, savedTarget: new)))
        precondition(model.status == .saved)
        precondition(model.receive(snapshot(6, accepted: new, savedTarget: new, frozen: true)))
        precondition(model.status == .failed("Session requires recovery") && !model.canSubmit)
        precondition(model.receive(snapshot(7, accepted: new, savedTarget: new, failure: "Journal barrier failed")))
        precondition(model.status == .failed("Journal barrier failed"))
        var dedupe = DisposableAutosaveProjection(binding: binding)
        precondition(dedupe.receive(snapshot(1, accepted: old, savedTarget: old)))
        precondition(dedupe.begin(operation))
        let later = coordinate(3, "later", operationID: UUID())
        let laterSnapshot = snapshot(2, accepted: later, savedTarget: new)
        precondition(dedupe.receive(laterSnapshot))
        precondition(dedupe.receive(laterSnapshot, acknowledgement: ack(new)))
        precondition(dedupe.pendingOperationID == nil && dedupe.snapshot?.accepted == later)
        precondition(dedupe.status == .saving) // original A receipt never marks current B saved
        print("PASS: native autosave projection — pending/stale/wrong-scope/wrong-coordinate/frozen/unknown-retry checks; no storage authority")
    }
}
