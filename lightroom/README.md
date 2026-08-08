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

The committed `Config.lua` contains paths, not credentials. Other users can
replace those paths with their own executable and environment loader.

## Safety and ownership

The plugin requires confirmation before applying a project or imported
selection set. It owns only the IPTC fields, people-keyword hierarchy,
workflow-keyword catalog, and collection trees declared by the Photara plan.
It does not modify user titles, captions, ratings, flags, labels, unrelated
keywords, or unrelated collections.

Repeated runs update the same smart collections and converge on the same
managed metadata. A regular collection occupying a required smart-collection
name is treated as a conflict rather than replaced.

Lightroom's supported plug-in API does not expose Save Metadata to File. Use
Lightroom's automatic XMP preference or invoke that command after the plugin
finishes.
