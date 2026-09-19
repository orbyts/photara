# Active handoff

This is the sole current operational handoff. Historical handoffs and detailed
experiment reports are reference material only.

## Baseline

- Authoritative branch: `main`.
- Production-code baseline before this documentation-only consolidation:
  `f91f8de1546896267bac13adeac00c1659133232`.
- The current `main` commit containing this file is the handoff publication;
  resolve it with `git rev-parse HEAD` and require it to match `origin/main`
  before continuing.
- The working tree must be clean. Do not modify the unrelated dirty
  `codex/promote-graph-lab` worktree.
- Preserve stash `ce39772a4008c886265ac9a25485b2970d0f8332` until the post-LL2a
  audit. Do not apply or drop it during PS2–LL2a work.

## Critical path and active gate

The delivery path is:

**PS2 production durability → PS3 one-project session/autosave → PS4 project
browse/reopen/switch → LL2a signed cross-Library acceptance.**

PS2 is active. LL1 contract/schema/security review is advanced parallel work,
but it cannot bypass PS2, PS3 or PS4 and does not authorize LL2 production
mutation.

The immediate PS2 engineering step is to integrate, in disposable fixtures:

1. automatically generated exact semantic and ownership closure for active
   prefixes, locator metadata and control generations;
2. original-token coverage, exact allocation charges and post-retirement
   project-ledger credits without speculative free-space credit;
3. genuine Core Graph commands, typed journal groups and original receipts;
4. the actual-v3 packed placement, active/recovery/all-pin liveness and
   qualified barrier path;
5. the provisional packed B-tree lead against the radix comparator.

Only consolidated evidence from that integration may support review/freeze of
the permanent wire and implementation of the shared Rust reader, writer,
recovery and admission boundary. macOS storage/barrier qualification remains a
separate PS2 prerequisite.

## Approved semantics — do not reopen without contradictory evidence

- Normal authoring uses continuous autosave; users do not need to save
  explicitly. `Command-S` may remain **Save Now**.
- Speculative or in-memory state is not committed. `Accepted` requires the
  exact journal group to pass its qualified durability barrier. `Saved`
  requires the matching selected package checkpoint/`HEAD.json` and original
  receipt to be durably verified for the current accepted authored revision.
- `HEAD.json` is the sole authoritative atomic selector. Active and recovery
  roots are independently verifiable. Structural opening is distinct from a
  full integrity audit; an imported, unaudited package may be inspected
  read-only but is not admitted for writing.
- The `.photara` package owns authored Graph state. Cloud is authoritative for
  account, Library membership and catalog coordination; local SQL is a
  projection/session/cache. Device-only observations and paths are not portable
  package state.
- Resource identity is separate from physical backing and placement. Ordinary
  open, autosave, validation and root turnover must not read or strongly hash
  large external media. Strong media hashing occurs only at explicit capture or
  verification boundaries.
- GUI, CLI/headless, scripts and future agents are peer Photara-controlled
  writers using the shared Rust admission, lease, journal, dedupe, publication
  and recovery protocol. The GUI is not a privileged writer.
- User-selected project locations are permitted only through qualified storage
  profiles; a Photara-managed project root is not mandatory. Storage roles use
  the existing logical location/host-binding architecture.
- Keep native host UI as the default: SwiftUI/AppKit on macOS and native Windows
  UI later. Custom treatment is an explicit exception.
- Photara remains the current identity. BR0 made branding, extension and roots
  configuration-driven, but a real rename still requires a coordinated trust,
  directory and compatibility cutover.
- Keep the existing Fly.io resources available for the controlled remote
  vertical slice. Do not deprovision them in this sequence.

## Proven or implemented

### PS2 disposable evidence

- Bounded packs and HEAD-selected locator roots avoid lifetime tail copying.
- Active/recovery selection, twelve pin classes, bounded interruption recovery,
  conservative capacity admission and exact old/candidate reconciliation have
  disposable real-file evidence.
- The packed physical comparison provisionally favors B-tree plus ordinal
  sequence; radix remains the required comparator. Neither is frozen wire.
- Genuine Graph commands, typed intents/receipts, finite journal groups,
  no-op/inverse behavior and old-operation lookup have separate fixture proof.
- Exact semantic/ownership closures within one selected locator, bounded typed
  recipe replay, active-prefix ownership and post-retirement project-charge
  release have separate prerequisite fixtures.
- The local macOS syscall/fault matrix provides useful evidence, but remount,
  abrupt-power, provider-path and production `Saved` qualification are absent.

The current synthesis and exact measurements are in
[PS2 consolidated engineering evidence](architecture/PS2_CONSOLIDATED_ENGINEERING_EVIDENCE.md).

### LL1 parallel evidence

- Keep the existing `RESTRICT` constraints. Disposable PostgreSQL and SQLite
  executors implement the same protected deletion semantics with engine-specific
  transaction strategies; current evidence does not justify `NO ACTION`.
- The service-authority model uses a dedicated authority boundary and exact,
  one-use transaction/backend-bound grants. Caller-set owner context alone is
  not authority.
