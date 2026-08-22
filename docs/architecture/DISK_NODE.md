# Built-in Disk node

Disk is an ordinary independently versioned node package bundled with Photara.
It is the first live-data source and the second definition offered by the node
catalog. It receives no privileged graph semantics merely because it ships in
the application.

Disk is also the first instance of a broader asset-provider family. Future
Dropbox, Google Drive, Box, iCloud Drive/File Provider, Photos/PhotoKit,
Lightroom Cloud, studio DAM, and other source nodes may use provider-specific
authorization and enumeration behind the host boundary while emitting the same
portable `photara.asset-set` contract. No provider identity belongs in Layout
or in Core's general asset semantics.

## Semantic contract

`photara.disk.folder` represents an explicitly user-authorized folder and emits
an ordered `photara.asset-set` value. Downstream nodes see semantic assets, not
paths, bookmarks, volumes, or AppKit objects.

The portable node state may contain:

- a stable folder-binding identity;
- user-authored scan policy such as recursion and supported representation
  selection;
- the ordered semantic asset membership published by the last accepted scan;
- forward-compatible node-owned fields.

It must not contain an absolute path, security-scoped bookmark bytes, current
mount path, sandbox status, file descriptor, availability result, proxy path,
or Gallery selection. Evaluation is deterministic over the accepted authored
membership and emits its `AssetSet` without performing ambient filesystem I/O.

## Permission and runtime binding

The native macOS client obtains folder authority through the standard system
picker. It creates and retains a security-scoped bookmark in native runtime
binding storage keyed by the portable folder-binding identity. Bookmark bytes
are machine-local capability material and never cross into Core or a portable
project document.

A scoped host adapter resolves that identity for a concrete request, starts
security-scoped access for the shortest practical lifetime, and supplies the
runtime service with an authorized folder handle or materialized paths. A
future Windows client provides its own binding adapter behind the same semantic
identity. Rebinding a folder changes runtime resolution, not node identity.

iCloud Drive content exposed as ordinary File Provider locations may work
through Disk and the system picker. A Photos integration should be a separate
provider node when it relies on PhotoKit library identities and authorization
rather than durable file paths. OAuth tokens, PhotoKit grants, account handles,
and cloud materialization URLs follow the same device/account capability
boundary as security-scoped bookmarks and never enter the portable project.

Missing, stale, moved, unmounted, or denied folders produce structured runtime
diagnostics and recovery actions. They do not erase the node, its last accepted
asset membership, or downstream authored state.

## Live-data reconciliation

Scanning is an explicit user or evaluation request, not an undeclared project
side effect. The host fingerprints supported representations and prepares a
reconciliation proposal. One revision-checked Core command atomically publishes
accepted asset/representation changes and the Disk node's ordered AssetSet.
Changed bytes update representation fingerprints; stable files retain semantic
identity; disappeared or inaccessible files are diagnosed according to scan
policy rather than silently destroying identity.

The first Stage 9 adapter may support a deliberately narrow visual-file subset
needed by real Layout projects. Format/container metadata remains separate from
consumer capabilities, and the node contract is not TIFF-specific.

## Catalog and presentation

Disk appears under a versioned package-supplied catalog path such as
`Input > Filesystem`. Layout appears under `Create > Layout`. Catalog paths,
search terms, icons, Inspector contributions, and optional Workspace hints are
presentation metadata; they are not Core evaluator variants.

Disk uses the generic Inspector plus a compact definition-specific control
surface for choosing/rebinding the folder, scan policy, refresh, availability,
and diagnostics. It does not advertise a rich canvas Workspace. Layout remains
the first node that does.

## Persistence boundary

Stage 9 uses the existing portable project plus filesystem state service. Disk
does not introduce a database. Stage 10 may add a database-backed Core state
service with explicit user/definition, project/instance, and device scopes, but
node code receives a scoped service/capability rather than SQL credentials or
ambient database access. A future synchronized Disk preference belongs to its
user/definition namespace; the macOS folder grant remains device-only.
