# CXT3b exact local inventory

Baseline: clean `415039e`. This slice is uncommitted/unpushed.

**129 protected files unchanged:** Core, NodeSDK, Store/package reader, node
packages, all 42 Graph/Graph Lab files, all 12 fixtures and six baseline migrations.

## Executable migrations

| File | Statements | SHA-256 | SQLx SHA-384 |
| --- | ---: | --- | --- |
| `0001_local_identity.sql` | 10 | `b99f33adc424a0aae3e717c972e33d72be5f0ec82f11ac0e7fb4ec28037377fd` | `82c3ecc7b460cf0a3811e6e45e67df58c2ed11c9be4bded390f0c1dda6c9f36525a8b2052d1019f271f63d396e1afd0c` |
| `0002_typed_library.sql` | 27 | `7562bc582d3d8a6a8bc1a8d0e5451c24670763c7d3031b78e903c9960c1416a9` | `c4cc2976a2a1bf064d6354815a64dc235d7a122385b74946ff5935647e20655df129406d82c3e9bd865b9bbdb63a240b` |
| `0003_catalog_device.sql` | 15 | `7331ab5c6742d194fc64eb41557763f92cb93ecdac358aa564d01b82f700fc3e` | `e83df5ee19969325010f218fcf0f56b8d8a812d2192394cba1442fdfb1bbfe89bced6b4a8c9e8ecc057992c9dbba47d5` |
| `0004_mutations_sync.sql` | 19 | `72c5d345b3a93ee6a5813b8000afd2f33fc614eb45109f2d37f310225148cf4f` | `87bfda6a54a7fc6a881d5746efe9cfb54d58e543410f4150105749fc968a8864385de203a9e6fec00c6bb1573f8350d3` |
| `0005_durable_recovery.sql` | 5 | `d0c97f3f49bbcb1581aac714ff5798d90200039867022d01ffed6e18bf59b3fb` | `c61fd241446cd7fd0cab4d6cc306e806b5bbe7cf6c64014c86cafab03bc4ca95c2a3d7c7a898c70843526923c7fb8891` |
| `0006_invariant_guards.sql` | 95 | `85ff08feb48040eb5bcd1324e275e5593ac98cd4ba2b826e66d867eecfdfd8de` | `dbf4c7ccd0b9e9e6665f92a9e3a7a060cf5967b794d087b2d78d4ee339d966fa097a6025a818825621223fa5ced631b5` |
| `0007_library_project_access.sql` | 14 | `039818c41830ca5da9ab0495034bb69b2d33295c8b94b462b3e0029fe4bac78e` | `2aa86c50aca1f3156b55ba762525c0a2c67f6367c954db095a5e791b581d3d18d800a673ed24775e9c8ad73c007075fb` |
| `0008_storage_locations_bindings.sql` | 12 | `66500faa616fb2e218472262072846a932ff1e086981eb5195fee614e7a2cb5f` | `ae6690aef2ef48d410a65c7fdc2d6055af81429b6eb1376a28cc2d40e72561117ea3685164377d1cac919bb8751143b6` |
| `0009_library_context.sql` | 12 | `de50a9cd855ab3f48529b90545e2df62f92ee8e1a9950213c360d5cd91d6568f` | `382c79ebb966325db8d6ede8ef9cb4e8fcb8d577a21b509d52f895b2b7fbae050c023d3a656456955ddee0574f4bc946` |
| `0010_context_apply_recovery.sql` | 7 | `84cbe8d71a02ad7b491af5a0ec425dd22a566a82de8cc3a2eb5d883536f02aed` | `55aacce3473464a084c594ec96752f9e6775863dadf686315d42473f4f3fa0a10182a5e3bfb4a08ac815065066cd7b53` |
| `0011_scoped_sync.sql` | 19 | `d52526b340caff095f16b9c335c4cf824915064eefccd966568afdf1f3f92d15` | `0706379386a8ec7875effd1afa045ac81d0d3efcbd9243161e1aa52b71d07524747141d4d1ba916b7dc06f45615e31cf` |
| `0012_d19_guards_and_floor.sql` | 75 | `dedcab39b5511922465061b107fa282bcfb046d625833a1cbed05c10163e0f50` | `802709b823c83348be16cd86ab4973b0a28bb674e1c2dcde1d598811f81108cc47a0335f5316b3af81acb04cb7811f5d` |

**310 statements; 70 tables; 68 explicit indexes; 169 triggers.**
Every 0007–0012 SQL token matches its accepted proposal body; only status comments differ.
Implicit PK/UNIQUE indexes and SQLx bookkeeping are excluded from domain totals.

## Closed production database inspection

Path: `/Users/suhail/Library/Application Support/Photara/State/photara-local-v2.sqlite`.
Created through the same UniFFI initializer called by the real AppModel.
No prior file existed. Mode 0600; exactly one active My Library/local controller.
The closed, checkpointed file was inspected read-only with immutable mode.
All 12 stored SQLx SHA-384 checksums match the executable files above.
Family photara.local.g2; epoch 1; user_version 2; application_id 1346917426;
minimum_reader/minimum_writer 2/2; integrity_check ok; zero FK violations.

## Changed source and build files

