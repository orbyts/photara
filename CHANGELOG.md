# Changelog

All notable changes to Photara will be documented in this file.

## 0.1.1 - Unreleased

- Reconcile already-present Photographer Final assets against either current
  provider-backed presence or immutable evidence verified in the exact Adobe
  inventory snapshot. A fully present selection now creates only a completed
  audit batch and reports that no transfer or DNG preparation is required.
- Preserve Pixieset imports as immutable evidence while adding audited local
  selection corrections with direct/effective membership, hierarchical
  Favorite/Shortlist/Hero implications, guarded cascade removal, dry-run
  output, current status, and append-only history.
- Report sanitized Adobe OAuth failure classes with direct reauthentication
  guidance, detect stored credentials issued for a different client ID, retain
  rotated refresh tokens, and expose a non-secret `adobe-status` diagnostic.
- Emit explicit structured success output from `health` and `config validate`
  instead of requiring verbose tracing to distinguish success from silence.

## 0.1.0 - 2026-08-17

- Treat Instagram's current 20-frame delivery capability as a maximum rather
  than a required package length. Accept positive packages from 1 through 20,
  reject empty or oversized packages, and preserve Red Meridian's exact
  20-frame expansion as a fixture-specific regression.
- Preserve publication-order filename prefixes for both Instagram and Threads
  Cloudinary backups while retaining compatibility with existing unnumbered
  Threads exports.
- Restrict placement authoring manifests to unresolved `crop` placements.
  Automatic `fill` and `contain` slots are never presented or assigned an
  authored crop; explicit `--reauthor` is scoped to a selected crop item.
- Add interactive stderr progress for layered-master checkpointing,
  flattening preparation/verification, and registration while preserving pure
  structured stdout and remaining quiet in non-interactive execution.

- Add immutable Instagram and Threads four-image grid templates. Instagram
  resolves four exact 2250×3000 cells; Threads resolves four exact 2250×4000
  cells. Generalize every ordinary placement to explicit `fill`, `contain`, or
  operator-authored `crop`; aspect mismatch alone never requires authoring.
  One dual-platform Photoshop session preserves independent 3:4 and 9:16
  transforms under separate post fingerprints.

- Add an installable **Prepare Photara HDR-SDR Master** Photoshop action. It
  wraps the finished 16-bit stack in one embedded source, creates ordinary
  shared Smart Object instances named HDR and SDR, opens Camera Raw Filter for
  operator-authored SDR, and validates the final paired structure without
  saving. Convert the parent to unmerged/unrasterized 32-bit Display P3 Linear
  before SDR authoring, and bind every action to the starting document ID so a
  tab change cannot redirect Camera Raw or later operations.
- Add `photara masters install-scripts` so the master-build, HDR/SDR-preparation,
  and paired-flattening scripts can be installed or refreshed without creating
  a new master batch.
- Add targeted `photara masters checkpoint --asset` recovery so one rebuilt or
  deliberately revised layered PSB can refresh its database fingerprint
  without accepting unrelated master changes.
- Add a guarded Lightroom Classic **Import Verified Layered Masters** action
  backed by a fingerprint-verifying Rust plan. It imports authoritative PSBs
  in place, records exact membership in read-only Photara plug-in metadata that
  exists only in the Lightroom catalog, and exposes it through smart
  collections guarded by native PSB file type. It writes no IPTC or keywords
  into layered files and reuses already imported files. Add a one-time
  reconciliation action for catalogs that used the prior keyword-driven PSB
  smart collections so Lightroom metadata conflicts can be cleared safely.
  Use Lightroom's direct plug-in-property lookup, visible stage captions, and
  a bounded catalog-write wait so reconciliation cannot remain as an opaque,
  indefinitely queued task. RAW/PSB stacking remains optional
  photographer-controlled catalog organization.
- Install `Build Photara Masters.psjs` in the shared Photara Scripts folder,
  remove only a byte-identical legacy Inbox copy, and make Photoshop master
  reports name the active project plus the first failure so stale manifests
  cannot masquerade as source-file errors.
