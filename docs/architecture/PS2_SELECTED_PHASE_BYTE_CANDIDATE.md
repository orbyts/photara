# Same-original selected phase byte candidate

**Unfrozen, disposable selected-byte/process-cut model. No production reader, native file effects, qualified durability, format freeze or release policy.** This continues the exact [selected admission](PS2_SELECTED_ADMISSION_CANDIDATE.md) through the same-tip Graph operation and cleanup. The original admitted record, its request, original codec, reserve and control limits are unchanged. It does not implement newborn enrollment, rollover, retirement, pending resource origins or Blob conversion.

The [new generator](proposals/ps2/selected-phase/generate.py), [canonical corpus](proposals/ps2/selected-phase/linked.json), [test target](../../crates/photara-store/tests/selected_phase_candidate.rs), [phase reader](../../crates/photara-store/tests/selected_phase_candidate/phases.rs) and [Core replay preparation](../../crates/photara-store/tests/selected_phase_candidate/replay_prepare.rs) are new files. The shared [selected-operation seam](PS2_SELECTED_OPERATION_VALIDATION_SEAM.md) traverses the actual published HEAD, packed roots, ledger and ownership; it does not substitute a normalized HEAD for selected authority.

## Exact finite selected stages

Every stage has an actual HEAD/commit with the exact previous commit ID/hash and incremented package revision. The admitted stage is byte-identical to the existing admission specimen. Its immutable `photara.storage.original-admission` v1 record O remains selected under token 1 through every stage. The reader accepts its original `photara.codec.ps2-same-tip-admission-v1` and `photara.codec.ps2-same-tip-layout-v1` bytes; an unsupported codec fences before a proposed next effect. This new reader generation does not reissue or change the original admission.

`photara.storage.operation-phase` v1 is an **unfrozen discriminated proposal**, not asserted compatible with earlier experimental phase layouts. Common fields are `schema,project_id,extensions`; every variant has `token,original,stage,consumed,remaining,newborn_binding,finalization,cleanup`. `newborn_binding` is always null in this route. Every stage after admitted also has the exact field `journal_completion:{frame,barrier:true}`. The inline frame retains the existing accepted-journal-frame v1 schema and is bounded/charged within the actual phase control; no extra unaccounted journal file is assumed.

| Stage | Selected semantics | C / effective remaining | F | Cleanup |
| --- | --- | --- | --- | --- |
| admitted | Original active ordinal 3 | 0 / 1,900,544 | null | null |
| journal-durable | Original active ordinal 3 | 0 / 1,900,544 | null | null |
| payload-durable | Original active ordinal 3 | 0 / 1,900,544 | null | null |
| finalizer-selected | Original active ordinal 3 | 0 / 1,900,544 | Exact F Ref | null |
| finalizer-durable | Original active ordinal 3 | 0 / 1,900,544 | Same F Ref | null |
| published | Actual prepared ordinal 4; recovery is exact old active; retained pin unchanged | 1,835,008 / 65,536 | Same F Ref | null |
| clean | Same published roots and charge | 1,835,008 / 0 | Same F Ref | `{remaining_roles:[],directory_barrier:true,released:"65536"}` |

The selected hold leaf retains one exact `{token,original,phase}` entry. Its `reserved` remains immutable original R, `consumed` is C and `remaining` is the effective held remainder. Clean's explicit release explains why the terminal summaries no longer sum to original R. Clean does not infer success from an absent hold. A terminal retry verifies the current bytes and exact original/request, returns the original receipt and reports zero additional charge and zero additional release. A subsequent new admission or eventual archival of this terminal replay entry is outside this bounded route.

The supplied journal vector models observed journal completion; it is not a second persistent file or a source of unowned bytes. An admitted HEAD may coexist with exactly its original authorized completed fourth frame before selector publication. Selector reconciliation requires that supplied completion and all candidate control bytes. Missing or substituted fourth frames cannot be manufactured by selector cleanup. `barrier:true` and directory cleanup flags are modeled observations in this pure fixture, not `fsync` calls or qualified Saved claims.

## Payload, finalization and accounting

The original deterministic decoder produces exact append bytes for seven existing tips. The payload boundary is derived by that codec: each data tip appends its planned semantic/global records before its first new allocation-claim frame; metadata tips have empty payload. All new ownership, locator, root-placement and padding frames belong to the finalizer suffix. Payload is **208,813 bytes** and finalizer suffixes total **1,626,195 bytes**, including charged padding. Their sum is the exact **1,835,008-byte** planned growth. Every manifest binds original allocation ID/arena/native witness, begin/end, full framed suffix SHA-256 and frame count; the original recipe also commits the complete original prefix hash. Hashes and counts include unreachable intermediate frames and padding.

The proposed `photara.storage.finalization-control` v1 F has exact common fields plus:

- `token,original,prepared_receipt`;
- `generation_plan:null,newborn_binding:null,sealed_allocations:[]`;
- `payload_completion:[Span],recipe_codec,writes:[Span]`;
- `target_projection:{active,recovery,pinned_roots,operation_index,conversion_source,retention_evidence,base_inventory,placement,active_state}`.

`Span={allocation_id,arena,witness,original_end,final_end,frame_count,framed_sha256}`. Its two ordered sets are exact, contiguous splits of O's full append manifests, with no new allocation, budget or generation. Target projection agrees exactly with O's semantic/physical target and the deterministic decoder. F is selected before any finalizer byte may appear. Partial payload is allowed only after the journal phase; partial finalizer bytes are allowed only after F selection. Every supplied partial tail must be an exact prefix of the admitted bytes. Unknown content, excess tails, altered original witnesses and finalizer bytes appearing before F selection refuse.

