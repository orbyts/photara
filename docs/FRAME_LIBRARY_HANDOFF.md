# Modular frame and local Library handoff

> Historical frame/Library implementation handoff. For current work, begin with
> [Active handoff](ACTIVE_HANDOFF.md) and the
> [Photara 0.2 execution roadmap](ROADMAP_0_2_EXECUTION.md). Preserve the work
> recorded here, but do not use its next-step guidance to bypass the current
> Architecture and Database Schema approval gates.

This change integrates the previously uncommitted Shell/Gallery/Inspector
preset and empty-state authoring work with the new module frame and local
Library. Graph and Graph Lab source/resources remain unchanged.

## Working model

- Rounded filled surfaces use a semantic Light/Dark canvas, gutters, contained
  icon headers, independent scrolling and native window/title-bar behavior.
- Shell Lab → Application Surfaces → **Global module corner radius** authors one
  radius for all modules. Canvas/material/fill, insets, gutter, border/elevation,
  emphasis, header treatment and bottom/status shape are authorable there too.
- Workspace menu/toolbar restores every module, including Graph and optional
  Work Surface. Account, People, Locations and Scenes have direct shortcuts.
  Visibility/placement preferences migrate without discarding older placements.
- New Project reveals Project Info. Assign from Library searches reusable records;
  Create New uses the shared feature editor and creates/assigns a missing record.
- People includes clients (individuals or organizations) and collaborator roles.
  Locations support parent/sub-location relationships. Scenes are reusable;
  assigning the same scene twice creates two distinct project occurrences.
- Entries have selectable thumbnails and record-specific fallbacks. The host
  prepares small PNGs, stores media by digest, and retains only portable thumbnail
  identities in Library records. Gallery HDR proxies use their existing path.
- Local SQLite records have owner scopes, revisions, tombstones and change cursors.
  Project references retain historical names/revisions and occurrence details;
  save/reopen and session assignment undo use the portable Project Document.

## Build and run

Run from `/Users/suhail/Library/CloudStorage/Dropbox/matrix/crates/photara`.

```sh
# All eight labs plus production
platform/macos/build-ui.sh

# Independently runnable module labs
platform/macos/photara-people-lab/build-people-lab.sh
open 'platform/macos/photara-people-lab/.build/Photara People Lab.app'
platform/macos/photara-locations-lab/build-locations-lab.sh
open 'platform/macos/photara-locations-lab/.build/Photara Locations Lab.app'
platform/macos/photara-scenes-lab/build-scenes-lab.sh
open 'platform/macos/photara-scenes-lab/.build/Photara Scenes Lab.app'
platform/macos/photara-project-info-lab/build-project-info-lab.sh
open 'platform/macos/photara-project-info-lab/.build/Photara Project Info Lab.app'

# Global frame authoring and production integration
platform/macos/photara-shell-lab/build-shell-lab.sh
open 'platform/macos/photara-shell-lab/.build/Photara Shell Lab.app'
platform/macos/photara-app/build-app.sh
open 'platform/macos/photara-app/.build/app/Photara.app'
```

Each `open` path is the exact generated app bundle. Executables live inside
`Contents/MacOS`, named `PhotaraPeopleLab`, `PhotaraLocationsLab`,
`PhotaraScenesLab`, `PhotaraProjectInfoLab`, `PhotaraShellLab`, and `Photara`.

Labs compile the same production sources, with deterministic populated, empty,
loading and error fixtures. Project Info composes the same feature field editors.
Source changes take effect in both lab and production after rebuilding.

Shell Lab saves its draft automatically. **Apply to Photara** installs validated
Shell/Theme development overrides; Photara polls them every 500 ms. **Remove
Photara Override** restores bundled values. To change shipped defaults, export
the validated Shell preset to
`platform/macos/photara-shell/Resources/photara-application-presentation-v1.json`,
review the diff and rebuild. Gallery and Inspector retain their own existing
preset/draft/export/override workflows.

## Verification

- `cargo fmt --all --check`
- `cargo test --workspace` — 64 tests passed, plus doc tests.
- `cargo clippy -p photara-library --all-targets -- -D warnings`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `platform/macos/build-ui.sh` — all eight labs and Photara built.
- `platform/macos/photara-app/verify-bridge.sh` — passed.
- `platform/macos/photara-ui-tests/verify-shared-ui.sh` — passed; inspected native
  Light/Dark module captures. Existing float/HDR policy checks remain intact.
- `platform/macos/photara-ui-tests/verify-production-ui.sh` — passed, including
  local thumbnails, assignments, stale revisions, undo and snapshot save/reopen.
- Production app launched successfully from its signed bundle in the background;
  no foreground UI automation was required.

Shared captures live in `platform/macos/photara-ui-tests/.build/snapshots`;
production captures live in `.build/production-snapshots`. Generated bundles,
module caches and captures are ignored by Git. The New Project assertion now
requires Graph plus Project Info, matching the requested assignment flow. The
old mutually exclusive Graph/Work Surface assertion now tests independent
visibility. Existing HDR tests have not been relaxed.

Graph's physical pointer/Accessibility suite was not run: Graph implementation
and its lab are unchanged, and this task does not require an uninterrupted
foreground automation session. The Graph lab builds in the assembly and its
unchanged renderer is exercised in native Shell/production captures. Actual HDR
panel luminance still requires a human display check.

## Intentionally deferred

Photara Cloud and iCloud appear as planned/unavailable in Library & Sync.
Production Auth0 PKCE, Photara API/Neon, subscriptions, CloudKit, sync conflict and
mutation-deduplication protocol, media transfer, legacy migration and backup/
recovery UI are not implemented. No cloud database credentials are embedded.

The first Library projection is bounded to 500 records. Paging, cross-project
indexes and advanced docking/tabbing/floating remain later work. Local media
reclamation and durable Project Info undo/redo beyond the current session are
also future work. See [Library architecture](LIBRARY_ARCHITECTURE.md) for schema
and authority boundaries.
