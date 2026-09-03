# Photara Roadmap

Photara is the operational brain of a photography workflow. Storexa provides
database infrastructure; Photara owns schemas, repositories, reconciliation,
and every photography-specific decision.

`0.1.0` is the first released reusable operator workflow. Versions `0.0.1`
through `0.0.9` remain Git checkpoints. Architectural work begins only with a
separate `0.2.0` discovery and dependency-mapping pass. The `0.1.x` branch is
the supported maintenance line for the complete CLI, Lightroom Classic, and
Photoshop workflow while generation two develops independently on `main`.

## Representation ownership invariants

Each representation has exactly one authoritative home:

| Representation | Authoritative home |
| --- | --- |
| Camera RAW and XMP | `images_root/YYYY/YYYY-MM/YYYY-MM-DD/`, managed by Lightroom Classic |
| Working DNG | Lightroom Desktop/Cloud |
| Layered PSB | Beside the original RAW |
| Flattened TIFF | `projects_root/<project>/masters/flattened/` |
| Delivery rendition | Configured delivery provider |
| Pixieset proof | Temporary and removed after selection reconciliation |

Camera-original filenames are immutable. A selected asset receives an expanded
basename only when it leaves the Lightroom Classic-managed phase. If the normal
expanded name collides, Photara adds a deterministic content-hash suffix and
never overwrites an existing file.

Version lineage is explicitly deferred until after `0.1.0`.

Asset identity and provider presence are global, while project membership is
many-to-many. Lightroom projects are projected through additive
`projects/<project>` keywords and provider album membership; the singular IPTC
Job Identifier describes the original or primary shoot and is never used as
the authoritative project-membership key.

## Milestones

### 0.0.0 — namespace reservation (complete)

- Publish the hello-world application.

### 0.0.1 — Storexa integration (complete)

- Consume Storexa 0.1.0.
- Connect to the Neon development database.
- Add a read-only health check.

### 0.0.2 — project foundation (complete)

- Load typed non-secret configuration from `$XDG_CONFIG_HOME/photara`.
- Keep secrets in environment variables supplied by any secret manager.
- Own and run Photara migrations through Storexa.
- Create idempotent project records and project directories.
- Initialize and verify the `red-meridian` project.

### 0.0.3 — registries and project correction (complete)

- Model people, aliases, roles, and platform-specific social handles.
- Add friendly add, list, show, and replace commands for people, locations,
  and scenes.
- Expose stable JSON output for the Lightroom plugin and future GUI clients.
- Reconfigure an existing project transactionally without changing its ID.
- Correct Red Meridian's model association to Trinity Woodward.

### 0.0.4 — asset identity and metadata plans (complete)

- Add assets, project membership, asset files, fingerprints, and provenance.
- Preserve original RAW filenames and archive paths.
- Expand names only for selected downstream representations.
- Add collision detection that never overwrites files.
- Produce a pure, inspectable reconciliation plan for a selected shoot:
  managed IPTC fields, hierarchical keywords, and collection membership.
- Keep Lightroom mutation out of this milestone.

### 0.0.5 — thin Lightroom Classic plugin MVP (complete)

- Scaffold `photara.lrplugin` as a thin Lua adapter to the Photara CLI.
- Show a whole-shoot dialog for project, people, location, and scene.
- Apply only Photara-managed IPTC fields and hierarchical keywords.
- Reconcile the People, Locations, Scenes, and Projects collection trees.
- Preserve every user-owned field and guide XMP persistence through Lightroom's
  supported automatic-write preference or Save Metadata command.
- Make repeated execution converge without duplicate keywords or collections.

### 0.0.6 — client-selection workflow (complete)

- Replace numeric collection prefixes with semantic collection sets for
  Originals, Selections, Cloud, and Masters.
- Export temporary Pixieset proofs while retaining original filenames.
- Import and retain explicitly assigned Pixieset favorite-list CSV evidence.
- Reconcile favorites, shortlist, and hero results back to unique source RAWs.
- Apply client-favorite, client-shortlist, and hero keywords while preserving
  photographer-final as an application-owned later decision.

### 0.0.7 — Lightroom Cloud baseline and guarded delivery (complete)

- Import the legacy Proetus packaging ledger as migration evidence without
  treating its local/uploaded lifecycle fields as authoritative Cloud presence.
