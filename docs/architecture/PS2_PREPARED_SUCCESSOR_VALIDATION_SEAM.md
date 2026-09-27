# Prepared successor validation seam

This disposable reader seam adds an explicit prospective-successor entry point
for the [admission implementation](PS2_SELECTED_TRANSITION_IMPLEMENTATION_PLAN.md).
The ordinary settled and origin entry points retain their previous dispatch and
refusals. No production reader or permanent format changes.

The successor entry first fully verifies the supplied original settled package.
It then checks the actual candidate HEAD and commit, exact parent commitment,
checked revision increment, unchanged bootstrap/capability floor, and fresh
commit/write IDs. It does not normalize candidate controls into an old snapshot.
Existing retained StateRoots must remain byte-identical; a new active StateRoot
must identify the exact original active predecessor and a different root UUID.
Predecessor fields remain provenance, not implicit retention edges.

The caller must supply additional canonical authored records from an actual
Core/PS1 preparation result. This is a trusted **test-code precondition**, not a
permission carried by the package or evidence inferred from its diagnostic rows.
Every selected candidate object still passes authenticated location, canonical
identity, typed closure, ownership, accounting and global-union checks. The
admission harness separately binds the exact prepared request/receipt, original
receipt prefix and deterministic append recipe. This helper alone establishes
neither operation acceptance nor replay authority.

Transition-fixture loading shares the existing physical/input bounds and admits
only explicit admission-only or prospective-sizing fixture tags. Those tags do
not bypass verification and are not proposed package wire fields. The current
successor helper remains scoped to the authored-only Graph specimen; composing
all origin policies and pending evidence is separate work.

The [verification record](verification/ps2-prepared-successor-seam.json) binds the
changed source and default regression. Full admission, publication, measured
finalizer, cleanup, newborn and retirement evidence must be recorded by their
own linked tests. No physical effects, qualified barriers or `Accepted`/`Saved`
claims follow from this refactor.
