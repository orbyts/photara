# LL1 fresh-device terminal receipt retrieval

Status: disposable PostgreSQL candidate evidence, 2026-09-27. No production
route, credential provisioning, schema migration, persistent codec, or live
Library deletion is implemented. This closes the focused **receipt retrieval
versus first execution** fixture gap identified by the
[service authority handoff review](LL1_SERVICE_AUTHORITY_HANDOFF_REVIEW.md).
It does not complete that review's other integration requirements.

## Fixture boundary

Run from the repository root:

```sh
python3 scripts/test_ll1_fresh_device_receipt.py
```

The [script](../../../scripts/test_ll1_fresh_device_receipt.py) composes the
existing protected executor and SQL authority fixtures in a generated private
`/private/tmp/photara-ll1-constraints-*` cluster. It accepts no database URL,
listens only on its own Unix socket, installs all 14 unchanged migrations,
and stops/removes that cluster afterward. The initial catalog contains 59
tables, 114 FKs, 122 triggers, 173 policies, 52 functions, and 45 forced-RLS
tables. Source migration hashes and baseline FK count remain unchanged.

Additional candidate-only relations hold a synthetic immutable original-device
review, a controllable review clock, and immutable receipt bytes captured once
when the executor inserts its terminal receipt. These are explicitly fixture
representations, not a proposed permanent token or receipt encoding. The
original sparse aggregate and narrow protected DELETE guard overlay are
inherited unchanged; this is not a new full-aggregate coverage proof.

The service-only `query_receipt` function authenticates the supplied synthetic
identity/account against active database rows, then locks and checks current
device state and credential commitment/state/revision. It selects terminal
bytes by initiating account/operation and compares the request hash. It does
not check former Library membership, the original device, or review expiry,
and it has no execution or grant call. Another account sees `NotFound` and no
receipt data. A changed hash for an accessible terminal operation refuses.

The separate `execute_review` wrapper first permits authenticated terminal
retrieval; when no terminal receipt exists, it requires the original reviewed
identity/device, exact digest and unexpired review before calling existing
owner admission and the one-use-grant executor. Raw grant/executor entry points
are revoked from the fixture service role so it cannot bypass this wrapper.
An eligible new device therefore cannot execute an original device's pending
review. `NotFound` remains an observation, never a definitive rejection or
permission to dispatch changed bytes.

## Executed evidence

The final run passes **42 cases**:

- A commits the deletion while the simulated client discards the entire reply.
  The harness independently reads the captured terminal bytes. After revoking A
  and suspending its credential, A cannot query or retry; eligible B on the same
  account retrieves exactly those bytes without the deleted membership. A
  different current active identity on that same account also succeeds.
- B cannot first-execute A's uncommitted review. Missing-terminal lookups stay
  `NotFound`; B's first `NotFound` is followed by A's successful commit of that
  same operation. The pending second reviewed operation cannot be
  executed by B before or after expiry/restart.
- Changed hash, substituted identity, another device's secret, a random
  correctly sized secret, stale credential revision, and an active device with
  a suspended credential refuse. A valid original-device review that is
  expired also refuses first execution.
- Ordinary API/control/auth logins with forged owner context cannot query,
  execute, read or write the private receipt table. The service role cannot
  invoke the raw grant/executor or write reviews.
- A trigger deliberately makes **any new grant insertion fail** after the
  committed deletion. B's receipt query and terminal retry still succeed after
  review expiry and, for the query, after a clean PostgreSQL restart.
- Every read/refusal compares exact contents of all baseline and all candidate
  overlay tables before/after. Terminal queries add no deletion, receipt,
  evidence, marker, inventory or request-binding event. The final state has one
  receipt, original-byte record, marker, inventory event and request binding;
  authorization and permit relations are empty. The deletion changes only the
  expected seeded target rows, retaining account/device/unrelated rows.

The private cluster required sandbox escalation because `initdb` could not
create its local shared-memory segment under the sandbox. The authorized rerun
used only the generated cluster above and completed its cleanup.

## Remaining limits

The supplied identity/account still models a trusted authenticated service
caller. No HTTP route, bearer signature/issuer/audience/expiry, Rust OIDC
handoff, or production secret-commitment codec is exercised. The synthetic
clock tests review expiry ordering, not expiry while waiting on a lock.
Credential revocation races and other lifecycle/concurrency states remain
separate work. A `NotFound` response never proves an in-flight request cannot
commit; this fixture does not expose a production `InFlight` protocol.

The inherited private server runs with `-F`; clean restart demonstrates retained
data after clean shutdown, **not** crash/power-loss durability qualification.
Empty grant/permit state here covers these wrapper paths only and does not
resolve the original fixture's separately identified abandoned-grant commit
case. No production deletion, deployment, policy change or `RESTRICT` change
is authorized by this evidence.
