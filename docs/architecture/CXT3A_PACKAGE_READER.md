# CXT3a package reader and explicit source mappings

Selected by Suhail on 2026-09-12 from clean checkpoint `44f397f`. The package
slice is separate from the subsequently authorized Library nomenclature rebaseline.
This record describes the reader boundary; the rebaseline record governs later
changes to the unshipped schemas and their fixture bytes.

## API and immutable ownership

`photara_store::package::v1_1::validate_directory(root, limits)` returns
`ValidatedPackageV1_1`: original bootstrap/HEAD/commits, exact authored/graph objects,
all verified canonical JSON bytes, verified managed blob count and diagnostics.
The existing `package::validate_directory` and its typed result remain the 1.0
entry point during this slice. Neither API publishes, evaluates a graph, opens a
Library database, resolves external bytes, grants access or applies a proposal.

The shared descriptor-relative, no-follow reader retains its limits, strict JSON
and canonical codec. The new reader accepts bootstrap 1.0 or 1.1 with supported
per-commit floors, verifies retained ancestor closures, and requires D19 features
when their records occur. Features accumulate across retained commits. The 1.0
reader refuses a 1.1 HEAD even when its immutable bootstrap is 1.0.

Closure follows declared fields, typed values and verified AST literal types.
Source strings, opaque node configuration, evidence payloads and namespaced
extensions do not acquire filesystem meaning by resembling an ObjectRef. Unknown
required schemas/features prevent validation; the original files are not rewritten.
External descriptors remain unresolved; managed blobs must exist and hash exactly.

New checks cover Project/Library/Graph/Node ownership, exact node/manifest pins,
representation/content/resource identity, explicit AssetSet page ordinals and
semantic digests, metadata observations and preconditions, groups/subsets,
expression source/AST correspondence, variable types/name claims/cycles, snapshots,
privacy labels, and immutable history/proposal/receipt links. An unknown receipt
remains unknown even when its Run succeeded. Current access and runtime resolution
remain outside this read-only package proof.

## Wire bindings to CXT1 contracts

The accepted [D19 object table](D19_STATIC_SCHEMA_DELTA.md#package-object-delta-and-closure)
selects schemas and edges. The following concrete bindings reuse the existing
checked Core types, rather than duplicating expression or context semantics:

- Expressions use the CXT1b record fields, with `source_utf8` → `source` and
  `bound_dependencies` → `dependencies`; `owner` must equal `environment.owner`.
- Variables use `names` for retained `claimed_names`. `current_value` contains
  `value_id`, `binding`, `origin` and optional provenance. Expression binding values
  are ObjectRefs; the reader resolves them to checked expression records before
  validating the aggregate. Literal binding values are CXT1b TypedValues.
- Context snapshots flatten SnapshotSpec and retain `content_digest`, `completeness`
  and `replayability`. Separate `captures` and `expressions` Ref arrays retain
  immutable AssetSet manifests and expression records. These package edges do not
  change the CXT1b semantic digest. Captured metadata cannot reduce its source labels.
- AssetSet pages store the semantic ordered members without a redundant ordinal.
  Their representation coordinates gain the containing Project/Asset IDs during
  checked Core construction; `metadata_refs` resolve exact observations. The manifest
  `content_digest` uses the unchanged CXT1a ordered-member formula.
- Managed resource `media` includes MIME and decimal byte length; the blob must match.
  External resource fields decode as ExternalResourceRevision with optional provenance.
- `photara.node.manifest` v1 is the owned package envelope for a CXT1a manifest v2
  under `manifest`. Node contract digests hash the exact canonical definition record;
  wrapper/package/definition pins must agree. Missing manifests remain inspection
  diagnostics, never executable fallbacks.
- Proposal/receipt fields reuse the checked CXT1b specifications. Apply receipt request
  digests must match their retained proposal. A containing commit is publication
  provenance outside the receipt's own hashed object.

Internal structural projections reuse the existing graph/assignment/history
invariant checks. They are temporary in-memory views and never replace original
object bytes or become a conversion/publication API. Resource and AssetSet version
checks occur against the actual 1.1 objects before those shared checks.

`package::compatibility` contains explicit identity, chosen-owner, unresolved/mapped
external resolver, selected-source snapshot and exact-definition presentation DTOs.
These are source mapping facts, not storage aliases, authority or migration workers.
The later user correction supersedes R1's physical-name preservation: the forward
Generation Two baseline must use Library naming; a v0.1.3 importer is optional and
non-gating. No dual writes or compatibility migration were added here.

## Files and verification

New Rust:

- `crates/photara-store/src/package/compatibility.rs`
- `crates/photara-store/src/package/v1_1/mod.rs`
- `crates/photara-store/src/package/v1_1/reader.rs`
- `crates/photara-store/src/package/v1_1/records.rs`
- `crates/photara-store/src/package/v1_1/closure.rs`
- `crates/photara-store/src/package/v1_1/links.rs`
- `crates/photara-store/tests/package_v1_1.rs`
- `crates/photara-store/tests/package_compatibility.rs`

Existing Rust edits: package `mod.rs` adds dispatch and an internal immutable object
lookup view; package `records.rs` accepts that view for its existing link checks.
New fixtures: `docs/fixtures/generation-two/d19-package-specimen.json` (38 files)
and `docs/fixtures/generation-two/d19-compatibility.json`.
Documentation: this record, `docs/ROADMAP_0_2_EXECUTION.md`, `docs/ACTIVE_HANDOFF.md`.

Offline CXT3a test evidence: **204 passed**: 159 retained Core/SDK/node/doctests,
25 retained package tests, 15 new package tests and five mapping tests. Matrix tests
include 22 rehashed semantic mutations, five membership/precondition mutations,
five history scope mutations, five missing-feature cases and three receipt/manifest
mutations. The two explicit ignored generators were separately run successfully;
normal tests compare their exact Rust canonical output. Offline library all-target
compilation, Clippy with `-D warnings`, `cargo fmt --all --check` and `git diff --check` pass. All package filesystem
activity uses fresh `photara-cxt3a-*` or retained disposable fixture roots.

Pre-rebaseline SHA-256:

- Package specimen: `be32df34ad90d00430dd4fb0e6e475dc2c5c0093d89b261ab8c6fb26a95113b7`
- Mapping fixture: `dba279e37302c4d2d7d3ddafc13d855ea74dbc640b148d0eb2115cbf227bb3a9`

Before documentation updates, 343 of 345 checkpoint files were byte-identical;
only the two named existing Rust files changed. This includes all eleven previous
fixture files, the old 33-file specimen, six L2 migrations, all 14 inert CXT2 files,
Core/SDK/CXT1 source and records, Cargo files and UI. No SQL/database, provider,
service, host adapter, publisher, staging, commit or push occurred in CXT3a.

The subsequent explicitly authorized rebaseline may revise unshipped Generation Two
schema and fixture bytes. Its regenerated evidence must be recorded separately;
the pre-rebaseline preservation counts above are historical CXT3a evidence.
