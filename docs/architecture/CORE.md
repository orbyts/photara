# Portable Core

## Current exact review packet — 2026-09-12

The [accepted D19 freeze](D19_CONTRACT_FREEZE.md) and [CXT1 slices](D19_STATIC_SCHEMA_DELTA.md#fixture-delta-implementation-slices-and-gates) define the accepted pure types/access/schema/AST/cache boundary. New semantics use explicit v2 entry points; existing APIs and keys stay intact. R1–R8 were accepted as proposed 2026-09-12; CXT2 inert DDL is complete. [CXT1a pure contracts](CXT1A_CONTRACTS.md) are implemented in `photara_core::contracts`. [CXT1b context contracts](CXT1B_CONTEXT_CONTRACTS.md) now add `photara_core::context`: explicit field admission, bounded parser/typed AST/interpreter, variables, immutable captures, metadata queries, cache v2 and proposal planning. Existing evaluator APIs and keys are unchanged; CXT3 adapters remain gated.

## D19 domain and evaluation amendment — not current runtime

[D19](LIBRARY_AND_NODE_WORK_SURFACES.md) replaces the persistent Library domain
with Library and requires exactly one owning Library per Project plus explicit
Project-access policy/grants. Core models typed AssetSet graph inputs and the
private identity/provenance/artifact ledger separately; it exposes no ambient
project Gallery, asset union or `$project.assets`. Expressions address declared
ports through `$input.<port>` and frozen Library values through `$library.*`.
Authoring permissions do not expand runtime reads. Capture exact typed IDs,
revisions/projections and transitive dependencies before evaluation.

Source/read, metadata enrichment and effects use explicit value families and
contracts. MetadataPatch is a typed value/proposal; only a declared authorized
effect applies it to files, catalogs or services and emits receipts/artifacts.
Reference/query nodes are optional; host pickers can author typed refs. Library
management is app-owned, while Layout/Gallery/metadata node Work Surfaces compose
shared host components. Category hierarchy, provider tags and built-in status
remain discovery/distribution metadata, never evaluator branches. Freeze the
logical/package/NodeSDK contracts and static schema delta before revised CXT1;
no current APIs, source identifiers, migrations or key formats change here.

Graph owns Nodes, connections, Graph Canvas identity and the Inspector selection/
lifecycle contract. Every exact Node definition supplies a required typed Inspector
description; native Photara renders it in the Graph-owned surface. Rich Work Surface
contribution is optional and does not change semantic ownership.

## Pending D18 extension, revised under D19

[D18](TYPED_CONTEXT_AND_EXPRESSIONS.md) builds on existing TypedValue/SchemaValue,
AssetSet and environment-key contracts. Add typed variable/schema validation,
versioned bounded ASTs, dependency resolution, frozen ContextSnapshots and
proposal commands through an additive context-aware evaluation entry point.
The current ValueTypeRegistry compares exact descriptors; it is not a complete
payload schema validator. Preserve old APIs/keys; new required context contracts
need explicit key/version separation. Nodes receive only declared snapshots,
never a shared mutable dictionary, filesystem path, SQL or raw secret. Account
preferences/host grants stay outside semantic Core state. Pure evaluation cannot
apply variable proposals. CXT1 requires its own scope approval; no Rust changed.

Core is a UI-, provider-, host-, storage-backend-, and media-kind-independent
Rust library.

It owns:

- stable semantic identity and versioning;
- typed value and port compatibility;
- graph documents, connections, revisions, and commands;
- portable project and standalone node-graph documents;
- private project identity/provenance records, representations, fingerprints, explicit
  AssetSet values, and materialization contracts;
- configuration versus authored-state separation;
- validation, planning, evaluation, dirty propagation, and diagnostics;
- cache keys, artifacts, receipts, and evidence semantics;
- repository and capability interfaces;
- the versioned application facade used by clients.

It does not own:

- SwiftUI/AppKit or Windows controls;
- Adobe, Cloudinary, filesystem-root, or account-specific policy;
- raw credentials;
- node catalog categories as evaluator variants;
- arbitrary node-private SQL;
- UI selection as an undeclared dependency.

Every state mutation is a semantic command against an expected revision.
Derived work captures its input revision/digest and cannot become current after
the source changes. Human-authored state and execution receipts are authority;
proxies and reproducible outputs are caches or artifacts according to contract.

The evaluator contracts remain general, but `0.2.0` implements only the depth
needed by first-party vertical slices. A mature scheduler, nested execution,
broad retry orchestration, optimization, and remote execution are not required
before Layout authoring unless a concrete node makes one necessary.

The first executable path applies immutable semantic command envelopes against
an expected graph revision, then validates and evaluates an acyclic graph in
deterministic topological order. Node semantics enter through an exact-definition
runtime callback. Per-node keys include definition, configuration, authored
state, typed inputs, environment, and implementation fingerprints; they form
the initial dirty-propagation/cache boundary without making the local executor
a permanent scheduling architecture.

Portable project semantics are distinct from three other state classes:
runtime/evaluation state, disposable caches/derived artifacts, and native-client
Window Layout state. The Project Document embeds the authored graph; a separate
Node Graph Document exports that same graph for trivial sharing. Neither format
contains secrets, machine bindings, evaluation progress, caches, or panel/window
layout.

Project asset context separates three identities that must not collapse:
`AssetId` is the user-recognized creative item, `AssetRepresentationId` is one
rendition such as a paired HDR or SDR flattened TIFF, and `ProjectResourceId`
binds that rendition to a movable project-relative location. Content changes
produce new representation fingerprints; path changes do not. Availability and
materialized machine paths are runtime results rather than portable state.
