# Photara

Photara is an experimental photography workflow and publishing tool.

The project will provide tooling for managing photographic projects from
selection and editing through derivative generation, media storage, and
publication bookkeeping.

## Status

`v0.1.0` proves the reusable Red Meridian and Sylvan workflows across paired
HDR/SDR masters, generalized placement authoring, Instagram and Threads
packages, manual-publication evidence, and exact-original Cloudinary backup.
Further work begins with the `0.2.0` operator-experience and visual-authoring
discovery described in the roadmap. Photara owns its schemas, SQL,
repositories, and photography workflow; Storexa owns connection and
transaction plumbing.

See [ROADMAP.md](ROADMAP.md) for the path to the first supported release and
[METADATA.md](METADATA.md) for the Lightroom metadata ownership contract.
[LAYOUTS.md](LAYOUTS.md) records the evolving layout, HDR/SDR, WSP handoff, and
publication-package design for `0.0.9`. Photographers should start with the
[Sylvan end-to-end photographer guide](docs/PHOTOGRAPHER_GUIDE.md); this README
is the lower-level technical reference.

## Configuration

Non-secret configuration lives under `$XDG_CONFIG_HOME/photara` (or the
explicit `PHOTARA_CONFIG_ROOT` override):

```text
photara/
├── config/
│   ├── photara.toml
│   ├── people.yml
│   ├── locations.yml
│   └── scenes.yml
├── cache/
└── schemas/
```

Initialize without overwriting any existing files, then validate after adding
registry entries:

```console
$ photara config init
$ photara config validate
```

Machine-specific storage roots may override the TOML values. This keeps the
configuration usable without Apogee while allowing a mounted archive to move
without changing database identities:

```console
$ export PHOTARA_IMAGES_ROOT=/Volumes/whisk/Pictures/Images
$ export PHOTARA_PROJECTS_ROOT=/Volumes/whisk/Pictures/Projects
$ export PHOTARA_LIGHTROOM_INBOX="$HOME/Pictures/Photara/Inbox"
$ export PHOTARA_TEMPLATES_ROOT="$DROPBOX/Pictures/Photara/Templates"
$ export PHOTARA_TEMPLATES_CACHE="$HOME/Library/Caches/photara/templates"
```

The configured `templates_root` is the authoritative, device-independent
registry for immutable layout template versions. Suhail's installation uses
`$DROPBOX/Pictures/Photara/Templates`; other users may choose any absolute
local or synchronized directory. `templates_cache` is a disposable,
checksum-verified device-local cache used before Photoshop handoff. Project
specifications store logical references such as `dynamic-range-comparison@2`
and checksums, never machine-specific template paths.

`lightroom_inbox` defaults to `~/Pictures/Photara/Inbox` and is stored in
`$XDG_CONFIG_HOME/photara/config/photara.toml`. Lightroom Desktop exports
edited DNGs there with **Original + Settings**. After the first export, its
**Export With Previous** command (`Command-E`) reuses the same settings and
destination. Photara keeps manifests, reports, and generated master output in
the inbox's internal `.photara` directory; users never need to navigate into
`~/.cache`. A future GUI will read and update this same TOML setting.

The database stores `images:<relative-path>` and the logical `images` root,
whose registered resolver is `PHOTARA_IMAGES_ROOT`; it never stores the current
host's absolute archive path. Empty strings are not valid stored paths.

Audit those invariants without modifying data:

```console
$ photara cloud storage-audit
```

An asset may belong to more than one Photara project. PostgreSQL project
membership, the additive `projects > <project>` Lightroom keyword, and Adobe
album membership express that relationship without duplicating the asset.
IPTC Job Identifier remains the original/primary shoot label and is not used
to decide project membership.

Manage registry entries through Photara so the same application services can
later back the Lightroom plugin or a GUI:

```console
$ photara people add kylee-nielsen \
    --display-name "Kylee Nielsen" \
    --alias "Kylee" \
    --role model \
    --social "instagram=_kylee_nielsen_"
$ photara people list
$ photara people show kylee-nielsen
```

