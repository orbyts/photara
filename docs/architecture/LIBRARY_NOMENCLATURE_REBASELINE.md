# Library nomenclature rebaseline

Authorized by Suhail on 2026-09-12 after the separately verified CXT3a package
slice. **Complete and verified.**

Generation Two is unshipped and contains no valuable user data. D19 R1's earlier
physical-name preservation decision is superseded. The forward domain, Rust
repository API, package records, SQLite baseline, PostgreSQL proposals and service
vocabulary use `Library`, `LibraryId`, `libraries` and `library_id`. No rename
migration, alias, shadow column, dual write or runtime naming adapter was added.
The v0.1.3 database is untouched reference material. A future optional importer,
possibly producing a sealed `Legacy Import` graph/run, is non-gating and undesigned.

## Authority boundaries

| State | Authority |
| --- | --- |
| Library, Account, catalog, access, device bindings | Local/cloud databases with their scoped authority rules |
| Project graphs, nodes, connections, configuration, authored Work Surface state, Project variables, asset/resource ledgers | `.photara` package; catalog/sync projections only as designed |
| Immutable runs and evidence | Package/cloud projections under their immutable history contracts |
| Reusable UI design tokens | Code or presets |
| Window geometry, open panes, selection, zoom, scroll, transient progress and unsaved edits | Per-device preferences/session state; no synced preference contract is implied |

The shell model is `EditorSessionModel`, with `EditorPanelID`, `EditorRegion` and
`EditorMode`. Optional node authoring views and bridge presentation fields use
Work Surface terminology. This UI state is not renamed into Library ownership.
Apple's framework API `NSWorkspace` and Cargo's `--workspace` remain technical
platform/build vocabulary, not Photara domain concepts.

## Exact changes and verification

The [file/hash inventory](LIBRARY_REBASELINE_INVENTORY.md) records every changed,
new and renamed path, all 12 fixture hashes and the six SQLx SHA-384 checksums.
The [D19 inventory](proposals/d19-cxt2/INVENTORY.md) was regenerated from actual
SQL text, including statement line numbers, object names and current hashes.

- Full offline Rust suite: **245 passed**, four explicit golden generators ignored.
  Core/SDK, package 1.0/1.1, Library SQLite, bridge, proxy and node tests pass.
- Offline all-target compilation, Clippy with `-D warnings`, formatting and
  whitespace checks pass. No dependency versions changed.
- SQLite baseline 0001–0006: **44 tables, 30 explicit indexes, 95 triggers,
  171 statements**. A fresh in-memory database reports `integrity_check=ok` and
  zero foreign-key violations; the full Rust suite also checks disposable file
  databases, CAS, migrations, checksum/floor refusal and persistence.
- S3 documentation has eight additional example statements, for **179** total.
  PostgreSQL baseline DDL has **35 tables / 263 statements**; S4 adds twelve
  examples for **275**. PostgreSQL was never executed.
- D19 proposals remain inert: SQLite **26 tables / 139 statements**, PostgreSQL
  **20 tables / 431 statements**. Static checks pass for all **46** relation
  signatures and **104** proposed foreign keys, unique targets/child indexes,
  scoped object names, identifier limits and policy declarations.
- Rust canonical verification passes for **12 containers / 92 embedded byte
  records**, including old and new package archives, requests and the logical export.
  The original package remains 33 files; the CXT3a specimen remains 38.
- Swift/Rust bridge verification passes. Shared UI verification passes with
  **98 snapshots**; production verification passes with **10 snapshots**, including
  action/undo/redo, HDR proxy, Library fixtures, save/reopen and presentation purity.
  Representative Light graph and Dark Layout snapshots were visually inspected.
- The naming guard permits only the exact Cargo flag, Apple framework API and
  explicitly marked removed-path inventory entries;
  retired forward domain vocabulary fails the check. Local Markdown links and
  anchors pass across the repository: **450** checked links.

Repeatable commands:

```sh
cargo test --offline --workspace
cargo check --offline --workspace --all-targets
cargo clippy --offline --workspace --all-targets -- -D warnings
cargo fmt --all --check
cargo build --offline -p photara-core --example fixture_codec
python3 scripts/verify_generation_two_naming.py
python3 scripts/verify_generation_two_fixtures.py
python3 scripts/verify_generation_two_schema.py
CARGO_NET_OFFLINE=true platform/macos/photara-app/verify-bridge.sh
CARGO_NET_OFFLINE=true platform/macos/photara-ui-tests/verify-shared-ui.sh
CARGO_NET_OFFLINE=true platform/macos/photara-ui-tests/verify-production-ui.sh
```

## Next gate

The CXT3a checkpoint passed 204 tests before this phase and preserved the prior
fixtures/migrations at that boundary. This authorized phase rebaselines those
unshipped bytes and recomputes their evidence. PostgreSQL remains text-only;
only fresh disposable SQLite fixtures are used. No live database is opened or
modified, no service or Neon schema is deployed, and nothing is staged/committed/pushed.

Next separate sequence: verified clean rebaseline → CXT3b executable local SQLite
and real app initialization → CXT3c disposable PostgreSQL/RLS → separately
authorized fresh Generation Two Neon deployment → minimum usable Project/UI/node
vertical slice. Current authorization stops before CXT3b and PostgreSQL execution.
