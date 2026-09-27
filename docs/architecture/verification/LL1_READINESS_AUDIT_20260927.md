# LL1 readiness after signed authority composition

Status: **read-only source/evidence audit, 2026-09-27**, against checkpoint
`4543a16588dac603f4444099d15f322da470d1e2`. No tests rerun, production changes,
DDL, approval or acceptance are supplied by this document.

LL1 has enough disposable evidence to review the candidate authority handoff;
it is not approved for production. The immediate LL2a prerequisites are the
reviewed create/select/rename contract and its PS3/PS4 activation mapping, followed
by the real durability/session implementation and signed acceptance. Additional
removal failure experiments are not automatically prerequisites to that first
acceptance. The [delivery roadmap](../../../ROADMAP.md#delivery-path-cross-library-acceptance-then-platform-ready-vertical-slice--2026-09-17)
explicitly separates LL2a from guarded removal in LL2b.

## What the evidence now establishes

The [signed adapter](LL1_HTTP_SQL_ADAPTER.md), [lifecycle fixture](LL1_HTTP_SQL_LIFECYCLE.md)
and [COMMIT relay](LL1_COMMIT_ACK_RELAY.md) compose actual HTTP OIDC/device checks
with candidate SQL authority in the same transaction. They test exact credential
revision/review binding, post-lock expiry and revocation, ordinary-role denials,
rollback after actual backend/lock completion, and original receipt retrieval by
an eligible replacement device without first-execution authority. The relay proves
two different server outcomes behind a client commit error; it does not classify
an arbitrary real error as rollback.

The [optional commit-guard composition](LL1_HTTP_COMMIT_GUARD_COMPOSITION.md)
passes those three modes with and without the guard, preserving baseline catalog
and privilege observations and leaving no committed work rows. Its
[recorded outputs and hashes](ll1-http-commit-guard-composition-20260927.json)
are the latest combined evidence. The [Rust test router](../../../crates/photara-service/src/http_ll1_sql_tests.rs)
and [private runner](../../../scripts/test_ll1_http_sql_adapter.py) remain test-only.
The [standalone guard](LL1_AUTHORITY_COMMIT_GUARD.md) separately tests mint-only
commit refusal; the historical raw fixture's committed orphan remains valid
counterexample evidence without that overlay. No production constraint mechanism
has been selected.

The earlier [activation fixture](LL1_ACTIVATION_FIXTURE.md) already covers all
nine proposed compatibility case groups in twelve synthetic tests. Repeating
those cases in another memory model would not establish SQL publication,
process restart, real SavedReceipt verification or native switching.

## Remaining gates, in delivery order

| Gate | Concrete missing result | Scope and authority |
| --- | --- | --- |
| LL1 review for LL2a | Resolve the [activation compatibility review](../LL1_PS3_PS4_COMPATIBILITY_REVIEW.md#decision-gates): one GUI-slot authority across principal scopes, Graph/view binding, complete distinct source SavedReceipt and target evidence, capsule ownership/lifetime and generation checks. Review the corresponding unnumbered physical fields; the memory fixture chooses no durable schema. | Cross-slice contract/schema review. Preserve accepted PS0 behavior; a different supersession or pointer/capsule policy needs explicit approval. |
| LL2a command readiness | Review create entitlement at service dispatch, owner/controller rename CAS, immutable operation/request/original receipt, name bounds, cloud/local routing and unchanged default/selection behavior in the [typed command table](../LL1_TYPED_CONTRACT_AND_SCHEMA_DELTA.md#proposed-commands-results-and-durable-state). Select the reviewed canonical codecs, physical deltas and current floors before implementation. | Existing LL0 behavior; exact wire/schema approval remains required. No evidence here substitutes a deletion review token for create/rename authorization. |
| Real cross-Library acceptance | Complete PS2 production durability, PS3 truthful autosave/session recovery, PS4 validated target restoration and safe switch; then wire create/select/rename through that coordinator and run the specified signed two-Library/two-project test. | Critical path, not another LL1 deletion fixture. No direct Swift pointer shortcut or expanded acceptance matrix. |
| LL2b protected deletion | Finish reviewed evidence allowlist/original authentication-byte verification; typed logical closure and unknown-codec refusal; participating writer lock/generation coverage; inventory privacy, paging and returning-device replay; complete removal/session/file-isolation and upgrade/floor tests. Dense table population and sparse signed execution do not prove every state or codec. | Concrete engineering under the [LL1 acceptance checklist](../LL1_TYPED_CONTRACT_AND_SCHEMA_DELTA.md#review-and-acceptance-checklist), after the applicable schema/security review and disposable implementation authorization. Keep RESTRICT; the earlier proposed FK changes are not current necessity. |
| Production authority and rollout | Review isolated authority credential provisioning/rotation/custody, ordinary-pool inheritance and readiness checks, endpoint exposure and operational recovery; choose reviewed physical guards and deploy all participating writers under compatible floors. | Explicit security/production boundary in the [handoff proposal](LL1_SERVICE_AUTHORITY_HANDOFF_REVIEW.md#proposed-minimum-production-handoff). Private roles, synthetic JWKS/clocks, Unix relay and fsync-disabled clusters cannot establish this. |

[LL1 R1–R5 remain review recommendations](../LL1_TYPED_CONTRACT_AND_SCHEMA_DELTA.md#concrete-recommendations-for-the-five-ll1-decisions),
not accepted DDL. The [physical review](../LL1_PHYSICAL_SCHEMA_AND_PRIVILEGE_REVIEW.md#compatibility-and-acceptance-gates)
is likewise not permission to migrate. The later RESTRICT-preserving executor
proofs supersede its earlier unproven need for current-name FK changes; they do
not silently approve the remaining schema, security or privacy choices.

The current R4 wording already restricts supersession to discovery before
preparation/confirmation and serializes afterward, matching accepted PS0. The
historical compatibility report's statement that R4 permits replacement until
freeze is stale. The remaining gate is the physical evidence/slot mapping, not
permission to weaken the already aligned serialization rule.

## Smallest next authorized step

The subsequent [LL2a review table](LL2A_FIELD_AUTHORITY_REVIEW.md) maps the existing typed create/select/
rename and activation fields to their authority, durable owner, original-ID retry,
and PS3/PS4 evidence dependency. It makes the remaining R4 physical mapping
questions explicit for review without choosing a permanent codec, new slot policy
or production pool.
This is contract readiness within the currently authorized parallel work.

After those mappings and physical choices are reviewed, the useful activation
experiment is one disposable real local transaction with the selected slot,
Library/Project/Graph/view and original receipt committed together, using injected
package evidence and actual restart/query cuts. Its schema and package interface
must follow the review; running it now would silently choose the still-open
physical mapping. Real package durability and native acceptance remain later.

There is no newly demonstrated contradiction requiring another cancellation or
COMMIT relay variant before LL2a review. Real transport integration, public
unknown-outcome presentation and production credential custody remain explicit
boundaries. Accepted original-ID reconciliation and preserved failure context
already apply; this audit neither invents a timeout policy nor relaxes them.
