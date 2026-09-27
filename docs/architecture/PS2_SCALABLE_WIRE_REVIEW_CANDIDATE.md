# PS2 scalable closure and publication candidate for review

Status: **draft engineering proposal, 2026-09-27; not a permanent encoding,
format freeze, production implementation or storage qualification**. Field names
below describe a concrete candidate model. They do not reserve schema IDs,
feature IDs, versions, byte tags, pack sizes or production limits. No existing
package is reinterpreted by this document.

This is the scalable physical delta to the
[approved publication boundary](PS2_PRODUCTION_CODEC_AND_PUBLICATION_CONTRACT.md)
and [earlier candidate wire appendix](PS2_PRODUCTION_WIRE_APPENDIX.md), not a
replacement for their operation, resource, conversion or retention contracts.
The [consolidated engineering evidence](PS2_CONSOLIDATED_ENGINEERING_EVIDENCE.md)
owns the final regression snapshot and readiness conclusions. The
[witnessed rollover](PS2_GRAPH_WITNESSED_ROLLOVER.md) and
[live relocation](PS2_LIVE_OWNED_RELOCATION.md) records establish bounded fixture
behavior; the proposed portable types below are not implemented by those tests.

## Proposed delta to the earlier appendix

Keep HEAD and the outer commit envelope, project/bootstrap identity, independent
active and recovery StateRoots, conversion-source retention, original receipts,
accepted-prefix semantics and managed-resource obligations. A predecessor or
receipt before/after digest remains provenance, not a byte-retention edge.
Ordinary operations still do not strongly hash large external media.

Replace flat lifetime inventory and operation arrays with immutable typed index
roots whose logical contents have exactly the earlier equality, ordering and
uniqueness rules. Add explicit placement and physical ownership roots. Neither
replacing an array with a tree nor moving a record changes its logical identity,
original operation result, retained history or pin eligibility.

The concrete proposal is a new required scalable-storage capability and new
typed index records, selected explicitly by RootSet. Do **not** make an existing
inventory-v1 or operation-index-v1 field accept an undocumented union. The exact
capability ID, schema versions, compatibility floor and changes to the earlier
RootSet/StateRoot candidate need explicit wire review. In particular, the
earlier proposed reader floor alone does not prove that every reader at that
floor understands packed placement. Old readers must refuse before treating a
shortened ancestry or incomplete flat inventory as complete.

For review, RootSet adds one inline `placement` selector and replaces its flat
inventory target with a typed `InventoryRoot`. Each StateRoot similarly selects
a typed inventory and operation index. Existing outer authored/history/inventory
equalities are retained; the inventory target's new schema must be understood
under the required capability. RootSet still selects every portable closure;
there is no independently authoritative pack manifest or ledger HEAD.

## Candidate records and exact edges

`ObjectRef` retains its existing exact byte length and digest meaning. Below,
`PhysicalRef` is a proposed direct reference with
`{allocation_id, arena, offset, byte_length, record_sha256}`. Integer overflow,
out-of-bounds slices, overlapping inconsistent frames and arena/type mismatches
refuse. `allocation_id` is a portable immutable generation identity, never a
native path or `(dev, ino)`. Its exact encoding and safe namespace mapping remain
wire/implementation review items. The digest authenticates the complete
referenced record bytes, not the entire enclosing allocation.

The selected commit and the direct roots needed to locate its dependencies must
be independently discoverable using the unchanged HEAD/commit bootstrap path.
The candidate uses direct `PhysicalRef` roots inside `placement`; it does not
look up the locator's own root through that locator. Newly reachable objects
must have a locator entry before selection. No proof requires an unselected
intermediate locator or a retired source after successful publication.

The disposable locator's numeric keys, fixture membership tags and surrogate
source HEAD are not portable identifiers or a second package authority. The
candidate must map real typed ObjectRefs, operation IDs and allocation IDs to
separate checked key spaces. Production reconstruction inputs must be specified
as original package/journal evidence; the retained builder and its conservative
16 MiB source-work allowance are measured fixture inputs, not proposed mandatory
production archives or reserve defaults. No existing conversion-source retention
obligation is removed by replacing that disposable builder.

