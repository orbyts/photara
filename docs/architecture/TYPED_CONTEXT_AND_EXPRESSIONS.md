# D18 — Typed Context, Variables, Metadata and Expressions

Status: **CXT1b pure context contracts complete**, 2026-09-12, after separate
selection by Suhail. Read the [implemented API, grammar, bounds and limits](CXT1B_CONTEXT_CONTRACTS.md).
R5 and the D19 freeze remain the accepted design; CXT1a/CXT2 and historical
S1–S6 bytes remain unchanged. CXT3a is the next separately selected slice; L3 is paused.
The remaining architecture below includes future adapters and unsupported fenced
containers and must not be read as a claim that those runtimes exist.

## Accepted exact D19 contract

The [accepted freeze](D19_CONTRACT_FREEZE.md#assetset-metadata-and-variables)
now refines D18: port families remain registered types but are forbidden in
variables recursively; ResourceDescriptor is persistable, live handles are not;
slot captures pin SlotId/revision/target; scope dependencies are directional.
Initial CXT1 rejects triple-fenced containers as unsupported. Existing language,
limit, privacy and single-authority apply recommendations are collected under
R5, accepted as proposed. [Static package/SQL/DTO changes](D19_STATIC_SCHEMA_DELTA.md)
are now specified; no old bytes or current API changed. R1–R8 were accepted as proposed 2026-09-12; CXT2 and [CXT1a pure contracts](CXT1A_CONTRACTS.md) are complete. CXT1a supplies recursive variable-family admission and opaque versioned context coordinates. [CXT1b](CXT1B_CONTEXT_CONTRACTS.md) now implements the bounded pure parser/AST/dependencies/capture/cache/proposal contracts. Package, repository and host integration remain CXT3.

## D19 supersession and implementation hold

[D19](LIBRARY_AND_NODE_WORK_SURFACES.md) revises this proposal before CXT1:
Library is the durable ownership domain; graph assets come from explicit
`$input.<port>` values, never `$project.assets` or a project-wide Gallery union.
Authoring visibility does not grant runtime access. Project membership/grants,
bounded snapshots, typed value families and embedded host pickers follow D19.
This document updates the conceptual examples; the existing inert D18 fixture is
preserved as historical proposal evidence and is not D19-conformant until a
separately reviewed fixture amendment. Uppercase HostPlace rules remain intact.
Logical/package/NodeSDK freeze and static CXT2 review now precede revised CXT1.

## Existing foundations and additive boundary

Current [Core](CORE.md) has `TypedValue { value_type, value }` and
`SchemaValue { schema, value }`; their payload is serde_json::Value, not a full
schema validator. `ValueTypeRegistry` registers exact descriptors and compares
port types, but does not yet validate arbitrary payloads against schemas.
`GraphDocument`/`NodeInstance` distinguish configuration and authored state;
`ProjectDocument` owns one graph and a typed ProjectAssetContext. Existing
`AssetSet` is the exact `photara.asset-set` v1 value, not a path abstraction.
`EvaluationRequest.environment` already participates in every node key alongside
definition, configuration, authored state, typed inputs and implementation digest.
`NodeEvaluationRequest` currently contains no resolved context. NodeSDK has broad
capability IDs, not validated context read/write declarations.

Retain these APIs and existing documents. Add validated wrappers, typed commands
and an explicit context-aware evaluation entry point; do not reinterpret old
extension bags, inspect arbitrary strings as expressions, or change old keys.
Unknown optional data still round-trips. Semantic D18 behavior needs a required
feature/version and must not masquerade as an ignorable extension.

## Scopes, identities and authority

| Scope | Authority / lifetime | Read and mutation boundary |
| --- | --- | --- |
| Built-in | Versioned Core descriptor; host resolves capabilities for one session/run | Read-only `$project.root`; no OS environment enumeration or project asset union |
| Input | Current node's exact declared connected port values | `$input.assets` or `$input.<port>`; no inventory/selection fallback |
| Host | Device adapter, explicitly frozen for one run | Granted resources and explicitly requested clock/locale facts, never portable machine paths |
| Account | Service/native preferences only in v1 | No ambient semantic variables; copy a preference into a named run override explicitly |
| Library | Typed local Library authority, accepted cloud history under S5 | Variables are separate typed aggregates, not columns added to Person/Location or Account |
| Project | Authored package root | Project variable definitions/values; edits require Project revision and package CAS |
| Saved Graph | Named Graph document | Graph definitions/values; edits require Graph revision and package publication CAS |
| Node namespace | NodeInstance within its owning Graph, exact definition pin | Schema-owned private authored values; no second mutable ambient node store |
| Run | Immutable overrides and resolved ContextSnapshot | Frozen before attempts; does not update Project/Graph/Library defaults |

Each mutable variable aggregate has nonnil `VariableId`, immutable owner
`ScopeRef`, definition schema version, local owner revision, timestamps and
`active | tombstoned` lifecycle. Library IDs use L2 positive i64 revisions and
S5 opaque server revisions; Project/Graph use their existing authored revisions.
Never compare those revision domains. A `VariableValueId` identifies the single
optional current explicit value owned by a definition; replacements retain its
ID and advance the aggregate revision. Immutable snapshots additionally capture
the exact revision/checksum. RunOverrideId and ContextSnapshotId are independent
IDs; IDs are not digest substitutes. No variable merge or resurrection in v1.

Definition fields: namespaced machine name, display label, description,
`ValueTypeRef`, exact `SchemaRef`, optional validated default binding, allowed
override scopes, sensitivity, portability and provenance. A binding is a tagged
`literal(TypedValue) | expression(ExpressionRef)` union, not arbitrary JSON.
Value records carry the same resolved type, origin (`manual | command | node-proposal
| imported`), optional source Run/Attempt/Operation IDs, and revision/time facts.
Defaults and explicit values belong to one CAS aggregate, not independently
updated tables. Expression result type must exactly match the definition.
Changing a semantic type/schema incompatibly creates a new VariableId and an
explicit reference migration; no coercion of existing values on read.

Namespace is lower-case reverse-domain ASCII; local name matches
`[a-z][a-z0-9_]{0,63}`. `(scope owner, name)` is unique across namespaces and reserved
through tombstone; rename reserves the previous spelling. Labels are Unicode and
need not be unique. `photara.*` and built-in scope members are reserved. These are
machine identifiers, not LocationKind terms; do not apply inflection/casefold
claim policy. Namespace remains immutable publisher/provenance identity, not a
way to shadow another definition's same-scope name. Distinct NodeInstances have
distinct owners, so their private names can repeat safely. Rename never retargets
an ID-bound expression.

V1 types: bool, string, signed i64, exact decimal with explicit scale, timestamp,
enum, registered bounded record/list/optional types, typed Library/Asset/Resource
references, AssetSet and ResourceHandle. i64 and decimal values use tagged
canonical decimal strings; no expression float arithmetic or implicit conversion.
EXIF rational values preserve signed numerator/positive denominator and unit.
Schemas validate every payload recursively; unknown required types are unsupported,
not accepted because they fit JSON. Optional absence differs from null; a stored
null is valid only if the named schema allows it.

## Qualification, defaults and references

There is **no cross-scope implicit shadowing**. Inside expression delimiters,
references use explicit lowercase scopes and lowercase snake_case member names:
`$library.delivery_policy`, `$project.delivery`, `$graph.caption`, `$node.mode`.
`$node` means the current NodeInstance only. `$run.overrides` is inspection data,
not a name-search fallback. Bare `$delivery` is invalid. Names are editor syntax:
the authored AST binds exact VariableId/ScopeRef/type and retains a display hint.
Compilation is an explicit command against a known owner revision, never a
runtime name lookup that could bind a newly created record.

**Casing is semantic and case-sensitive.** Uppercase reserved symbols identify
global host runtime places; lowercase qualified symbols identify portable
Photara scopes (`$project`, `$library`, `$graph`, `$node`, `$run`, `$asset`, `$input`).
Functions use lowercase qualified names such as `path.join(...)`. `$HOME` and
`$project.root` both resolve through capabilities, but root is lowercase because
it belongs to Project semantics; uppercase denotes global host places, not every
capability. `$PROJECT`, `$Input.assets`, `$input.Assets`, `$home`,
`$library.DeliveryPolicy`, `Path.Join` and unknown `$PATH`/`$CUSTOM` are invalid.
Do not casefold, consult process environment or guess a replacement. Built-in
members (`project.root`, `run.overrides`, `asset.metadata`) and the `input` scope are
reserved against user definitions. Same spelling in different explicit scopes
is allowed; same-scope collisions, including cross-namespace ones, are rejected.
All these references occur inside the opt-in single-backtick syntax below;
ordinary text is not globally reinterpreted.

`$asset.metadata.exif.camera.model` is available only in an explicitly asset-bound
evaluation/preview context naming one captured AssetId and representation policy.
It is not an implicit loop or Gallery selection. Without that binding it fails
`asset-context-required`; collection operations remain explicit bounded queries.

For one exact definition only, effective precedence is an allowed Run override,
then the explicit stored value, then its default binding, else `unavailable`.
A Graph that wants a Library fallback must author that reference explicitly
(for example a Project default expression referencing a pinned Library value).
Override permission is opt-in by definition; an override cannot change owner,
type, permission or sensitivity. V1 override payloads are validated literals only.

Library reads in portable Projects use explicit immutable captures.
Updating them is a Refresh Context command that shows differences and advances
authored revision. A separately requested run refresh can capture newer values
without changing saved defaults; it is recorded in that Run. Offline open uses
the stored capture or reports unavailable, never silently substitutes live data.
Live Library authoring previews are labeled live and cannot be a Run snapshot.
Cross-Library captures convey contained facts only, never read/write permission.

Library access uses typed IDs/assignments and versioned bounded query descriptors
(`person.by_id`, `organization.by_id`, `project.assignments`) implemented by the
host resolver. Queries resolve before evaluation, record exact selected IDs,
revisions, schema and field projection, and then become immutable inputs. No
flattened global Person/Organization/Location bag, dynamic SQL, arbitrary property
walk, or network lookup from an expression. Selection from UI is not an input
until an explicit semantic command captures its AssetSet/typed reference.

## Expression v1

Proposed source language `photara.expression.v1`; semantic encoding
`photara.expression-ast.v1`. Source is retained for editing, AST is authoritative
for execution; a compiler verifies source/AST correspondence against its exact
version and binding context. Unknown AST opcodes/features refuse evaluation.
V1 ships an AST interpreter; optional bytecode is a disposable compiler-versioned
cache with an AST checksum, never portable authority or trusted uploaded code.

Allowed syntax is bounded literals, qualified refs, registered record fields,
typed equality/order, bool `and/or/not`, checked i64 `+/-/*`, `if`, and a fixed
function registry. No loops, recursion, user functions, assignment, reflection,
eval, regex engine, shell/subprocess, environment interpolation, filesystem or
network access. Both branches are statically typed and declared dependencies are
the union of all branches; conditionals are lazy only for value evaluation.
No `now()`, randomness, process locale or directory listing: such facts require
an explicit frozen host input. Overflow, division support not present in v1,
unknown fields and failed optional unwrap produce typed diagnostics, not strings.

The user's preferred authored syntax is **single backtick (U+0060) delimiters**
around inline references/expressions, inspired by Houdini. Delimiters are an editor
source convention, not a shell operation or part of the reference identity. For
example, an opted-in field may contain the following source bytes:

```text
`$project.delivery`
Delivery: `$project.delivery`
`path.join($project.root, "exports", "preview")`
```

Fields must explicitly declare `literal-only | expression | template` in their
versioned schema. A literal-only string, ordinary description or existing node
configuration never interprets backticks. An expression field requires exactly
one delimited expression (plus surrounding whitespace); a template alternates
literal spans and delimited expressions. A user selects a literal binding to
store code-looking text without evaluation. Plain `${...}` has no special meaning.
Single-line template literal spans use backslash followed by U+0060 for a literal
backtick, and `\\` for a
literal backslash; other escapes are rejected rather than guessed. Inside an
expression, double-quoted strings use JSON escapes; a literal backtick must use
`\u0060` so it cannot accidentally close the source delimiter. Empty/unclosed
delimiters, nested delimiters and trailing expression-field text are errors.

Future multiline source containers use **triple-backtick fences** on their own
lines with a required language/version tag `photara-expr-v1` or
`photara-template-v1`. The closing fence is a line containing exactly three
backticks. Expression-fence bodies contain the same bounded expression grammar;
template-fence bodies contain literal text and inline delimited expressions.
They are not Markdown execution, Bash, PowerShell, Swift, JavaScript or arbitrary
code. Unknown/missing tags are unsupported. A literal would-be closing fence in
a template is escaped using the same backtick escape, never executed. Store exact
UTF-8 source separately; CRLF is accepted as whitespace/literal source as typed,
not silently normalized before source checksums. Parser error spans are zero-based
half-open UTF-8 byte offsets into the complete authored source including fences;
DTOs also provide derived one-based line/Unicode-scalar columns for display.
Escapes and multibyte characters must preserve the mapping. General multiline
scripting, loops and Turing-complete code remain deferred; even future fenced
v1 content is only a bounded expression/template. Initial CXT1 may refuse fenced
containers with a typed unsupported diagnostic until its exact parser is tested.

Source records name language version, container mode and compiler version.
The parsed AST records explicit `reference`, `literal`, `call` or `template`
nodes; delimiters/fences never survive as executable shell text. AST bytes and
typed bindings, not cosmetic source whitespace, determine semantic expression
digest. CXT2 must preserve both exact source and AST checksums and validate their
correspondence. Changing formatting can change source checksum but not AST digest.

Supported calls initially: `text.format` (explicit locale/version),
`optional.or_else` (absence only, never permission/type/error suppression),
`path.join`, `metadata.values`, `metadata.common`, `metadata.distinct` and
`assets.count`. No arbitrary iteration; metadata operations have fixed bounded
semantics. Template mode is an explicit expression kind, not global string
interpolation. Only strings interpolate directly; other permitted scalars need
explicit format. Resource/AssetSet/SecretRefs never stringify into templates.

Limits accepted through R5, 2026-09-12: source 16 KiB UTF-8, AST 1,024 nodes/depth
32, 256 direct variable dependencies per expression, 10,000 expanded dependencies
per snapshot, 10,000 assets per metadata query, 64 KiB per literal/value, 1 MiB
ContextSnapshot and 100,000 interpreter operations per expression. Existing S6
limits remain stricter where applicable. No hidden truncation or partial success.
All sets have prescribed canonical order; compilation and query-result closure
detect indirect variable cycles before execution, including fallback/defaults.

## Resource and asset metadata semantics

`$project.root` is a capability-backed **ResourceRootHandle**, not an absolute
path or string. Its portable descriptor identifies ProjectId plus a logical root
kind; the host supplies a revocable grant outside serialization. `path.join(root,
"exports", "preview")` returns a narrower ResourceHandle with safe relative
components; it grants no new read/write right and performs no I/O. Traversal,
absolute/drive/UNC forms, separators in a component, reserved names, alternate
streams, unsafe Unicode/case aliases and package-internal control targets are
rejected or unsupported. L1 portable path rules are a floor, not host containment
proof. Output grants designate approved output subtrees, never HEAD/objects/
locks. Only the package publisher writes internal package files.

macOS resolves bookmarks/security grants; Windows resolves native handles and
reparse-point/drive/UNC policy in a separate adapter. Neither platform path nor
grant enters portable JSON. Identical logical paths survive move/rebind; actual
path-dependent behavior is an explicit nonportable dependency and disables shared
cache reuse. A ResourceHandle is not authority to overwrite an existing file.

Library storage slots follow the same typed rule. Conceptual
`$library.storage.raw_archive` source resolves an ID-bound Library Storage
Location; the current device separately supplies an authorized Host Binding.
Renaming the display label or slot cannot retarget a compiled expression, and an
unavailable binding remains distinct from deletion. External source resources,
managed Project resources, external artifacts and transient cache have separate
retention and authority classes. See the canonical
[storage-location and host-binding contract](STORAGE_LOCATIONS_AND_HOST_BINDINGS.md).

Node-private or Project variables may hold portable resource references. A node
can propose a Project-variable update only through a revision-checked declared
command and capability. Values that participate in workflow dataflow—including
AssetSet membership—remain typed ports rather than hidden variable coupling.

### Reserved host places, not process environment

The user's requested `$HOME`, `$DOWNLOADS`, `$DESKTOP`, `$DOCUMENTS`, `$PICTURES`
and `$TEMP` form a closed HostPlace registry. They return typed capability-backed
DirectoryHandle/ResourceRoot values, never strings, without knowing a username
or hard-coded path. Opted-in source examples:

```text
`$DOWNLOADS`
`path.join($PICTURES, "Photara", "exports")`
```

Prefer `$DOWNLOADS` over `path.join($HOME, "Downloads")`: known folders can move,
localize or be provider-backed. Uppercase spellings are exact reserved built-ins;
no user variable shadows them. Unknown names, `$PATH`, `$TOKEN` or `env("NAME")`
are unsupported. Never enumerate/read inherited process environment or shell
state to implement this language. Discovery of a place is not permission to use it.

| Symbol | macOS native adapter | Future Windows native adapter |
| --- | --- | --- |
| HOME | Account home-directory service | User-profile Known Folder |
| DOWNLOADS | User Downloads directory service | Downloads Known Folder |
| DESKTOP | User Desktop directory service | Desktop Known Folder |
| DOCUMENTS | User Documents directory service | Documents Known Folder |
| PICTURES | User Pictures directory service | Pictures Known Folder |
| TEMP | Run-scoped native temporary-directory lease | Run-scoped native temporary-directory lease |

Use native known-location APIs, never concatenate localized folder names onto
HOME. Core/AST symbols do not change across platforms. Resolution may be
unavailable, denied, not-yet-granted or unsupported. Exact node declarations
name the place, read/write/create/list rights and allowed subtree/operation
bounds. HOME does not grant recursive access to all user data. Host policy may
prompt before freezing runnable context; expressions/nodes cannot open permission
prompts or bypass sandbox grants. Resource use rechecks current authorization.

Freeze each declared symbol, opaque nonsecret binding identity, binding generation
and availability in a private DeviceContextSnapshot. Its canonical digest
(`domain: photara.device-context.v1`) enters affected node keys and the Run's
environment digest alongside portable ContextContentDigest. No resolved path or
access token is hashed. A changed effective native binding advances generation;
availability changes also change the digest. Captured availability is historical,
not a continuing guarantee. Revocation stops new use/retries without rewriting
the snapshot. Binding IDs/grants/paths stay device-local, excluded from sync and
portable export. Portable Run evidence stores the symbol, device-dependent/
non-replayable marker and dependency digest, not a reconstructable native binding.
Another device must resolve and capture new context, not reuse results merely
because the uppercase spelling matches. This separates a nonsecret binding
identity from the runtime authorization grant, which is never serialized/hashed.

TEMP is explicitly ephemeral/run-bound: its lease identity prevents shared cache
reuse. Promote essential results to durable storage before cleanup. Do not expose
a general `$CACHE` in v1; cache locations are host-owned implementation detail,
accessible only through scoped scratch/cache capabilities, never stable authored
output targets. No host-place symbol permits portable absolute-path variables.

`$input.assets` is the frozen typed AssetSet on the current node's declared
`assets` input port, with explicit IDs/order and captured representation revisions.
`$input.<port>` addresses another declared input with that port's exact type.
A missing required input reports unavailable; there is no `$project.assets`,
ambient Gallery or inventory fallback. The private package identity/provenance/
artifact ledger supports reference closure and retention without becoming a node
input union. Existing AssetSet bytes remain unchanged pending exact versioned
contract review. UI selection affects evaluation only through authored commands.

`AssetMetadataObservation` is immutable package evidence identified by its own
ID, AssetId, RepresentationId and exact content revision/fingerprint. It carries
namespaced schema/key, typed value, extractor ID/version/implementation digest,
capture time and source (`extracted | user-asserted | provider-reported`), plus
confidence/verification facts where schema-defined. SHA-256 is not proof that
EXIF claims are true. Metadata from an older fingerprint cannot describe newer
bytes; current pointers change only by an authored selection/refresh command.
Imported metadata is untrusted data, not instructions or automatically live
variables. Extraction uses a separately granted host/node operation, never an
expression side effect. Discovery-only temporary extraction is a cache and must
be captured/promoted before becoming a durable Run dependency.

Field examples: `exif.camera.make`, `exif.camera.model`, `exif.lens.model` and
`exif.capture.exposure_time` (typed rational seconds). GPS, serial numbers, face
and contact fields are sensitive; extraction and export default to an approved
field allowlist, not every EXIF tag. User corrections retain both the assertion
and its provenance, not overwrite extracted history.

`metadata.values(asset_set, field, representation_selector)` returns an
AssetId-ordered typed result per member: value, missing, unavailable, ambiguous
representation, or conflicting observations. The selector is an exact captured
representation ID or versioned explicit role-selection policy; no arbitrary
first representation. All selected observations/fingerprints and negative facts
enter dependencies. `metadata.common` requires a nonempty set, every member
resolved and exactly equal typed values; otherwise returns a structured empty/
missing/ambiguous/conflict error. `metadata.distinct` returns canonical typed
values with contributing AssetIds and explicit missing/error members, not one
misleading camera/lens label. No majority winner, first-value fallback or implicit
string coercion when assets disagree. Queries run over the frozen snapshot only.

## Capabilities, sensitivity and deterministic snapshots

An exact NodeSDK definition adds a required context-contract version containing
read selectors (scope/type/field projections), resource rights, optional inputs,
and proposed write targets/value types. Configuration binds declared selector
slots to exact IDs; compiler expands transitive default/query dependencies and
checks them against grants. No wildcard `read all` or runtime-discovered reads in
v1. Host approval plus Library authorization is required; declaration alone is
not a grant. Built-ins follow the same contract. Existing nodes receive no new
context access automatically, and old manifest readers reject required D18 nodes.

Sensitivity is `ordinary | personal | restricted`; portability is separately
`portable | capture-consent-required | host-only`. Derived values inherit the
strictest inputs; there is no v1 expression declassification. Export/cloud copy
requires permitted policy and an explicit projection. A portable snapshot is a
disclosure boundary: removing a Library record cannot recall an already shared
Project. Required restricted inputs without capture permission block durable
replay rather than quietly persisting them in evidence. Error DTOs/logs contain
codes, opaque IDs and source spans, never raw secret values or private paths.

Secrets are **opaque host SecretRefs only**, not a persistable variable type.
Portable definitions can name a credential requirement slot, not a SecretRef ID,
token, hash of token, OS keychain identity, or binding. SecretRefs cannot be
compared/formatted/exported/logged and expression v1 cannot dereference them.
Only an explicit effect capability accepts the live handle. Such attempts and
tainted derived results are not reusable shared-cache entries; tainted values
cannot become variable proposals or portable output bytes. A host keeps any
grant generation in private attempt context, never hashes raw credentials.
Portable Run evidence records the requirement slot and `host-secret-required`
non-replayable marker, not a pretend complete secret snapshot.

Before scheduling, the host freezes `ContextSnapshot` with schema/interpreter/
query versions, scope IDs, exact definition/value revisions, effective-source
selection, AST refs, projected typed values, query membership/negative facts,
metadata/fingerprint refs and captured allowed host facts. Closure is bounded and
sorted by typed dependency coordinate; duplicate coordinates with differing facts
fail. Snapshot IDs/timestamps are provenance; `ContextContentDigest` covers its
canonical semantic payload, not its ID or capture time. If time is a requested
input it is a value and therefore hashed. Canonical bytes use unchanged S2 codec;
the digest material contains `domain: photara.context-content.v1` to separate it
from graph/AST keys. No runtime grants, absolute paths or raw secrets are hashed.

The Run start durably references the exact snapshot object/checksum and content
digest before attempts. Each node receives only its declared resolved subset;
its context digest covers that subset's complete semantic dependency closure.
Node-cache v2 combines existing key components with this digest, any declared
private DeviceContextDigest, contract and
interpreter/implementation versions. The graph evaluation key includes the run's
context/device digests even for an empty graph. Legacy entry points retain their existing
environment digest/key contract; adapters cannot silently mix v1/v2 caches.
Host execution-platform/tool versions remain explicit environment dependencies.
Frozen values never refresh mid-run. Changed unrelated Library variables do
not dirty a node that never declared them; changed declared values/metadata do.

Unavailable, forbidden, revoked, tombstoned, stale-fingerprint, ambiguous,
type-mismatch, cyclic, unsupported-version and limit-exceeded are distinct
diagnostics. Optional inputs may explicitly represent absence, not mask denial
or malformed values. Revocation cancels future capability use, including a retry;
it does not rewrite a historical snapshot or promise remote erasure.

## Writes, ordering, retries and evidence

Pure evaluation cannot mutate shared variables, Library or Project state. A node
may emit a typed `VariableChangeProposal` with target VariableId/owner, expected
aggregate and owner revisions, proposed typed literal, source snapshot/Run/
Attempt/output digest, and stable OperationId. The application validates the
declared write target, current authorization, sensitivity and all CAS preconditions.
The optional built-in **Set Variable** is the same proposal/effect contract, not
an evaluator exception or arbitrary target string.

V1 commits proposals only after the producing Run succeeds, through an explicit
Apply command (or previously approved automation policy with exact targets).
Failed/cancelled/interrupted runs never auto-apply. Effects already observed are
not undone by a later failure. Proposed writes do not change the current Run's
frozen reads; downstream nodes consume explicit typed outputs/graph edges.
Reading and proposing to the same variable is permitted only against that frozen
revision; implicit feedback/restart loops are forbidden. A subsequent Run needs
a new snapshot. Previewing a proposal is not accepting it.

One application command targets **one authority boundary**: one Library
transaction with all affected aggregate CAS checks, or one Project package commit
covering Project/Graph/node changes and all owner revisions. Cross-Library or
Library-plus-package atomic commands are unsupported in v1; no fake distributed
transaction. Independent steps require separate explicit commands/evidence and
may partially succeed. Duplicate writes to the same target in one batch fail;
concurrent runs cannot last-writer-win past CAS. Conflicts retain unapplied
proposals for explicit rebase with a new OperationId, never silent recomputation.

Durably record intent before application, and idempotency receipt plus changed
state in the owning transaction/publication unit. Retry with the same OperationId
and exact request digest returns the prior receipt; changed bytes under that ID
are rejected. An uncertain package publication is reconciled against the commit
chain before replay; local recovery queues are not caches. Run success and
proposal application are separate immutable facts: `proposed | applied | rejected
| conflict | application-unknown`. An offline Library apply is local success,
not server acceptance; S5 receipts/conflicts remain separate facts. L2's existing
mutation log alone does not implement this new apply protocol.

## Package, database, sync and compatibility amendment

Proposed package feature `photara.context.v1` introduces schema-selected authored
ObjectRefs for Project variable aggregates, immutable Library captures and
metadata selections; named Graph objects add Graph/node variable aggregates and
AST refs. History gains ContextSnapshot, metadata observations and proposal/apply
evidence schemas. Defaults/expressions and selected metadata affect authored
digests; captured Runs, extraction evidence and receipts do not change authored
Graph digests merely by being recorded. Inventories/HEAD closure must follow the
new references, not arbitrary JSON fields. Context objects use normal SHA/length/
path safety. No secrets, host bindings or active SQLite files enter a package.

Old one-JSON documents without D18 data remain valid and round-trip unchanged.
Do not sneak `context` into their currently rejected runtime extension fields.
Explicit conversion to the reviewed directory-package schema introduces empty
variable sets only when needed; enabling semantic D18 data adds the required
feature/minimum reader. L1 currently rejects it as unsupported. Preserve opaque
future objects for inspection/export; an old reader cannot claim runnable support
or perform a lossy save. Exact field/schema versions and compatibility fixtures
are frozen in CXT2 before changing validators or authoring new packages.

Physical design **deferred**, not added to existing SQL in this amendment:

- Library definition/value, reserved-name and expression/dependency records
  form typed aggregate-owned tables with same-Library FKs, type/schema checks,
  one-current-value and lifecycle/CAS guards. Multi-root mutation/change/feed
  material uses S5, not generic key/value persistence. Package variables remain
  package authority and are never mirrored as editable SQL roots.
- Optional local asset-metadata search projections are keyed by ProjectId,
  observed CommitId/checksum, AssetId, RepresentationId, fingerprint, metadata
  observation/schema/key. Rebuildable indexes never select truth or authorize a
  live project read. No default service-side asset metadata index; any future
  cloud summary needs an explicit privacy projection and separate schema gate.
- CXT2 must write ordered additive SQLite/PostgreSQL proposal DDL, migration
  ledger/floor changes, invariants, FK/delete policy, RLS/service allowlists and
  fixture counts. Use the clean Library-named baseline; no deployed database migration is required. No broad Core,
  existing-adapter or Storexa change is implied.

S5 gains versioned allowlisted variable commands/poststates only after CXT2;
exact sealed bytes, dedup receipts, local/server revisions, offline predecessor
chains, tombstone reservation and explicit conflict/rebase apply unchanged.
Expression data is validated, never executed by the sync service. Unauthorized
read dependencies do not become authorized because a mutation mentions them.
Portable Library variables and permitted captures can join D17 export/import
with ID/provenance/conflict reporting. Excluded/restricted dependencies yield
explicit unresolved placeholders; IDs are remapped only by a reviewed import
plan that rebinds AST references and changes digests. Never reactivate grants or
SecretRefs on restore. CloudKit stays an optional record adapter, not an evaluator.

## Module ownership and façade pane contract

Core owns schema/type validation, IDs, expression AST/type/dependency checking,
pure resolver/interpreter semantics, snapshots, proposals and cache-key versions.
Start with small modules; a later separate `photara-context` crate is an extraction
decision, not a required rewrite. NodeSDK owns exact declaration schemas and
validation. Host/application owns authorized resolution, extraction scheduling,
secret/resource grants, command orchestration and receipts. photara-store owns
package codecs/reference closure/publication; photara-library owns Library SQL
repositories and optional catalog indexes; Storexa owns database mechanics only.

Future immutable façade DTOs: ContextScopeDescriptor, VariableDescriptor,
EffectiveValuePreview (typed/redacted), DependencySummary, ExpressionDiagnostic
(code/span/expected type), ContextSnapshotSummary and ChangeProposalStatus.
List requests carry scope/owner/revision, filters and bounded cursor; previews
carry draft/request IDs plus exact base revisions and `live | captured | run`
source. Responses distinguish loading, ready, stale, unavailable, forbidden,
unsupported, conflict and failed. Native clients discard superseded responses.
Commands: Define/Edit/Tombstone Variable, Validate/Preview Expression, Capture/
Refresh Context, Prepare Run Overrides, Apply/Reject Proposal. Every mutation
returns revisions/structured conflict facts. No SQL, raw secrets or object graphs
cross the façade.

The future **Context/Variables pane** is a reusable state consumer inside Graph authoring composition, linked to optional Inspector details. Selection, expansion, filter,
draft text and pane placement are client-only; persisted values go through typed
commands. It shows explicit scope/effective-source/type/provenance/dependencies
and permission/read-only facts, not a merged ambiguous dictionary. This specifies
state, not appearance: no layout, SwiftUI, Windows control or raster mockup is
implemented; visual work still requires separate raster approval.

## Review decisions and phases before L3

Approve D18 details separately: (a) explicit qualification/account preferences,
(b) reserved names/one-CAS definition-value aggregate and literal run overrides,
(c) bounded AST language/types/limits, (d) snapshot capture versus live refresh,
(e) secret non-replayability and privacy defaults, (f) metadata disagreement policy,
(g) post-success single-authority proposal application, (h) required package/key
versioning and additive physical schema. Concept approval does not pre-approve
these details or their implementation.

1. **D19 consistency / revised CXT0:** review D19 and these revised D18 semantics;
   leave inert baseline fixtures unchanged until an exact reviewed addendum.
2. **Contract freeze before code:** logical Library/access/ledger/value contracts,
   package schema requirements and NodeSDK declarations/composition are settled.
3. **CXT2 static physical/portable review:** exact package fields, compatibility,
   SQLite/PostgreSQL additive/rename plan, API/RLS/project-scoped sync/export and
   proposal idempotency/recovery. Review counts and fixture deltas without DDL.
4. **Revised CXT1 — separately authorized pure foundation:** implement the frozen
   Core/NodeSDK types, bounded parser/AST/interpreter, declared input/context
   checks and cache-key tests; no database, host effects or UI.
5. **CXT3 — separately authorized disposable adapters:** reviewed codecs and
   additive Library migrations/repos, permission/project-only capture tests and
   facade DTOs. Use the clean Library-named baseline; no deployed database migration is required. Temporary roots/DBs only.
6. **Resume L3 after CXT1–3 acceptance:** create one-Library packages using the
   settled format and prove staged publication/receipts. Package-target proposal
   application joins L3 crash tests. L2b remains independent and unexposed.

Acceptance: explicit scope/collision and cyclic dependency failures; type and
numeric bounds; path/grant denial on macOS/Windows fakes; metadata mixed/missing/
stale evidence; secret nonserialization/noncacheability; declared subset/cache
invalidation; immutable retries/Run overrides; CAS conflicts/idempotency/uncertain
apply; offline/sync/import privacy; old one-JSON/L1/L2 behavior; future-version
refusal; delayed preview DTOs; no UI appearance implemented.

[Inert D18 cases and byte vectors](../fixtures/generation-two/context-amendment.json)
contain 38 scenario specifications and four proposed semantic byte/hash vectors.
They are a separate proposal addendum, not changes to the 51 S6 scenarios or nine
frozen canonical-codec vectors. Static parsing/hash/link checks are evidence only
of artifact integrity, not interpreter, authorization or database conformance.
