# Photara Lightroom Classic plugin

`photara.lrplugin` is the thin Lightroom Classic adapter for Photara. Rust owns
the project, metadata, naming, and collection rules. Lua presents the operator
flow and applies the resulting plan through Lightroom's public SDK.

## Development installation

1. Install the current Photara binary at the path configured in
   `photara.lrplugin/Config.lua`.
2. Make sure the configured environment loader supplies
   `PHOTARA_DEV_DATABASE_URL` and `XDG_CONFIG_HOME` without writing secrets to
   standard output.
3. In Lightroom Classic, open **File > Plug-in Manager**, choose **Add**, and
   select the `photara.lrplugin` directory.
4. Choose **Library > Plug-in Extras > Validate Photara Connection**. This
   checks configuration and PostgreSQL access without changing the catalog.
5. Select one or more shoot photos in the Library module.
6. Choose **Library > Plug-in Extras > Apply Project to Selected Shoot**.

## Pixieset selection reconciliation

Photara deliberately leaves Pixieset authentication and proof upload to the
official Pixieset publish service. After the client finishes choosing images,
download one CSV for each explicitly named favorite list: `Client Favorites`,
`Client Shortlist`, and `Hero`. Import them by assigning each file to its
meaning; the opaque Pixieset download filename is never used to infer intent:

```console
$ photara selections import-pixieset red-meridian \
    --source-root /path/to/the/original/raw/folder \
    --client-favorites /path/to/client-favorites.csv \
    --client-shortlist /path/to/client-shortlist.csv \
    --hero /path/to/hero.csv
```

Photara validates the collection name, embedded favorite-list name, and every
proof basename against a unique camera RAW. It retains the source CSV and its
SHA-256 digest in PostgreSQL, and replacing the current memberships is atomic.
Repeating the same import is safe.

Reload the plugin, then choose **Library > Plug-in Extras > Apply Imported
Selections**. The plugin clears and reapplies only the three imported selection
keywords on photos belonging to the chosen project. It also reconciles the
managed smart collections. A Hero implies Client Shortlist and Client Favorite;
a Client Shortlist implies Client Favorite, while the original provider lists
remain preserved unchanged in the database.

## Verified Lightroom Cloud presence

After `photara cloud adobe-inventory` reports a completely reconciled Adobe
inventory, choose **Library > Plug-in Extras > Apply Verified Cloud Presence**.
Photara matches the verified evidence to Classic originals using the full path
below the archive's `Images` directory, never the camera filename alone.

The dialog defaults to selected verified originals when any are selected, so
test one photo first. A second run can apply all matched originals. The action
only adds `workflow|cloud|present`; it does not upload, remove, or rename files.

## Photographer Final

Select camera originals belonging to one Photara project, then use **Add
Selected to Photographer Final** or **Remove Selected from Photographer
Final**. The plugin asks for confirmation, Photara fingerprints and records the
decision, and Lightroom receives only the corresponding managed keyword.

Do not type the hierarchical keyword manually. The explicit add/remove actions
are idempotent, preserve client selections, reject virtual copies and mixed
projects, and provide the application boundary later used by the standalone
UI. Save metadata after the action to persist the keyword to XMP.

When the final set is ready, choose **Prepare Photographer Final DNGs**. The
first dialog is a read-only comparison against the latest complete Adobe
inventory. Confirming it reserves a durable batch with one state per asset;
already-present assets are skipped. A second confirmation renders the pending
camera originals as DNGs into
`$PHOTARA_STAGING_ROOT/<batch-id>` or the default
`$XDG_CACHE_HOME/photara/transfers/<batch-id>`. Photara validates the exact
reserved filename, TIFF/DNG header, byte size, SHA-256, and RAW provenance
before recording each item as exported. Interrupted batches are resumable and
existing staged files are validated rather than overwritten. This action does
not upload or delete anything.

Choose **Test One** for the first run. Photara records one canary DNG and leaves
the batch resumable. Inspect that file, then run the same action again and
choose **Prepare All** for the remaining items.

## Layered master catalog import

After `photara masters promote PROJECT --confirm` installs and registers the
authoritative PSBs beside their camera RAWs, choose **Library > Plug-in Extras
> Import Verified Layered Masters**. The action obtains a read-only plan from
Photara, rechecks each current PSB against its registered size and SHA-256, and
requires every exact camera original to exist in the open catalog.

On confirmation, Lightroom imports each missing PSB in place through
`LrCatalog:addPhoto`. Photara records membership in a read-only custom plug-in
field that lives only in the Lightroom catalog, then creates Masters smart
collections requiring both that project marker and native PSB file type. It
does not add IPTC fields or keywords to layered files. Repeating the action
reuses PSBs that are already imported.
Stacking with the source RAW is optional photographer-controlled catalog
organization and is not required or verified by Photara.

For catalogs created by the earlier keyword-driven master import, select the
project's imported PSBs and choose **Reconcile Layered Master Collections**.
Photara moves their membership to its catalog-only field and rebuilds the
corresponding `Masters > PSB` smart collections with both membership and PSB
file-type guards; it does not change standard photo metadata or write files.
Select the PSBs in the resulting smart collection and use **Metadata > Read
Metadata From File** once to discard stale standard catalog metadata and clear
Lightroom's up-arrow/conflict badges. Use **Edit Original** when opening an
authoritative PSB in Photoshop.

The committed `Config.lua` contains paths, not credentials. Other users can
replace those paths with their own executable and environment loader.

## Safety and ownership

The plugin requires confirmation before applying a project or imported
selection set. It owns only the IPTC fields, people-keyword hierarchy,
workflow-keyword catalog, and collection trees declared by the Photara plan.
It does not modify user titles, captions, ratings, flags, labels, unrelated
keywords, or unrelated collections.

Repeated runs update the same smart collections and converge on the same
managed RAW metadata. Layered-master `PSB` smart collections are the exception
to keyword-based membership: they use Photara's searchable, read-only custom
metadata field, which Lightroom stores only in its catalog and cannot write to
XMP or the PSB.

Lightroom's supported plug-in API does not expose Save Metadata to File. For
camera originals, use Lightroom's automatic XMP preference or invoke that
command after the plugin finishes. Do not invoke it for imported layered PSBs:
keep their Photara metadata catalog-only and let Photoshop be the sole writer
to the master file, especially on SMB storage.
