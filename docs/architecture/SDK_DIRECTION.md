# Internal SDK and properties-panel direction

## Stage the SDK

Do not publish a third-party SDK before built-in Disk, Layout, and Photoshop
nodes prove the contract. Build an internal registration API first, keep its
Rust modules private or explicitly unstable, and record every place a built-in
node needs an escape hatch. External stabilization should follow real plugin
experiments.

Built-in nodes should nevertheless use the same conceptual definition,
validation, execution, diagnostics, and property contracts expected of future
plugins. Avoid a privileged GUI-only Layout implementation whose rules cannot
be invoked from Core.

## Node definition metadata

Minimum internal definition:

- namespaced type ID and behavior version;
- display name, category, description, icon semantic name;
- typed input/output ports and cardinality;
- configuration and authored-state schema references;
- execution environment and requirements;
- cache policy and determinism declaration;
- capability IDs;
- validation/planning/execution implementation;
- inspector descriptor;
- migration functions from supported earlier config/state versions.

Status is not plugin-authored arbitrary prose. Nodes emit structured progress,
diagnostics, and actions; the application derives presentation.

## Declarative inspector schema

The schema describes semantics rather than SwiftUI/AppKit controls:

```rust
enum PropertyField {
    Enum { path, label, choices, allow_custom },
    Toggle { path, label },
    Text { path, label, constraints, secret: bool },
    Number { path, label, unit, min, max, step },
    Path { path, label, purpose, access },
    Account { path, label, provider_capability },
    Device { path, label, host_capability },
    Status { source, label },
    Action { action_id, label, safety },
    AssetGrid { source_port, selection_binding },
    ImagePreview { value_binding, overlays },
    Group { label, children, visibility },
}
```

The native client decides typography, spacing, color, focus, accessibility,
platform menus, and control implementation. A Windows client renders native
Windows controls from the same semantics.

Fields bind to typed config/authored-state paths and issue Core commands. They
do not directly mutate plugin memory. Validation returns field-addressable
diagnostics. Actions declare whether they are pure, reversible, confirmable,
or externally mutating.

## Standard versus rich inspectors

### Standard declarative inspector

Suitable for Disk and Photoshop initially:

- project picker/query and proxy policy;
- host detected/version/plugin status;
- execution target;
- output location/reuse policy;
- install, update, recheck, and run actions;
- structured status and diagnostics.

### Rich custom authoring inspector

Layout requires a Core-provided authoring session plus a native renderer:

- asset browser with proxies;
- editorial item list and drag reorder;
- template gallery;
- slot assignment by drag/drop;
- canvas compositor and safe-area overlay;
- crop/rotate interaction;
- fit/contain/crop controls;
- item/placement validation and preview.

The custom inspector participates in the same selection, docking, undo/redo,
focus, shortcuts, validation, status, and property lifecycle as standard
inspectors. It identifies itself through a capability such as
`photara.inspector.layout-authoring/v1`; it does not hand SwiftUI source to the
SDK.

Core owns layout commands and geometry. The native view supplies pointer and
keyboard gestures and sends semantic commands (`SetPlacementCrop`,
`AssignAssetToSlot`, `ReorderItem`). Core returns a new revision and derived
preview geometry.

## Host and execution capabilities

Definitions declare requirements such as:

```text
photara.host.photoshop
photara.host.photoshop.materialize-layout/v1
photara.rendition.paired-hdr-sdr
photara.proxy.thumbnail
photara.proxy.authoring
```

Capability negotiation produces a structured readiness report. This is the
foundation for installer/onboarding “green lights” without embedding installer
logic in every node.

## Security and isolation direction

Do not load arbitrary third-party Rust dynamic libraries into the GUI process
as the first plugin model. Future options include WebAssembly components or an
out-of-process JSON/RPC/component bridge with capability-scoped services. The
choice affects filesystem, credential, network, cancellation, crash, and
version isolation and remains open.

Regardless of runtime, plugins should receive capabilities rather than global
paths or raw credentials. Host/API execution must be declared and visible to
the user. Secret values use account references and credential services, never
serialized node config.

## Candidate crate boundaries

Crates are an end state after module seams are tested, not the first patch.

### Phase 1: modules inside the current crate

```text
domain/
  asset.rs
  representation.rs
  layout/{model,profile,template,transform}.rs
application/
  project_assets.rs
  layout_edit.rs
  layout_resolve.rs
graph/
  model.rs
  value.rs
  evaluate.rs
  cache.rs
host/
  photoshop/{protocol,adapter}.rs
infrastructure/
  storexa/
  filesystem/
  providers/
```

This exposes dependency mistakes without creating a workspace migration.

### Later workspace candidates

| Crate | Responsibility and API | Consumers | Dependencies |
| --- | --- | --- | --- |
| `photara-core` | Domain IDs/values, layout/profile/template model, application commands, graph contracts, diagnostics | CLI, GUI bridge, tests, built-in nodes | serde/uuid and small pure libraries; no SQL, HTTP, Photoshop, Swift |
| `photara-store` | Storexa repositories, migrations, SQL mapping, transactional unit-of-work | application bootstrap | `photara-core`, Storexa/sqlx |
| `photara-host-protocol` | Versioned host requests/receipts, host capability/status model | Core adapters, Photoshop plugin, CLI diagnostics | `photara-core`; no host SDK |
| `photara-builtins` | Disk, Layout, Photoshop node definitions/executors | CLI, GUI runtime | Core plus repository/host service traits |
| `photara-cli` | Clap parsing and presentation only | executable | application facade, built-ins |
| `photara-bridge` | Stable local IPC/FFI boundary for native clients | macOS/Windows client | application facade; transport-specific |
| `photara-sdk` | Eventually stable external node/value/property contracts | third-party node developers | smallest stable subset of Core/protocol |

Do not create one crate per provider or node. Adobe, Cloudinary, Lightroom, and
Photoshop adapters can remain modules until independent release/versioning or
dependency isolation creates a concrete need.

The future Swift client should not link directly to every internal Rust type.
Choose a versioned bridge: generated FFI (for example UniFFI), local IPC, or a
hybrid. That decision requires a prototype measuring cancellation, streaming
progress, large preview transfer, crash isolation, signing, and update behavior.

## What third-party nodes should eventually depend on

Only stable semantic IDs/values, node definition contracts, diagnostics,
property schema, capability APIs, and execution context. They should not depend
on Storexa, Photara SQL tables, current CLI structs, project filesystem layout,
SwiftUI, or Photoshop SDK objects.

## SDK stabilization gates

1. All three built-in nodes use the internal contract.
2. Standard and rich inspectors use one property/command lifecycle.
3. Cache invalidation is proven with Red Meridian and Sylvan.
4. One experimental out-of-tree node can register without private imports.
5. Config/state migrations and unknown-field behavior are tested.
6. Capability security and plugin crash isolation have a chosen design.
7. The CLI can inspect/validate/evaluate the same node instances as the GUI.
