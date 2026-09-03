# Codex handoff: Photara

> Historical working document retained as release evidence. The `0.1.0`
> vertical slice is complete. Maintenance checkpoint updated: 2026-09-03.

## 0.1.1 maintenance checkpoint

- Maintenance work is isolated on branch `0.1.x`, cut from `v0.1.0`; do not
  merge the generation-two `main` tree into it.
- Copper Mist is the active operator project. Its accepted live checkpoint is
  24 Photographer Final assets, 47 effective Client Favorites, 23 effective
  Client Shortlist assets, and one Hero.
- All 24 Photographer Final assets resolve in the retained Adobe inventory.
  The no-transfer reconciliation recorded completed audit batch
  `14a48dbe-c301-48c0-95e4-042647f92aba`; it generated and uploaded no DNGs.
- The exact `0.1.1` Lightroom bundle is installed as a real directory in the
  native Modules location. The CLI now owns atomic `plugin install`, byte-level
  `plugin status`, and recoverable `plugin uninstall`; do not restore the old
  repository symlink workflow.
- Pixieset evidence remains immutable. Use `selections add/remove/status/history`
  for audited operator corrections, then run Lightroom's **Apply Imported
  Selections**. Hero implies Shortlist and Favorite; Shortlist implies Favorite.
- Person social accounts use quoted `platform=value` arguments, for example
  `--social "instagram=_kylee_nielsen_"`; never add an `@` unless it is
  intentionally part of the provider value.

## Resume objective

Continue Photara from the completed `v0.1.0` reusable workflow. Red Meridian
remains the accepted regression fixture. Sylvan independently completed the
photographer guide with 10 Instagram and 14 Threads frames, manual publication
evidence, and byte-verified Cloudinary originals. Do not begin the `0.2.0`
architectural refactor without a separate discovery and dependency-mapping
pass.

Read these files before changing code:

1. `ROADMAP.md` — release scope and remaining work.
2. `LAYOUTS.md` — authoritative layout, HDR/SDR, Photoshop, WSP, and Red
   Meridian 20-slot rules.
3. `docs/PHOTOGRAPHER_GUIDE.md` — current Sylvan end-to-end operator runbook.
4. `README.md` — current CLI and lower-level operator workflow.
5. `CHANGELOG.md` — released checkpoints and current documentation work.
6. This handoff — live state and sharp edges which are intentionally too
   temporary for durable product documentation.

## Repository and Git state

- Repository: `/Users/suhail/Library/CloudStorage/Dropbox/matrix/crates/photara`
- Maintenance branch: `0.1.x`, cut from `v0.1.0`.
- Current release under validation: `v0.1.1` on `0.1.x`.
- Earlier stable rollback point: commit `c9ec2db`, tag `v0.0.8`.
- `Cargo.toml` is `0.1.1` on the maintenance branch.
- The `0.1.0` release includes the complete Sylvan generalization proof.
  Preserve later implementation changes; do not reset, restore, clean, or
  replace the worktree wholesale.
- Dropbox has previously produced conflict copies. It is currently safe to
  leave Dropbox running, but inspect any new conflict copy before choosing a
  canonical file.
The Rust layout engine, Lightroom Edit Comparison bridge, generalized
Photoshop scripts, publication and Cloudinary migrations, and template
contracts are all included in `v0.1.0`.

## Live installation and data paths

- CLI: `/Users/suhail/.local/bin/photara`
- User configuration: `$XDG_CONFIG_HOME/photara`
- Secrets/environment: Apogee loads them; never copy secret values into the
  repo or this file.
- Lightroom Classic plug-in source:
  `lightroom/photara.lrplugin`
- Installed Lightroom plug-in: `~/Library/Application Support/Adobe/Lightroom/Modules/photara.lrplugin`
  (an exact bundled copy; verify it with `photara plugin status`)