- Make a prepared master canary session process only the marked canary, even
  when the Inbox already contains the complete DNG batch; an explicit prepare
  without `--canary` advances Photoshop to the full batch.
- Make project initialization tolerate network filesystems that reject file
  synchronization with `ENOTSUP`, recover an exact legacy temporary project
  manifest after interruption, and never promote mismatched temporary data.
- Add a photographer-facing Sylvan runbook covering the complete current
  workflow from an untagged Lightroom Classic import through registries,
  managed collections, Pixieset selections, Photographer Final, Lightroom
  Cloud, paired masters, Instagram and Threads layouts, WSP, Cloudinary exact-
  original backup, publication evidence, checkpoints, and recovery.

## 0.0.9 - 2026-08-14

- Generalize placement authoring around post schema v2 transforms with
  normalized crops and clockwise quarter-turn rotation, while preserving
  schema-v1 compatibility and exact legacy panorama geometry.
- Add fingerprinted, atomic multi-placement Photoshop authoring and capture
  sessions with stale-source/post rejection and idempotent report replay.
- Complete the independent 17-frame Red Meridian Threads package with
  `stacked-three@1`, inspected 9:16 Dynamic Range and Edit Comparison
  templates, independently authored crops, and rotated full-frame placements.
- Preserve the accepted 18-item/20-frame Instagram package as a regression
  fixture while allowing full-package and single-item render preparation.
- Reuse verified Reset + Adobe Color Edit Comparison TIFFs by project asset
  and rendering contract across platforms instead of regenerating identical
  sources for every post.
- Add guarded flattened-rendition refresh for deliberate external HDR/SDR TIFF
  replacement while retaining superseded provenance.
- Add durable manual-publication evidence tied to the exact post-specification
  checksum without inventing provider URLs or timestamps.
- Add signed, non-overwriting Cloudinary backup of exact WSP HDR JPEGs with one
  bundled keychain credential, immutable manifests, canary verification,
  per-asset provider evidence, full original-download SHA-256 verification,
  bounded network waits, and duplicate-free retry behavior.
- Complete Red Meridian Package A on Instagram and Threads, record both manual
  publications, and verify all 20 Instagram plus 17 Threads WSP originals in
  Cloudinary. Cloudinary remains backup storage, not the website media model.

- Add immutable, PSD-backed `edit-comparison@1` layouts with two ordered
  Before/After rows, exact inspected square-cell geometry, and camera/capture
  labels sourced from each authoritative camera RAW.
- Add a Lightroom Classic handoff that temporarily establishes the real Reset
  + Adobe Color state, exports a neutral full-resolution TIFF, restores and
  verifies the complete authored develop state, and fingerprints the result
  before Photoshop rendering.
- Build WSP-ready Edit Comparison pairs with the same neutral Before image and
  annotations in both composites, authored SDR After imagery in the base, and
  authored HDR After imagery in the alternate.
- Add `posts prepare-render --item <item-id>` for fast single-layout review
  while retaining full-package verification as the production default.
- Preserve Lightroom's tagged 16-bit ProPhoto neutral TIFF until it is placed
  into the verified 32-bit Display P3 Linear template. This lets Photoshop
  color-manage the cross-document transfer and avoids its temporary-document
  conversion falling back to a generic linear-sRGB working profile.
- Recreate camera and capture value layers with an explicit 27-point SF
  Compact Ultralight style and preserved baseline anchors, avoiding inherited
  mixed text runs from the Photoshop reference placeholders.
- Measure metadata using Photoshop's rendered glyph bounds, retain 27 pt when
  it fits, scale proportionally to a 22-point floor when necessary, and reject
  strings that cannot remain phone-readable within their template column.

- Add immutable, PSD-backed `dynamic-range-comparison@1` layouts with two
  ordered asset pairs and exact image-cell geometry captured from the design.
- Keep annotations identical across the Web Sharp Pro pair while allowing HDR
  gain only in the right image cells and the 1-to-10 headroom ramp. The SDR
  base uses the same image's SDR rendition on both sides and a flat-white
  headroom ramp.
