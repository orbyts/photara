# Storexa integration

## Current exact review packet — 2026-09-12

The [D19 static delta](D19_STATIC_SCHEMA_DELTA.md) is complete for review and requires no Storexa API change. Photara owns additive schemas, scoped authorization/sync and package contracts. R1–R8 were accepted as proposed 2026-09-12; CXT2 inert DDL is complete, followed by separately selected CXT1a/CXT3; L3 remains paused.

**2026-09-12 D19 amendment:** [Library and node contracts](LIBRARY_AND_NODE_WORK_SURFACES.md)
and revised D18 pause L3. Library is the persistent ownership domain; Project grants,
typed variable SQL and project-filtered authorization remain Photara-owned.
Consistency review and logical/package/NodeSDK freeze precede static exact CXT2
DDL/sync review, then revised CXT1/CXT3. No new Storexa abstraction or change to
L2's six applied migrations is implied. Resource/SecretRefs, expression evaluation
and package publication remain outside Storexa. No D19 runtime/DDL is implemented.

Status: S0 complete, 2026-09-11. Storexa 0.2.0 was released from sibling commit
`22c4270`, tagged `v0.2.0`, pushed to GitHub, and published to crates.io. This
document freezes the integration boundary. S7 later approved D1–D17; bounded L2
now adopts Storexa for a separate local family with temporary-database tests.

## Released foundation

Storexa 0.2.0 preserves its PostgreSQL 0.1.0 API and adds explicit SQLite types:

- `Database`, `DatabaseConfig`, and `Transaction` remain PostgreSQL-specific;
- `SqliteDatabase`, `SqliteDatabaseConfig`, and `SqliteTransaction` are explicit;
- both support pools, health, acquisition, transactions, application-owned
  migrations, classified/redacted errors, tracing, statistics, and shutdown;
- SQLite supports literal file paths and isolated memory databases, explicit
  file creation, foreign keys, busy timeout, journal and synchronous policies;
- Storexa does not provide an ORM, application records, SQL translation, sync,
  network-share coordination, or a global registry.

The sibling source is
[`../../../storexa`](../../../storexa/README.md). Photara must consume a pinned
published release, beginning with `storexa = "0.2.0"`, rather than an indefinite
unversioned path dependency.

## Ownership boundary

| Layer | Owns | Must not own |
| --- | --- | --- |
| `photara-core` | Typed identities, invariants, commands, revisions and evaluation semantics | Connections, SQL or backend drivers |
| Photara domain repositories | Records, queries, schemas, DDL, migrations, validation and mapping | Pool implementation or secrets |
| `photara-store` | `.photara` package contracts and safe file publication | Library SQL or cloud authentication |
| Storexa | Database lifecycle, engine-specific transactions, migration execution, diagnostics and errors | Photara entities, authorization, subscriptions, packages or NodeSDK policy |
| Photara service | Auth0 authorization, Library/Project access policy, sync protocol and Neon access | Native presentation or node-private authority |
| Native host | Local paths, bookmarks, Keychain and platform CloudKit authorization | Portable semantic identity |

Storexa is a mechanics dependency below Photara repositories. It does not replace
`photara-store`; package persistence is not a database transaction.

## PostgreSQL and Neon

The Photara API uses Storexa's existing PostgreSQL types. Photara owns separate
PostgreSQL migration files and SQL, including authorization predicates and
server constraints. Neon is provider metadata and deployment infrastructure;
it does not change repository semantics. The desktop application never receives
the privileged connection URL.

PostgreSQL integration tests require disposable direct and optional pooled URLs.
Migrations use a direct endpoint. Provider project/branch creation remains a
control-plane concern outside Storexa.

## Local SQLite

Photara's current `SqliteLibraryRepository` stays synchronous and built on
`rusqlite`; its public behavior and files are preserved. L2 adds an asynchronous
`gen2::LocalLibraryStore` with a different schema family/API, not an in-place
substitution. See [L2 implementation](LOCAL_LIBRARY_IMPLEMENTATION.md).

Storexa is pinned to 0.2.0, SQLx to 0.9.0. Their SQLite binding range conflicts with
rusqlite 0.40.2, so the narrowly approved compatibility pin is rusqlite 0.39.0 /
libsqlite3-sys 0.37.0. Existing source/API and all five retained tests are unchanged.
No Storexa or SQLx fork/change was made.

Any future cutover/import from the retained v1 adapter still requires:

1. Preserve schema-v1 data and migration history in a fixture copy.
2. Introduce a Storexa-backed adapter behind the existing typed repository contract.
3. Run both adapters against the same behavioral conformance suite.
4. Verify revisions, immediate write-conflict behavior, foreign keys, hierarchy,
   tombstones, change cursors, corruption reporting, cancellation and reopen.
5. Define how the existing `user_version` ledger transitions to SQLx migrations;
   never replay an already applied semantic migration under a new identity.
6. Cut over only after parity; retain a rollback/read-compatibility plan.

L2 uses explicit paths and requires local WAL/FULL; future host integration may
choose local Application Support. No
live SQLite/WAL database belongs inside an SMB `.photara` package. One SQLite
writer and bounded busy/acquisition timeouts are explicit operational facts.

## CloudKit

CloudKit is not SQL and cannot implement `SqliteTransaction` or PostgreSQL
transaction semantics. A future integration uses a separate record/change-sync
capability or platform-hosted adapter with explicit:

- containers, databases/zones and record identifiers;
- change/conflict tokens and partial-failure reporting;
- batching, retry, rate limits and idempotency;
- deletion/tombstone retention and account/container availability;
- native entitlements and credentials outside Rust semantic state.

The local outbox remains Photara-owned. Remote changes are validated and applied
through Photara repositories. Non-Apple clients never depend on CloudKit-only IDs.

## Cross-store consistency

There is no distributed ACID transaction across a project package, local SQLite,
Neon/PostgreSQL and CloudKit. Cross-boundary workflows use:

1. a durable local intent and idempotency key;
2. one backend-local transaction at a time;
3. atomic package publication where package state changes;
4. an outbox/inbox and retryable state machine;
5. verification plus recovery or compensation;
6. durable evidence when an external effect is uncertain.

Timestamps do not resolve conflicts. Expected revisions, mutation IDs, server
revisions and change cursors remain distinct.

## Release verification recorded

Storexa 0.2.0 passed:

- 13 unit tests and 6 SQLite integration tests;
- 2 documentation tests;
- strict Clippy for all targets and formatting;
- migration/checksum, commit/rollback/drop, persistence, pragmas, foreign keys,
  lock contention, acquisition timeout, memory isolation and redaction coverage;
- executable SQLite example, `cargo package`, and publish dry run;
- crates.io publication verification (`cargo search` returned 0.2.0).

The two PostgreSQL integration tests compiled but remained ignored because no
disposable test URLs were configured. Existing CI retains declared Rust 1.94
coverage; local verification used Rust 1.95.

## S0 acceptance

- [x] PostgreSQL API preserved.
- [x] SQLite capability is explicit and released.
- [x] Storexa remains domain-agnostic.
- [x] Photara/Storexa/host ownership is unambiguous.
- [x] CloudKit is not modeled as SQL.
- [x] Cross-store work rejects distributed-transaction claims.
- [x] Photara adoption and `rusqlite` migration have bounded gates.

L2's 16 new temporary-database tests and retained regressions pass; no cloud
backend was executed. Next is D19 consistency and contract/static review; L3 is paused. Neither a legacy
importer nor automatic application cutover is required.
