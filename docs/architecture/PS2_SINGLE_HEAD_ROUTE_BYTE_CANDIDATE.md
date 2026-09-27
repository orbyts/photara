# PS2 single-HEAD route byte candidate

Status: **unfrozen disposable review specimen; no production reader, permanent wire freeze, filesystem qualification, or migration authority**. This extends the [settled joined specimen](PS2_JOINED_SETTLED_BYTE_CANDIDATE.md) with selected original-token phase accounting. The [phase subproof](PS2_CANONICAL_PHASE_SUBPROOF.md) remains a separate historical component, not the authority for this route.

The new [generator](proposals/ps2/route/generate.py) produces [linked-route.json](proposals/ps2/route/linked-route.json) and [coherent negative variants](proposals/ps2/route/route-negatives.json). The [independent operation generator](proposals/ps2/route/operation.py) produces [operation-four.json](proposals/ps2/route/operation-four.json). These are fixed canonical bytes, independently constructed from the Rust implementation. The actual reader is the private integration test [single_head_route_candidate.rs](../../crates/photara-store/tests/single_head_route_candidate.rs) and its [selected reader](../../crates/photara-store/tests/single_head_route_candidate/route.rs), [finite contract](../../crates/photara-store/tests/single_head_route_candidate/contract.rs), and [control pool model](../../crates/photara-store/tests/single_head_route_candidate/control.rs). Existing production readers are unchanged.

## One selection, actual bytes

The first world is byte-for-byte the original settled specimen: same manifest, HEAD, commit, allocation files, retained conversion snapshot, and original local witnesses. Its selected active state has three accepted operations and recovery has two. The route applies an actual fourth supported Core `SetNodePosition(13,14)` command through the existing PS1 planner, checks its exact canonical authored and Graph output, and uses acceptance ordinal `4` with distinct journal sequence `19`. Existing operation IDs retain their original request, receipt, and before/after provenance after this later mutation and after source retirement.

Every later HEAD selects one outer commit, one scalable RootSet, and one direct accounting envelope. The envelope selects the core ledger, immutable original admission `O`, and current finite phase `P`, including `R`, consumed `C`, and remaining liability. No external `phase_order` provides selection authority. The snapshot order in the corpus is a test schedule only; current-state opening follows actual selected bytes.

The original frozen HEAD/commit codecs remain version 1. Later commits explicitly require `example.ps2.selected-route.draft-v1` and `example.ps2.inventory-overlay.draft-v1` in addition to the existing specimen capabilities. Missing required dispatch refuses before placement interpretation. These names are experimental and do not occupy the permanent coordinates discussed in the [source audit](PS2_WIRE_COORDINATE_SOURCE_AUDIT.md).

## Acyclic selected control layout

The packed base inventory contains the exact current semantic closure, StateRoot inventory implementation records, selected sealed charge tree and charge objects, and ConversionSource. Ownership and locator implementation records retain their separate exact physical traversal. A new direct `example.ps2.inventory-overlay` contains:

- `base`: the packed inventory tree ObjectRef;
- `controls`: strictly sorted unique ObjectRefs for required current controls absent from that base;
- `count`: exact base-member count plus new control-member count.

The overlay excludes itself. The RootSet selects it directly. The new `example.ps2.route-accounting` envelope contains `ledger`, `original`, `phase`, `project_limit`, `reserve`, `consumed`, and `remaining`. Its direct ObjectRefs are real closure edges. The original frozen base already contains its core ledger ObjectRef, so the initial overlay subtracts that existing member rather than duplicate it. Canonical bytes for every actual loose control are verified, including duplicate journal receipt bytes already represented in packed storage.

`example.ps2.route-original` binds the exact old HEAD, commit, core ledger, optional old envelope/overlay, and full old active/recovery StateRoot bodies. Their hashes match the actual old selector during admission. These embedded old values are **bounded reconstruction evidence and commitments, not traversed historical closure edges**. In particular, old envelope ObjectRefs do not recursively retain all previous original/phase records. The new admission transition requires the actual current previous route to be clean and its selected controls to equal the embedded old values.