| Candidate type | Required payload beyond common identity/schema fields | Edges and validation |
| --- | --- | --- |
| `PlacementSelector` (inline RootSet member) | Active and recovery locator roots; active and recovery physical-ownership roots; selected physical-accounting state reference; placement generation | Locator/ownership roots are direct `PhysicalRef`s. Accounting state must be directly reachable without resolving itself through an index it defines. The containing commit binds all of them atomically. |
| `LocatorBranch` | Ordered separator keys, child `PhysicalRef`s, checked key-range summary | Children are physical edges; exact ordering/ranges and branch shape must be validated. A summary never replaces verification of the accessed child. |
| `LocatorLeaf` | Strictly ordered unique logical keys, typed membership (`semantic` or `ownership`), exact `ObjectRef`, destination `PhysicalRef` | Every selected logical record resolves to exactly its named bytes/type. Ownership descriptors cannot masquerade as authored objects. |
| `InventoryRoot` / `InventoryNode` | Typed child roots or sorted membership entries; checked count/range summaries | Enumerates exactly the schema-defined logical closure. The inventory object and StateRoot exclusions from the earlier appendix still prevent self-cycles. RootSet inventory is the exact union of retained roots and separately selected dependencies. |
| `OperationIndexRoot` | Package accepted coordinate; operation-ID lookup root; acceptance-ordinal lookup root | Both indexes resolve the same unique receipt set. Ordinals are contiguous and accepted-prefix commitments recompute exactly; no receipt disappears with undo or ancestry eviction. |
| `OwnershipBranch` | Ordered allocation keys and typed child references | A scalable tree, not a lifetime allocation array in HEAD. Includes allocations needed for semantic records, ownership records and locator pages. |
| `OwnershipClaim` | Allocation identity/arena; owned extent; authenticated-prefix length/digest; sealed/growable status; applicable sealed charge reference | Prefix length is at most owned extent. Every selected suffix object/page is independently authenticated. Sealed extent is exact; growable extent changes only under original admitted work. A claim is a physical retention edge, not authority to debit or refund space. |
| `SealedCharge` | Allocation identity; accounting-domain incarnation; measured extent; recorded charged high-water; observation provenance | Unique attributable charge for that generation/domain. Shared active/recovery/pinned claims refer to the same charge; they do not multiply it. This record must not require hashing its own enclosing final bytes. |
| `PhysicalAccountingState` | Finite standing-control allowance; bounded current-tip registrations; bounded unresolved operation/retirement references; accounted totals/last credit binding | Fixed-size summaries plus scalable immutable roots as necessary, never lifetime per-file entries. Actual count/size bounds require evidence and wire review. Persisted local observations are not portable proof of another host's free space. |

The table fixes semantic roles, not tree fanout or serialized node layout.
B-tree remains the provisional placement lead with radix as comparator.
Selecting either physical tree must preserve these same typed closure rules.
Pin records retain their approved reasons and exact root/closure references;
the ten extra fixture classes are coverage, not new public pin spellings.

Two distinct equalities must be tested. Logical inventory equals the typed
semantic/ownership closure selected by each root. Physical ownership covers all
allocations required to resolve that closure, including the inventory and
locator implementation itself. Sharing is counted once by allocation identity.
An authenticated prefix is not falsely presented as a full-allocation digest;
an extent is not falsely presented as proof of every byte's integrity.

## Breaking charge and self-reference cycles

The candidate preserves the proven two-level accounting arrangement: sealed
charges live in immutable typed ownership records, while a bounded selected
summary tracks current growable tips. A source allocation's final sealed charge
is written outside that now-sealed allocation. Path-copy records and ownership
descriptors themselves occupy already admitted finalization tips. Those tips'
observed high-water charges are selected in the bounded accounting summary,
which does not claim a digest of its own enclosing bytes.

Control files use a separately counted finite standing allowance covering the
maximum simultaneously present admitted controls and directory overhead. They
do not accumulate another standing charge on every HEAD transition. Maximum
encoded witness/finalization/cleanup states must fit before admission. The
fixture's numerical allowance and inode rules are not proposed production
defaults or proof that the same allowance suffices on every profile.

Device/inode, opened descriptor identity, mount/volume binding, allocation
blocks and barrier observations belong to qualified local admission evidence.
They may be persisted for original-attempt reconciliation, but must be tagged
with their accounting/lease incarnation and cannot become a reusable portable
authority. A byte-identical replacement does not inherit an existing local
allocation charge or unlink authorization. A copied/remounted package requires
the reviewed binding/reconciliation path before writable admission; this draft
does not specify or approve an automatic rebinding policy.

## Immutable original admission and phase proofs

The proposed `OriginalAdmission` commits these exact semantic components:

- Original operation/group identity and request digest; exact old selected
  commit/roots, accepted prefix and control base; target authored/receipt roots.
- Qualified local binding reference, original tip prefixes and generation
  identities; bounded fresh-generation birth slots with fixed nonce/intent.
- Payload plan commitment and finalization shape/corridor commitment, including
  admitted destination generations and maximum ends in each arena.
- Per-domain immutable reserve `R`, separately identified cleanup bound, control
  envelope bounds and every retained source/journal dependency.

