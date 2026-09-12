# D19 CXT2 responsibility ledger

**Inert design evidence, not runtime proof.** This ledger accompanies every
[proposal SQL file](INVENTORY.md). R1–R8 were accepted 2026-09-12; later implementation
must satisfy both the SQL rules and the responsibilities below. No database was
opened to test these statements.

## Physical translation and transaction entry

SQL spells every accepted column with explicit nullability, scoped ownership FKs,
primary/unique keys and child-FK indexes. New local tables are STRICT. SQLite IDs
are nonnil 16-byte BLOBs; service IDs are nonnil UUIDs; digests are 32 bytes.
New time fields are UTC integer milliseconds locally and `timestamptz(3)` on the
service. Rust must reject excess precision before binding: PostgreSQL's type
coercion can round it. No implicit defaults choose a Library, principal, Project
policy, root classification or remote identity.

All mutable roots retain identity/ownership/creation fields and advance their
revision exactly once on update. SQL bounds revisions; the repository supplies
`WHERE revision = expected` (local `local_revision`) and requires one affected
row. A trigger's +1 check cannot prove that the caller supplied the expected
revision. Initial root revision, registered coordinate values, clock policy and
no-op handling belong to the command validator. Names and immutable evidence
reject deletion and updates. Explicit host deselection is the only proposal
DELETE exception; it still needs device authorization and selection CAS.

Local commands use `BEGIN IMMEDIATE`, enabled foreign keys, typed repository
dispatch and checks immediately before COMMIT. There is no raw mutation API for
children or cached remote access facts. The local-only principal must be the
explicit controller principal in A1. SQLite is not an authorization sandbox
against someone editing their own database file.

Service entry locks current Account(s), then Identity(s), each set in stable ID
order, then Library (shared for reads, exclusive for writes), then Project policy
and grant rows, then typed roots in stable order. The two authorize helpers cover
a single actor's entry; invitation/disable/claim commands must acquire the full
actor/target/inviter set first, including rows that could be revoked. Command
handlers recheck device state where relevant, action prerequisites, expected
revisions, current access generation and exact scope under those locks. Locking
one root or relying on a deferred trigger does not establish the complete lock
order. No host, provider, identity-provider or media I/O happens under a DB lock.

## Access, invitations and association

SQL checks masks 0..255, discover/read prerequisites and manage-access's invite
prerequisite. Inherited role masks are restricted to 0/1/3/71/11/79; restricted
policy requires four zero masks. `can_project` denies revoked grants even when
membership would otherwise inherit access, and unions active grants with only
explicit library-visible role masks. Editing never implicitly grants run.

An active registration must have A1 contract state, A3 policy, and retain an active
manager (the 255 manager preset).
The service's deferred manager guard also checks an active Account with an active
identity. It intentionally reads only access/identity facts across Libraries to
catch Account disable or last-identity revocation; it cannot be a Project-content
query. Existing Library-owner invariants remain in force. The local repository
must prove the same invariant at commit, using the local principal for local-only
Libraries and cached current Account facts for online-authorized cloud commands.
SQLite cannot express an arbitrary deferred commit trigger. A temporary no-manager
state during a correctly atomic transfer must not be rejected prematurely.

Controller duties, all in one transaction with immutable control receipt/audit:

- Default new Projects to restricted plus explicit creator manager. Verify the
  explicit one-Library association before activating A2; never infer ownership
  from catalog visibility, paths, nullable historical origin or Account absence.
- Check role ceilings and action containment. Library admin/owner status grants
  no Project self-grant or hidden recovery permission. Regrant is a dedicated CAS
  command on the retained row; generic patching must not revive a revoked grant.
  Removing an override is active mask 0, distinct from revocation. Membership
  revocation retains independent grants; remove-all enumerates them explicitly.
- Advance A1 authorization_generation and revision once for the entire access
  command, and A3 generation/revision once per affected Project. Membership,
  grants, policies, Account/identity disable and closure all invalidate access.
  SQL verifies generation steps on A1/A3; it cannot infer the affected command
  set or prove that every related row changed in the same logical operation.
- Require an existing target Account, current inviter authority and seven-day
  maximum expiry; verify opaque token, exact target, current policy revision,
  expiry and idempotent request bytes at acceptance. SQL enforces one pending
  invitation, terminal state, grant scope and a one-time verifier consumption;
  it cannot verify token cryptography or current inviter permissions. Expired
  pending rows are finalized before replacement. No local/offline acceptance.
- Atomically bind accepted_grant_id to the intended target and exact permitted
  mask, and consume its verifier. Complete only pending invitations; acceptance
  time must precede expiry. An invitation or token alone grants no read access.