| Path | Status | SHA-256 |
| --- | --- | --- |
| `Cargo.lock` | modified | `c73d779ea4f308895b49eca132208b3d1886b41389f0a4fdeec99aaa51aa3e3c` |
| `crates/photara-bridge/Cargo.toml` | modified | `26452655d35015448d4e6a07ab49c169260c31d04aebe2dfd55c8e7bd8233aa5` |
| `crates/photara-bridge/src/lib.rs` | modified | `205bcee587e3f03af83542c2ada91e9ac8988f49055e548336b5f332acc31a0e` |
| `crates/photara-bridge/src/local_state.rs` | added | `89ddc3b9b9c70fc9a2dc27bf2eae52382c0709cb95666ef63c1c6f4d1e1bd087` |
| `crates/photara-library/migrations/generation_two/0007_library_project_access.sql` | added | `039818c41830ca5da9ab0495034bb69b2d33295c8b94b462b3e0029fe4bac78e` |
| `crates/photara-library/migrations/generation_two/0008_storage_locations_bindings.sql` | added | `66500faa616fb2e218472262072846a932ff1e086981eb5195fee614e7a2cb5f` |
| `crates/photara-library/migrations/generation_two/0009_library_context.sql` | added | `de50a9cd855ab3f48529b90545e2df62f92ee8e1a9950213c360d5cd91d6568f` |
| `crates/photara-library/migrations/generation_two/0010_context_apply_recovery.sql` | added | `84cbe8d71a02ad7b491af5a0ec425dd22a566a82de8cc3a2eb5d883536f02aed` |
| `crates/photara-library/migrations/generation_two/0011_scoped_sync.sql` | added | `d52526b340caff095f16b9c335c4cf824915064eefccd966568afdf1f3f92d15` |
| `crates/photara-library/migrations/generation_two/0012_d19_guards_and_floor.sql` | added | `dedcab39b5511922465061b107fa282bcfb046d625833a1cbed05c10163e0f50` |
| `crates/photara-library/src/gen2/context.rs` | added | `e4e43abcbe6510aeecff419f9cc0e29c11c0ec0984d5e23c7c6de4d9f7fed29b` |
| `crates/photara-library/src/gen2/local.rs` | added | `6674851ba345520d9f274853ca2ba9afa143a11ceba8e7b8a20104706807a180` |
| `crates/photara-library/src/gen2/mod.rs` | modified | `83283921f34841b6b8cd22451afddf0a497959a32d1454b45f80f9094745a8bd` |
| `crates/photara-library/src/gen2/recovery.rs` | added | `187d30002b839f022fc1826a6d4deb823e4fb7a6edb09f443ac2f83f92057ed1` |
| `crates/photara-library/src/gen2/scoped.rs` | added | `cb83d4c632e4c019bf285ae299f990b79a08334a1b2b415a3747c4bc5caa4674` |
| `crates/photara-library/src/gen2/storage.rs` | added | `32f47a1f70e0b0679509771c042e2573adea5cb1e30d82c253e914c20802fd60` |
| `crates/photara-library/src/gen2/tests.rs` | modified | `61f75d7e5cb42e5faf45aee6d91c9fa8275ae9e68f6ecf0e77198b0feb656eeb` |
| `crates/photara-library/src/gen2/types.rs` | modified | `3855b8c922bb9b6eb113664471ed81e0a162d0bfecf258b4e7b4c436e60072c9` |
| `platform/macos/photara-app/BridgeVerification/main.swift` | modified | `fdf7bf358ffa39864e562071efc5281b11c9f19a136d3db7543c21f2a28bc695` |
| `platform/macos/photara-app/Sources/AppModel.swift` | modified | `e196fb52c357d926119cd5b72fa7b6fbc85df433815c6440fa92202de561d9ed` |
| `platform/macos/photara-ui-tests/ProductionUIChecks.swift` | modified | `db1cc4bc53207ec7f3777a7645b292fbfb6dc9d884d265a855d6a43a5e3d15ab` |

## Documentation changes

Document paths are listed without self-referential content hashes.
- `crates/photara-library/README.md`
- `docs/ACTIVE_HANDOFF.md`
- `docs/ROADMAP_0_2_EXECUTION.md`
- `docs/architecture/CXT3B_LOCAL_RUNTIME.md`
- `docs/architecture/proposals/d19-cxt2/INVENTORY.md`
- `docs/architecture/proposals/d19-cxt2/README.md`
- `docs/architecture/CXT3B_LOCAL_INVENTORY.md`

## Verification logs

- `/private/tmp/photara-cxt3b-tests.log`: 258 passed / 0 failed / 4 ignored.
- `/private/tmp/photara-cxt3b-check.log`, `photara-cxt3b-clippy.log`: passed offline.
- `/private/tmp/photara-cxt3b-bridge.log`: native bridge passed.
- `/private/tmp/photara-cxt3b-production-ui.log`: production UI/startup passed.
- `/private/tmp/photara-cxt3b-shared-ui.log`: shared UI passed.
- `/private/tmp/photara-cxt3b-graph.log`: first run, 15,430 assertions / one failure.
- `/private/tmp/photara-cxt3b-graph-replay.log`: failing seed replay, 541 / zero.
- `/private/tmp/photara-cxt3b-graph-full-retry.log`: full rerun, 15,586 / zero.

The [runtime record](CXT3B_LOCAL_RUNTIME.md) describes scope and inspection commands.
