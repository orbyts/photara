# CXT3b local runtime

Started from clean `415039e` on 2026-09-12. The accepted local D19 proposal bodies
are now executable migrations `0007`–`0012`; the six baseline migration files remain
byte-for-byte unchanged. There are no rename migrations or compatibility columns.
The final local schema has 70 domain tables, 68 explicit indexes and 169 triggers.
The 12 migration files contain 310 top-level statements. SQLite family
`photara.local.g2`, application ID `0x50485432`, user version 2 and schema epoch 1
remain fixed; minimum reader and writer are both 2.

## Startup and cutover

`LocalLibraryStore::open_app_state` inspects existing files read-only before opening
an SQLx pool. It checks the SQLite header/family, epoch, reader/writer floor and
stored nonnil device ID; the regular opener then verifies the normalizer, SQLx
checksums and migration ledger under `BEGIN IMMEDIATE`. Unknown/foreign files,
symlink paths, newer floors, changed checksums and unsupported cloud associations
fail closed. Existing files are never deleted or converted.

Fresh creation uses exclusive file creation and mode 0600. Baseline installation,
metadata/device bootstrap, D19 migrations and the new floor commit in one controlled
transaction. A floor-update failure rolls back all six D19 migrations and their
ledger entries. Only an empty known `0006` database upgrades automatically: a
populated baseline has no explicit D19 authority mappings and is refused. Startup
retains the device ID and creates a database-scoped default `My Library` with an
explicit local controller. Reopening or renaming the Library does not create another.
A deliberate local-controller transfer is retained across restart.

The real Swift `AppModel` calls the generated UniFFI `initializeLocalState` entry
point at startup. Its exact production path is:

`~/Library/Application Support/Photara/State/photara-local-v2.sqlite`

The existing Library panel adapter remains its prior independent feature; no old
Library records, authored Projects or user storage are adopted into the new database.
Graph, node, port, noodle, selection, camera and Work Surface implementations are
unchanged. Window/session preferences remain outside Library authority.

## Typed local controllers

- Local membership exposes the explicit controller, revision and authorization
  generation. It does not fabricate an Account or cached online membership.
- Project registration atomically retains one Library association and commit pin,
  restricted policy, explicit creator-manager grant and catalog entry. Current
  Project authorization checks the retained grant on each call; Library ownership
  does not bypass a revoked or missing Project grant.
- Policy/grant commands use current manager permission and exact CAS. Widening
  requires matching actor/policy disclosure evidence with reported package and
  projection pins. Commands advance affected generations once and retain immutable
  control audit envelopes. Controller transfer revokes and grants atomically,
  validating every active Project's last manager at commit.
- Storage roots and classification are one CAS aggregate. Slot names stay reserved.
  Host paths are device-only, with separate observation revision, effective binding
  generation and selection CAS. Fake-host resolution runs outside the transaction;
  selected scope, rights and generation are checked before and after its callback.
  The host remains responsible for actual platform permissions, containment,
  content verification and lease expiry; these tests touch no user source storage.
- Library variables use checked Core types, immutable identity/type coordinates,
  retained names, stable ValueId on clear, exact aggregate CAS and cycle validation.
  Definitions/defaults/current values and new expressions/dependencies commit
  together. Typed provenance preserves structural type and owner coordinates;
  readers check their indexed type/time projections. Project variables/context
  remain authoritative in the checked CXT3a package reader, with no SQLite duplicate.
- Private device-context observations retain bounded typed host-place facts and
  both canonical/content digests. Snapshot IDs are immutable and Run/device scoped;
  exact retries are idempotent. Captured readiness is never a live handle or lease.
- Accepted Library proposals revalidate current target facts, apply the literal,
  and retain an immutable receipt in the same transaction. Exact retries return
  that receipt after restart; stale targets and changed requests fail. Project
  proposals retain their exact expected package commit in an awaiting-publication
  intent. No package-published receipt or package write is fabricated.
- Media linking verifies supplied immutable bytes and binds digest, Project,
  purpose, source commit and disclosure projection. Reads require current Project
  read permission and the exact active digest/purpose link. A digest alone is
  insufficient. This is local projection registration, not upload or URL issuance.
- Scoped transport tables are installed. Typed Library-variable offline intents
  retain exact canonical post-state, scoped base or predecessor, and idempotent
  operation bytes. Channel scope/generation/revision are checked transactionally.
  Local-only startup does not create channels, seal requests, dispatch work, issue
  online authorization, accept service receipts or install service snapshots.
  Those online adapter gates remain CXT3c; unresolved sealed work blocks activation.

## Verification

258 full offline Rust tests pass, with four intentional ignored tests. Full
all-target check, Clippy with warnings denied, formatting and whitespace checks
pass. The static schema and naming checks pass; all 12 canonical fixture containers
and 92 embedded records remain verified by the Rust codec. Bridge verification
passes at graph revision 19 with three progress and two cancellation events.
Shared UI checks pass with 98 top-level snapshots; production checks pass with
10 snapshots, including the real AppModel startup wiring under a disposable root.
Light Graph and Dark Library production snapshots were visually inspected.

The production initializer created exactly one active `My Library` and explicit
local controller at the path above. Read-only inspection found all 12 migration
entries, floor 2/2, 70 domain tables, `integrity_check=ok` and no FK violations.
File permissions are 0600. The database was closed and checkpointed before the
inspection. The ordinary read-only CLI failed while WAL sidecars were absent;
immutable mode successfully inspected the closed, sidecar-free file without writes.

See [exact migration/file inventory](CXT3B_LOCAL_INVENTORY.md) and the execution
roadmap for the completed check results. Tests use disposable SQLite databases,
fake storage hosts and fake cached transport facts. Coverage includes fresh and
empty-0006 initialization, floor refusal, failed migration rollback, foreign keys,
integrity, independent-pool CAS, last-manager rollback and transfer, context/value
retention, expression storage, recovery/retry, and scoped media/intent checks.

The first dedicated Graph Lab run reported one randomized branch prerequisite
failure (`20260909`, dark/curved, routing knot changed). Its isolated replay passed
541 assertions. All 42 Graph/Graph Lab source/resource/test files match `415039e`.
The full rerun passed 15,586 assertions with no failures. Both runs are retained in the handoff evidence.

No PostgreSQL, Neon, Auth0, CloudKit, package publisher or external media provider
was invoked. No v0.1.3 database, SMB storage, commit or push is part of this slice.

## Read-only inspection

Close Photara first and confirm no `-wal` file exists before using immutable mode.
This command opens no writable database connection:

```sh
sqlite3 -readonly 'file:/Users/suhail/Library/Application Support/Photara/State/photara-local-v2.sqlite?immutable=1' "SELECT schema_family, schema_epoch, minimum_reader, minimum_writer FROM schema_metadata; SELECT display_name, state FROM libraries; PRAGMA integrity_check; PRAGMA foreign_key_check;"
```

For an ordinary read-only connection while the app is managing its WAL files:

```sh
sqlite3 -readonly '/Users/suhail/Library/Application Support/Photara/State/photara-local-v2.sqlite' "SELECT version, success FROM _sqlx_migrations ORDER BY version;"
```
