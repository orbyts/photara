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

### 0.0.2 — project foundation

- Load typed non-secret configuration from `$XDG_CONFIG_HOME/photara`.
- Keep secrets in environment variables supplied by any secret manager.
- Own and run Photara migrations through Storexa.
- Create idempotent project records and project directories.
- Initialize and verify the `red-meridian` project.

### 0.0.3 — asset identity and authoritative representations

- Add assets, project membership, asset files, fingerprints, and provenance.
- Preserve original RAW filenames and archive paths.
- Expand names only for selected downstream representations.
- Add collision detection that never overwrites files.

### 0.0.4 — Lightroom Cloud baseline

- Add Adobe authorization and a read-only Lightroom Cloud adapter.
- Import the existing 1,520-asset inventory before enabling uploads.
- Persist Adobe catalog IDs, asset IDs, SHA-256 values, and import runs.
- Enforce database uniqueness and produce dry-run reconciliation reports.

### 0.0.5 — thin Lightroom Classic plugin

- Scaffold `photara.lrplugin` in Lua.
- Reconcile project Collection Sets and Smart Collections without duplicates.
- Apply project metadata and hierarchical workflow keywords.
- Preserve metadata in XMP sidecars and keep business logic in Rust.

### 0.0.6 — client-selection workflow

- Export temporary Pixieset proofs while retaining original filenames.
- Reconcile favorites and shortlist results back to source assets.
- Apply client-favorite, client-shortlist, and photographer-final keywords.

### 0.0.7 — guarded Lightroom Cloud delivery

- Generate selected DNGs with expanded basenames.
- Plan uploads against the complete Cloud inventory.
- Reserve operations transactionally and treat duplicates as already present.
- Remove temporary local DNGs only after Cloud presence is verified.

### 0.0.8 — PSB and flattened-master workflow

- Register layered PSBs beside original RAWs.
- Register flattened TIFFs in project directories.
- Verify authoritative location and provenance without permanent copies.

### 0.0.9 — publication workflow

- Add WSP, Cloudinary, and social-publication adapters.
- Add retryable publication operations and a photography ledger.
- Define the handoff contract consumed later by Codexa.

### 0.1.0 — first supported release

- Complete the Red Meridian vertical slice.
- Prove repeated reconciliation creates no duplicate records, collections, or
  Lightroom Cloud assets.
- Stabilize migrations, configuration, recovery, CI, and operator docs.

## After 0.1.0

- Granular Lightroom named-version and Photoshop revision lineage.
- Historical Proetus adoption at scale.
- Layout generation and richer Codexa integration.
