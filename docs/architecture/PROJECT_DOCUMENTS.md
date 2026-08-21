# Portable project and node-graph documents

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
  identities, roles, capabilities, content fingerprints, and bindings to those
  supporting resources;
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
type. Provider-neutral external references are added later when a real provider
requires them.

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
  selection, focus, or other workspace UI state.

Runtime/evaluation state belongs to the Core state service. Reproducible cache
data belongs to cache storage. Workspace state belongs to the native client.
Deleting either runtime/cache or workspace state cannot damage the portable
project semantics.

## Serialization and missing dependencies

Pretty JSON is for people; whitespace and object-key order are not semantic.
Canonical JSON and SHA-256 supply stable document digests. Both documents are
validated before acceptance and carry explicit schema boundaries.

Opening a document does not require its node packages to be installed. Exact
package/definition pins and opaque schema-tagged node state remain available so
Core can later diagnose missing or incompatible packages, providers, hosts,
accounts, or resources without destroying authored data.
