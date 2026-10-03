# PS2 repeatable writer amendment

Status: **proposed, not approved**, 2026-10-03. Implementation exposed a concrete
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

## Proposed bounded amendment

Authorize a separately versioned repeatable planning recipe, preserving all
existing frozen codecs for exact decode/replay. Proposed dispatch names:

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

## Decision requested

Approve this bounded additive repeatable-codec amendment and its explicit
persisted-input approach, allowing implementation and exact-byte validation.
The original freeze stays intact; changing existing codec meanings is not an
alternative. Storage qualification follows implementation, then PS3 → PS4 → LL2a.
This does not authorize real-library writes, migration, deletion, deployment,
automatic conversion, expiry, new product limits or weaker Accepted/Saved claims.
