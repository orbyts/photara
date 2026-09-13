# Library rebaseline file and hash inventory

This inventory compares the separately completed CXT3a checkpoint with the
authorized Library nomenclature phase. The [CXT3a record](CXT3A_PACKAGE_READER.md)
separately lists its package implementation files. Generated native build/test
outputs are ignored build artifacts and are not source changes.

## Rebaseline source/document inventory

121 modified files, 9 new files and 3 removed paths.
Removed paths are the three renamed editor source/symlink paths.

- Modified: `CHANGELOG.md`
- Modified: `README.md`
- Modified: `ROADMAP.md`
- Modified: `crates/photara-bridge/src/library.rs`
- Modified: `crates/photara-bridge/src/production.rs`
- New: `crates/photara-core/examples/fixture_codec.rs`
- Modified: `crates/photara-core/src/asset.rs`
- Modified: `crates/photara-core/src/contracts/ids.rs`
- Modified: `crates/photara-core/src/project.rs`
- Modified: `crates/photara-core/tests/contracts.rs`
- Modified: `crates/photara-library/README.md`
- Modified: `crates/photara-library/migrations/generation_two/0001_local_identity.sql`
- Modified: `crates/photara-library/migrations/generation_two/0002_typed_library.sql`
- Modified: `crates/photara-library/migrations/generation_two/0003_catalog_device.sql`
- Modified: `crates/photara-library/migrations/generation_two/0004_mutations_sync.sql`
- Modified: `crates/photara-library/migrations/generation_two/0005_durable_recovery.sql`
- Modified: `crates/photara-library/migrations/generation_two/0006_invariant_guards.sql`
- Modified: `crates/photara-library/src/gen2/catalog.rs`
- Modified: `crates/photara-library/src/gen2/library.rs`
- Modified: `crates/photara-library/src/gen2/mod.rs`
- Modified: `crates/photara-library/src/gen2/tests.rs`
- Modified: `crates/photara-library/src/gen2/types.rs`
- Modified: `crates/photara-library/src/lib.rs`
- Modified: `crates/photara-node-sdk/src/lib.rs`
- Modified: `crates/photara-node-sdk/tests/contracts_v2.rs`
- Modified: `crates/photara-proxy/src/lib.rs`
- Modified: `crates/photara-store/src/package/records.rs`
- Modified: `crates/photara-store/src/package/types.rs`
- Modified: `crates/photara-store/src/package/v1_1/links.rs`
- Modified: `crates/photara-store/src/package/v1_1/records.rs`
- Modified: `docs/ACTIVE_HANDOFF.md`
- Modified: `docs/CODEX_HANDOFF.md`
- Modified: `docs/FRAME_LIBRARY_HANDOFF.md`
- Modified: `docs/LIBRARY_ARCHITECTURE.md`
- Modified: `docs/ROADMAP_0_2_EXECUTION.md`
- Modified: `docs/architecture/ASSETS.md`
- Modified: `docs/architecture/CORE.md`
- Modified: `docs/architecture/CXT1A_CONTRACTS.md`
- Modified: `docs/architecture/CXT1B_CONTEXT_CONTRACTS.md`
- Modified: `docs/architecture/CXT3A_PACKAGE_READER.md`
- Modified: `docs/architecture/D19_CONTRACT_FREEZE.md`
- Modified: `docs/architecture/D19_STATIC_SCHEMA_DELTA.md`
- Modified: `docs/architecture/DISK_NODE.md`
- Modified: `docs/architecture/GENERATION_TWO_FIXTURES.md`
- Modified: `docs/architecture/GENERATION_TWO_PRODUCT_ARCHITECTURE.md`
- Modified: `docs/architecture/LIBRARY_AND_NODE_WORK_SURFACES.md`
- New: `docs/architecture/LIBRARY_NOMENCLATURE_REBASELINE.md`
- New: `docs/architecture/LIBRARY_REBASELINE_INVENTORY.md`
- Modified: `docs/architecture/LOCAL_LIBRARY_IMPLEMENTATION.md`
- Modified: `docs/architecture/LOCAL_SQLITE_SCHEMA.md`
- Modified: `docs/architecture/LOGICAL_DATA_MODEL.md`
- Modified: `docs/architecture/NATIVE_CLIENTS.md`
- Modified: `docs/architecture/NODE_PACKAGES.md`
- Modified: `docs/architecture/PERSISTENCE.md`
- Modified: `docs/architecture/PROJECT_DOCUMENTS.md`
- Modified: `docs/architecture/PROJECT_PACKAGE_CODEC.md`
- Modified: `docs/architecture/PROJECT_PACKAGE_SCHEMA.md`
- Modified: `docs/architecture/README.md`
- Modified: `docs/architecture/SCHEMA_REVIEW.md`
- Modified: `docs/architecture/SERVICE_POSTGRESQL_SCHEMA.md`
- Modified: `docs/architecture/SOCIAL_PROFILES_AND_LIBRARY_EXPORT.md`
- Modified: `docs/architecture/STOREXA_INTEGRATION.md`
- Modified: `docs/architecture/SYNCHRONIZATION_CONTRACT.md`
- Modified: `docs/architecture/THEMES.md`
- Modified: `docs/architecture/TYPED_CONTEXT_AND_EXPRESSIONS.md`
- Modified: `docs/architecture/proposals/d19-cxt2/INVENTORY.md`
- Modified: `docs/architecture/proposals/d19-cxt2/RESPONSIBILITIES.md`
- Modified: `docs/architecture/proposals/d19-cxt2/postgresql/0008_library_project_access.proposal.sql`
- Modified: `docs/architecture/proposals/d19-cxt2/postgresql/0009_storage_locations.proposal.sql`
- Modified: `docs/architecture/proposals/d19-cxt2/postgresql/0010_library_context.proposal.sql`
- Modified: `docs/architecture/proposals/d19-cxt2/postgresql/0011_scoped_sync.proposal.sql`
- Modified: `docs/architecture/proposals/d19-cxt2/postgresql/0012_d19_access_guards.proposal.sql`
- Modified: `docs/architecture/proposals/d19-cxt2/sqlite/0007_library_project_access.proposal.sql`
- Modified: `docs/architecture/proposals/d19-cxt2/sqlite/0008_storage_locations_bindings.proposal.sql`
- Modified: `docs/architecture/proposals/d19-cxt2/sqlite/0009_library_context.proposal.sql`
- Modified: `docs/architecture/proposals/d19-cxt2/sqlite/0010_context_apply_recovery.proposal.sql`
- Modified: `docs/architecture/proposals/d19-cxt2/sqlite/0011_scoped_sync.proposal.sql`
- Modified: `docs/architecture/proposals/d19-cxt2/sqlite/0012_d19_guards_and_floor.proposal.sql`
- Modified: `docs/fixtures/generation-two/README.md`
- Modified: `docs/fixtures/generation-two/canonical-vectors.json`
- Modified: `docs/fixtures/generation-two/context-amendment.json`
- Modified: `docs/fixtures/generation-two/normalization-and-limits.json`
- Modified: `docs/fixtures/generation-two/package-specimen.json`
- Modified: `docs/fixtures/generation-two/records.json`
- Modified: `docs/fixtures/generation-two/scenarios.json`
- Modified: `docs/fixtures/generation-two/social-export.json`
- Modified: `docs/fixtures/generation-two/sync-trace.json`
- Modified: `nodes/photara-asset-set/src/lib.rs`
- Modified: `nodes/photara-disk/src/lib.rs`
- Modified: `nodes/photara-layout/src/lib.rs`
- Modified: `platform/macos/SHARED_UI.md`
- Modified: `platform/macos/photara-app/BridgeVerification/main.swift`
- Modified: `platform/macos/photara-app/README.md`
- Modified: `platform/macos/photara-app/Sources/AppModelLibrary.swift`
- Modified: `platform/macos/photara-app/Sources/ApplicationAdapter.swift`
- New: `platform/macos/photara-app/Sources/EditorSessionModel.swift`
- New: `platform/macos/photara-app/Sources/EditorSessionView.swift`
- Modified: `platform/macos/photara-app/Sources/InspectionAdapter.swift`
- Modified: `platform/macos/photara-app/Sources/PhotaraMacApp.swift`
- Modified: `platform/macos/photara-app/Sources/ProductionGalleryView.swift`
- Modified: `platform/macos/photara-app/Sources/ProductionGraphView.swift`
- Modified: `platform/macos/photara-app/build-app.sh`
- Modified: `platform/macos/photara-app/verify-bridge.sh`
- Modified: `platform/macos/photara-graph/Sources/GraphPreset.swift`
- Modified: `platform/macos/photara-inspector-lab/README.md`
- Modified: `platform/macos/photara-lab-support/Sources/ShellFixtures.swift`
- Modified: `platform/macos/photara-layout/Sources/LayoutAuthoringSurfaceView.swift`
- Modified: `platform/macos/photara-layout/Sources/LayoutCanvasCell.swift`
- Modified: `platform/macos/photara-layout/Sources/LayoutCanvasView.swift`
- Modified: `platform/macos/photara-library-ui/Sources/LibraryBrowser.swift`
- Modified: `platform/macos/photara-library-ui/Sources/LibraryPresentation.swift`
- Modified: `platform/macos/photara-shell-lab/Sources/ShellLabApp.swift`
- Modified: `platform/macos/photara-shell/README.md`
- Modified: `platform/macos/photara-shell/Sources/ApplicationPresentation.swift`
- Modified: `platform/macos/photara-shell/Sources/ApplicationShell.swift`
- New: `platform/macos/photara-shell/Sources/EditorSessionModel.swift`
- Modified: `platform/macos/photara-shell/Sources/ProjectChrome.swift`
- Modified: `platform/macos/photara-shell/Sources/ProjectLauncherView.swift`
- Modified: `platform/macos/photara-theme/Resources/photara-default.json`
- Modified: `platform/macos/photara-theme/Sources/PhotaraTheme.swift`
- Modified: `platform/macos/photara-theme/Sources/ThemeLabApp.swift`
- Modified: `platform/macos/photara-theme/Sources/ThemeLabView.swift`
- Modified: `platform/macos/photara-theme/build-theme-lab.sh`
- Modified: `platform/macos/photara-ui-tests/ProductionUIChecks.swift`
- Modified: `platform/macos/photara-ui-tests/SharedUIChecks.swift`
- Modified: `platform/macos/photara-ui-tests/VERIFICATION.md`
- Modified: `platform/macos/photara-ui-tests/verify-production-ui.sh`
- New: `scripts/verify_generation_two_fixtures.py`
- New: `scripts/verify_generation_two_naming.py`
- New: `scripts/verify_generation_two_schema.py`
- Removed old path: `platform/macos/photara-app/Sources/WorkspaceModel.swift`
- Removed old path: `platform/macos/photara-app/Sources/WorkspaceView.swift`
- Removed old path: `platform/macos/photara-shell/Sources/WorkspaceModel.swift`

