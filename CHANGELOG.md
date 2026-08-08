# Changelog

All notable changes to Photara will be documented in this file.

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
