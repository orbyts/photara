import Foundation
import Darwin

@main
struct DisposablePipeReadChecks {
    static func main() throws {
        let pipe = Pipe()
        let line = Data("PHOTARA_PS3 {\"id\":\"short-response\"}\n".utf8)
        try pipe.fileHandleForWriting.write(contentsOf: line)
        // Writer deliberately stays open: filling a 64KiB Foundation read would
        // block here, exactly as the interactive Rust child's small response did.
        guard case .bytes(let actual) = readDisposablePipe(pipe.fileHandleForReading.fileDescriptor) else {
            preconditionFailure("expected short response while writer remains open")
        }
        precondition(actual == line)
        let descriptor = pipe.fileHandleForReading.fileDescriptor
        let flags = fcntl(descriptor, F_GETFL)
        precondition(flags >= 0 && fcntl(descriptor, F_SETFL, flags | O_NONBLOCK) == 0)
        guard case .deferred = readDisposablePipe(descriptor) else {
            preconditionFailure("empty live nonblocking pipe must defer, not EOF")
        }
        try pipe.fileHandleForWriting.close()
        guard case .end = readDisposablePipe(descriptor) else { preconditionFailure("expected EOF") }
        guard case .failed(EBADF) = readDisposablePipe(-1) else { preconditionFailure("expected typed read failure") }
        print("PASS: bounded short pipe response with writer open, EAGAIN, EOF, EBADF")
    }
}
