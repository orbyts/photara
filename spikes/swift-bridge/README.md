# Swift bridge spike

This is a disposable command-line interoperability harness, not the Photara
application. It uses Foundation and a newline-delimited JSON child process; it
has no SwiftUI, AppKit, Metal, or macOS 27 SDK dependency.

From the repository root:

```console
cargo build --example swift_bridge_server -p photara-bridge
swift build --package-path spikes/swift-bridge
spikes/swift-bridge/.build/debug/photara-swift-bridge-spike \
  target/debug/examples/swift_bridge_server
```

Swift supplies command, request, and evaluation IDs. The harness verifies a
real applied Core command, a structured revision conflict, portable Project and
standalone Node Graph JSON, correlated progress, and cooperative cancellation.
