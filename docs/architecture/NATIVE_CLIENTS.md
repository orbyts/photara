# Native clients

## macOS 0.2.0

The first product client is macOS-only and uses SwiftUI/AppKit. AppKit/Metal may
provide graph, crop, drag/drop, color-managed proxy, HDR/EDR, or performance-
sensitive surfaces. Swift owns presentation, platform interaction, accessibility,
menus, focus, and native document behavior.

Swift calls a narrow versioned Rust application facade. The facade exposes
immutable DTOs, semantic command/request IDs, expected revisions, structured
diagnostics, progress streams, cancellation, and stable recovery actions. It
does not expose repositories, SQL, executor objects, or the Rust object graph.

The bridge spike compares UniFFI, a small C ABI, and IPC/XPC where isolation is
valuable. Large proxies should cross as verified cache/file references or a
measured low-copy representation rather than repeated JSON/pixel copies.

## Future Windows

Windows uses a separate native UI implementation over the same semantic
inspector descriptors, commands, DTOs, progress, and cancellation contracts.
Portable Core and node packages must avoid macOS-only dependencies. Platform
clients may differ visually and structurally while preserving behavior.
