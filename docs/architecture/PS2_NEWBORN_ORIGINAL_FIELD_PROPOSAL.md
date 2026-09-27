# Newborn/original finalizer field proposal

**Proposed, unfrozen, documentation only.** This bounded contract exports four controls to the whole-HEAD integration owner. It chooses no production behavior, qualification profile or new numeric cap. Evidence: [birth implementation](../../crates/photara-store/examples/ps2_index_furnace/placement_v3/typed_inventory/fresh_generation.rs), [rollover implementation](../../crates/photara-store/examples/ps2_index_furnace/placement_v3/typed_inventory/rollover.rs), [rollover report](PS2_GRAPH_WITNESSED_ROLLOVER.md), and [integration checklist](PS2_NEWBORN_REPLAY_INTEGRATION_CHECKLIST.md). Existing fixture bytes are not these proposed bytes.

## Exact exported shapes

Every record has exactly `schema:{id,version:1}`, `project_id:Id`, `extensions:ExtensionMap` and the fields below. `Id` is a nonnil canonical UUID; `D` is canonical decimal u64 text; `H` is lowercase SHA-256 text; `N` uses existing QualifiedName; `Path` uses existing RelativeComponents. Unknown required fields/versions refuse. Extensions are namespaced opaque nonedges included in byte bounds. Arithmetic is checked.

`Ref={kind:"json",sha256:H,byte_length:D}` authenticates exact canonical bytes. **Edge** means selected typed dependency; **commitment** means checked identity without recursive historical/prospective retention. `PhysicalRef={allocation_id:Id,arena:A,offset:D,byte_length:D,record_sha256:H}`, where `A="data"|"metadata"`. `Witness={device:D,inode:D}` has nonzero inode and is local only.

| Schema ID, version 1 | Fields beyond common envelope |
|---|---|
| `photara.storage.generation-plan` | `token:D, nonce:H, scope:Scope, request_sha256:H, old_head_sha256:H, reserve:D, cleanup_bound:D, project_limit:D, payload_codec:N, payload_recipe_sha256:H, semantic_target_sha256:H, slots:[Slot], finalizer_codec:N, corridor:[Corridor], shape_sha256:H, control_bounds:Bounds` |
| `photara.storage.generation-marker` | `token:D, generation_plan:Ref, nonce:H, ordinal:D, allocation_id:Id, arena:A` |
| `photara.storage.newborn-binding` | `token:D, generation_plan:Ref, slots:[SlotBinding]` |
| `photara.storage.finalization-control` | `token:D, generation_plan:Ref, newborn_binding:Ref, payload_completion:[Range], sealed_allocations:[SealedAttribution], recipe_codec:N, writes:[FinalizerWrite], target_projection:Target` |

Exact nested types:

```text
Scope = {profile:N, incarnation:Id, directory:Witness}
Slot = {ordinal:D, allocation_id:Id, arena:A, stage_path:Path,
        final_path:Path, maximum_extent:D, marker_allocation_bound:D}
Corridor = {allocation_id:Id, arena:A, start:D, maximum_end:D}
Bounds = {marker_bytes:D, newborn_binding_bytes:D, finalization_bytes:D,
          phase_bytes:D, aggregate_control_bytes:D, aggregate_control_count:D}
SlotBinding = {ordinal:D, state:"intended"|"bound"|"empty"|"promoted",
               witness:null|BirthWitness}
BirthWitness = {device:D, inode:D, marker_byte_length:D,
                marker_sha256:H, marker_observed_charge:D}
Range = {allocation_id:Id, arena:A, witness:Witness, original_end:D,
         original_sha256:H, final_end:D, framed_bytes:D,
         framed_sha256:H, frame_count:D}
SealedAttribution = {allocation_id:Id,
                     observation:LocalObservationV1, charge:SealedChargeV1}
FinalizerWrite = {allocation_id:Id, arena:A, witness:Witness,
                  original_end:D, original_sha256:H, final_end:D,
                  frame_count:D, framed_sha256:H, recipe_sha256:H}
Target = {active:Ref, recovery:Ref, pinned_roots:Ref, operation_index:Ref,
          conversion_source:null|Ref, retention_evidence:Ref,
          base_inventory:Ref,
          placement:{generation:D, root_placements:PhysicalRef}}
```

`LocalObservationV1` / `SealedChargeV1` are full embedded canonical bodies of the proposed `photara.storage.local-observation` / `photara.storage.sealed-charge` v1 [accounting candidate](PS2_ACCOUNTING_SCALING_CANDIDATE.md), with pack subject. The charge's observation Ref must equal the embedded observation's canonical Ref. This avoids selecting finalization-control that depends on charge records not yet written into finalizer output. Published trees must contain the exact same bodies/references.

## Binding and transition rules