Locations and scenes follow the same `add`, `list`, and `show` pattern. Pass
`--replace` to intentionally update an existing registry entry; omission is a
guard against accidental replacement.

## Development database

Photara reads its Neon development connection from
`PHOTARA_DEV_DATABASE_URL`. The variable may be supplied by Apogee, another
secret manager, a shell, or any process supervisor; Photara does not depend on
how it is loaded.

Use the direct Neon URL because Storexa already manages a SQLx connection
pool. Verify the configured database without changing its schema:

```console
$ photara health
```

Apply Photara-owned migrations:

```console
$ photara migrate
```

Initialize a project after its scene, location, and people exist in the
registries:

```console
$ photara project init red-meridian \
    --display-name "Red Meridian" \
    --scene architectural-portrait \
    --location golden-gate-bridge \
    --person trinity-woodward
```

The operation is idempotent. Repeating the same command verifies the existing
database record and `project.json`; supplying conflicting values fails rather
than silently changing project identity.

Correct an existing project's associations without changing its durable ID:

```console
$ photara project configure red-meridian \
    --display-name "Red Meridian" \
    --scene architectural-portrait \
    --location golden-gate-bridge \
    --person trinity-woodward
```

Generate the JSON contract that the thin Lightroom plugin applies. This command
is read-only with respect to Lightroom:

```console
$ photara metadata plan red-meridian
```

The plan contains Photara-managed IPTC values, hierarchical people and
workflow keywords, and idempotent smart-collection definitions for the People,
Locations, Scenes, and Projects trees.

## Lightroom Classic plugin

Install or verify the exact Lightroom bundle carried by the running release:

```console
$ photara plugin install
$ photara plugin status
$ photara plugin uninstall
```

Reload it in Lightroom Classic's Plug-in Manager, then run **Library > Plug-in
Extras > Validate Photara Connection** for a read-only bridge check. Select the shoot
photos, then choose
**Library > Plug-in Extras > Apply Project to Selected Shoot**. The plugin asks
for an existing Photara project, previews the managed values, and requires
confirmation before changing the catalog.

The Lua layer only executes the plan. It preserves titles, captions, ratings,
flags, color labels, unrelated keywords, and unrelated collections. Lightroom's
public SDK does not provide a supported command to force an XMP write, so after
application use **Metadata > Save Metadata to File**, or enable Lightroom's
automatic XMP writing preference.

Each project branch uses semantic collection sets: `Originals`, `Selections`,
`Cloud`, and `Masters`. Numeric ordering prefixes are intentionally avoided
because Lightroom displays actual asset counts beside collection names.
The `Selections` group includes Client Favorites, Client Shortlist, Hero, and
Photographer Final; Hero is intentionally neutral about who chose it.

## Client selections

Pixieset proofing remains temporary and provider-specific. Photara imports the
three exported favorite-list CSVs into durable, provider-neutral selection
memberships and maps proof basenames back to unique original RAW filenames.
Because Pixieset's downloaded filenames are opaque, callers must explicitly
assign each CSV rather than relying on its filename:

```console
$ photara selections import-pixieset red-meridian \
    --source-root /path/to/originals \
    --client-favorites /path/to/favorites.csv \
    --client-shortlist /path/to/shortlist.csv \
    --hero /path/to/hero.csv
```

The import is validated and atomic, stores the source evidence and checksum,
and can be repeated safely. In Lightroom, run **Library > Plug-in Extras >
Apply Imported Selections** to apply the resulting keywords and reconcile the
smart collections. Effective workflow membership is hierarchical: Hero implies
Client Shortlist and Client Favorite, and Client Shortlist implies Client
Favorite. Direct provider memberships remain unchanged for auditing. Operator
corrections are separate overrides and never rewrite retained Pixieset
evidence:

