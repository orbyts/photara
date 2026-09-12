# D19 CXT2 inert DDL proposal

**NON-EXECUTABLE REVIEW STATUS. Do not apply these files.** Suhail accepted
[R1–R8](../../D19_CONTRACT_FREEZE.md) as proposed on **2026-09-12**. CXT2 acceptance
covers this static translation and its inventory only. It installs no migration,
changes no database and authorizes no runtime implementation.

The eleven `.proposal.sql` files are review artifacts outside runtime migration
paths. Nothing registers them with SQLx, a build script or a service. SQL spelling
is deliberately concrete so later CXT3 work can review, adapt and test it; the
comments and suffix are a workflow boundary, not a SQL execution interlock.

Read the [approved signatures](../../D19_STATIC_SCHEMA_DELTA.md), the
[exact inventory and static evidence](INVENTORY.md), and the
[SQL/repository/controller responsibility ledger](RESPONSIBILITIES.md) together.
The ledger is part of the proposal: SQL syntax alone cannot implement authorization,
typed canonical bytes, package evidence or a commit-level SQLite aggregate contract.

| Backend | Proposal reservations | Added relations | Result if later implemented |
| --- | --- | --- | --- |
| SQLite | 0007–0012 | 26 = 14 common + 12 device/local | 44 → 70 |
| PostgreSQL | 0008–0012 | 20 = 14 common + 6 service | 35 → 55 |

Service **0007 remains the original, unexecuted privileges reservation** in S4.
There is no second service 0007. Its seven baseline sections are unchanged. Local
0001–0006, S2–S6 SQL/JSON baselines and package fixture bytes stay pinned. The sole
baseline data-column extension is four nullable columns on service
`media_upload_sessions`; all other proposed DDL adds relations, constraints,
indexes, guards or policy/grant replacements. Metadata floor UPDATEs are inert
text too: local reader/writer 2 and service API 2, with family/epoch and local
application_id/user_version preserved.

The next eligible slice is **CXT1a: pure Rust IDs, access masks, portable resource
contracts, AssetSet v2 and manifest v2 validation**, after separate scope selection.
CXT1b follows for bounded expressions. CXT3a/b/c separately prove codecs, disposable
SQLite and disposable PostgreSQL/fake transport. No Neon or other live service is
the next step. L3 publication, user projects, SMB, UI and release remain gated.
