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

The disposable spike is complete and recorded in `SWIFT_BRIDGE_SPIKE.md`. Its
production replacement now uses workspace-pinned UniFFI 0.32 in library mode.
The facade owns application-shaped project sessions and exposes generated Swift
records, enums, objects, and a foreign progress observer. A handwritten C ABI
would add manual ownership and unsafe surface without improving the semantic
contract; IPC/XPC remains appropriate where process isolation is itself
required. JSON remains the portable document format and a useful encoding for
opaque namespaced authored state, not the transport for every fine-grained
production call.

`PhotaraApplication` opens the durable store and creates or reopens
`PhotaraProject` sessions. A session returns immutable project, graph, node,
asset, typed Layout-inspection, proxy-descriptor, diagnostic, and
command-response DTOs. It applies revision-checked Core
commands, saves with project compare-and-swap, and creates one-shot evaluation
handles whose progress is delivered through a Swift-implemented observer.
Cancellation is an explicit handle method connected to Core's cooperative
token rather than an assumption about foreign async cancellation semantics.

Layout authored-state JSON is opaque to Swift. Rust validates and interprets it
into presentation-oriented canvas, frame, cell, placement, content-mode,
rotation, and authored-digest DTOs. Layout intent is resolved inside Rust.
Accepted Layout edits, undo, and redo
all cross the authoritative graph boundary as Core `SetAuthoredState` commands;
Swift never edits persisted JSON directly. Workspace placement, visibility,
selection, and restoration remain separate Swift-owned state and invoke no
semantic bridge method.

Local import separates runtime I/O from semantic publication. The adapter
copies and fingerprints selected files into project resources, then a
project-level Core command validates and adds the portable asset/resource
values. Explicit Layout binding is an atomic graph command. Native clients do
not mutate Project Asset Context or graph documents directly.

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

The recognizable native product distinguishes four concepts:

1. Graph contains nodes and explicit typed data flow.
2. A standard Inspector explains every selected node through identity, typed
   ports, parameters, output summaries, evaluation state, and diagnostics.
3. A node may optionally advertise a rich Workspace; Layout is the first.
4. Project panels such as Assets and Diagnostics expose shared project context
   independently of any node.

Every exact node-definition version may advertise presentation capabilities in
addition to its typed semantic contract. The generic Inspector shell always
remains available; a definition may augment it with a node-specific control
surface for semantic parameters and commands. A rich canvas Workspace is a
separate, optional capability rather than something every node must implement.
Layout needs one for visual frame and crop authoring, while a future automation
node may need only menus, script choices, and status controls. Platform control
implementations are replaceable skins keyed by versioned definition identity;
they cannot become authoritative node state or bypass Core commands. Package
purchase, download, and installation are future distribution concerns and do
not change this boundary.

The bridge supplies generic immutable port inspection records with direction,
value-type identity/version, connected-node identity, and node/runtime-produced
summary fields. Swift renders the standard shell without assuming photography
or Layout. A built-in may add parameter controls inside that shell, but opaque
authored state remains interpreted by Rust. Workspace availability and icon
hints are UI-facing definition metadata; their current built-in mapping is the
small incremental contract, not a new Core node kind.

The compiled shell uses three resizable regions and independently identified
Assets, Graph, Layout Workspace, Inspector, and Diagnostics surfaces. Panels can
move or hide, and the native Workspace menu restores the default without
altering project semantics. Graph is now a deliberately primitive spatial
canvas with dots, node cards, typed port indicators, connections, selection,
pan, and zoom. Single-click drives the standard Inspector; double-clicking a
node that advertises a Workspace activates it. Graph owns the node-creation
interaction: `Tab` (and the equivalent add control) opens a definition menu in
the conventional node-editor style. Layout is currently the only offered
definition, but the menu and shell are not modeled as Layout-specific. Final
graph editing and visual language remain deferred.

The visual Layout surface and conventional Inspector are distinct consumers of
the same immutable Rust DTOs. Rust resolves explicit `AssetSet` input into
normalized and pixel cell geometry; Swift composes leased proxy files into that
geometry. Fit/Fill/Crop, focal/alignment, rotation, structure, and assignment
cross the facade only as semantic commands. Crop dragging keeps an ephemeral
Swift translation while the pointer is down and commits one crop command when
the gesture ends.

With no active project, the client presents Create, Open, and Recent instead of
silently manufacturing or reopening a session. Recent locations are native
`UserDefaults` state. Opening a portable project document validates it and
imports it into the filesystem repository while refusing divergent content
with an existing semantic identity. This launcher adds no database dependency:
portable project JSON remains independently openable, and database-backed
capabilities wait for a concrete synchronization, provider, publication,
collaboration, or evidence requirement.

## Future Windows

Windows uses a separate native UI implementation over the same semantic
inspector descriptors, commands, DTOs, progress, and cancellation contracts.
Portable Core and node packages must avoid macOS-only dependencies. Platform
clients may differ visually and structurally while preserving behavior. Each
platform supplies the smallest native presentation and interaction layer
practical; project lifecycle, authored-state interpretation, semantic commands,
asset binding, proxy policy, diagnostics, and immutable presentation models
remain reusable Rust services.