- Persist independent Photographer Final decisions for any project asset.
- Add Adobe authorization and a read-only Lightroom Cloud adapter.
- Inventory all 1,520 existing Cloud assets before enabling uploads.
- Reconcile the Proetus evidence against Adobe inventory, then mark confirmed
  catalog originals with `workflow|cloud|present` through Lightroom Classic.
- Persist Adobe catalog IDs, asset IDs, SHA-256 values, and import runs.
- Generate selected DNGs with expanded basenames.
- Plan uploads as a subset of Photographer Final against the complete Cloud
  inventory, while allowing observed legacy Cloud assets outside that set.
- Reserve operations transactionally and treat duplicates as already present.
- Remove temporary local DNGs only after Cloud presence is verified.

### 0.0.8 — PSB and flattened-master workflow

- Make Photographer Final and later editorial membership reversible without
  deleting assets or completed evidence; preserve append-only decision history
  and invalidate only unfinished downstream work.
- Provide one resumable withdrawal operation: use a provider API when one is
  officially supported, otherwise persist an exact manual-action handoff and
  resume automatically from provider verification. Never depend on memory or
  silently call undocumented endpoints.
- Project Locations, Scenes, People, and Projects into Lightroom Cloud as
  shallow provider-owned collection sets with the project as each leaf album;
  reference the same Cloud assets from every relevant leaf without duplicating
  files, and persist provider IDs plus verified memberships. Treat these as
  service-owned Adobe `project`/`project_set` evidence, not as ordinary
  user-owned Lightroom Albums. They may remain invisible in Lightroom clients
  until Photara is approved and recognized by Adobe as a Connection.
- Harden every Lightroom plug-in progress scope: attach it to a function
  context, guarantee cleanup on success, cancellation, and error, avoid invalid
  indeterminate-to-determinate transitions, yield after completion to work
  around Lightroom's lingering-progress SDK bug, and keep the Dock indicator
  consistent with actual Photara subprocess state.
- Take the 12 remaining Red Meridian Photographer Final assets through manual
  Lightroom Desktop and Photoshop editing as the first complete master
  workflow. Preserve the original 14-member decision history and the two
  provider-verified withdrawals as evidence of the reversible workflow.
- Register layered PSBs beside their corresponding camera RAWs without moving
  or copying either representation.
- Require every layered PSB to contain its edited Cloud DNG as an embedded
  Camera Raw Smart Object. Linked Smart Objects and flattened TIFF Smart
  Objects do not satisfy the master contract: the PSB must remain
  self-contained after temporary DNG cleanup and must reopen the raw content in
  Camera Raw for later nondestructive adjustment.
- Prefer Lightroom Desktop's `Original + Settings` handoff when the Cloud
  source is already DNG so the exact DNG and its current adjustment metadata
  enter isolated staging without an unnecessary DNG conversion. Retain a
  provider-specific `DNG` export fallback, and prove both semantics through a
  canary before treating them as interchangeable.
- Normalize staged extensions to uppercase (`.DNG`, `.PSB`, `.TIF`) through a
  collision-safe, case-insensitive-filesystem-aware rename; never depend on a
  provider application's extension casing.
- Make the parent master document contract configurable and evidence-backed.
  Red Meridian uses a 32-bit HDR P3 PSB for Photoshop's native HDR raster
  workflow; Photara records and validates the actual bit depth and embedded
  ICC profile and never performs an implicit bit-depth or profile conversion.
- Automate the repeatable master handoff through a Photara manifest and a
  narrowly scoped Photoshop UXP script: open the staged DNG with its settings,
  create an embedded Camera Raw Smart Object, configure and verify the parent
  document, save the exact PSB name beside the RAW, reopen it, and emit a
  machine-readable verification report. Require one visual canary approval
  before the remaining batch proceeds.
- Register flattened TIFFs under
  `projects_root/<project>/masters/flattened/` without creating another
  permanent copy.
- Preserve immutable camera-original names and use the expanded
  `<camera-stem>_<capture-date>_<author>` basename for downstream masters.
- Associate masters with source assets deterministically and reject ambiguous
  filenames, collisions, unexpected locations, and mismatched representations.
- Validate actual file type, location, byte size, and SHA-256 before
  registration.
- Record the provenance chain from camera RAW through the verified Lightroom
  Cloud DNG to layered PSB and flattened TIFF.
