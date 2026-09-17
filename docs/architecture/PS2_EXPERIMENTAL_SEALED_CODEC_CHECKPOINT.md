# PS2 experimental sealed-root codec checkpoint — 2026-09-17

Status: disposable on-disk fixture, not a production package format or migration.
This implements the approved sealed-root and Resource Storage, Verification and
Retention semantics only under `crates/photara-store/tests/package_planning/`.
The existing 1.1 reader, production writer, application and live packages are
unchanged. The feature string and root-set discriminator are deliberately
`example.fixture.*`; the minimum-reader coordinate is a fixture-only sentinel,
not a reserved production version.

## Implemented boundary

- `HEAD.json` alone dispatches to an immutable, checksummed root-set commit.
  Commit feature, discriminator, bootstrap, identity, legacy-shaped active
  references, package revision and root-set inventory are checked. The actual
  1.1 reader refuses the required feature before treating this as legacy
  ancestry. Authored revision is separate from monotonic package revision.
- Active and recovery roots each validate their complete package-resident
  schema closure and exact inventory without walking predecessors. Operation
  inclusion/dedupe records are package objects retained independently of
  history or undo; the active accepted prefix must extend recovery's prefix.
- The package closure includes logical Resource, exact Version, Backing,
  Provenance and Retention records. The backing uses a logical StorageLocationId
  and opaque locator; a previously verified offline backing stays a recorded
  obligation, not an implicit media read or confirmed loss. A reference-looking
  opaque authored field is not interpreted as a dependency.
- Opt-in conversion fixture snapshots the complete original 1.1 directory,
  including unknown optional bytes, with exact path/digest/length inventory.
  The original remains separately inspectable by the actual 1.1 reader through
  interrupted conversion and later root turnover. No automatic conversion or
  removal of that snapshot is implemented.
- Per-operation durable fixture intents bind the original HEAD, proposed HEAD,
  immutable commit bytes, original operation/request and inclusion. Recovery
  distinguishes old HEAD (pending), exact new HEAD with verified inclusion and
  known barrier (acknowledge), unknown barrier (freeze), and unrelated HEAD
  (conflict). Repeated preparation reuses the persisted IDs and bytes; receipts
  remain independent of later root turnover.

## Measured disposable evidence

The tests use temporary real directories, canonical JSON, SHA-256 object refs,
file synchronization and atomic replacement of `HEAD.json`. They exercise 32
on-disk root turnovers, two sequential journaled publications, 40 original
snapshot cut positions, and six dispatch/barrier/reopen/receipt cut positions.
The external-media probe models an 8,000,000,000-byte backing without allocating
it. Root opening, validation and turnover make **zero** external provider opens
and read **zero** external bytes; an explicit strong verification path records
one provider open and 8,000,000,000 logical bytes. The zero count therefore
tests the boundary, not an absent provider stub.

Verification on 2026-09-17: **24 focused sealed-codec tests passed**, zero
failures; `cargo test -p photara-store` passed **149 tests** with four
intentionally ignored fixture generators/probes; strict
`cargo clippy -p photara-store --all-targets -- -D warnings`,
`cargo fmt --all -- --check`, and `git diff --check` passed. All fixture package
paths are disposable. No app installation, live data or cloud operation is
part of this checkpoint.

## Limits and next gate

This is not power-loss qualification. Some immutable root objects are staged
before the interruption cuts, and the synthetic barrier-known input is not a
qualified storage-profile proof. The fixed fixture closure budget, operation
shape and synthetic 8 GB medium are not production thresholds or a media
implementation. There is no Asset Store, external publication protocol, garbage
collection, production codec identifier, live conversion, package writer, native
session/autosave coordinator or project switching here.

The next production boundary must separately review exact stable feature/codec
identifiers and representation, qualified publication/admission and recovery
ordering, and conversion-source retention/release. After that, implement the
shared Rust session/autosave authority before wiring browsing and switching in
Swift; Library lifecycle follows that durability boundary.

Exact changed implementation files:

- `crates/photara-store/tests/package_planning/mod.rs`
- `crates/photara-store/tests/package_planning/sealed_codec/mod.rs`
- `crates/photara-store/tests/package_planning/sealed_codec/codec.rs`
- `crates/photara-store/tests/package_planning/sealed_codec/recovery.rs`
- `crates/photara-store/tests/package_planning/sealed_codec/tests.rs`
