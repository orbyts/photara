# PS2 exact-wire review readiness

Status: **engineering checklist, not a format approval**, 2026-09-27.
The [codec/publication contract](PS2_PRODUCTION_CODEC_AND_PUBLICATION_CONTRACT.md#field-level-wire-completion-required-before-codec-code)
requires field-level review before production reader changes. The
[scalable candidate](PS2_SCALABLE_WIRE_REVIEW_CANDIDATE.md) supplies the physical
extension; its examples do not silently supersede the existing operation,
conversion-source or managed-resource requirements.

## Evidence needed for the review

| Area | Verified evidence | Remaining engineering |
| --- | --- | --- |
| Bootstrap and physical closure | [Joined settled snapshot](PS2_JOINED_SETTLED_BYTE_CANDIDATE.md) and [single-HEAD route](PS2_SINGLE_HEAD_ROUTE_BYTE_CANDIDATE.md): exact semantic/physical/control closure and independent recovery | [Integrated settled coordinates](PS2_INTEGRATED_SETTLED_COORDINATE_CANDIDATE.md) now compose scalable replacements with three roots; final transitions remain. [Whole-Blob component](PS2_WHOLE_BLOB_BYTE_CANDIDATE.md) now proves existing managed Blob placement separately |
| Actual authored operations | [Operation specimen](PS2_SCALABLE_OPERATION_BYTE_CANDIDATE.md) proves actual Core/PS1/no-op/original retry; [single-HEAD route](PS2_SINGLE_HEAD_ROUTE_BYTE_CANDIDATE.md) selects operation four and preserves all four receipts through source retirement | Final additive schema/capability dispatch and linked bytes |
| Scalable inventory and charge | [Branch specimen](PS2_SCALABLE_BRANCH_BYTE_CANDIDATE.md) and [single-HEAD route](PS2_SINGLE_HEAD_ROUTE_BYTE_CANDIDATE.md) verify typed trees, supplied contents, original witnesses and exact once-only charges | [Accounting component](PS2_ACCOUNTING_SCALING_CANDIDATE.md) proves typed observation/retained-file roots and a three-node sparse source proof; compose it into the final selected route |
| Original admission and publication | [Single-HEAD route](PS2_SINGLE_HEAD_ROUTE_BYTE_CANDIDATE.md) selects original admission/phase/C; complete preflight checks all phases and simultaneous controls before effects | Original-token newborn/rollover fields and final proposed coordinates; historical phase records remain separate evidence |
| Retirement and compact replay | [Compact fixture](PS2_COMPACT_RETIREMENT_COMMITMENTS.md) supplies useful reclamation evidence; [single-HEAD route](PS2_SINGLE_HEAD_ROUTE_BYTE_CANDIDATE.md) verifies actual post-unlink bytes and terminal original retry | Sparse bounded original proof now passes independently in the accounting component; compose final replay coordinates. The small route has negative retirement economics |
| Retained roots and operational holds | [Origin/evidence mapping](PS2_RETENTION_ORIGIN_EVIDENCE_PROPOSAL.md) distinguishes five portable origins from twelve operational blockers; furnace tests cover blockers | [Integrated settled reader](PS2_INTEGRATED_SETTLED_COORDINATE_CANDIDATE.md) now selects nonempty retained roots with independent locator/ownership and a typed pin tree. All origin classes and nonempty operational holds remain; local reader holds remain separate authority |
| Codec evolution | Earlier fixture documents decode-only compatibility and possible original-ticket hash fencing | [Phase subproof](PS2_CANONICAL_PHASE_SUBPROOF.md) explicitly preserves codec-v1 bytes across two hypothetical reader generations and refuses reserialization/unsupported versions; final proposed permanent coordinate/dispatch set remains |
| Conversion-source retention | [Resource/conversion specimen](PS2_RESOURCE_CONVERSION_BYTE_CANDIDATE.md) validates eighteen original files and nineteen copy cuts; [single-HEAD route](PS2_SINGLE_HEAD_ROUTE_BYTE_CANDIDATE.md) preserves exact original descriptor, files, charges and witnesses | Typed scalable per-file charge/observation roots pass twelve component tests; compose the route. Observations remain synthetic |
| Managed resource records | [Resource/conversion specimen](PS2_RESOURCE_CONVERSION_BYTE_CANDIDATE.md) and [single-HEAD route](PS2_SINGLE_HEAD_ROUTE_BYTE_CANDIDATE.md) validate real selected resource bytes without external-media reads | [Factored resource component](PS2_FACTORED_RESOURCE_BYTE_CANDIDATE.md) passes nine tests with shared immutable requirements, typed selections and exact immutable source associations; complete other approved origin dispatch and whole-HEAD composition |

The nineteen earlier golden values establish canonical encoding of illustrative
fields. Their placeholder references are not a complete valid closure. Similarly,
a furnace using its own fixture types proves the exercised invariant; it does
not prove a different proposed schema by renaming that type.

Each completed row needs exact source hashes, independently generated expected
bytes, focused positive/negative validation, and an honest statement of limits.
The final review package must identify one concrete set of candidate schema IDs,
field layouts, digest domains, frame tags and supported dispatch combinations.
Separate specimens must not imply incompatible layouts are interchangeable.
The [source-coordinate audit](PS2_WIRE_COORDINATE_SOURCE_AUDIT.md) enumerates
actual historical schemas, field layouts, digest domains, frame tags, dispatch
and parser/probe bounds. It identifies the incompatible forms and remaining
scalability questions; it is not the final proposed coordinate mapping.

The final proposed-coordinate transition must begin at the exact new
[integrated settled HEAD](PS2_INTEGRATED_SETTLED_COORDINATE_CANDIDATE.md) and preserve
its original receipt prefix, witnesses and charges. The historical joined-to-route
proof remains separate; this new settled package establishes a fresh synthetic
registration epoch and does not perform an in-place upgrade or rebind. A selected admission must
exist before its first packed effect. Any bounded direct-control overlay must
have an exact, disjoint union with the packed inventory and fit the original
standing-control allowance. Original admission and phase records must not
transitively hash their own current selector. Publication and retirement must
verify actual selected destination bytes, including after source removal,
without consulting an unselected historical corpus as a reconstruction fallback.
Pending retirement charges remain attributable until the original authorized
absence/barrier evidence permits once-only project credit.

The final coordinate proposal also needs a scalable observation registry or an
exact trusted local-registry contract. Allocation witnesses are scoped to a
local storage profile and incarnation; they do not turn native paths, device
identity or watcher observations into portable resource authority. A copied or
imported package cannot acquire writable admission merely by matching content
hashes. No automatic rebinding policy is implied. Namespaced extension keys
need the actual QualifiedName grammar and explicit value bounds, rather than
the weaker predicates used by some experimental specimens.

The [integration plan](PS2_FINAL_COORDINATE_INTEGRATION_PLAN.md) now spells out
RootSet/StateRoot v2 selectors, a per-root physical placement tree and the bounded
loose-control partition. The [newborn field proposal](PS2_NEWBORN_ORIGINAL_FIELD_PROPOSAL.md)
spells out original generation-plan, marker, binding and measured-finalizer
records. The settled coordinate subset now has actual integrated bytes and reader
evidence; newborn and transition fields remain specifications awaiting their
own selected bytes. Neither completes the rows above.
The next concrete slices specify [all-origin fields](PS2_SELECTED_ORIGIN_FIELD_PROPOSAL.md)
and a [selected admission successor](PS2_SELECTED_TRANSITION_IMPLEMENTATION_PLAN.md).
These retain the approved semantics and explicitly defer pending-origin authority
until actual selected original/phase dispatch exists.
The [selected-origin candidate](PS2_SELECTED_ORIGIN_BYTE_CANDIDATE.md) now proves
authored/history/recovery/explicit evidence through whole HEADs, with 16 tests,
19 coherent refusals and independent subset-aware recovery. Pending and actual
origin/pin transition authority remain open; two internally valid static states
do not establish permission to rotate or release them.

The [selected Blob plan](PS2_SELECTED_BLOB_COMPOSITION_PLAN.md) preserves the
actual legacy project and distinguishes a standalone HEAD proof from conversion.
A Blob-containing ConversionSource still needs structural path/type/extent and
witness checks separated from explicit strong audit; the present accounting
fixture hashes all retained files and cannot be reused as routine media open.

## Separate prerequisites and decision boundaries

[Storage qualification](PS2_MACOS_STORAGE_QUALIFICATION.md) remains independent.
The three authorized [clean-remount cases](PS2_MACOS_CLEAN_REMOUNT.md) are
observations; clean detach may flush outstanding writes. They do not authorize
power interruption or establish a production writable/Saved profile. Missing
qualification does not prevent completing the pure candidate review package.

Once the engineering rows are complete, explicit review is still required before
freezing a permanent format or changing the production reader. That review must
cover the exact compatibility/provenance spellings and local binding model; it
must not silently choose a retention release, historical-ID expiry, authorization,
Saved guarantee, numeric product threshold or visible backpressure policy.
Production writes, conversion, migration, retirement and deployment remain later
boundaries. PS3, PS4 and LL2a retain their existing dependency order.


## Latest bounded increments

The [selected admission proof](PS2_SELECTED_ADMISSION_CANDIDATE.md) now binds an
actual O/P/nonempty-hold successor and independently regenerates its entire
prospective append from old bytes and Core output. Sixteen tests pass; R, C's
prospective growth and simultaneous controls are bounded before effects. The
prospective empty-hold package is sizing input, not publication under O; original
prefix-only replay and actual selected phase/finalizer/cleanup remain open.

The [D19 Blob preparation](PS2_SELECTED_BLOB_PREPARATION.md) passes ten tests for
trusted original semantic preservation, generic raw accounting and instrumented
structural versus explicit audit. Its proposed HEAD has not passed the shared
physical reader. Neither these adapters nor the no-ConversionSource corpus close
full Blob composition or converted-source audit behavior.
