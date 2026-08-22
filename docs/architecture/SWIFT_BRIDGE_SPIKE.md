# Swift bridge spike

> Historical gate: the disposable NDJSON harness has now been superseded by
> the production-shaped UniFFI facade and Quasar verification under
> `platform/macos/photara-app`.

## Outcome

The early interoperability gate is complete. A deliberately disposable Swift
command-line harness compiled and ran on Quasar with:

```text
macOS:  26.5.2 (25F84)
Xcode:  26.6 (17F113)
Swift:  6.3.3, arm64-apple-macosx26.0
```

It uses Foundation plus newline-delimited JSON over a child process. It has no
SwiftUI, AppKit, Metal, Xcode 27, or macOS 27 dependency. This transport was
chosen for the spike because it exercised the semantic boundary with no new
Rust dependency, generated binding tool, unsafe code, or production UI target.
It is evidence about facade shape, not the production transport selection.

Swift supplied command, request, and evaluation UUIDs and observed:

- a real revision-checked Add Node command and immutable applied response;
- the same command rejected with a structured `revision-conflict` diagnostic;
- a complete portable Project Document and standalone Node Graph Document;
- request/evaluation-correlated validating, planning, and evaluating progress;
- cooperative cancellation sent by Swift after evaluating progress arrived;
- cancelled progress and a structured `evaluation-cancelled` error carrying the
  same request and evaluation identities.

No Rust graph object, repository, executor, or pixel buffer crossed the boundary.

## Boundary comparison

| Option | Smallest credible surface | Result |
|---|---|---|
| UniFFI | Generated Swift bindings over versioned facade records and async/callback handles | Preferred for the production in-process application facade. It avoids handwritten pointer ownership and gives Swift typed bindings. Add it when the production facade crate begins, not to this disposable harness. |
| Handwritten C ABI | Opaque handles plus UTF-8/byte-buffer request and response functions, explicit allocation/free, callbacks for events | Viable but rejected as the default. It introduces a permanent manual memory/ownership contract and an unsafe Rust edge for little benefit over generated bindings. |
| NDJSON child-process IPC | Two pipes, immutable JSON messages, request IDs, and cancellation messages | Implemented and passed. Excellent for contract testing and debugging, but process lifecycle, streaming backpressure, and serialization overhead are unnecessary for the trusted in-process Core. |
| XPC | Codable messages and service lifecycle across process isolation | Reserve for a boundary that genuinely needs crash, privilege, or untrusted-package isolation. It is macOS-specific and should not define the portable application facade. |

The selected production direction is UniFFI for the narrow in-process Swift ↔
Rust facade, with transport-neutral immutable DTO semantics. Stage 8A adopted
UniFFI 0.32 and verified foreign progress callbacks, explicit cancellation, and
Swift 6 concurrency compilation. IPC/XPC can reuse those semantics at later
isolation boundaries.

## Machine and SDK policy

Quasar remains the reference machine for Rust/Core, portable documents, package
and persistence work, and bridge implementation. The bridge must compile and
run with Xcode 26.6 and must not import or encode macOS 27-specific APIs.

Eclipse is the intentional leading-edge UI machine. macOS 27 and Xcode 27 may be
used there for the production SwiftUI/AppKit shell, Layout Inspector, current UI
design language, and visual behavior testing. SDK-specific presentation code
stays above the facade. This division lets Eclipse explore the newest UI while
Quasar continuously proves that infrastructure did not accidentally acquire a
new-SDK dependency.

The spike package's deployment declaration is a harness build setting, not the
eventual application's minimum supported macOS policy. That policy remains a
separate product decision.

## Reproduction

See `spikes/swift-bridge/README.md`. The harness is disposable and should not be
grown into the native application.
