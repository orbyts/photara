# Changelog

All notable changes to Photara will be documented in this file.

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
