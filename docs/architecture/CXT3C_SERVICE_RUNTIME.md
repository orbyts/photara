# CXT3c disposable service boundary

Implemented from clean synchronized `7e6d794`, started 2026-09-12 and completed
2026-09-13. This slice is
uncommitted and unpushed. The service is a Rust library with injected identity,
clock and media ports; no HTTP daemon or deployed service was created.

## Executable database

`crates/photara-service/migrations/postgres` contains 13 ordered migrations,
executed through Storexa 0.2.0 and SQLx 0.9.0 against PostgreSQL 18.6. The actual
engine accepted all 697 migration statements. The measured inventory contains
55 domain tables, 569 columns, 153 indexes, 115 noninternal triggers, 48 functions
and 173 policies. All 45 RLS-enabled tables force RLS. The remaining identity,
metadata and control-only tables use explicit privileges. SQLx bookkeeping is
outside the 55-table domain total.

[Measured engine inventory](CXT3C_POSTGRES_INVENTORY.json) includes every column,
constraint, index, policy expression, function owner/search path, trigger and
stored migration checksum. PostgreSQL 18 reports 1,064 constraints: 379 CHECK,
476 NOT NULL, 55 primary keys, 108 foreign keys, 34 UNIQUE and 12 constraint
triggers. These counts include engine-created constraints and indexes.

Promotion preserves the S4 baseline bodies and accepted D19 proposal bodies with
two explicit adaptations:

- 0012 uses `CREATE OR REPLACE FUNCTION` for `authorize_library`, which 0007
  already defines with the same signature. No duplicate compatibility function
  or broad policy remains beside the replacement.
- 0013 grants schema usage to the identity reader, metadata read to controller
  and identity reader, and read-only SQLx ledger access to the three runtime
  roles. These are necessary for scoped access-fact lookup and startup validation.

A fresh migration installs baseline 0001–0007, inserts the explicit family and
normalization-policy bootstrap rows, and applies 0008–0013 in one outer
transaction. Existing databases must already have this family's current floor 2
and matching ledger checksums. Existing floor 1, newer floors, unknown families
and prepopulated foreign schemas refuse migration. There is no importer,
checksum rewrite, down migration or activation of existing sealed v1 work.

## Service ownership and authorization

Storexa owns pools, connections and transactions. Photara owns schema, canonical
codecs, role validation, typed commands, authorization, receipts and fakes.
`Service::connect` accepts distinct API, controller and identity-reader configs.
Runtime logins must be nonsuperuser, non-BYPASSRLS, unable to create roles, unable
to inherit the migration owner, and unable to inherit another runtime role.
Migration-owner credentials cannot initialize the runtime service. Pools are private.

The injected identity verifier returns signature-verified claims; the service
checks exact issuer, audience, nonempty subject and expiry, then resolves the
exact active `(issuer, subject)` and active Account. No email matching occurs.
`FakeAuth0` is an opaque random bearer registry for tests, not a JWT implementation.
The production verifier and real provider wiring are outside this disposable slice.

Every request uses transaction-local Account, Identity, Library, Project, device,
purpose, scope and authorization-generation settings. It locks all participating
Accounts and Identities in stable UUID order, validates the device under its lock,
then authorizes the Library/Project and checks the current generation. Writes
serialize through the Library; roots use expected-revision CAS. The current API
floor is checked under a transaction lock even on an already connected service.
No provider verification or media I/O occurs under a database transaction.

The controller role is trusted server code, not an arbitrary-SQL user interface.
A client-supplied Account UUID or GUC is not authentication. Ordinary API logins
cannot read access grants, invitation secrets, raw streams, receipts, upload
sessions or object keys. Trigger helpers have no API execution grant.

## Implemented typed routes

- Explicit Library UUID claim with collision refusal, owner membership, contract
  state, empty stream, immutable claim/control receipt and audit.
- Library membership role ceilings and CAS; Project registration as restricted
  with one explicit creator manager; Project grant create/replace/revoke/regrant,
  action containment, policy CAS and atomic manager transfer.
- Target-bound Project invitations with a seven-day maximum, current inviter
  rights, pinned policy revision, one-time verifier, expiry, accepted-grant binding
  and immutable receipt retries. Issuance returns a token; it sends no message.
- Minimal Project discovery DTO, separating discover from content read.
- Reported immutable package/catalog observations and Library storage projections;
  Library entitlement and Account developer-capability evaluation.
- Actor/generation-bound uploads, immutable-byte verification, reported commit/
  projection and current policy consent, exact Project digest/purpose links and
  bounded media URL issuance. Fake media URLs use the reserved `.invalid` domain.
