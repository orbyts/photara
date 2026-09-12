# Generation-two fixtures and conformance — S6

## Current exact review packet — 2026-09-12

The [D19 fixture delta and gates](D19_STATIC_SCHEMA_DELTA.md#fixture-delta-implementation-slices-and-gates) now enumerate ten required conformance families and five future separate addenda. [CXT1a](CXT1A_CONTRACTS.md) adds the Rust-generated [d19-contracts.json](../fixtures/generation-two/d19-contracts.json) using the reserved `62000000-0000-4000-8000-…` synthetic namespace. Its canonical bytes are independently regenerated and checked by the SDK tests. Every pre-existing S6/D18 fixture, including the fixture README, is byte-identical. [CXT1b](CXT1B_CONTEXT_CONTRACTS.md) additionally generated [d19-context.json](../fixtures/generation-two/d19-context.json), independently checked by Rust for exact source/AST/snapshot/cache bytes. CXT1a and all earlier fixtures remain byte-identical. CXT3 and L3 proof remains gated; the two pure addenda do not certify those adapter scenario families.

**D19 supersession note (2026-09-12):** [Libraries and node Work Surfaces](LIBRARY_AND_NODE_WORK_SURFACES.md)
is the current conceptual target. Library replaces durable Workspace; each Project
has one Library and explicit Project access. Graphs use connected AssetSets and
declared frozen context; a private package ledger is not an ambient Gallery/asset
union. Library management is app-owned; node Work Surfaces embed authorized host
pickers/components. First install opens local My Library. The pre-D19 implementation,
physical identifiers, examples and fixture contracts below remain baseline evidence,
not approval to reinterpret stored bytes. Exact contract/static schema review and
revised CXT1/CXT3 precede L3; no migration, source or fixture bytes change here.

**D18 addendum, 2026-09-12:** [typed context proposal](TYPED_CONTEXT_AND_EXPRESSIONS.md)
has a separate [inert context fixture](../fixtures/generation-two/context-amendment.json).
It exercises proposed qualification/backtick/fence syntax, typed AST/context bytes,
metadata/permission/privacy and proposal/CAS expectations without changing the
51 S6 baseline scenarios, nine original canonical vectors or package specimen.
D18 runtime results are not run; physical DDL/counts are deferred to CXT2.

Status: **S7 approved; bounded L1/L2 verified**, 2026-09-11. The S6
artifacts remain inert synthetic data. L1 materializes copies only beneath fresh
temporary roots and passes 25 focused tests, including all nine canonical vectors.
See [L1](PROJECT_PACKAGE_CODEC.md) and [L2](LOCAL_LIBRARY_IMPLEMENTATION.md).
L2 now executes six local migrations and selected Library/catalog scenarios in
fresh temporary databases, with 16 new tests passing. No existing user database,
service or user Project was opened. Remaining full merge/sync/service/publication
scenarios are specifications, not passed tests.

Read [S1](LOGICAL_DATA_MODEL.md), [S2](PROJECT_PACKAGE_SCHEMA.md),
[S3](LOCAL_SQLITE_SCHEMA.md), [S4](SERVICE_POSTGRESQL_SCHEMA.md),
[S5](SYNCHRONIZATION_CONTRACT.md), and the [S7 review index](SCHEMA_REVIEW.md).
Clean generation two is the acceptance target. Existing Neon and `v0.1.3`
remain optional reference/salvage; legacy import is not an acceptance gate.

## Artifact inventory

All data is under [generation-two fixtures](../fixtures/generation-two/README.md).
These are test inputs, not migrations, seeds for a live service, or credentials.

| Artifact | Exact role |
| --- | --- |
| [records.json](../fixtures/generation-two/records.json) | Account/Workspace, multi-role Person, client Organization, relationship, Kind/Location hierarchy, Node Store metadata, catalog and separately device-only locators |
| [package-specimen.json](../fixtures/generation-two/package-specimen.json) | Inert archive of all 33 exact files in a new `.photara` specimen; 28 JSON objects, two commits, one managed blob, bootstrap and HEAD |
| [canonical-vectors.json](../fixtures/generation-two/canonical-vectors.json) | Nine valid actual-Rust byte/hex/SHA-256 vectors and seven invalid-input expectations |
| [sync-trace.json](../fixtures/generation-two/sync-trace.json) | Four exact sealed request bodies, offline A/B chain, acceptance/feed milestones, stale conflict and explicit rebase |
| [normalization-and-limits.json](../fixtures/generation-two/normalization-and-limits.json) | Concrete proposed Unicode/alias policy, eight term vectors and finite bounds |
| [scenarios.json](../fixtures/generation-two/scenarios.json) | 51 named scenario specifications plus 12 crash boundaries; every runtime result explicitly not run |
| [social-export.json](../fixtures/generation-two/social-export.json) | D16/D17 addendum: three manual/provider profile examples, selected display snapshot, Workspace-wide subject collision and exact checksummed three-member logical export specimen |

An archive file entry is `{path, utf8, byte_length, sha256}`. Decode the JSON
string to obtain exact UTF-8 file bytes; do not hash its JSON-escaped spelling or
append a newline. The outer archive is ordinary readable JSON, not package
authority. This representation deliberately avoids creating an active macOS
package or invoking today's one-JSON project opener. After approval, a disposable
harness can materialize entries beneath a newly allocated test directory only.
No shell/SQL/provider command is embedded or intended to execute on opening it.

Fixture IDs use the fixed `60000000-0000-4000-8000-…` namespace; sync trace
mutations use `61000000-…`. The independent S5 byte vector retains its published
IDs. `fixtures.invalid`/`fixture.invalid` are synthetic, never destinations to
contact. The tiny managed PPM is synthetic color data; external JPEG bytes are
deliberately absent. Node catalog metadata advertises no installed executable.

## Concrete package acceptance

The specimen has ProjectId `60000000-0000-4000-8000-000000000011`, title
“North Coast Fixture”, originating Workspace, Person/Organization assignments,
and a dated Location assignment with matching LocationKind and participant.
Snapshots are immutable typed subsets, with local source revision `1`; they do
not carry Account records, current authorization, or whole Library exports.

The two project-owned assets are selected from three discovery candidates. One
has a managed 2×2 PPM representation with verified byte digest; the other has an
external JPEG handle with weak file-observation evidence. Its relative path and
logical root ID are portable, but the SMB URI, mount path and bookmark **reference**
exist only in records.json's `device_only` section. No bookmark bytes or secrets
appear anywhere. Missing external bytes must not mark the package corrupt or
upgrade its fingerprint. Reproducible cache is entirely absent.

“Capture” pins `photara.fixture.effect@0.0.0` / definition
`photara.fixture.effect.observe@1`; “Review” is empty. The ordinary embedded
Core GraphDocument retains numeric revision zero, exact pin, and opaque node
extension. The wrapper's required package set exactly matches node pins. A null
manifest is intentional and permitted by S2: missing manifest/runtime yields a
diagnostic while preserving metadata, not invented executable trust. Category
`photara.category.automation` and search terms are separate catalog fixture facts.

Commit 1 contains authored state and empty history. Commit 2 reuses the **same
authored ObjectRef** and adds an immutable Run, Attempt, effect intent, interrupted
terminal facts, evidence, and an inconclusive reconciliation observation receipt.
The receipt is not a provider success claim. Operation knowledge remains unknown.
Run input captures the exact managed representation revision and source authored
snapshot; the graph digest names the ordinary GraphDocument, not its wrapper.

Every commit inventory is the exact transitive authored/history object closure,
sorted by kind then digest; it excludes itself, commit and HEAD to avoid cycles.
The archive retains both commits and all their closures. HEAD commits to commit
2, which commits to its parent and bootstrap. Current HEAD checksum:

`e0a938eaa8b8ee4a8ef8e9df81fceb4f10cc535552510f5e8edda29b3e543e7e`

This is the SHA-256 of the selected **commit file**, not the HEAD file. Each
archive entry records its own separate hash. Rebuilding catalog indexes must use
the verified ProjectId + CommitId + commit checksum; cloud reports remain
client-reported. Neither wall-clock newest nor uploaded observation wins package
authority. A move changes device location, not these semantic bytes.

The newly spelled envelope IDs for inventories, snapshots, and history collections
in this specimen are concrete S2 review refinements, not implemented Rust types.
S7 approved these shapes; do not treat a successful JSON parse as full domain
deserialization. Only embedded Core GraphDocuments were deserialized by current
Rust during static validation.

## Frozen canonical bytes

The vector output was produced using the retained Core `canonical_json` and
`canonical_digest` functions with `serde_json = 1.0.151`, default number handling,
in a temporary offline probe. No product source changed. All package JSON bytes
were independently compared with that Rust serializer; both Core graph payloads
must deserialize with their exact IDs/pins. Source bytes are compact UTF-8 with
no BOM or trailing newline. Keys use Rust string order (UTF-8 lexical/scalar
order), **not JavaScript UTF-16 key order**. Arrays retain order; text is never
Unicode-normalized while hashing. Domain sets must be sorted before encoding.

Notable actual vectors:

| Case | Frozen behavior |
| --- | --- |
| Unicode keys | decomposed `é`, composed `é`, U+E000, U+1F600 in that order; no composition or UTF-16 sorting |
| Negative zero | parsed `[-0,-0.0,0,0.0]` becomes `[-0.0,-0.0,0,0.0]` |
| Floating format | preserves `1.0`; writes `1e+20`, smallest subnormal `5e-324`, and finite maximum `1.7976931348623157e+308` |
| Integer limits | retains i64 minimum and u64 maximum exactly; new revision/counter fields remain decimal strings |
| S5 request | 837 bytes; SHA-256 `035103e3c2b8b4e4cf43ffd420c4361e86ff961057d34d73ce94c75ca0f40031` |

The codec is not RFC 8785. JSON.stringify/JSONB rendering/Swift JSONSerialization
are not interchangeable substitutes. A future native encoder must pass every
vector; preferred first implementation is Rust-owned encoding behind the facade.
Dependency upgrades that alter bytes require a new codec version, not rewritten
immutable history. Add regression vectors before extending accepted numeric forms.

Input validation is a separate boundary. `serde_json::Value` loses duplicate-key
information: **current canonical_json alone cannot enforce S5's duplicate-key
rejection**. The approved implementation must reject duplicates in a streaming
parser before Value construction, reject malformed UTF-8/surrogates/BOM/nonfinite
or out-of-range input, enforce exact typed integers before lossy conversion, and
compare sealed input bytes to canonical output. Invalid vector entries are pending
validator tests, not claims that today's Core rejects them. Namespaced optional
fields must round-trip; unsupported required schemas/features stop writes/apply.

## Normalization and bounds proposed for S7

`normalization-and-limits.json` pins proposed policy 1 to Unicode 16.0.0:
NFC, full default case folding, NFC, then trim/collapse UCD White_Space to ASCII
space. Graph names use these text steps only. LocationKind terms additionally
use immutable alias dictionary `photara.kind-aliases.v1`, initially containing
mandatory Beach/beaches and Studio/studios equivalence groups. This is a small
explicit seed, not a claim to recognize every language's synonyms or plurals.
Custom semantic aliases require explicit claims and user reconciliation.

Crucially, creation from **either** `beach` or `beaches` must claim both keys;
omitting the inverse alias is invalid. A key `beaches` is not itself normalized to
`beach`: both terms resolve to one owner through shared claims. Rust local and
service validators recompute closure; SQL's Workspace+term-key uniqueness then
protects claims under concurrency. Direct SQL alone cannot infer dictionary
closure. Policy upgrades use a new immutable version and collision review, never
in-place reinterpretation. Eight vectors are specified but the complete Unicode
algorithm is not implemented or verified in this slice.

The bound fixture preserves S5's 1 MiB command/root, 1,000 changed roots, 3 MiB
batch, 4 MiB response, 16 MiB/10,000-root snapshot limits and supplies concrete
string/collection/package-reader ceilings. Text sizes mean UTF-8 bytes, not Swift
characters. JSON depth is 64; display names 512 bytes, descriptions 8,192 bytes,
extensions 65,536 bytes. Package control JSON is bounded at 16 MiB and managed
blobs are streamed with declared length/storage-budget enforcement. These are
proposed initial interoperability ceilings, not subscription entitlements or a
promise every Workspace fits. Max/max+1 and multibyte tests must reject oversize
atomically; batch splitting cannot weaken merge invariants.

## Scenario execution contract

Every case in scenarios.json names its setup mutation and exact semantic outcome.
The fixture records are logical adapter inputs, **not SQL row dumps**: local
UUID BLOB16 and server UUID columns decode to the same IDs; local/server revisions
remain distinct. Fixtures are reset independently, not concatenated into a live
database. SQL harnesses later use bound values and the published migration order.

| Family | Required proof after S7 |
| --- | --- |
| LIB/LIFE | Identity/role separation; required kind/hierarchy; interval overlap; immutable ID/created time; +1 changed-root revisions; explicit tombstone/reference handling |
| KIND-01–07 | Case/alias collisions in either creation order; full claim transfer; source retirement terms; target canonical promotion; omitted/concurrent dependents rollback; chains; tombstone reservation; explicit offline A-absent/B-exact collision reconciliation |
| PKG-01–08 | Exact package closure; managed versus external; move/rebind/missing/duplicate; copy/fork provenance; corruption; stage/commit/HEAD/catalog crash boundaries; single-writer ownership; path safety |
| RUN/NODE | Durable interrupted versus unknown effect; immutable receipt/evidence; append-only reconciliation; source digests unchanged; missing exact runtime preserves opaque state |
| SYNC-01–07 | Offline chains; current authorization on dedup; receipt and feed milestones; whole-batch quarantine; explicit conflict/rebase; immutable request identities; bounded atomic snapshot install preserving device/recovery/overlays |
| MEDIA/CAT | Immutable final object separate from staging; digest/length verification; safe unknown outcome; privacy profile and cross-Workspace redaction; observations never package authority |
| AUTH-01–04 | Synthetic JWT verification failures; actual runtime-role grants/RLS; membership/identity revocation races; last-owner guard; no billing or private identity access; entitlements do not grant membership |
| LIMIT/VERSION | Byte/count/depth max/max+1; unsupported required features/schema/normalizer; preserved optional fields; no lossy saves/cursor movement |
| STORE-01–03 | Both explicit Storexa 0.2 backends; lifecycle/acquire/health/version; commit/rollback/drop; file persistence; transaction-local scope reset; safe diagnostics; migration checksum drift |

For KIND-03, merging Studio into Beach is an intentional **synthetic merge-test
action**, not product taxonomy advice. The source's frozen descriptors and term
snapshot survive; all current claims transfer, eligible-owner constraints remain
deferred to commit, live Locations rebind, and one accepted batch changes every
affected root exactly once. Promoting transferred `studios` to target canonical
must leave no duplicate owner. Rejected multi-root commands create no accepted
batch and no partial term movement. Tombstones retain claims; no arbitrary term
stealing, resurrection, or unmerge is allowed.

Sync trace starts independently at server/local revision 7 and feed sequence 20.
A and dependent B commit offline; A becomes server revision 8/batch 21. Receipt
alone leaves B unsealed. Ordered application installs A as base while B remains
visible overlay; only then may B seal at `sr1:8`, yielding revision 9/batch 22.
Stale C receives a durable conflict, not a batch. Explicit D is a new MutationId
at `sr1:9`, yielding revision 10/batch 23. Reverse receipt/feed ordering has the
same final state. Repeated identity with different bytes/actor must not disclose
or replay another receipt. These steps specify expected acceptance/feed content;
they are not captured live HTTP responses or signed cursor samples.

## D16/D17 fixture addendum

[Social profiles and portable Library export](SOCIAL_PROFILES_AND_LIBRARY_EXPORT.md)
adds SOCIAL-01–05 and EXPORT-01–05 without changing the original package archive
or canonical golden vectors. The new JSON has three profiles (two for one Person,
one for the client Organization), manual handle rename, a rejected cross-owner
bound-subject collision, consented cache-only versus durable media policy, and
chosen Project snapshot display fields with explicit sensitive exclusions.

The inert export has canonical Library records, portable discovery hints and one
rights-permitted synthetic media member, all size/hash checked. Its manifest has
a detached hash; no active bundle is extracted or imported. Packages remain
separate, and restored root/project IDs require user rebind plus actual package
verification. Auth/device/absolute-path/sync state is absent. Encryption wrong-key
and transactional restore/collision expectations are specifications only: no
encryption algorithm, database or importer was executed. The original six JSON
fixtures plus this addendum total seven JSON artifacts.

## Verification layers and implementation gates

**Static now:** parse every outer JSON, decode/hash all archive entries, validate
derived object paths and lengths, recompute both inventory closures, parent/HEAD/
bootstrap links, graph pins/IDs and unchanged authored root; compare byte vectors
and package JSON with actual Core; verify request hashes and scenario references;
parse S3/S4 SQL/function text without a connection; check local links and diff
whitespace. These checks cannot prove SQL constraints or service authorization.

**Completed L1 scope:** bounded duplicate-safe JSON and directory validators,
canonical vectors, temporary specimen materialization and tamper/path/typed-link
tests. No publication or database tests. See the linked implementation report.

**After separate authorization of each next disposable scope:** execute clean SQLite/PostgreSQL migration
families, check FK/deferred triggers/RLS with the real unprivileged roles, run the
shared scenario outcomes and transaction/revocation races, exercise migration
failure/checksum handling and both Storexa adapters. Use local fake API/Auth0/
object-store transports first; no real credentials needed. Any Neon disposable
resource, Auth0 tenant, object storage, or NAS write requires its own authorization.

**Before release claims:** inject all 12 named crash points and process/pool
restarts; verify disk/SMB fsync/rename/locking behavior on explicitly scoped test
roots; test native encoder vectors if any; bound memory/time under limits; verify
privacy/redaction and stable replay bodies. A local SQLite test cannot prove SMB
publication, and SQL grammar cannot prove privileges or deferred constraints.

S5 earlier wording that fixtures must run “before S7” is narrowed by this approved
documentation-only S6 scope: **specification and static evidence precede S7;
runtime execution follows explicit S7/disposable-test authorization**. Approval
must be conditional on passing those implementation tests before release, not a
claim that unimplemented service behavior already passed.

## Acceptance checklist

- [x] Synthetic clean gen2 records, package bytes, offline trace, vectors, limits
  and 51 named scenario expectations prepared, including D16/D17 additions.
- [x] Actual Core canonical-byte behavior frozen, including floating/Unicode edges.
- [x] Static package/hash/link/SQL grammar checks performed without database access.
- [x] No legacy import dependency; runtime checks and S7 choices distinguished.
- [x] User approved physical/protocol/package/normalizer/limit choices at S7, 2026-09-11.
- [x] L1 disposable reader test scope explicitly selected and passed.
- [ ] Runtime validators, schemas, authorization, races and recovery pass before
  corresponding feature/release claims.

L2 is now complete within its recorded scope, not a blanket pass for all 51 cases.
Next: authorize L3 recoverable package creation and temporary tests,
following [the recorded S7 gates](SCHEMA_REVIEW.md). Full Kind merge remains L2b.
