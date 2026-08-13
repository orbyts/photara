# Codex handoff: Photara

> Temporary working document. Keep it updated while the 0.1.0 vertical slice
> is in progress, then remove it when the project no longer needs a cross-task
> handoff. Last updated: 2026-08-12.

## Resume objective

Continue Photara `0.0.9` from the current Red Meridian layout checkpoint. The
immediate goal is to finish the accepted 20-slot Instagram carousel, then build
the independent Threads package, publish both with durable evidence, and
complete the end-to-end `0.1.0` release. Do not redesign Storexa or restart the
workflow from ingest.

Read these files before changing code:

1. `ROADMAP.md` — release scope and remaining work.
2. `LAYOUTS.md` — authoritative layout, HDR/SDR, Photoshop, WSP, and Red
   Meridian 20-slot rules.
3. `README.md` — current CLI and operator workflow.
4. `CHANGELOG.md` — changes accumulated for 0.0.9.
5. This handoff — live state and sharp edges which are intentionally too
   temporary for durable product documentation.

## Repository and Git state

- Repository: `/Users/suhail/Library/CloudStorage/Dropbox/matrix/crates/photara`
- Branch: `main`
- Clean rollback point: commit `c9ec2db`, tag `v0.0.8`, also on `origin/main`.
- `Cargo.toml` is already `0.0.9`.
- The complete 0.0.9 implementation is intentionally uncommitted and the
  worktree is broadly dirty. Several files are staged while later refinements
  are unstaged. Preserve all of it. Do not reset, restore, clean, or replace the
  worktree wholesale.
- Dropbox has previously produced conflict copies. It is currently safe to
  leave Dropbox running, but inspect any new conflict copy before choosing a
  canonical file.
- Do not commit or tag 0.0.9 until the user explicitly asks.

At handoff time `git status --short` includes the 0.0.9 Rust layout engine,
Lightroom Edit Comparison bridge, Photoshop scripts, migrations 0016/0017,
template contracts, and documentation. New untracked files include
`lightroom/photara.lrplugin/PrepareEditComparisonMain.lua`,
`templates/dynamic-range-comparison/v2.json`, and
`templates/edit-comparison/v1.json`.

## Live installation and data paths

- CLI: `/Users/suhail/.local/bin/photara`
- User configuration: `$XDG_CONFIG_HOME/photara`
- Secrets/environment: Apogee loads them; never copy secret values into the
  repo or this file.
- Lightroom Classic plug-in source:
  `lightroom/photara.lrplugin`
- Installed Lightroom plug-in: `~/Library/Application Support/Adobe/Lightroom/Modules/photara.lrplugin`
  (linked to the repository)
- Installed Photoshop scripts: `/Users/suhail/Pictures/Photara/Scripts`
- Preferred host terminal: Ghostty. It loads the user's full environment; use
  it rather than Terminal.app when a host-side Photara command cannot run
  directly through Codex.
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
  sequence, assets, crops, and repeated placements.
- Instagram authoring canvas is 4500×6000 (3:4).
- The planned Threads authoring canvas is 4500×8000 (9:16), but no Threads
  package has been authored yet.
- Every Photoshop handoff document has `HDR` above `SDR` for Web Sharp Pro.
  Annotations are identical between them; only declared HDR-variable image/ramp
  regions may differ.
- WSP performs final resizing and splits continuous panoramas. Photara owns the
  continuous editorial surface, crop, seam intent, output order, and evidence.
- `full-frame@1`, `stacked-two@1`, `continuous-panorama@1`,
  `dynamic-range-comparison@2`, and `edit-comparison@1` are implemented.
- Photoshop rendering supports a single-item debug manifest:

  ```bash
  photara posts prepare-render red-meridian package-a \
    --platform instagram --item edit-comparison-01
  ```

  Omit `--item` for a production full-package manifest. The live manifest was
  last prepared in single-item debug mode; regenerate it without `--item`
  before production rendering.
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
  permutation. Full Instagram render preparation now rejects any package that
  does not expand to exactly 20 delivery frames.

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
| 19 | `edit-comparison-02` | DSC05445 + DSC05417 | Edit Comparison | Added; Lightroom neutral sources pending |
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

