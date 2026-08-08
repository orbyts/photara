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

- Take the 14 Red Meridian Photographer Final assets through manual Lightroom
  Desktop and Photoshop editing as the first complete master workflow.
- Register layered PSBs beside their corresponding camera RAWs without moving
  or copying either representation.
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
- Complete and verify all 14 Red Meridian master chains with no permanent local
  DNGs.

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
- Decide Photara's commercial product boundary and future license before
  publishing 0.1.0. Existing MIT releases remain MIT; do not automatically
  publish Photara 0.1.0 to crates.io until this decision is complete.

## 0.2.0 — product experience, website artifacts, and performance

- Add the standalone Photara desktop experience, installer-managed Lightroom
  plugin, background jobs, notifications, account management, and onboarding
  for nontechnical photographers.
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
- Profile CLI startup, database round trips, serialization, and Lightroom
  catalog matching; publish latency budgets for interactive actions.
- Add an optional local read-through cache for provider inventory snapshots and
  computed reconciliation plans, invalidated by account, catalog, and snapshot
  hash. Adobe and PostgreSQL remain authoritative; cached state must always be
  disposable and must never become a second operational ledger.

## Later

- Granular Lightroom named-version and Photoshop revision lineage.
- Historical Proetus adoption at scale.
- Additional publication, delivery, content, and website targets after their
  artifact contracts are proven independently.