- Installed Photoshop scripts: `/Users/suhail/Pictures/Photara/Scripts`
  - `Prepare Photara HDR-SDR Master.psjs` is the ad-hoc prototype for the future
    UXP panel action after manual 16-bit cleanup. It must preserve an ordinary
    duplicated/shared embedded `16-bit` Smart Object as `HDR` above `SDR`, open
    Camera Raw Filter for manual SDR authoring, and convert the parent to
    unmerged/unrasterized 32-bit Display P3 Linear before that optional
    authoring checkpoint. Every operation must be rebound to the starting
    document ID so tab changes cannot retarget the script. It does not save.
- Lightroom may catalog authoritative PSBs but must not write catalog metadata
  back into those layered files. Exact PSB membership belongs in Photara's
  read-only custom metadata field, which Lightroom stores only in its catalog;
  visible Masters smart collections intersect that marker with native PSB file
  type. Keep Photoshop as the sole PSB writer, especially over SMB, and use
  Lightroom's Edit Original handoff. The one-time **Reconcile Layered Master
  Collections** action migrates old keyword-driven membership before **Read
  Metadata From File** clears legacy catalog/disk conflicts. `photara masters
  checkpoint PROJECT --asset ASSET` provides targeted recovery/re-registration
  without accepting unrelated PSB drift.
- Ghostty loads the user's full environment, but do not open it merely to run a
  command. Run background commands directly through Codex. If an operator-side
  command truly cannot run there, give the user the exact Ghostty command and
  let the user run it.
- Images root: `/Volumes/whisk/Pictures/Images`
- Projects root: `/Volumes/whisk/Pictures/Projects`
- Red Meridian project: `/Volumes/whisk/Pictures/Projects/red-meridian`
- Lightroom handoff inbox: `/Users/suhail/Pictures/Photara/Inbox`
- Authoritative global templates:
  `/Users/suhail/Library/CloudStorage/Dropbox/Pictures/Photara/Templates`
- Disposable template cache: `~/Library/Caches/photara/templates`

Configuration is XDG-first and can be overridden by environment variables:
`PHOTARA_IMAGES_ROOT`, `PHOTARA_PROJECTS_ROOT`,
`PHOTARA_LIGHTROOM_INBOX`, `PHOTARA_TEMPLATES_ROOT`, and
`PHOTARA_TEMPLATES_CACHE`.

## Product boundaries

Photara owns the photography domain and workflow. Storexa is a separate,
domain-agnostic Rust persistence library. Photara uses Storexa for PostgreSQL
infrastructure but owns its schema, migrations, SQL, repositories, and domain
rules. Neon is the current PostgreSQL provider, not a Photara domain concept.

Authoritative representation homes are deliberate:

- RAW + XMP: NAS Images archive, managed by Lightroom Classic.
- Cloud DNG: Lightroom Desktop/Cloud; no permanent duplicate DNG elsewhere.
- Layered PSB: beside the RAW on the NAS.
- Flattened HDR and SDR TIFF pair: project directory on the NAS.
- Publication working artifacts: project directory and removable only after
  verified delivery evidence.
- Exact WSP HDR JPEG backups: project `workspace/exports/<platform>/<package>`
  plus verified Cloudinary originals. Cloudinary does not own website order.
- Immutable reusable templates: Dropbox template registry, not the project,
  Lightroom Cloud, or NAS.

The database stores logical source keys and provenance, not stale absolute
machine paths. Reusing an asset in multiple projects or multiple slides creates
relationships/placements, not duplicate asset identity.

## Completed workflow before 0.0.9

- Storexa 0.1.0 supplies configuration, PostgreSQL connection/pool, health,
  migrations, transaction wrapper, consistent errors, and tracing.
- Photara has project/person/location/scene configuration and CLI operations.
- Lightroom Classic plug-in applies project metadata and idempotent keyword and
  smart-collection structures, then the user saves metadata to XMP.
- Pixieset CSV client favorites, shortlist, and hero selections were imported.
- Photographer Final is independent of client selections.
- Adobe OAuth native-app flow stores refresh tokens in the keychain. Account
  label is `personal`; catalog id is
  `d27d5d9acd4e4e2fbfe493fe18068253`.