## Current synthetic fixture hashes

| Fixture | Bytes | SHA-256 |
| --- | ---: | --- |
| `canonical-vectors.json` | 7561 | `2fc9018d84767d4d63cbb385f60db8cf4dd45fd810dfc038c62007388eb7463d` |
| `context-amendment.json` | 18683 | `5db3146e0aaad53c6e2c52f1d16737531c02d77f2b29426e42d9e0bee3c1e83b` |
| `d19-compatibility.json` | 995 | `dba279e37302c4d2d7d3ddafc13d855ea74dbc640b148d0eb2115cbf227bb3a9` |
| `d19-context.json` | 5824 | `6e49a4156fa664f3844372ccdb1932a04e681abec48b762386a81a1826a6dcd4` |
| `d19-contracts.json` | 3785 | `c4cd563e2a454bb1f8353250ee3bfd38a5c88a825dc0da64677ffcdeb2c0b893` |
| `d19-package-specimen.json` | 43396 | `be32df34ad90d00430dd4fb0e6e475dc2c5c0093d89b261ab8c6fb26a95113b7` |
| `normalization-and-limits.json` | 2682 | `6ef0289b65e9da84d96655287257e29149fe800daca5aa9b75c44cc0111e65b0` |
| `package-specimen.json` | 38924 | `ff83aa26537bc9d134ae0257a4d92870e9b91dfa1c938f5a60bb8086230fdcef` |
| `records.json` | 7331 | `79c4ce72cfbab22d0dad4898c9f19523607800b93a9c305de69c6b8e8c49a741` |
| `scenarios.json` | 17688 | `61185ac18d9a69cd5724adf728cb4550d8e8c489295ceaca6efeba47c47b409b` |
| `social-export.json` | 15753 | `29dde1b14af1e3655db162564fda45845cafadca75d30cfb5c89ac844cf71e0a` |
| `sync-trace.json` | 7057 | `561e0240ddd6881d0e3287eb48d7deb5e4e6cc97f41c2b94c7da03c3403b56d6` |