- Add checksum-verified Photoshop reference installation and reject missing or
  mutated reference documents before rendering.
- Fit comparison imagery entirely inside each square cell, retaining the
  template background as pillarbox or letterbox bars without cropping.
- Isolate each comparison role in its own duplicated Photoshop document and
  explicitly reactivate it before ramp edits, avoiding stale UXP document IDs.

- Add the first typed layout vertical slice with an immutable global
  `full-frame@1` template and explicit Instagram and Threads authoring
  profiles.
- Add project-owned post initialization and idempotent full-frame item
  creation using friendly master filenames that resolve to stable Photara
  asset IDs.
- Resolve project posts against exact template checksums, authoritative PSBs,
  paired flattened HDR/SDR TIFFs, normalized placements, and outstanding
  SDR-authoring requirements without pretending an incomplete WSP pair is
  render-ready.
- Split the flattened-master representation into explicit HDR and SDR roles.
  Existing verified TIFFs migrate in place as HDR evidence; no file is copied,
  renamed, or falsely registered as an SDR rendition.
- Define the authoritative PSB contract as top-level `HDR` above top-level
  `SDR`, allowing one deliberate layered edit to produce both verified TIFF
  inputs for layout composition and WSP gain-map export.
- Extend the UXP handoff to accept either Smart Objects or groups for those two
  containers, render Smart Filters, and produce paired
  `<CANONICAL_BASE>_HDR.TIF` and `<CANONICAL_BASE>_SDR.TIF` files. Require both
  outputs to be flattened 32-bit Display P3 Linear documents with matching
  dimensions before replacing current rendition records atomically.
- Keep template selection global and versioned while keeping editorial order,
  asset choice, and repeated placements in platform-specific project files.
- Add a manifest-driven Photoshop UXP compositor for deterministic
  `full-frame@1` HDR-over-SDR documents and Web Sharp Pro handoff.
- Add immutable `stacked-two@1`, ordered top/bottom asset bindings, exact
  4500×3000 Instagram slot resolution, and matching HDR/SDR composition.
- Position placed layers from Photoshop-reported pixel bounds and reject any
  placement that does not exactly occupy its resolved slot.
- Activate the destination document before translating duplicated layers so
  Photoshop applies placement to the correct document context.
- Add immutable `continuous-panorama@1` geometry for two horizontal 3:4
  frames, with Web Sharp Pro retaining responsibility for the physical split.
- Add non-destructive panorama crop authoring and capture scripts with a 3:2
  marquee, seam guide, normalized project coordinates, and Rust-side source
  and specification fingerprint verification.
- Allow a stacked placement to reuse an existing authored crop only when the
  referenced item places the same stable asset, and resolve that crop against
  the verified paired TIFF dimensions before Photoshop composition.
- Add guarded, idempotent post reordering by exact item permutation and reject
  a full Instagram render manifest unless it expands to exactly 20 ordered
  delivery frames.
- Verify each immutable source rendition once per render-manifest preparation,
  even when the same asset has repeated placements, without weakening byte-size
  or SHA-256 validation.

## 0.0.8 - 2026-08-10

- Promote the final layered-master contract to 32-bit HDR P3 and refresh its
  byte size, SHA-256, and bit-depth evidence after raster editing.
- Add a manifest-driven Photoshop UXP flattening handoff that duplicates each
  authoritative PSB, flattens the duplicate, writes exactly one uppercase
  32-bit `.TIF` directly to the configured project directory, reopens it, and
  reports the Display P3 Linear profile, bit depth, and layer count for
  independent verification.
- Add guarded flattened-master verification and registration with
  PSB-to-TIFF provenance and an atomic `flattened` workflow transition.

- Model Lightroom project membership with additive hierarchical project
  keywords instead of the singular IPTC Job Identifier, allowing one asset and
  its shared Cloud representation to belong to multiple projects.
- Build the layered-master smart collection from Lightroom's native PSB file
  type rather than an incidental `psb` keyword.
- Keep capture IPTC and additive project membership as separate concepts; the
  selected-shoot action still applies the chosen shoot metadata, while future
  membership-only actions must not rewrite that intrinsic metadata.

