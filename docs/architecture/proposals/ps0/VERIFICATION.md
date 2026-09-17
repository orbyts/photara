# PS0 acceptance furnace and review evidence

The following BR0 and PS1–PS4 tests are required designs, **not claims of completed tests**.
All persistence fixtures use disposable roots, synthetic projects and fake catalog/
runtime adapters. No production app launch, service, live catalog, user package,
source photograph, Keychain or network account is a test fixture.

## Multi-surface authority — required, not yet implemented

Use a deterministic fixture with two logical clients before selecting transport:

- GUI and agent submit against one exact coordinate: one ordered success, one
  explicit stale conflict; retry the successful operation after reconnect and owner
  turnover with the original ID/intent and no duplicate effect. Different intent
  under that ID refuses. Simulate an acknowledged append with a lost response.
- Reject forged actor/grant/scope, expired or revoked admission and unauthorized
  receipt lookup. Keep original accepted provenance through recovery/compaction;
  assert no credentials, bookmarks or prompts enter mutation evidence.
- Flush a fixed sequence while the other client continues; prove finite-prefix
  inclusion without falsely labelling newer state Saved. Detach/switch one client
  without cancelling another's work; stale owner/attachment events cannot apply.
- Direct second-process acquisition while a GUI or headless owner holds the stable
  lifetime lease returns WriterBusy. Test owner death and exact recovery before new
  admission. This proves safety only, not routing, handoff or fairness/liveness.
- Keep undo groups client/principal-bound, reject implicit cross-client undo, and
  preserve exact IDs/dedupe at sealed-root turnover. Voice/chat proposals get no
  alternate authority or bypass; planning and approval UX remain a later iteration.

These are new acceptance obligations, not executable fixtures or completed evidence.
Actual routing/handoff and qualified-storage failure tests remain required before
promising multi-process progress or production autosave.

### Completed bounded logical-client evidence

The [authority fixture](../../../../crates/photara-store/tests/package_planning/authority.rs)
now exercises GUI-like and agent-like attachments to one in-memory Rust ordering
model. It uses actual PS1 RenameGraph planning and independently re-verifies virtual
candidate bytes. Tests cover exact stale-coordinate rejection, original-operation
dedupe before stale checks, different-intent refusal, ordered observations, reconnect
gap/snapshot and future-cursor refusal, a finite checkpoint prefix while later edits
exist, and detach without discarding another client's accepted work. Private
synthetic host grants enforce principal, grantor, package/Graph scope, generation
and revocation; original credential-free acceptance provenance survives grant renewal
and revocation. A second logical owner receives WriterBusy; a stale owner token
cannot submit.

This model has no durable append, production receipt, authorization service, IPC,
real OS contention, owner-turnover recovery or coordinator implementation. Its
two-event observation window is a fixture input, not a production retention limit;
accepted evidence/dedupe remains intact. Other journal/process furnaces remain
separate evidence and are not implicitly composed into end-to-end durability by
these two model tests. The remaining obligations above stay open.

## BR0 acceptance — required before PS1

These tests are proposed, not run in PS0. Build a disposable release using a synthetic
product name (for example `Juniper Studio`) and synthetic extension (`jprtest`) from
the descriptor/generated typed configuration, with no production trust enrollment.

- Audit all public coordinates: display/short/product/executable names, generated
  bundle metadata, document UTI/display type/current extension/read aliases, callback
  URL and scheme, audience, Keychain service, user agent, support/cache/journal roots,
  service/public URLs and defaults. Validate generation consistency across Swift,
  Rust, bundle and service artifacts, including fields absent from today's descriptor.
- Create a synthetic package with the configured suffix, explicitly save, quit and
  reopen it through the existing bounded package lifecycle. Verify byte manifests,
  stable identities and recovery locators. BR0 must not require PS1's future writer:
  for UI1's read-only route, save verifies the initially durable package; exercise
  mutation persistence with a disposable contract harness, never claim autosave.
  Repeat the full edit/save/quit/reopen scenario once PS1–PS3 exist as a regression gate.
- Open a development-era `.photara` fixture through the reviewed alias; opening
  changes no bytes or filename. Verify new publications use only `jprtest`, and an
  explicitly approved extension-only rename changes no internal byte or revision.
  Refuse legacy write admission until the filename cutover is safely reconciled.
