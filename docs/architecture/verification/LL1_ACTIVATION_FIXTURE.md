# LL1 / PS3 / PS4 disposable activation evidence

Date: 2026-09-18. **Synthetic model evidence only; no production acceptance.**
Implements the nine case groups in the
[compatibility review](../LL1_PS3_PS4_COMPATIBILITY_REVIEW.md) without changing
its architecture/schema gates or the accepted PS0 contract.

Files: [tests](../../../crates/photara-store/tests/ll1_activation.rs) and
[injected model](../../../crates/photara-store/tests/ll1_activation/model.rs).
Only these new test files and this report belong to this checkpoint. No production
module, manifest, migration, PS2 fixture, UI, live data or package was changed.

## Results

12 tests passed, 0 failed. Targeted Clippy with warnings denied and targeted
rustfmt check passed. Relative report links and whitespace checks passed.

| Case group | Observed test coverage |
| --- | --- |
| Supersession | Discovery replaces earlier request; preparation serializes the slot. Cancel settles before new preparation; old operation callbacks refuse. |
| Save evidence | 17 mutations of source owner/identity/incarnation/bootstrap/capability/prefix/authored/Graph/HEAD coordinates refuse both pre-dialog preparation and frozen detach. Missing ledger object refuses despite supplied receipt. |
| Two barriers | Agent advances after confirmation and after captured frozen prefix. Stale dialog-wide claim becomes false; old pre-dialog receipt cannot satisfy new barrier. Exact finite frozen prefix permits progress while later agent work remains. |
| Library-only target | Source barrier remains mandatory; unknown save leaves old generation. Same-Library selection preserves active Project as no-op. Explicit absence of a source Project needs no fabricated save/capsule. |
| Slot and scopes | Two coordinators sharing one injected slot cannot both prepare. Winning scope/Library/Project/Graph/view and receipt publish together; old scoped preference remains; losing stale source evidence refuses. |
| Capsule cuts | Capsule-write failure prevents detach/publication. Precommit recovery retains source/read-only state. Capsule survives receipt commit and failed target establishment, then releases only after successful establishment. |
| Publication cuts | Six target identity/view/evidence substitutions refuse. Pointer failure preserves old pointer; lost reply recovers original receipt. Changed request under same operation ID refuses before another side effect; committed exact retry requires lookup. |
| Owner/attachment isolation | Requested stop is not terminal. Owner, attachment, authorization generation or grant changes prevent publication and editable rollback. Agent-run flag and owner-exit counter remain unchanged. |
| Removal | Source/target removal before commit invalidates intent; removal after commit clears/fences slot while retaining historical receipt/capsule. Removal adds no fake file effects. Unrelated removal preserves visible/editable slot; removed target cannot be newly prepared. |

## Reproduce

Run from repository root. The isolated disposable target directory avoids the
shared cache's observed lock-write permission failure; it is build output only.

```sh
CARGO_TARGET_DIR=/private/tmp/photara-ll1-activation-target cargo test -p photara-store --test ll1_activation --locked --offline
CARGO_TARGET_DIR=/private/tmp/photara-ll1-activation-target cargo clippy -p photara-store --test ll1_activation --locked --offline -- -D warnings
rustfmt --edition 2024 --check crates/photara-store/tests/ll1_activation.rs crates/photara-store/tests/ll1_activation/model.rs
git diff --check
```

## Limits and remaining gates

- Uses existing Core Library/Project/Graph/Commit/Operation ID types, not a new
  wire. Other counters, digests and evidence are fixture-only structures. Digests
  are fixed test bytes compared against injected trusted ledgers, not cryptographic
  verification of real packages. One Graph per sample does not prove multi-Graph
  closure, full versioned codec bounds or capability qualification.
- A shared `Rc<RefCell<_>>` record store models one atomic local slot publication.
  Restart is a query over retained memory, not a process restart, SQL transaction,
  disk flush, actual capsule reconstruction or durable recovery. No database
  schema/key placement is accepted; scoped operation-key/actor policy and multiple
  windows/slots remain outside the model.
- Auth, leases, run settlement, pending-input drain, view restoration and target
  evidence are injected. The PS3 queue fixture is not linked into this model;
  these tests do not prove real GUI/headless/agent concurrency, native confirmation
  text, transport, authorization, journal ordering or production detach behavior.
- File effects are an in-memory call ledger. Zero removal-origin calls is a model
  property, not a sandbox or filesystem interception test. Capsule lifecycle outside
  the tested successful-establishment/removal cases, cleanup of cancelled intents,
  missing/corrupt capsule files and real reacquisition still need acceptance.
- The conservative supersession cutoff, complete barrier/target mapping, GUI slot
  arbitration and capsule lifetime still require cross-slice review. LL2a physical
  schema/privileges, PS3 durability, PS4 native integration and rollout remain gated.

No DDL, live data, commit or push was performed for this fixture.
