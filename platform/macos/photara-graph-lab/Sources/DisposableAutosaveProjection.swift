import Foundation

// Presentation DTOs only. The private Rust host constructs these from the
// private Rust SessionSnapshot; these values never authorize storage or mutation.
struct DisposableAutosaveBinding: Codable, Equatable, Sendable {
    let projectID: UUID
    let incarnationID: UUID
    let ownerEpoch: UUID
    let attachmentID: UUID
    let attachmentGeneration: UInt64
    let principalSHA256: String
    enum CodingKeys: String, CodingKey {
        case projectID = "project_id"
        case incarnationID = "incarnation_id"
        case ownerEpoch = "owner_epoch"
        case attachmentID = "attachment_id"
        case attachmentGeneration = "attachment_generation"
        case principalSHA256 = "principal_sha256"
    }
}

struct DisposableJournalLink: Codable, Equatable, Sendable {
    let recordID: UUID
    let checksum: String
    enum CodingKeys: String, CodingKey {
        case recordID = "record_id"
        case checksum
    }
}

struct DisposableAcceptedCoordinate: Codable, Equatable, Sendable {
    let revision: UInt64
    let authoredDigest: String
    // Hashes of the exact canonical Rust coordinate/frame, not Swift encoding.
    let coordinateSHA256: String
    let mutation: DisposableJournalLink?
    let acceptedFrameSHA256: String?
    let operationID: UUID?
    enum CodingKeys: String, CodingKey {
        case revision
        case authoredDigest = "authored_digest"
        case coordinateSHA256 = "coordinate_sha256"
        case mutation
        case acceptedFrameSHA256 = "accepted_frame_sha256"
        case operationID = "operation_id"
    }
}

struct DisposableSavedEvidence: Codable, Equatable, Sendable {
    let target: DisposableAcceptedCoordinate
    let headSHA256: String
    let commitSHA256: String
    let checkpointReceipt: DisposableJournalLink
    enum CodingKeys: String, CodingKey {
        case target
        case headSHA256 = "head_sha256"
        case commitSHA256 = "commit_sha256"
        case checkpointReceipt = "checkpoint_receipt"
    }
}

struct DisposableSessionSnapshot: Codable, Equatable, Sendable {
    let binding: DisposableAutosaveBinding
    let eventSequence: UInt64
    let accepted: DisposableAcceptedCoordinate
    let saved: DisposableSavedEvidence?
    let frozen: Bool
    let closed: Bool
    let failure: String?
    enum CodingKeys: String, CodingKey {
        case binding
        case eventSequence = "event_sequence"
        case accepted
        case saved
        case frozen, closed
        case failure
    }
}

struct DisposableOperationAcknowledgement: Codable, Equatable, Sendable {
    let binding: DisposableAutosaveBinding
    let operationID: UUID
    let accepted: DisposableAcceptedCoordinate
    let operationReceiptSHA256: String
    enum CodingKeys: String, CodingKey {
        case binding, accepted
        case operationID = "operation_id"
        case operationReceiptSHA256 = "operation_receipt_sha256"
    }
}

enum DisposableAutosaveStatus: Equatable {
    case unavailable
    case saving
    case saved
    case failed(String)

    var label: String {
        switch self {
        case .unavailable: "Session unavailable"
        case .saving: "Saving…"
        case .saved: "Saved"
        case .failed: "Save Failed"
        }
    }
}

/// No timer, dirty flag, successful call, or locally persisted JSON can mint Saved.
/// This reducer is not wired to GraphLab's synthetic graph or to real libraries.
struct DisposableAutosaveProjection {
    let binding: DisposableAutosaveBinding
    private(set) var snapshot: DisposableSessionSnapshot?
    private(set) var pendingOperationID: UUID?
    private(set) var submissionFailure: String?

    var status: DisposableAutosaveStatus {
        if let submissionFailure { return .failed(submissionFailure) }
        guard let snapshot else { return .unavailable }
        if let failure = snapshot.failure { return .failed(failure) }
        if snapshot.frozen { return .failed("Session requires recovery") }
        guard pendingOperationID == nil, snapshot.saved?.target == snapshot.accepted else {
            return .saving
        }
        return .saved
    }

    var canSubmit: Bool {
        guard let snapshot else { return false }
        return pendingOperationID == nil && submissionFailure == nil
            && !snapshot.frozen && !snapshot.closed && snapshot.failure == nil
    }

    /// The operation ID must come from the retained request; never regenerate on retry.
    mutating func begin(_ operationID: UUID) -> Bool {
        guard canSubmit else { return false }
        pendingOperationID = operationID
        return true
    }

    /// Only an acknowledgement for this same binding and exact pending operation
    /// may clear its draft. An old Saved callback cannot acknowledge a new edit.
    @discardableResult
    mutating func receive(_ value: DisposableSessionSnapshot, acknowledgement: DisposableOperationAcknowledgement? = nil) -> Bool {
        guard value.binding == binding else { return false }
        if let snapshot {
            guard value.eventSequence > snapshot.eventSequence
                || (acknowledgement != nil && value == snapshot) else { return false }
        }
        if let acknowledgement {
            guard acknowledgement.binding == binding,
                  acknowledgement.operationID == pendingOperationID,
                  acknowledgement.accepted.operationID == acknowledgement.operationID,
                  acknowledgement.accepted.mutation != nil,
                  acknowledgement.accepted.acceptedFrameSHA256 != nil,
                  !acknowledgement.operationReceiptSHA256.isEmpty else { return false }
        }
        snapshot = value
        if acknowledgement != nil {
            pendingOperationID = nil
            submissionFailure = nil
        }
        return true
    }

    /// An uncertain submission retains its original operation and visible draft.
    /// Retrying must call the trusted session with that same request/ID.
    mutating func submissionFailed(_ operationID: UUID, message: String) {
        guard pendingOperationID == operationID else { return }
        submissionFailure = message
    }
}
