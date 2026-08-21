# Portable Core

Core is a UI-, provider-, host-, storage-backend-, and media-kind-independent
Rust library.

It owns:

- stable semantic identity and versioning;
- typed value and port compatibility;
- graph documents, connections, revisions, and commands;
- portable project and standalone node-graph documents;
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
workspace UI state. The Project Document embeds the authored graph; a separate
Node Graph Document exports that same graph for trivial sharing. Neither format
contains secrets, machine bindings, evaluation progress, caches, or panel/window
layout.