## Current executable SQLite baseline hashes

These six files are the unshipped baseline, verified only in disposable databases.
SQLx uses SHA-384; SHA-256 is also recorded for ordinary file comparison.

| Migration | Statements | SHA-256 | SQLx SHA-384 |
| --- | ---: | --- | --- |
| `0001_local_identity.sql` | 10 | `b99f33adc424a0aae3e717c972e33d72be5f0ec82f11ac0e7fb4ec28037377fd` | `82c3ecc7b460cf0a3811e6e45e67df58c2ed11c9be4bded390f0c1dda6c9f36525a8b2052d1019f271f63d396e1afd0c` |
| `0002_typed_library.sql` | 27 | `7562bc582d3d8a6a8bc1a8d0e5451c24670763c7d3031b78e903c9960c1416a9` | `c4cc2976a2a1bf064d6354815a64dc235d7a122385b74946ff5935647e20655df129406d82c3e9bd865b9bbdb63a240b` |
| `0003_catalog_device.sql` | 15 | `7331ab5c6742d194fc64eb41557763f92cb93ecdac358aa564d01b82f700fc3e` | `e83df5ee19969325010f218fcf0f56b8d8a812d2192394cba1442fdfb1bbfe89bced6b4a8c9e8ecc057992c9dbba47d5` |
| `0004_mutations_sync.sql` | 19 | `72c5d345b3a93ee6a5813b8000afd2f33fc614eb45109f2d37f310225148cf4f` | `87bfda6a54a7fc6a881d5746efe9cfb54d58e543410f4150105749fc968a8864385de203a9e6fec00c6bb1573f8350d3` |
| `0005_durable_recovery.sql` | 5 | `d0c97f3f49bbcb1581aac714ff5798d90200039867022d01ffed6e18bf59b3fb` | `c61fd241446cd7fd0cab4d6cc306e806b5bbe7cf6c64014c86cafab03bc4ca95c2a3d7c7a898c70843526923c7fb8891` |
| `0006_invariant_guards.sql` | 95 | `85ff08feb48040eb5bcd1324e275e5593ac98cd4ba2b826e66d867eecfdfd8de` | `dbf4c7ccd0b9e9e6665f92a9e3a7a060cf5967b794d087b2d78d4ee339d966fa097a6025a818825621223fa5ced631b5` |
