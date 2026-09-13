# CXT3c implementation and verification inventory

Baseline: clean synchronized `7e6d794`; uncommitted/unpushed.

## Preservation

297 existing code/platform/fixture files are byte-for-byte unchanged, including
all 42 Graph/Graph Lab files, every UI source, Core/SDK/package/local runtime,
all local migrations and all Generation Two fixtures. Only root Cargo manifests
and the listed documentation change among the initial 376 files.

The real local database was read only as bytes, never opened: `/Users/suhail/Library/Application Support/Photara/State/photara-local-v2.sqlite`.
Size: 1179648 bytes. SHA-256 before and after all native gates:
`ece4b613e678da9f4c2bf2f676db641547b2a49f29c6b4f71026f556a854c31d`. No v0.1.3 data or external storage/provider was used.

## Exact migrations

All 0001–0011 SQL tokens match the baseline/proposals. 0012 differs only in
the documented `CREATE OR REPLACE` spelling; 0013 adds three runtime grants.
All 13 stored SQLx SHA-384 checksums match the executable files. The two explicit
bootstrap INSERTs are controller initialization statements, outside the 697-file
statement total. See [all measured engine objects](CXT3C_POSTGRES_INVENTORY.json).

| Migration | Statements | SHA-256 |
| --- | ---: | --- |
| `0001_identity_library.sql` | 15 | `6fad375e9eb6a892c29848fb2c68bdd57d244bc5e760687756dd02deaf16334d` |
| `0002_entitlements_media.sql` | 11 | `796515d975de7accd53b57c49b4700d55a0f68003b31ea46d5fd0921f4bc72b9` |
| `0003_typed_library.sql` | 29 | `60276682ab7d7b8268da81893b214eadb67ce2d4e917cdc49dbaebc62a2ce28a` |
| `0004_catalog.sql` | 8 | `afaf7a42f0f006b83484848f43478873243921c8b721c4aed4ca946caca9c7a2` |
| `0005_mutations_feed.sql` | 9 | `dbea00bae9d1411d290f65c8e927f0784363b3ff6abec8072441f06c31527416` |
| `0006_guards.sql` | 91 | `83d7be3b40dbc347c639aead78891163f2769e220c599b5687552d357d4a1252` |
| `0007_privileges.sql` | 100 | `f7aebf23dca7753b776c6917697895c847c12d8e7e2f1dd7f3ed6975b1316728` |
| `0008_library_project_access.sql` | 15 | `62fedcc53017b273a1fad5594988fd71dfd0b368fe56771edbb7a928c55cb20f` |
| `0009_storage_locations.sql` | 6 | `a44b2ff69b775e93f2b353481cd1380ebf007d497b7bff7ec19b539b8557296f` |
| `0010_library_context.sql` | 13 | `6a81e63b3ae9f0042568558c30209c68748bac6c58af41938ed55e03c74223ef` |
| `0011_scoped_sync.sql` | 23 | `22b9d26c9b7f640ca02ee6c6d4754fdc753b4c87cfe6fbd3b398233a8a290c33` |
| `0012_d19_access_guards.sql` | 374 | `e2ea1a2c256a7566429a6a728e7a0640b0fe53600fdccfe1f21b140b94aec596` |
| `0013_runtime_boundary.sql` | 3 | `8b7970566e82b2de4cd1fa4be81165be4b3d13f6716375e99c04ef9a9cd09294` |

## Source and artifact changes