- Enforce one authoritative current PSB and one authoritative current flattened
  TIFF per asset while retaining historical database evidence.
- Add idempotent discovery, registration, verification, and master-status
  commands that a future GUI can call without embedding rules in Lua.
- Project verified master readiness into Lightroom/XMP while keeping the
  database and filesystem as the authoritative state.
- Import only the exact authoritative PSBs through the Lightroom Classic SDK,
  mark them with read-only Photara plug-in metadata stored only in the catalog,
  and expose the database-verified set through smart collections requiring both
  exact project membership and native PSB file type. Do not require a user to
  synchronize an entire filesystem folder or write Lightroom catalog metadata
  into layered PSBs. Photoshop remains the sole PSB writer, and Lightroom opens
  them with Edit Original. Stacking with camera originals is optional
  photographer-controlled catalog organization and must not gate import or
  master readiness.
- Complete and verify all 12 current Red Meridian master chains with no
  permanent local DNGs; retain auditable evidence for the two withdrawn chains
  without incorrectly treating them as current masters.

#### 0.0.8 delivery slices

1. **Reversible editorial state and Cloud withdrawal — complete.** Preserve
   append-only decisions, verify exact provider removal, and reconcile only
   Photara-owned Lightroom keywords. Red Meridian has exercised this path for
   two assets.
2. **Provider-owned Cloud collection projection — complete at the backend.**
   Persist deterministic Adobe collection IDs and verified memberships while
   documenting that Connection visibility is deferred until Adobe recognizes
   the fully realized Photara application targeted for 0.2.0.
3. **Lightroom progress lifecycle hardening.** Introduce one shared,
   context-bound progress helper and migrate every plug-in action so completed
   work cannot leave a stale Lightroom or macOS Dock progress indicator.
4. **Red Meridian editing handoff.** Finish the 12 current Photographer Final
   edits in Lightroom Desktop, export one canary with `Original + Settings`
   into the configurable, visible `~/Pictures/Photara/Inbox` handoff, normalize
   its extension, and use the
   Photoshop automation to create an HDR P3 PSB containing an embedded Camera
   Raw DNG Smart Object, then use 32-bit depth for final raster editing. Reopen
   and visually approve the canary before
   batching the remaining 11. Do not rely on Lightroom Desktop's generic
   `Open as Smart Objects in Photoshop` command when it supplies a flattened
   TIFF for an edited source.
   Keep operational artifacts inside the inbox's `.photara` workspace and
   reserve XDG cache for internal scratch; no photographer-facing action may
   require browsing into a hidden cache directory. The GUI will expose this
   TOML-backed inbox path in Settings.
5. **Layered PSB discovery and registration.** Find each authoritative PSB
   beside its RAW, validate identity, location, file signature, size, and
   checksum, and register it idempotently without moving or copying it. Record
   whether the embedded-DNG Smart Object contract was machine-verified or
   explicitly attested; temporary DNG cleanup remains blocked until that
   contract is satisfied.
6. **Flattened TIFF discovery and registration.** Use a manifest-driven UXP
   handoff to flatten a duplicate of every ready 32-bit PSB directly into the
   project's configured master directory. Reopen each TIFF, require Photoshop's
   32-bit Display P3 Linear HDR working profile with exactly one layer, then
   validate its signature, size, checksum, and provenance before separately
   confirmed registration.
7. **Master provenance and readiness.** Link RAW, verified Cloud DNG evidence,
   PSB, and TIFF; expose status and audit commands; and project only verified
   readiness into Lightroom/XMP.
8. **Vertical-slice verification and release cleanup.** Verify all 12 current
   chains, the two withdrawals, repeat-run idempotency, guarded cleanup, tests,
   documentation, migrations, and the final 0.0.8 commit and tag.

### 0.0.9 — layouts and HDR export checkpoint (complete)

The Red Meridian Instagram Package A project specification resolves to the
accepted 18-editorial-item, 20-frame sequence in `LAYOUTS.md`. The independent
Threads package resolves to 17 9:16 editorial frames. Both packages have been
authored, rendered, reviewed, exported through WSP, manually published, and
recorded with publication evidence. The exact WSP HDR JPEGs are staged under
the project and verified in Cloudinary: 20 Instagram originals and 17 Threads
originals.

