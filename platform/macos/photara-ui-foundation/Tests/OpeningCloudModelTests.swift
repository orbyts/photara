import Foundation

@MainActor
private final class SyntheticOpeningDriver: OpeningCloudDriver {
    var availability: OpeningCloudAvailability
    var calls = 0
    var saved: NativeOnboardingState = .localReady
    var localReads = 0, recoveries = 0
    var logouts = 0, picker: NativeLibraryPicker?
    func setLibraryPicker(_ picker: @escaping NativeLibraryPicker) { self.picker = picker }
    func signOut() async throws -> NativeOnboardingState { logouts += 1; saved = .signedOutCached; return saved }
    func accountProfile(refreshAvatar: Bool) async throws -> NativeAccountProfile? {
        NativeAccountProfile(name: "Synthetic Person", email: "person@example.invalid", picture: nil)
    }
    func savedState() async throws -> NativeOnboardingState? { localReads += 1; return saved }
    func recoverIfNeeded(progress: @escaping @MainActor (NativeOnboardingState) -> Void) async throws -> NativeOnboardingState? {
        recoveries += 1; return .reconciling
    }
    var progress: (@MainActor (NativeOnboardingState) -> Void)?
    var pending: CheckedContinuation<NativeOnboardingState, Error>?
    init(availability: OpeningCloudAvailability = .available) { self.availability = availability }
    func signIn(progress: @escaping @MainActor (NativeOnboardingState) -> Void) async throws -> NativeOnboardingState {
        calls += 1
        self.progress = progress
        // Deliberately ignore task cancellation to exercise late completions.
        return try await withCheckedThrowingContinuation { pending = $0 }
    }
    func finish(_ state: NativeOnboardingState) { pending?.resume(returning: state); pending = nil }
    func fail(_ error: Error = NativeAuthenticationError.transportUnavailable) { pending?.resume(throwing: error); pending = nil }
}

@main
struct OpeningCloudModelTests {
    @MainActor static func main() async throws {
        var assertions = 0
        func check(_ value: Bool) { precondition(value, "Synthetic opening invariant failed"); assertions += 1 }
        func settle() async { for _ in 0..<10 { await Task.yield() } }
        for saved in [NativeOnboardingState.localReady, .outcomeUnknown, .freshSignInRequired, .cloudOffline, .signedOutCached] {
            let driver = SyntheticOpeningDriver()
            driver.saved = saved
            let model = OpeningCloudModel(driver: driver)
            check(!model.showsSignIn && !model.isWorking)
            model.loadSavedState(); model.loadSavedState()
            model.cancel() // View disappearance before the passive read completes.
            await settle()
            check(driver.calls == 0 && driver.recoveries == 0 && driver.localReads == 1)
            check(model.state == saved && !model.isWorking && model.generation == 0)
            check(model.showsSignIn == (saved != .cloudOffline))
            if saved == .cloudOffline {
                check(model.hasCloudLibrary && model.message == nil)
                model.signIn(); await settle()
                check(driver.calls == 0)
            } else if saved != .localReady { check(model.signInTitle == "Sign in again") }
        }
        for availability in [OpeningCloudAvailability.signingRequired, .integrationRequired] {
            let driver = SyntheticOpeningDriver(availability: availability)
            let model = OpeningCloudModel(driver: driver)
            model.signIn()
            await settle()
            check(driver.calls == 0 && model.state == .localReady && model.message != nil)
        }
        let driver = SyntheticOpeningDriver(), model = OpeningCloudModel(driver: driver)
        model.signIn()
        await settle()
        check(driver.calls == 1 && model.state == .authenticating)
        model.signIn()
        check(driver.calls == 1)
        model.cancel()
        check(model.state == .localReady)
        driver.progress?(.bootstrapPending)
        driver.finish(.cloudReady)
        await settle()
        check(model.state == .localReady && !model.hasCloudLibrary)
        model.signIn()
        await settle()
        driver.progress?(.bootstrapPending)
        check(model.isWorking && model.message != nil)
        model.cancel()
        check(model.state == .outcomeUnknown)
        driver.finish(.cloudReady)
        await settle()
        check(model.state == .outcomeUnknown)
        model.signIn()
        await settle()
        driver.progress?(.reconciling)
        driver.fail()
        await settle()
        check(model.state == .reconciliationRequired && model.failureMessage == nil && model.message != nil)
        model.signIn()
        await settle()
        driver.finish(.cloudReady)
        await settle()
        check(model.hasCloudLibrary && model.failureMessage == nil)
        await settle()
        check(model.account?.name == "Synthetic Person")
        model.signOut(); model.signOut(); await settle()
        check(driver.logouts == 1 && model.state == .signedOutCached && model.account == nil)
        model.signIn(); await settle()
        let choices = [NativeLibraryChoice(id: "synthetic-library", name: "My Library")]
        let selection = Task { try await driver.picker!(choices) }
        await settle()
        check(model.libraryChoices == choices && model.state == .libraryChoiceRequired && model.isWorking)
        model.chooseLibrary("unavailable-library")
        check(model.libraryChoices == choices)
        model.chooseLibrary("synthetic-library")
        check(try await selection.value == "synthetic-library" && model.libraryChoices.isEmpty)
        driver.finish(.cloudReady); await settle()
        check(model.isSignedIn && model.message == nil)
        model.signOut(); await settle(); model.signIn(); await settle()
        let cancelledSelection = Task { try await driver.picker!(choices) }
        await settle(); model.cancel()
        do { _ = try await cancelledSelection.value; preconditionFailure("Cancelled selection succeeded") }
        catch { check(model.libraryChoices.isEmpty && model.state == .signedOutCached) }
        driver.finish(.cloudReady); await settle()
        check(!model.isSignedIn && model.account == nil)
        for phase in NativeOnboardingPhase.allCases {
            let driver = SyntheticOpeningDriver(), model = OpeningCloudModel(driver: driver)
            model.signIn(); await settle()
            let failure = NativeOnboardingFailure(phase: phase, reason: .failed, submitted: false, receiptRecorded: false)
            driver.fail(failure); await settle()
            check(model.state == .localReady && model.message == failure.message)
        }
        let retainedDriver = SyntheticOpeningDriver(), retainedModel = OpeningCloudModel(driver: retainedDriver)
        retainedModel.signIn(); await settle()
        let retainedFailure = NativeOnboardingFailure(phase: .receipt, reason: .failed, submitted: true, receiptRecorded: false)
        retainedDriver.fail(retainedFailure); await settle()
        check(retainedModel.state == .outcomeUnknown && retainedModel.message == retainedFailure.message)
        print("opening-cloud-model: \(assertions) synthetic presentation assertions passed")
    }
}
