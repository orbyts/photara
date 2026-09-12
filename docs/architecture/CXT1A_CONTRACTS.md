# CXT1a — portable Core and NodeSDK contracts

Status: **complete, 2026-09-12**, following Suhail's explicit selection of CXT1a.
This records the implemented extent of the [accepted D19 contract](D19_CONTRACT_FREEZE.md)
and [bounded gates](D19_STATIC_SCHEMA_DELTA.md#fixture-delta-implementation-slices-and-gates).
CXT2 remains an inert SQL proposal. CXT1b is the next separately selected slice;
CXT3a/b/c and L3 have not begun.

## Public API and compatibility

The additive entry points are `photara_core::contracts` and `photara_node_sdk::v2`.
No old export is replaced. The existing application API remains version 1; the
new response metadata identifies `CONTRACT_API_VERSION = 2`. Legacy manifests,
AssetSet v1, evaluation/cache keys, package codecs and installed nodes keep their
existing behavior. No dependency or Cargo lockfile change belongs to this slice.

| Surface | Implemented contract |
| --- | --- |
| [IDs](../../crates/photara-core/src/contracts/ids.rs) | 35 distinct nonnil UUID types; canonical lowercase hyphenated wire strings; explicit same-UUID Workspace/Library and StorageRoot/StorageLocation adapters; checked legacy Core adapters. Legacy external resolver identity cannot coerce into HostBindingId. |
| [Access](../../crates/photara-core/src/contracts/access.rs) | Eight action bits, prerequisite validation, exact six presets, explicit restricted/library-visible policy, per-role inheritance, scoped grants and deny precedence, distinct local controller/account principals, Library invitation ceilings and pure freshness/offline decisions. |
| [Schemas](../../crates/photara-core/src/contracts/schema.rs) | Exact names/type/schema/version coordinates, canonical decimal u64 and SHA-256, positive local revisions, S2 ObjectRef shape, bounded recursive variable-family admission, duplicate-key-rejecting JSON decoding and MIME media types. |
| [Resources](../../crates/photara-core/src/contracts/resource.rs) | Logical locations/slots and capture pins, filesystem/provider coordinate unions, immutable external/managed resource contracts, rights and storage classes, closed host status, bounded operation-bound resolution requests, nonserializable live leases and secrets. |
| [AssetSet v2](../../crates/photara-core/src/contracts/asset_set.rs) | Immutable Project-scoped ordered snapshots, selected representation/content revisions, explicit metadata dependencies and missing facts, separate content/membership/dependency digests, bounded pages and exact checksum/length reconstruction. |
| [Response coordinates](../../crates/photara-core/src/contracts/dto.rs) | Explicit Library/Project/Graph/Node scope, distinct local/server/package revision domains, authorization generation, request/base matching and eight response states with bounded diagnostic codes. |
| [Manifest v2 types](../../crates/photara-node-sdk/src/v2/types.rs) | Complete definition envelopes with independent ports, authored schemas, execution/effects, capabilities, determinism/cache, opaque context coordinates, mandatory Inspector, optional Work Surface, taxonomy, platform/runtime/tool requirements and explicit migration declarations. |
| [Manifest v2 validation](../../crates/photara-node-sdk/src/v2/validation.rs) | Structural and exact registered-schema/type checks, direct port connection validation, version/namespace/bounds enforcement, byte-preserving unsupported fallback and independent trust/runtime/platform/skin availability. No presentation or migration code executes. |

Checked wrappers validate during construction and deserialization. Public `Spec`
and DTO records are construction inputs: callers must use the documented
`validate`, checked conversion or registered-validation method before accepting
untrusted facts. Manifest structural validity does not establish trust, install an
implementation, register a schema or grant a declared capability. The registry is
host-supplied authority for the exact schema/type/family association; this slice
does not implement a general schema language or validate arbitrary payload bodies.

## Boundaries enforced in pure code

Access evaluation requires supplied principal, Library, Project, registration,
policy, membership and grant facts to agree. Restricted Projects have no Library
admin content bypass. Explicit Project revocation denies inherited access;
removing an override and revoking Library membership remain distinct facts.
Local-only control requires its own controller identity and no fabricated account.
These functions validate and combine supplied facts; they do not authenticate a
caller, fetch current permissions, commit a grant or prove last-manager races.

Resource descriptors contain no absolute path or host credential. Filesystem
components retain their exact UTF-8 spelling; they are never normalized, joined
or resolved here. Traversal, separators, drive syntax, control characters,
trailing dots/spaces and Windows device names are rejected. Provider object and
revision identifiers are opaque, bounded data, never URLs to fetch. Revision
pins and requested SHA evidence must agree with the descriptor. Managed resources
name blob ObjectRefs and MIME media types. Project roots are read-only; managed
artifact writes require the publisher contract and cannot resolve as direct paths.
A live lease retains the exact validated request, rights, operation and expected
content, rejects use for another request, and redacts its handle in Debug output.
Live handles and `SecretRef<T>` implement neither Serialize nor Deserialize.
Host safety checks, symlinks/reparse points, permission refresh, verified rebind
and lease lifecycle enforcement belong to CXT3b adapters, not these DTOs.

AssetSet membership is explicit. No API consults `ProjectAssetContext`, package
inventory or a selected UI window. Empty sets are valid; absent usable
representations require an explicit missing fact. Ordinals, unique identities,
Project/Asset ownership coordinates and exact selected metadata revisions are
checked. The content digest hashes canonical `members` and `project_id`; changing
SnapshotId or page boundaries does not alter semantics. Membership and dependency
digests have distinct domains. Page tokens bind Project, snapshot, content digest,
start and total count. Reconstruction rejects missing, duplicate, out-of-order,
truncated, extra, wrong-scope or tampered pages and verifies all three digests.
Object ownership claimed by a DTO still requires package closure evidence in CXT3a.
This code does not read or write package 1.1 objects.

Manifest v2 requires exact definition/package/schema/version coordinates. Typed
connections require matching type, schema and family; no v1 coercion, fan-in or
live/secret/workflow-reference ports. AssetSet ports require version 2. Authored
variable shapes recursively reject workflow and live families, including reserved
names mislabeled as configuration. Pure nodes cannot acquire live/credential
access; effects must name their declared targets, rights, operations, credentials
and request/receipt schemas. Cache validation cannot reuse effects or nondeterministic
results; captured reads need complete verified declared facts. Declaring access
never creates a grant. This is cache eligibility validation, not a new evaluation
key implementation.

Every definition requires all six Inspector sections. Work Surfaces are optional
and use five closed host component contracts with explicit input/action binding.
Category, brand and discovery metadata cannot determine evaluator behavior.
Missing schemas, code, trust or platform/runtime/tool support prevent evaluation;
a missing native skin independently disables node presentation code. Unsupported
bounded input is retained as exact original bytes for host-owned inspection.
V1 manifests remain inspectable through their original validator and acquire no
v2 rights. Migration declarations are deterministic, exact from/to coordinates
with implementation digest, ID-remap and diagnostics contracts. Inspection and
availability never authorize or execute migrations.

## Wire and resource bounds

| Input | Enforced bound |
| --- | --- |
| AssetSet snapshot | 10,000 members; no truncation |
| AssetSet page | 1–500 members requested; explicit empty-snapshot paging |
| Relative filesystem coordinate | 1–128 components, at most 255 UTF-8 bytes each and 4,096 total |
| Recursive variable schema | Depth 32, 1,024 shape nodes, declared list bound at most 10,000 |
| Manifest JSON | 1 MiB; strict duplicate-key rejection before typed conversion |
| Manifest collections | At most 256 definitions; at most 256 ports/capabilities per definition |
| Schema names | Qualified names at most 256 ASCII bytes; local names at most 64 |
| MIME type | Lowercase type/subtype, at most 255 bytes; no parameters or header syntax |
| Response diagnostics | At most 256 unique qualified codes; no value/path/secret payload |

The context/expression version fields are opaque compatibility coordinates.
There is no parser, AST, dependency resolver, capture engine or proposal executor
in CXT1a. Context version 1 and expression version 1 identify the planned contract;
they do not certify that CXT1b behavior exists.

## Golden fixture

[d19-contracts.json](../fixtures/generation-two/d19-contracts.json) is the only new
fixture file. Rust's existing canonical encoder generates its exact bytes using
the reserved `62000000-0000-4000-8000-…` UUID namespace. It contains the accepted
mask set, a complete manifest v2 and its digest, an explicit missing-representation
AssetSet snapshot/descriptor/page/ObjectRef, and composed/decomposed Unicode path
components. It has canonical compact JSON bytes with no trailing newline. Its SHA-256 is
`c4cd563e2a454bb1f8353250ee3bfd38a5c88a825dc0da64677ffcdeb2c0b893`.

The ignored `generate_additive_d19_fixture` test is the explicit generation entry
point; normal tests only read and independently regenerate/compare those bytes.
The pre-existing nine S6/D18 files, including their README, remain byte-identical.
This addendum proves the pure contracts tested here, not all ten later D19
conformance families. The four other proposed D19 fixture files remain uncreated.

## Verification

All checks were offline. The selected suite passed **99 tests**: 36 new Core
integration tests, 22 new SDK integration tests, two serialization compile-fail
doctests, and 39 retained tests (Core 24, SDK 3, Layout 8, AssetSet node 1, Disk 3).
The golden generator passed separately; it remains ignored in ordinary runs.
The SDK suite was rerun after regeneration with the reserved namespace.

```sh
cargo fmt -p photara-core -p photara-node-sdk --check
cargo check --offline --workspace --all-targets
cargo clippy --offline --workspace --all-targets -- -D warnings
cargo test --offline -p photara-core -p photara-node-sdk -p photara-layout-node -p photara-asset-set-node -p photara-disk-node
cargo test --offline -p photara-node-sdk --test contracts_v2 generate_additive_d19_fixture -- --ignored
cargo test --offline -p photara-node-sdk --test contracts_v2
```

The tests exhaust every u16 action-mask candidate and valid-mask union pair,
exercise access/policy matrices, malformed unions, exact boundary limits,
ID/adapters, path controls and UTF-8 preservation, live lease privacy, AssetSet
ordering/ownership/digest/page tampering, variable-family recursion and manifest
version/capability/cache/presentation/migration/fallback cases. The full workspace
was compiled and linted; database suites were not executed. Retained node tests
use disposable filesystem fixtures, never user Projects or SMB.

A pre-slice SHA-256 inventory confirms 302 pre-existing files are byte-identical,
including all other pre-existing
Rust/Swift files, Cargo manifests/lockfile, six applied L2 SQL migrations, all
CXT2 proposal files and nine baseline fixture files. Existing Core/SDK module
bodies are intact with only the additive module declarations. Markdown links and
whitespace checks pass (214 local Markdown links/anchors checked). No database or service was opened, SQL executed, runtime
migration installed, UI changed, package published, or git staging/commit/push done.

## Next gate

Stop at CXT1a. Separately select **CXT1b** for bounded D18 parser/AST/type checking,
directional dependencies, frozen context/capture, evaluation/cache v2 and proposal
contracts with their numeric/size/cycle/privacy tests. CXT3a owns package 1.1 closure
and explicit compatibility mapping; CXT3b owns disposable local repositories and
fake host conformance; CXT3c owns disposable service/RLS/sync conformance. L3 remains
paused until those required contracts and proofs are accepted.
