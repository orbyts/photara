# PS2 wire-coordinate source audit

This is an engineering inventory of implemented disposable candidates, not a final coordinate proposal, approval request, permanent format decision or provider qualification. It records incompatible historical shapes rather than renaming them. The [original-token route](PS2_SINGLE_HEAD_ROUTE_BYTE_CANDIDATE.md) now has nineteen verified tests tying its selected envelope/overlay, physical caller and complete preflight together. Its bounded inline evidence still needs scalable replacements.

## Source boundary

The relevant generators and corresponding readers are:

| Group | Generator/data under `docs/architecture/proposals/ps2/` | Reader under `crates/photara-store/tests/` |
|---|---|---|
| Initial scalable closure | [scalable/generate.py](proposals/ps2/scalable/generate.py), [scalable/linked-vectors.json](proposals/ps2/scalable/linked-vectors.json) | [scalable_wire_candidate/wire.rs](../../crates/photara-store/tests/scalable_wire_candidate/wire.rs) |
| Real operation semantics/index | [operations/generate.py](proposals/ps2/operations/generate.py), [operations/linked-operations.json](proposals/ps2/operations/linked-operations.json) | [scalable_operation_candidate/operations.rs](../../crates/photara-store/tests/scalable_operation_candidate/operations.rs) |
| Multilevel typed trees | [branches/generate.py](proposals/ps2/branches/generate.py), [branches/linked-branches.json](proposals/ps2/branches/linked-branches.json) | [scalable_branch_candidate/trees.rs](../../crates/photara-store/tests/scalable_branch_candidate/trees.rs) |
| Portable resource/conversion closure | [resource-conversion/generate.py](proposals/ps2/resource-conversion/generate.py), [resource-conversion/linked.json](proposals/ps2/resource-conversion/linked.json) | [resource_conversion_candidate/wire.rs](../../crates/photara-store/tests/resource_conversion_candidate/wire.rs) |
| External phase subproof | [phases/generate.py](proposals/ps2/phases/generate.py), [phases/linked-phases.json](proposals/ps2/phases/linked-phases.json) | [scalable_phase_candidate/phases.rs](../../crates/photara-store/tests/scalable_phase_candidate/phases.rs) |
| Joined settled snapshot | [joined/generate.py](proposals/ps2/joined/generate.py), [joined/linked-joined.json](proposals/ps2/joined/linked-joined.json) | [scalable_joined_candidate/joined.rs](../../crates/photara-store/tests/scalable_joined_candidate/joined.rs) |
| Original-token route | [route/generate.py](proposals/ps2/route/generate.py), [route/linked-route.json](proposals/ps2/route/linked-route.json), [route/operation.py](proposals/ps2/route/operation.py) | [single_head_route_candidate.rs](../../crates/photara-store/tests/single_head_route_candidate.rs), [selected reader](../../crates/photara-store/tests/single_head_route_candidate/route.rs), [contract](../../crates/photara-store/tests/single_head_route_candidate/contract.rs), [control model](../../crates/photara-store/tests/single_head_route_candidate/control.rs) |

The checked-in canonical corpora and their source hash manifests establish the individual proof epochs. The audit does not turn historical bytes into a common reader. The native `ps2_index_furnace` remains a different disposable encoding and operational proof, not the byte-candidate decoder.

## Reference and framing coordinates

The packed specimens exercise the JSON ObjectRef subset `{kind:"json",sha256,byte_length}`: SHA-256 covers exact canonical JSON body bytes and the length is a canonical decimal string. The existing permanent ObjectRef contract also supports Blob and orders Blob before Json; its digest domain is unchanged. These JSON-only packed vectors do not establish Blob placement or conversion compatibility. PhysicalRef is `{allocation_id,arena,offset,byte_length,record_sha256}`: the digest covers the complete selected frame, the arena is `data` or `metadata`, and offset/length identify an exact frame boundary. A physical allocation UUID is not a resource ID, content digest or portable device identifier.