`prepared_receipt` and `active_state` are **embedded canonical reconstruction bodies, not physical closure edges**. Their exact ObjectRefs must equal O.request.receipt and O.semantic_target.active. They allow deriving the historical control chain and outer active fields when read-only recovery lacks active payloads. Full verification still resolves the actual packed receipt and StateRoot; the embedded bodies cannot satisfy a missing physical object. The original O.request.receipt remains a typed dependency of the full selected closure. These bounded redundancies are proposed for review, not an implicit general wire policy.

Only after actual supplied finalizer bytes match the complete plan does publication select the semantic roots, ownership, current tip observations and charged ledger in one HEAD. Synthetic tip observations derive from final bytes and preserve original witnesses/highwaters; they do not precede their own finalizer writes. Ledger total becomes **3,911,680** from original **2,076,672**. The immutable R is **1,900,544**, with exactly **65,536** cleanup remainder. No existing source/sealed allocation is recharged, no charge decrease is inferred, no reserve expansion occurs, and cleanup adds no second C or OS free-space credit.

F is **8,075 bytes**, below the original 16,384-byte cap. The largest phase body is **956 bytes**, below 4,096. Every adjacent original/candidate coexistence is proved before effects, including actual O/P/F, ledger, hold, envelope, overlay, request/receipt controls, manifests, both selectors/commits, the actual bounded selector intent/staged HEAD, and one directory allowance. Hash-identical immutable control objects share one content-addressed file; mutable selector roles remain separately charged.

| Candidate stage | Simultaneous modeled charge | Roles |
| --- | ---: | ---: |
| admitted | 94,208 | 18 |
| journal-durable | 102,400 | 20 |
| payload-durable | 102,400 | 20 |
| finalizer-selected | 110,592 | 21 |
| finalizer-durable | 110,592 | 21 |
| published | 118,784 | 22 |
| clean | 106,496 | 20 |

Peak **118,784 bytes / 22 roles** fits unchanged original **131,072 / 24** limits, leaving 12,288 bytes and two roles. These are finite synthetic charge assumptions, not qualified filesystem measurements or a universal allowance for newborn operations.

## Fresh replay and independent recovery

Fresh full replay obtains O from actual selected controls, authenticates its exact embedded original HEAD/commit/ledger/envelope/overlay/state identities, and constructs a historical logical view only from still-present actual original prefixes. That view is not current physical accounting. A separate exhaustive proof accounts for every live suffix under original R and the selected phase's exact allowed bytes. Full replay refuses missing original allocations; it never reads an old golden package to replace them.

The new preparation helper reads actual authenticated authored/history closure bytes from those prefixes, builds a bounded in-memory flat PS1 preparation view, verifies that view, and applies the exact O.intent through the existing Core/PS1 planner. Its derived original coordinate and resulting receipt must match O. It has no corpus include, filesystem/media lookup or golden `base_files` fallback. Original receipt provenance and all three prior results survive unchanged. The temporary PS1 wrapper is a computation input, not selected package authority.

Published/clean full verification uses the actual selected controls and physical package with the original packed empty hold retained by O.old_hold. Exact global inventory, typed locator membership, ownership, selected charge classifications, retained roots and actual receipt bytes are checked by the shared reader. The compiled route proof supplies only independently derived control/phase authority; no arbitrary caller control list is accepted as original evidence.

Read-only recovery separately validates the original metadata contract, exact old active/recovery/pinned set, StateRoot headers, O/P/F identity, request/receipt coordinates, sorted original tip manifests, all supplied prefix/suffix bytes and native witnesses, per-control caps, selected control chain and actual own recovery closure. It succeeds with unrelated active and retained allocation pairs absent. It does **not** claim those absent allocations or their semantics were verified, does not establish global physical accounting, and cannot authorize replay/admission/cleanup. The full path refuses the same missing inputs. Strict embedded receipt/schema checks do not convert a prepared metadata commitment into actual active closure or Core proof.

## Process cuts, verification and remaining scope

The corpus supplies exact candidate immutable control bytes, selector intent, candidate commit and optional staged HEAD. The model exercises all six selector transitions before HEAD, with staged HEAD and after HEAD: **18 cuts**. It also exercises **nine matching partial payload/finalizer tails** across data and metadata. Reconciliation validates all prerequisites and known sidecar bytes before returning a modeled selected result. Unknown sidecars, missing candidate bodies and missing journal completion remain unchanged and refuse. There are no filesystem writes or native barriers in these tests.

The final target has **22 tests**: nine new route/recovery/process-cut tests, three new actual-prefix Core replay tests, one inherited preparation test and nine inherited resource/accounting tests. Negative coverage includes altered C/remaining/release, unknown codec, premature/foreign suffixes, extra loose controls, contradictory original token/kind/nonce/scope/capacity/state/generation, malformed receipt, unordered manifests, arbitrary third retained root, individual F cap overflow, unavailable candidate controls and absent/substituted journal completion. Exact named errors are asserted for those targeted boundaries. Clean original retry cannot double-charge or double-release.

[Verification evidence](verification/ps2-selected-phase-candidate.json) binds the exact sources, original admission dependency, canonical corpus and release/strict logs. The tested shared reader is the selected-operation seam at `21b1505` (the `b217b47` seam plus a visibility-only export of its unchanged strict receipt validator). Later Blob edits are separate work and do not retroactively change this run's provenance.

The next newborn slice requires a fresh honest original admission, finite witnessed generation plan, deterministic born-payload/finalizer corridors and worst-case control/marker coexistence before effects. It cannot expand an already admitted R or reuse this same-tip number as a universal bound. This checkpoint makes no newborn, retired-source refund, pending-origin, new qualification, real device durability or permanent compatibility claim.