- Compare pre/post cutover HEAD, manifest, objects, format/node/value/schema IDs,
  operation IDs and migration history. Existing reader accepts unchanged bytes;
  deployed PostgreSQL namespace expectations remain unchanged. No mass substitution.
- Inspect generated UI, alerts, menus, defaults and public errors for unintended
  user-visible `Photara`. Allowlist only intentional legacy-alias explanations or
  technical compatibility diagnostics; source-symbol occurrences do not prove a leak.
- Verify isolated callback/trust fixtures and mocked Keychain service addressing;
  separately review production Auth0/Apple/service enrollment and required sign-in.
  Test controlled directory cutover interruption, collision, permissions, rollback
  and UUID/dedupe continuity in disposable roots. Never test against live credentials.

The known source leaks in the architecture document are required BR0 inputs, not
an exhaustive clean bill of health. BR0 acceptance requires evidence for each row;
synthetic configuration does not authorize the later production security cutover.

## Deterministic furnace matrix

Use a seed and stable IDs/timestamps; record codec/build/profile versions, injected
boundary and expected outcome. Each attempt emits before/after manifests containing
relative path, file type, identity, mode, length and SHA-256; independent current
reader validates surviving HEAD. Keep manifests for journal and package separately.

| Boundary / scenario | Required oracle |
| --- | --- |
| Each journal header/frame write, short write, sync and rotation/index replace | Kill before/after each step; acknowledged prefix survives; uncertain append reconciles same ID; torn tail preserved; never skip interior corruption |
| Every object create/write/full-sync/no-replace/directory-sync | HEAD remains old or references complete valid closure; retry verifies existing hash bytes; no unaccounted temporary/orphan inode |
| Commit create/write/full-sync/publish/parent-sync | Old HEAD valid until replacement; intent identifies every unreachable published object/commit |
| HEAD temp creation/write/full-sync, CAS check, atomic replace, root-sync | Old or new complete HEAD; unknown result is never Saved; changed HEAD cannot be overwritten by cooperating writer |
| Post-publish reader validation and local receipt write/sync | Reconcile commit ID/write ID/digest exactly; no duplicate commit or duplicate mutation |
| Journal compaction and deletion | Old or new complete recovery base/index; undo/dedupe survive; uncheckpointed records never deleted |
| Death after durable append before UI receipt | Retry returns original result, not a second edit |
| Duplicate operation with altered payload | Typed conflict, no bytes changed |
| HEAD ahead/equal/behind/divergent, same revision different digest | Only proven inclusion trims replay; unrelated changes freeze |
| Two processes, stale ownership metadata, lock-file substitution | Exactly one cooperating writer; no timed lock stealing; changed inode rejects |
| Symlink/hardlink/reparse, FIFO/device, root/parent swap, case alias | Refuse before unsafe write; sibling/outside byte manifests identical |
| Disk full, permission loss, volume disconnect at every step | Current retained; journal/intent preserved; repeat retry after identity check |
| Unknown schemas/fields, huge/deep JSON, overflow, history limits | Refuse unsupported edits; opaque bytes preserved; no unreadable HEAD published |
| Long gesture, end, Escape/cancel, parameter debounce/focus loss | Bounded durable checkpoints, one undo group; cancellation compensates already durable checkpoints with a new transaction, never erases them |
| Undo/redo across checkpoint/restart, edit after undo | Exact revisions/digests; redo invalidation durable; no runtime effects replayed |
| Independent verifier and byte reachability | Every HEAD reference exists and hashes correctly; reachable/retained immutable bytes unchanged; all extras accounted by intent or explicit retained diagnostic |

Enumerate every IO adapter call with a fault ID and test both pre-effect failure
and post-effect unknown result. Exhaust small state spaces, then seeded long command
sequences. Process-kill tests are additional to in-process injection; neither proves
physical power-loss safety. Qualification requires controlled disposable-volume
power-loss/remount tests or supported platform guarantees with reviewed evidence.
No NAS support inferred from local tests. For SMB/NAS/provider folders, initial
acceptance is a deterministic read-only refusal with zero package writes. Future
qualification includes server/client crash, dropped acknowledgements, partition,
lease loss, reconnect, third-party edits and sync conflict replicas.

## Switch and native acceptance

