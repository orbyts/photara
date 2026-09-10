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