Candidate frames use eight magic bytes `PS2PKD01`, a one-byte tag, three zero reserved bytes, little-endian u32 body length, then body bytes. Header length is 16 bytes. Tag 0 is zero-filled padding; tag 1 is semantic JSON; tag 2 is ownership/accounting implementation JSON; tag 3 is locator JSON in the metadata arena. Readers reject unsupported tags, wrong membership, malformed boundaries and nonzero reserved/padding content. Tag assignment is not inferred from an arbitrary ObjectRef-shaped extension. The earlier native furnace uses other frame layouts and does not share this magic/header contract.

Canonical body identity, full framed-record identity and prefix/suffix identities are distinct commitments. Raw allocation and compact suffix hashes include every committed byte, including padding and unreachable intermediate frames. No selected semantic closure proof alone authenticates those unreachable bytes.

## Record names and field shapes

Unless identified otherwise below, these candidate records use schema version 1, `project_id`, and an `extensions` object in addition to listed fields. Names beginning `example.ps2` are explicitly unfrozen. The inventory below excludes ordinary pre-existing Graph authoring records and negative-only injected fields; those retain their own typed semantic reader.

| Actual schema ID | Implemented fields beyond common envelope | Source group |
|---|---|---|
| `example.ps2.root-set` | library_id, bootstrap_sha256, kind, active, recovery, pinned_roots, operation_index, conversion_source, inventory, placement | scalable, phase, joined |
| `example.ps2.state-root` | library_id, bootstrap_sha256, root_id, authored_revision, authored, history, resource_state, operation_index, accepted, journal_inclusion, predecessor, inventory | scalable, phase, joined |
| `example.ps2.inventory-root` | count, entries | initial scalable only |
| `example.ps2.inventory-leaf` / `inventory-branch` | count, entries / count, children | branches, joined |
| `example.ps2.locator-leaf` / `locator-branch` | count, entries / count, children | scalable, operations, branches, joined |
| `example.ps2.ownership-claim` | allocation_id, arena, authenticated_prefix, owned_extent, sealed, sealed_charge | scalable, joined |
| `example.ps2.ownership-branch` | count, children | scalable, joined |
| `example.ps2.sealed-charge` | allocation_id, arena, domain_incarnation, measured_extent, charged_high_water, observation | scalable, branches, joined |
| `example.ps2.operation-index-root` | accepted, operation_ids, ordinals | initial scalable empty-index probe |
| `example.ps2.operation-index-tree` | library_id, bootstrap_sha256, accepted, by_id, by_ordinal | operations, branches, joined |
| `example.ps2.operation-id-leaf` / `operation-ordinal-leaf` | count, entries | operations, branches, joined |
| `example.ps2.operation-id-branch` / `operation-ordinal-branch` | count, children | branches, joined |
| `example.ps2.owned-charge-leaf` / `owned-charge-branch` | count, charged_high_water, entries / count, charged_high_water, children | branches, joined |
| `example.ps2.physical-accounting` | domain_incarnation, standing_control, tips, sealed_charges, unresolved, total_charge | initial scalable |
| `example.ps2.accounting-tree` | profile, incarnation, standing_control, tips, sealed_charge_tree, sealed_observations, tickets, retained_conversion, total_charge | joined settled |
| `example.ps2.resource-closure-projection` | library_id, root_id, resource_state, representation, features, pin_sources, inventory | resource subproof only |

Tree details are significant schema coordinates. Inventory child bounds are ObjectRefs; operation-ID child bounds are UUIDs; ordinal bounds are canonical decimals. Branch children bind `first,last,count,child`; owned-charge children additionally bind subtree charged high water. Locator entries bind logical reference, physical reference and semantic/ownership membership. Exact distinct subtree membership, order, count and bounds are checked; the reusable multilevel fixture currently bounds fanout to 2–4. A leaf-shaped array is not interchangeable with a branch root.

Joined placement is `{generation,active_locator,recovery_locator,active_ownership,recovery_ownership,accounting}`. Ownership authenticated-prefix hashes are distinct from extent-only ownership and sealed charge. StateRoot predecessor is provenance, not a retention edge: settled joined uses null when no actual prior StateRoot is supplied, while the phase subproof binds a genuine predecessor where available.