Edit Comparison neutral sources are now project-asset evidence rather than
platform/post artifacts. The Lightroom bridge reuses an already verified
Reset + Adobe Color TIFF across Instagram, Threads, and future targets, and
only resets/exports assets missing from that registry. Red Meridian's four
Instagram sources are the accepted shared evidence for its Threads package.

Completed technical slices include paired HDR/SDR flattened masters,
`full-frame@1`, `stacked-two@1`, `continuous-panorama@1`,
`stacked-three@1`, platform-specific Dynamic Range and Edit Comparison
templates, project-owned post JSON, generalized placement authoring,
deterministic render manifests, Photoshop UXP rendering, guarded manual
publication evidence, and Cloudinary backup delivery. Photoshop remains an
operator-controlled UI: Codex may prepare and verify background artifacts, but
must not open Ghostty or drive Photoshop on the operator's behalf.

For this checkpoint, Cloudinary is strictly an off-site backup of the exact WSP
HDR JPEG originals. It is not the website media model and does not define
editorial or website order. Photara records stable asset identity, local and
remote byte counts, SHA-256 evidence, provider IDs/URLs, and optional
project/post/platform/item provenance. Website derivatives, thumbnails,
presentation order, and the Loomara contract remain deferred until the website
is designed.

### 0.1.0 — first complete reusable workflow (complete)

`0.1.0` is the first supported technical and operator workflow, not merely a
Red Meridian demo. Red Meridian is the regression fixture. A second project,
Sylvan, must run through the same workflow without source changes, project-name
branches, or hard-coded asset IDs.

The release covers RAW ingest, metadata, Pixieset selections, Photographer
Final, Lightroom Cloud DNG, Lightroom Desktop editing, paired layered and
flattened masters, platform-specific layout authoring, HDR gain-map output,
verified Cloudinary backup delivery, and evidence-backed Instagram and Threads publication.
Neon remains the authoritative operational database. Project-owned JSON remains
the editable layout source; immutable resolved snapshots and provider receipts
become database evidence. No alternate persistence store, standalone desktop
application, website pipeline, Codexa integration, Loomara integration, or
polished visual editor belongs in this release.

#### Placement-authoring architecture decision

The pre-generalization panorama implementation was the compatibility baseline:

- `PostPlacement` stored focal point plus an optional normalized `crop`; it had
  no rotation.
- `prepare_panorama_crop` accepted only one placement using
  `continuous-panorama@1`, wrote one special-case manifest, and installed two
  special-case Photoshop scripts.
- Photoshop opened one flattened HDR source, created a 3:2 rectangular
  selection, and wrote one report containing source pixels and normalized
  coordinates.
- `apply_panorama_crop` verified project/post/item identity, the complete post
  file SHA-256, the source TIFF SHA-256, source dimensions, aspect ratio, and
  agreement between pixel and normalized coordinates before updating the post.
- Render preparation resolved the same normalized crop against equal-sized HDR
  and SDR TIFFs. `Build Photara Layouts.psjs` crops each source before fit/fill,
  so HDR and SDR geometry was identical. It could not rotate a source.
- Post specifications are durable project-owned JSON on the configured project
  root. At the start of this slice, the database contained asset/master
  evidence but no publication or Cloudinary-delivery tables.

The generalized contract introduced these platform-neutral
concepts:

```text
PlacementTransform
  crop: optional normalized source rectangle
  rotation_quarter_turns_cw: 0, 1, 2, or 3

AuthoringManifest
  project/post/platform identity
  source-specification and canonical authoring-input fingerprints
  ordered unresolved placements with template slot bounds and source evidence

AuthoringReport
  matching session identity and fingerprints
  one result per ordered placement, including source dimensions and transform
```

The transform belongs to `PostPlacement`, because the same asset can require a
different crop or rotation in each post, platform, item, and slot. One transform
is applied identically to HDR and SDR renditions. Rotation occurs before crop;
normalized crop coordinates are interpreted in the rotated source coordinate
space. For `0.1.0`, rotation is restricted to exact clockwise quarter turns so
pixel geometry stays deterministic.

Post JSON schema v2 stores an optional structured `transform`. The reader
continues to accept schema-v1 placements and translates legacy `crop` to a
zero-rotation transform in memory. A v1 file is never rewritten merely because
it was read. A successful authoring apply may upgrade the touched post to v2;
the old panorama crop must resolve to exactly the same pixel rectangle. Unknown
schema versions or conflicting legacy and v2 transform fields fail closed.