- For access widening, retain the exact actor, reported commit ID/hash,
  projection hash and proposed policy hash in disclosure_ack in the control
  receipt/audit. The host verifies the pinned package and capture audience;
  the service verifies reported evidence and current consent, not SMB contents.

Closed/revoked/tombstoned data remains historical. Service acceptance is not proof
of a package write or external effect. Cloud publication/protected run/effects
require fresh access; retained bytes, drafts and pure replay remain available
under the accepted offline contract. No offline lease is fabricated.

## Storage and typed context

SQL preserves StorageRoot/Workspace UUIDs, enforces location kind/provider
immutability, keeps names forever reserved and closes each current name through
its scoped name claim. Deferred name FKs permit slot/claim and variable/claim
creation in either safe statement order. Supporting composite UNIQUE indexes
on service stream/batch scope implement the approved scoped FKs without adding
identity columns. Local schema names remain physical compatibility adapters.

An active slot requires an active classified root; root retirement blocks active
slots and locators. Repository commands must update root/spec as one CAS aggregate,
validate rights and provider coordinates, and emit one composed root. A legacy
root without spec stays unresolved until explicit classification. Slot rename or
retarget uses CAS; captured SlotId/revision/target remains pinned. Old external
resolver IDs map only through verified append-only compatibility coordinates.

Host selection is unique per device/Library/root and points to a verified
candidate in that same scope. Retire requires deselect/reselect first. Rebinding
or changed availability advances binding generation; observation refresh advances
revision. The resolver must decide whether changed evidence also changes effective
resolution and therefore generation. Device permission, actual platform evidence,
lease expiry and provider rights require the host adapter. Paths/secure-reference
IDs are confined to H1; secret/bookmark bytes and live leases never serialize.
`$project.artifacts` can be materialized for publishing only through the publisher;
`$project.root` gives no arbitrary package-write authority.

Variable definition/default/value/name/dependency changes form one parent CAS and
one exact post-state. ValueId is stable on replace/clear. SQL prevents owner/type/
schema/namespace changes and foreign-scope dependency targets; the typed compiler
must validate registered type coordinates, namespace/name/reserved built-ins,
allowed overrides, literal/expression binding tags, expression owner and version,
default/current agreement and canonical dependency ordering. Append-only dependency
rows may be inserted only while creating their new expression, not appended later
to an already accepted AST. No separate child mutation endpoint exists.

Rust proves source/AST correspondence and hashes, UTF-8/canonical agreement,
1,024 AST nodes/depth 32, contiguous up-to-256 direct dependencies, no cycles,
10,000 expanded bindings, 100,000 interpreter operations, 64 KiB values and
1 MiB snapshots. SQL bounds source to 16 KiB and AST bytes to 64 KiB, but cannot
prove those semantics. Initial containers are expression/template; fenced blocks
remain unsupported. Library expressions refer only to same-Library variables and
slots. Project→Graph→Node direction, current-node input references, capture privacy
and resource permissions are package/compiler duties.

Recursively reject AssetSet, metadata patch/set, artifact/group/receipt, live
handles and workflow-output refs inside variable values. AssetSet v2 is an ordered,
duplicate-free explicit snapshot (10,000 members, 500/page maximum), with exact
representation/metadata revisions. No ambient Project union is an input. The
service stores and validates ASTs without evaluating them. Restricted and host-only
payloads never enter cloud variable rows, receipts, shared batches or snapshots;
personal values require portable policy. Ordinary capture-consent-required data
requires the accepted explicit projection consent. Local restricted rows survive
claim only with an explicit omission/unresolved inventory, never a false full-backup
claim. SQL shape tests do not replace recursive privacy inspection.

## Recovery, scoped sync and media

SQL retains apply requests and observations, separates local-applied from
package-published authority and permits only one terminal observation per
operation. Rust verifies canonical bytes/hashes, source proposal/run/attempt,
single target authority and exact expected commit, deduplicates unchanged requests,
checks verified publication evidence and settles the intent only with its correct
receipt. Unknown outcomes require reconciliation, not blind redispatch. External
effects remain in their existing permitted recovery contract; O1/O2 are variable
apply journals, not mislabeled package-save or generic effect rows.

Local SQL preserves sealed request/receipt bytes, same-channel predecessor FKs,
one pending inbox/snapshot and retained access-lost evidence. Repository duties:

- Validate the full state-transition matrix, including accepted receipt schema,
  immutable sealing, changed-request collisions and no superseding unknown or
  accepted work. Reject root additions after sealing and cyclic predecessors.
