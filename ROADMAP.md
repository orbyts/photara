# Photara Roadmap

Photara is the operational brain of a photography workflow. Storexa provides
database infrastructure; Photara owns schemas, repositories, reconciliation,
and every photography-specific decision.

Only `0.1.0` is intended for the next crates.io publication. Versions `0.0.1`
through `0.0.9` are Git checkpoints and may be combined when work lands
together naturally.

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
  stack them with their camera originals, apply additive project membership,
  and use Lightroom's native file type for master smart collections; do not
  require a user to synchronize an entire filesystem folder.
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

### 0.0.9 — layouts, HDR export, and publication

- Add a versioned project-owned JSON layout specification that references
  stable Photara asset IDs rather than filenames.
- Support one or more flattened TIFFs in a frame with explicit canvas, aspect
  ratio, crop, scale, translation, rotation, stacking order, and output intent.
- Model many-to-one provenance so a single published frame can derive from
  multiple edited assets.
- Validate layouts independently from rendering and support a manual
  JSON-plus-preview authoring loop for the 0.1.0 vertical slice.
- Model SDR-base and HDR-layer inputs for HDR gain-map output.
- Treat Web Sharp Pro as an adapter: support the current Photoshop-assisted
  workflow and add a headless adapter later if a stable CLI becomes available.
  Do not make Photara core depend on Photoshop or WSP internals.
- Keep libvips or Sharp available as alternative rendering adapters where they
  can satisfy the output contract.
- Add Cloudinary archival/delivery and Instagram and Threads publication
  adapters, with a manual-posting fallback when an API cannot preserve required
  media capabilities.
- Add retryable publication operations and an evidence-backed photography
  ledger. Mark media published only after a provider receipt is stored or the
  user explicitly confirms a manual publication.
- Clean temporary publication outputs only after delivery evidence is durable.
- Keep website generation, Codexa, Loomara, and a polished visual layout editor
  outside the 0.1.0 scope.

### 0.1.0 — first supported release

- Complete the Red Meridian vertical slice from RAW ingest, metadata, Pixieset
  selections, Photographer Final, Lightroom Cloud DNG, Lightroom Desktop edit,
  layered PSB, flattened TIFF, layout, and HDR gain-map output through verified
  Instagram and Threads publication.
- Prove repeated reconciliation creates no duplicate records, collections, or
  Lightroom Cloud assets, authoritative masters, layouts, or publications.
- Require verifiable provenance for every representation and evidence-backed
  publication state.
- Keep temporary-file cleanup guarded, restart-safe, and independently
  repeatable.
- Stabilize migrations, configuration, recovery, CI, and operator docs.
- Ship a concise photographer-facing CLI guide that follows the real workflow:
  Lightroom Classic import, recent-import selection, people/location/scene
  creation, project initialization, whole-shoot metadata and XMP, proofing and
  selections, Photographer Final, Cloud editing, masters, layouts,
  publication, reversals, and recovery. Commands should be copyable and each
  step should say what success looks like.
- Keep a separate operator reference for configuration, credentials,
  migrations, provider limitations, evidence, retries, audits, and disaster
  recovery; do not burden the photographer guide with infrastructure details.
- Decide Photara's commercial product boundary and future license before
  publishing 0.1.0. Existing MIT releases remain MIT; do not automatically
  publish Photara 0.1.0 to crates.io until this decision is complete.

## 0.2.0 — product experience, website artifacts, and performance

- Ship Photara as a fully realized desktop product rather than asking users to
  assemble a CLI, Lightroom plug-in, configuration files, and credentials.
- Add the standalone Photara desktop experience with an installer-managed
  Lightroom plug-in, background jobs, progress and notifications, account
  management, recovery, updates, and guided onboarding for nontechnical
  photographers. Keep the CLI as a supported automation and diagnostic
  surface backed by the same application services.