```console
$ photara selections add copper-mist --asset DSC09984.ARW --to hero \
    --reason "Photographer correction"
$ photara selections remove copper-mist --asset DSC09993.ARW --from hero \
    --reason "Added accidentally"
$ photara selections status copper-mist --asset DSC09993.ARW
$ photara selections history copper-mist --asset DSC09993.ARW
```

Adding Hero implies Shortlist and Favorite; adding Shortlist implies Favorite.
Conflicting removals fail closed unless `--cascade` is explicit.

## Lightroom Cloud evidence

Photara distinguishes an upload attempt from confirmed Cloud presence. Existing
manual Lightroom Desktop imports can be adopted as user-confirmed evidence;
future Adobe API inventory uses the same asset-scoped ledger with stronger
provider verification.

## Photographer Final

Photographer Final is the photographer's independent editorial decision; it is
not computed as a subset or union of client selections. In Lightroom Classic,
select project camera originals and choose **Add Selected to Photographer
Final** or **Remove Selected from Photographer Final**. Photara fingerprints
the originals, stores portable `images:<relative-path>` identities, commits the
database decisions, and then reconciles only
`workflow|selection|photographer-final` in Lightroom.

The equivalent operator interfaces are:

```console
$ photara decisions add red-meridian --original /path/to/DSC05181.ARW
$ photara decisions remove red-meridian --original /path/to/DSC05181.ARW
$ photara decisions plan red-meridian
```

These services are intentionally outside Lua so a future standalone Photara UI
can call the same decision boundary. Lightroom keywords and XMP are the
portable projection of the database state, not the business-rule engine.

Changing an editorial decision after Cloud delivery uses a guarded withdrawal.
Photara records the exact Adobe asset ID and DNG filename before the operator
deletes it in Lightroom Desktop. Because Adobe's public Lightroom API does not
document asset deletion, Photara never calls an inferred endpoint. It refreshes
the complete provider inventory and finalizes the withdrawal only after that
exact ID is absent:

```console
$ photara cloud withdrawal-plan red-meridian --account personal --original /path/to/DSC05424.ARW
$ photara cloud begin-withdrawal red-meridian --account personal --original /path/to/DSC05424.ARW --confirm
# Delete the reported DNG from All Photos and then permanently from Deleted.
$ photara cloud verify-withdrawal <withdrawal-id> --account personal
```

After verification, select the retained RAW in Lightroom Classic and choose
**Apply Verified Cloud Withdrawal**. Photara removes only Photographer Final
and Cloud Present, after which **Metadata > Save Metadata to File** updates the
existing XMP. The RAW, XMP, asset record, transfer batch, hashes, remote ID,
and append-only decision history remain intact.

Photara can also project project organization into Lightroom Cloud without
copying any media. The compact hierarchy uses provider-owned folders and a
project leaf album:

```text
Locations / <location> / <project>
Scenes    / <scene>    / <project>
People    / <person>   / <project>
Projects  / <project>
```

Preview and then synchronize the projection explicitly:

```console
$ photara cloud collection-plan red-meridian --account personal
$ photara cloud sync-collections red-meridian --account personal --confirm
```

The same provider-verified DNG may be referenced by every relevant project
album. Adobe stores one asset; album membership does not create another file.
Photara uses deterministic IDs, manages only collections created by its own
Adobe client, verifies every expected membership, and records the completed
projection in PostgreSQL.

Preview the guarded Cloud transfer derived from Photographer Final, then
reserve its immutable manifest separately:

```console
$ photara cloud transfer-plan red-meridian --account personal
$ photara cloud reserve-transfer red-meridian --account personal
```

The equivalent Lightroom action is **Prepare Photographer Final DNGs**. It
compares every final asset with the latest complete Adobe inventory, shows
planned versus already-present counts, and requires confirmation before writing
the batch. Reservation is idempotent by account, project, inventory snapshot,
and manifest contents. A separate confirmation renders only the pending DNGs
into a batch-specific operational cache, then validates their reserved names,
TIFF/DNG headers, sizes, SHA-256 fingerprints, and RAW provenance. The export
is resumable and never overwrites a staged file. It does not upload or delete
files.

