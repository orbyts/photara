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
