# Photara Theme Lab

Theme Lab is the single editor for the shared paired Light/Dark palette. Read
[`../DESIGN_LANGUAGE.md`](../DESIGN_LANGUAGE.md) and the
[UI0 contract](../../../docs/architecture/UI_LADDER_AND_AUTHORING_SEQUENCE.md).

The editor presents Foundation, Primary and Inset together, followed by the photograph
reference surround, text, borders/custom focus, custom selection, semantic status and
small category affordances. Every field shows both appearances. Compatibility roles
are read-only aliases; exports contain 25 authored slots rather than 32 duplicate
values. Imported schema-1 literal aliases are discarded in favor of their canonical
owner. Unknown roles are rejected. Imported chromatic/low-contrast authored values are
retained with warnings; Apply requires a neutral ordered ladder and passing contrast.

The shared `ThemeLadderSpecimen` renders both appearances together. Production Opening
uses the exact `ApplicationShell` and `OpeningLibraryView` sources with fixture actions.
Opening uses native macOS window/text colors; editing the ladder does not repaint it.
No Rust bridge, database or authentication driver is instantiated. The historical
`GlassTestScene.swift` is retained but is neither compiled nor exposed by Theme Lab.
Native materials, vibrancy, sidebar selection and Liquid Glass optics are not tokens.

```sh
platform/macos/photara-theme/build-theme-lab.sh
open 'platform/macos/photara-theme/.build/Photara Theme Lab.app'
platform/macos/photara-theme/.build/photara-theme validate /path/theme.json
platform/macos/photara-theme/.build/photara-theme use /path/theme.json
platform/macos/photara-theme/.build/photara-theme reset
```

Open/Save edit portable sRGB JSON. Apply writes a development override that Photara,
Shell Lab, Graph Lab and the feature labs consume read-only through `PhotaraThemeStore`.
Ordinary editing changes only this lab's document until Apply or deliberate promotion
of the bundled resource. Remove Theme Override affects only Theme. Verification hosts
explicitly disable development overrides so captures remain deterministic.

Theme/presentation preferences never enter Core, Project packages, graph digests or
cloud synchronization. Geometry belongs to Shell; feature content belongs to its typed
module. Graph's accepted controls and interaction machinery remain Graph-owned.