The immutable target commits active/recovery StateRoot refs, operation-index and conversion refs, packed base inventory, placement generation, and both direct locator/ownership selectors. It excludes the mutable accounting envelope and inventory overlay. The fixed recipe binds each touched allocation's original local witness, old extent and SHA, final extent and full SHA, exact append SHA, and complete frame count. This removes any hash cycle between target bytes and current accounting controls. The source charge proof retained for retirement is anchored to the exact old charge-tree root.

The full existing 16-byte frame grammar applies to all actual supplied allocation bytes and appended suffixes, including bounded all-zero padding. Padding is real charged, owned bytes; it is not free capacity. An initial 64 KiB-per-tip corridor failed construction. This fixed specimen explicitly uses a 128 KiB corridor in each of four existing tips. No admitted recipe grows its bound later.

## Finite phases and accounting

The route uses the original fixed project bound `2,621,440`, original admission `R=1,048,576`, and cleanup reserve `65,536`. These are local fixture choices, not new product defaults or wire-wide storage policy. Before admission, the declared final extents yield a checked worst positive rounded tip delta plus cleanup that fits R; original charged amount plus R fits the fixed bound. All later observations must name exactly the original four tips with unchanged identity and nondecreasing registered high-water.

| Selected phase | Token | Charged bytes | Consumed C | Held remainder | Source allocation 1 |
| --- | ---: | ---: | ---: | ---: | --- |
| Original settled | — | 761,856 | — | — | owned and charged |
| Graph admitted | 101 | 761,856 | 0 | 1,048,576 | owned and charged |
| Graph published | 101 | 1,286,144 | 524,288 | 524,288 | owned and charged |
| Graph clean | 101 | 1,286,144 | 524,288 | 0 | owned and charged |
| Retirement admitted | 202 | 1,286,144 | 0 | 1,048,576 | owned and charged |
| Retirement released | 202 | 1,810,432 | 524,288 | 524,288 | charged ticket; live source present |
| Retirement unlinked | 202 | 1,810,432 | 524,288 | 524,288 | absent; original ticket still charged |
| Retirement clean | 202 | 1,806,336 | 524,288 | 0 | absent; exactly 4,096 credit applied |

Graph publication atomically selects operation 4, unchanged recovery prefix 2, complete ownership/placement, observed ledger growth, and the original reduced hold. Retirement rewrites both role placements so every retained live object formerly in sealed allocation 1 is available from actual destination frames. Active/recovery StateRoot refs, operation-index ref, and ConversionSource remain exactly equal to the pre-retirement selection. Surviving sealed charge objects equal the original immutable objects. Removing a source from ownership transfers its existing charge to an exact original-token ticket; it does not grant credit. The clean phase grants the original 4,096-byte source credit once, after the modeled unlink/barrier transition.

This small fixed-corridor retirement increases the charged amount by **520,192 bytes net**. It demonstrates exact accounting and replay semantics, not profitable reclamation or sustained throughput. The retained conversion snapshot remains completely charged and untouched. No OS free-space credit or confirmed backing status is inferred.

## Bounded control preflight

The current-state raw control count is not treated as proof of simultaneous capacity. The separate preflight receives every fully planned phase world before admission. It accounts for coexistence of current and candidate content-addressed loose controls, both commits, both mutable HEAD versions, and the shared original manifest. Each distinct immutable file is rounded to the fixture's 4,096-byte accounting unit; sharing requires the same role and exact content hash. Three additional 4,096-byte roles cover the original-token selector intent, staged HEAD, and modeled directory bookkeeping. Each of the seven fixed canonical `example.ps2.route-selector-intent` records is 447 bytes and binds the original ObjectRef/token plus exact current and next HEAD hashes; the admission helper checks its actual bytes against that slot. The original selected standing pool remains 131,072 bytes and the admitted role limit is 24. The maximum is exactly **131,072 bytes across 19 roles**, at retirement admission; Graph publication uses 114,688 bytes across 19 roles. There is no unused byte allowance at the peak. Unknown large controls, wrong hashes, oversized staged HEADs, excess roles, or an insufficient pool refuse before admission. Full and independent recovery opening also check the actual current control pool; the preflight checks complete stage coverage, one immutable original, exact parent/phase succession, and every planned candidate before admitting any payload effect.

