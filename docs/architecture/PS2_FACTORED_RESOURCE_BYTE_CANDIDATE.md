# PS2 factored resource byte candidate

Status: **UNFROZEN proposed additive coordinates, disposable metadata only**.
This candidate addresses a concrete write-scaling gap in the current
[resource projection](PS2_RESOURCE_CONVERSION_BYTE_CANDIDATE.md): its authored
obligation origins name the selected StateRoot ID. A different StateRoot ID
therefore changes every such obligation, even when the resource requirements
are unchanged. Replacing only the ResourceState arrays with trees does not
remove that repeated rewriting.

This specimen does not change the [reviewed retention semantics](PS2_PRODUCTION_WIRE_APPENDIX.md#additive-managed-backing-records-and-portable-binding),
production schemas, package reader, existing vectors, pin classes, release
eligibility, conversion behavior, or qualification policy. The proposed IDs and
field layouts below require explicit wire review. They are not globally renamed
fixture aliases or an implemented production format.

## Concrete additive shape

Every record has exact `schema`, `project_id`, and checked, preserved
`extensions`. Unknown required fields refuse. Extension keys use the actual
`QualifiedName` grammar; extension values create no reference edges. Existing
canonical JSON, exact ObjectRef bytes/hash/length and typed UUID/string-decimal
rules apply.

| Proposed coordinate | Exact fields beyond common fields | Typed edges and meaning |
| --- | --- | --- |
| `photara.resource.state` v2 | `library_id`, `identities`, `working_bindings`, `versions`, `backings`, `requirements`, `retention_sources` | All six selection members are JRefs to trees of their explicit selection kind, including the source-association tree. V1 remains the unchanged flat selection/obligation contract. |
| `photara.resource.selection-leaf` v1 | `selection_kind`, `count`, `entries` | Entries are ordered, unique `{id,record:JRef}`. Kind determines target ID field: resource/binding/version/backing/requirement/association ID. |
| `photara.resource.selection-branch` v1 | `selection_kind`, `count`, `children` | Children are `{first,last,count,child:JRef}` with exact recursive range/count summaries. Child ranges are strictly ordered and disjoint. |
| `photara.resource.retention-requirement` v1 | `requirement_id`, `revision`, `resource_id`, `version_id`, `minimum_qualified_copies`, `required_backing_ids`, `required_qualification` | Per-version requirements retain the existing copy, backing and qualification meanings. The origin moves into a separately selected association. IDs and qualification coordinates are not JRefs. |
| `photara.resource.retention-source` v1 | `association_id`, `origin:{kind,source_id}`, `requirements:JRef` | The requirement tree selects an arbitrary subset, not necessarily the complete requirement set. Origin must resolve to the actual selecting StateRoot context. The requirement-tree reference is a retention edge; association and source UUIDs are checked identities, not filesystem references. |

The new path requires the explicit draft capability
`photara.resource-state-trees.v1` together with
`photara.resource-backings.v1`. The proposed capability is unfrozen; its floor and binding
to StateRoot v2 remain the whole-wire proposal's decision. A v1 state cannot
silently acquire tree fields, and a v2 requirement record under the new v1 ID is
not accepted.

An association UUID identifies one exact immutable association record. Changing
its origin or subset creates a new association ID; an unchanged association may
reuse its exact ID and bytes. Across retained roots the same association ID
cannot resolve to different records. Two subsets of the same origin have
distinct association IDs.

Each association subset must resolve the exact immutable requirement records
selected by the global requirement tree. A shared requirement ID with different
bytes refuses. The union of association subsets must cover the global set
exactly; unassociated requirements refuse. Overlapping associations may retain
the same exact requirement. The fixture uses two disjoint interleaved subsets,
showing that there is no all-requirements-share-one-source assumption.

The original identity, working-binding, captured-version, backing and
capture/publication-evidence schemas remain v1. Their seven selected metadata
and representation records for the first resource remain byte-identical to the
prior linked specimen. CapturedVersion gains no backing or retention pointer.
Representation-content v3 still binds the exact selected captured version,
resource/version IDs, media information and verified fingerprint; its v2 form
is rejected for `managed-captured`. Context ResourceValue gains no new case.

## Executed bounded proof

The [independent Python generator](proposals/ps2/resource-factored/generate.py)
produces [canonical records](proposals/ps2/resource-factored/linked.json).
The [Rust target](../../crates/photara-store/tests/resource_factored_candidate.rs)
and its [direct validator](../../crates/photara-store/tests/resource_factored_candidate/wire.rs)
validate the actual proposed v2/tree/association bytes. They do not rewrite a
synthetic v1 state or root to obtain a validation result. Existing scalar and
canonical-byte helpers are reused read-only.

Two distinct supplied StateRoot contexts select the new resource state. These
contexts are explicitly resource projections, not invented complete StateRoot
inventories or an independently verified package HEAD. Their selected root IDs
are different. All requirement and selection-tree bytes are shared; changing
the root changes only its association records, source-selection paths and resource-state
record.

| Resources | Shared closure records | New root-local records | New root-local bytes |
| ---: | ---: | ---: | ---: |
| 1 | 12 | 3 | 1,843 |
| 16 | 143 | 4 | 2,411 |
| 64 | 575 | 4 | 2,411 |

Removing every object unique to root A leaves root B's identical requirement
closure independently readable. No release of A's association releases B's
requirement. These figures prove bounded changed resource bytes for the tested
Graph-only root change; they do not claim constant-time full validation,
constant total package size, or bounded whole-package physical write cost.
The validator currently walks all selected metadata and evaluates backing
support across selected requirements; production indexing/performance is not
implemented here.

Twenty independently generated scenarios include the valid case, seventeen
coherently rehashed typed refusals, and two structurally readable but unsupported
retention states. Refusals cover wrong/unresolved root origins, uncovered or
conflicting requirements, missing required backing, zero-copy requirements,
wrong record versions, unknown fields, wrong tree kind/range, and an illegal
CapturedVersion policy pointer, duplicate/changed association IDs and aliased
source-tree nodes. Reusing an association ID with different root-local bytes
refuses when checking both retained roots. A two-copy requirement with one backing and a
mismatched qualification remain readable metadata, with support `false`; they
fail the separate retained-publication admission helper. Other tests cover
capability dispatch, v3 binding, nonedge opaque extensions and preservation of
original v1/v3 bytes.

The supported qualification is the same explicitly synthetic, single-copy
fixture profile. Metadata support is not proof that the declared 8 GB media
exists or is durable. The store exposes canonical metadata records only; no
media path is opened and no external media bytes are read or hashed.

## Bounds and remaining composition gates

Fixture bounds are 4 entries per leaf, 2–4 children per branch, depth below 8,
at most 256 entries per selection kind (including source associations), and 4,096 stored
records/closure entries. Canonical records use the shared 1 MiB parse cap.
These are test limits, not product limits or lifetime pin ceilings. A separate
positive case selects sixteen associations through real source-tree branches,
exceeding the former four-item array fixture. The cost of a Graph root change
is proportional to changed origin associations and their tree paths, not a
promise of constant work as the number of changed origins grows. The two-group
cases isolate the resource-count-independent part of that change.

This specimen supports authored origins derived from the selecting root only;
other approved origin/pin classes are not reinterpreted or removed. Their exact
contexts and evidence dispatch remain required in the final proposal. The
source index is now typed and scalable; general multi-origin semantics are not
proved by authored-only examples.

Still required before claiming one complete scalable package: compose these
bytes with the sole selected HEAD, StateRoot v2 inventories, operation roots,
placement, physical accounting and original phase authority; specify complete
pin/origin dispatch; review actual names/versions and
capabilities; and prove that resource-tree updates and ownership/charge updates
fit the admitted physical write bounds. No v1-to-v2 migration policy or
production storage/lease qualification is chosen here.

The exact source/output hashes and commands are recorded in the
[focused evidence](verification/ps2-factored-resource-candidate.json).