The authoring manifest records both the exact source-specification SHA-256 and a
canonical authoring-input fingerprint over placement identity, template/slot,
asset identity, and source evidence. Apply rejects a changed post or source.
After a successful apply, an identical report is an idempotent success even
though the post file hash changed; a different report requires a newly prepared
session. The project-owned post retains the durable transform, while immutable
render/publication snapshots later preserve the exact resolved geometry and
checksums in Neon.

The general Photoshop session authors in the real target composition whenever
practical. One manifest represents every unresolved placement in a post or
package; Photoshop steps through placements in deterministic item/slot order
and writes one report; one Rust apply operation validates and persists all
results atomically. A failure leaves the post unchanged and reports the exact
placement that needs attention.

A dual-platform authoring manifest may pin one secondary platform
specification and carry a separate ordered placement set for it. Photoshop
opens both platform context sets in one session. Apply validates both
specification fingerprints and every source/aspect result, then writes each
platform's independently authored transforms to its own project post. An
interrupted second write is recoverable by replaying the same report;
already-applied transforms are idempotent. No Instagram crop is copied into a
Threads placement.

#### Ordered implementation plan

Each step begins only after the prior gate passes. Patches remain small enough
to test and review independently.

1. **Audit and document the panorama contract — complete.** Record current
   model, scripts, fingerprints, persistence, HDR/SDR geometry, render behavior,
   affected files, and regression fixture in this roadmap.
   **Gate:** the architecture decision above agrees with `src/layout.rs`, the
   panorama scripts, `Build Photara Layouts.psjs`, templates, and migrations.
2. **Add the smallest generalized model — complete.** Add `PlacementTransform` and
   platform-neutral authoring manifest/report types without yet replacing the
   panorama commands.
   **Gate:** unit tests cover identity transform, normalized crop validation,
   quarter-turn validation, rotation-before-crop semantics, and deterministic
   serialization.
3. **Prove schema and persistence compatibility — complete.** Add schema-v1 reading and
   schema-v2 writing rules, canonical authoring-input fingerprints, atomic
   project JSON updates, and explicit conflict handling. Add a migration only
   if immutable snapshot/evidence tables are needed; do not move editable
   placement state into Neon.
   **Gate:** existing v1 Instagram JSON resolves without being rewritten, and
   invalid/conflicting versions fail closed.
4. **Render persisted rotation — complete.** Carry the transform through resolution and
   render manifests, apply the same quarter-turn before crop to HDR and SDR in
   Photoshop, and verify final pixel bounds.
   **Gate:** fixture tests cover 0°, 90°, 180°, and 270° with matching HDR/SDR
   geometry and no implicit stretch.
5. **Migrate `continuous-panorama@1` — complete.** Route panorama prepare/capture/apply
   through the generalized contract while retaining compatible operator entry
   points until the replacement is proven.
   **Gate:** the DSC05417 authored crop resolves to its existing exact pixel
   rectangle and two-frame seam; no re-authoring is required.
6. **Freeze the Instagram regression — complete.** Capture the accepted Package A
   resolved/render manifest as a fixture and compare item order, 18 editorial
   outputs, 20 delivery frames, template versions, canvas/slot/crop pixel
   geometry, rotation, and source checksums.
   **Gate:** pre- and post-generalization geometry is identical, apart from
   explicitly versioned metadata fields.
7. **Add one multi-placement authoring session — complete.** Prepare all unresolved
   placements, author them in deterministic target contexts, emit one report,
   and validate/apply atomically with clear resume behavior.
   **Gate:** a synthetic multi-item post proves ordered traversal, partial-report
   rejection, stale-source rejection, stale-post rejection, and idempotent
   replay.
8. **Define the Red Meridian Threads package — complete.** Add independent 4500×8000
   templates and a project-owned 17-item Threads post. Reuse asset identity and
   editorial intent, never Instagram crop geometry. A no-gutter three-stack
   uses exact parent rows 4500×2667, 4500×2666, and 4500×2667; any future
   gutters or margins must be subtracted from the parent before row allocation.
   **Gate:** validation resolves all 17 items, all 12 source assets, intentional
   repeats, exact slot bounds, and the order below.
