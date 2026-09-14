import Foundation
import Combine

struct NativeLibraryChoice: Identifiable, Equatable, Sendable {
    let id: String
    let name: String
}
typealias NativeLibraryPicker = @MainActor ([NativeLibraryChoice]) async throws -> String

enum OpeningCloudAvailability: Equatable {
    case available
    case signingRequired
    case integrationRequired

    var message: String? {
        switch self {
        case .available: nil
        case .signingRequired: "Cloud sign-in requires a signed development build. You can keep working locally."
        case .integrationRequired: "Cloud sign-in is not yet available in this development build. You can keep working locally."
        }
    }
}

/// The production driver will own native auth plus the Rust journal/bridge.
/// Synthetic drivers exercise presentation without granting local/cloud authority.
@MainActor
protocol OpeningCloudDriver {
    var availability: OpeningCloudAvailability { get }
    func signIn(progress: @escaping @MainActor (NativeOnboardingState) -> Void) async throws -> NativeOnboardingState
    func recoverIfNeeded(progress: @escaping @MainActor (NativeOnboardingState) -> Void) async throws -> NativeOnboardingState?
    func savedState() async throws -> NativeOnboardingState?
    func signOut() async throws -> NativeOnboardingState
    func accountProfile(refreshAvatar: Bool) async throws -> NativeAccountProfile?
    func setLibraryPicker(_ picker: @escaping NativeLibraryPicker)
}
extension OpeningCloudDriver {
    func recoverIfNeeded(progress: @escaping @MainActor (NativeOnboardingState) -> Void) async throws -> NativeOnboardingState? { nil }
    func savedState() async throws -> NativeOnboardingState? { nil }
    func signOut() async throws -> NativeOnboardingState { throw NativeAuthenticationError.signingRequired }
    func accountProfile(refreshAvatar: Bool) async throws -> NativeAccountProfile? { nil }
    func setLibraryPicker(_ picker: @escaping NativeLibraryPicker) { }
}

@MainActor
struct UnavailableDevelopmentCloudDriver: OpeningCloudDriver {
    var availability: OpeningCloudAvailability {
        // Inspect only public signing metadata. Do not read Keychain on local
        // launch, and do not enable cloud until durable reconciliation is wired.
        if (try? NativeCredentialStore.declaredAccessGroup(bundleIdentifier: ReleaseConfiguration.current.identity.bundleIdentifier)) == nil {
            return .signingRequired
        }
        return .integrationRequired
    }

    func signIn(progress: @escaping @MainActor (NativeOnboardingState) -> Void) async throws -> NativeOnboardingState {
        throw NativeAuthenticationError.signingRequired
    }
}

@MainActor
final class OpeningCloudModel: ObservableObject {
    @Published private(set) var state: NativeOnboardingState = .localReady
    @Published private(set) var failureMessage: String?
    @Published private(set) var account: NativeAccountProfile?
    @Published private(set) var libraryChoices: [NativeLibraryChoice] = []
    private var librarySelection: CheckedContinuation<String, Error>?
    private var entryState: NativeOnboardingState = .localReady
    private(set) var generation: UInt64 = 0
    let availability: OpeningCloudAvailability
    private let driver: any OpeningCloudDriver
    @Published private var work: Task<Void, Never>?
    @Published private(set) var loadedSavedState = false
    private var loadingSavedState = false

    init(driver: any OpeningCloudDriver = UnavailableDevelopmentCloudDriver()) {
        self.driver = driver
        availability = driver.availability
        driver.setLibraryPicker { [weak self] choices in
            guard let self else { throw NativeAuthenticationError.cancelled }
            return try await self.requestLibraryChoice(choices)
        }
    }

    var isWorking: Bool { work != nil || [.authenticating, .bootstrapPending, .reconciling].contains(state) }
    var hasCloudLibrary: Bool { [.cloudReady, .cloudOffline, .signedOutCached, .authRequired].contains(state) }
    var isSignedIn: Bool { [.cloudReady, .cloudOffline].contains(state) }
    var showsSignIn: Bool { loadedSavedState && !isSignedIn }
    var signInTitle: String { [.freshSignInRequired, .outcomeUnknown, .authRequired, .signedOutCached].contains(state) ? "Sign in again" : "Sign in with Google" }
    var message: String? {
        if let unavailable = availability.message { return unavailable }
        if let failureMessage { return failureMessage }
        switch state {
        case .freshSignInRequired: return "Sign-in expired. Your library is safe."
        case .authenticating: return "Complete sign-in in your browser."
        case .bootstrapPending: return "Connecting your library to the cloud…"
        case .reconciling: return "Saving your cloud library connection…"
        case .outcomeUnknown: return "Previous sign-in needs a check."
        case .reconciliationRequired: return "Your cloud connection needs to be reconciled with local changes."
        case .libraryChoiceRequired: return "Choose a library to continue."
        case .cloudOffline: return nil
        case .authRequired, .signedOutCached: return "Sign in again to reconnect this cloud library."
        case .accessDisabled: return "Cloud access is unavailable. Your local work is retained."
        case .protocolIncompatible: return "Update the app before reconnecting to the cloud."
        case .localContentEnrollmentRequired: return "This library has local content. Cloud enrollment for existing content is not yet available."
        default: return nil
        }
    }

