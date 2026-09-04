# Photara photographer guide: Sylvan from import to publication

This is the end-to-end operator guide for Photara `v0.1.1`, the maintained
reusable operator workflow. It assumes Sylvan is
already imported into Lightroom
Classic, but has no Photara metadata, people, project, client selections, or
publication package yet.

The guide describes the personal setup currently in use: Lightroom Classic,
Lightroom Desktop/Cloud, Photoshop, Web Sharp Pro (WSP), the Pixieset Lightroom
publish service, Cloudinary backup, Ghostty, and the mounted `whisk` NAS.

Photara is intentionally cautious. Commands without `--confirm` or
`--override` are usually previews. Read the result, then run the confirmed form
only when it names the project and files you expect.

## Table of contents

1. [What the finished workflow produces](#1-what-the-finished-workflow-produces)
2. [Words, names, and paths used below](#2-words-names-and-paths-used-below)
3. [Start-of-session checks](#3-start-of-session-checks)
4. [Create Sylvan's people, place, scene, and project](#4-create-sylvans-people-place-scene-and-project)
5. [Tag the shoot and build its Lightroom collections](#5-tag-the-shoot-and-build-its-lightroom-collections)
6. [Publish proofs to Pixieset](#6-publish-proofs-to-pixieset)
7. [Import client favorites, shortlist, and hero](#7-import-client-favorites-shortlist-and-hero)
8. [Choose Photographer Final](#8-choose-photographer-final)
9. [Send Photographer Final to Lightroom Cloud](#9-send-photographer-final-to-lightroom-cloud)
10. [Edit in Lightroom Desktop and build layered masters](#10-edit-in-lightroom-desktop-and-build-layered-masters)
11. [Finish and flatten the paired HDR/SDR masters](#11-finish-and-flatten-the-paired-hdrsdr-masters)
12. [Design the Instagram and Threads packages](#12-design-the-instagram-and-threads-packages)
13. [Author crops and render the Photoshop layouts](#13-author-crops-and-render-the-photoshop-layouts)
14. [Export with WSP and stage exact JPEGs](#14-export-with-wsp-and-stage-exact-jpegs)
15. [Back up the JPEGs to Cloudinary](#15-back-up-the-jpegs-to-cloudinary)
16. [Publish manually and record publication](#16-publish-manually-and-record-publication)
17. [End-of-project checklist](#17-end-of-project-checklist)
18. [Recovery and common problems](#18-recovery-and-common-problems)
19. [Current limits](#19-current-limits)

## 1. What the finished workflow produces

At the end, Sylvan has:

- camera RAWs and XMP sidecars in their original dated NAS folder;
- registered model, location, scene, and project records;
- Photara-managed Lightroom keywords and smart collections;
- retained Pixieset CSV evidence for Client Favorites, Client Shortlist, and
  Hero;
- a separate Photographer Final decision;
- verified working DNGs in Lightroom Cloud;
- self-contained layered PSBs beside their source RAWs;
- paired flattened HDR and SDR TIFFs under the Sylvan project;
- independent Instagram and Threads layout specifications and rendered PSBs;
- exact WSP HDR JPEGs staged in the Sylvan project and byte-verified in
  Cloudinary; and
- durable evidence for manual Instagram and Threads publication.

Photara does not permanently copy the RAWs, use Pixieset as a master archive,
or treat Cloudinary as the future website schema.

## 2. Words, names, and paths used below

Use these fixed values for this project:

| Meaning | Value |
| --- | --- |
| Photara project slug | `sylvan` |
| Display name | `Sylvan` |
| Social package name | `package-a` |
| Lightroom/Adobe account | `personal` |
| Project folder | `/Volumes/whisk/Pictures/Projects/sylvan` |
| Lightroom handoff inbox | `/Users/suhail/Pictures/Photara/Inbox` |
| Photoshop scripts | `/Users/suhail/Pictures/Photara/Scripts` |

The examples use obvious placeholders such as `MODEL_SLUG`, `SCENE_SLUG`,
`LOCATION_SLUG`, `RAW_FOLDER`, `BATCH_UUID`, and `DSC01234.ARW`. Replace the
whole placeholder, including capital letters. Do not type angle brackets.

A slug is a short lowercase identifier with hyphens, for example
`jane-doe`, `woodland-editorial`, or `discovery-park`.

Open Ghostty in the Photara repository before running commands:

```console
cd "/Users/suhail/Library/CloudStorage/Dropbox/matrix/crates/photara"
```

Ghostty already loads the full environment. Do not run Apogee setup commands
and do not expect Photara to open a terminal window on its own.

## 3. Start-of-session checks

First confirm that the NAS is mounted in Finder. Then run:

```console
photara --version
photara health
photara config validate
photara migrate
photara cloud adobe-verify --account personal
photara delivery cloudinary-probe
photara layouts install
```

Success means every command completes without an `Error:` line. `migrate` and
`layouts install` are safe to repeat. The Adobe and Cloudinary checks use
credentials already stored in macOS Keychain.

In Lightroom Classic, run **Library > Plug-in Extras > Validate Photara
Connection**. Stop here if it fails; later Lightroom actions use the same
connection.

## 4. Create Sylvan's people, place, scene, and project

### 4.1 Add the model

Use the model's real display name. This is the minimal command:

```console
photara people add MODEL_SLUG \
  --display-name "MODEL DISPLAY NAME" \
  --role model

photara people show MODEL_SLUG
```

To include known alternate names or social accounts on the first run, append
one `--alias "ALTERNATE NAME"` or quoted `--social "platform=value"` argument
for each value. Copper Mist's model is a complete example:

```console
photara people add kylee-nielsen \
  --display-name "Kylee Nielsen" \
  --alias "Kylee" \
  --role model \
  --social "instagram=_kylee_nielsen_"
```

If the person already exists and you intentionally need to replace the
registry entry, rerun the complete command with `--replace`. Never use
`--replace` merely to get past a typo without first reading `people show`.

### 4.2 Add the location and scene

The location is where the shoot happened. The scene describes the creative
setting or concept.

```console
photara locations add LOCATION_SLUG \
  --display-name "LOCATION DISPLAY NAME" \
  --sublocation "SPECIFIC AREA" \
  --city "CITY" \
  --state "STATE" \
  --country "United States" \
  --iso-country-code US

photara scenes add SCENE_SLUG \
  --display-name "SCENE DISPLAY NAME" \
  --description "SHORT CREATIVE DESCRIPTION"
```

### 4.3 Create the project

```console
photara project init sylvan \
  --display-name "Sylvan" \
  --scene SCENE_SLUG \
  --location LOCATION_SLUG \
  --person MODEL_SLUG

photara project show sylvan
photara metadata plan sylvan
```

`project init` creates the database record and the project directory. If
Sylvan already exists and an association is wrong, use the same complete
arguments with `photara project configure` instead of `project init`.

**Checkpoint:** `project show` names the correct model, location, and scene.

## 5. Tag the shoot and build its Lightroom collections

1. In Lightroom Classic's Library module, select all Sylvan camera originals.
   Do not include virtual copies, exported JPEGs, PSBs, or unrelated photos.
2. Choose **Library > Plug-in Extras > Apply Project to Selected Shoot**.
3. Choose **Sylvan**, inspect the summary, and confirm.
4. When it finishes, use Lightroom's **Metadata > Save Metadata to File**,
   unless automatic XMP writing is enabled.

The plug-in applies only Photara-owned IPTC fields and hierarchical keywords.
It also builds or reconciles Photara's People, Locations, Scenes, Projects,
Originals, Selections, Cloud, and Masters collection structure. It preserves
ratings, flags, captions, titles, color labels, unrelated keywords, and
unrelated collections.

**Checkpoint:** select one Sylvan RAW and confirm that its Photara project,
person, location, and scene keywords are present. Confirm the Sylvan project
collections exist. Repeating the action should converge on the same result.

## 6. Publish proofs to Pixieset

Photara does not log in to Pixieset or upload proofs. Use the official Pixieset
Lightroom publish service:

1. Create a Pixieset collection named exactly **Sylvan**.
2. Add the proof images while retaining each camera original's filename.
3. Publish the collection through the Pixieset service.
4. In Pixieset, create these favorite lists with exactly these names:
   **Client Favorites**, **Client Shortlist**, and **Hero**.
5. Send the gallery to the client and wait for the client to finish.
6. Download one CSV for each of the three lists. Keep the three files separate.

The CSVs must describe the collection `Sylvan` and use Pixieset's normal
columns: `Name, Note, Photo Set, Created at`. The opaque downloaded CSV
filename does not tell Photara which list it represents; the command below
assigns each file explicitly.

Pixieset proofs are temporary delivery copies, not authoritative masters.

## 7. Import client favorites, shortlist, and hero

Find the one dated NAS folder containing the Sylvan camera RAWs. In the current
workflow these source files are Sony `.ARW` originals and each stem must be
unique in that folder.

```console
photara selections import-pixieset sylvan \
  --source-root "RAW_FOLDER" \
  --client-favorites "/PATH/TO/Client Favorites.csv" \
  --client-shortlist "/PATH/TO/Client Shortlist.csv" \
  --hero "/PATH/TO/Hero.csv"

photara selections plan sylvan
```

Photara validates every proof name against exactly one source RAW and stores
the CSV evidence and checksum. The import replaces the current three imported
selection sets atomically; repeating the same import is safe.

Then in Lightroom Classic:

1. Select a Sylvan camera original.
2. Choose **Library > Plug-in Extras > Apply Imported Selections**.
3. Choose Sylvan and confirm the summary.
4. Save metadata to XMP.

Hero implies Client Shortlist and Client Favorite. Client Shortlist implies
Client Favorite. Photara retains the original Pixieset list evidence even
while applying those effective memberships in Lightroom.

For a correction discovered after import, retain the Pixieset evidence and
record a separate audited local override. Preview it first:

```console
photara selections remove copper-mist \
  --asset DSC09993.ARW --from hero \
  --reason "Added accidentally" --dry-run

photara selections remove copper-mist \
  --asset DSC09993.ARW --from hero \
  --reason "Added accidentally"

photara selections status copper-mist --asset DSC09993.ARW
photara selections history copper-mist --asset DSC09993.ARW
```

Adding Hero automatically produces effective Shortlist and Favorite
membership. Adding Shortlist automatically produces effective Favorite
membership. A removal that would violate that hierarchy stops unless
`--cascade` is explicit. Rerun **Apply Imported Selections** and save metadata
after an accepted correction.

**Checkpoint:** the Sylvan selection smart collections and three managed
selection keywords show the expected photos.

## 8. Choose Photographer Final

Client choices are evidence, not the photographer's final edit. Review the
images in Lightroom Classic and select the camera originals that will receive
full editing and master treatment.

Choose **Library > Plug-in Extras > Add Selected to Photographer Final**,
inspect the count, and confirm. Save metadata afterward.

To change the decision before downstream work, select the originals and use
**Remove Selected from Photographer Final**. If an image has already been
uploaded to Lightroom Cloud, use Photara's verified withdrawal workflow rather
than merely removing the keyword; see [Recovery](#18-recovery-and-common-problems).

**Checkpoint:** the Photographer Final smart collection contains the exact
set you intend to edit.

For the in-progress Copper Mist `v0.1.1` verification, the accepted checkpoint
is 24 Photographer Final assets, 47 effective Client Favorites, and 23
effective Client Shortlist assets.

## 9. Send Photographer Final to Lightroom Cloud

### 9.1 Refresh the inventory

```console
photara cloud adobe-inventory --account personal
```

### 9.2 Prepare DNGs in Lightroom Classic

1. Choose **Library > Plug-in Extras > Prepare Photographer Final DNGs**.
2. Read the plan: already-present files should be skipped and pending files
   should match Photographer Final.
   If every asset is already verified, Photara reports **No Transfer Required**,
   reconciles the local presence ledger, and prepares no DNGs.
3. Confirm reservation.
4. Choose **Test One** first.
5. Inspect the canary DNG when Lightroom finishes.
6. Run the same menu action again and choose **Prepare All**.

The Lightroom action only renders and validates local DNGs. It does not upload
or delete them. Record the batch UUID shown by Photara as `BATCH_UUID`.

### 9.3 Upload and verify

```console
photara cloud upload-preflight BATCH_UUID --account personal
photara cloud upload-canary BATCH_UUID --account personal
photara cloud verify-canary BATCH_UUID --account personal
photara cloud upload-remaining BATCH_UUID --account personal
photara cloud verify-batch BATCH_UUID --account personal
```

If canary or batch verification says Adobe is still processing, rerun only
the matching `verify-...` command. Do not upload the same item again.

Back in Lightroom Classic, select one verified Sylvan original and choose
**Apply Verified Cloud Presence**. After the one-photo check succeeds, rerun
it for all matched Sylvan originals. Save metadata.

Provider-owned Adobe collection projection is optional in the current personal
prototype:

```console
photara cloud collection-plan sylvan --account personal
photara cloud sync-collections sylvan --account personal --confirm
```

Adobe may keep these Connection-owned collections invisible in Lightroom
clients until the Photara application is formally recognized. That visibility
does not invalidate verified Cloud asset evidence.

Only after every DNG is verified remotely, clean the transfer staging folder:

```console
photara cloud cleanup-batch BATCH_UUID --confirm
```

## 10. Edit in Lightroom Desktop and build layered masters

### 10.1 Edit the Cloud DNGs

Open the verified Sylvan DNGs in Lightroom Desktop and complete the
nondestructive Lightroom edits. Keep the DNGs in Cloud; do not rename them.

Prepare Photara's master handoff, choosing one recognizable Photographer Final
camera filename as the canary:

```console
photara masters prepare sylvan --canary DSC01234.ARW
```

The result lists the exact expected DNG and PSB names and installs **Build
Photara Masters.psjs** beside the other Photara scripts in
`/Users/suhail/Pictures/Photara/Scripts`. The Inbox remains reserved for DNGs
and its hidden `.photara` operational workspace.

### 10.2 Build and inspect one canary

1. In Lightroom Desktop, export the canary using **Original + Settings** to
   `/Users/suhail/Pictures/Photara/Inbox`.
2. In Photoshop, run **File > Scripts > Browse**, then choose
   `/Users/suhail/Pictures/Photara/Scripts/Build Photara Masters.psjs`.
3. When asked for a folder, choose `/Users/suhail/Pictures/Photara/Inbox`.
4. Inspect the canary PSB in
   `/Users/suhail/Pictures/Photara/Inbox/.photara/output`.
5. Confirm it contains the DNG as an embedded, not linked, Camera Raw Smart
   Object and that the initial document looks correct.

Canary mode processes only the marked canary even if other DNGs are already in
the Inbox. Do not run `masters verify` or promote yet; full verification
requires the complete report.

### 10.3 Build the remaining initial PSBs

1. Export every remaining edited Sylvan DNG from Lightroom Desktop using the
   same **Original + Settings** preset and inbox.
2. Switch the manifest from canary mode to the complete batch:

```console
photara masters prepare sylvan
```

3. Run **Build Photara Masters.psjs** again. It verifies the existing canary
   and builds the rest.
4. Run:

```console
photara masters status sylvan
photara masters verify sylvan
photara masters promote sylvan
photara masters promote sylvan --confirm
```

Read the unconfirmed promotion plan before confirming it. Promotion installs
each verified PSB beside its source RAW, registers its provenance, and removes
the redundant inbox PSB only after the authoritative copy is safe.

### 10.4 Import the authoritative PSBs in Lightroom Classic

Promotion puts each PSB in its permanent location, but it does not mutate the
Lightroom catalog. In Lightroom Classic choose **Library > Plug-in Extras >
Import Verified Layered Masters**, choose Sylvan, inspect the counts, and
confirm.

Photara re-verifies every current database-registered PSB before presenting
the import. Lightroom adds each new PSB without moving it and places the exact
verified set in read-only Sylvan **Masters > PSB** smart collections. Their
rules require both Photara's read-only, catalog-only project marker and native
PSB file type. Photara does not add keywords or IPTC metadata to the PSBs. An
identical rerun recognizes already imported PSBs instead of creating
duplicates.

Do **not** use **Metadata > Save Metadata to File** on the imported PSBs. The
Photara master marker exists only in the Lightroom catalog and cannot be saved
to XMP or the file, while the database and filesystem remain authoritative for
master state. Keep Photoshop as the sole writer of layered PSBs, especially
over SMB. Open a master from Lightroom with **Edit In > Adobe Photoshop > Edit
Original**.

Catalogs that previously used Photara's keyword-driven PSB smart collections
need a one-time cleanup. Select every PSB for the project, choose **Library >
Plug-in Extras > Reconcile Layered Master Collections**, and confirm the exact
selection. Then select those PSBs from the rebuilt smart **Masters > PSB**
collection and choose **Metadata > Read Metadata From File**. This makes the
current PSB files authoritative for standard metadata and clears the old
up-arrow and metadata-conflict badges without removing Photara's catalog-only
membership. Do not choose **Overwrite Settings** for this cleanup; that writes
Lightroom's stale catalog metadata into the PSBs.

Confirm that every PSB appears under Sylvan's Masters collection before
beginning raster editing. Stacking a PSB with its RAW is optional catalog
organization; do it manually when useful, but Photara does not require or
verify it.

## 11. Finish and flatten the paired HDR/SDR masters

The promoted and cataloged initial PSB is a self-contained 16-bit P3 PQ
starting master. Open each authoritative PSB and complete Photoshop raster work.
Use Generative Expand, skin retouching, dodge and burn, healing, clone work,
and any other 16-bit finish first. Do not manually build the final HDR/SDR
containers.

With the finished PSB active, run:

`/Users/suhail/Pictures/Photara/Scripts/Prepare Photara HDR-SDR Master.psjs`

The script validates the saved 16-bit P3 PQ starting document, adds an empty
top layer named `16-bit`, and converts the complete editable stack into one
embedded Smart Object. It performs an ordinary duplicate—the equivalent of
Command-J—so HDR and SDR continue to share the same embedded 16-bit source;
this is deliberately not **New Smart Object via Copy**. It names the upper
instance `HDR` and the lower instance `SDR`.

Photara first converts the parent document to 32-bit with **Merge disabled**
and **Rasterize disabled**, converts it to Display P3, and requires Photoshop
to report Display P3 Linear. Camera Raw Filter then opens on `SDR`; the color
contract therefore does not depend on completing the optional SDR-authoring
session. Turn **HDR off**, author the SDR appearance, and click **OK**. The
script remains bound to the PSB that was active when it started and reactivates
that exact document before every operation, even if you click another tab
during an alert. It also verifies that both top-level layers remain Smart
Objects. It never saves the PSB.

Inspect both renditions and the SDR Smart Filter, then save the document. If
the script stops after making a partial change, it leaves the PSB unsaved so
you can use **Edit > Undo** and report the exact alert.

The resulting structure expected by the flattening script is:

- a top-level `HDR` container above a top-level `SDR` container;
- both renditions at the same canvas dimensions;
- the HDR rendition carrying the HDR finish; and
- the SDR rendition carrying its intended SDR appearance.

Save the PSB, then checkpoint whenever useful:

```console
photara masters checkpoint sylvan
```

For recovery of one deliberately rebuilt or revised PSB, scope the checkpoint
so unrelated master changes are neither inspected nor accepted:

```console
photara masters checkpoint sylvan --asset _SUH5128.ARW
```

When every layered master is final, preview and confirm readiness:

```console
photara masters mark-ready sylvan
photara masters mark-ready sylvan --confirm
photara masters prepare-flattening sylvan
```

Long-running master commands report their current broad stage, asset, and
completed/total count on an interactive terminal. Structured JSON or Lua stays
on stdout; progress uses stderr and is automatically quiet when stderr is not
interactive.

Run `/Users/suhail/Pictures/Photara/Scripts/Flatten Photara Masters.psjs` in
Photoshop. When prompted, choose the reported Sylvan project folder and then
the configured Images root. The script duplicates and flattens both renditions
without modifying the layered PSB.

Then verify and register:

```console
photara masters verify-flattening sylvan
photara masters register-flattening sylvan
photara masters register-flattening sylvan --confirm
```

**Checkpoint:** every Photographer Final asset has one authoritative PSB beside
its RAW and one registered HDR/SDR TIFF pair in
`/Volumes/whisk/Pictures/Projects/sylvan/masters/flattened`.

## 12. Design the Instagram and Threads packages

Instagram and Threads are independent editorial packages. First make a simple
worksheet listing, in order:

- a short unique item ID such as `hero`, `stacked-01`, or
  `full-frame-01234`;
- the source camera filename(s);
- the layout type; and
- the fit policy for each placement and any quarter-turn rotation.

The fit policy is explicit and applies to every ordinary placement, including
full-frame images and grid cells. It is not inferred from the source aspect:

- `fill` automatically scales and center-crops the image to cover the entire
  target. No crop-authoring step is required.
- `contain` fits the complete image inside the target. No crop-authoring step
  is required; unused space becomes letterbox or pillarbox area.
- `crop` means you will author the exact platform-specific crop. This is the
  safe default for new full-frame and four-grid items.

Current reusable layout types are:

| Editorial need | Command | Normal placement behavior |
| --- | --- | --- |
| One image | `add-full-frame` | Choose `fill`, `contain`, or `crop` |
| Two stacked images | `add-stacked-two` | Set the policy independently per slot |
| Three stacked images | `add-stacked-three` | Set the policy independently per slot |
| Four-image 2×2 grid | `add-grid-four` | Choose a starting policy, then override any slot |
| Two-frame panorama | `add-continuous-panorama` | Author its panoramic crop |
| HDR/SDR comparison | `add-dynamic-range-comparison` | `contain` by default |
| Before/after edit | `add-edit-comparison` | `contain` by default |

Create each package once:

```console
photara posts init sylvan package-a --platform instagram
photara posts init sylvan package-a --platform threads
```

Add worksheet items with the matching commands. These examples show the
syntax; replace item IDs and asset filenames with Sylvan's choices:

```console
photara posts add-full-frame sylvan package-a \
  --platform instagram --item hero --asset DSC01234.ARW --fit fill

photara posts add-stacked-two sylvan package-a \
  --platform instagram --item stacked-01 \
  --top DSC01235.ARW --bottom DSC01236.ARW

photara posts add-stacked-three sylvan package-a \
  --platform threads --item stacked-01 \
  --top DSC01235.ARW --middle DSC01236.ARW --bottom DSC01237.ARW

photara posts add-grid-four sylvan package-a \
  --platform instagram --item grid-01 \
  --top-left DSC01234.ARW --top-right DSC01235.ARW \
  --bottom-left DSC01236.ARW --bottom-right DSC01237.ARW \
  --fit crop

photara posts add-grid-four sylvan package-a \
  --platform threads --item grid-01 \
  --top-left DSC01234.ARW --top-right DSC01235.ARW \
  --bottom-left DSC01236.ARW --bottom-right DSC01237.ARW

photara posts add-continuous-panorama sylvan package-a \
  --platform instagram --item panorama-01 --asset DSC01238.ARW

photara posts add-dynamic-range-comparison sylvan package-a \
  --platform threads --item dynamic-range-01 \
  --top DSC01234.ARW --bottom DSC01238.ARW

photara posts add-edit-comparison sylvan package-a \
  --platform threads --item edit-comparison-01 \
  --top DSC01234.ARW --bottom DSC01238.ARW
```

Three-image stacks work on both platforms. With no `--rows` option they use an
equal one-third distribution. To emphasize the center image, add
`--rows 30,40,30`; Photara derives the exact crop-selection dimensions
independently for Instagram and Threads. Percentages must total 100 unless
`--outer-letterbox` explicitly permits centered black padding, for example
`--rows 25,40,25 --outer-letterbox`.

`add-full-frame` and `add-grid-four` accept `--fit fill`, `--fit contain`, or
`--fit crop`. Use `set-fit` to change any existing placement or to give the
slots of a multi-image item different policies:

```console
photara posts set-fit sylvan package-a \
  --platform instagram --item hero --fit contain

photara posts set-fit sylvan package-a \
  --platform threads --item grid-01 --slot top-left --fit fill
```

Changing to a different fit clears any old crop for that placement while
preserving its quarter-turn rotation. Repeating its current fit is idempotent
and preserves an existing authored crop. A multi-placement item requires
`--slot`.

Add exact clockwise quarter-turns only when intentionally designed:

```console
photara posts set-transform sylvan package-a \
  --platform threads --item full-frame-01234 \
  --rotation-quarter-turns-cw 1
```

Set the final order by repeating `--item` in order:

```console
photara posts reorder sylvan package-a --platform instagram \
  --item hero \
  --item stacked-01 \
  --item NEXT_ITEM
```

Run the equivalent order command for Threads, then inspect both:

```console
photara posts show sylvan package-a --platform instagram
photara posts show sylvan package-a --platform threads
photara posts resolve sylvan package-a --platform instagram
photara posts resolve sylvan package-a --platform threads
```

An Instagram package may contain from 1 through 20 delivery frames. Twenty is
the current platform maximum, not a required package size. A panorama is one
editorial item but expands to two delivery frames. Instagram's grid uses four
2250 x 3000, 3:4 cells. Threads fills its 4500 x 8000 frame with four 2250 x
4000, 9:16 cells. The master PSB/TIFF may have any authored aspect.

If the package uses Edit Comparison, select one Sylvan project photo in
Lightroom Classic and choose **Prepare Edit Comparison Sources**. Choose
`package-a`; the plug-in resolves both platforms and reuses already verified
Before TIFFs. Follow the dialog to export Lightroom **Reset + Adobe Color**
sources, then rerun the menu action to verify them.

## 13. Author crops and render the Photoshop layouts

Only placements whose policy is `crop` and which do not yet have a captured
crop enter placement authoring. `fill` and `contain` are automatic regardless
of the source aspect. Panorama authoring remains its specialized continuous
crop workflow. Prepare both independent social crop sets in one session:

```console
photara posts prepare-authoring sylvan package-a \
  --platform instagram \
  --also-platform threads
```

To deliberately replace an existing authored crop, select its item (and slot
when needed) and add `--reauthor`. Automatic `fill` and `contain` placements
are never included, even when they share that item:

```console
photara posts prepare-authoring sylvan package-a \
  --platform threads --item stacked-3-01 --slot middle --reauthor
```

The package-wide command opens every unresolved crop from both posts. Context
names end in `instagram` or
`threads`; adjust each selection for that named platform, then run **Capture
Photara Placement** once. Apply the captured report through the primary
platform command. Photara validates both post fingerprints and persists the
3:4 and 9:16 transforms independently:

```console
photara posts apply-authoring sylvan package-a --platform instagram
```

Use `--also-platform` only for the convenience of one Photoshop session. It
does not make the two packages share crop geometry.

To redo only one matching editorial item on both platforms, add
`--item ITEM_ID`. To redo one platform alone, omit `--also-platform`.

For each unresolved item:

```console
photara posts prepare-authoring sylvan package-a \
  --platform threads \
  --item ITEM_ID
```

Then in Photoshop, in this order:

1. Run `/Users/suhail/Pictures/Photara/Scripts/Author Photara Placement.psjs`.
2. Adjust the image in its real target composition.
3. Run `/Users/suhail/Pictures/Photara/Scripts/Capture Photara Placement.psjs`.
4. Return to Ghostty and apply the captured report:

```console
photara posts apply-authoring sylvan package-a --platform threads
```

Repeat for Instagram, changing the platform. For a multi-placement item you
may prepare the complete item; Photara visits its slots in deterministic order.
Always run Author before Capture for the current session.

When both specifications resolve cleanly:

```console
photara posts prepare-render sylvan package-a --platform instagram
photara posts prepare-render sylvan package-a --platform threads
```

Run `/Users/suhail/Pictures/Photara/Scripts/Build Photara Layouts.psjs` in
Photoshop after each preparation. Review every generated PSB at full size.
Use `prepare-render ... --item ITEM_ID` only to debug a single item; production
uses the full-package command above.

## 14. Export with WSP and stage exact JPEGs

Run Web Sharp Pro manually on the reviewed layout PSBs. Export the exact HDR
JPEG originals; temporary HLG story files are ephemeral and must not enter the
Photara export folders.

Stage the final files here:

```text
/Volumes/whisk/Pictures/Projects/sylvan/workspace/exports/instagram/package-a
/Volumes/whisk/Pictures/Projects/sylvan/workspace/exports/threads/package-a
```

Filename rules are strict because they connect each JPEG to the post:

- Instagram: `01_hero.jpg`, `02_stacked-01.jpg`, and so on. Panorama frames use
  the rendered logical suffixes such as `_col1` and `_col2` after the item ID.
- Threads may retain the same publication-order prefix, such as `01_hero.jpg`
  or `03_stacked-3-01.jpg`. Photara records that ordered filename while mapping
  the remainder to the exact item ID. Legacy unnumbered Threads exports remain
  readable.
- Use `.jpg`; do not add descriptive words after export.

The final folder must contain no missing, extra, duplicated, or stale JPEGs.
Photara will reject a folder rather than guess.

## 15. Back up the JPEGs to Cloudinary

Cloudinary currently stores byte-verified backups of these exact WSP files. It
does not define website order, thumbnails, derivatives, or Loomara input.

For Instagram:

```console
photara delivery prepare sylvan package-a --platform instagram
photara delivery upload-canary INSTAGRAM_BATCH_UUID --confirm
photara delivery verify-canary INSTAGRAM_BATCH_UUID
photara delivery upload-remaining INSTAGRAM_BATCH_UUID --confirm
photara delivery verify INSTAGRAM_BATCH_UUID
```

Repeat with `--platform threads` and the Threads batch UUID. Read each prepare
result and copy its actual UUID; do not reuse the Instagram UUID for Threads.

Repeated preparation reuses a matching immutable batch. Repeated upload reuses
matching Cloudinary evidence and does not create duplicate remote objects.

## 16. Publish manually and record publication

Publish the final packages manually to Instagram and Threads. After confirming
the live post, preview and then record the evidence.

```console
photara posts confirm-manual-publication sylvan package-a \
  --platform instagram \
  --note "Operator confirmed manual Instagram publication"

photara posts confirm-manual-publication sylvan package-a \
  --platform instagram \
  --note "Operator confirmed manual Instagram publication" \
  --confirm

photara posts confirm-manual-publication sylvan package-a \
  --platform threads \
  --note "Operator confirmed manual Threads publication"

photara posts confirm-manual-publication sylvan package-a \
  --platform threads \
  --note "Operator confirmed manual Threads publication" \
  --confirm
```

If you know the public post URL or exact publication time, add `--url` or
`--published-at` to both the preview and confirmed command. Omit unknown values;
never invent them.

## 17. End-of-project checklist

- [ ] `project show` names the correct model, location, and scene.
- [ ] Lightroom Classic has the Sylvan project metadata and collections.
- [ ] Pixieset's three exact CSV lists were imported and applied.
- [ ] Photographer Final is correct and distinct from client selections.
- [ ] Every final DNG is verified in Lightroom Cloud.
- [ ] Every final image has a current layered PSB beside its RAW.
- [ ] Every final image has registered paired HDR/SDR flattened TIFFs.
- [ ] Instagram resolves to at least 1 and no more than 20 delivery frames.
- [ ] Threads resolves independently with its intended 9:16 designs.
- [ ] Four-grid crops, if used, resolve independently to exact 3:4 Instagram
      and 9:16 Threads rectangles.
- [ ] Every layout PSB was visually reviewed and exported through WSP.
- [ ] The two export folders contain only exact final HDR JPEGs.
- [ ] Both Cloudinary batches pass full verification.
- [ ] Both manual publications have `recorded: true` evidence.
- [ ] Ephemeral DNG staging and HLG story files are not treated as masters.

When every box passes without source-code changes or a Sylvan-specific branch,
Sylvan has proved the reusable `0.1.0` workflow and the release may close.
Running more real projects before tagging is useful optional hardening; it is
not a new release gate unless one exposes an actual validation or recovery
failure.

## 18. Recovery and common problems

### The NAS path is missing or access is denied

Confirm `/Volumes/whisk` is mounted. If Photara still reports access trouble,
run the same command directly in Ghostty. Do not change database checksums or
project JSON by hand. Project initialization safely tolerates NAS filesystems
that report `Operation not supported` for file synchronization and recovers an
exact interrupted temporary project marker on the next identical run.

### A source or flattened TIFF was intentionally replaced

Photara correctly rejects a changed registered file. Preview and then confirm
the provenance-preserving refresh for that asset:

```console
photara masters refresh-flattened sylvan --asset DSC01234
photara masters refresh-flattened sylvan --asset DSC01234 --override
```

### Photoshop cannot find an authoring context

Run `prepare-authoring` again for the exact platform and item. In Photoshop,
run **Author Photara Placement** before **Capture Photara Placement**, then run
`apply-authoring`. Do not use old `v2` or `v3` copies of the scripts.

### Build Photara Masters names the wrong project

The Inbox contains a stale hidden master manifest. Run `photara masters
prepare PROJECT --canary CAMERA_FILENAME` for the current project before
running the global script. Confirm the command output names the current project
and lists the DNG filenames actually present in the Inbox.

### A report says the source or post changed

This is a stale-session safety check. Prepare a new authoring/render session
from the current source and repeat that session. Do not edit manifest hashes.

### Lightroom reports Adobe Standard instead of Adobe Color

The Edit Comparison source contract requires Lightroom Reset plus Adobe Color.
Confirm the selected source supports Adobe Color, apply that profile, and
rerun the plug-in action. Do not accept Adobe Standard as equivalent evidence.

### Adobe or Cloudinary verification is still processing

Use the relevant verification command again. Verification is resumable; do
not start a second upload batch merely because the provider response was slow.

### A Photographer Final image must be withdrawn from Cloud

Do not remove the Cloud keyword manually. Begin the guarded withdrawal and
follow the exact reported manual/provider action:

```console
photara cloud begin-withdrawal sylvan \
  --original "/ABSOLUTE/PATH/TO/DSC01234.ARW" \
  --account personal \
  --reason "WHY THE IMAGE WAS WITHDRAWN"
```

After reviewing, rerun with `--confirm`, perform the reported provider action,
verify the withdrawal using the UUID Photara returns, then choose **Apply
Verified Cloud Withdrawal** in Lightroom Classic.

### Cloudinary credentials need to be restored

The normal setup already stores them in Keychain. If `cloudinary-probe` fails
because the credential is absent, restore it without printing the secret:

```console
CLOUDINARY_API_KEY="$(op read 'op://API/Cloudinary/API Key')" \
CLOUDINARY_API_SECRET="$(op read 'op://API/Cloudinary/API Secret')" \
photara delivery cloudinary-login --cloud-name dicttuyma
```

Then rerun `photara delivery cloudinary-probe`. Approve one current Keychain
request; do not launch several concurrent commands that produce stacked
prompts.

## 19. Current limits

- Pixieset proof upload, Lightroom Desktop editing, Photoshop authoring,
  Photoshop master finishing, WSP export, and social publication are manual UI
  work.
- Instagram packages currently support 1 through 20 delivery frames.
- Sylvan's editorial item order and asset choices are photographer decisions;
  Photara does not invent a package automatically.
- Cloudinary is only an off-site backup of exact WSP HDR JPEGs for now.
- Website layout, derivatives, thumbnail rules, Loomara schema, and automated
  website publication are deliberately deferred.
- Current commands and scripts prove the workflow; a more guided operator UI
  belongs to later product work.

For implementation details and invariants, see [the main README](../README.md),
[the layout contract](../LAYOUTS.md), and [the roadmap](../ROADMAP.md).