9. **Author unresolved Threads transforms — complete.** Use the multi-placement session
   for every 9:16 full frame, every independent stack slot, and the two rotated
   full frames. Resolve comparison TIFFs with contain fit against their actual
   taller templates; author a comparison crop only when explicitly requested.
   **Gate:** no Threads placement remains unresolved and every transform has
   current source/post fingerprint evidence.
10. **Render and validate Threads — complete.** Build all HDR-over-SDR PSBs, review them,
    run WSP manually, and verify ordered delivery outputs and checksums.
    **Gate:** every item has valid source, template, PSB, WSP, and output
    evidence, with identical non-HDR-variable geometry.
11. **Complete backup delivery and publication — complete for Red Meridian.**
    Both packages were manually published and recorded through guarded manual
    confirmation. Their 37 exact WSP JPEGs were uploaded through signed,
    non-overwriting Cloudinary requests and then downloaded and SHA-256
    verified. Immutable per-batch manifests and database evidence connect each
    backup object to its source file without assigning website order.
    **Gate:** both platform packages have durable Cloudinary and publication
    evidence connected to immutable snapshots. **Passed.**
12. **Prove release-scope recovery and idempotency — complete.** Exercise
    interruption, retry, stale
    manifests, repeated reconciliation, cleanup guards, and second-machine/NAS
    remount recovery. Temporary outputs are removed only after durable evidence.
    Repeated Red Meridian preparation reused both batch IDs; repeated upload
    created zero objects and reused all 20 Instagram plus 17 Threads assets.
    **Gate:** repeats create no duplicate assets, masters, snapshots, provider
    objects, or publications, and operator recovery steps are documented.
13. **Close the release documentation — complete.** The
    photographer-facing Sylvan runbook now follows the complete current
    workflow from an untagged Lightroom import through Pixieset, Photographer
    Final, Cloud DNGs, paired masters, both social packages, WSP, Cloudinary,
    and publication evidence. Reconcile `LAYOUTS.md`, `METADATA.md`, the
    operator reference, `CHANGELOG.md`, and the handoff as Sylvan proves or
    corrects each step.
    **Gate:** Red Meridian completes end to end, Sylvan completes after the
    discovered generic defects are fixed and the stabilized workflow is rerun,
    and release checks are green. **Passed.**
    Future licensing/productization review is explicit but does not block this
    reusable personal/operator release.
14. **Prove reusable four-image grids with Sylvan — complete.** Add immutable
    `grid-four@1` Instagram geometry
    with four exact 2250×3000 cells and `grid-four-threads@1` with four exact
    2250×4000 cells. A flattened TIFF may have any authored aspect. Generalize
    every ordinary placement—not only grid cells—to an explicit `fill`,
    `contain`, or operator-authored `crop` policy. Aspect mismatch alone never
    requires authoring; only unresolved `crop` placements do. Instagram and
    Threads always retain independent transforms.
    **Gate:** Sylvan prepares, captures, applies, renders, and reviews both
    packages from one dual-platform crop session without altering either accepted Red
    Meridian package. **Passed.** Additional real projects are optional
    hardening unless they reveal an actual validation or recovery failure.
15. **Generalize Instagram package length — complete.** Treat the platform's
    current 20-frame carousel capability as a maximum, accept every positive
    package length through that maximum, and keep Red Meridian's exact
    20-frame expansion as fixture-specific regression evidence.
16. **Stabilize the release candidate — complete.** Restrict placement
    authoring to unresolved `crop` slots, add explicit item-scoped
    `--reauthor`, expose interactive master-command progress on stderr without
    contaminating structured stdout, and preserve publication-order prefixes
    in delivery evidence.

Sylvan completed the operator proof with 10 Instagram and 14 Threads frames.
Both packages were rendered, reviewed, manually published, reordered in their
project specifications to match the authoritative numbered publication files,
backed up to Cloudinary, and downloaded for exact SHA-256 verification. Both
manual publications have durable evidence tied to the final specification
hashes. This closes the `0.1.0` release gate.

Red Meridian remains a valid completed project and the authoritative regression
fixture. After Sylvan proves the current catalog and layout behavior, run only
non-destructive reconciliation and regression checks against Red Meridian.
Do not add the new grid to, reorder, rerender, or republish either accepted
package merely to adopt newer machinery.

#### Red Meridian Threads package order

All canvases are 4500×8000 (9:16). “Stack” means three independently authored
source placements in one parent canvas. Repeated assets are intentional.

