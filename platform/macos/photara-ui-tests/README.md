# Shared UI verification

Run `verify-shared-ui.sh` for isolated Gallery/Inspector fixtures and native HDR
policy checks; run `verify-production-ui.sh` for the real production composition
and adapter/Core integration. The latter builds Photara first. Both accept
`--build-only` to compile before an exclusive native-window test session.

These are test executables, not additional design labs. They write native PNGs
and temporary test projects under `.build`, use isolated UserDefaults suites,
and do not modify user projects. Run them separately from Graph's physical
pointer verification to avoid stealing its input focus.

The source compilation boundaries are also checked by building Gallery Lab and
Inspector Lab independently: neither includes generated bridge code, production
models, Rust libraries, or the other feature's views.

UI0 adds canonical and legacy theme parsing, all seven aliases, 50 contrast pairs,
all ordered Shell scenario transitions, the paired ladder specimen, Opening, and UI1 Create Project variants
at narrow/standard sizes in Light/Dark. An external accessibility probe checks the
test app's native hierarchy and dispatches Create/Open through its real buttons.
Test hosts disable developer Theme overrides and use native application launch
registration. Captures allow at most three compositor attempts.

`scripts/verify_ui0_contract.py` guards palette ownership and compares the Graph
sources, preset and behavioral assertions with the UI0 baseline, allowing its three
shared background-consumption edits and the explicit macOS 27 verification launch/
input adapter. The Graph suite runs its exact-coordinate preflight before the matrix. It runs before shared compilation. Use
`scripts/verify_ui0_rasters.py docs/architecture/mockups/ui0` with Python/Pillow
with `--native-opening` to check native Opening flatness and the ladder's exact neutral samples.
Run Graph's unchanged `verify-interactions.sh` separately with exclusive foreground
focus. The [UI0 evidence](../../../docs/architecture/UI_LADDER_AND_AUTHORING_SEQUENCE.md#verification-and-raster-evidence)
records the final runs and the limitations of automated visual/accessibility checks.

The accepted UI1 checkpoint verifies Compact as the single shared default while
retaining all three Lab comparisons. Its native probe types a project name into
the real field and presses Choose/Cancel/Create against fixture callbacks only.
Production creation regression sends the existing intent while `EditorSessionView`
is mounted, so its `projectSetupRequest` observer is actually exercised. This does
not wire the new Create Project sheet into production or implement package creation.