**Original plan.** Token is nonzero and equals admission. Nonce is the original random 32-byte value encoded as H. Request and old-HEAD hashes commit exact original canonical bytes. R, cleanup and project limit equal admission and never expand. `payload_recipe_sha256` hashes the canonical original payload-recipe value excluding this plan/mutable controls; `semantic_target_sha256` hashes admission's canonical semantic target, excluding placement/accounting/global overlay. It also excludes admission-dependent retention-evidence records that refer to the not-yet-created original admission. The pre-admission target can bind fixed pending OperationId/requirement intent; later pending evidence links the actual original. This semantic projection is distinct from the later finalizer Target, which includes selected retention_evidence. Plan has **no back-reference to original admission**, which references the plan.

Slots are finite, unique allocation IDs and stage/final paths, with contiguous ordinals from one. Paths resolve inside the witnessed directory; stage differs from final. Corridor is a finite admitted set sorted by arena then allocation ID, each naming an original registered tip or admitted slot; start/end agree with the payload recipe. The codec determines the exact required set. Existing furnace evidence uses two final tips; this is not a universal package restriction or proof for a four-tip/retained-root layout. Finalizer cannot create another generation. `shape_sha256` hashes canonical `{old_head_sha256,payload_recipe_sha256,semantic_target_sha256,slots,corridor,finalizer_codec}`. This is an input-shape commitment: the immutable decoder must deterministically derive the complete changed-key/tree shape from authenticated old-base and payload bytes before sizing. The digest alone proves no size bound; sizing sentinels never become observed charge. Positive Bounds are admitted under supported codec/profile limits. Preflight covers full R, worst-width future records and simultaneous rounded controls/namespace overhead; aggregate bytes is a coexistence cap, not per-file multiplication. No new cap numbers are proposed here.

**Marker and binding.** Marker generation_plan is a **commitment**, not a future requirement to retain the truncated marker file. Expected marker bytes are deterministically reconstructed with empty extensions. Binding generation_plan is an **edge**. Binding has exactly one slot entry per plan ordinal. Intended alone has null witness; later states preserve the exact original BirthWitness. Each selected update advances one slot one step in admitted order, preserving all other original fields.

Intended→bound requires a complete exact marker and file/directory barriers before witness selection. Empty/partial unbound markers fence without adoption/deletion. Bound→empty truncates only that witnessed inode and barriers. Empty→promoted uses no-replace final linking, barrier, removal of only the known stage link, and final barrier/reopen. Only the exact stage/final pair may temporarily have two links. Same-byte inode replacement, symlink, extra link or unknown occupant refuses. All slots are promoted before Graph acceptance/payload effects; ordinary completion cannot bypass pending birth.

**Finalization.** Plan and newborn_binding are **edges**; the latter is all-promoted. After payload barriers, select finalization-control with actual sealed attributions **before finalizer writes**. Range list is the exact payload-touched set, sorted unique by allocation ID. Prefix/end, witness and full raw suffix hash/count include unreachable frames and reject unexplained tails. Sealed list is the exact newly sealed set, sorted unique, excluding finalizer tips. Preserve authoritative prior high water; no guessed charge.

Writes match the exact corridor allocation set/order and are confined to its bounds and codec. Each commits a post-payload prefix, exact final end/raw suffix and canonical per-arena recipe hash. The supported codec must regenerate those exact bytes from original admission, payload evidence and embedded attributions. **Target references are prospective commitments while pending**, not prepublication fetch edges. Target follows the [integration plan](PS2_FINAL_COORDINATE_INTEGRATION_PLAN.md): root_placements directly authenticates independent active/recovery/retained-root placements. It excludes current accounting/global overlay and its own control hash, preventing a self-hash cycle. The final root-placement-tree field codec remains an integration dependency.

Final-tip high water is measured only after finalizer barriers and lives in the selected loose ledger summary. Joint HEAD publication selects independent Graph/ownership/locator closures, sealed charge/observation trees and increment C; immutable R becomes remaining R−C with cleanup retained. Cleanup releases remaining reserve without charging C twice. Local witness equality never grants qualification or copy/rebind authority.

## Whole-HEAD seam and remaining evidence

Proposed exports: admission `generation_plan:null|Ref` (**edge**); phase `newborn_binding:null|Ref` and `finalization:null|Ref` (**edges**). Null is allowed only by explicit phase/codec dispatch, never generic permission to create files. The integration owner must finalize the exact admission/phase IDs and null/variant table, semantic-target/payload-recipe projection fields, supported decoder names, and control-closure inventory inclusion. RootSet/StateRoot v2 mapping remains theirs.

Still required: independent canonical vectors/readers for these four actual IDs; a whole-HEAD born-and-sealed rollover with all phase cuts and finite finalizer; coherent equal-total charge/witness swaps; exact sealed-before-finalizer versus final-tip-after-barrier accounting; original-byte replay under a later reader; and unsupported original-codec refusal before effects. Distinct codec-v2 transition requires distinct tested forms—reader-generation replay of v1 alone is insufficient. Runtime qualification, retirement pins/authorization/absence/once-only credit remain separate obligations. This document is a concrete field proposal, not evidence that the integration already passes.
