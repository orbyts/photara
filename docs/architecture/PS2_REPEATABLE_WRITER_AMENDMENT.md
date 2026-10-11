# PS2 repeatable writer amendment

Status: **approved for bounded additive implementation and exact-byte validation**,
2026-10-03. Implementation exposed a concrete
compatibility gap in the narrowly frozen packet at `9b7facf`. Existing approved
bytes and codec meanings remain frozen. This is not a new fixture workstream.

## Exact gap

The [admission candidate](PS2_SELECTED_ADMISSION_CANDIDATE.md#independent-complete-layout-regeneration)
explicitly describes one next operation with prescribed specimen IDs and fixed
corridors. In [the reviewed decoder](../../crates/photara-store/tests/selected_admission_candidate/decoder.rs):

- Line 372 selects active root UUID ending `8001` (`8201` for pending).
- The following association loop selects IDs from a fixed `8000`/`8200` base.
- `aid(1..9)` and `STEP = 262_144` prescribe the physical allocation topology.

The existing published golden HEAD selects a StateRoot with ID
`96000000-0000-4000-8000-000000008001`. Repeating that recipe would use this ID
again for a different new active state while the old active becomes recovery.
The frozen reader correctly rejects two different retained states sharing one
root ID (`unique retained root UUID`). Single-birth similarly cannot reuse its
already occupied newborn allocation ID. No additional furnace is needed to
establish these direct conflicts.

The original codec promises deterministic reconstruction of exact append bytes.
Changing its identifier/topology choices silently would change those bytes under
an already frozen codec. The earlier consolidated packet should have called out
this limitation before requesting a freeze for production implementation.

## Approved bounded amendment

Authorize a separately versioned repeatable planning recipe, preserving all
existing frozen codecs for exact decode/replay. Approved additive dispatch names:

- Original codec: `photara.codec.ps2-repeatable-admission-v1`.
- Layout codec: `photara.codec.ps2-repeatable-layout-v1`.

Retain the existing O/P/F schemas, semantic target, nonce, original operation and
receipt identities, generation binding, retained authority, accounting and
publication order. The new codec's original recipe must explicitly persist the
planner choices that the specimen previously hardcoded:

| Input | Required meaning |
| --- | --- |
| New semantic IDs | Fresh active-root UUID and exact old-to-new association-ID mapping; check uniqueness against retained roots and immutable associations before admission. |
| Allocation roles | Exact selected allocation IDs for each target root's data and locator roles, plus root-placement metadata; verify arena, original witness and allowed extent. No implicit `aid(n)` positions. |
| Tree geometry | Explicit deterministic leaf/branch capacities and canonical ordering rules used to regenerate the admitted trees. Values are bounded implementation inputs, not product defaults. |
| Finite append corridors | Original prefix digest/extent and maximum end for every writable allocation, plus any already approved generation-plan/newborn binding inputs. No fixed 256-KiB assumption. |

Generate these choices once before admission and retain them under the original
O commitment. Retry/recovery must reproduce the same bytes, identities and
reserve; never generate replacement IDs after an unknown outcome. Reject
collisions, unsupported topology, arithmetic overflow and plans that do not fit
preflight capacity/control bounds before effects. Existing single-birth/pending
acyclicity rules remain mandatory; the new recipe must not prehash an inventory
that depends on its own original admission or future native witness.

Implement and validate this additive recipe using the existing golden and
selected-phase test infrastructure, including a second consecutive operation
and interruption/retry of that same original attempt. Preserve the old golden
bytes and their refusal behavior. Do not add a fixture family or generalize
unsupported semantic commands, undo/provenance spellings or retention policy.
The exact new field layout and canonical vectors must be recorded with the
implementation; none may be silently treated as part of the earlier freeze.

## Approval recorded — 2026-10-03

The user explicitly approved this bounded additive amendment and persisted-input
approach for implementation and exact-byte validation. Planner inputs must be
generated once before admission, persisted under the original commitment, strictly
validated before effects and reused exactly for retry/recovery. Required validation
includes at least two consecutive operations and interruption/retry of the same
original attempt using the existing infrastructure. No further approval is needed
within this scope; stop only for a new concrete correctness/compatibility boundary.
The original freeze stays intact; changing existing codec meanings is not an
alternative. Storage qualification follows implementation, then PS3 → PS4 → LL2a.
This does not authorize real-library writes, migration, deletion, deployment,
automatic conversion, expiry, new product limits or weaker Accepted/Saved claims.

## Shared implementation layout

The additive implementation is in `package::v1_3::repeatable`. The old candidate
decoders and canonical corpus files remain unchanged. The new original O uses
`kind: graph` and preserves the existing `{root, value}` rows in `old.states`.
Its `payload_recipe.planner` contains the following exact persisted inputs:

| Field | Representation |
| --- | --- |
| `token`, `nonce` | Original decimal token and lowercase SHA-256-shaped nonce. |
| `selectors` | Seven ordered `{commit_id, write_id}` pairs: admitted, journal-durable, payload-durable, finalizer-selected, finalizer-durable, published, clean. |
| `planner.codec` | `photara.codec.ps2-repeatable-layout-v1`. |
| `planner.active_root_id` | Fresh active StateRoot UUID. |
| `planner.associations` | Exact `{original, replacement}` association UUID mappings. |
| `planner.roles` | Ordered `{role, data, locator}` bindings to existing registered allocations. |
| `planner.root_placement`, `planner.shared_sealed` | Root-placement allocation UUID and shared sealed allocation UUID list. |
| `planner.geometry` | Integer capacities `semantic_leaf`, `operation_leaf`, `ownership_leaf`, `physical_leaf`, `branch`. |
| `planner.corridors` | `{allocation_id, arena, witness, original_end, original_sha256, maximum_end}` for each writable allocation. Extents are canonical decimal strings. |
| Admission bounds | Canonical decimal strings `project_limit`, `cleanup_bound`, `charge_unit`, `aggregate_control_bytes`, `aggregate_control_count`, `phase_bytes`, `finalization_bytes`. |

All input structures reject unknown fields. `charge_unit` must equal the
independently supplied registration policy; persisting a smaller caller value
cannot reduce the reservation. IDs, allocation partition, witnesses, original
prefixes and finite growth are checked before effects. Geometry is an explicit
implementation input, not a production product limit. A singleton final tree
group carries its existing child upward instead of creating an invalid one-child
branch; this rule belongs only to the additive layout codec.

The recipe also records typed prior-control dependencies in `retained_controls`.
These preserve exact completed O/P/F, request and hold evidence across the next
admission, including restart. They are charged packed bytes, not an expiry or
permission to drop liabilities. A completed hold must independently prove zero
remaining reserve before another operation is admitted. Implementation checks
and canonical-vector results are recorded below.

### Disposable control-space parameter

The unchanged integrated specimen reserves 131,072 bytes for standing controls.
The second operation's original admission needs 151,552 bytes while its previous
terminal evidence and new controls coexist (20 records plus three scratch roles).
Refusal at the smaller bound is correct. Consecutive-operation tests therefore
register 262,144 bytes **before their initial admission**, adding the same
131,072-byte difference to the initial total charge and coherently rebuilding
the loose ledger, envelope, inventory overlay and selector. The checked-in
specimen bytes remain untouched. This is a disposable test parameter, not a
production default, an automatic allowance increase or permission to undercount
previous operation evidence.

### Exact additive vectors

`repeatable::tests::two_consecutive_operations_resume_the_same_original_after_partial_append`
pins these canonical SHA-256 fingerprints:

| Operation | Fingerprint |
| --- | --- |
| First Graph update | `b56de6aa7dfa328f00b55d2263c1912e13146a4537ef110d5a985542c9ce9e82` |
| Consecutive Graph no-op, new operation identity and receipt | `fdbbf81b9a6f5bd6472a2b515f45b1f029fa91576401745a576dd49c447e1915` |

Each fingerprints canonical JSON with `original_sha256`, seven ordered `stages`
(each containing canonical HEAD/commit hashes and the control digest-to-decimal-
length map), and the allocation-ID-to-exact-byte-digest map. Thus the assertion
covers every admitted original, selected phase and allocation output rather than
only a final semantic result. The second operation starts from the first clean
publication. Both are interrupted during an append and reconstructed from their
persisted original records and observed allocation prefixes. The second restart
also reconstructs its prior completed operation without an in-memory plan. Exact
retry of the first request after the second returns the first original receipt.

## Verification checkpoint — 2026-10-03

The final affected command `cargo test -p photara-store --lib --test
ps2_frozen_codec --test ps2_frozen_reader --test package_v1_1 --quiet` passes
145 tests: 21 library, 4 frozen codec, 5 composed reader and 115 legacy package.
Eleven preexisting cases and the manually invoked native image test remain opt-in.
Checks cover before/after selector uncertainty, failed retry barriers, registration/
capacity/collision/corridor refusals, exact retry after subsequent operations and
persisted two-original restart. The [separate native observation](PS2_MACOS_CLEAN_REMOUNT.md#shared-repeatable-rust-path--2026-10-03)
also passes and retains its image detached. Frozen corpus/generator/decoder hashes
remain unchanged (all 50 entries in the existing manifest).

The reader caches framing indexes only within each borrowed immutable allocation
view; each accessed frame still checks its hash, tag and canonical bytes. It does
not turn a prior audit or a persisted ledger into current native authorization.
The repeatable execution result deliberately cannot mint production Accepted/Saved.
Unsupported pending-origin authority, Blob mutation and newborn/retirement
execution remain outside this bounded repeatable path; no compatibility support is
claimed merely because a previous candidate fixture demonstrated those variants.

Strict library/tests Clippy, targeted Rustfmt and diff checks also pass. The
[source-bound verification manifest](verification/ps2-shared-repeatable-regression.json)
records the commands, exact source hashes and output logs.

## Incremental layout amendment — approved 2026-10-10

The current disposable writer is not ready for sustained autosave. Its latest
attempt refused before admission with `preflight corridor exhausted`; no Mutation
was acknowledged and no new original was persisted. The selected state remains
Saved at position `(13, 14)`. Seven writable tips each have 3,145,728 prepaid bytes
remaining, but twelve 262,144-byte physical units do **not** imply twelve admissible
operations: every generated append must independently fit its original corridor.

The preserved active-locator append sizes for the last five completed operations
were 235,320, 240,106, 251,351, 253,985 and 259,654 non-padding bytes. The last leaves
only 2,474 bytes after the required padding-frame header. These are measured
completed outputs, not a claimed exact size for the refused attempt: that
attempt's randomly chosen planner IDs were not persisted. Its request and refusal
are retained in the private trial evidence; no fixture rewrite or cap increase is
proposed.

### Existing permission versus new compatibility boundary

Fresh geometry and finite corridors are already approved explicit inputs. The
adapter's fanout four and uniform 262,144-byte corridors are fixture choices,
not frozen format requirements. Choosing better inputs before a new admission
can reduce overhead; it cannot change an already admitted original. Such tuning
has not been measured here and is not evidence that history scaling is solved.

`repeatable/layout.rs` currently sorts and rechunks the entire logical tree in
`Builder::tree` and the entire locator in `Builder::physical_tree`. It reuses exact
matching pages, but inserted hash keys shift chunk boundaries and rewrite many
otherwise unchanged entries. Global inventories and every role locator include
all retained terminal controls. Wider pages reduce constants; wider corridors
only postpone refusal. CPU memoization cannot remove these serialized bytes.

Changing that deterministic construction under layout v1 would invalidate retry:
`repeatable/recovery.rs::recover` recompiles the persisted inputs and requires exact
original equality. A path-copy implementation therefore needs additive dispatch;
existing v1 outputs, parser behavior and golden vectors must remain unchanged.

### Approved additive contract

The user approved this concrete packet for implementation and validation on
2026-10-10. Approved dispatch names are
`photara.codec.ps2-repeatable-admission-v2` and
`photara.codec.ps2-repeatable-layout-v2`. Implementation is in progress. All v1 dispatch,
bytes, refusal and retry rules remain frozen.

All O/P/F fields retain their representation and meaning except:

| Location | v2 value/change |
| --- | --- |
| `O.original_codec` | `photara.codec.ps2-repeatable-admission-v2` |
| `O.payload_recipe.codec` | `photara.codec.ps2-repeatable-layout-v2` |
| `O.payload_recipe.planner.planner.codec` | `photara.codec.ps2-repeatable-layout-v2`; the outer planner value remains the existing persisted Attempt. |
| `O.payload_recipe.retained_controls` | Removed and forbidden in v2. |
| `O.payload_recipe.retained_control_inventory` | Exact `{base, additions, result}` object below. |
| `F.recipe_codec` | `photara.codec.ps2-repeatable-layout-v2` |

Unknown fields/codecs refuse. No v1 alias/fallback, journal changes, package
version changes, new geometry field or new product bound is introduced.

`base` is exactly one of:

- `{"kind":"empty"}` only when the independently verified predecessor has no
  retained operation controls.
- `{"kind":"legacy","original":Ref}` where the ref is the actual selected,
  completed v1 predecessor O; its authenticated flattened retained list is the
  seed. An arbitrary historical original is not eligible.
- `{"kind":"tree","root":Ref}` where the ref equals the actual selected,
  completed v2 predecessor's `retained_control_inventory.result`, resolved from
  old packed bytes. This is a typed edge to the inventory and all its entries.

`additions` is the exact sorted array of `{record:Ref,membership}` for predecessor
selected overlay controls retained by the current v1 rule. It is empty for the
initial settled case. Membership is `semantic` or `ownership`, derived from the
actual schema; this graph-only path admits no new Blob controls. Equal keys
coalesce only with equal membership, otherwise refuse. The full seed plus these
additions is the exact retained set. No old entry expires. Prior holds still
require independent terminal/zero-remaining proof; an inventory is not authority.

`result` is the ObjectRef of the new inventory root. Before payload publication
it is a reconstruction commitment, like the existing recipe's `base_inventory`.
Published full proof must resolve it and all typed edges from actual selected
bytes. Nodes are packed/charged with ownership membership in every role's
independent locator and included in the exact global inventory. Current O is not
in its own inventory; it enters the next original through `additions`. No self-hash
cycle or embedded-body substitute for missing packed bytes is permitted.

Two exact new node schemas are proposed. Both have fields
`schema:{id,version:1}`, `project_id:UUID`, `count:DecimalU64`, the array below,
and `extensions:{}`; unknown fields refuse. IDs are
`photara.storage.retained-control-inventory-leaf` and
`photara.storage.retained-control-inventory-branch`.

| Node | Array and invariants |
| --- | --- |
| Leaf | `entries:[{record:Ref,membership}]`; strictly increasing `(sha256,numeric byte_length)` keys; count equals length. Empty is allowed only at the root. Each record is a typed edge with its recorded membership. |
| Branch | `children:[{first:Ref,last:Ref,count:DecimalU64,child:Ref}]`; at least two children; exact positive subtree counts, disjoint ordered ranges and checked count sum. Each child is a typed inventory-node edge. |

All nodes match Project identity. Existing JSON/object/frame/tree/work budgets
apply before allocation. Leaf capacity is persisted `semantic_leaf`; branch
capacity is `branch`. Reuse never exempts a node from schema/identity/budget checks.

### Deterministic incremental layout

Seed trees come only from the authenticated original: operation indexes from old
active indexes; semantic inventories from their corresponding predecessor; global
inventory from the old overlay base; ownership and locators from the original
same-role placement in the same destination allocation; root placement from the
old selected root-placement tree. Missing eligible seeds use canonical empty
leaves. Reused physical references must remain in the new role's exact ownership
closure; otherwise pack a valid new location. No foreign-role shortcut is allowed.

Derive the unchanged exact target set, including the new inventory nodes. Apply
one delta per key in ascending order: remove absent keys, replace differing values,
insert new keys. ObjectRefs sort by `(sha256,numeric byte_length)`, ordinals
numerically, UUIDs by existing canonical string order. Equal entries/subtrees
retain exact bytes/references. Descent selects the first child whose last key is
at least the update key, otherwise the last child. Copy only changed paths.

On overflow split the ordered `capacity+1` entries/children at `floor(length/2)`,
left then right. Remove empty children and collapse singleton branches; an empty
root becomes the canonical empty leaf. Do not merge underfull neighbors or globally
rechunk. Apply the deltas in canonical key order in memory, then emit only the
final reachable changed nodes bottom-up, left to right (postorder), retaining
existing digest deduplication and frame encoding. Superseded intermediate nodes
within this unadmitted computation are not serialized. This clarifies the first
draft's ambiguous “in delta order” wording before any v2 package vector or effect;
delta application order is unchanged. For logical frame emission, keep the existing
non-tree object slots/order and fill only changed-tree slots by deduplicated
first-occurrence postorder. Forest order is retained inventory, operation-ID,
operation-ordinal, active semantic inventory, global inventory, then ownership
in persisted role order. Physical locator/root-placement frames emit postorder.
This makes the approved traversal rule explicit before pinning complete vectors.
Refuse geometry smaller than an eligible seed node rather than silently rebuilding it. Count/range/seed inconsistencies refuse before effects.

Persist planning choices once under O. Recovery dispatches by that original and
reproduces exact append bytes, identities, reserve and selectors. Complete plan
and control coexistence must fit unchanged admitted/registered bounds before any
effect. Neither larger corridors nor geometry tuning is part of this fix.

### Draft serialization vectors

Editorial correction during implementation: all ObjectRefs retain their existing
required `kind:"json"` member. The first draft examples accidentally omitted it;
the corrected hashes below follow the packet's unchanged `Ref` type. No new
reference type or compatibility rule is introduced.

These isolated node vectors specify canonical bytes/splitting, not complete
package proof. Project is `10000000-0000-4000-8000-000000000001`. Entry A has record
SHA-256 of 64 `a` characters, byte_length `"1"`, membership `"ownership"`; B uses
64 `b` characters, length `"2"`, membership `"semantic"`. With leaf capacity one,
inserting B into A yields leaves A/B and their two-child branch. The branch child
ranges are the corresponding record refs, child count `"1"`, and child refs are
those leaves' hashes/lengths. Every reference includes `kind:"json"`. Existing canonical sorted-key UTF-8 JSON has no
whitespace or trailing newline.

| Object | Bytes | SHA-256 |
| Empty leaf | 172 | `cc2273172590226214739fca9d25ea8d9c2e1e9d288229b5c053d8750fb08c5a` |
| Leaf A | 317 | `9589cf84a0d58cb4f401f20642f008a18d640bf54c0a1dc7fee1a66071dc6549` |
| Leaf B | 316 | `d127162ff7aa3dc9634f238be09763082ee4a099a4c58f90e3cce93d6b090090` |
| Branch A/B | 912 | `221400d6eb5f4c6f3cef37fb37f59577bd127663170bd6520450122597386ecb` |

### Required implementation and acceptance

The retained inventory and path-copy changes are one cohesive amendment; neither
alone is claimed to solve autosave. Implement bounded exact-input reuse of
validated immutable subtrees/history so new commands do not rematerialize every
prior original. Initial/reopened audit remains complete. Every request still
checks actual current observations, registered identity/charges, authorization
and barriers; no persisted or memoized proof can mint Saved. Changed bytes,
limits or observations invalidate affected reuse.

Approval authorizes implementation of this packet and exact-byte validation in
existing infrastructure, followed by measured disposable acceptance. Required
gates: unchanged v1 vectors/recovery; exact retained set and liabilities;
independent role recovery; split/removal/order/malformed-input refusals; exact
interrupted retry; and same-history measurements of append bytes, work, peak
memory and total save latency. Publish complete v2 package vectors with results.
Stop for review only if implementation reveals a material change to the exact
fields or authority semantics above. No native writes resume merely because
isolated serialization vectors match. No new fixture family is needed.

No deletion, retention expiry, history/undo truncation, cap increase, weaker
Saved/barrier claim, migration or real-library write permission is included.
Existing v1 bytes remain unchanged; new v2 attempts may reference their verified
legacy authority. Finite physical capacity can still refuse. Desired latency
remains an acceptance test, not an unmeasured guarantee.


### Implementation evidence — acceptance failed on grown history

The build-gated session chain preserves v1 and executes two consecutive v2
operations, each with interrupted append and original-attempt restart. Complete
package-vector hashes are asserted in `repeatable/incremental_tests.rs`:

| Operation | Complete vector SHA-256 |
|---|---|
| Unchanged v1 | `b56de6aa7dfa328f00b55d2263c1912e13146a4537ef110d5a985542c9ce9e82` |
| First v2, legacy base | `8b3d1c1fb1aa79868568350f67dabb46e916b2ec8aa104470bc020733999a305` |
| Second v2, tree base | `71070d53e6f2a34408bc1f4f396b63dade03055bc3fec498907e5a8c4539276c` |

These establish deterministic bytes, not acceptable autosave performance.
The existing immutable grown-history capture refuses under unchanged
262,144-byte corridors. An explicitly separate in-memory sizing experiment
produces 547,413 active-locator bytes for the second v2 operation (recovery
540,350; pinned 443,946). It is diagnostic only: no native bounds changed and
no effects were dispatched. First import also exceeds its data corridor.

Same-destination deduplication and same-role predecessor reuse work. Seven new
retained controls generate 26 retained-inventory nodes; changed semantic/global
inventory paths then cause 473 of 577 active locator pages to change. This is
nested index amplification, not a successful incremental-save implementation.
The native adapter remains v1; no new app build or native write is justified.
Evidence: `/private/tmp/ll2a-v2-import-sizing.log`; exact-vector run:
`/private/tmp/ll2a-v2-final-vectors.log`. LL2a performance remains unaccepted.


## Follow-on review — inventory and locator batch overlays (approved 2026-10-10)

Status: the user approved this exact follow-on packet on 2026-10-10. Implementation and bounded disposable validation may proceed within its stated gates; approval does not imply measured performance acceptance.

Measured boundary. V2 is correctly reproducible and refuses before effects, but fails grown-history acceptance. After the diagnostic-only import, seven new retained controls generate 26 retained-inventory pages; active/global inventories change 52/133 pages; active locator changes 473 of 577 pages and writes 547,413 bytes. Those inventory implementation-page hashes become dispersed locator keys. The unchanged 262,144-byte corridor fails; v2 native dispatch remains disabled. The proposal below has not been implemented or performance-qualified.

Dispatch. Add only photara.codec.ps2-repeatable-admission-v3 / photara.codec.ps2-repeatable-layout-v3. Their O/F codec labels change; all other v2 original fields, retained-control inventory base/additions/result meanings, project/accounting limits and existing receipt/phase bytes remain unchanged. V1/v2 recovery dispatch and vectors remain exact. A v3 retained-inventory base may reference the selected completed v2 or v3 result; legacy v1 import retains its existing rule. Operation ID/ordinal, retained-control, ownership and root-placement trees keep v2 path-copy. Semantic/global inventories and per-role locators use the overlay types below.

Exact new records. Every record has exactly schema:{id,version}, project_id, extensions:{} plus listed fields. Version is 1 except the new logical inventory overlay root, which is version 2 under the approved narrow correction below. Ref means the unchanged JSON ObjectRef. PhysicalRef means the unchanged frame coordinate, restricted to the same independently owned metadata allocation for locator records. Decimal means canonical DecimalU64. All unknown fields, nonempty extensions, nil/foreign identity, false summaries and incorrect tags refuse.

Two families: prefix photara.package.inventory-overlay (logical JSON, semantic membership) and photara.storage.locator-overlay (physical metadata frames). Each family has four schema IDs: the prefix itself, prefix-batch, prefix-index-leaf, prefix-index-branch.

Inventory overlay root: {purpose:"state"|"global", base:Ref, base_count:Decimal, batches:Ref, through_sequence:Decimal, count:Decimal}.
Locator overlay root: {role:String, allocation_id:UUID, base:PhysicalRef, base_count:Decimal, batches:PhysicalRef, through_sequence:Decimal, count:Decimal}. Role is the existing exact persisted planner role label; allocation_id equals the role's registered locator destination. Reader compares it with the actual selected role and independent ownership, not caller eligibility.

Both batch records: {sequence:Decimal,before_count:Decimal,after_count:Decimal,entries:[...]}. Inventory entries are exactly {object:Ref,change:"insert"|"remove"}. Locator entry object uses the exact existing JSON/Blob ObjectRef union, preserving existing blob identity fields. Locator entries are exactly {object:<existing locator ObjectRef>,change:{kind:"put",membership:"semantic"|"ownership"|"blob",physical:<existing membership-specific physical union>}} or {object:<existing locator ObjectRef>,change:{kind:"remove"}}. This graph writer emits only existing supported memberships/allocations; no new Blob mutation support. Keys strictly ascend by existing (sha256,numeric byte_length), with no duplicate keys. Batches are nonempty. A remove requires present key; inventory insert requires absent key; locator put requires absent or different exact value. No redundant changes. No live authority is conveyed by a batch.

Both index leaf records: {count:Decimal,entries:[{sequence:Decimal,batch:<family reference>}]}. Both index branch records: {count:Decimal,children:[{first:Decimal,last:Decimal,count:Decimal,child:<family reference>}]} where family reference is Ref for inventory and PhysicalRef for locator. Leaf count equals rows; branch count sums positive child row counts, at least two children, exact ordered disjoint summaries. Sequences are exactly 1..through_sequence without gaps. Root count is resulting set size, not index row count. Index leaf capacity uses semantic_leaf for inventory, physical_leaf for locator; branch uses existing branch. Index nodes use the approved sorted append/path-copy/split/collapse/postorder algorithm; no linear previous-root chain.

Original binding and deterministic construction. At first overlay adoption, base is exactly the independently verified selected predecessor full tree of the same purpose/role/destination; base_count is its audited count. Base must be an existing leaf/branch root, never another overlay. Subsequent originals preserve the exact anchor and existing batch prefix from their selected predecessor overlay, append one sequence, and retain all earlier batches. Derive the exact current target set with existing closure semantics; diff against the previous resolved map in canonical key order. Serialize one batch (no geometry-dependent subdivision), then final reachable changed index nodes, then root; if no diff at first adoption, reuse the existing full-tree root without a sequence-zero overlay; for an existing overlay, reuse its exact root. A batch exceeding existing JSON/frame/work/entry limits refuses before effects. Each batch before_count and after_count is independently checked by exact sequential map application: before_count equals anchor count or preceding after_count, after_count equals the new map size, and the final root count equals the final map size. Planner inputs already persisted under O determine every byte; no new mutable runtime choice or planner input field.

Typed closure and placement. Inventory base, batch and index pages are implementation dependencies, not members of the semantic/global set they describe. Each role independently resolves the same logical inventory edges through its own locator and includes all these implementation objects in its exact locator/physical keep-set. They do NOT become entries in an older global hash-inventory tree. Locator base/batch/index/root pages are direct physical metadata edges; they are never logical locator entries or semantic/global members. Thus new implementation hashes create only one canonical locator-delta batch, not another round of hash-tree path copying. Existing ownership claims cover every byte through the admitted corridor, all anchor/batch/index pages and padding; all frames and standing controls retain normal charges. No old record or selected historical proof is deleted, expired, or granted credit.

Lookup/audit/restart. Audit anchor and the balanced index once, authenticate every reachable batch and sequence, then apply batches in ascending order. Lookup may use an inspection-local resolved map; a cold scoped lookup searches batches newest-first then anchor, with a remove terminally absent. Existing depth/page/entry/JSON/physical-work bounds apply to the complete traversal before allocation; no new product limit. Logical inventory refs must resolve independently in each role; locator physical descendants must remain in that role's registered destination. Full reader compares exact final locator keys/memberships/locations and exact semantic/global set, not merely batch checksums. An immutable-prefix memo may reuse deterministic batch application only after fresh actual bytes and observations match; current closure/authority/lease/barriers and Saved proof stay independent. Restart recompiles the exact original with the new dispatch and checks exact O/files/selectors. No arbitrary anchor substitution, chain reset, compaction, or rebase.

Acceptance before native enable. Pin canonical records and full v1→v3→v3 outputs; reject changed anchors, roles, destinations, missing/reordered batches, tombstone errors, false counts, narrowed limits and changed observations. Compare cache/uncached results. On the same immutable grown capture demonstrate unchanged-corridor first import and repeated updates, actual page/byte/decode counts, bounded memory and useful timing. If it fails, keep dispatch disabled and report; do not widen geometry/corridors. Then permit one controlled native return-to-original timing/close/reopen check. Performance is an acceptance condition, not a claimed result of this proposal.


The sizing experiment also exposes canonical-decode cache thrashing (7,936 then
9,840 parses per sizing compile). Smaller appends alone do not qualify this
proposal for low-resource autosave. Within equivalent authenticated behavior,
load only needed current closure/pages and reuse verified immutable computation;
retain fresh framing, identity, authority and closure checks. Measure total
save latency, decode work and peak memory before native handover.


### Narrow compatibility correction — approved 2026-10-10

Implementation review found a collision in the approved packet: the existing
`photara.package.inventory-overlay` version 1 selected control already means
`{schema,project_id,extensions,base,controls,count}`. The proposed new logical
root used the same schema ID/version for a different exact shape. Context-specific
dispatch happens to distinguish them today, but must not redefine that frozen
schema meaning.

Approved correction: **only the new logical inventory overlay root uses
`photara.package.inventory-overlay` version 2**. Its approved fields remain exactly
`{schema,project_id,extensions,purpose,base,base_count,batches,through_sequence,count}`.
The new inventory batch/index schemas remain version 1. All locator overlay
schemas remain version 1. Admission/layout codec IDs remain v3. Every existing
version-1 selected-control byte and meaning remains unchanged. Writer and reader
must dispatch on both schema ID and version and refuse wrong versions/shapes.
No other format, limits, history, authority, durability or native scope changes.

The user explicitly approved this exact correction on 2026-10-10: “Approve the
version-2 correction.” Implement it and pin final v3 vectors before native
dispatch. All original performance and disposable-validation gates remain.
There were no native v3 writes before this approval.

### Implementation verification — 2026-10-10

The approved corrected v3 format is implemented; existing v1/v2 vectors remain
unchanged. The mandatory session suite now has 15 passing tests and the existing
reader/recovery corpus has 5 passing tests. Controlled and default-store strict
checks pass. Six-operation v1→v3 continuation, original-attempt interrupted retry,
wrong schema versions, counts, roles, bounds and changed-byte/context refusal are
covered. Complete v3 chain vectors are pinned in
`crates/photara-store/src/package/v1_3/repeatable/incremental_tests.rs`.

New disposable originals dispatch v3 only after those gates; stored originals
retain their original codec. Same-capture comparisons verify scoped locator
lookup, one full audit per role per inspection, lazy canonical objects, borrowed
observations, charge-only ceiling metadata and completed-journal prefix reuse.
Reuse authenticates exact context/limits, byte prefixes and observations; new
selected state requires a freshly verified matching receipt. No barrier changed.

**Performance is not accepted.** The final native out/back restores Saved revision
28 at x=13/y=14, closes normally in 0.368 seconds and reopens with the exact Saved
coordinate. Total submit/checkpoint is 4.47/4.60 seconds versus the initial v3
8.47/9.43 seconds, but peak RSS remains 915,210,240 bytes and reopen takes 14.57
seconds. No app build or handover was performed. The existing LL2a evidence file
contains source-gate logs, binary hashes, intermediate failures and final timing.
All disposable writers are stopped. No fixture bounds or production scope changed.