- Legacy Cloud evidence for 1520 Proteus assets was imported and provider
  inventory reconciled. Thirteen Red Meridian DNGs were added; two were later
  withdrawn after the photographer changed the edit, leaving 12 current Red
  Meridian finals. Withdrawals preserved RAW, XMP, database history, and
  transfer evidence while removing only Photographer Final and Cloud Present
  projections after provider verification.
- Layered 32-bit Display P3 Linear PSBs were registered beside their RAWs.
  Each authoritative PSB now has top-level `HDR` and `SDR` Smart Objects or
  groups, with HDR above SDR.
- Paired flattened TIFFs use canonical `_HDR.TIF` and `_SDR.TIF` names and live
  under `red-meridian/masters/flattened`.
- Version 0.0.8 was committed, pushed, and tagged before layout work began.

## 0.0.9 layout architecture already implemented

- Global templates are immutable and versioned. The project post owns platform,
  sequence, assets, per-placement transforms, and repeated placements. Post
  schema v1 remains readable; generalized authoring writes schema v2 with
  normalized crop plus clockwise quarter-turn rotation.
- Instagram authoring canvas is 4500×6000 (3:4).
- Threads is implemented as an independent 17-item 4500×8000 (9:16) post
  specification. Its placements were authored and its 17 WSP outputs verified.
- Every Photoshop handoff document has `HDR` above `SDR` for Web Sharp Pro.
  Annotations are identical between them; only declared HDR-variable image/ramp
  regions may differ.
- WSP performs final resizing and splits continuous panoramas. Photara owns the
  continuous editorial surface, crop, seam intent, output order, and evidence.
- `full-frame@1`, `stacked-two@1`, `stacked-three@1`,
  `continuous-panorama@1`, `dynamic-range-comparison@2` (Instagram),
  `dynamic-range-comparison@3` (Threads), `edit-comparison@1` (Instagram), and
  `edit-comparison@2` (Threads) are implemented.
- Generalized placement authoring now prepares ordered source/target contexts,
  fingerprints the complete input, captures one report, validates every source
  and target aspect, and applies all transforms in one atomic post update.
  Legacy panorama commands route through this contract. Identical report replay
  is idempotent.
- `stacked-three@1` uses exact 2667/2666/2667 Threads rows. The two inspected
  4500×8000 PSDs are installed as immutable comparison references with their
  measured geometry and verified SHA-256 values.
- Photoshop rendering supports a single-item debug manifest:

  ```bash
  photara posts prepare-render red-meridian package-a \
    --platform instagram --item edit-comparison-01
  ```

  Omit `--item` for a production full-package manifest. A complete 18-item
  production manifest has been prepared successfully; regenerate it if the
  source specification or implementation changes before production rendering.
- Full manifest preparation can take several minutes because it re-hashes large
  masters. Incremental fingerprint caching is a known performance follow-up;
  do not weaken verification to make it faster.
- One production preparation exposed repeated hashing across placements. The
  current worktree now deduplicates verification by immutable rendition ID
  within one command, while still checking every unique file's byte size and
  SHA-256. Persistent cross-command fingerprint caching remains deferred.
- `posts add-stacked-two` now accepts `--top-crop-from-item` and
  `--bottom-crop-from-item`. It verifies that the referenced item places the
  same stable asset before reusing its normalized crop.
- `posts reorder` accepts repeated `--item` values and requires an exact
  permutation. Full Instagram render preparation accepts 1 through 20 delivery
  frames. Twenty remains the platform maximum and the exact Red Meridian
  regression-fixture count, not a generic package-size requirement.

### Dynamic Range Comparison contract

- Both rows are square contain-fit cells so portrait and landscape images use
  the same surface.
- Left SDR image is the same SDR rendition in the SDR base and HDR top.
- Right image is SDR in the base and HDR in the top.
- Standard 0–1 ramp is identical.
- SDR base headroom ramp is flat white (`1 → 1`); HDR top is the true `1 → 10`
  ramp. This makes actual display headroom visible through the gain map.