## Phase and new route records

The frozen phase-only family is deliberately separate:

| Actual schema ID | Fields beyond common envelope |
|---|---|
| `example.ps2.phase-local-scope` | profile, incarnation, codec, project_binding_sha256, qualification |
| `example.ps2.phase-accounting` | profile, incarnation, standing_control, tips, sealed, tickets, total_charge |
| `example.ps2.phase-inventory-leaf` / `phase-locator-leaf` | count, entries |
| `example.ps2.phase-ownership-leaf` / `phase-ownership-branch` | claim / children |
| `example.ps2.phase-original-admission` | token, scope, request, receipt, original_selection, target_selection, original_head_sha256, target_head_sha256, reserve, cleanup_bound, maximum_control_frame, maximum_simultaneous_controls, standing_control, original_charge, source_witnesses, payload_sha256, births, existing_tip, codec |
| `example.ps2.phase-retirement-admission` | token, scope, original_selection, target_selection, original_head_sha256, target_head_sha256, source, reserve, cleanup_bound, maximum_control_frame, maximum_simultaneous_controls, standing_control, compact, codec |
| `example.ps2.phase-birth-marker` | token, scope, allocation_id, nonce |
| `example.ps2.phase-birth-proof` | original, marker, witness, events, promotion_pair, final_links, stage_absent, namespace_observation |
| `example.ps2.phase-compact-plan` | mode, full_data, full_metadata, data, metadata, target |
| `example.ps2.phase-selected-phase` | previous, phase, original, selection, head_sha256, consumed, held, plus exact variant-specific birth/payload/finalization/cleanup/authorization/absence/credit fields |

`phase-selected-phase` has admitted, witnessed, payload, finalized, published, clean, retirement-held, released, unlink-authorized, unlinked, absence and credited variants. The phase reader enforces each variant's exact field set. Full and compact plan modes are mutually exclusive; raw manifests bind original prefix, final end, suffix length/hash/count and scoped witness. These external phase records bind whole target RootSets and HEAD hashes; they cannot simply become children of those same targets without a self-reference. They are not the new route authority.

The new `single_head_route_candidate/contract.rs` defines different version-1 records:

- `example.ps2.route-original`: token, kind, scope, old, target, recipe, reserve, **project_limit**, cleanup_bound, maximum_control_bytes, maximum_control_count, operation. Kind is graph or retire. Scope binds profile/incarnation and `example.ps2.route-v1`. `old` embeds exact head/commit/ledger evidence and nullable envelope/overlay reconstruction values. A prior route envelope/overlay is committed evidence, not a retention edge to prior admission/phase. `target` binds active/recovery/operation_index/conversion_source/base_inventory plus placement without the accounting selector. Recipe codec is `example.ps2.closed-layout-v1`; each allocation binds allocation_id, arena, witness, old_end, old_sha256, end, sha256, append_sha256, frame_count. Optional retirement source binds allocation_id, witness, byte_length, sha256, registered_charge. Graph operation holds intent/receipt ObjectRefs.
- `example.ps2.route-phase`: original ObjectRef, stage, consumed, remaining. Non-admitted variants add exact observations `{allocation_id,witness,extent,registered_charge}` for every original tip. Clean adds credit and cleanup `{remaining_roles:[],directory_barrier:true}`. Graph permits admitted→published→clean; retirement permits admitted→released→unlinked→clean.

Original admission requires old total plus whole reserve to fit immutable project_limit before effects, and planned positive profile-rounded tip growth plus cleanup to fit reserve with checked arithmetic. Publication charges C once and holds R−C; later stages preserve those exact observations/C, and cleanup releases the remaining hold. The maximum-control value here is an **aggregate simultaneous byte cap**, unlike phase subproof per-frame sizing. The physical route caller must prove actual role count/sum and worst permitted later control forms before admission, original sealed-charge preservation/source-to-ticket transfer, and actual supplied bytes/observations. The helper alone does not prove selected authority or actual filesystem barriers.

## Portable resource/conversion names

