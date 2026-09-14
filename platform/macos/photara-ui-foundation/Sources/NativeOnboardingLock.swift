import Foundation
import Darwin

/// Held until asynchronous cleanup completes. A process exit releases flock,
/// allowing the next attempt to retire an abandoned, unsubmitted journal.
final class NativeOnboardingLock {
    private let descriptor: Int32
    init(url: URL) throws {
        let fd = open(url.path, O_RDWR | O_CREAT | O_NOFOLLOW | O_CLOEXEC, 0o600)
        guard fd >= 0 else { throw NativeAuthenticationError.keychainUnavailable }
        var info = stat()
        guard fstat(fd, &info) == 0, info.st_uid == getuid(),
              (info.st_mode & S_IFMT) == S_IFREG, (info.st_mode & 0o077) == 0,
              info.st_nlink == 1 else {
            close(fd)
            throw NativeAuthenticationError.keychainUnavailable
        }
        guard flock(fd, LOCK_EX | LOCK_NB) == 0 else {
            close(fd)
            throw NativeOnboardingFailure(phase: .journalRecovery, reason: .busy, submitted: false, receiptRecorded: false)
        }
        descriptor = fd
    }
    deinit { flock(descriptor, LOCK_UN); close(descriptor) }
}
