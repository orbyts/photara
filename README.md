# Photara

Photara is an experimental photography workflow and publishing tool.

The project will provide tooling for managing photographic projects from
selection and editing through derivative generation, media storage, and
publication bookkeeping.

## Status

`v0.0.7` is developing the guarded Lightroom Cloud delivery workflow. Photara owns its
schemas, SQL, repositories, and photography workflow; Storexa owns connection
and transaction plumbing.

See [ROADMAP.md](ROADMAP.md) for the path to the first supported release and
[METADATA.md](METADATA.md) for the Lightroom metadata ownership contract.

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
├── schemas/
└── templates/
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
```

The database stores `images:<relative-path>` and the logical `images` root,
whose registered resolver is `PHOTARA_IMAGES_ROOT`; it never stores the current
host's absolute archive path. Empty strings are not valid stored paths.

Audit those invariants without modifying data:

```console
$ photara cloud storage-audit
```

Manage registry entries through Photara so the same application services can
later back the Lightroom plugin or a GUI:

```console
$ photara people add trinity-woodward \
    --display-name "Trinity Woodward" \
    --alias Trin --alias Trinity \
    --role model \
    --social instagram=@theetr1n1ty \
    --social threads=@theetr1n1ty
$ photara people list --json
$ photara people show trinity-woodward
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

The development plugin lives at `lightroom/photara.lrplugin`. Install it through
Lightroom Classic's Plug-in Manager, then run **Library > Plug-in Extras >
Validate Photara Connection** for a read-only bridge check. Select the shoot
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
Favorite. Direct provider memberships remain unchanged for auditing.

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

An asset is identified by the SHA-256 fingerprint of its original RAW. The
camera filename remains unchanged in the archive. Only downstream
representations use the expanded `<ORIGINAL_STEM>_<YYYY_MM_DD>_<AUTHOR>`
basename; a deterministic fingerprint suffix resolves the rare collision
without overwriting an existing file.

The connection URL must remain outside the repository. Non-secret application
settings live under `$XDG_CONFIG_HOME/photara/`; environment overrides are
optional and can be emitted by any environment manager.

## License

MIT