| # | Editorial item |
| ---: | --- |
| 1 | DSC05250 hero, authored 9:16 |
| 2 | Stack: DSC05445, DSC05442, DSC05441 |
| 3 | DSC05217, authored 9:16 |
| 4 | DSC05406, authored 9:16 |
| 5 | DSC05382, authored 9:16 |
| 6 | DSC05409, authored 9:16 |
| 7 | Stack: DSC05417, DSC05419, DSC05382 |
| 8 | DSC05421, authored 9:16 |
| 9 | Stack: DSC05382, DSC05372, DSC05441 |
| 10 | DSC05372, authored 9:16 |
| 11 | DSC05421 repeated, independently authored 9:16 |
| 12 | DSC05250 + DSC05421 Dynamic Range Comparison, 9:16 design |
| 13 | DSC05250 + DSC05421 Edit Comparison, 9:16 design |
| 14 | DSC05417, rotate 90° clockwise, then author 9:16 crop |
| 15 | DSC05445, rotate 90° clockwise, then author 9:16 crop |
| 16 | DSC05445 + DSC05417 Dynamic Range Comparison, 9:16 design |
| 17 | DSC05445 + DSC05417 Edit Comparison, 9:16 design |

#### Release-wide acceptance criteria

- Every representation and output has verifiable provenance and checksums.
- Reconciliation is restart-safe and creates no duplicate database records,
  Adobe collections/assets, authoritative masters, layouts, Cloudinary objects,
  or publication evidence.
- WSP remains an adapter. Photoshop-assisted operation is supported; Photara
  core does not depend on undocumented WSP internals. A headless adapter may be
  added later if a stable supported interface exists.
- The photographer guide follows the real workflow with copyable commands and
  observable success states. A separate operator reference covers credentials,
  migrations, provider limitations, evidence, retries, audits, and recovery.
- Existing MIT releases remain MIT. Before productization or external
  distribution, deliberately review Photara's commercial boundary and future
  license. This roadmap does not select that license.

### 0.1.1 — legacy workflow maintenance and recovery

`0.1.1` is cut from `v0.1.0` on the dedicated `0.1.x` branch. It preserves the
released workflow and database rather than merging the generation-two tree
back into it. Changes must remain additive, upgrade-safe, and reusable by a
future Lightroom or native UI through structured application-service output.

#### 0.1.1 delivery slices

1. **Cloud no-op reconciliation.** Use one definition of provider-verified
   presence during transfer preview and reservation. Accept either a current
   provider-backed presence record or immutable imported evidence that still
   resolves in the exact Adobe inventory snapshot. Reconcile missing local
   presence rows transactionally. When every Photographer Final asset is
   already present, report `no-transfer-required`, create no upload workload,
   and tell Lightroom that zero DNGs need preparation. A failure must leave no
   partial batch, item, evidence link, or presence update.
2. **Audited client-selection corrections.** Keep Pixieset imports immutable
   and store operator additions/removals in a separate current override table
   plus append-only event history. Add idempotent `selections add`, `remove`,
   `status`, and `history` commands with required reasons, dry-run output, exact
   asset resolution, and explicit cascade behavior. Enforce
   `Hero ⊆ Client Shortlist ⊆ Client Favorites`; Photographer Final remains
   independent. Lightroom reconciliation consumes provider evidence plus the
   local override ledger and reports both direct and effective membership.
3. **Adobe authentication recovery.** Preserve secrets in Keychain, parse and
   sanitize Adobe OAuth error bodies, distinguish revoked/expired grants from
   client configuration and transient provider failures, detect client-ID
   mismatch, persist rotated refresh tokens atomically, and provide an
   actionable `adobe-status`/`adobe-doctor` path. Never print authorization
   codes, access tokens, refresh tokens, or client secrets.
4. **Observable health and configuration.** Successful `health` and
   `config validate` commands emit explicit human-readable or structured
   results for configuration, database, credentials, storage roots, Adobe
   connectivity, and installed integration versions. Remove misleading
   connection-string warnings without weakening TLS requirements.
5. **Self-contained Lightroom installation.** Package the exact release
   plug-in and add install, status, verification, and uninstall commands for
   Lightroom Classic's native Modules directory. Installation is atomic,
   version-aware, checksum-verified, and recoverable; it never symlinks into a
   mutable source checkout. Detect incompatible CLI/plug-in versions clearly.
