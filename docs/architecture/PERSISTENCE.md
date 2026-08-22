# Persistence

One Core-owned state service provides the authoritative transaction and revision
boundary for a workspace.

The portable Project Document is the authoritative serialized project and graph
contract. Backend tables, indexes, search records, and materialized evaluation
views may accelerate or coordinate the application, but they must round-trip
the document without becoming a second incompatible authored-state format. The
standalone Node Graph Document is an explicit import/export boundary, not a
database dump.

Stage 4A persists exactly two record classes:

- whole portable Project Documents, including the embedded graph, exact node
  pins, configuration, authored state, project-relative resource references,
  and project-owned semantic asset/representation context;
- validated exact package manifests used to rebuild the ordinary definition
  registry after reopen.

`ProjectRepository` and `PackageManifestRepository` are backend-neutral. For
this slice, each whole-project create or revision-checked replace is the
transactional unit of work. There is no separate authoritative graph table to
coordinate with the project JSON. `InMemoryStateStore` supports tests and
short-lived services. `FileSystemStateStore` is the first durable adapter: it
opens a clean directory without a database, stores human-readable project and
manifest JSON, synchronizes temporary files, and atomically publishes or
replaces them. A per-project write lock protects compare-and-swap replacement
across store instances.

Package registration is append-only in Stage 4A. Persisting a manifest does not
implement download, installation, enable/disable, update, rollback, migration,
uninstall, trust, or signing behavior.

Unknown or newer portable project, graph, node, connection, configuration, and
authored-state fields remain preserved by the Project Document contract. A
missing package does not prevent the project from loading or make Core erase an
unresolved node instance.

Runtime/evaluation records, caches, credentials, and native workspace layout
remain outside the Project Document even when a backend persists them for local
operation. Deleting those records cannot delete portable authored semantics.

Stage 4A does not persist runtime/evaluation/evidence/artifact/receipt records,
workspace layout, credentials, or node-private state outside the portable
document because none is required by its gate. When a concrete later node needs
private durable state, it receives a scoped namespace rather than database
access. Credentials likewise remain behind future scoped host handles.

The future state service distinguishes scope explicitly:

- **user + exact node definition** stores reusable libraries and preferences,
  such as saved Layout presets or a Lightroom node's saved selections. This
  scope may participate in authenticated cross-device synchronization;
- **project + node instance** stores private operational state for one instance
  when that state is not portable authored semantics;
- **device** stores machine-only grants, security bookmarks, credential handles,
  cache locations, and local availability;
- the **Project Document** remains the portable authority for graph topology and
  authored edits that must travel with the project.

Moving data between these scopes is explicit. Saving a reusable preset is not
the same operation as applying that preset to authored project state. Sync is a
host/account capability over a user-scoped namespace, with schema versioning,
conflict handling, and deletion semantics; it is not network or database access
granted to node implementation code.

Representation availability and materialized machine paths remain runtime
state. Stage 6 proxy files and indexes are derived cache data stored outside the
portable project aggregate; deleting them cannot delete assets, representation
descriptors, fingerprints, authored state, or evidence.

No PostgreSQL, Storexa, environment-variable, or `v0.1.0` schema dependency is
present. A future backend may reuse those lessons without changing the
repository contracts or replacing the portable authoritative format.
