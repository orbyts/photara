# Photara Library

Backend-neutral user/studio records, portable project references and a first local
SQLite repository. This crate is separate from Core's node graph and project
asset Gallery. Node packages have no dependency on this crate and receive no SQL
or credential access.

`LibraryRepository` exposes optimistic `put`, owner-scoped `get`/`search` and
bounded change-cursor reads. `SqliteLibraryRepository` applies schema v1 under an
immediate transaction; it rejects unknown database schema versions. Tombstones
are retained, location hierarchies are owner-scoped and acyclic, and record kinds
cannot change under a stable ID. The host owns thumbnail preparation/storage and
resolves content-addressed PNG media independently from portable records.

`ProjectLibraryContext` contains project assignment identities and historical
record snapshots. It is persisted by the Project Document, never as a second
SQLite project authority. See [Library architecture](../../docs/LIBRARY_ARCHITECTURE.md).

```sh
cargo test -p photara-library
cargo clippy -p photara-library --all-targets -- -D warnings
```

Cloud modes describe future synchronization over this same local working copy.
No cloud transport, Auth0 session, Neon credential or CloudKit implementation is
included. Incremental changes are synchronization groundwork, not a complete
outbox/conflict/idempotency protocol.

## Separate generation-two adapter

`gen2::LocalLibraryStore` is the additive async Storexa 0.2 / SQLx 0.9 adapter for
the new `photara.local.g2` family. It never converts or silently opens a schema-v1
file. Explicit create/open modes, six ordered migrations, typed Workspace Library
records, CAS/tombstones, Unicode-16 Kind claims, immutable local changes and
catalog/device projections are implemented. SQL/pool access remains private.

The retained adapter is pinned to rusqlite 0.39.0 so both drivers share
libsqlite3-sys 0.37.0; its source/public behavior and five tests are preserved.
See [L2 API, dependency rationale, examples and exclusions](../../docs/architecture/LOCAL_LIBRARY_IMPLEMENTATION.md).
Merge/claim transfer, sync, media, application cutover, package publication and
legacy import are not exposed. Tests create only temporary databases; never use
a user's Library file as a test target.