6. **Documentation and release engineering.** Add a concise `0.1.x` operator
   guide covering reauthentication, integration installation, selection
   corrections, XMP persistence, backups, retries, and recovery. Test a fresh
   install and an in-place `v0.1.0` migration, mock OAuth failure classes,
   exercise selection add/remove/restore and Cloud no-op idempotency, run the
   existing Red Meridian/Sylvan regressions, and publish the CLI, Lightroom
   bundle, checksums, and release notes together under tag `v0.1.1`.

#### 0.1.1 exclusions

- No generation-two Core, node-runtime, bridge, SwiftUI, or database redesign.
- No new publishing provider, layout system, visual editor, or product GUI.
- No undocumented Adobe endpoint and no attempt to bypass Adobe approval.
- No wholesale merge of `0.1.x` into `main`; reusable work is salvaged later as
  reviewed application services or individual commits.

### 0.2.0 — operator experience and visual authoring

- Improve the existing CLI, Lightroom Classic adapter, Photoshop workflow, and
  progress/recovery experience based on the completed `0.1.0` workflow.
- Replace scattered Lightroom Classic menu commands with one Photara plug-in
  UI that presents project context, progress, recovery, and the next applicable
  action.
- Add richer visual placement authoring and a persistent Photara Photoshop UXP
  panel. First generalize the proved scripts—including the shared 16-bit source
  to HDR/SDR master-preparation action—behind that panel without changing their
  validated workflow contracts.
- Prototype a guided installer/onboarding experience that installs and verifies
  both Adobe plug-ins, connects Lightroom Desktop/Cloud, locates or creates the
  Lightroom archive, establishes Photara account/database access, and reports
  every required permission and connection as an observable status. Decide
  whether production data is Photara-managed or bring-your-own Neon before
  exposing database setup to photographers.
- Generate color-managed project thumbnails for the Photara UI and establish
  an HDR-capable preview contract with a tested SDR fallback.
- Keep Pixieset behind a provider adapter. Its currently documented supported
  photographer workflows are the Lightroom Classic publish plug-in and
  Favorites/CSV handoffs; do not depend on an undocumented private web API.
- Reduce command and script ceremony without moving workflow policy out of
  Photara's shared application services.
- Improve Adobe authorization, reliability, cancellation, retry reporting, and
  account lifecycle using real operator evidence.
- Do not prematurely commit to a standalone desktop architecture. Choose the
  eventual product surface only after the reusable workflow and visual
  authoring needs are proven.

### 0.3.0 — personal production maturity

- Generalize from Red Meridian, Sylvan, and additional real shoots.
- Harden reliability, layout/template evolution, project/post/publication
  lifecycle, delivery adapters, diagnostics, and operator experience.
- Measure repeated hashing, database, provider, and rendering costs; reuse
  immutable verification checkpoints where safe.
- Add granular Lightroom named-version and Photoshop revision lineage when real
  recovery needs justify it.
- Do not add speculative AI or agent features before the production workflow
  identifies a concrete need.

### 0.4.0–0.9.0 — productization and external readiness

- Define packaging, installation, updates, background jobs, notifications,
  onboarding, support, privacy, terms, data export/deletion, and clean
  uninstall behavior.
- Decide the long-term desktop, Lightroom, and Photoshop surfaces from tested
  operator workflows; keep the CLI as a supported automation and diagnostic
  interface.
- Prepare Adobe production review only when the complete application is
  reviewer-ready. Treat OAuth approval and Lightroom Connection recognition as
  separate verified outcomes.
- Test clean accounts, consent, refresh, revocation, relogin, pagination,
  entitlement/storage failures, interrupted synchronization, duplicate names,
  and idempotent recovery with multiple beta users.
- Define a target-neutral artifact contract before adding website, Codexa,
  Loomara, or additional publication integrations. Photara remains the
  photography source of truth.
- Establish public identity, documentation, release engineering, signed and
  notarized artifacts where applicable, CI evidence, and a deliberate license
  decision before external distribution.

### 1.0.0 — public or commercial release target

The exact `1.0.0` contract will be set from evidence gathered through
`0.1.0`–`0.9.0`. It requires a stable supported workflow, migration and recovery
policy, provider boundaries, installation/update story, documentation, support
expectations, privacy terms, and an explicit license/product decision. Do not
speculate beyond those gates now.
