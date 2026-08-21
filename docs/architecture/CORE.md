# Portable Core

Core is a UI-, provider-, host-, storage-backend-, and media-kind-independent
Rust library.

It owns:

- stable semantic identity and versioning;
- typed value and port compatibility;
- graph documents, connections, revisions, and commands;
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
