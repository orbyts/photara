import Foundation
import SwiftUI

private struct PhotaraThemeEnvironmentKey: EnvironmentKey {
    static let defaultValue: PhotaraResolvedTheme? = nil
}

extension EnvironmentValues {
    var photaraTheme: PhotaraResolvedTheme? {
        get { self[PhotaraThemeEnvironmentKey.self] }
        set { self[PhotaraThemeEnvironmentKey.self] = newValue }
    }
}

final class PhotaraThemeStore: NSObject, ObservableObject {
    @Published private(set) var document: PhotaraThemeDocument
    @Published private(set) var loadError: String?

    private let bundledURL: URL
    private var loadedURL: URL
    private var loadedModificationDate: Date?
    private var timer: Timer?
    private var timerTarget: PhotaraThemeTimerTarget?

    override init() {
        guard let bundledURL = Bundle.main.url(
            forResource: "photara-default",
            withExtension: "json",
            subdirectory: "Themes"
        ) else {
            fatalError("Photara.app is missing Themes/photara-default.json")
        }
        do {
            let document = try PhotaraThemeDocument.load(from: bundledURL)
            self.bundledURL = bundledURL
            self.document = document
            loadedURL = bundledURL
            loadedModificationDate = Self.modificationDate(bundledURL)
        } catch {
            fatalError("Photara default theme is invalid: \(error.localizedDescription)")
        }
        super.init()
        refresh()
        let timerTarget = PhotaraThemeTimerTarget(store: self)
        self.timerTarget = timerTarget
        timer = Timer.scheduledTimer(
            timeInterval: 0.5,
            target: timerTarget,
            selector: #selector(PhotaraThemeTimerTarget.fire),
            userInfo: nil,
            repeats: true
        )
    }

    deinit {
        timer?.invalidate()
    }

    func refresh() {
        let candidate = PhotaraThemeDevelopmentSettings.overrideURL ?? bundledURL
        let modificationDate = Self.modificationDate(candidate)
        guard candidate != loadedURL || modificationDate != loadedModificationDate else { return }
        do {
            let next = try PhotaraThemeDocument.load(from: candidate)
            document = next
            loadedURL = candidate
            loadedModificationDate = modificationDate
            loadError = nil
        } catch {
            // Preserve the last valid palette while the author is midway through
            // an edit or while an invalid override is selected.
            loadedURL = candidate
            loadedModificationDate = modificationDate
            loadError = error.localizedDescription
        }
    }

    private static func modificationDate(_ url: URL) -> Date? {
        try? url.resourceValues(forKeys: [.contentModificationDateKey]).contentModificationDate
    }
}

private final class PhotaraThemeTimerTarget: NSObject {
    weak var store: PhotaraThemeStore?

    init(store: PhotaraThemeStore) {
        self.store = store
    }

    @objc func fire() {
        store?.refresh()
    }
}