| Path | Status | SHA-256 |
| --- | --- | --- |
| `Cargo.lock` | modified | `049de085ee464db1fc1fa1c2aa61368f21380ffe70fd86c9faa6abfd2c9b10b4` |
| `Cargo.toml` | modified | `7f1cae5242e88e62f3589ec10c4ac712bdf24ec32808ee1f9b14ee0763fa744d` |
| `crates/photara-service/Cargo.toml` | added | `37b129155aeef5872bc9c38bbba0973c5bc6ae82eb5b97f0e2678140e59e61fe` |
| `crates/photara-service/examples/migrate_disposable.rs` | added | `e4568ca782efa5ab6230e79fa1110608e03d6729047028a19a87719d09d7534f` |
| `crates/photara-service/migrations/postgres/0001_identity_library.sql` | added | `6fad375e9eb6a892c29848fb2c68bdd57d244bc5e760687756dd02deaf16334d` |
| `crates/photara-service/migrations/postgres/0002_entitlements_media.sql` | added | `796515d975de7accd53b57c49b4700d55a0f68003b31ea46d5fd0921f4bc72b9` |
| `crates/photara-service/migrations/postgres/0003_typed_library.sql` | added | `60276682ab7d7b8268da81893b214eadb67ce2d4e917cdc49dbaebc62a2ce28a` |
| `crates/photara-service/migrations/postgres/0004_catalog.sql` | added | `afaf7a42f0f006b83484848f43478873243921c8b721c4aed4ca946caca9c7a2` |
| `crates/photara-service/migrations/postgres/0005_mutations_feed.sql` | added | `dbea00bae9d1411d290f65c8e927f0784363b3ff6abec8072441f06c31527416` |
| `crates/photara-service/migrations/postgres/0006_guards.sql` | added | `83d7be3b40dbc347c639aead78891163f2769e220c599b5687552d357d4a1252` |
| `crates/photara-service/migrations/postgres/0007_privileges.sql` | added | `f7aebf23dca7753b776c6917697895c847c12d8e7e2f1dd7f3ed6975b1316728` |
| `crates/photara-service/migrations/postgres/0008_library_project_access.sql` | added | `62fedcc53017b273a1fad5594988fd71dfd0b368fe56771edbb7a928c55cb20f` |
| `crates/photara-service/migrations/postgres/0009_storage_locations.sql` | added | `a44b2ff69b775e93f2b353481cd1380ebf007d497b7bff7ec19b539b8557296f` |
| `crates/photara-service/migrations/postgres/0010_library_context.sql` | added | `6a81e63b3ae9f0042568558c30209c68748bac6c58af41938ed55e03c74223ef` |
| `crates/photara-service/migrations/postgres/0011_scoped_sync.sql` | added | `22b9d26c9b7f640ca02ee6c6d4754fdc753b4c87cfe6fbd3b398233a8a290c33` |
| `crates/photara-service/migrations/postgres/0012_d19_access_guards.sql` | added | `e2ea1a2c256a7566429a6a728e7a0640b0fe53600fdccfe1f21b140b94aec596` |
| `crates/photara-service/migrations/postgres/0013_runtime_boundary.sql` | added | `8b7970566e82b2de4cd1fa4be81165be4b3d13f6716375e99c04ef9a9cd09294` |
| `crates/photara-service/src/access.rs` | added | `fb8efa1d53f5d35d9935c683f6f2f3d40cea0764198381a6d13a1aef18964236` |
| `crates/photara-service/src/auth.rs` | added | `01b472245015b6e286e1fa976dbfde241b025d7b3694fcd7777a09dbf0995806` |
| `crates/photara-service/src/fake_sync.rs` | added | `29456531e1f699e9b743860ca76ee67671c926b1dc7f5a909bbfbc8c983051fa` |
| `crates/photara-service/src/lib.rs` | added | `ba18e139a3cdb46803156c85cf38aa7ed2e1cade6ff4e11e752f0c6c60b99a7c` |
| `crates/photara-service/src/management.rs` | added | `91c0386958a11597c952979fd448ef06b75caf910a8c01577e81c7bdbfe2d62c` |
| `crates/photara-service/src/media.rs` | added | `96838be21dfbb2600f03acbf35a404fe79cfe18dd1d89251a7885f58d7b3e712` |
| `crates/photara-service/src/pgtests.rs` | added | `2fb36d2f5e3aab31d617d71260741c6481aaa7b2a9c4b02287280ad143df90a1` |
| `crates/photara-service/src/runtime.rs` | added | `6a2edd80e3f0b1eaf1e62a70784d4a3d28162435c9ead791c0bc140d9e9cebb9` |
| `crates/photara-service/src/sync.rs` | added | `de863b197a0faf2c4980378e6e74528293be12b70ec8ae01e8baeba34e8ee274` |
| `docs/architecture/CXT3C_POSTGRES_INVENTORY.json` | added | `661ee574003e5b48e34cc7038c1e71e003533e1c4bcd09930b4fc4a1bebcafc9` |
| `scripts/verify_service_postgres.py` | added | `e950139db5ea6ba25091b1f173a9fdc88c3eb25f116028b5947d253f7c54e426` |

## Documentation changes

- `docs/ACTIVE_HANDOFF.md`
- `docs/ROADMAP_0_2_EXECUTION.md`
- `docs/architecture/CXT3C_IMPLEMENTATION_INVENTORY.md`
- `docs/architecture/CXT3C_SERVICE_RUNTIME.md`
- `docs/architecture/SERVICE_POSTGRESQL_SCHEMA.md`
- `docs/architecture/STOREXA_INTEGRATION.md`

## Verification results

| Gate | Result / evidence |
| --- | --- |
| Full offline Rust tests and doctests | 258 passed, 0 failed, 9 ignored; `/private/tmp/photara-cxt3c-tests.log` |
| Disposable PostgreSQL | Five suites passed, 678 counted access/privilege/sensitivity cells; `/private/tmp/photara-cxt3c-postgres-final.log` |
| All-target check / Clippy | Passed with warnings denied; `/private/tmp/photara-cxt3c-check.log`, `/private/tmp/photara-cxt3c-clippy.log` |
| fmt / whitespace | `cargo fmt --all --check`; `git diff --check` |
| Static schema | 46 proposal signatures / 104 scoped FKs; unchanged accepted inventory |
| Naming / fixtures | Naming guard passes; 12 canonical containers / 92 embedded records pass Rust codec |
| Native bridge | Passed; revision 19, 3 progress and 2 cancellation callbacks; `/private/tmp/photara-cxt3c-bridge.log` |
| Production UI | Passed; 10 top-level snapshots, real startup under disposable state override; `/private/tmp/photara-cxt3c-production-ui.log` |
| Graph Lab first run | 15,050 assertions / one randomized prerequisite failure; `/private/tmp/photara-cxt3c-graph.log` |
| Graph exact replay | 548 assertions / zero failures; `/private/tmp/photara-cxt3c-graph-replay.log` |
| Graph full rerun | 15,586 assertions / zero failures; `/private/tmp/photara-cxt3c-graph-full-retry.log` |

The five PostgreSQL suites are ignored by ordinary offline runs because they require
explicit disposable role provisioning. The runner actually executed all five; the
remaining four ignored tests are preexisting explicit fixture generators.
The Graph failure seed was `22597503690035777`, Light/straight, 180 actions;
the initial failure and both successful runs are retained without changing Graph code.
Light Graph and Dark Library production snapshots were visually inspected.

A positive entitlement fixture initially used `reason` instead of the actual
`reason_code` column. That test failed, the runner still stopped its cluster,
and the corrected final clean run passed. Evidence:
`/private/tmp/photara-cxt3c-7zftnrjl/proof.log` (failure) and the final log above.

All disposable PostgreSQL clusters were stopped, including the initial manual
cluster. No network listener, real provider integration or deployed service remains.
The [runtime record](CXT3C_SERVICE_RUNTIME.md) states codec/adapter limits and the
minimal separately gated Neon role/schema/explicit seed plan.
