# PS2 linked real-operation byte candidate

Status: **unfrozen pure-test proposal, 2026-09-27**. This extends the operation
vector group of the [scalable review candidate](PS2_SCALABLE_WIRE_REVIEW_CANDIDATE.md)
while preserving the earlier [packed bootstrap specimen](PS2_SCALABLE_PACKED_BYTE_CANDIDATE.md)
as a separate historical subset. New storage schema names below are experimental;
receipt, semantic-intent, accepted-prefix and journal-prefix meanings remain
those of the [existing appendix](PS2_PRODUCTION_WIRE_APPENDIX.md).

The [standalone test](../../crates/photara-store/tests/scalable_operation_candidate.rs)
uses actual `photara_core::apply_graph_command` and the existing production
`photara_store::package::planning::plan`. There is no command string simulator,
production reader change, native filesystem effect or permanent format freeze.
The [private operation verifier](../../crates/photara-store/tests/scalable_operation_candidate/operations.rs)
reads only supplied byte arrays.

This is a linked operation and authored-semantic closure specimen, **not yet a
joint bootstrap/ownership/accounting/publication proof** for these changed
operation roots. The fixture's `selected_roots` projects the relevant StateRoot
fields; it is not a new authoritative selector or substitute RootSet. The
separate phase vector group must compose these exact logical records with both
physical locator/ownership closures and the original admission protocol.

## Real mutation, authored no-op and later retry

The independent generator starts from the checked D19 node definition specimen,
constructs a minimal real package v1.1 with one Graph/node, empty resource/asset/
history lists and complete node/context dependencies, and fixes its exact bytes.
The existing `VerifiedClosure::verify` accepts those 15 in-memory source files.
No external resource is opened. Source bytes are test-oracle inputs; they are not
silently proposed as mandatory production retained staging.

The sequence is:

| Acceptance ordinal | Journal sequence | Operation ID suffix | Actual requested command | Authored result |
| ---: | ---: | --- | --- | --- |
| 1 | 3 | `…000030` | Set existing node position to `(5,6)` | Graph revision 0→1, authored revision 1→2 |
| 2 | 7 | `…000020` | Set the same position to `(5,6)` | Existing PS1 returns `Unchanged`; authored bytes/revision stay exactly equal |
| 3 | 11 | `…000010` | Set position to `(9,10)` | Graph revision 1→2, authored revision 2→3 |

IDs intentionally sort in the reverse order from acceptance ordinals. Active
selects all three original receipts; recovery independently selects the first
two. Their shared receipt bytes and recomputed accepted prefix must agree.

The no-op distinction is tested explicitly. Core's
[`apply_graph_command`](../../crates/photara-core/src/command.rs) always advances
its returned Graph revision, including an identical position command. The raw
second result is therefore **not** byte-identical to the input. Existing PS1
[`commands::apply`](../../crates/photara-store/src/package/planning/commands.rs)
compares the result with the original revision restored, detects identical
content, and returns false; [`planning::plan`](../../crates/photara-store/src/package/planning/mod.rs)
then returns `PlanOutcome::Unchanged` without producing a candidate or advancing
authored state. Tests assert the raw revision advance, exact normalized canonical
content equality, actual `Unchanged` result and unchanged source file map.
No production semantics were changed to manufacture this result.

For both actual mutation checkpoints, tests compare the real planner's full
AuthoredCoordinate and exact emitted saved-graph/authored canonical bytes with
independent fixed expectations. Every portable receipt's before/after revision
and digest also match the actual `PreparedReceipt` coordinates. Expected graph
semantic, graph payload and saved-envelope digests remain distinct coordinate
fields with the existing PS1 meanings.

Retry looks up the already selected original operation ID before considering
its now-stale expected authored state. The original ID plus identical canonical
semantic-intent digest returns the exact first receipt, including its original
before/after result and historical provenance, after mutation three. Changed
intent under that same ID refuses. A byte fingerprint of all supplied packed
bytes, locator entries, roots and journal records stays unchanged on either
retry path. Portable receipt metadata is not a new live authorization decision
or a qualified Saved acknowledgement.

## Proposed index records and unchanged evidence bytes

Each new storage record has common exact
`schema:{id,version:1}`, `project_id`, and `extensions` fields. These schemas are
separate from the historical empty `example.ps2.operation-index-root` probe;
no old array field silently becomes a root-reference union.

| Proposed schema ID | Exact additional fields |
| --- | --- |
| `example.ps2.operation-index-tree` | `library_id`, `bootstrap_sha256`, `accepted:{through_ordinal:Decimal,prefix_sha256:Digest}`, `by_id:ObjectRef`, `by_ordinal:ObjectRef` |
| `example.ps2.operation-id-leaf` | `count:Decimal`, `entries:[OperationEntry]` strictly sorted/unique by canonical operation UUID |
| `example.ps2.operation-ordinal-leaf` | `count:Decimal`, `entries:[OperationEntry]` in contiguous acceptance ordinals 1..N |

