# PS1 retention proposal — review gate, not implemented

PS1 preserves the complete ancestry required by the current package 1.1 reader.
A synthetic 1,024-commit package validates; a plan for commit 1,025 returns a
resource-limit failure and leaves every input byte intact. Raising caller limits
does not raise this planner's default reader ceilings. This is **not an indefinite
autosave policy**. No history truncation, compaction, garbage collection, automatic
conversion or new format feature is implemented.

## Proposed decision for PS2 review

Keep all current 1.1 packages read-only in the native application until the
retention and reader-compatibility decision is accepted. The disposable PS2 furnace
may exercise bounded checkpointing and exhaustion, but must freeze admission before
any package or journal budget is exceeded. A resource failure retains the journal,
last verified HEAD and unsaved draft; it must never be reported as Saved.

For production autosave, propose a separately versioned checkpoint-root protocol,
not deletion of ancestors behind the existing reader. A future reader would validate
a sealed checkpoint root, its complete authored/history object closure, and an
explicit predecessor commitment. Older readers must refuse its required feature
before treating a cut chain as complete. A new format/minimum-reader contract,
explicit migration consent, recovery binding and compatibility fixtures are required.
This document does not reserve a version number or authorize that migration.

A compacted root must preserve every object still reachable from authored state and
retained history, including execution source-authored/Graph snapshots, immutable
resource versions, evidence and opaque extensions. An ancestor pointer alone is not
an archive. A proposal for separately retained history must specify where exact
bytes reside, how availability and integrity are verified, and which semantics can
be discarded with explicit policy. Local operation-ID dedupe and journal recovery
must survive independently of user-visible undo retention.

Capacity should be enforced in bytes and reachable objects as well as commit count.
PS2 should measure realistic edit workloads in disposable fixtures before selecting
compaction thresholds, retained-history/undo budgets or an admission reserve. An
arbitrary timer-based autosave rate cannot make a 1,024-commit chain sustainable.
A policy that merely stops indefinitely at the ceiling does not pass the production
autosave gate. Retaining the existing explicit-save/read-only routes is the fallback
until a usable bounded policy is approved.

## Required approval evidence

- Specify root/ancestry semantics, required feature, minimum reader, exact canonical
  bytes and reference traversal. Demonstrate current readers refuse new semantics.
- Prove all retained authored/history/source/evidence and opaque bytes are accounted
  for; define approved pruning semantics rather than inferring ownership from paths.
- Pin old HEAD, journal inclusion, candidate closure and dedupe index. Inject death
  before/after every publish, directory flush, receipt and compaction-index barrier.
- Preserve a verified old-or-new recovery base through unknown outcomes. Remove
  only registered obsolete objects after durable publication and local receipts;
  interrupted cleanup must be resumable and never authorize recursive deletion.
- Test exhaustion, rollback, downgrade refusal, conflicting roots, move/rebind and
  interrupted opt-in migration. No live package is a fixture.
- Review measured budgets and backpressure behavior, including user-visible failure
  and recovery. PS3 cannot advertise automatic durable saving until this review and
  the separate storage qualification/journal furnace pass.

The present slice deliberately makes no production retention choice. Review this
proposal alongside [PS1 evidence](PS1_PURE_PACKAGE_PLANNER.md) and the normative
[PS0 architecture](PROJECT_SESSION_DURABILITY.md).