The Lightroom confirmation offers **Test One** so a single canary DNG can be
inspected before resuming the batch with **Prepare All**.

The staging root is operational data, not configuration. Override it with
`PHOTARA_STAGING_ROOT`; otherwise Photara uses
`$XDG_CACHE_HOME/photara/transfers` (or the conventional `$HOME/.cache`
fallback). The CLI lifecycle used by the plugin is also available directly:

```console
$ photara cloud export-batch <batch-id>
$ photara cloud record-export <batch-id> --asset <asset-id> --file <exact-dng-path>
$ photara cloud finish-export <batch-id>
$ photara cloud upload-preflight <batch-id> --account personal
$ photara cloud upload-canary <batch-id> --account personal
$ photara cloud verify-canary <batch-id> --account personal
$ photara cloud upload-remaining <batch-id> --account personal
$ photara cloud verify-batch <batch-id> --account personal
$ photara cloud cleanup-batch <batch-id> --confirm
```

The upload preflight is read-only. It requires a fully validated export batch,
then checks Adobe's current Lightroom entitlement, catalog, and available
storage against the exact staged byte total. Remote asset creation remains a
separate, explicitly confirmed operation.

`upload-canary` creates and uploads exactly one Adobe asset, then refreshes the
complete provider inventory and requires the observed remote filename and
SHA-256 to match the validated DNG. If Adobe processing outlasts the first
inventory refresh, `verify-canary` resumes verification without uploading the
master again.

After the canary is verified, `upload-remaining` transmits each remaining DNG
sequentially and persists progress item by item. It performs one complete
inventory refresh afterward and marks the batch complete only when every
remote ID, reserved filename, and SHA-256 matches. `verify-batch` resumes the
final inventory gate without retransmitting files.

`cleanup-batch` is a separately confirmed, restart-safe operation available
only after the batch is complete. Before removing anything it rechecks every
remote ID, reserved filename, and SHA-256 against the latest Adobe inventory;
revalidates every staged DNG; rejects symlinks, path traversal, unexpected
files, and non-empty directories; and records each working DNG as removed. It
uses non-recursive deletion and never targets camera RAWs, XMP sidecars, or
remote assets. Repeating a completed cleanup is safe.

Import a fully reconciled Proetus ledger only after explicitly confirming that
its rows are present in Lightroom Cloud:

```console
$ photara migrate
$ photara cloud import-proetus \
    --database "$PROETUS_DB_PATH" \
    --account personal \
    --confirmed-present
$ photara cloud status --account personal
```

The importer requires every legacy row to be `uploaded`, `approved`, and
`removed`, fingerprints the SQLite source, stores its complete row evidence,
and derives paths relative to the historical `Images` root. Evidence can exist
before an original is registered; later reconciliation links it to an asset and
optionally to an Adobe catalog asset ID.

For the first read-only Adobe API connectivity test, configure the OAuth Native
App values through the process environment and run:

```console
$ photara cloud adobe-probe
```

On macOS, Photara installs a private, locally compiled callback helper under its
XDG cache directory, registers Adobe's generated custom URL scheme with Launch
Services, and removes the captured one-time callback immediately after reading
it. The access token remains only in memory, is used to retrieve the Lightroom
catalog identity, and is never printed. After the catalog request succeeds,
Photara opens a local confirmation page stating that the browser tab can be
closed.

After the probe succeeds, authorize durable access once and store only the
refresh token in the operating system credential store:

```console
$ photara cloud adobe-login --account personal
$ photara cloud adobe-verify --account personal
```

`adobe-verify` performs no browser login. It retrieves the refresh token from
macOS Keychain, exchanges it for a memory-only access token, rotates the stored
refresh token when Adobe returns a replacement, verifies the catalog, and
updates the account's remote catalog ID in Photara's provider-neutral ledger.
Use `photara cloud adobe-logout --account personal` to remove the credential.

Enumerate every complete Lightroom Cloud image, persist the provider snapshot,
and reconcile it against the latest Proetus evidence import:

```console
$ photara migrate
$ photara cloud adobe-inventory --account personal
```

The inventory follows Adobe's pagination links, refuses links outside the
authenticated `lr.adobe.io` origin, and matches only filenames that are unique
on both sides. A count alone is not treated as proof; the report separately
identifies missing, extra, and ambiguous evidence.

For legacy Proetus rows whose staging `dng_path` was cleared, reconciliation
derives only the stable `<ORIGINAL_STEM>_<CAPTURE_DATE>_` prefix from the dated
archive path. It does not assume an author suffix and accepts the association
only when the prefix identifies exactly one evidence row and one Adobe DNG.

Generate the immutable Lightroom handoff from the latest fully reconciled
inventory, or apply it through **Library > Plug-in Extras > Apply Verified
Cloud Presence**:

```console
$ photara cloud presence-plan --account personal
```

The plugin matches originals by their archive-relative paths, excludes virtual
copies, and adds only `workflow|cloud|present`. Its scope defaults to the
selected verified originals, allowing a one-photo test before applying all
matched originals. Repeated application is idempotent.

See [lightroom/README.md](lightroom/README.md) for development installation and
bridge configuration.

## Representation ownership

Camera RAW names are immutable. RAWs and XMP sidecars live only in the dated
image archive; working DNGs live in Lightroom Cloud; layered PSBs return beside
their original RAWs; and flattened TIFF masters live in their project folder.
Photara records relationships and never creates permanent convenience copies.

After the UXP report verifies every generated master, preview and then confirm
promotion into the authoritative archive location:

```console
$ photara masters promote red-meridian
$ photara masters promote red-meridian --confirm
```

Promotion copies each PSB beside its RAW through a verified temporary file,
atomically installs it with the uppercase downstream name, records the edited
DNG and layered PSB provenance in PostgreSQL, begins the PSB in `editing`
state, and only then removes the redundant inbox PSB. DNG cleanup is a later,
separately guarded operation.

Use the Lightroom Classic **Import Verified Layered Masters** action to import
those exact PSBs in place. Photara marks them through read-only plug-in metadata
stored only in Lightroom's catalog and exposes them through smart collections
that also require native PSB file type. It does not add IPTC or keywords to the
layered files; Photoshop is the sole PSB writer. Existing catalogs made with
the earlier keyword-driven master collections can run **Reconcile Layered
Master Collections**, then **Metadata > Read Metadata From File** once, to
clear catalog-versus-disk badges without losing membership. Open masters with
**Edit Original**.

During Photoshop raster work, checkpoint the authoritative layered documents
whenever useful. When dodging, burning, Generative Fill, canvas extension, and
other 16-bit raster edits are complete, run the installed **Prepare Photara
HDR-SDR Master.psjs** against the active PSB. It wraps the complete layer stack
in one embedded `16-bit` Smart Object, makes an ordinary shared-source
duplicate named `HDR` above `SDR`, converts the parent to 32-bit Display P3
Linear without merging or rasterizing, and only then opens Camera Raw Filter on
SDR for the operator to disable HDR and author the SDR appearance. SDR
authoring therefore cannot gate the document contract. The script binds every
operation to the starting document ID, validates the paired Smart Object
structure, and does not save.

Install or refresh all master-workflow scripts independently at any time:

```console
$ photara masters install-scripts
```

After inspecting and saving the result, preview and confirm the transition to
flattening readiness:

```console
$ photara masters checkpoint red-meridian
$ photara masters mark-ready red-meridian
$ photara masters mark-ready red-meridian --confirm
```

Each checkpoint refreshes the current PSB size and SHA-256 and records an
append-only workflow event. `mark-ready` uses the same inspection but requires
explicit confirmation before changing workflow state.

The final layered-master contract is 32-bit HDR P3. After raster editing,
refresh and confirm readiness, then generate the project-scoped flattening
handoff:

```console
$ photara masters mark-ready red-meridian --confirm
$ photara masters prepare-flattening red-meridian
```