Transient credentials do not enter the portable semantic request digest.
Original admission's local proof is distinct from that request digest. It must
be reconstructed from immutable admitted inputs, never authenticated by cloning
an untrusted recovered hold and comparing it with itself.

| Selected phase | Preconditions and permitted progress | Required retained proof |
| --- | --- | --- |
| Admitted / births | Full `R` and maximum later control states fit before creation or journal acceptance. Exact planned marker may bind only under a qualified exclusive namespace; partial unbound occupancy fences. | Original base, nonce/slots and liability. No replacement token or budget expansion. |
| Witnessed payload | All fresh destinations bound and promoted without replacement; exact destination identity and admitted prefix/suffix rules hold. Journal acceptance uses the original canonical group and qualified barrier. | Original receipts/group, source evidence, all witnesses and original selected semantic roots. Payload writes are not yet selected semantic state. |
| Finalization bound | Payload barriers complete; actual sealed extents/charges measured. Deterministic finalizer fits the previously admitted corridor. | Exact measured sealed charges, finalizer commitment and immutable witnesses. Maximum-width sizing values never become measured charges. |
| Graph/accounting published | Verify original source/journal bytes, recipes, closures, tip identities and actual high-water growth. One HEAD selects final semantic/ownership/locator/accounting state. | Original token with immutable `R`; selected consumed charge `C`; outstanding reservation `R−C` still covers cleanup. |
| Cleanup complete | Exact selected candidate and original cleanup evidence verified; remove only known redundant controls with required barriers. | Selected final state and original receipt/dedupe evidence. Never charge `C` again. |

The current fixture enrolls births before any journal/source effects. This is
an evidenced bounded ordering; it does not authorize weakening the production
qualified lease/barrier requirement. Phase-only selection must preserve the
previous semantic roots and physical coordinates until joint publication.

Before *any* recovery mutation, validate the full allowed old/candidate selector,
original reserve, pins/accounting state, canonical bounded controls, known
journal artifacts, source identity and every already-present source suffix.
An exact known HEAD staging file may be cleaned only while its intent/candidate
evidence remains sufficient to resume. Unknown occupants or byte/inode changes
retain evidence and refuse. Generic pin/reservation changes must not invalidate
an admitted original control base; final shared-authority concurrency policy is
separate from the fixture's conservative refusal.

## Retirement and compact manifest candidate

Retirement eligibility is the absence of every active, recovery and extra pin
edge to the exact source generation. The original ticket first retains the
source charge after both current ownership edges are replaced. It then selects
unlink authorization, validates the exact local source, unlinks, performs the
directory barrier and proves fresh absence. Only then does one selection apply
the once-only project-domain credit and settle replacement growth. Neither
namespace absence alone nor a previously issued authorization creates credit.
Recovery must recheck absence and selected closure before discarding dispatch
evidence, including after a candidate HEAD was already selected.

**Next disposable compact-manifest experiment: proposed, not implemented or
tested by this document.** Keep the original ticket's source claim/charge, full
control base, original data/metadata prefixes, tip claims, candidate active and
recovery locator/inventory roots, and token. Replace the two embedded full write
plans with two committed suffix manifests:

```text
SuffixManifest = {
  arena, allocation_id, original_end, final_end,
  frame_count, total_framed_bytes, framed_suffix_sha256
}
```

The hash covers the exact complete appended framed range, including dead
intermediate records. An additional canonical typed-plan commitment binds the
reconstruction inputs/outputs if needed. Exactly one supported representation
is legal: both legacy full plans, or both manifests. Mixed or absent modes
refuse. The fixture's 48 KiB ticket cap and same-tip constraint stay unchanged.

Before hold selection, deterministic bounded planning must establish candidate
roots, both manifests and the complete reserve. Before prepublication replay,
regenerate from the immutable original source/base and compare every commitment;
only matching ephemeral bytes may be written. After selection or source unlink,
verify the original prefixes, exact suffixes, candidate closure and controls;
do not require regenerating from an absent source. Unexpected tails, changed
source/prefix, same-byte replacement inode, mixed representation and interrupted
dispatch cleanup require explicit no-effect/refusal cases.

This proposal addresses serialization amplification; it does not by itself
solve same-tip exhaustion, unprofitable relocation, arbitrary live candidate
size, late free-space loss or maintenance scheduling. The existing compact
UTF-8 full-plan fixture has decode compatibility only; old whole-ticket hashes
may fence after re-encoding. A permanent codec must specify canonical original
bytes/version dispatch so an admitted original is not silently rehashed under a
different encoding. No cross-version retry promise is inferred from that fixture.

## Canonical vector and proof requirements

