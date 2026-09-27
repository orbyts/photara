# Selected evidence validation seam

This is a disposable reader refactor, not new wire or production behavior. It
prepares the [selected origin proposal](PS2_SELECTED_ORIGIN_FIELD_PROPOSAL.md)
without copying the complete settled physical/closure reader. The original
[settled checkpoint](PS2_INTEGRATED_SETTLED_COORDINATE_CANDIDATE.md) remains the
same canonical byte specimen. Its original verification manifest records the
sources at that checkpoint; the [seam verification](verification/ps2-selected-evidence-seam.json)
records the changed sources and new regression results separately.

The ordinary resource and package entry points still require authored origins,
empty retention evidence, the original capability set, and the settled
bootstrap. Their former refusals remain. A separately compiled test policy may
add an exact capability set, validate resource origins, and resolve selected
evidence using authenticated objects. Resource collection, canonical identity,
record/subset validation, and support checks continue through the same bounded
resource adapter.

The package reader supplies actual selected StateRoots, placement entries, pin
records and role proofs to the evidence validator. The returned dependency sets
participate in exact global membership, per-role locator equality, shared-byte
comparison, physical ownership and accounting checks. Policy code is part of the
disposable decoder, not a serialized permission or a user-controlled callback.
No policy is inferred from an extension or from the diagnostic record table.

Full validation and read-only recovery have distinct contexts. Recovery receives
its verified role and the selected root commitments; it cannot claim missing
other roots' complete semantic closure or physical accounting. A subsequent
origin decoder must validate the recovery root's own evidence without treating
its subset as the complete policy union across all roots. The default recovery
proof and its limitations are unchanged.

The independent Python generator gains optional resource/evidence builders and
an explicit additive feature list. Calling it with the original defaults must
reproduce both settled corpora exactly; the factored resource corpus also stays
identical. A custom builder constructs new linked records and does not authorize
rewriting an existing pack, rebinding witnesses, or performing a transition.

The new origin implementation, selected original/phase controls, actual append
execution, newborn enrollment, Blob composition and qualified storage remain
separate work. This seam supplies none of their missing authority or completion
claims.