Run the generated **Flatten Photara Masters.psjs** through Photoshop. Choose
the reported Red Meridian project directory first and the configured Images
root second. The UXP script opens each authoritative PSB, duplicates and
flattens it without modifying the layered source, writes one uppercase `.TIF`
directly to `projects_root/red-meridian/masters/flattened/`, reopens it, and
records 32-bit depth, Photoshop's Display P3 Linear HDR working profile, and a
one-layer result. Photara then
independently verifies the TIFF headers and fingerprints before registration:

```console
$ photara masters verify-flattening red-meridian
$ photara masters register-flattening red-meridian
$ photara masters register-flattening red-meridian --confirm
```

If the photographer deliberately replaces an already registered flattened
HDR/SDR TIFF in place, plan and then confirm a targeted provenance-preserving
refresh instead of editing its checksum row:

```console
$ photara masters refresh-flattened red-meridian --asset DSC05382
$ photara masters refresh-flattened red-meridian --asset DSC05382 --override
```

The confirmed refresh retires each changed current rendition and registers a
new authoritative row. Unchanged paired renditions remain current.

An asset is identified by the SHA-256 fingerprint of its original RAW. The
camera filename remains unchanged in the archive. Only downstream
representations use the expanded `<ORIGINAL_STEM>_<YYYY_MM_DD>_<AUTHOR>`
basename; a deterministic fingerprint suffix resolves the rare collision
without overwriting an existing file.

The connection URL must remain outside the repository. Non-secret application
settings live under `$XDG_CONFIG_HOME/photara/`; environment overrides are
optional and can be emitted by any environment manager.

## Layout prototype

Install or verify Photara's immutable global templates. New configuration
files pin the default full-frame template under `[layouts.defaults]`; older
configuration files receive the same default through backward-compatible
deserialization and may add the section explicitly.

```console
$ photara layouts install
$ photara layouts show full-frame@1
```

Create an Instagram post inside the Red Meridian project and add the first
clean full-frame hero. The friendly PSB filename is accepted only at the CLI
boundary; the project JSON stores the stable Photara asset UUID.

```console
$ photara posts init red-meridian package-a --platform instagram
$ photara posts add-full-frame red-meridian package-a \
    --platform instagram \
    --item hero \
    --asset DSC05250_2021_06_11_SUHAIL.PSB \
    --fit crop
$ photara posts resolve red-meridian package-a --platform instagram
$ photara posts prepare-render red-meridian package-a --platform instagram
```

Append a clean two-image stack. The first asset occupies the top 4500×3000
slot and the second occupies the bottom slot; each is independently fitted
without stretching, then identically composed for HDR and SDR:

```console
$ photara posts add-stacked-two red-meridian package-a \
    --platform instagram \
    --item stacked-01 \
    --top DSC05445_2021_06_11_SUHAIL.PSB \
    --bottom DSC05442_2021_06_11_SUHAIL.PSB
$ photara posts prepare-render red-meridian package-a --platform instagram
```

Either stacked placement may explicitly reuse an authored crop from an
existing item that places the same asset. Photara verifies the asset identity
and normalized crop before copying that placement intent:

```console
$ photara posts add-stacked-two red-meridian package-a \
    --platform instagram \
    --item stacked-03 \
    --top DSC05441_2021_06_11_SUHAIL.PSB \
    --bottom DSC05382_2021_06_11_SUHAIL.PSB \
    --bottom-crop-from-item panorama-05382
```

Every ordinary placement has an explicit policy: `fill` covers its target with
an automatic focal-point crop, `contain` fits the complete source inside it,
and `crop` requests manual platform-specific authoring. Aspect mismatch does
not choose the policy. New full-frame and four-grid items default to `crop`;
pass `--fit` while adding them or use `posts set-fit` later for any item/slot.

Add the four-image grid to both platform specifications. Instagram uses
`grid-four@1` with 3:4 cells; Threads automatically uses
`grid-four-threads@1` with 9:16 cells:

```console
$ photara posts add-grid-four sylvan package-a --platform instagram \
    --item grid-01 \
    --top-left DSC01234.ARW --top-right DSC01235.ARW \
    --bottom-left DSC01236.ARW --bottom-right DSC01237.ARW \
    --fit crop
$ photara posts add-grid-four sylvan package-a --platform threads \
    --item grid-01 \
    --top-left DSC01234.ARW --top-right DSC01235.ARW \
    --bottom-left DSC01236.ARW --bottom-right DSC01237.ARW
$ photara posts prepare-authoring sylvan package-a \
    --platform instagram --also-platform threads
```

Policies may differ within one layout:

```console
$ photara posts set-fit sylvan package-a --platform threads \
    --item grid-01 --slot top-left --fit contain
```

Run **Author Photara Placement** and **Capture Photara Placement** once. The
ordered contexts identify their platform. Applying through the primary command
validates both pinned specifications and stores independent 3:4 Instagram and
9:16 Threads transforms:

```console
$ photara posts apply-authoring sylvan package-a --platform instagram
```

Reorder a draft only by supplying an exact permutation of every item ID.
Photara rejects duplicates, omissions, and unknown IDs. An Instagram render
manifest may expand to any positive number of delivery frames up to the
platform maximum of 20 after continuous panoramas are counted:

```console
$ photara posts reorder red-meridian package-a --platform instagram \
    --item hero --item stacked-01 --item full-frame-05217
```

The project-owned post is written to
`projects_root/red-meridian/posts/instagram/package-a.json`. Resolution pins
the exact global template checksum, authoritative PSB, and independently
verified HDR and SDR flattened TIFFs. The PSB contract is top-level `HDR` above
top-level `SDR`; both TIFF records retain provenance to that same PSB. Existing
0.0.8 flattened TIFFs are migrated in place as HDR renditions. Until a verified
SDR-authored rendition exists, the plan reports `ready: false` with an explicit
requirement instead of producing an invalid WSP handoff.

`prepare-render` rechecks the registered byte sizes and SHA-256 fingerprints,
writes `Photara Layout Manifest.json` at the project root, installs **Build
Photara Layouts.psjs** under `~/Pictures/Photara/Scripts`, and creates the
project render directory. Run that UXP script in Photoshop and choose the
project root. For `full-frame@1`, it creates a 4500×6000 Instagram PSB with an
`HDR` layer above an `SDR` layer, both pixel-aligned from the same crop and
ready for visual review and WSP.
`stacked-two@1` uses the same output contract while composing two 2:3
landscape slots into the upper and lower halves of one 3:4 frame.

Append and author a continuous two-frame panorama:

```console
$ photara posts add-continuous-panorama red-meridian package-a \
    --platform instagram --item panorama-05382 \
    --asset DSC05382_2021_06_11_SUHAIL.PSB
$ photara posts prepare-panorama-crop red-meridian package-a \
    --platform instagram --item panorama-05382
```

Run **Author Photara Panorama Crop.psjs**, choose the Red Meridian project
folder, and use **Select > Transform Selection** to position the 3:2 marquee.
Then run **Capture Photara Panorama Crop.psjs** and choose the same folder. It
places a vertical guide at the seam between the two horizontal 3:4 frames; if
needed, adjust the selection and capture it again. Apply the approved report:

```console
$ photara posts apply-panorama-crop red-meridian package-a \
    --platform instagram --item panorama-05382
```

Photara stores normalized source coordinates shared by the HDR and SDR
renditions. It neither crops nor saves the source, and WSP remains responsible
for splitting and resizing the continuous output.

Install and use a versioned Dynamic Range Comparison design reference:

```console
$ photara layouts install-reference dynamic-range-comparison@2 \
    /path/to/3x4_HDR_Compare.psd
$ photara posts add-dynamic-range-comparison red-meridian package-a \
    --platform instagram --item dynamic-range-01 \
    --top DSC05250 --bottom DSC05421
```

