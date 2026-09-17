# PS2 positive sealed-root fixtures — 2026-09-17

Status: fixture-only semantic checkpoint, ready for review. The approved
[resource/storage contract](RESOURCE_STORAGE_AND_VERIFICATION_CONTRACT.md) and
[sealed-root proposal](PS2_SEALED_ROOT_PROTOCOL_PROPOSAL.md) govern this work.
The fixture model is not a package codec, writer, managed Asset Store, migration,
garbage collector, provider adapter, or power-loss qualification.

## Exact change inventory

- `crates/photara-store/tests/package_planning/mod.rs` registers the positive
  fixture module.
- `crates/photara-store/tests/package_planning/sealed_positive/mod.rs` contains
  test-only hashing and refusal helpers.
- `crates/photara-store/tests/package_planning/sealed_positive/roots.rs` models
  two independently verifiable roots, schema-defined package-object closure,
  exact inventory, included-operation evidence, pins, publication cuts,
  reconciliation, and conservative retirement eligibility.
- `crates/photara-store/tests/package_planning/sealed_positive/resources.rs`
  models semantic Resource and Captured Version IDs independently of working
  files and backings, cheap observations, retention/availability, placement,
  storage-slot capture, cross-store interruption, capacity, and backup claims.

No production source, current 1.1 reader, schema, migration, format/feature
identifier, or existing negative boundary fixture changed.

## Measured fixture evidence

The 19 positive tests pass. A full `cargo test -p photara-store` run passes
125 tests, with four pre-existing intentionally ignored tests and zero failures.
`cargo clippy -p photara-store --all-targets -- -D warnings`,
`cargo fmt --all -- --check`, and `git diff --check` pass.

| Fixture observation | Result |
| --- | --- |
| Repeated sealed-root turnover | 64 turnovers; maximum exact retained set is eight IDs (two roots, six package objects). Predecessor commitments do not recursively retain ancestry. |
| Accepted-operation continuity | 127 acknowledged original operations retained in the dedupe ledger; one later journal entry remains pending. Altered ID/evidence and non-prefix inclusion refuse. |
| Root publication interruption | Nine modeled cuts; pre-HEAD outcomes remain unresolved, while post-HEAD reconciliation requires a modeled barrier and structural reopen before acknowledgement. Unavailable barrier refuses. |
| Cross-store publication interruption | Six modeled cuts; original operation ID is required, and no package success receipt or premature backing cleanup follows an unknown outcome. |
| Large mutable working media | Synthetic 8,000,000,000-byte resource, not an allocated 8 GB file. Cheap observation and 64 integrated metadata-open/autosave/root-turnover cycles produce zero external-media reads and zero media hash bytes. Offline and ambiguous-backing cases each repeat 64 zero-read cycles. |
| Strong exact-byte boundary | Capture accounts for one synthetic 8 GB source read; verified backing publication separately accounts for an 8 GB destination verification read. Same-size/same-mtime observations do not prove equality. |
| Version retention | Captured V1 survives mutable working V2; capture alone creates no indefinite pin. Six explicit pin classes block retirement, while stale scans and unresolved operations refuse eligibility. |
| Availability versus retention | Previously verified offline/ambiguous backing keeps package metadata valid and its receipt; confirmed loss/corruption breaches only an unmet active replica obligation. Missing required embedded package objects fail validation. |
| Placement and backup | Qualified/authorized beside-source fallback, required-destination refusal, stable slot capture, separate/shared capacity domains, and package-only versus copied-and-verified complete-backup claims are tested. |

The old 1.1 negative reader-boundary tests still pass unchanged: an optional
marker cannot legalize truncated ancestry, and an unsupported required feature
refuses. Fixture object IDs, stages, barriers, reservations, backing keys and
byte counters are synthetic. These tests establish semantic invariants, not
actual 8 GB throughput, filesystem durability, or a production retention SLA.

## Next boundary

No new architectural decision arose from these fixtures. Production work still
requires separate review of the exact root/required-feature/dispatch contract,
conversion-source representation and capacity/retirement/rollback semantics,
qualified package and Asset Store publication, the remaining failure matrix,
and platform durability evidence. Existing full validation still hashes embedded
blobs; the target structural-open/deferred-hash distinction is not implemented.
The fixture's zero-read assertion applies to external large media and cannot be
used to claim that the current production reader has changed.