`OperationEntry` retains the earlier appendix's exact fields:
`{operation_id,request_sha256,acceptance_ordinal,receipt:ObjectRef}`. Both indexes
must contain identical entries and point to the same unique original receipt
set. Counts equal the root's accepted coordinate. This initial linked verifier
supports 1–32 operations and leaf indexes; index branch shapes and production
limits remain separate work.

Receipts retain the proposed `photara.package.operation-receipt` v1 shape from
the earlier appendix, including the original credential-free provenance and
before/after authored commitments. Semantic intent hashes retain exactly
`photara.package.operation-intent.v1`, with package/library/bootstrap identity,
operation ID, complete expected coordinate, supported typed Graph envelope,
fixed `updated_at`, null undo group and `single` boundary. That portable intent
hash is distinct from PS1's internal MutationRequest digest; the test compares
the same typed command and expected coordinates without substituting one digest
protocol for the other.

`P0`, accepted links, `J0` and journal links use the existing appendix's exact
domain strings and canonical inputs. The fixed journal projection contains
sequences **3, 7 and 11**, with gaps permitted; those values are never replaced
by ordinals 1, 2 and 3. Every included frame must bind the matching original
receipt ID, request digest, exact receipt digest and receipt journal coordinate.
State inclusion's final authored result comes from the last included receipt's
`after`, not an invented journal field. An authored no-op still has its own
acceptance ordinal and journal frame.

The provenance specimen uses the earlier account-principal/gui-actor/project-
grant shape and action mask 71. It is fixed historical evidence, not a grant or
policy check performed by this pure reader. Replacing an original actor while
rebuilding both active index roots and their accepted prefix still refuses
against the independently retained original journal commitment.

## Linked bytes and retention edges

The [fixed corpus](proposals/ps2/operations/linked-operations.json) contains
35 canonical records, 22 selected packed records and one 18,388-byte data pack.
The recovery prefix ends at byte 11,721. The exact locator leaf points to the
existing proposed `PS2PKD01` 16-byte frame convention. Every locator resolution
checks the body ObjectRef and complete framed PhysicalRef against actual bytes;
semantic frames cannot resolve as padding or metadata. The locator is a supplied
lookup specimen here, not a new selected authority or a completed physical
ownership proof.

The selected records include real authored v2/saved-graph v2 values, their
complete minimal typed dependencies, original receipts and both operation
indexes. The verifier checks their schema-directed edges and exact union across
active/recovery. It does not interpret arbitrary JSON-shaped optional fields as
references. Full real package correctness is independently exercised by the
existing PS1 verifier/planner oracle described above.

Receipt `before.digest` and `after.digest` are commitments, not ObjectRefs.
Tests rebuild an active-only physical pack with the actual required logical
records and no initial or recovery authored/Graph records. Active validation and
original receipt retry still succeed, even though old receipt digests name the
absent states. Separately, recovery validates with all active-only records
removed; active then correctly refuses. These are independent closure tests,
not ancestry walks or retention inferred from a matching digest string.

## Fixed expectations, negative cases and boundaries

The [generator](proposals/ps2/operations/generate.py) uses Python standard-library
JSON sorting/UTF-8, explicit little-endian frame encoding and SHA-256. It does not
invoke Rust or sample runtime output to construct expected Graph/receipt bytes.
The Rust test compares actual Core/PS1 results to committed literal canonical
strings, lengths and hashes, and compares packed frames to literal hex bytes.

Seven tests cover the real command/no-op oracle, exact canonical/frame bytes,
reciprocal typed indexes and independent recovery, immutable retry/provenance,
missing/duplicate ordinals and mismatched ID entries, journal projection/result
corruption, and before/after nonedges. Negative semantic cases rebuild affected
index/ancestor ObjectRefs before verification, so they exercise actual index or
journal invariants rather than only a stale outer checksum. Those fault inputs
are generated in the test; the positive expected bytes remain independent and
fixed. Strict Clippy passes for this standalone target.

```sh
python3 docs/architecture/proposals/ps2/operations/generate.py
cargo test -p photara-store --test scalable_operation_candidate
cargo clippy -p photara-store --test scalable_operation_candidate -- -D warnings
```

Stable links for subsequent **candidate** phase vectors:

| Canonical record | SHA-256 |
| --- | --- |
| First semantic intent | `9532f68d88cb5cbd253f745ba8e25a781f4a1a353dd7e4d96d9d1109339a11d8` |
| Original first receipt | `c81f0b63006489c0b0c0da3c0da7a7f4a652dc7cebf6479815f7f53e9fee0dd0` |
| Active operation root (3) | `61a9b81668079dce05bfdb722f84261373f01dc95897a68a41eb11077dcc97f2` |
| Recovery operation root (2) | `967ffa976b3cff4d4c28fdf5d94dc15436176498c18cdb9daa7b724696a17f42` |

These links do not freeze compatibility. Joint root/ownership/charge publication,
original admission and local birth witnesses, retirement/compact suffixes,
codec-transition retention and maximum later control envelopes remain further
linked vector groups. The native furnace's 12/4-byte frames must not be mistaken
for this proposed 16-byte framing when connecting their evidence.