The resource/conversion reader uses `photara.resource.identity`, `working-binding`, `captured-version`, `backing`, `publication-evidence`, `retention-obligation`, `state`, `photara.project.representation-content`, and `photara.package.conversion-source`. Their field authority comes from `resource_conversion_candidate/wire.rs` and the existing package planning typed contracts, not the `example.ps2` helper schemas. Representation-content candidate version 3 carries the durable binding; version-2-with-binding is an explicit rejected case. Identity carries resource_id/custody/purpose/producer; captured version carries resource/version identity, byte length/hash/media type and capture evidence; backing names a backing ID/location and publication evidence; obligation names required backing IDs/qualification/copy count. State holds typed identity/version/backing/obligation/working-binding references. Conversion source commits snapshot directory and exact source files with source bootstrap/HEAD/commit/format identity.

Portable backing/representation/conversion records must not acquire host device/inode identity. The resource fixture explicitly rejects a working-binding inode addition. Local allocation witnesses use `{profile,incarnation,allocation_id,device,inode}`; retained source-file observations additionally bind local namespace/path. These live in local physical accounting/observation evidence and remain synthetic/unqualified here. Digest identity is not a claim that copied bytes preserve physical allocation charge or local inode authority.

## Digest domains and dispatch

The real-operation generator/reader use exact canonical SHA-256 inputs with domains `photara.package.operation-intent.v1`, `photara.package.accepted-prefix.v1`, `photara.package.accepted-prefix-link.v1`, `photara.package.journal-prefix.v1`, and `photara.package.journal-prefix-link.v1`. Intent identity includes project/library/bootstrap, operation UUID, expected authored coordinate, command and canonical update metadata. Accepted links include ordinal, operation/request/receipt hashes; journal links include prior prefix and exact frame hash. Acceptance ordinal and journal sequence remain separate coordinates. `photara.package.accepted-journal-frame` schema version 1 binds journal_id, sequence, operation_id, request_sha256 and receipt_sha256. `photara.package.operation-receipt` schema version 1 remains an immutable typed record under the actual operation reader. Route/phase original ObjectRefs hash exact original canonical bytes without adding a new digest domain or reserializing them into a successor codec.

HEAD and outer commit retain `photara.package.head` and `photara.package.commit` version 1. Current joined/phase dispatch requires minimum reader 1.3 and capability sets that include original manifest capabilities plus relevant `photara.sealed-roots.v1`, `photara.resource-backings.v1`, `example.ps2.scalable-storage.draft-v1`, `example.ps2.typed-branches.draft-v1`, and `example.ps2.owned-accounting.draft-v1`. Exact sets differ by component; they are not a shared final feature negotiation table. Legacy readers are expected to fence unsupported capabilities before effects.


## Current executable bounds

These are fixture reader limits, not final package or product limits. A final coordinate proposal must choose explicit limits and overflow/unsupported dispatch consistently; it cannot silently inherit whichever component helper happened to parse a record.

| Reader | Enforced bound |
|---|---|
| Shared candidate frame reader | 1 MiB allocation/JSON wrapper, u32 framed body length, exact 16-byte header; checked frame-end arithmetic |
| Operation index subproof | 1–32 linked accepted operations; canonical package JSON parsing |
| Typed branch traversal | recursion level below 16; fewer than 256 prior visited nodes; nonempty leaves at most 3 entries; branch fanout 2–4; subtree count at most 4096; locator leaf at most 256 entries |
| Joined physical locator | depth below 16, fewer than 256 prior visited locator nodes, nonempty leaf at most 256 entries, fanout 2–4 |
| Joined ownership | depth below 16, fewer than 256 prior visited ownership nodes, branch fanout 2–8 |
| Resource/conversion subproof | 128 array/record/source-file entries; 1 MiB JSON bodies; at most 2 MiB hex input encoding 1 MiB bytes |
| Frozen phase subproof | 1 MiB allocation wrapper; admitted new/end corridor at most 262144; control frame at most 16384; actual supplied phase bytes checked against original cap |
| New route helper | nonempty original tip set at most 16; recipe allocation count no greater than original tips; exact all-tip observations; finite control count 1–64; original/phase body no larger than immutable aggregate cap; cap no greater than original standing control; checked capacity/reserve/framing arithmetic |