Test every switch state from confirmation through active-pointer publication:
Cancel; same-project activation; double activation; stale confirmation; pending
text/gesture; queued Save Now; active run; stop refused/unknown/late callback;
flush/journal/checkpoint/digest failure; target missing/corrupt/wrong IDs/unsupported;
view-state fallback; required local-state write failure; current reacquisition
failure; process death at each state. Assert visible title/Graph/selection remain
current until target is fully ready. Failures retain current editable session or
explicit read-only recovery snapshot. Never empty content, wrong-project callbacks,
silently dropped work or two writer leases. Persisted active pointer must match the
activation receipt after restart. Close, URL open, recent open, creation completion,
window close, sleep/wake and termination all pass the same barriers.

PS3 synthetic native lab uses built-in alert/sheet and status semantics. Capture
Saving…, Saved, Save Failed, Switching, active-run and target-open failure in Light/
Dark, narrow/standard sizes and long project names. Test mouse double click vs
single click, Return/Space activation, Cmd-S, Escape, focus loss, native command
enabling and focus restoration. Accessibility inspection checks names/roles/status;
human VoiceOver speech/navigation remains a manual gate, never inferred from AX
metadata. No Gallery or Graph redesign is needed for this lab.

## PS0 evidence (current checkpoint)

Baseline inspection: `git status --short` empty; HEAD and local `origin/main` both
`114ce43af1089c8bac7fa5c3010a440521027571`. This is local-ref verification, not a
claim of a fresh remote fetch. No applicable AGENTS.md was present in the worktree.
Source audit covered Core commands/canonical bytes, legacy bridge/store, package
1.1 reader/creation, Swift model/open/close/save/lifecycle and local-state boundary.

| Check | Result |
| --- | --- |
| `CARGO_TARGET_DIR=/private/tmp/photara-ps0-target cargo test --locked --offline -p photara-core -p photara-store` | Exit 0; 174 passed, 0 failed, 3 explicitly ignored fixture generators; includes current package creation/validation/compatibility tests using disposable fixtures |
| `CARGO_TARGET_DIR=/private/tmp/photara-ps0-target cargo check --locked --offline -p photara-bridge` | Exit 0; existing bridge and dependencies compile |
| `git diff --check` | Pass |
| PS0 relative Markdown links | All resolve |
| Changed-file inventory | Exactly the five Markdown files below; no production source or migration changes |

Raw local evidence (ephemeral, not repository artifacts):

- `/private/tmp/photara-ps0-contract-tests.log` — SHA-256 `c52bec66c6258a113303da84d7f165db26120293a0cd132a355b80d63485768b`.
- `/private/tmp/photara-ps0-bridge-check.log` — SHA-256 `68cfa6dd7f2672e683487003654a70b23a33ecec7c1f2d10bd489e270f234f69`.

Exact changed-file inventory:

1. `ROADMAP.md` — PS0 ordering/review gate; preserves historical LL0 acceptance.
2. `docs/ACTIVE_HANDOFF.md` — current PS0 handoff and no-commit boundary.
3. `docs/architecture/PROJECT_SESSION_DURABILITY.md` — audit and bounded architecture.
4. `docs/architecture/proposals/ps0/CONTRACTS.md` — review-only Rust signatures and unnumbered logical schema deltas.
5. `docs/architecture/proposals/ps0/VERIFICATION.md` — furnace design and this evidence.

No native lab was added: this architecture review does not require new UI code.
Native interaction/VoiceOver, new journal power-loss behavior, NAS qualification
and production autosave are unverified future gates. Existing test success is not
evidence that the proposed writer exists. No commit/push, app install/launch,
service deploy, actual Project switch, live DB/Keychain/account access, package
conversion or user-data mutation was performed. All tracked files outside this
five-file inventory remain identical to the baseline. Stop here for Suhail review.

### Documentation-only BR0 amendment checks

Reran relative-link resolution, exact five-file inventory/empty staging checks,
Markdown fence/trailing-whitespace checks and `git diff --check`: all pass. The
source-only leak audit confirms CreateProjectView destination/extension, Rust name
length, Application Support fallback and UI defaults remain BR0 work. Production
code, configuration and migrations are unchanged. Earlier Core/store/bridge results
above remain the PS0 baseline evidence; no new implementation tests or BR0 acceptance
runs are claimed for this documentation amendment. Exactly five Markdown files
were preserved; no additional artifact or inventory exception was necessary.