Use the earlier appendix's candidate canonical JSON rules for review values;
do not silently adopt a different canonicalizer. Packed framing, direct refs,
node tags and digest domains still need an explicit byte appendix. Each vector
must record the unsorted input value, exact expected canonical UTF-8/frame bytes,
byte length, SHA-256, typed interpretation and expected validation outcome.
Do not derive expected bytes with the serializer under test. Shared prefix and
plan digests need domain/version definitions and fixed cross-implementation
vectors. Unknown required fields/tags, duplicate keys and noncanonical numbers
refuse; preserved optional extensions retain the existing non-edge semantics.

The minimum linked vector set is:

| Vector group | Positive complete fixture | Required negative variants |
| --- | --- | --- |
| Bootstrap/feature dispatch | Unchanged HEAD/outer commit selecting the proposed scalable capability and direct placement roots | Missing capability; incompatible reader; flat/scalable target confusion; wrong project/bootstrap; locator self-bootstrap cycle |
| Independent closures | Different active/recovery states sharing allocations; semantic and ownership records plus actual locator pages | Missing ownership page/allocation; wrong membership; inconsistent duplicate claim; dangling locator; invented optional-extension edge; recovery requiring active |
| Operation evidence | Real supported Graph mutation, authored no-op, same-ID retry; ordinal and journal sequence deliberately different | Changed request under same ID; altered original provenance; missing/duplicate ordinal; inconsistent two indexes; receipt.before/after digest mistaken for retention edge |
| Birth and original admission | Same fixed request/token across intended, witnessed and promoted generations | Partial unbound marker; occupied final path; altered nonce/R/control base/pin; byte-identical inode substitution in the local profile |
| Finalization/accounting | Born-and-sealed payload pack plus external sealed-charge descriptor and current final tips; `R→(C,R−C)→C` | Self-hashed sealed descriptor; estimated charge persisted; overflow; `C>R`; cleanup underreserve; duplicate charge/refund; source suffix corruption before cleanup |
| Retirement | Both roots release; pending ticket still charged; exact unlink/barrier/absence; credit replay | Every pin class retaining source; source reappears; wrong generation; candidate closure corruption; absence without authorization; filesystem credit inferred from project credit |
| Compact manifests (pending) | Exact original regeneration before selection and source-independent suffix verification afterward | Mixed plan modes; altered prefix/suffix/frame count; unknown extra tail; missing source before publication; unexpected dependency on source after unlink |
| Compatibility and limits | Fixed original bytes remain interpretable across an explicitly supported codec transition; largest admitted phase fits | Re-encode changes original ticket digest; unsupported transition; worst-width later control exceeds admission envelope; bounded no-effect capacity refusal |

The earlier nineteen encoding vectors remain historical candidate examples,
not complete package-closure tests. These new linked vectors must include actual
referenced bytes and expected traversal/selection results. Pure byte/typed
validators can be disposable; production reader dispatch waits for exact-wire
review. Controlled-return/fresh-handle tests do not claim arbitrary syscall-cut,
process-kill or power-loss coverage. Qualified Saved still requires the matching
post-barrier HEAD receipt for the current accepted authored state.

## Open review points and completion boundary

The following are engineering work within approved semantics: complete wide and
tight fixed-capacity runs for both maps; demonstrate useful repeated eligible
reclamation or characterize the exact blocking bound; test the compact manifest;
measure retained live/receipt growth separately from garbage and standing
controls; finish the linked vectors and candidate byte layout. A safe capacity
refusal is valid, but a run in which every maintenance attempt refuses is not
evidence of sustained reclamation. No indefinite plateau is promised while
required receipts/live content grow. No measured fixture limit becomes a product
default. Update the consolidated evidence rather than duplicating final counts
or source hashes in this draft.

Explicit review is required for permanent feature/schema compatibility and
original-operation behavior across supported codec versions; any changed
retention/custody, authorization, Saved or visible backpressure policy; and the
qualified local binding/provider/lease model. The proposal chooses no automatic
pin release, historical-ID expiry, reusable historical grant, GUI routing,
conversion policy or production maintenance trigger. Portable schema spellings
for authority provenance remain subject to the original contract's review.

The three authorized private APFS clean-remount cases remain
[unqualified observations](PS2_MACOS_CLEAN_REMOUNT.md). No abrupt-power test is
authorized by this proposal, and none is required merely to present this wire
candidate. Storage qualification is an independent prerequisite for production
writable/Saved support. After exact-wire review, production reader and synthetic
publication/recovery implementation may follow the approved sequence; this draft
does not authorize that implementation, live writes, migration or production
retirement. RootSet/placement encoding must be reviewed before any freeze.