- Current immutable reference is `dynamic-range-comparison@2`.

### Edit Comparison contract

- Before is Lightroom Reset + Adobe Color with no user adjustments, exported
  by the Lightroom plug-in as a 16-bit tagged ProPhoto TIFF. Photara restores
  and verifies the exact authored develop state afterward.
- Neutral Before evidence is registered by project asset and rendering
  contract, not by post or platform. Instagram and Threads therefore reuse the
  same verified TIFF for the same asset; a platform-specific export is neither
  required nor desirable. Shared TIFFs live under
  `sources/edit-comparison/before/`; the Lightroom menu asks for the package,
  unions its platform specifications, and does not ask for a platform.
- After is the verified SDR TIFF in the SDR base and HDR TIFF in the HDR top.
- Camera and capture labels come from the RAW, never hard-coded text.
- Canvas: 4500×6000, 32-bit Display P3 Linear.
- Exact image cells:
  - top left: x=226, y=779, 2000×2000
  - top right: x=2276, y=779, 2000×2000
  - bottom left: x=226, y=3331, 2000×2000
  - bottom right: x=2276, y=3331, 2000×2000
- Generated metadata point text uses SF Compact Ultralight, preferred 27 pt,
  minimum 22 pt, with 24 px right padding. Photoshop measures actual glyph
  bounds and only scales down when needed; it rejects text that cannot fit at
  the minimum.
- Current first pair:
  - `DSC05250`: Sony α7R III · SAMYANG AF 14mm F2.8; ISO 125 · 14mm · f/6.3 · 1/640
  - `DSC05421`: same camera/lens; ISO 125 · 14mm · f/2.8 · 1/8000
- The responsive text-fit change is installed but has not been visually rerun
  since the last successful 27 pt render.

## Red Meridian Instagram Package A

The accepted plan is 18 editorial items producing Instagram's maximum 20
carousel slots. Panoramas occupy two slots each. Repeated sources are
intentional placements, not duplicates.

| Slots | Suggested item id | Source(s) | Layout | State |
| --- | --- | --- | --- | --- |
| 1 | `hero` | DSC05250 | Full-frame hero | Complete |
| 2 | `stacked-01` | DSC05445 + DSC05442 | Stacked | Complete |
| 3 | `full-frame-05217` | DSC05217 | Full frame | Complete |
| 4 | `full-frame-05406` | DSC05406 | Full frame | Complete |
| 5–6 | `panorama-05382` | DSC05382 | Two-slot panorama | Complete |
| 7 | `full-frame-05409` | DSC05409 | Full frame | Complete |
| 8 | `stacked-02` | DSC05417 + DSC05419 | Stacked | Complete |
| 9 | `full-frame-05421-a` | DSC05421 | Full frame | Added; render pending |
| 10 | `stacked-03` | DSC05441 + DSC05382 | Stacked; reuse the previously authored 05382 crop intent | Added with verified crop reuse; render pending |
| 11 | `full-frame-05382` | DSC05382 | Full frame | Added; render pending |
| 12 | `full-frame-05372` | DSC05372 | Full frame | Added; render pending |
| 13 | `full-frame-05421-b` | DSC05421 | Repeated full frame | Added; render pending |
| 14 | `dynamic-range-01` | DSC05250 + DSC05421 | Dynamic Range Comparison | Complete |
| 15 | `edit-comparison-01` | DSC05250 + DSC05421 | Edit Comparison | Complete |
| 16–17 | `panorama-05417` | DSC05417 | Two-slot panorama | Crop captured and applied; render pending |
| 18 | `dynamic-range-02` | DSC05445 + DSC05417 | Dynamic Range Comparison | Added; render pending |
| 19 | `edit-comparison-02` | DSC05445 + DSC05417 | Edit Comparison | Added; neutral sources verified; render pending |
| 20 | `full-frame-05250-repeat` | DSC05250 | Repeated full frame | Added; existing hero render intent is reusable |

