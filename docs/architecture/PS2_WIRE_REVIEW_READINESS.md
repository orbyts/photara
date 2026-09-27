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
| Bootstrap and physical closure | [First packed specimen](PS2_SCALABLE_PACKED_BYTE_CANDIDATE.md): unchanged HEAD/outer envelope, explicit feature dispatch, typed semantic/ownership/locator coverage, independent recovery | Compose these selected fields with the changed operation roots and publication phases; a supplied lookup map or separate root-selection projection is insufficient |
| Actual authored operations | [Operation specimen](PS2_SCALABLE_OPERATION_BYTE_CANDIDATE.md): real Core/PS1 mutation, authored no-op, original retry and receipt/journal commitments | [Branch specimen](PS2_SCALABLE_BRANCH_BYTE_CANDIDATE.md) now verifies multi-level indexes; joint physical/authority selection remains |
| Scalable inventory and charge | [Branch specimen](PS2_SCALABLE_BRANCH_BYTE_CANDIDATE.md) proves independent fixed tree bytes, summaries, coverage and checked sums; physical furnace proves attributable charges | Compose actual held allocation contents and current tips with the complete selected package; synthetic sealed observations alone are insufficient |
| Original admission and publication | Witnessed rollover proves original-token birth/payload/finalization; [phase subproof](PS2_CANONICAL_PHASE_SUBPROOF.md) adds canonical controls, actual caps and coherent refusal cases | Select original admission, phase and C in the joined accounting root; external phase records are not authority |
| Retirement and compact replay | [Compact fixture](PS2_COMPACT_RETIREMENT_COMMITMENTS.md): original suffix commitments and post-unlink verification; bounded useful runtime reclamation for both maps | [Phase subproof](PS2_CANONICAL_PHASE_SUBPROOF.md) now checks canonical suffixes, both closures, twelve pins, actual destination bytes and original retry; final joined HEAD-selected transition remains |
| Codec evolution | Earlier fixture documents decode-only compatibility and possible original-ticket hash fencing | [Phase subproof](PS2_CANONICAL_PHASE_SUBPROOF.md) explicitly preserves codec-v1 bytes across two hypothetical reader generations and refuses reserialization/unsupported versions; final proposed permanent coordinate/dispatch set remains |
| Conversion-source retention | [Resource/conversion specimen](PS2_RESOURCE_CONVERSION_BYTE_CANDIDATE.md) validates proposed descriptor bytes, the fifteen-file legacy package, all eighteen snapshot files, nineteen copy cuts and explicit same-attempt registration | Select the conversion descriptor and exact file charges in the joined package; no native conversion or ownership proof is inferred |
| Managed resource records | [Resource/conversion specimen](PS2_RESOURCE_CONVERSION_BYTE_CANDIDATE.md) links additive records, v3 binding and typed negatives, separating structural readability from retention support with no external-media reads | Select ResourceState through the actual joined StateRoot and validate its typed closure; standalone v3 projection and synthetic single-copy qualification retain their stated limits |

The nineteen earlier golden values establish canonical encoding of illustrative
fields. Their placeholder references are not a complete valid closure. Similarly,
a furnace using its own fixture types proves the exercised invariant; it does
not prove a different proposed schema by renaming that type.

Each completed row needs exact source hashes, independently generated expected
bytes, focused positive/negative validation, and an honest statement of limits.
The final review package must identify one concrete set of candidate schema IDs,
field layouts, digest domains, frame tags and supported dispatch combinations.
Separate specimens must not imply incompatible layouts are interchangeable.

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
