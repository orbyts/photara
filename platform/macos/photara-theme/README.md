# Photara Theme Lab

Theme Lab owns Photara's shared semantic Light/Dark palette and contrast validation.
Read [`../DESIGN_LANGUAGE.md`](../DESIGN_LANGUAGE.md) before changing surface roles.

The key hierarchy is:

- `surface.canvas`: neutral application base.
- `surface.panel`: common module base.
- `surface.elevated`: optional, genuine inset work/content region.
- `selection.background`: subtle focused-module title tint.
- text and status roles: shared legibility and semantic feedback.

These roles are consumed by every assembled module. Feature labs must not duplicate
them as local module-frame colors. Theme Lab does not own module geometry, feature
content, toolbar grouping or Liquid Glass optics.

The native window toolbar and standard controls inherit Liquid Glass from the current
macOS SDK. The preserved Glass Test Scene is exploratory tooling only; its adjustable
blur, shadow and tint values are not product design tokens and must not be copied into
module backgrounds.

```sh
platform/macos/photara-theme/build-theme-lab.sh
platform/macos/photara-theme/.build/photara-theme validate /path/theme.json
platform/macos/photara-theme/.build/photara-theme use /path/theme.json
platform/macos/photara-theme/.build/photara-theme reset
```

The exported theme is portable sRGB semantic data. Photara's development host can
live-reload it; Project Documents never contain theme or lab preferences.
