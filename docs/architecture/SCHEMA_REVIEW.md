# Generation-two database design — S7 review index

**2026-09-12: Suhail accepted R1–R8 as proposed; CXT2 acceptance is complete.**
[Accepted contract freeze](D19_CONTRACT_FREEZE.md), [static schema delta](D19_STATIC_SCHEMA_DELTA.md),
and [inert DDL/inventory](proposals/d19-cxt2/README.md) form the exact design record.
No database execution, runtime migration, Rust/UI, manifest or fixture change
occurred. [D19](LIBRARY_AND_NODE_WORK_SURFACES.md) and revised
[D18](TYPED_CONTEXT_AND_EXPRESSIONS.md) remain the conceptual foundation.
Subsequent [CXT1a pure Rust](CXT1A_CONTRACTS.md) was separately selected and
completed with one additive golden fixture. Subsequent [CXT1b](CXT1B_CONTEXT_CONTRACTS.md)
is complete with a separate context golden and 159 passing selected tests. Next:
separately selected CXT3a, then CXT3b/c. L3 remains paused; no live service or Neon work is authorized.

Status: **S7 approved; bounded L1 and L2 complete**, 2026-09-11. The user reviewed the packet
and explicitly directed work to the next stage, approving D1–D17 and the bounded
L1 codec/validator implementation, then separately authorized L2 local SQLite
implementation and fresh temporary databases. S0 Storexa 0.2.0 is published;
S1–S7 are accepted design inputs. This does not approve live database/deployment,
publication/locking, cloud/auth, live projects, UI, staging/commits or release.
Runtime tests are evidence only when actually run. Legacy import is non-gating.

## Review packet

Read [D19](LIBRARY_AND_NODE_WORK_SURFACES.md) and revised D18 first. Their approved
conceptual direction supersedes the conflicting terminology/access/asset assumptions
in this historical S1–S7 packet. The exact physical/portable delta was accepted through R1–R8 on 2026-09-12.

1. [Logical model / S1](LOGICAL_DATA_MODEL.md): accepted IDs and ownership.
2. [Package / S2](PROJECT_PACKAGE_SCHEMA.md): portable authority and publication.
3. [SQLite / S3](LOCAL_SQLITE_SCHEMA.md): full ordered local DDL and boundaries.
4. [PostgreSQL / S4](SERVICE_POSTGRESQL_SCHEMA.md): full ordered service DDL,
   Auth0 mapping, grants/RLS and entitlement boundary.
5. [Sync / S5](SYNCHRONIZATION_CONTRACT.md): versioned API, durable intent/feed,
   atomic Kind transfer and offline collision reconciliation.
6. [Fixtures / S6](GENERATION_TWO_FIXTURES.md): concrete bytes and expected results;
   [fixture index](../fixtures/generation-two/README.md) links every inert input.
7. [Social profiles and Library export — D16/D17](SOCIAL_PROFILES_AND_LIBRARY_EXPORT.md):
   requested typed profiles/privacy and future non-gating portable backup boundary.
8. [Storexa integration](STOREXA_INTEGRATION.md) and
   [execution roadmap](../ROADMAP_0_2_EXECUTION.md): library boundary and sequence.

## Authority summary

The table uses the historical physical baseline names. Under D19, Workspace means
Library; Project policy/grants qualify every catalog/content read independently
of blanket membership. Project assets mean the private graph/run ledger, not an
implicit AssetSet union. Project-only collaborators get assigned bounded snapshots.

| Data | Authority | Other stores |
| --- | --- | --- |
| Account, exact Auth0 issuer/subject, billing/developer grants | Authenticated service/control plane | Bounded local cache; Account is never a Person |
| Workspace membership and cloud authorization | Service | Offline local view cannot grant remote rights |
| Local Workspace Library: typed People/Organizations/relationships/LocationKinds/Locations | Local SQLite for On This Mac; local-first edits plus accepted service base for a cloud Workspace | Explicit mutation/receipt/feed reconciliation; no last-write-wins |
| Project metadata, immutable Library snapshots/assignments, assets/representations/resources, named graphs and run/effect/evidence history | Movable `.photara` package verified commit | Catalog is a projection, never a replacement or cloud backup claim |
| Paths, mounts, bookmark references, device availability | Device-local SQLite/host | No secrets/bookmark bytes or machine locators in portable package/cloud report |
| Publication/sync recovery intents, sealed requests, receipt/inbox/snapshot staging | Durable local state | Not disposable cache; no cross-store distributed transaction |
| Cloud catalog/media | Client-reported bounded observation; verified media descriptor + private storage binding | No client observation establishes verified package HEAD |
| Node release identity/category/search metadata | Versioned package catalog/manifest | Installation/trust/license separate; package pin never follows latest silently |
| Person/Organization social profiles | Typed Workspace Library roots; one owner, own revisions | No Auth0 identity proof; provider connections private; chosen display facts only in Project snapshots |
| Future Library export | Versioned logical snapshot with typed IDs/provenance and permitted media | Not a live DB copy or Project backup; import is dry-run/isolated, no auth/device/sync-state restoration |
| Thumbnails, proxies, browsing indexes | Rebuildable device cache | Essential evidence/authored assets never demoted to cache |