- Add a configurable Lightroom handoff inbox, defaulting to the visible
  `~/Pictures/Photara/Inbox`, while keeping batch artifacts in its internal
  `.photara` workspace and preserving the same XDG TOML setting for a future
  GUI.
- Add a guarded layered-master promotion that copies each UXP-verified PSB
  beside its camera RAW, verifies the destination before registration, records
  RAW-to-DNG-to-PSB provenance, begins the editing lifecycle, and removes only
  the redundant staged PSB after the database commit.
- Track the raster-editing lifecycle with idempotent PSB checkpoints and a
  separately confirmed `ready-for-flattening` transition, refreshing current
  file evidence while retaining append-only workflow events.
- Preflight every configured camera RAW before hashing or promoting masters and
  report a stale storage-root override with an actionable manifest-regeneration
  command.
- Support verified promotion to SMB/network archives that do not implement
  `fsync`, while retaining mandatory byte-size and SHA-256 readback before the
  same-filesystem atomic rename.

- Preserve append-only Photographer Final decision history while maintaining a
  separate idempotent current-state projection.
- Add a guarded two-phase Cloud withdrawal that records the exact Adobe asset,
  requires manual deletion through Lightroom Desktop, verifies absence through
  a fresh provider inventory, and preserves transfer evidence and prior state.
- Reconcile a verified withdrawal back to Lightroom Classic by removing only
  Photographer Final and Cloud Present keywords from the retained RAW/XMP.
- Add a provider-neutral Cloud collection projection and Adobe album adapter
  for idempotent Locations, Scenes, People, and Projects hierarchies, using
  project leaf albums without duplicating Cloud assets.

## 0.0.7 - 2026-08-08

- Add an asset-scoped, provider-neutral Cloud evidence and presence schema.
- Add durable manual/API transfer batches and independent Photographer Final decisions.
- Add an idempotent Proetus SQLite evidence importer requiring explicit user
  confirmation of Lightroom Cloud presence.
- Preserve portable archive-relative paths so legacy volume roots can change.
- Add a read-only Adobe OAuth Native App probe using PKCE, callback state
  validation, automatic macOS custom-scheme callback capture, and
  in-memory-only token handling.
- Accept Adobe Lightroom's whitespace-tolerant JSON protection prefix and show
  an explicit local browser confirmation only after catalog verification.
- Store account-scoped Adobe refresh tokens in the operating system credential
  store, rotate them during refresh, and keep access tokens memory-only.
- Add durable Adobe login, non-interactive verification, logout, and remote
  catalog registration commands.
- Add paginated, read-only Adobe image inventory snapshots with strict
  pagination-origin checks and Proetus evidence reconciliation.
- Reconstruct missing legacy DNG names from camera stem and dated archive path
  only when the resulting Adobe association is unique on both sides.
- Replace legacy host-specific evidence paths with logical storage-root keys
  and archive-relative identities, and prohibit empty path values.
- Allow `PHOTARA_IMAGES_ROOT` and `PHOTARA_PROJECTS_ROOT` to override portable
  TOML defaults without introducing an Apogee dependency.
- Add a read-only storage audit for canonical keys, empty values, stale payload
  paths, and removed legacy columns.
- Add a fully reconciled Cloud-presence plan and a guarded Lightroom Classic
  action that can mark one selected original before applying all verified
  originals by archive-relative path.
- Add explicit Lightroom actions and shared Rust services for adding or
  removing selected camera originals from the independent Photographer Final
  decision set.
- Register decision assets by content fingerprint while storing portable
  `images:<relative-path>` camera-original locations instead of volume paths.
- Add deterministic, collision-aware transfer planning against the latest
  complete Adobe inventory and separately confirmed, idempotent batch
  reservation without generating or uploading files.
- Add resumable Lightroom Classic DNG rendering into isolated XDG cache
  batches, with exact-name, TIFF/DNG-header, byte-size, SHA-256, and RAW-lineage
  validation before each exported item advances. No upload or cleanup occurs.
