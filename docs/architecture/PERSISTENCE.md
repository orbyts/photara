# Persistence

One Core-owned state service provides the authoritative transaction and revision
boundary for a workspace.

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

The repository boundary remains backend-neutral. The first persistent adapter
may reuse PostgreSQL/Storexa experience, but no PostgreSQL or Storexa type enters
Core or the node SDK.