The live file is:
`/Volumes/whisk/Pictures/Projects/red-meridian/posts/instagram/package-a.json`.
It now contains the accepted final 18-item order:

1. `hero`
2. `stacked-01`
3. `full-frame-05217`
4. `full-frame-05406`
5. `panorama-05382`
6. `full-frame-05409`
7. `stacked-02`
8. `full-frame-05421-a`
9. `stacked-03`
10. `full-frame-05382`
11. `full-frame-05372`
12. `full-frame-05421-b`
13. `dynamic-range-01`
14. `edit-comparison-01`
15. `panorama-05417`
16. `dynamic-range-02`
17. `edit-comparison-02`
18. `full-frame-05250-repeat`

This is the accepted regression fixture. Future implementation must preserve
its order, 18 editorial items, and exact 20-frame expansion unless an actual
validation failure is discovered. The existing 05382 normalized panorama crop
is:

```text
x=0.080871670702
y=0.376513317191
width=0.91186440678
height=0.45593220339
```

That crop belongs to the panorama placement. `stacked-03` now reuses it through
the identity-checked `--bottom-crop-from-item panorama-05382` command; the
renderer resolves the normalized rectangle independently against the verified
paired TIFF dimensions.

The `panorama-05417` fingerprint gate accepted the 7673×5115 capture and
stored this normalized crop:

```text
x=0.050686987646
y=0.114132317284
width=0.88592541277
height=0.885867682716
```

`posts resolve` reports `ready: true`, no requirements, 18 items, and exactly
20 delivery frames. All four Edit Comparison neutral sources have been
generated, restored, and verified for the accepted specification.

The user manually published Instagram Package A. Migration 0018 and
`posts confirm-manual-publication` now store evidence without inventing a
provider receipt. Live evidence ID `3da127dd-178c-4781-9fc7-fecc3f4a95da` is
tied to source-specification SHA-256
`82c0282fbe1123c322e3de8499a19c573cbfd2bda087d2f152860cb1b036d943`.
The provider URL and original publication timestamp were not supplied and are
intentionally null.

The 20 exact Instagram WSP HDR JPEGs are staged under
`workspace/exports/instagram/package-a` and byte-verified in Cloudinary batch
`f450b548-c1e7-4f0c-9962-2b7e8e5571bf`.

Both flattened DSC05382 TIFFs were deliberately changed externally on
2026-08-14. `masters refresh-flattened --asset DSC05382 --override` registered
the new HDR and SDR hashes as authoritative while retaining the old rows as
removed provenance. Use that confirmed refresh command for deliberate in-place
flattened replacements; never overwrite registered checksums directly.

Codex may receive `Operation not permitted` for NAS file contents when its app
process lacks macOS Network Volumes access, even though metadata reads work.
First confirm that `/Volumes/whisk` is mounted. If direct access remains
blocked, provide the same Photara command for the user to run in Ghostty; do not
open a terminal UI, bypass Photara's fingerprint gates, or hand-edit the
project JSON.

Existing render directories live under
`/Volumes/whisk/Pictures/Projects/red-meridian/posts/<platform>/renders/package-a`.
Both packages were completed and reviewed. Preserve these PSBs; later backup
or website work must not unnecessarily rebuild unchanged verified outputs.

## Threads state

Threads remains part of `0.1.0`. Its separate project-owned post specification
and 4500×8000 templates resolve to 17 editorial frames, 27 placements, and 12
stable source assets. Every placement is authored and applied. The eight
Dynamic Range/Edit Comparison placements use the complete TIFF with `contain`
fit, so landscape images letterbox and portrait images pillarbox in their
square cells unless a crop is explicitly requested. All 17 PSBs were reviewed,
exported through WSP, manually published, and recorded. Never derive its
geometry from Instagram.

Threads manual publication evidence ID is
`257d0fa1-212a-4302-a6b0-a30a9794e496`. Its 17 exact WSP HDR JPEGs are staged
under `workspace/exports/threads/package-a` and byte-verified in Cloudinary
batch `800e8484-3090-4d78-b1a4-57c66d6f801c`.