- Add a read-only Adobe upload preflight that verifies the batch export gate,
  Lightroom entitlement, catalog identity, and sufficient Cloud storage before
  any remote asset is created.
- Add a restart-safe single-asset Adobe canary upload with a persisted remote
  ID, full-inventory refresh, exact filename and SHA-256 verification, and
  provider-confirmed Cloud presence before the remaining batch can advance.
- Add resumable sequential upload for the remaining transfer items, followed
  by one complete Adobe inventory refresh and atomic verification of every
  remote ID, filename, and checksum before the batch becomes complete.
- Reconcile Lightroom Cloud presence from both legacy evidence and newly
  provider-verified uploads, and surface actionable plug-in errors without
  misleading encoded process statuses or Lua source prefixes.
- Treat unmatched Cloud inventory entries as reportable state rather than a
  global blocker; apply local presence only where the latest provider inventory
  verifies an unambiguous camera-original mapping.
- Show immediate Lightroom progress while loading Cloud mappings and matching
  them against the active catalog.
- Add explicitly confirmed, restart-safe cleanup for completed transfer
  batches, with fresh provider checks, staged hash validation, strict path
  containment, non-recursive deletion, and durable removed-file state.

## 0.0.6 - 2026-08-08

- Replace numeric Lightroom collection prefixes with semantic nested groups.
- Keep workflow ordering understandable without names that resemble asset counts.
- Add a provider-neutral Hero selection state and smart collection.
- Add atomic Pixieset CSV imports with validation, source evidence, checksums,
  direct memberships, and idempotent replacement semantics.
- Add provider-neutral effective selection plans where shortlist implies favorite
  and hero implies both shortlist and favorite.
- Add a Lightroom action that applies imported selection keywords and reconciles
  their smart collections without modifying unrelated metadata.
- Read the project Job Identifier through Lightroom's formatted-metadata API so
  selection reconciliation can identify project photos on supported SDK versions.
- Read and cache catalog filenames through the same supported formatted-metadata
  API before entering the catalog write transaction.
- Match Lightroom smart collections against keyword leaf labels while retaining
  hierarchical keyword creation and assignment.
- Verify the applied Red Meridian XMP state across all 266 camera originals.

## 0.0.5 - 2026-08-07

- Add the thin `photara.lrplugin` Lightroom Classic adapter.
- Add a read-only connection validator for the CLI, Storexa, and PostgreSQL bridge.
- Add a project-selection and confirmation flow for the selected shoot.
- Reconcile managed IPTC fields, hierarchical people keywords, and smart collections.
- Add a Lua serialization bridge while preserving JSON as the public plan format.
- Preserve user metadata and collections and require explicit XMP persistence.

## 0.0.4 - 2026-08-07

- Add durable asset, project membership, representation, and provenance tables.
- Identify original assets by SHA-256 while preserving camera filenames.
- Add collision-safe downstream basename generation.
- Add a deterministic, read-only metadata and Lightroom collection plan.
- Keep all Lightroom mutations deferred to the thin plugin milestone.

## 0.0.3 - 2026-08-07

- Revise milestones around registry services, metadata planning, and the Lightroom plugin.
- Model social profiles as platform-to-handle maps.
- Add friendly CLI management for people, locations, and scenes.
- Add supported project reconfiguration for corrected registry associations.
- Correct Red Meridian's model from Valentina Reneff-Olson to Trinity Woodward.

## 0.0.2 - 2026-08-07

- Add the first full development roadmap through 0.1.0.
- Add typed XDG configuration and non-destructive configuration initialization.
- Add application-owned migrations for projects and workflow events.
- Add idempotent project initialization and directory materialization.
- Preserve single authoritative homes and immutable camera-original filenames.

## 0.0.1 - 2026-08-07

- Consume the supported Storexa 0.1 release from crates.io.
- Add a Photara-owned development persistence boundary.
- Add a read-only database health command for the Neon development branch.
- Document the separation between secret loading and application configuration.

## 0.0.0 - 2026-07-24

- Initial project scaffold.
- Reserve the Photara crate namespace.
- Add initial CLI entry point.