## Physical proposal inventory and evidence

D18/D19 introduce no concrete DDL: the counts below and applied L2 migration
checksums remain unchanged. Its separate inert fixture does not enlarge the
51-case S6 baseline or replace its nine canonical-codec vectors.
D18 adds 38 unexecuted scenarios and four proposed semantic byte/hash vectors.

| Proposal | Current static inventory | What this does not prove |
| --- | --- | --- |
| S3 SQLite | 44 tables; 179 parsed SQL statements; 71 FK pairings; 95 trigger bodies; six L2 migrations and selected local runtime tests pass | Full merge/sync/recovery conformance, service behavior or package filesystem durability |
| S4 PostgreSQL | 35 tables; 275 parsed SQL statements; 19 PL/pgSQL bodies; 58 FK pairings; 25 scoped RLS policies; 73 trigger/function bindings | Runtime privileges/RLS, security-definer context, transactional owner/claim constraints or Neon behavior |
| S6 package | 33 files in inert archive; 28 JSON objects, one blob, two commits; nine canonical positive vectors, seven negative expectations; L1 disposable reader tests pass | Application integration, publication/locking, SMB durability or complete future envelope support |
| S6 behavior | 51 scenario specifications; 12 named crash points; four sealed requests; social/profile/export addendum with three checksummed export members | Any executed database/service/SMB/import/provider conformance |

Counts describe authored proposal DDL, not installed databases. Neither physical
schema contains authoritative Project/Graph/Asset/Run tables. SQLite is local
disk only, never an active WAL file on SMB or inside a package. The PostgreSQL
family is clean generation two; no dependency on existing Neon layout or data.

## Decisions approved on 2026-09-11

The user reviewed D1–D17 and explicitly directed the next stage. The following
recommendations are accepted design choices, with the exclusions recorded below.

