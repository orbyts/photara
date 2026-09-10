# Photara Gallery Lab

Independent native authoring host for `../photara-gallery`. It compiles the exact
Gallery view, cards, justified photo layout, square grid, activity badges and
full-image sheet used by Photara, with the shared native HDR renderer. No bridge,
project, NAS or network is required.

```sh
platform/macos/photara-gallery-lab/build-gallery-lab.sh
open 'platform/macos/photara-gallery-lab/.build/Photara Gallery Lab.app'
platform/macos/photara-ui-tests/verify-shared-ui.sh
```

Fixtures cover portrait/landscape/square, SDR and true float HDR highlights,
multiple representations, loading/updating/failed previews and selection. Use
the production controls to change grid style, filter, resize, select and View.
The sidebar switches appearance, compatible assignment context and empty state.
Open/Assign callbacks are recorded rather than touching real files or projects.

The sidebar's authored thumbnail default, gaps and selection stroke update the
shared view live. **Export Shipped Preset** validates and exports JSON; promote
to `../photara-gallery/Resources/photara-gallery-presentation-v1.json` and rebuild
both hosts. Unsaved experiments are local to this lab session. Colors are shared
theme roles. Inspector and shell do not need copies of this preset.

On an HDR display, select **Portrait HDR** or **Landscape HDR** and press **View**.
The exact same NSImage is presented with `.constrainedHigh` in Gallery and `.high`
in the focused sheet. The fixture has an extended-linear Display P3 white patch
at 4× reference white, paired with SDR examples. The automated check verifies
float values and native policy; judge display luminance on hardware.

See [shared architecture and handoff](../SHARED_UI.md).

For the next Gallery task: own `photara-gallery/Sources`, its shared preset, and
this lab's host/fixtures. Keep semantic actions in `ProductionGalleryView.swift`.
Run this lab and `../photara-ui-tests/verify-production-ui.sh` before handoff.
Finish with the commit, promoted preset changes, and verification results;
`platform/macos/build-ui.sh` assembles the result into Photara.
