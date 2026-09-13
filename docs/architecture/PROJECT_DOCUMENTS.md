# Portable project and node-graph documents

## Current exact review packet — 2026-09-12

The [D19 compatibility plan](D19_STATIC_SCHEMA_DELTA.md#compatibility-mappings-and-activation) now specifies explicit association/conversion into package 1.1 while preserving these one-JSON bytes, IDs and opaque state. No old extension is reinterpreted as context or implicit input membership. R1–R8 were accepted as proposed 2026-09-12; this page still describes implemented v1 documents.

**D19 supersession note (2026-09-12):** [Libraries and node Work Surfaces](LIBRARY_AND_NODE_WORK_SURFACES.md)
is the current conceptual target. Library is the durable ownership domain; each Project
has one Library and explicit Project access. Graphs use connected AssetSets and
declared frozen context; a private package ledger is not an ambient Gallery/asset
union. Library management is app-owned; node Work Surfaces embed authorized host
pickers/components. First install opens local My Library. The pre-D19 implementation,
physical identifiers, examples and fixture contracts below remain baseline evidence,
not approval to reinterpret stored bytes. Exact contract/static schema review and
revised CXT1/CXT3 precede L3; no migration, source or fixture bytes change here.

This page describes the implemented single-JSON generation-two contract.
The proposed multi-document `.photara` package, immutable history, publication
and compatibility mapping are specified in
[Project package schema — S2](PROJECT_PACKAGE_SCHEMA.md). That proposal does
not imply the runtime already reads or writes packages.

Photara has two small, human-inspectable JSON document boundaries built from
the same semantic graph vocabulary.

## Project Document

`ProjectDocument` is the portable authoritative project state. It contains:

- document schema version;
- project identity and project revision;
- optional human-facing title and description;
- exact required package identities and release versions;
- one embedded `GraphDocument` containing graph identity/revision, nodes,
  connections, exact definition pins, generic configuration, and generic
  authored state;
- semantically identified supporting resources whose locations are normalized
  relative to an explicit project root;
- project-owned semantic asset context containing asset/representation
  identities, roles, capabilities, content fingerprints, and portable
  project-resource or stable runtime-resolution bindings;
- preserved unknown extension fields where the current schema can safely carry
  them.

The intended local shape is conceptually:

```text
my-project/
├── photara-project.json
├── assets/
└── optional project-owned supporting files
```

The working filename is `photara-project.json`. Standalone graph exports use a
human-chosen name with the suffix `.photara-graph.json`. These are product/file
conventions over the versioned JSON schemas, not semantic identity.

Resource paths never become semantic asset identity. A resource has its own ID;
its relative path only tells a project adapter where to look. Absolute paths,
parent traversal, and drive-qualified paths are rejected by the portable Core
type. Provider/external representations carry only a stable runtime-resolution
handle in portable JSON. Machine paths, provider locators, accounts, and
credentials remain outside the document and can be rebound without changing
asset or representation identity.

One asset may expose multiple related representations. Paired HDR and SDR
flattened TIFFs remain one asset with two representation identities and content
fingerprints. Moving either resource changes only its binding path; replacing
its bytes changes the fingerprint. Runtime availability and materialized local
paths are not serialized.

## Standalone Node Graph Document

`NodeGraphDocument` is the trivial graph-sharing file. It copies the same
`GraphDocument` and exact package requirements into a self-describing wrapper,
but excludes project identity, project revision, and the project resource
inventory. A user can send this file when someone asks, “Can I get your node
graph and how it is connected?”

The export preserves node IDs, connections, definition pins, configuration,
authored state, and unknown node-owned fields. Project-specific bindings inside
opaque node state may be unresolved for the recipient and must be diagnosed,
not erased. A future template/import workflow may remap identities or omit
bindings using the same graph vocabulary; that is not part of the first format.

## Explicitly separate state

Neither portable document contains:

- evaluation attempts, dirty/cooked markers, progress, or cancellation;
- proxy, thumbnail, typed-value, preview, or intermediate caches;
- materialization results, current availability, or Gallery selection;
- passwords, API keys, OAuth material, environment values, or resolved secret
  contents;
- machine-specific host paths, mount paths, or account bindings;
- panel placement, split sizes, tabs, floating geometry, monitor choice,
  selection, focus, or other editor UI state.

Runtime/evaluation state belongs to the Core state service. Reproducible cache
data belongs to cache storage. Library state belongs to the native client.
Deleting either runtime/cache or editor session state cannot damage the portable
project semantics.

## Serialization and missing dependencies

Pretty JSON is for people; whitespace and object-key order are not semantic.
Canonical JSON and SHA-256 supply stable document digests. Both documents are
validated before acceptance and carry explicit schema boundaries.

Opening a document does not require its node packages to be installed. Exact
package/definition pins and opaque schema-tagged node state remain available so
Core can later diagnose missing or incompatible packages, providers, hosts,
accounts, or resources without destroying authored data.
