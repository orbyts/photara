# Social profiles and portable Library export — S7 additions

## Current exact review packet — 2026-09-12

D16/D17 were approved at S7; older conditional-approval wording below is historical. The [D19 export delta](D19_STATIC_SCHEMA_DELTA.md#sync-v2-and-export) adds permitted logical slots/variables/ASTs while excluding access grants, invitations and all device/sync/recovery state. Export/import/encryption runtime remains deferred. R1–R8 were accepted as proposed 2026-09-12; CXT2 is complete without changing export runtime or fixtures.

**D19 supersession note (2026-09-12):** [Libraries and node Work Surfaces](LIBRARY_AND_NODE_WORK_SURFACES.md)
is the current conceptual target. Library is the durable ownership domain; each Project
has one Library and explicit Project access. Graphs use connected AssetSets and
declared frozen context; a private package ledger is not an ambient Gallery/asset
union. Library management is app-owned; node Work Surfaces embed authorized host
pickers/components. First install opens local My Library. The pre-D19 implementation,
physical identifiers, examples and fixture contracts below remain baseline evidence,
not approval to reinterpret stored bytes. Exact contract/static schema review and
revised CXT1/CXT3 precede L3; no migration, source or fixture bytes change here.

Status: requested design additions; **D16/D17 conditional approval pending**.
These extend the accepted [logical model](LOGICAL_DATA_MODEL.md) without changing
package authority, Storexa's domain-agnostic boundary, or implementation gates.
No provider/API/database access or runtime implementation is part of this slice.

## D16 — typed social profiles

`SocialProfileId` identifies a Library-scoped typed Library aggregate. Each
profile has exactly one immutable same-Library Person **or** Organization owner;
each owner has zero to many profiles, including multiple accounts on one provider.
Profiles have their own local/server revisions and ordinary typed mutations.
`SocialIdentity` means the optional provider subject coordinate held by a profile,
not a second Account table or an authentication credential.

The record retains:

- Immutable profile/Library/owner IDs, namespaced `provider_id`, schema version,
  created time; mutable revision/update time and `active | tombstoned` lifecycle.
- Optional exact `(subject_namespace, provider_subject_id)` pair. Namespace must
  distinguish app-scoped/tenant-scoped provider identifiers; no case folding.
  It may be adopted once when previously unknown, but cannot be silently replaced.
  A different provider subject means a new profile plus explicit retirement and
  provenance, not a handle-based identity rewrite.
- Mutable handle, display name, safe canonical public profile URL; provider-neutral
  account kind `unknown | personal | creator | business | organization | service |
  other` plus optional provider-specific kind. Handle/URL alone is sufficient.
- Verification observation `unverified | user-asserted | provider-authorized`,
  observation time, bounded typed provenance, fetch/refresh/next-refresh times,
  and fetch state `never | available | unavailable | revoked | expired | error`.
- Optional consented provider-avatar policy, consent/expiry timestamps and durable
  Library media digest. Cache-only avatar bytes/temporary URLs are device cache,
  not synchronized or exported durable media.

Manual entry works completely offline without OAuth. Unknown provider adapters
preserve manual facts; no request is required to save or tag a profile. Provider
subject IDs are stable coordinates only within their documented namespace—not
proof that the Person is that account's legal owner. `provider-authorized` records
how an observation was obtained, never current access, endorsement, identity proof,
or permission to fetch again. Account/Auth0 identity and membership remain separate.

Uniqueness is **Library + provider + subject namespace + subject ID** whenever
the subject is bound, including tombstones. A bound scoped subject cannot attach
to two different People/Organizations. Subject namespace distinguishes genuinely
app/tenant-scoped provider IDs; adapters must pin/validate it, not let a client
invent another namespace to bypass uniqueness. Handles are not unique identity
keys. Manual duplicate handles/URLs warn for review; they never silently merge
owners. SQL has one Library-wide partial unique index, an exclusive-owner check
and scoped FKs. Provider observations still are not legal identity proof.
No ordinary resurrection, subject reassignment or hard delete is approved.

Owner retirement/merge must explicitly retire active profiles in the same atomic
multi-root command. Immutable source profiles and bound subject reservations remain
historical. A bound subject is **not cloned or reattached** to the target in this
initial policy; an explicit transfer/reconciliation command requires a separate
approval and corresponding constraint design. Manual unbound profiles may be
recreated on the target with new IDs/provenance. This conservative restriction is
an explicit D16 decision, not implemented transfer support. All affected expected
roots/revisions are in the command; package snapshots remain unchanged.

### Avatar, refresh, and privacy policy

Default display order: user-selected durable thumbnail, then eligible consented
provider avatar, then initials. User choice is never overwritten by a background
refresh. Provider bytes are obtained only through an authorized/terms-permitted
adapter with consent and bounded size/media decoding; a manual URL is **not** an
instruction to fetch arbitrary content. Validate URL schemes/origins, redirects,
SSRF boundaries and redaction before any future request. No tokens or signed URLs
belong in profile URL/provenance/extension fields.

`none` performs no avatar retrieval. `cache-only` requires consent, records expiry
and provenance, and stores bytes outside durable Library media. Expiry, adapter
deletion requirements or revoked consent invalidate/delete cache and fall back;
network errors alone do not erase manual profile facts. `durable-consented` is an
explicit promotion only when user consent **and provider rights** permit storage,
redistribution in Library sync, project snapshots and/or backup as separately
applicable. Each use rechecks that scope; durable does not mean exempt from expiry
or erasure. Keep source/rights/consent provenance; do not export or embed media
whose license disallows that use. Current append-only media descriptors do not
authorize retaining prohibited bytes: a future erasure workflow can remove bytes
and retain a minimal unavailable/tombstoned descriptor with an audit reason.
Do not implement provider-derived durable collection before that policy is ready.

Refresh may update observed handle/display/URL/kind and permitted avatar metadata,
but cannot replace a bound subject or overwrite explicit user fields without an
explicit preference/reconciliation rule. Updates are normal revision-checked
commands. Rate limits, lack of permission, unsupported account type or deletion
return typed availability states, not guesses. Tokens and provider connection
handles stay in a private host/service credential store with independent consent
and revocation; none is added to the social_profiles tables or public sync feed.

Instagram is an **optional future adapter**, not a universal directory. The review
constraint is authorized Professional Business/Creator coverage under the supported
Meta API/permissions; do not promise arbitrary personal-account lookup. Adapter
implementation must recheck then-current official API availability and terms.
Manual handles/URLs remain usable when lookup is unsupported or unauthenticated.

### Project creation and cloud replication

Selecting/tagging a social profile resolves its typed Person/Organization owner,
then creates/reuses the ordinary ProjectPartyAssignment. Do not add a parallel
social-only party or authenticate an Account from this selection. The immutable
Person/Organization snapshot may include **chosen display facts**: provider label,
handle, display name and public profile URL, captured time and originating ProfileId
for provenance. It does not copy OAuth tokens, app-scoped provider subject IDs,
permission evidence, expiring avatar URLs or a provider response dump. Embedding
avatar bytes additionally requires the durable rights policy and explicit selection.
Lookup/deletion/rename never silently refreshes a saved snapshot.

Cloud replication uses the typed `social-profile` root with full revision/CAS,
receipt/feed/snapshot rules. Safe source metadata is shared only inside its
authorized Library. Membership changes still govern remote access. Provider
connection state/credentials are private and never part of the root. Do not
automatically attach social display data to cloud Project Catalog reports; any
rich sharing follows explicit same-Library opt-in. No public profile existence
is proof of Library membership or permission to upload its avatar.

## D17 — future portable Library export/import

This is a **non-gating future capability**; the present schema leaves typed stable
IDs, media descriptors, revisions and provenance available for it. No backup jobs,
new database, encryption implementation or importer is installed by this proposal.
Export is a logical snapshot bundle, never a copied live SQLite/WAL file.

Proposed family: `photara.library-export`, independently versioned from package,
SQL schema and sync protocol. A manifest has ExportId, format major/minor, codec,
source LibraryId and source revision facts, capture time, inclusion/privacy
policy, required features, sorted member paths, exact byte lengths/SHA-256 and
completeness/omission report. Content is bounded canonical typed JSON and verified
durable media objects. A checksum inventory covers all payload members; a detached
manifest checksum avoids a self-hash cycle. Checksums detect corruption, not
authenticity. No executable SQL, path traversal, symlinks, hardlinks, or automatic
network resolution is permitted when inspecting a bundle.

An optional encrypted outer envelope must use a separately reviewed standard
authenticated-encryption/container library; do not invent crypto or put recovery
keys/passwords in the archive. Password/key recovery, KDF/container choice and
encrypted filename/metadata leakage are future feature approval decisions.
Unencrypted exports require an explicit privacy warning and protected destination.
An authenticated envelope is verified before trusting decrypted manifests; wrong
key/tampering is failure, never partial import. Decompression/count/length/hash
and path limits apply before allocation or extraction.

| Included, subject to explicit selection/rights | Always excluded |
| --- | --- |
| Library display metadata and stable ID; typed People, Organizations, relationships, social profiles, LocationKinds/claims/retirement provenance and Locations | Account/Auth0 identity exports, Membership/billing/developer entitlements, authentication/authorization claims |
| Supported logical record revisions, tombstones, source schema/policy IDs and approved provenance | Tokens, credentials, cookies, signed URLs, provider connection IDs/secrets, security-bookmark bytes |
| Verified durable Library media authorized for export; omissions with reason | Provider cache, proxies/thumbnails derived as cache, expired/disallowed media |
| ProjectIds, logical StorageRootIds, selected portable catalog summaries, last observed CommitId/checksum and safe root-relative hints | Device IDs, absolute/mounted/UNC paths, SMB host/share URIs, bookmarks, machine fingerprints |
| Alias policies needed to interpret exported claims, format requirements and loss report | Active SQLite/WAL/SHM files, migration ledger, sync cursors/epochs/receipts/queues/leases, recovery intents, locks and transient state |

Project `.photara` packages and their managed assets/history are **separately
backed up**, not embedded by default. Export reports referenced/missing packages
and expected identities/commit checksums but never labels this as a complete
Project backup. Optional future collect-projects behavior needs explicit separate
scope and storage estimates; it cannot alter package authority.

Logical StorageRootId and ProjectId plus validated relative hints let a new
machine ask the user to rebind a root and rediscover packages. No old device ID,
absolute path, credential or bookmark is reused. Restored observations are
`imported/unverified` until actual package HEAD/commit checks pass. Missing or
duplicate ProjectIds use the existing catalog quarantine rules. Moving a root
does not change Project identity. External project bytes remain separately
unavailable until the authorized host resolves their new bindings.

### Import protocol and collision handling

1. Inspect/dry-run only: validate container, versions, limits, canonical bytes,
   all hashes, referential completeness, policy IDs, scope, privacy and media rights.
   Reject corrupt/unsupported input before writes; report omitted media/packages,
   unresolved provider subjects, missing kinds and every normalization collision.
2. Default restore is **side-by-side into an isolated new local store**, preserving
   Library/root/Project/record IDs and source revision/provenance facts. It is
   never a second automatic cloud writer. Do not restore membership, credentials,
   pending server work or an authenticated cloud association from a bundle.
3. Capture a reviewable import plan keyed to export manifest digest and current
   destination revision snapshot. Same ID + same semantic digest is a no-op;
   same ID + different content is an explicit conflict. Revisions from another
   device/export are provenance, never automatic “newest wins” authority.
4. Import-into-existing uses typed commands with expected local revisions and
   one bounded all-or-nothing transaction per approved import unit. A larger
   restore uses a staged new store and atomic activation, not silently split
   half-imported state. Revalidate the plan if destination changed during review.
   Stage/hash media first; activate verified bindings with recoverable bookkeeping.
   On failure leave the prior Library active and retain a diagnostic/report.
5. Resolve Kind canonical/alias collisions through the existing explicit reuse/
   claim-transfer rules, never a disabled unique constraint. Owner/relationship/
   social-subject duplicates require typed reconciliation. Import into a different
   Library or fork requires an explicit ID mapping/provenance report; it is
   not the default identity-preserving restore. Missing required parents reject.
6. Record ImportId, source ExportId/digest, chosen mappings, omissions and source
   revisions in a durable import report. New accepted local edits get new local
   revisions; old server revision strings cannot be installed as fresh accepted
   cloud state. Historical provider-authorized observations convey no restored
   provider permission; fresh authorized refresh is independent.
7. Verify reopened typed records/media, counts/IDs/digests/term ownership and
   rollback behavior before activating the restore. Then prompt for root rebind
   and verify discovered packages. Cloud association/upload is a later explicit
   authorized operation using current server state and normal conflict checks.

Exports contain personal data, including social identities and historic retired
records. Offer documented inclusion/redaction choices and explicit completeness
reports; protect transport/storage, avoid sensitive filenames/logs, and explain
that deleting local/cloud data cannot recall already exported copies. Provider
expiry/deletion and legally required erasure need a backup-retention/rotation
policy and user-visible limitations. A future erasure workflow must cover bytes,
metadata, caches and retained exports under the user's control; no universal remote
erase guarantee. This policy and encryption recovery require approval before the
export feature ships, but are not prerequisites for initial local schema work.

## Fixtures and gates

[Social/export fixtures](../fixtures/generation-two/social-export.json) supply
manual/multiple/stable-subject profile examples, selected snapshot facts and an
inert checksummed logical export specimen. SOCIAL/EXPORT cases in
[scenarios](../fixtures/generation-two/scenarios.json) specify ownership, rename,
provider absence, consent/expiry, safe snapshot, root rebind, corruption, collision
and transactional restore behavior. Static checks do not prove provider access,
SQL constraints, cryptography or runtime import safety.

D16 approves the typed schema/manual profile direction; automatic lookup/avatar
adapters require their own consent/provider review and are optional. D17 reserves
the portable export/import boundary; implementation/encryption/retention scope is
future and non-gating. See [S7 review](SCHEMA_REVIEW.md) for conditional approval.
