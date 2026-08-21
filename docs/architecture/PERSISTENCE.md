# Persistence

One Core-owned state service provides the authoritative transaction and revision
boundary for a workspace.

The portable Project Document is the authoritative serialized project and graph
contract. Backend tables, indexes, search records, and materialized evaluation
views may accelerate or coordinate the application, but they must round-trip
the document without becoming a second incompatible authored-state format. The
standalone Node Graph Document is an explicit import/export boundary, not a
database dump.

The first implementation is deliberately the minimum foundation required by
the Layout vertical slice: package/definition registrations, workspaces,
graphs, exact version-pinned instances, connections, project asset references,
configuration, authored state, and namespaced node state with revision-safe
save/reopen. Repository and transaction boundaries remain capable of growing
without requiring the full package-distribution lifecycle first.

Shared Core records include:

- workspaces and projects;
- asset identity and representation references;
- package definitions and installations;
- graphs, revisions, instances, and connections;
- typed values and evaluation records;
- artifacts, diagnostics, and evidence receipts;
- credential references, never secret material.

Nodes receive private state through a namespace keyed by package, definition
version, instance, state kind, schema version, revision, and digest. They do not
receive the database connection or inspect another namespace. Cross-node
bookkeeping must become a declared typed value, artifact, or receipt.

Node state initializes lazily, migrates explicitly, and remains when a package
is disabled or uninstalled unless the user separately confirms destructive
deletion.

Unknown or newer persisted fields are preserved where practical. A missing or
disabled package must not make Core erase its graph instance or namespaced
state merely because that package cannot currently execute.

Runtime/evaluation records, caches, credentials, and native workspace layout
remain outside the Project Document even when a backend persists them for local
operation. Deleting those records cannot delete portable authored semantics.

The repository boundary remains backend-neutral. The first persistent adapter
may reuse PostgreSQL/Storexa experience, but no PostgreSQL or Storexa type enters
Core or the node SDK.