- Scoped canonical content receipts, batches, snapshots, signed opaque cursors,
  feed pages and acknowledgements. Cross-Library and cross-Project requests fail.

The content codecs in this service slice are **StorageProjection** for a Library
and **CatalogProjection** for one Project. Snapshot roots explicitly use this
closed enum. Other installed schema entities do not have generic JSON write or
snapshot codecs exposed here. These snapshots describe the supported service
projections, not a complete backup of all 55 tables. Project files remain authored
package authority; service observations and receipts do not prove package writes.

Access mutations advance the Library authorization generation once and affected
Project policy generations once. Revoked grants override inherited membership;
active zero masks inherit. Library ownership never supplies hidden Project
manage-access. Deferred guards retain an active manager with an active identity
and retain an active Library owner. Account/device disable is exercised as a
trusted control operation in fixtures, not exposed as a public account-admin API.

## Fake online transport

`FakeSync` exercises explicit bootstrap, v1-settlement refusal, immutable sealing,
unknown outcomes after a dropped server reply, same-byte receipt reconciliation,
whole-batch application, access-loss retention, fresh-generation bootstrap and
conflicting working overlays. It stages every batch/root before replacing base
state or advancing the cursor. Access loss retains content and intents; it does
not manufacture a domain tombstone. It is an in-memory fake; it does not activate
or alter the CXT3b SQLite worker, device state, native app or real local database.

Server writes commit root, receipt, batch, child and high-water update together.
Deferred SQL checks close accepted receipt/batch/stream identity and scope;
controllers additionally compare complete canonical bytes, digests, typed root
sets, revisions, scope, actor/device and current generation. Feed bytes are never
filtered per reader. Snapshot budgets are 16 MiB/10,000 roots; feed pages are
bounded to 100 complete batches and 16 MiB. Current commands emit one root/batch,
with a conservative 1 MiB canonical command/batch cap. Receipts preserve their
original cursor; old generations require fresh bootstrap, not cursor reinterpretation.

## Reproduction and proof

```sh
python3 scripts/verify_service_postgres.py \
  --inventory docs/architecture/CXT3C_POSTGRES_INVENTORY.json
```

The runner creates a mode-0700 directory under `/private/tmp`, a private Unix
socket, disposable roles and a fresh database. TCP listening is disabled. It
uses no deployed database settings, runs the five explicitly ignored PostgreSQL
proof suites serially, records the actual engine inventory and stops PostgreSQL
in `finally`. Logs and stopped test data remain under the reported temporary root.
Ordinary offline Rust runs intentionally skip these five environment-dependent
suites; the runner executes all five successfully.

The five suites cover 678 counted authorization/privilege/sensitivity cells,
including actual unprivileged SELECT/INSERT/UPDATE/DELETE, all Project presets,
all inherited role masks and Library privacy sensitivities. Additional assertions
cover token/issuer/audience/expiry, Project-only access, invitations, receipt
collisions, explicit regrant, read/revocation serialization, concurrent content
and access CAS, last-manager/last-identity rejection, atomic transfer, feed rollback,
pooled commit/rollback/drop cleanup, checksum/floor refusal, disabled Accounts,
revoked devices, scoped media and fake transport recovery.

Full Rust, native and preservation evidence is recorded in
[CXT3c implementation inventory](CXT3C_IMPLEMENTATION_INVENTORY.md).

## Next separately gated action: fresh Neon schema and explicit seed

1. Confirm the exact fresh Generation Two Neon project/branch/database and approved
   service-host secret storage. Inspect that destination before any write; refuse
   an existing populated or unknown database.
2. Provision the migration owner and three separate runtime roles without owner
   inheritance, superuser or BYPASSRLS. Keep migration credentials out of runtime
   configuration. Require encrypted provider connections.
3. Apply these exact checked migrations and verify family/floor, all 13 stored
   checksums, role membership, RLS and scoped smoke tests in that fresh destination.
4. Seed only explicitly supplied verified issuer/subject → Account coordinates,
   device, Library UUID/name, owner membership, contract state and empty scoped
   stream. No email inference, package adoption or user-data import. Optional
   synthetic Project evidence requires an explicit restricted manager association.
5. Review the concrete seed and environment before enabling real identity/media
   adapters or a network service. Native online-worker activation and package
   publication remain separate integration work; the fake is not deployed.

No Neon, real Auth0/CloudKit/media, v0.1.3 data, package publication, commit or push
was performed. A failed fresh deployment stays disabled for inspection; no automatic
rollback rewrites a migration ledger or adopts an older schema.