    func signIn() {
        begin()
    }

    func signOut() {
        guard availability == .available, !isWorking, isSignedIn else { return }
        generation += 1
        let expected = generation
        failureMessage = nil
        work = Task { [weak self] in
            guard let self else { return }
            do {
                let result = try await driver.signOut()
                guard generation == expected else { return }
                state = result
            }
            catch {
                guard generation == expected else { return }
                if let failure = error as? NativeOnboardingFailure {
                    state = failure.state; failureMessage = failure.message
                } else { failureMessage = "Sign out could not be completed. Please retry." }
            }
            guard generation == expected else { return }
            if !isSignedIn { account = nil }
            work = nil
        }
    }

    private func requestLibraryChoice(_ choices: [NativeLibraryChoice]) async throws -> String {
        try Task.checkCancellation()
        guard !choices.isEmpty, librarySelection == nil else { throw NativeAuthenticationError.invalidResponse }
        let expected = generation
        return try await withTaskCancellationHandler {
            try await withCheckedThrowingContinuation { continuation in
                libraryChoices = choices
                state = .libraryChoiceRequired
                librarySelection = continuation
            }
        } onCancel: {
            Task { @MainActor [weak self] in
                guard let self, self.generation == expected else { return }
                self.dismissLibraryPicker()
            }
        }
    }

    func chooseLibrary(_ id: String) {
        guard libraryChoices.contains(where: { $0.id == id }), let selection = librarySelection else { return }
        librarySelection = nil
        libraryChoices = []
        state = .reconciling
        selection.resume(returning: id)
    }

    private func dismissLibraryPicker() {
        let selection = librarySelection
        librarySelection = nil
        libraryChoices = []
        selection?.resume(throwing: NativeAuthenticationError.cancelled)
    }

    private func loadAccount(refreshAvatar: Bool, generation expected: UInt64) {
        Task { [weak self] in
            guard let self else { return }
            let profile = try? await driver.accountProfile(refreshAvatar: refreshAvatar)
            guard generation == expected, isSignedIn else { return }
            account = profile
        }
    }

    func loadSavedState() {
        guard !loadingSavedState else { return }
        loadingSavedState = true
        let expected = generation
        Task { [weak self] in
            guard let self else { return }
            // Local metadata only. Never create a busy/authenticating launch.
            let saved = try? await driver.savedState()
            if generation == expected, let saved { state = saved }
            loadedSavedState = true
            if isSignedIn { loadAccount(refreshAvatar: false, generation: expected) }
        }
    }

    private func begin() {
        guard availability == .available, !isWorking, !isSignedIn else { return }
        generation += 1
        let expected = generation
        failureMessage = nil
        entryState = state
        state = .authenticating
        work = Task { [weak self] in
            guard let self else { return }
            do {
                let progress: @MainActor (NativeOnboardingState) -> Void = { [weak self] progress in
                    guard let self, self.generation == expected else { return }
                    self.state = progress
                }
                let result = try await driver.signIn(progress: progress)
                guard generation == expected else { return }
                state = result
                if isSignedIn { loadAccount(refreshAvatar: true, generation: expected) }
            } catch {
                guard generation == expected else { return }
                if let failure = error as? NativeOnboardingFailure {
                    state = failure.state
                    failureMessage = failure.message
                } else if state == .bootstrapPending { state = .outcomeUnknown }
                else if state == .reconciling { state = .reconciliationRequired }
                else {
                    state = .localReady
                    failureMessage = "Cloud sign-in could not be completed. You can retry or keep working locally."
                }
            }
            if generation == expected { work = nil }
        }
    }

    func cancel() {
        // Disappearing during a passive local read must not discard that result.
        guard isWorking else { return }
        dismissLibraryPicker()
        generation += 1
        work?.cancel()
        work = nil
        if entryState == .signedOutCached || entryState == .authRequired { state = .signedOutCached }
        else if state == .libraryChoiceRequired { state = .outcomeUnknown }
        else if state == .bootstrapPending { state = .outcomeUnknown }
        else if state == .reconciling { state = .reconciliationRequired }
        else if isWorking || state == .libraryChoiceRequired { state = entryState }
    }
}