`ROADMAP.md` is authoritative for the fixed 17-item Threads order. Every slide
uses the 9:16 direction; three ordinary editorial items are three-image stacks
whose placements are authored independently in exact parent row geometry; two
full-frame placements rotate 90° clockwise before their 9:16 crops; and the
Dynamic Range and Edit Comparison items use new taller template geometry. Do
not ask the user to reconfirm the order, the 9:16 direction, or two- versus
three-image stacks. Those editorial decisions are closed unless implementation
finds an actual validation failure.

## Recommended next actions

`v0.1.0` is complete. Preserve Red Meridian and Sylvan as the accepted
regression projects and follow `ROADMAP.md` for `0.2.0` discovery and work:

1. Improve the operator experience around the proved CLI, Lightroom Classic,
   Photoshop, and recovery workflows without changing their contracts.
2. Consolidate Lightroom commands into one project-aware plug-in UI and move
   the proved Photoshop scripts behind a persistent UXP panel.
3. Prototype installation, connection checks, database onboarding, and
   color-managed project thumbnails with an HDR-capable preview contract.
4. Keep Pixieset behind its supported Lightroom/CSV boundary and retain manual
   Instagram and Threads publication as a valid evidence-backed path.
5. Reconcile Red Meridian only through non-destructive catalog or
   infrastructure actions. Do not add the grid to, reorder, rerender, or
   republish either accepted package.
6. Keep website order, derivatives, Cloudinary-specific website layouts, and
   Loomara integration deferred until that website contract is designed.

## Validation commands

Use a temporary Cargo target so build output does not pollute Dropbox:

```bash
cargo fmt --check
CARGO_TARGET_DIR=/private/tmp/photara-handoff-target cargo test
```

Install refreshed layout contracts/scripts through Photara rather than copying
individual runtime files by hand:

```bash
photara layouts install
photara posts prepare-render red-meridian package-a --platform instagram
```

The first command materializes the verified template cache; the second creates
the production full-package handoff. Run **Build Photara Layouts.psjs** from
Photoshop afterward. Use `--item ITEM_ID` only for debugging.

## Known limitations and deferred work

- Current menus/scripts prove the workflow but are not an acceptable final UX
  for nontechnical photographers. Guided Lightroom UI and a unified Photoshop
  UXP panel are planned after the 0.1.0 CLI vertical slice, primarily for 0.2.0.
- Adobe Lightroom Services production approval and Cloud album creation are
  deferred to 0.2.0. The present native OAuth development connection is enough
  for the personal prototype.
- Lightroom Desktop does not provide the desired direct automation for every
  edit/handoff action, so provider inventory plus durable local evidence is the
  current source of truth.
- WSP has no integrated stable CLI yet. Keep it behind an adapter and preserve
  the manual Photoshop path.
- Cloudinary exact-original backup and durable evidence are implemented.
  Cloudinary-specific website layouts, derivatives, thumbnails, presentation
  order, Codexa, Loomara, and website publication remain later work.
- Publication API HDR behavior must be verified; keep manual Instagram/Threads
  posting as a valid evidence-backed fallback.
- Photara's existing releases remain MIT. Keep future licensing and commercial
  distribution as an explicit productization decision; it does not block the
  reusable personal/operator `0.1.0` workflow and this handoff does not choose
  a future license.

## Definition of the stable `v0.1.0` checkpoint

The checkpoint is complete:

- Red Meridian retains its accepted 18-item/20-frame Instagram fixture and
  separately specified 17-frame Threads package;
- Sylvan proves reusable 10-frame Instagram and 14-frame Threads packages in
  the authoritative numbered publication order;
- sources, transforms, immutable templates, WSP JPEGs, Cloudinary objects, and
  publication records have connected provenance and checksum evidence; and
- repeat preparation, upload, reconciliation, and cleanup paths are guarded
  against duplicate or premature mutation.