- Apply one complete batch in order: verify actor/scope/generation/epoch,
  cursor-before, signature, whole canonical byte/digest agreement and exact typed
  root set; compare every expected base and local revision before updating any.
  Advance cursor only with the complete apply; preserve pending overlays.
- Install a bounded snapshot (16 MiB/10,000 roots), compare expected cursor/base,
  atomically retain working overlays/recovery/device data, and quarantine omitted
  access projections. Access loss is not a domain tombstone; regrant requires
  fresh authorized bootstrap. No opaque cursor comparison or skipping a failed row.
- Freeze v1 transport before activation only after sealed operations settle under
  their original bytes/protocol. Unresolved outcomes block activation. No rewriting
  old checksums, mutation encodings, receipts or SQLx ledger entries; no dual workers.

Service deferred guards check accepted receipt↔batch identity/scope, stream epoch,
contiguous bounded child ordinals/counts and high-water closure. They are a concrete
proposal, not measured production queries; whole-Library scans may later be narrowed
without weakening the invariant after concurrency tests. Controller preflight must
also prove exact typed scope: Library batches contain no Project catalog/locators,
Project batches contain only their one Project and no Library directory/ACL.
Canonical bytes may never be per-user filtered. Request actor/device, current
generation and permissions are rechecked on receipt retry as on first mutation;
a control-accepted receipt has no content batch. No package/effect receipt is
fabricated. Acknowledgement guards bound sequence and prevent regression within a
generation; the controller additionally proves bootstrap on a new generation.

Upload columns preserve historical all-null sessions. New Library uploads carry
actor/generation; Project uploads also carry exact Project/purpose. V2 refuses
historical session resumes, checks recorded actor/current generation/edit/consent
on create and finalize, and verifies immutable bytes before media linking. Staging
state is visible only through its actor-bound route. Project media access needs
exact active P1 digest/purpose and current Project read; a digest alone gives no
Library access. Object-store verification and URL issuance occur outside SQL.

## RLS and privilege boundary

0012 drops all 25 old workspace_scope policies before installing replacements.
It does not add an OR-able policy beside the broad baseline policy. API content
policies check Library or Project actions; context children inherit the owning
variable's sensitivity; locator writes additionally need manage-storage. Access
facts are controller/auth-read only. Raw v1/v2 streams, receipts, upload state and
object keys have no ordinary API grants. Typed service routes use the protected
controller boundary for scoped commands, media staging and bounded responses.

Owner policies on access/header facts are deliberately narrow in table reach and
nonrecursive, so SECURITY DEFINER helpers can read current authorization and the
deferred account-disable check can inspect affected access facts. Functions have
fixed qualified search paths and no PUBLIC execution. No ordinary runtime login
may own objects, inherit photara_owner or have BYPASSRLS. Private trigger functions
have no API execution grant. Existing identity/billing controllers retain their
separate restrictions; their credentials are never a user-facing Project bypass.

The controller role remains trusted server code, with scoped table privileges.
RLS cannot make arbitrary SQL under that role safe: each route must call the
appropriate authorization helper and return an allowlisted DTO. Workspaces full
rows and Library directory queries require membership. Project-only access uses
a separate bounded header/access DTO. Discovery returns only ProjectId, owning
Library header and request-access availability; titles, counts, locators, previews,
receipts and joins require read. Errors mask inaccessible existence consistently.
No caller-supplied account, mask, scope GUC or role selection is proof of authority.
Transaction-local Account/Identity/Library/Project/purpose context is cleared by
commit/rollback and verified on pool reuse in CXT3c.

## Activation and proof gates

Floor statements require exact known baseline metadata, one affected row, explicit
association/claim mappings, settled sealed-v1 work and the complete guard cutover in
one controlled migration boundary. Neither backend is activated by this document.
Unknown versions must refuse writes; no automatic down migration or reinterpretation.

CXT1a/b prove pure contracts before CXT3 adapters. CXT3b must test fresh/local-0006
upgrade, foreign-key and integrity checks, rollback, last-manager transfers, CAS,
selected rebind and floor refusal in disposable SQLite only. CXT3c must parse and
execute actual SQL, test unprivileged SELECT/INSERT/UPDATE/DELETE, concurrent
revocation/grant/disable/invitation, pooled GUC cleanup, full feed closure and fake
media/identity flows in disposable PostgreSQL only. Grammar, RLS, permissions,
triggers and races are not certified by the static inventory. L3 remains separately
gated for package writer readiness, durable publication and crash recovery.