| ID | Recommendation | Boundary / cost |
| --- | --- | --- |
| D1 Package | Immutable objects, commit/inventory chain, one HEAD; independent authored/history roots; specimen envelope shapes | Retain ancestors/no automatic GC initially; control-size ceilings and later sharding |
| D2 Publication | Single writer with no timeout-only lock stealing; stage/verify/publish and durable recovery; explicit local/SMB limitations | Ambiguous SMB ownership stops writes; never promise cross-store ACID |
| D3 Identity | Stable ProjectId; moves/rebinds only locator changes; duplicate writable copies quarantine; explicit fork with provenance | No timestamp-wins or copied-ID silent divergence |
| D4 Library | Typed Workspace aggregates; multiple Person roles; Organization client identities; required concrete LocationKind | No Scene table or duplicate Client aggregate; UI label remains a later presentation choice |
| D5 Kind terms | Approve narrow atomic claim transfer, immutable source retirement snapshot, canonical promotion and explicit offline collision reconcile | Preserve Workspace+term uniqueness; no arbitrary stealing, unmerge, resurrection or tombstone key reuse |
| D6 Normalizer | Unicode 16.0.0 NFC/full-default-casefold/NFC/White_Space policy; immutable bidirectional Beach/beaches and Studio/studios seed; explicit custom aliases | Not general-language stemming; both creation directions claim full group; upgrades require collision audit |
| D7 Codec | Retain actual Rust canonical-json.v1 byte behavior and S6 vectors, not RFC8785/JSON.stringify | Add duplicate-key/typed-number streaming validation; dependency drift needs codec version |
| D8 Local persistence | Adopt Storexa 0.2 explicit SQLite types; app-owned SQL/migrations; local STRICT/WAL/FULL/FK settings | Deferred Storexa begin requires explicit SQLx BEGIN IMMEDIATE path for local CAS; no ORM/domain leakage |
| D9 Service security | Auth0 JWT/API mapping; exact issuer+subject; Account-first/Workspace lock order; least-privilege roles and forced scoped RLS | Desktop never has Neon credentials; RLS is defense-in-depth, not authentication; test real unprivileged roles |
| D10 Sync | Immutable sealed commands/receipts, one-batch feeds, receipt+ordered-base milestones, explicit rebase/no automatic merge | Durable overlay and history retention; reject oversized atomic commands, never split secretly |
| D11 Reset/retention | Bounded complete snapshot with CAS install; no v1 feed/receipt/tombstone pruning or automatic epoch reset | Oversize/lost history are explicit errors; future erasure/retention protocol is separate work |
| D12 Media/privacy | Staging separate from immutable final media; minimal catalog by default, rich explicit same-Workspace opt-in | No foreign-Workspace snapshot leakage, portable locators or secret URLs; revocation cannot erase offline cached bytes |
| D13 Limits | S6 exact protocol/string/depth/reader bounds; advertise lower service capacity when needed | Initial product limits, not finalized paid plan quotas; reject honestly before partial state |
| D14 Rollout | Clean generation two; only current gen2 one-JSON compatibility retained as L1 test | Legacy/v0.1.3/live Neon import is never schema, implementation or release gate |
| D15 Future adapters | CloudKit deferred; selected local/cloud authority explicit, no same-Workspace dual write | No CloudKit-via-SQL fiction; developer access does not bypass Workspace authorization |
| D16 Social profiles | Typed manual-first Person/Organization profiles; optional exact scoped subject, mutable handle/display/URL, provenance/refresh facts; Workspace-wide bound-subject uniqueness including tombstones | No identity proof/OAuth requirement; chosen snapshot display facts only; consent/expiry-aware avatars; Instagram optional. Bound-subject reattachment/transfer is not approved by this initial policy |
| D17 Portable Library export/import | Future versioned/checksummed logical bundle, optional reviewed encryption, typed records/social profiles/permitted durable media and portable discovery hints | Non-gating; packages separate; no credentials/device/absolute-path/sync state; dry-run/isolated or transactional restore, explicit collisions/rebind/privacy and erasure limits |

No final paid prices/quota schedule, retention/erasure SLA, Node Store commerce,
CloudKit container, or production Auth0/Neon resource is selected by this packet.
Their absence does not require a legacy importer or a rewrite. If D6's Unicode
version/seed or D13 limits change during review, regenerate the corresponding
fixture expectations before implementation; historical codec vectors never drift.

### Recorded approval scope

**D19 — conceptual direction approved 2026-09-12:** Library ownership/onboarding,
ProjectAccessGrant/invitations/policy separation, Work Surface vocabulary,
app-owned Library Browsers and embedded pickers, explicit asset ports and private
ledger, read/enrich/effect contracts, typed families, Layout/Gallery proposed
built-ins and stable hierarchical discovery taxonomy. Documentation consistency
and exact logical/package/NodeSDK contracts remain next gates. Approval does not
cover physical Workspace/ProjectAsset renames, new grant/RLS/feed schemas, changing
applied migrations, fixture baselines, manifests, runtime or UI. Existing D1–D17
rows above are a historical record and are superseded where D19 explicitly changes
the model; completed L1/L2 evidence remains valid for their bounded old contracts.

**D18 — exact details accepted through R5, 2026-09-12:** recommended explicit typed scopes,
ID-bound bounded ASTs with opt-in backtick/template syntax, closed uppercase
host-place bindings and lowercase semantic scopes, frozen least-privilege
context, typed metadata, secret-safe resource handles and post-success CAS
proposals. Concept inclusion/source preference are accepted; language limits,
snapshot/privacy/write policies, exact schema changes and implementation scopes
need the [D18 review decisions](TYPED_CONTEXT_AND_EXPRESSIONS.md). D1–D17 approval
does not cover new physical tables, migrations, evaluator or pane implementation.

S7 approves D1–D17, including the conservative bound-subject reservation policy.
L1 and subsequently L2 received separate bounded implementation authority. D16 reserves typed manual
profiles now; automated lookup/provider-avatar collection remains optional future
adapter work with separate consent/terms/erasure review. D17 reserves export/import
compatibility but does not make backup UI, encryption or an importer a condition
of L1/L2. Bound subject ownership is unique across a Workspace; the conservative
no-transfer/tombstone reservation choice is accepted. Any future transfer needs
a separately reviewed atomic transfer model before implementation.