Each row compares one asset. Its left cell remains SDR in both WSP layers; its
right cell changes from SDR in the base to HDR in the top layer. The headroom
ramp similarly changes from flat SDR white to the true 1-to-10 HDR gradient.
Photara pins and verifies the PSD checksum before preparing the render.
Images are contained inside the square cells without cropping: portrait images
use side bars, while landscape images use top and bottom bars supplied by the
template background.

After upgrading each PSB to exactly two top-level Smart Objects or groups named
`HDR` and `SDR` (in that order), rerun the readiness checkpoint and prepare the
new handoff. Photoshop renders Smart Filters and writes
`<CANONICAL_BASE>_HDR.TIF` and `<CANONICAL_BASE>_SDR.TIF` together under
`masters/flattened/`. Photara requires both files to be flattened 32-bit
Display P3 Linear TIFFs with identical dimensions before their database records
are atomically registered or replaced.

Repeating the same commands is idempotent. A future project such as Sylvan uses
the same commands, template, and Rust code with only its project slug, post
name, asset choices, and project configuration changed.

Edit Comparison Before TIFFs are also idempotent across targets. Photara
registers each verified Lightroom Reset + Adobe Color export by project asset
and rendering contract, so preparing another platform reuses the same TIFF and
only asks Lightroom to export assets that do not yet have valid evidence.
Shared TIFFs live at `sources/edit-comparison/before/` inside the project. The
Lightroom action asks for the package once and resolves all of that package's
platform specifications automatically.

When publication is performed manually, record the operator confirmation
against the exact current post-specification checksum. Omit unknown provider
URLs or timestamps rather than inventing them:

```console
$ photara posts confirm-manual-publication red-meridian package-a \
    --platform instagram \
    --note "Operator confirmed manual Instagram publication"
$ photara posts confirm-manual-publication red-meridian package-a \
    --platform instagram \
    --note "Operator confirmed manual Instagram publication" \
    --confirm
```

### Cloudinary exact-original backup

Cloudinary currently backs up the exact HDR JPEGs exported by Web Sharp Pro.
It is not the website media model: filename order, social post order, website
order, derivatives, and thumbnails are deliberately outside this contract.
Stage only the final JPEG originals under
`workspace/exports/<platform>/<package>/` in the project. Photara rejects a
missing, extra, duplicate, renamed-to-an-unknown-item, or changed source.
Publication-order prefixes such as `01_hero.jpg` are retained in backup
evidence on both platforms; Instagram requires them and Threads accepts them
while remaining compatible with legacy unnumbered exports.

Authenticate once by supplying the Cloudinary API key and secret through the
environment. Photara verifies the account, then stores the cloud name, key, and
secret in the system keychain under the account label; it never writes them to
the project or delivery manifest:

```console
$ CLOUDINARY_API_KEY="$(secret-manager-read-api-key)" \
  CLOUDINARY_API_SECRET="$(secret-manager-read-api-secret)" \
  photara delivery cloudinary-login --cloud-name CLOUD_NAME
$ photara delivery cloudinary-probe
```

Prepare an immutable manifest, inspect its reported path, upload one canary,
and byte-verify the downloaded Cloudinary original before the remainder:

```console
$ photara delivery prepare red-meridian package-a --platform instagram
$ photara delivery upload-canary BATCH_UUID --confirm
$ photara delivery verify-canary BATCH_UUID
$ photara delivery upload-remaining BATCH_UUID --confirm
$ photara delivery verify BATCH_UUID
```

Uploads are signed and non-overwriting. The Cloudinary public ID is namespaced
as `photara/<project>/<platform>/<package>/<wsp-file-stem>`, while the original
WSP filename remains the leaf. A retry reuses a ledger-recorded asset and a
matching manifest reuses its batch UUID. If an unrecorded public ID already
exists, Photara requires matching byte count and Photara SHA-256 context; it
never replaces a conflicting remote object. Full verification downloads every
original and compares its SHA-256 with the staged WSP file before marking the
batch verified.

## License

MIT