- Bundle one persistent Photoshop UXP panel that presents the applicable next
  action for the selected Photara project, including master creation,
  readiness checkpoints, flattening, verification, retries, and status. Users
  must not need to locate or remember individual `.psjs` files.
- Replace the Lightroom Classic plug-in's command-oriented menus with a guided
  project UI for metadata, selections, Cloud presence, withdrawals, master
  handoffs, progress, and recovery. Keep Lua as a thin host adapter; workflow
  policy and persistence remain in shared Photara application services.
- Make ordinary user-owned Lightroom Albums and provider-owned Connections an
  explicit product boundary. Photara must never imply that Adobe partner
  `project`/`project_set` records are the user's normal Albums hierarchy.
- Prepare and submit Photara as an Adobe production integration only when the
  desktop application and its Adobe workflow are reviewer-ready. Approval and
  recognition as a Lightroom Connection are release goals, not assumptions.
- Establish the Orbyts public product identity: stable Photara name and icon,
  organization profile, marketing website, support contact, privacy policy,
  terms of use, and clear data export/deletion documentation. Keep Adobe marks
  out of Photara branding except where their published guidelines permit them.
- Package a reproducible Adobe review candidate with signed/notarized macOS
  artifacts where applicable, a bundled Lightroom plug-in, clean install and
  uninstall paths, supported-version declarations, a tagged source revision,
  and CI evidence from a clean build.
- Produce an Adobe reviewer packet containing a concise walkthrough video,
  exact installation and test steps, a small non-sensitive sample project,
  expected results, requested-scope justifications, known provider
  limitations, disconnect/deletion behavior, and reviewer support contact.
- Complete the Adobe account lifecycle in-product: PKCE authorization,
  encrypted operating-system credential storage, refresh-token rotation,
  provider-side token revocation on disconnect, local credential removal,
  reconnection, and an understandable retained-evidence policy.
- Add a bounded Adobe HTTP reliability layer with short-lived disposable
  inventory caching, request de-duplication, rate-limit handling, exponential
  backoff, at most three eligible retries, cancellation, progress reporting,
  and actionable errors. PostgreSQL and Adobe remain authoritative.
- Exercise the complete Adobe workflow with multiple beta users and clean
  accounts before review. Test consent, refresh, revocation, relogin,
  entitlement and storage failures, empty catalogs, pagination, duplicate
  filenames, interrupted synchronization, and idempotent recovery.
- Track Adobe review state and reviewer feedback as release evidence. Do not
  claim that OAuth production approval automatically makes Photara visible in
  Lightroom Connections; verify Connection recognition separately with Adobe.
- Publish parallel GUI and CLI photographer guides generated from the same
  versioned workflow documentation so their terminology and outcomes cannot
  drift.
- Add visual layout authoring and website-specific photography layouts without
  coupling social layouts to website presentation.
- Define a versioned, target-neutral Photara artifact contract for media,
  layouts, attribution, accessibility, visibility, and provenance.
- Allow Photara artifacts to flow directly to Loomara or through a Codexa
  content projection. Photara remains the photography source of truth; Codexa
  compiles Git-native structured content; Loomara owns website assembly,
  rendering, application integration, and deployment.
- Keep Photara independent of Loomara's website framework and keep Codexa free
  of media processing or website rendering responsibilities.
- Profile desktop and CLI startup, database round trips, serialization,
  provider calls, and Lightroom catalog matching; publish latency budgets for
  interactive actions.
- Reuse immutable verification checkpoints across dry runs and confirmed
  writes so large PSBs and TIFFs are hashed once per file revision rather than
  once per command or database row.
- Extend the Adobe review cache into an optional provider-neutral read-through
  cache for inventory snapshots and computed reconciliation plans, invalidated
  by account, catalog, provider cursor, and snapshot hash. Cached state must
  always be disposable and must never become a second operational ledger.

## Later

- Granular Lightroom named-version and Photoshop revision lineage.
- Historical Proetus adoption at scale.
- Additional publication, delivery, content, and website targets after their
  artifact contracts are proven independently.