- PostgreSQL dense coverage includes all 48 reviewed owned tables. SQLite
  coverage populates all 79 source tables, retires 73 target tables, retains six
  designated tables and publishes detached evidence in the fixture.

The current entry is
[LL1 protected executor contract](architecture/verification/LL1_PROTECTED_EXECUTOR_CONTRACT.md),
with focused authorization, coverage and service-handoff links. These are
disposable proofs, not production deletion.

## Unimplemented or unproven

- No permanent PS2 wire, production package reader/writer, live conversion,
  migration, production GC, Asset Store or production `Accepted`/`Saved` path.
- No integrated ownership/refundable-capacity/Graph/journal/qualified-barrier
  implementation. The current high-water model is safe but can over-refuse.
- No shared Rust production session coordinator, writer queue or native
  autosave status/recovery path (PS3).
- No safe real project browse/reopen/switch flow on that durability boundary
  (PS4).
- No production Library create/select/rename wiring or signed cross-Library
  acceptance (LL2a). Library removal is LL2b and remains later.
- LL1 Rust/OIDC authority handoff, canonical evidence/replay codecs, remaining
  lifecycle/concurrency states and production implementation remain absent.
- No production deletion, live-data migration, deployment or public launch.

## Dependency order and human acceptance

1. Complete PS2 integration evidence, storage qualification, wire review/freeze
   and shared Rust reader/writer/recovery/admission.
2. PS3: one-project production session and autosave with truthful
   `Saving…`/`Saved`/failure state, final flush and crash/relaunch recovery.
3. PS4: minimal native project browser, reopen and safe switch; flush and verify
   the current project before target activation and restore it on target failure.
4. Finish the parallel LL1 review/security prerequisites needed by LL2.
5. LL2a: wire Library create/select/rename and run the signed, real
   two-Library/two-project acceptance on `main`.

LL2a is the next major Suhail test. In the signed app, he must be able to make
ordinary authored edits without manually saving, observe truthful saving state,
quit/relaunch and recover the exact state, browse/reopen/safely switch projects,
switch through the intended two-Library/two-project matrix, and verify each
project restores correctly. Preserve unknown/failure context. This consolidates
the existing PS3, PS4 and LL2a roadmap criteria; it does not add Library removal,
two projects per Library, second-Mac acceptance or final Gallery polish.

After LL2a passes, audit the preserved stash against validated `main` and ask
Suhail before dropping it. The broader platform-ready vertical slice remains a
later gate before Layout becomes the first product node.

## Parallel work

- LL1 contract/schema/authorization review may proceed without production DDL,
  deletion or live data.
- A minimal native project-browser presentation fixture may proceed, but must
  not simulate safe switching before PS3/PS4 durability exists.
- Final Gallery aesthetics, branding, website, store, LLC and public launch are
  later work and do not change this critical path.

## Prohibited claims and actions

- Do not call disposable fixtures production, a permanent wire, a qualified
  storage profile, or a user-visible `Saved` guarantee.
- Do not patch Swift `Browse Projects`/`closeProject()` into apparent safe
  switching before the Rust session/durability boundary.
- Do not perform live package writes/conversion, numbered migration, production
  Library deletion, GC/retirement, deployment, live-data mutation or Fly.io
  deprovisioning without the later gate.
- Do not change `RESTRICT` to `NO ACTION` unless new full-schema evidence shows
  an independent necessity.
- Do not force-push, discard stashes, or modify unrelated worktrees.

## Stop conditions

Iterate autonomously when a disposable implementation is slow, fails, or needs
another engineering design within the approved semantics. Stop and ask Suhail
before changing an approved durability/`Saved`, compatibility/wire, resource
retention/custody, security/authorization, destructive file/data, or visible
lifecycle policy; before freezing a permanent format; or before production/live
migration, deployment or deletion. Also stop if evidence requires a schema or
constraint change, or if LL2a acceptance would materially differ from the
roadmap definition above.

Use an Astra task for major architecture, production-code or broad repository
changes. Keep routine bounded work moving without asking for approval unless a
stop condition above is reached.

## Minimal reading order

1. This file.
2. [Root roadmap — delivery path only](../ROADMAP.md#delivery-path-cross-library-acceptance-then-platform-ready-vertical-slice--2026-09-17).
3. [Project session and durability](architecture/PROJECT_SESSION_DURABILITY.md).
4. [PS2 production codec/publication contract](architecture/PS2_PRODUCTION_CODEC_AND_PUBLICATION_CONTRACT.md).
5. [PS2 consolidated engineering evidence](architecture/PS2_CONSOLIDATED_ENGINEERING_EVIDENCE.md).

For parallel LL1 work only, additionally read
[Library lifecycle](architecture/LIBRARY_LIFECYCLE.md),
[LL1 typed contract/schema delta](architecture/LL1_TYPED_CONTRACT_AND_SCHEMA_DELTA.md)
and the
[current protected-executor evidence entry](architecture/verification/LL1_PROTECTED_EXECUTOR_CONTRACT.md).
Follow their focused links only as needed.
