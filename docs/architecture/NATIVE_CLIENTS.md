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

The disposable spike is complete and recorded in `SWIFT_BRIDGE_SPIKE.md`. It
ran on Quasar with macOS 26.5.2, Xcode 26.6, and Swift 6.3.3. The production
in-process facade should use UniFFI unless implementation measurements uncover
a concrete blocker. A handwritten C ABI would add manual ownership and unsafe
surface without improving the semantic contract; IPC/XPC remains appropriate
where process isolation is itself required. JSON remains the portable document
format and a useful diagnostic/test encoding, not a requirement that every
fine-grained production call serialize through JSON.

The bridge and Core must not depend on macOS 27 APIs. Quasar is the reference
machine for bridge and Rust/Core development using its stable Xcode 26.6
toolchain. Eclipse may run macOS 27 and Xcode 27 for the later native shell,
Layout Inspector, new SwiftUI behavior, and visual design validation. UI SDK
experiments on Eclipse must remain above the versioned facade and cannot leak
SDK-specific types into Core DTOs or portable documents.

The production client uses independently identified dockable panels/surfaces.
Panel identity is separate from placement: `AssetGallery` does not mean left
sidebar, and `Inspector` does not mean right pane. A default Layout Authoring
workspace may initially use an Assets/Workspace/Inspector three-region preset,
while leaving room for resizing, rearrangement, splits, tabs, visibility,
floating windows, multiple displays, named presets, and restoration.

Workspace presentation state is client-owned and does not dirty a graph. Node
selection and semantic edits still cross the facade as identities and Core
commands, so the real Layout Inspector works wherever its panel is placed. The
first native milestone implements only the docking/restoration depth required
for daily Layout authoring and keeps the graph rendering intentionally plain.

## Future Windows

Windows uses a separate native UI implementation over the same semantic
inspector descriptors, commands, DTOs, progress, and cancellation contracts.
Portable Core and node packages must avoid macOS-only dependencies. Platform
clients may differ visually and structurally while preserving behavior.
