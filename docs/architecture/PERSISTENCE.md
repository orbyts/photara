# Persistence

## Current exact review packet — 2026-09-12

The [D19 exact static delta](D19_STATIC_SCHEMA_DELTA.md) now distinguishes package 1.1 authority, additive Library/access/storage/context tables, device recovery and v2 scoped transport. Physical Workspace/StorageRoot identifiers remain compatibility names. Current store implementation below is retained; R1–R8 were accepted as proposed 2026-09-12. CXT2 inert DDL is complete; CXT1a and CXT3 still require separate scope selection.

## D19 persistence amendment — contracts pending, no DDL

[D19](LIBRARY_AND_NODE_WORK_SURFACES.md) makes Library the durable catalog and
collaboration boundary and assigns each Project exactly one Library. Library
records include People/Organizations/SocialProfiles/Locations/kinds, policies,
variables, memberships and catalog. ProjectAccessGrant/invitations and restricted
versus library-visible policies require explicit authorization contracts; Library
membership is not automatic Project access. Package/SMB grants remain device state.

A package may retain a private identity/provenance/representation/artifact ledger
for Graphs/Runs; nodes see explicit AssetSet ports and declared frozen context,
not a semantic project asset union or `$project.assets`. Typed metadata enrichment
produces AssetSet/MetadataPatch; effect nodes materialize outputs and retain typed
receipts. Connector secrets/device paths remain host/account capabilities.

Existing S2–S6 Workspace/ProjectAsset names and six L2 migrations are retained as
physical compatibility baselines, conceptually superseded where D19 conflicts.
No new grant table, rename, nullable-origin reinterpretation, schema or format is
approved here. Logical/package/NodeSDK freeze and static exact SQLite/PostgreSQL/
sync deltas precede revised CXT1/CXT3 and L3. The implementation account below is
historical/current store evidence, not the amended target's access contract.

The Stage 4A implementation below remains the current store. The reviewable
[S2 project package schema](PROJECT_PACKAGE_SCHEMA.md) proposes its evolution
into immutable objects/commits with separate authored and durable-history roots.
Package publication is a Photara contract, separate from Storexa transactions;
S7 approved the design on 2026-09-11. The additive read-only
`photara-store::package` codec/validator now checks disposable S6 packages; see
[its exact boundary](PROJECT_PACKAGE_CODEC.md). It does not replace this store,
switch application authority, convert documents or publish packages. Storexa
adoption now exists in the separate [L2 adapter](LOCAL_LIBRARY_IMPLEMENTATION.md),
not a cutover of this Stage 4A store.

The pending [D18 amendment](TYPED_CONTEXT_AND_EXPRESSIONS.md) pauses L3. Library
variables are typed Library aggregates; Project/Graph/node variables are authored
package records; Run overrides/context/metadata evidence are immutable history.
Account scope is preferences only, superseding any implication below of ambient
user-scope semantic variables. Reusable presets become authored values only by
explicit application. CXT2 will review additive physical/package fields, required
features and apply/idempotency receipts; existing L2 migrations stay unchanged.
One command covers one Library transaction or one package commit, never a
distributed transaction across both. Private grants/SecretRefs remain host-only.

One Core-owned state service provides the authoritative transaction and revision
boundary for its explicit Library or Project scope.

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

Runtime/evaluation records, caches, credentials, and native Window Layout
remain outside the Project Document even when a backend persists them for local
operation. Deleting those records cannot delete portable authored semantics.

Stage 4A does not persist runtime/evaluation/evidence/artifact/receipt records,
Window Layout, credentials, or node-private state outside the portable
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
present in the implemented Stage 4A adapter. The generation-two target now
selects the sibling [`storexa`](../../../storexa/README.md) crate as the
domain-agnostic database mechanics layer. Photara's repository contracts,
application SQL/migrations, package publication, and portable authoritative
format remain owned here; adopting Storexa must not replace or weaken them.

Storexa 0.2.0 supports explicit PostgreSQL and SQLite SQLx capabilities.
CloudKit, if later coordinated through Storexa, is a record/change
sync capability or platform-hosted adapter—not a SQL backend. Cross-store work
uses staged operations, an outbox, idempotency, and recovery rather than a
distributed transaction claim.