Do not interpret conditional design approval as permission to execute DDL, create
cloud resources, migrate user data, implement unapproved UI, stage/commit or ship.
L1's read-only evidence is recorded in [the codec boundary](PROJECT_PACKAGE_CODEC.md);
the subsequently authorized [L2 boundary](LOCAL_LIBRARY_IMPLEMENTATION.md) records
local runtime results and its exclusions. D18/CXT1–3 now precede any L3 authorization.

## Exact implementation gates

1. **S7 decision record — complete 2026-09-11:** user approved D1–D17 and recorded
   unresolved exclusions. Approval is for the design and named implementation
   scope, not a blanket live database/deployment authorization.
2. **First bounded slice — L1 codec/validator harness complete:** implemented
   directory-package record codecs, path/reference validation, exact canonical
   vectors and inert specimen materialization only in disposable test roots;
   preserve current gen2 JSON import/export and existing Core behavior. Add
   duplicate-key rejection before Value construction. No UI or cloud writes.
3. **L2 local schema scope — complete:** separate Storexa 0.2 adapter/six migrations,
   16 new tests plus five retained Library tests. Only temporary DBs were used.
   No current Library store cutover. L2b full affected-root merge/claim transfer
   remains unexposed pending its separate bounded implementation/proof.
4. **D19/revised D18 gates before L3:** R1–R8 accepted as proposed 2026-09-12;
   CXT2 inert DDL/static inventory and CXT1a/b pure contracts are complete.
   Next separately select CXT3a, then CXT3b/c adapters/migrations/tests. Never rewrite
   L2 0001–0006; no implicit DDL execution.
5. **L3–L6 package publication, currently paused:** choose test roots for create/open/save/move/rebind/
   duplicate/copy/fork/recovery. Prove HEAD/catalog crash windows and single-writer
   behavior first locally, then on expressly authorized SMB test storage.
6. **G/C service slices in roadmap order:** durable multi-graph/history and sync
   harnesses pass their scenario families before integration. PostgreSQL schema
   execution, RLS/security/concurrency testing and fake API/media flows precede
   any cloud deployment. Creating Neon/Auth0/object-store resources requires a
   distinct authorized environment and credential/deployment scope.
7. **Release/presentation:** required runtime fixture families must pass before
   corresponding release claims. UI restructuring needs approved typed contracts
   and **raster mockup approval before implementation**; S7 does not approve UI.
   Existing unrelated dirty UI work stays untouched.

## Review acceptance

- [x] S1–S6 links, authority summary, physical counts, exact byte specimen,
  recommendations and static/runtime distinction assembled.
- [x] D1–D17 approved 2026-09-11; bounded L1 codec/validator work only authorized.
- [x] First bounded implementation/test scope chosen: L1, fresh temporary roots only.
- [x] L1 read-only codec/validator: 25 focused tests plus 29 retained tests passed.
- [x] L2 separately authorized: 44-table family and bounded typed repositories;
  selected regression total 75 passed, workspace Clippy/check passed.
- [x] D18 concept/source syntax direction incorporated in a separate inert proposal.
- [x] D19 conceptual approval recorded and documentation amended.
- [x] D19 consistency reviewed; exact logical/package/NodeSDK freeze candidate prepared.
- [x] Suhail accepted R1–R8 as proposed, including the static physical/portable proposal, 2026-09-12.
- [x] CXT2 inert DDL, exact inventory and static checks complete; no execution.
- [x] CXT1a separately selected and complete; pure contracts and Rust-generated golden verified.
- [x] CXT1b separately selected and complete; pure context contracts, golden and regression verified.
- [ ] CXT3a/b/c scopes accepted before implementation.
- [ ] Runtime conformance completed as implementation evidence, not prose approval.
- [ ] Any live infrastructure, migration of user data, UI, staging/commit or
  release action separately authorized when needed.

Next eligible action: separately select **CXT3a package reader/closure contracts** from the
[accepted slices](D19_STATIC_SCHEMA_DELTA.md#fixture-delta-implementation-slices-and-gates).
CXT2 and CXT1a/b are complete; CXT3/L3 have not begun. L3 waits for required contract and
runtime conformance; UI appearance still needs raster approval.