Package canonical parsing uses `JsonLimits::default()` from [package/json.rs](../../crates/photara-store/src/package/json.rs); its defaults are 16 MiB total input, depth 64, 4096 members and 100000 array elements, inherited separately from the stricter candidate framing limits. The route helper accepts already parsed `Value`s and cannot replace the caller's bounded byte parser. Canonical u64 decimal strings, lowercase SHA-256 text and nonnil canonical UUID constraints are checked in the current strict path; historical component shortcuts are recorded below.

## Incompatibilities and review obligations

1. Initial scalable inventory-root/operation-index-root/physical-accounting, phase-prefixed leaves/inline accounting, and joined multilevel inventory/index/accounting-tree are different layouts. Matching version numbers do not make different schema IDs interchangeable. Even shared RootSet/StateRoot IDs need one final complete nested placement/accounting/typed-edge interpretation; a projection proof is insufficient.
2. Initial scalable bootstrap probes are historical component evidence: their generated outer commits omit the mandatory current bootstrap field and use probe revision/predecessor values. Correct joined/phase envelopes add authentic bootstrap and provenance checks. Do not promote the earlier probe generator wholesale into the final unchanged-envelope claim.
3. Lifetime-sized inline arrays remain in historical operation-index-root, physical-accounting.sealed_charges and phase-accounting.sealed. Joined replaces charge entries with a tree, but sealed_observations, retained_conversion.files, resource-state member arrays and conversion-source.files still require explicit bounded-versus-lifetime treatment in the final review. Four current joined tips are fixture topology, not a promised universal constant. Route recipe/observation arrays are bounded per original operation, not a lifetime registry.
4. Extension validation differs: initial scalable/resource readers use dot-presence checks; operation/branch readers have component-specific object validation; frozen phase/joined checks are not the new route's full grammar guarantee. The new route helper uses existing `contracts::schema::QualifiedName` and admitted byte caps. Unknown extension-shaped refs stay nonedges. Final required schema fields/feature IDs must not be smuggled into ignored extensions.
5. Distinguish original canonical-byte preservation across reader evolution from unsupported codec transition. Frozen phase reader generations 1/2 retain admitted codec-v1 bytes; they do not establish a distinct codec-v2 representation or upgrade policy. New route has explicit codec identity and must fence unsupported originals.
6. Final authority must traverse one actual HEAD→commit→RootSet→accounting-envelope→original/phase route and exact overlay/base inventory. Control aliases already in the base inventory must not be duplicated, and equal aggregate charge cannot authorize swapping original sealed allocation identities. Actual destination suffixes, including unreachable records, must be verified after authorized source unlink; detached corpus copies cannot substitute.

The audit does not supply a final newborn-generation or codec-transition field specification; that explicit proposed coordinate layer still follows the integrated route review.

The completed route adds full old active/recovery StateRoot bodies to `old.states`
and exact source `charge` plus `charge_path` records to the retirement recipe.
The latter is the entire three-charge fixture tree, not a sparse proof. Its
`example.ps2.route-selector-intent` v1 binds token, original ObjectRef and exact
current/next HEAD hashes in 447 canonical bytes. Complete admission checks the
entire same-original phase sequence and simultaneous control allocation before
effects; current full and recovery readers also enforce the selected control
pool. Peak simultaneous allocation is exactly 131072 bytes across nineteen
roles under the original twenty-four-role cap. These are bounded model results,
not filesystem qualification or final permanent coordinates.

Existing managed-resource Blob edges are implemented in
[v1.1 closure traversal](../../crates/photara-store/src/package/v1_1/closure.rs)
and required by the original [ObjectRef contract](PS2_PRODUCTION_WIRE_APPENDIX.md).
Final placement/ownership/accounting needs compatible Blob byte evidence; an
unsupported-packed-Blob fence in a JSON-only prototype cannot silently narrow
the existing permanent conversion contract.

This audit records current source facts and remaining mapping work. It proposes no permanent magic, schema name, feature ID, pin policy, coordinate family or codec transition.