Do not call that order final. Add the missing items and reorder it to the table
above before freezing or publishing. The existing 05382 normalized panorama
crop is:

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

After adding `edit-comparison-02` and reordering, `posts resolve` reports
`ready: true`, no requirements, 18 items, and exactly 20 delivery frames. The
Edit Comparison source manifest has been refreshed but Lightroom Classic has
not yet generated and restored the neutral sources for the new specification.

Codex may receive `Operation not permitted` for NAS file contents when its app
process lacks macOS Network Volumes access, even though metadata reads work.
First confirm that `/Volumes/whisk` is mounted. If direct access remains
blocked, run the same Photara command through Ghostty; do not bypass Photara's
fingerprint gates or hand-edit the project JSON.

Existing render directory:
`/Volumes/whisk/Pictures/Projects/red-meridian/posts/instagram/renders/package-a`.
It contains verified working PSBs for the nine current item ids, including the
successful 479 MB `edit-comparison-01.psb`. Preserve these files; adding or
reordering placements should not unnecessarily rebuild unchanged verified
outputs.

## Threads state

Threads remains part of the 0.1.0 vertical slice but has not been authored.
Create it as a separate project-owned post specification after Instagram is
complete. It may reuse the same stable assets and narrative, but it needs an
explicit 4500×8000 template/profile, independent crops and layout choices,
ordered delivery manifest, WSP exports, and publication evidence. Do not assume
the Instagram 20-slot sequence is automatically the desired Threads sequence;
confirm its editorial order with the user when implementation reaches it.

The current design direction is explicitly 9:16 for every Threads slide. Do
not stretch the existing Instagram templates vertically. The taller comparison
surface may support three Before/After or SDR/HDR image rows instead of two,
which requires a new versioned Threads-specific Dynamic Range Comparison and
Edit Comparison geometry rather than silently changing the accepted Instagram
versions. The ordinary stacked layout also has an open editorial decision:
retain two taller placements or introduce a three-image stack. Resolve these
choices visually with the user before authoring the Threads post or freezing
template contracts.

## Recommended next actions

1. In Lightroom Classic, use **Prepare Edit Comparison Sources** for the
   refreshed manifest. It includes DSC05445 and DSC05417 and also re-verifies
   the first pair against the final post specification.
2. Optionally rerender only `edit-comparison-01` to visually confirm responsive
   metadata text still looks correct.
3. Prepare the full render manifest without `--item`, build or reuse all PSBs,
   run WSP, and verify exactly 20 ordered final Instagram files.
4. Design and build the separate Threads package.
5. Publish with provider receipt evidence or explicit manual confirmation,
    then test guarded cleanup of ephemeral artifacts.
6. Finish the photographer CLI guide, operator guide, recovery/idempotency
    tests, license decision, and 0.1.0 release preparation.

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
- Cloudinary, Codexa, Loomara, website layouts, and website publication are
  post-0.1.0 work.
- Publication API HDR behavior must be verified; keep manual Instagram/Threads
  posting as a valid evidence-backed fallback.
- Photara is currently MIT licensed, but the user is considering selling it.
  Resolve the licensing/distribution strategy before the supported 0.1.0
  release rather than casually changing it mid-prototype.

## Definition of the next stable checkpoint

The next checkpoint is not merely “all Photoshop documents opened.” It is:

- the accepted Instagram project JSON has 18 ordered editorial items;
- the delivery manifest expands those into exactly 20 ordered slots;
- every source, crop, template version, PSB, WSP output, and checksum has
  connected provenance;
- the independent Threads package is also rendered and verified;
- publication is confirmed with durable evidence;
- repeat runs create no duplicate assets, placements, outputs, or receipts;
- cleanup is restart-safe and only removes ephemeral files after evidence is
  durable.
