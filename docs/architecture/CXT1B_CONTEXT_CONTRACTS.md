# CXT1b — pure typed context contracts

Historical slice/review record. Current naming and gates are governed by the
[Library rebaseline](LIBRARY_NOMENCLATURE_REBASELINE.md) and
[execution roadmap](../ROADMAP_0_2_EXECUTION.md); earlier byte-preservation and
next-step labels below refer to their original verification date.

Status: **complete, 2026-09-12**, after Suhail separately selected CXT1b.
This is the implementation record for [D18](TYPED_CONTEXT_AND_EXPRESSIONS.md),
the [accepted D19 freeze](D19_CONTRACT_FREEZE.md) and its
[bounded implementation gates](D19_STATIC_SCHEMA_DELTA.md#fixture-delta-implementation-slices-and-gates).
CXT1a and CXT2 remain preserved baselines. Next eligible slice is separately
selected CXT3a; CXT3b/c and L3 remain unstarted.

## Additive API and ownership

The new entry point is `photara_core::context`. Only its declaration was added to
existing Rust source. Existing Core contracts, graph evaluation and cache keys,
NodeSDK v1/v2 validators, AssetSet v1/v2 digests, package codecs, library/store code,
Cargo manifests/lockfile and Swift sources are unchanged. No new dependency is used.

| Module | Public boundary |
| --- | --- |
| [context](../../crates/photara-core/src/context/mod.rs) | Versioned bounds, redacted error codes and UTF-8 half-open spans with derived one-based line/scalar columns |
| [value](../../crates/photara-core/src/context/value.rs) | Closed scalar/composite/reference/resource payloads, exact type/schema descriptors, host-supplied TypeRegistry, recursive variable admission |
| [expression](../../crates/photara-core/src/context/expression.rs) | Explicit field admission, ID-bound coordinates, directional binding environment, typed AST/source records, compile/verify and exact dependency extraction |
| [parser](../../crates/photara-core/src/context/parser.rs) | Private bounded lexer and precedence parser; no public unchecked executable AST |
| [interpreter](../../crates/photara-core/src/context/interpreter.rs) | Private budgeted interpreter; public evaluated result retains sensitivity, portability and source/context digests |
| [variable](../../crates/photara-core/src/context/variable.rs) | Checked one-CAS aggregates, stable names/IDs/value IDs, proposed transition validation, default/current/Run override resolution and transitive cycle checks |
| [snapshot](../../crates/photara-core/src/context/snapshot.rs) | Immutable context captures, exact dependency closure, consent/audience checks, completeness/replayability, private device captures and frozen expression-bound subsets |
| [metadata](../../crates/photara-core/src/context/metadata.rs) | Bounded queries over supplied immutable AssetSet selections and metadata observation references, including negative facts |
| [cache](../../crates/photara-core/src/context/cache.rs) | Separate node/graph cache v2 material and current-authorization lookup validation; no cache store |
| [proposal](../../crates/photara-core/src/context/proposal.rs) | Validated literal proposals, single-authority Apply planning, immutable receipt observations and derived application knowledge; no application or dispatch |

Construction records are inputs, not authority. Use checked aggregate conversions,
`Expression::verify`/`from_json`, `ContextSnapshot::build`/`from_json` and the exact
TypeRegistry validation path before accepting untrusted records. `compile_registered`
checks host-registered type/schema associations; `compile` takes an already trusted
binding/type environment. A supplied declaration, authorization observation or
source receipt is not authenticated by these pure functions. The host/controller
still obtains current permissions and verifies external evidence in CXT3.

The NodeSDK field modes retain their existing exact wire spellings. A future caller
maps its declared field mode and registered schema into this Core entry point;
no adapter silently activates expressions in current node configuration or v1 APIs.

## Source language and values

`CompiledField::compile` requires an explicit `literal-only`, `expression` or
`template` mode. Literal-only fields return a typed literal and never create an
expression record. Expression records accept expression/template containers only,
as required by CXT2. Ordinary descriptions and `${...}` have no special meaning.

Expression mode requires exactly one single-backtick expression with only optional
surrounding whitespace. Template mode alternates literal spans and single-backtick
expressions. In template literal spans, backslash-backtick means a literal backtick
and double backslash means one backslash; other escapes fail. Expression strings
use JSON escapes and encode a literal backtick as `\u0060`. Empty/unclosed or nested
delimiters and trailing expression text fail. Initial triple-backtick containers
return `unsupported-version`; multiline template execution is deferred. CRLF source
bytes are retained exactly. Surrounding expression whitespace is accepted.

The grammar includes bool/string/i64/null literals, homogeneous bounded lists,
registered record fields, explicit qualified references, parentheses, `+ - *`,
`== != < <= > >=`, `and`, `or`, `not`, unary minus, and
`if(condition, then_value, else_value)`. Both branches are statically typed and
included in dependencies; only the selected branch executes. Division, floats,
assignment, loops, user functions, reflection, regex, process environment, clocks,
randomness, shell/process execution and provider access are absent.

| Intrinsic call | Exact implemented behavior |
| --- | --- |
| `text.format(value, "und", 1)` | Explicit invariant-locale formatting version 1 for bool/string/i64/decimal/timestamp/enum; other locale/version pairs are unsupported |
| `optional.or_else(optional, fallback)` | Lazy fallback for explicit absence only; denial, revocation, malformed values and unavailable facts are never converted into absence |
| `path.join(resource, component, ...)` | Narrows a portable resource or captured slot address using CXT1a relative-component rules; no path resolution or added rights |
| `assets.count($input.port)` | Counts the exact supplied AssetSet v2 descriptor; no inventory or selected-window fallback |
| `metadata.values(input, field, selector)` | AssetId-ordered rows with value/missing/unavailable/ambiguous/conflict/stale/forbidden/revoked status and explicit optional value |
| `metadata.common(input, field, selector)` | Requires a nonempty input and identical resolved typed values for every member; no first-value or majority choice |
| `metadata.distinct(input, field, selector)` | Canonically ordered distinct typed values with contributing AssetIds and explicit issue rows |

Metadata field and versioned selector names must be literal qualified names in a
host-declared query binding. The input must be the current node's explicitly bound
AssetSet port. The supplied capture includes the exact immutable AssetSet, selected
representation/content revisions and observation ObjectRefs. Membership is complete
and duplicate-free; query/input descriptor disagreement and stale selections fail.
The module does not extract metadata or prove the truth/content of referenced
observation objects; package/host evidence verification remains CXT3.

I64 values use tagged canonical decimal strings and checked arithmetic. Decimal
values have a canonical signed coefficient of at most 38 digits and explicit
scale 0–18; there is no decimal arithmetic or implicit numeric coercion. Rational
values retain signed i64 numerator, positive i64 denominator and exact unit.
Timestamps are canonical UTC `YYYY-MM-DDTHH:MM:SS.mmmZ`, years 0001–9999, with calendar
validation and no leap-second/offset coercion. Decimal, timestamp, enum, rational,
optional and reference literals can be supplied as validated typed bindings;
quoted expression strings do not silently convert into those types. Optional
absence and schema-admitted null are distinct. Unknown required types/versions fail.

## Binding, variables and immutable evaluation

Compilation binds names to exact typed VariableId/scope coordinates and registered
field projections against an explicit owner revision. Source spelling is retained
for editing, but runtime evaluation performs no name search. Rename cannot retarget
an AST. Source SHA-256 covers exact UTF-8 bytes; semantic AST digest excludes source
spans/formatting and expression identity while retaining literal payload fields,
types, opcodes, IDs and projections. Verification recompiles against the recorded
binding context and checks source, AST and dependency correspondence. Unknown fields,
versions, opcodes and duplicate JSON keys fail closed.

Direction is Library → Project → Graph → current Node, with same-Library and
same-Project/Graph restrictions. Library references cannot reach child scopes;
Graph/Node references cannot reach siblings. Input ports are current-node coordinates.
Asset metadata requires an explicitly bound AssetId/representation/content revision.
Host places are the six CXT1a uppercase symbols, never environment-variable lookup.
Slots bind SlotId and capture revision plus StorageLocationId; later retargeting does
not change a frozen slot address. Resource roots do not become package write paths.

Variable definitions/defaults/current values share one checked aggregate revision.
Names remain reserved across rename/tombstone, type/schema/namespace/owner identities
remain stable, clear retains ValueId, and proposed transitions require exact CAS
bases and revision advancement. There is no resurrection or implicit type migration.
Cycle detection includes current bindings, defaults, fallback and branch references,
even when an explicit value or override would hide a cycle during one evaluation.
Effective precedence is permitted literal Run override, explicit value, default,
then unavailable. Run override coordinates and producing Run remain in captured
facts; another Run cannot reinterpret them as its own overrides.

Snapshots validate bounded, canonically ordered exact facts and full transitive
closure. Duplicate, omitted or unrelated extra closure entries fail. Negative facts,
revisions, effective sources, AST digests and projections participate in semantic
digests. The snapshot's own ID and capture timestamp are provenance and excluded;
an explicitly requested time value participates as ordinary input. Live previews
cannot become durable captures. Stale/unavailable required facts produce incomplete,
blocked replay state rather than an apparently complete snapshot.

Sensitivity is monotone ordinary/personal/restricted; portability is independently
portable/capture-consent-required/host-only. Captures enforce inherited labels and
exact projection/audience consent. Host-only payloads cannot enter portable snapshots;
restricted payloads cannot enter shared-project snapshots. Widening an audience
requires new disclosure approval. Secret requirement names can mark replay blocked,
but credentials, SecretRefs, grants, paths and their hashes have no value variant.

A frozen subset binds the exact expression digest, owner, Library and Run and has
no mutation or serde API. `EvaluatedValue` carries privacy labels and exact expression/
context digests and has no serializer. Device captures hash nonsecret binding identity,
positive i64 generation and availability; TEMP additionally binds its lease nonce and
Run. DeviceContextSnapshot has no portable serializer and Debug is redacted. These
records do not resolve a host path or establish continuing authorization.

## Cache and proposal contracts

Node cache v2 covers exact definition/package/implementation coordinates, configuration,
authored state, ordered typed inputs, context/interpreter/contract versions, environment,
complete declared context projection, authorization boundary and any declared device
digest. Input and private Node dependency coordinates must match the target Node/Graph.
Unrelated Library values do not invalidate an undeclared subset. Graph keys include
context/device/environment digests even for an empty graph. Effects, nondeterministic
and secret-tainted work cannot create reusable keys; device context cannot enter shared
Project cache entries. Lookup separately checks current Project/principal/generation/
audience/sensitivity authority and device digest; matching content is not permission.
No v1 key path or current evaluator is changed, and no cache entry is read or written.

VariableChangeProposal contains exact target/owner/CAS coordinates, typed literal,
producing Project/Run/Attempt/snapshot/output evidence and stable ProposalId/OperationId.
ApplyPlan checks successful Run, explicit acceptance, exact source evidence, declared
currently authorized targets, sensitivity, type and every CAS precondition. Duplicate
writes and mixed Library/Project authorities fail. The result is a plan only; no
transaction, publisher, journal or effect dispatcher exists here. Receipt observations
retain exact operation/request/authority and prior receipts. Unknown is distinct from
failure; contradictory terminal observations yield disputed knowledge, and changed
request bytes fail even after an earlier dispute. No reconciliation is performed.

## Bounds and evidence

| Boundary | Maximum |
| --- | --- |
| Authored field source | 16 KiB UTF-8 |
| AST | 1,024 nodes, depth 32, 64 KiB canonical bytes |
| Direct dependencies | 256 per expression |
| Expanded closure | 10,000 facts; no truncation |
| Literal/value | 64 KiB canonical payload |
| Snapshot | 1 MiB canonical bytes including its envelope |
| Metadata query | 10,000 assets, independently subject to value/snapshot limits |
| Interpretation | 100,000 counted operations per expression |

The independent byte/shape limits can reject an input before it reaches a count
ceiling. Diagnostics distinguish unknown, forbidden, revoked, tombstoned, unavailable,
stale/fingerprint, ambiguous, conflict, type, cycle, version, scope, consent, source/AST,
overflow and limit failures. Error DTOs contain codes/spans only, never source values,
private paths or secret text.

The separate [d19-context.json](../fixtures/generation-two/d19-context.json) is generated
and verified by the unchanged Rust canonical encoder. It contains a typed template,
source/AST hashes, immutable capture, evaluated result labels and node/graph cache v2
keys in the reserved D19 synthetic UUID namespace. Normal tests compare exact bytes;
the ignored generator writes only this new file. Its SHA-256 is
`6e49a4156fa664f3844372ccdb1932a04e681abec48b762386a81a1826a6dcd4`.
Every previous fixture, including
CXT1a's `d19-contracts.json` and the fixture README, is untouched.

Verification passed offline: **57 new integration tests plus three new serialization
compile-fail tests**, alongside the retained CXT1a/Core/SDK/node suite, for **159 passed**.
Both golden generators remain ignored in ordinary runs; the new generator passed
separately. The 57 tests include a deterministic 441-case arithmetic property matrix,
source/escaping/spans, AST mismatch/version checks, scalar/collection/operation limits,
renames/cycles/defaults, capture privacy/closure/freshness, Run overrides, metadata
negative facts, cache dirty/non-dirty/auth boundaries and proposal/receipt planning.

```sh
cargo fmt -p photara-core --check
cargo check --offline --workspace --all-targets
cargo clippy --offline --workspace --all-targets -- -D warnings
cargo test --offline -p photara-core -p photara-node-sdk -p photara-layout-node -p photara-asset-set-node -p photara-disk-node
cargo test --offline -p photara-core --test context generate_d19_context_golden -- --ignored
```

The full library was compiled/linted; database suites were not executed. Retained
node tests use disposable filesystem fixtures. The pre-slice audit confirms 317 pre-existing files are byte-identical and verifies CXT1a sources,
tests and implementation record, all prior fixtures, six applied L2 migrations,
14 CXT2 proposal files and unrelated existing files. Git whitespace and local Markdown
link/anchor checks pass (232 local links/anchors checked). No DB/SQL, resolver, package codec/publisher, Swift/UI, user
Project/SMB, service, staging, commit, push or release action occurred.

## Next eligible slice

Stop at CXT1b. Separately select **CXT3a** for package 1.1 reader/closure validation and
explicit compatibility mapping DTOs in disposable roots. CXT3b owns new local
migrations/repositories and fake host/apply conformance; CXT3c owns disposable service,
RLS and scoped sync proof. CXT1b does not certify those adapter, authorization,
filesystem containment, durability or publication scenarios. L3 remains paused.
