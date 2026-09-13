# L1 read-only package codec and validator

**Current baseline — 2026-09-12:** the user superseded D19 R1 physical-name
preservation because Generation Two is unshipped. Domain, package and SQL names
below use Library consistently. This is a clean baseline rewrite, with no rename
migration, alias, shadow column or live database change. See the
[rebaseline authority and evidence](LIBRARY_NOMENCLATURE_REBASELINE.md) and
[current execution order](../ROADMAP_0_2_EXECUTION.md). CXT3a is complete; local
D19 execution/app initialization and PostgreSQL/RLS remain separate CXT3b/c gates.

## API and ownership

The additive [`photara_store::package`](../../crates/photara-store/src/package/mod.rs)
module owns filesystem/codec validation. Core, its canonical encoder, existing
ProjectDocument import/export, repositories and current FileSystemStateStore are
unchanged. Storexa remains the later database boundary; no SQL was introduced.

- `parse_json(bytes, JsonLimits)` rejects duplicate decoded object keys before
  constructing Value, malformed UTF-8/BOM, unpaired surrogate escapes, non-finite
  numbers, out-of-range integer lexemes and trailing non-whitespace input.
- `parse_canonical_json` additionally compares exact retained Core
  `photara.canonical-json.v1` bytes. Ordinary JSON whitespace is accepted only by
  `parse_json`; canonical input allows no extra whitespace/newline. Arbitrary
  configuration floats retain the existing finite serde_json semantics.
- `inspect_bootstrap(root, limits)` offers bounded header inspection while
  preserving future feature/version values. It does **not** validate a package.
- `validate_directory(root, PackageLimits)` verifies one HEAD and every retained
  ancestor, returning typed bootstrap/HEAD/commits/authored/Graphs, original JSON
  values and bytes, verified blob count and explicit diagnostics.
- `validate_resource_path` checks portable relative hints without resolving them.
  Typed UUID/digest/decimal-string/ObjectRef/schema envelopes validate spelling and
  bounds. Known record fields are checked separately from optional opaque data.

Object discovery follows schema-selected reference fields, never arbitrary
reference-shaped values inside node configuration/evidence/extensions. Generated
internal paths are fixed ASCII shapes; SHA-256 and exact byte lengths cover all
JSON objects and every managed blob. Inventories must equal transitive closures,
including historical source authored roots; parent IDs/digests, root revision,
revision increments, bootstrap checksum and HEAD linkage are checked.

Known authored checks include exact Graph/node pins and Core graph validity,
assignment/snapshot ownership, required LocationKind snapshots, asset membership,
representation/resource links and media numeric bounds. Known history checks
include source Graph/config digests, Run/Attempt/Operation identities, attempt
ordinals and evidence/receipt/outcome linkage. UUIDs are strict canonical lowercase;
u64 counters use canonical decimal strings, typed u32 values reject float/string/
overflow forms, positive counters reject zero, and timestamps use checked UTC
calendar dates with exactly millisecond precision.

## Filesystem and compatibility limits

Unix reads use safe rustix descriptor-relative `openat` with no-follow on every
component, exact directory-entry spelling, regular-file/single-link checks,
before/after metadata checks and final bootstrap/HEAD rereads. Symlink/hardlink/
nonregular internal files fail closed. Non-Unix returns `UnsupportedPlatform`.
This reduces path races but is not a writer lock, coherent adversarial filesystem
snapshot or SMB durability proof. Callers must not interpret it as one.

Relative external hints reject traversal, empty/dot components, absolute/drive
forms, backslashes, controls, trailing dot/space and reserved first-component
ASCII aliases. They are never followed; a future resolver must enforce host-specific
Unicode/case aliases and containment again. Local limits bound depth, members,
arrays, objects, commits, JSON memory and streamed blob bytes; callers can lower
them. Errors contain categories only, not paths, provider payloads or raw input.

Unknown optional fields and original canonical bytes remain available, never
rewritten. Unknown required features/schema/codec/minimum-reader values fail closed
with unsupported errors; safe bootstrap inspection remains separate. Supported
envelopes are the version-one S6 control, authored, assignment/snapshot, asset/
resource, Graph and immutable history shapes. Other shapes are not a promise of
compatibility merely because their JSON parses.

Explicit limitations:

- Full Unicode 16 graph-name normalization is not implemented. ASCII duplicate
  names are rejected; non-ASCII names produce
  `GraphNameNormalizationUnsupported`, so no uniqueness/write-readiness claim is
  made. A caller must inspect diagnostics. There is no writer API to bypass this.
- The specimen's exact package requirements/pins are checked, but its manifests
  are absent. `NodeManifestUnavailable` records that executable/category/manifest
  availability or trust has not been verified. Embedded NodeSDK manifests and
  future envelope variants are not supported by this initial project-record reader.
- External bytes are not opened and produce `ExternalResourceNotResolved`.
  Hash-verified managed bytes do not establish provider state or effect success.
- No package writer/converter, publication/lease/recovery logic, catalog, database,
  cloud/auth/provider, UI or application opener integration was added. Existing
  one-JSON APIs remain the current application boundary, not automatically migrated.

## Verification and next gate

[`package_validation.rs`](../../crates/photara-store/tests/package_validation.rs)
materializes trusted inert fixture entries only under fresh tempfile roots. It
rehashes mutation fixtures so semantic tests reach record checks rather than only
checksum rejection. All 25 focused tests pass: nine exact canonical vectors,
malformed/duplicate JSON and budgets, tampering, unsafe paths/symlinks/hardlinks,
unknown features/versions, closure/parent/bootstrap failures, typed bounds and
linkages, unknown-field retention and existing document round trips. The valid
33-file specimen remains unchanged after reading: 28 JSON objects, two commits,
two named Graphs and one managed blob.

Also passed offline: 24 Core tests, three NodeSDK tests, two retained store tests,
their doc tests, store all-target Clippy `-D warnings`, fmt check and whole-repository
all-target compile. No database was opened. Library tests were intentionally
not executed because Library tests create databases. These results do not certify
unimplemented schema, service, writer, erasure, export or SMB scenario families.

Subsequently, roadmap **L2** was separately authorized and completed: see
[the local Library implementation](LOCAL_LIBRARY_IMPLEMENTATION.md). As of
2026-09-12, [D19 consistency and contract freeze](LIBRARY_AND_NODE_WORK_SURFACES.md),
static CXT2 review and revised CXT1/CXT3 precede L3, which is paused.
Current L1 supports no D18 feature/AST/context envelope. CXT2 must freeze
exact required features and closure before CXT3 changes this codec. Subsequent
L3 creation still requires its own scope and temporary roots.
L3/L4 retain conversion/publication and broader record integration. No automatic
continuation, live project access, staging/commit, cloud deployment or UI work is
authorized by this reader's completion.
