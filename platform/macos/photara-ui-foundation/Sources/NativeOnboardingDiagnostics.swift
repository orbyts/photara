import Foundation

/// Only these fixed phrases cross the presentation boundary. Underlying errors,
/// HTTP bodies, codes, callback URLs and provider descriptions are never retained.
enum NativeOnboardingPhase: String, CaseIterable {
    case signing, service, capabilities, journalRecovery, preparation, challenge
    case browser, callback, tokenExchange, credentialStorage, dispatch, bootstrap
    case receipt, session, reconciliation, cleanup, sessionRefresh, operationLookup, operationReplay

    var title: String {
        switch self {
        case .signing: "Signing check"
        case .service: "Local service startup"
        case .capabilities: "Service compatibility check"
        case .journalRecovery: "Previous sign-in recovery"
        case .preparation: "Local library preparation"
        case .challenge: "Sign-in challenge"
        case .browser: "Browser sign-in"
        case .callback: "Sign-in callback validation"
        case .tokenExchange: "Google token verification"
        case .credentialStorage: "Keychain storage"
        case .dispatch: "Local submission journal"
        case .bootstrap: "Cloud library creation"
        case .receipt: "Cloud receipt validation"
        case .session: "Cloud access check"
        case .reconciliation: "Local library reconciliation"
        case .cleanup: "Sign-in cleanup"
        case .sessionRefresh: "Retained session recovery"
        case .operationLookup: "Cloud operation lookup"
        case .operationReplay: "Cloud operation recovery"
        }
    }
}

struct NativeOnboardingFailure: Error, Equatable {
    enum Reason: Equatable { case failed, cancelled, retainedOperation, busy, proofRequired, differentAccount }
    let phase: NativeOnboardingPhase
    let reason: Reason
    let submitted: Bool
    let receiptRecorded: Bool
    var httpStatus: Int? = nil
    var serviceCode: NativeServiceFailureCode? = nil
    var stateOverride: NativeOnboardingState? = nil

    var state: NativeOnboardingState {
        if let stateOverride { return stateOverride }
        if receiptRecorded { return .reconciliationRequired }
        if submitted || reason == .retainedOperation { return .outcomeUnknown }
        return .localReady
    }

    var message: String {
        if reason == .differentAccount { return "Sign in with the Google account that owns this library." }
        if reason == .busy { return "Another sign-in is finishing. Try again shortly." }
        if reason == .retainedOperation { return "A previous cloud result needs recovery. Your local library is safe." }
        if reason == .proofRequired { return "The cloud operation has no receipt and its original sign-in proof is unavailable or expired. The original operation and local library are retained." }
        if reason == .cancelled && !submitted { return "Sign-in cancelled. Your library is unchanged." }
        let suffix = submitted ? " Connection evidence is retained." : " You can retry."
        let diagnostic = httpStatus.map { " (HTTP \($0)" + (serviceCode.map { "; " + $0.rawValue } ?? "") + ")" } ?? ""
        return phase.title + " failed" + diagnostic + "." + suffix
    }

    static func response(phase: NativeOnboardingPhase, data: Data, status: Int, receiptRecorded: Bool = false) -> Self {
        var failure = Self(phase: phase, reason: .failed, submitted: true, receiptRecorded: receiptRecorded)
        failure.httpStatus = (400...599).contains(status) ? status : nil
        failure.serviceCode = NativeServiceFailureCode.decode(data)
        return failure
    }
}

enum NativeServiceFailureCode: String, Equatable {
    case authenticationRequired = "authentication-required", accessDenied = "access-denied"
    case unavailable, throttled, conflict, integrity
    case invalidCommand = "invalid-command", incompatible = "protocol-incompatible"
    case operationNotFound = "operation-not-yet-found"
    static func decode(_ data: Data) -> Self? {
        guard (try? AuthenticationJSON.validate(data, maximum: 65_536)) != nil,
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              object["schema"] as? String == "photara.onboarding.v1",
              let code = object["code"] as? String else { return nil }
        return Self(rawValue: code)
    }
}
