# Photara Shell Lab

Native authoring host for Photara's shared application composition. **Photara —
Shell Preview** renders the exact production `ApplicationShell` and feature views;
**Shell Authoring** contains fixtures and shell-owned controls. No bridge, database,
source folder, provider or network is required.

Read [`../DESIGN_LANGUAGE.md`](../DESIGN_LANGUAGE.md) before changing shared chrome.
It defines the authoritative surface hierarchy, Liquid Glass policy and lab ownership.

```sh
platform/macos/photara-shell-lab/build-shell-lab.sh
open 'platform/macos/photara-shell-lab/.build/Photara Shell Lab.app'
```

## What this lab owns

- Launcher composition, copy, typography, hero treatment and launcher actions.
- Application and project identity in the native toolbar.
- Module placement, visibility, sizing and compact behavior.
- Shared module geometry: gutter, outer/content inset, corner radius, header metrics
  and optional restrained elevation.
- Shared status-bar geometry and integrated empty-project composition.
- Scenarios showing Graph, Gallery, Inspector, Library modules and node work surfaces
  inside the authoritative production frame.

Theme owns the adaptive application base, module base, optional inset-content surface,
selection tint, text and status colors. Shell Lab exposes those shared semantic roles
for convenient live authoring but does not duplicate them in the Shell preset.

The native title bar and toolbar Liquid Glass are system-owned. Photara supplies a
leading project thumbnail/title, centered application identity and at most three
semantic trailing groups. macOS owns material, blur, shadow, item glass, overflow,
contrast, accessibility behavior and future SDK appearance. Do not add title-bar blur,
custom toolbar material or pill-shape controls to this lab.

Every module uses one borderless `surface.panel` frame. Its title is part of that base.
Focused-pane feedback is the shared `selection.background` title tint. There are no
resting module outlines, header separators or split rules; a resize guide appears only
while dragging and fades afterward. Feature modules must not recreate this frame.

## What feature labs own

Graph Lab owns Graph canvas, nodes, ports, connections and its already accepted
floating tool rail/zoom controls. Gallery Lab owns Gallery content, HDR rendering,
layout, cards and Gallery states. Inspector Lab owns Inspector sections, fields and
Inspector states. People, Locations, Scenes and Project Info Labs own their respective
browsers, editors and data-state presentations. Node labs own their optional work
surfaces. Shell Lab hosts these exact sources but does not author their internals.

## Scenarios and identification

Scenarios cover opening/recents, empty project, first node, cleared selection, assets,
Layout capability, Review, evaluation, diagnostics, compact, saved and syncing states.
Fixture actions are recorded rather than touching production state.

Enable **Identify controls in preview** to point at an area and see its owning authoring
section. Preview interaction pauses while identification is active. This overlay is
lab-only and never compiles into Photara.

## Draft, Apply and Export

The lab automatically stores a local authoring draft in `com.photara.shell-lab`.
**Apply to Photara** writes validated Shell and Theme development overrides; a running
development app reloads them. **Remove Photara Override** returns to bundled defaults.
Neither preference enters a Project Document.

**Export Shell Preset** produces the versioned Shell JSON for deliberate promotion to
`../photara-shell/Resources/photara-application-presentation-v1.json`. **Export Shared
Theme** produces the adaptive semantic palette. Review exported diffs and rebuild.

## Verification

```sh
platform/macos/photara-shell-lab/build-shell-lab.sh
platform/macos/photara-ui-tests/verify-shared-ui.sh
platform/macos/photara-ui-tests/verify-production-ui.sh
platform/macos/photara-app/verify-bridge.sh
```

Run Graph's interaction suite only when shared assembly changes touch its interaction
surface. A completed Shell task hands off source/preset paths, visible behavior and
verification results; production assembly never copies a lab view.
