# PS2 retention-origin resolution proposal

Status: **read-only design proposal, unfrozen and unimplemented**. This maps
approved retention meanings onto a final selected-package resolver. It does not
change the frozen [factored resource vectors](PS2_FACTORED_RESOURCE_BYTE_CANDIDATE.md),
choose release/expiry policy, or turn local lease observations into portable
resource authority.

The [wire appendix](PS2_PRODUCTION_WIRE_APPENDIX.md#additive-managed-backing-records-and-portable-binding)
requires authored/history/recovery origins to resolve their corresponding
selected package state and explicit/pending origins to have corresponding
portable policy or operation evidence. The
[approved resource boundary](RESOURCE_STORAGE_AND_VERIFICATION_CONTRACT.md#capture-is-not-indefinite-retention)
also requires conservative revalidation against all other pins and unknown
outcomes. These are distinct requirements: resolving one resource origin does
not prove that a physical allocation is eligible for retirement.

## Concrete resolver input and identity

Keep the proposed immutable `retention-source` record as
`{association_id,origin:{kind,source_id},requirements:JRef}` plus its existing
common schema/project/extensions fields. Changing origin or requirement subset
creates a new association ID. The selected source tree and requirement tree
already establish exact subset coverage and shared-ID equality.

The final reader derives an origin resolver from actual selected data, not a
caller-supplied list of accepted IDs:

1. Resolve RootSet active, recovery and every explicitly pinned StateRoot,
   retaining their exact ObjectRefs, root IDs, library/project/bootstrap identity,
   authored/history references and role/pin evidence. The same ID cannot resolve
   contradictory retained records. Predecessor commitments add no states.
2. Resolve explicit/pending evidence through a separately selected typed portable retention-evidence index, keyed
   by `(origin kind, source_id)`. This is a **proposed additional selected edge**,
   not an authoritative second selector. It must appear in the exact RootSet
   dependency union and be covered by placement/ownership/accounting. Its final
   field and schema coordinates remain for the lead wire proposal to settle.
3. Resolve admitted operation references through HEAD-selected original
   admission and phase records. Exact operation identity, original request
   digest, scope/incarnation and selected unresolved phase must agree. A record
   found in a loose directory or an old corpus is not selected evidence.

Resource readers receive verified contexts derived from these traversals.
They do not synthesize an origin by copying its ID out of the association they
are supposed to authenticate.

## Five origin cases

| Origin | Source identity and positive evidence | Refusals and limits |
| --- | --- | --- |
| `authored` | `source_id` is the owning retained StateRoot's `root_id`; the source association is selected by that root's ResourceState, alongside its exact authored projection. Active, recovery or an explicit pin may retain that authored snapshot. | A previous root digest, same-ID different record, or association imported from another root does not resolve. This does not add an edge from CapturedVersion to a backing or requirement. |
| `history` | `source_id` identifies the owning retained StateRoot context; its exact selected history reference and history-scoped source association declare the selected reconstruction requirement subset. | Mere receipt provenance, predecessor identity or an artifact's arbitrary retention text does not imply a reconstruction obligation. Do not upgrade all old history entries into media pins. A precise source context must remain retained. |
| `recovery` | `source_id` identifies a retained recovery StateRoot with its independent closure and corresponding selected recovery-role or unresolved-recovery pin evidence. | Never fetch a former recovery root solely from predecessor provenance. A role change must preserve an explicit selected source context or reconcile the origin; it cannot silently relabel or release the requirement. Exact role/pin transitions remain a wire validation case. |
| `explicit` | `source_id` is a portable policy ID resolved through a selected immutable policy record. Proposed minimal evidence: project/library/bootstrap identity, `policy_id`, `revision`, and an exact requirement-subset tree. Each associated requirement must be an identical member of that policy's selected subset. | A UUID alone, device preference, credential, path or a generic operation receipt is not policy evidence. No implicit expiry, freshness requirement or universal replica default is introduced. The selected policy is metadata evidence, not a fresh permission grant. |
| `pending` | `source_id` is the original OperationId. Proposed evidence binds that ID, original canonical request digest, package scope/incarnation, exact selected original-admission reference and the requirement subset. The selected phase must still retain unresolved work or outcome evidence. | Receipt presence alone does not prove a pending operation, and request-digest equality alone does not prove current selection. An unrelated operation, replacement token, terminal phase presented as pending, or absent original hold refuses. Unknown outcome keeps its original evidence and holds; it is not a timeout-based release. |

For explicit policy and pending-operation evidence the table specifies required
meaning and exact links, not final JSON schema IDs. A policy record is permitted
by the existing appendix as evidence in its own right; this proposal does not
invent a new production SetPolicy command or credential flow. If an operation
receipt is also recorded as policy provenance, it remains provenance unless an
explicit typed contract gives it a stronger role.

The pending requirement tree must be fixed as part of the original admitted
request or original immutable retention intent. Do not put the current phase,
current accounting envelope or containing RootSet hash back into that original
record. An acyclic construction is: fixed requirement tree and original intent,
then immutable original admission, then pending evidence referring to those
fixed bytes, then selected phase/accounting/RootSet. A pending source record may
be independently selected; it need not be made part of an original payload
projection that would hash back to itself. The final joined specimen must make
these edge exclusions and the full union explicit.

The authored-only resource fixture proves one case. Extending it requires
positive and negative linked contexts for the other four cases; none is
silently covered by its existing `source_id == root_id` check. In particular,
old production history-artifact validation accepts `retention` as text
([records.rs](../../crates/photara-store/src/package/v1_1/records.rs:387)); that
is not an existing typed reconstruction-policy dispatcher.

## The twelve operational pin classes are not twelve origin tags

The [furnace enum and selected pin record](../../crates/photara-store/examples/ps2_index_furnace/placement_v3.rs:1635)
model the following blockers. A resource association may provide an additional
obligation for some of these operations; it never replaces their package and
physical allocation holds.

| Operational class | Exact selected or local evidence to retain | Relationship to resource origins |
| --- | --- | --- |
| Active | RootSet active StateRoot, semantic inventory and physical ownership closure | Its selected authored/resource associations commonly use `authored`; the active root is independently a package pin. |
| Recovery | RootSet recovery StateRoot and its independently readable physical closure | Supports the corresponding recovery context; recovery retention is not lost when active changes. |
| AcceptedJournal | Original journal incarnation, accepted prefix, original operation IDs/requests/receipts and checkpoint inclusion evidence | May support a `pending` association for resource work. An accepted Graph receipt is not automatically a media-retention policy. Dedupe survives undo eviction. |
| Undo | Exact retained undo/group snapshot or root, with selected undo pin and applicable original token/epoch | May retain authored/history requirements of that snapshot. It is not a new resource-origin enum, and the fixture chooses no new undo horizon. |
| History | Exact selected history closure and any explicitly pinned history root | Supports only declared history/reconstruction requirements, not every provenance descriptor. |
| ConversionSource | RootSet ConversionSource, original manifest/HEAD/commit and exact original regular-file snapshot | Separately retained package bytes. The first approved contract has **no automatic release**; do not fabricate an external resource origin for this pin. |
| Export | Exact original export operation and its selected source snapshot/requirement subset, retained through completion/reconciliation | Can require pending resource retention, but export's package/reader hold is separate. Export completion is not automatic release of every other source. |
| Backup | Exact original backup operation or selected policy and captured source snapshot | Can use explicit or pending evidence according to its actual contract. A successful backup does not end the source's unrelated requirements. |
| ReaderLease | Exact reader token/epoch/snapshot and original local profile/incarnation; actual reader completion | A local concurrency blocker. Do not serialize a live access grant, native handle or lease as a portable retention-source policy. |
| UnresolvedIntent | Exact original admitted intent, old/candidate controls, original request/token and unresolved outcome | Pending evidence can refer to it. Unknown outcome retains both dispatch evidence and liability until original reconciliation. |
| Relocation | Exact original source charge/witness, destination plan, both selected roots and retirement ticket/phase | Holds source/destination allocations independently of resource pin categories. Publication, absence and once-only credit proofs remain required. |
| ResourceObligation | Exact selected requirement/source associations and retained backing/evidence closure | This is the aggregate resource-retention blocker, not another origin tag. Releasing one source leaves all other roots/sources effective. |

The current [furnace release guard](../../crates/photara-store/examples/ps2_index_furnace/placement_v3.rs:1937)
requires completed work, reader completion for ReaderLease, reconciliation for
unknown outcomes and exact token/epoch/snapshot equality. Those are fixture
proof inputs, not an approved production policy for how a UI ends a promise.
The [approved retirement rule](PS2_RETENTION_STORAGE_DECISION.md#recommended-policy-to-develop-and-verify)
requires all retained roots/history/undo/recovery references and other
obligations to end before eligibility. No origin resolver changes it.

## Bounds and minimum new evidence

Use typed trees for the portable evidence index and source/requirement
selections; do not put lifetime evidence into a new flat array. Check exact
identity uniqueness, leaf order, child ranges/counts, canonical bytes, record
and depth limits before use. Runtime reader/operation holds need their existing
finite admission bounds and qualified local control authority; they are not
bounded by claiming that only five origin kinds exist. Fixed fixture limits
are not production pin ceilings.

The smallest next joined proof should resolve all five origin kinds from one
selected RootSet, preserve twelve operational blockers as separate test cases,
and demonstrate one requirement shared across roots and origins. Positive
release removes one exact association while another retains the requirement.
Negatives must coherently rehash wrong root/role/history projection, unresolved
policy, changed policy subset, wrong original operation/request/incarnation,
terminal-as-pending, duplicate evidence identity and contradictory shared
association identity. After source removal the selected closure and original
charge evidence must verify without an unselected historical source fallback.

This is authorized schema/closure engineering under existing retention
semantics. Choosing a new expiry rule, weakening reconstruction or copy
requirements, deciding a new automatic release policy, or granting production
custody/security authority would require a separate decision. No such choice
is made here.