This is an explicit pure-memory control allocation model. It supplies neither filesystem sync qualification nor an implementation of actual control-file writes. The earlier furnace evidence remains the separate support for tested filesystem phase/barrier behavior.

## Independent current-state and retry checks

Full opening validates both role closures, exact base/control union, every required supplied allocation and local witness, exact once-only charges, actual original prefix and full suffix bytes, and current source presence or authorized absence. After unlink the verifier receives only the current world; it has no historical allocation fallback. The retained original charge proof and old control commitments are bounded metadata. Corrupting even an unreachable appended byte, substituting a same-byte destination/source inode, or changing a surviving charge coherently refuses.

Independent recovery opening uses the selected recovery locator and ownership roots directly and succeeds with active-only allocations removed. It still checks its selected control closure, original phase/ledger metadata, supplied recovery recipe prefixes/suffixes, and complete typed recovery semantics. This is **read-only recovery readability**, not a claim that missing active allocations permit writable admission, full project accounting, unlink authorization, or settlement.

Terminal cleanup retry requires the exact original ObjectRef and explicit selected clean phase. It returns the original consumed amount and credit with zero additional credit. Original operation retries resolve the actual selected operation-ID tree and return the original typed receipts; a changed request under the same ID refuses. Receipt before/after digests and predecessor commitments remain nonedges.

## Deliberate limits before a permanent proposal

This route is same-tip publication and live retirement. It does not add fresh-generation enrollment, rollover, profile qualification, codec transitions, a production journal/control writer, or a permanent replay codec. Original-token birth/finalization behavior has separate furnace and phase subproof evidence and still needs explicit coordinates in a permanent proposal.

The original source charge proof here contains the entire three-entry charge tree. It is not a sparse scalable Merkle inclusion proof. Core sealed observation rows, retained-conversion per-file rows, and old ledger reconstruction evidence are bounded specimen data; they must not become lifetime-sized loose control arrays. A final proposed wire must choose typed scalable local-observation and retained-file accounting roots and bounded original proof paths. Physical device/inode evidence remains profile/incarnation-scoped local admission evidence, never portable resource truth, and copied/imported bytes cannot automatically rebind it.

This bounded Graph fixture also rewrites its small ResourceState v1 obligation set because authored origin names the new StateRoot UUID. A typed ResourceState tree alone would not remove that origin churn; scalable root-local source association and shared immutable requirement records need separate additive byte evidence. No stable-root-ID shortcut or weaker retention semantics are claimed.

A final permanent-coordinate review package must explicitly reconcile the earlier RootSet/StateRoot/operation-index v1 appendix with new versions, capability dispatch, exact extension grammar and parser bounds, original-token newborn fields, codec replay boundaries, and actual canonical vectors. This checkpoint is not that freeze or a declaration that the full wire review package is complete.

## Verification checkpoint

The complete integration target passed **19 tests** (74.03 s), including actual Core/PS1 operation 4, exact selected frames, original admission and phase succession, source-independent retirement, indexed original receipts, independent recovery, coherent changed-state/charge variants, control bounds and compatibility refusal. A final focused test passed (2.37 s) after adding all seven exact intent-byte and control-counter comparisons; strict Clippy passed (2.55 s). The [evidence manifest](verification/ps2-single-head-route-candidate.json) records the exact source/corpus hashes, reproduction commands, separate logs, and scope. Parent review runs are recorded independently when available.

The real planner path is [commands.rs](../../crates/photara-store/src/package/planning/commands.rs) calling existing Core application, and [planning/mod.rs](../../crates/photara-store/src/package/planning/mod.rs) returning the actual PS1 outcome. The earlier authored no-op remains the existing planner normalization of the second operation; raw Core revision advancement is asserted separately in the historical operation tests. This route does not modify either production path.
