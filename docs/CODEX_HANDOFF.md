# Codex handoff

## Resume objective

Build generation two toward the first daily-usable `0.2.0` application. Work
Rust-first: complete the general Core, node package, persistence, asset/proxy,
and built-in Layout semantics before building the production SwiftUI/AppKit UI.
Do not wait for all of that Rust work before testing interoperability: run one
tiny Swift bridge spike as soon as the first real Core command/evaluation path
is concrete, then return to the Rust implementation before building the full
client.

Read in order:

1. `README.md`
2. `ROADMAP.md`
3. `docs/architecture/README.md`
4. the focused architecture documents linked there
5. this handoff

## Locked decisions

- Photara is a code name; branding is deferred.
- `0.2.0` is a clean application generation, not an extension of the old CLI.
- `0.2.0` ships only on macOS.
- Core and heavy logic are portable Rust.
- The Mac client is SwiftUI/AppKit, with AppKit/Metal where appropriate.
- A future Windows-native client uses the same semantic application facade.
- The workflow model is Houdini-like procedural composition with general typed
  nodes, evaluation, dirty propagation, caches, artifacts, and receipts.
- Layout is the first built-in package under `photara.layout`; it uses the same
  package contract future downloadable nodes use.
- Core has no fundamental Source/Layout/Host/Destination or media-kind node
  variants.
- One Core state service owns shared records. Nodes receive namespaced state and
  scoped capabilities, not ambient database access.
- Legacy data import, provider nodes, final branding, marketplace, macros, and
  Windows are not `0.2.0` blockers.

## Current tree

```text
crates/photara-core
crates/photara-node-sdk
crates/photara-store
crates/photara-bridge
nodes/photara-layout
nodes/photara-asset-set
nodes/photara-disk
```

The workspace is `0.2.0-alpha.0`. Core now contains canonical namespaced IDs,
separate package/definition/value-type/schema versions, typed-value descriptors
and a minimum registry, generic port compatibility, version-pinned node
instances, identified connections, configuration/authored-state separation,
canonical JSON/SHA-256 digests, graph documents/revisions, and diagnostics. It
now also applies revision-checked Add Node, Connect, Set Configuration, and
Set Authored State commands; validates exact definitions, schemas, ports,
cardinality, and cycles; and evaluates a small graph deterministically through
a general node-runtime callback with request/evaluation identity, progress,
cooperative cancellation, structured failures, and per-node dirty keys. The
scaffold also contains a revision-safe in-memory repository, immutable bridge
DTOs, and a Layout package expressed through the ordinary node SDK.

Portable authored state now has two validated JSON boundaries. `ProjectDocument`
contains project identity/revision, human metadata, exact package requirements,
the existing `GraphDocument`, and semantically identified project-relative
resources. `NodeGraphDocument` exports that same configured graph and topology
as a standalone shareable file without project identity or resource inventory.
Both use canonical digests, preserve generic/unknown node state where practical,
and exclude runtime, caches, secrets, machine paths, and workspace UI state.

Stage 4A is complete. Exact package manifests validate and rebuild an ordinary
package/definition registry. Backend-neutral repositories now persist exact
manifest registrations and whole portable Project Documents through in-memory
and atomic filesystem adapters. The real Layout package, exact node pins,
configuration, authored state, future unknown state, save/reopen, and stale
revision rejection pass together against a brand-new store with no legacy
database dependency.

Stage 5 is complete. Project Documents now contain semantic asset context with
separate asset, representation, project-resource, and stable runtime-binding
identity; representation roles/capabilities and SHA-256 content fingerprints;
and no serialized machine locator, availability, materialization, proxy, cache,
or Gallery-selection state. Core
defines an explicit ordered `photara.asset-set` typed value used by Layout's
input port. The local development adapter imports paired HDR/SDR TIFF paths as
two renditions of one asset, verifies local materialization, survives path
moves, and detects/refreshes changed content without decoding TIFFs.

Stage 6A is complete. Core defines backend-neutral proxy requests, fully
versioned profiles, exact profile references, descriptors, and cache keys over
source fingerprint plus all output-affecting policy. The reproducible Quasar
benchmark compares ImageIO/Core Image, libvips, ImageMagick, and Rust `image`
against 8000×5333 high-entropy paired SDR/HDR TIFFs plus orientation. The
optimized ImageIO floating-point thumbnail path passed exact color, ICC,
orientation, F16, negative-sample, and HDR-headroom checks and measured 0.45 s
for SDR thumbnails and 1.09 s for HDR authoring previews. It is the selected
first macOS backend; details and candidate limitations are in
`docs/architecture/PROXIES.md`.

Stage 6B is complete. The runtime-only `photara-proxy` crate provides a
project-scoped content-addressed cache with atomic publication, verified hits,
quota/LRU eviction, corruption recovery, unavailable-source retry, and bounded
generation. In-flight deduplication happens before slot acquisition. The
initial bound is one active generation, based on the measured 954 MiB isolated
HDR peak and near-linear one-versus-two helper RSS scaling, not CPU count. The
selected ImageIO/Core Image implementation is a short-lived macOS helper built
with Xcode 26.6 and no macOS 27 API; no Apple imaging object enters Core.

## Local operator environment

These notes describe Suhail's development machine and how Codex should operate
on it. They are conveniences for development and possible legacy recovery, not
generation-two product configuration or Core API contracts.

### Development machines

- `quasar` is the M1 Mac Studio and current reference development machine. Keep
  Rust/Core, persistence/package work, and the Swift bridge compiling with its
  stable macOS 26.5.2, Xcode 26.6, and Swift 6.3.3 environment.
- `eclipse` is the M1 MacBook Pro reserved for later macOS 27/Xcode 27 native UI
  and Layout Inspector experimentation. It is not available to this Codex
  context.
- Never make the bridge depend on macOS 27 UI or SDK APIs. New UI design work
  stays in the Eclipse-side SwiftUI/AppKit presentation layer above the facade.

### Shell execution

- The repository is
  `/Users/suhail/Library/CloudStorage/Dropbox/matrix/crates/photara`.
- Ghostty receives the user's full shell environment, but do not open Ghostty
  merely to run a command or obtain that environment. Run builds, tests,
  inspections, and other noninteractive work directly in the background shell.
- Use a terminal UI only when the user asks for one or an operation genuinely
  requires the user's interaction. Otherwise, keep long-running work in the
  background and report progress through Codex.
- Request one narrowly scoped macOS or Codex sandbox permission when required.
  Do not replace a permission request with instructions for the user to rerun a
  background-capable command manually.

### Apogee and environment

- Apogee `0.1.3` is installed at `/opt/homebrew/bin/apogee` and supplies the
  machine's shell environment through its existing shell integration. A login
  background shell should inherit that environment without launching Ghostty.
- Do not run bare `apogee` merely to inspect its output: its command contract is
  to emit shell configuration, which may include values that should not appear
  in captured logs.
- Useful non-secret environment names include `DROPBOX`, `PHOTARA_REPO`,
  `XDG_CONFIG_HOME`, `XDG_CACHE_HOME`, `XDG_DATA_HOME`, and `XDG_STATE_HOME`.
- Existing legacy variables include `PHOTARA_CONFIG_ROOT`,
  `PHOTARA_IMAGES_ROOT`, `PHOTARA_PROJECTS_ROOT`,
  `PHOTARA_DEV_DATABASE_URL`, `PHOTARA_DEV_DATABASE_URL_POOLED`,
  `PHOTARA_ADOBE_CLIENT_ID`, and `PHOTARA_ADOBE_REDIRECT_URI`. Their presence
  does not authorize generation-two code to depend on them. Consult `v0.1.0`
  only when deliberately recovering or importing legacy behavior.
- Check whether a variable exists without printing its value. Never capture a
  complete environment in task output.

### Secrets and credentials

- The 1Password CLI is installed at `/opt/homebrew/bin/op`. Retrieve a secret
  only when the current operation requires it, keep it in memory for the
  shortest practical time, and avoid placing it in command arguments, files,
  documentation, Git, or task output.
- Existing 1Password references may be used without revealing their resolved
  values. For example, legacy Cloudinary credentials live at
  `op://API/Cloudinary/API Key` and `op://API/Cloudinary/API Secret`; the known
  non-secret cloud name is `dicttuyma`.
- Prefer an already established keychain or provider session. If macOS requires
  authorization, perform credential access serially so the user receives one
  meaningful prompt rather than several concurrent Keychain dialogs.
- Never rotate, replace, persist, or change credentials, Keychain ACLs, or
  secret-manager configuration unless the user explicitly requests it.

### Network volume

- `whisk` is an SMB share normally mounted at `/Volumes/whisk`. macOS may
  unmount it after inactivity.
- If work needs the share and `/Volumes/whisk` is unavailable, reconnect it from
  the background shell using the machine's existing macOS network credentials
  and configuration. Do not ask the user to mount it in Finder first.
- Reuse `/Volumes/whisk`; do not change the SMB server configuration,
  credentials, permissions, mount destination, or persistent macOS network
  settings merely to reconnect it.
- After mounting, verify `/Volumes/whisk` is accessible before performing the
  dependent operation. If mounting or access requires macOS Network Volumes or
  Codex sandbox permission, request that scoped permission and continue after
  approval.
- Historical roots were `/Volumes/whisk/Pictures/Images` and
  `/Volumes/whisk/Pictures/Projects`. They are legacy/operator context, not
  generation-two identity or storage contracts.

## Historical archive

The official legacy archive is:

```text
tag:    v0.1.0
commit: 5b33e5981396fea5ab976bc9a3d75cdea8ccd5a0
```

Do not restore the old tree wholesale. Inspect or salvage a specific file with
Git when a future node needs proven behavior:

```console
git show v0.1.0:src/layout.rs
git show v0.1.0:photoshop/Build\ Photara\ Layouts.psjs
git show v0.1.0:lightroom/photara.lrplugin/Photara.lua
git ls-tree -r --name-only v0.1.0
```

The archived tree included:

```text
src/                    legacy Rust CLI/domain/provider implementation
migrations/             PostgreSQL migrations 0001 through 0019
lightroom/               Lightroom Classic Lua plug-in and documentation
photoshop/               PSJS master, authoring, flattening, and layout scripts
templates/               immutable v0.1 layout template JSON/reference contracts
tests/fixtures/           Red Meridian and later compatibility fixtures
README.md                complete legacy CLI/operator reference
ROADMAP.md               historical product and release decisions
LAYOUTS.md               layout/HDR/WSP implementation reference
METADATA.md              Lightroom metadata ownership contract
docs/PHOTOGRAPHER_GUIDE.md
docs/architecture/       v0.1 analysis and the superseded migration study
```

Potential salvage includes pure geometry, fingerprints, atomic-network-write
lessons, Lightroom Lua, Photoshop PSJS, template measurements, provider
reconciliation, and their tests. Adapt them behind new node/application
contracts; never make legacy tables, config, paths, or provider types Core APIs.

## Immediate next work

The Roadmap section 3 Swift bridge spike is complete. Its disposable harness
has been removed; the historical result and transport comparison remain in
`docs/architecture/SWIFT_BRIDGE_SPIKE.md`.

Roadmap Stage 8 is complete. The production UniFFI facade and Swift gate live in
`photara-bridge` and `platform/macos/photara-app`. Rust alone decodes Layout
authored state and exposes typed inspection DTOs. The first vertical slice has
real create/open/save/close/reopen lifecycle, paired local TIFF import into
project resources, an ordinary explicit `AssetSet` source, explicit Layout
binding, Gallery thumbnails, and a proxy-backed Layout preview. Proxy payloads
cross as leased verified file references with color/HDR descriptors, never
pixel buffers. Gallery placement, visibility, filters, and selection remain
Swift workspace state and the verification proves they cannot change the graph
digest.

Roadmap Stage 9 is in progress. The facade now exposes deterministic resolved
normalized and pixel rectangles alongside typed Layout inspection and accepts
semantic structure, Fit/Fill/Crop, focal/alignment, rotation, crop, and explicit
assignment commands. History stores exact forward/reverse Core graph
transactions, so assignment's AssetSet-plus-Layout batch and ordinary Layout
edits undo and redo coherently under revision checking. The macOS workspace has
a separately identified Layout Authoring surface that composes shared per-cell
proxy references, plus conventional Inspector controls. Crop dragging keeps
only transient Swift translation during pointer movement and commits one Core
command on gesture completion. Keep Stage 9 open until the complete real-project
gate in `ROADMAP.md` is exercised and hardened. Exact definition metadata now
owns each node's independent brand, neutral icon resource, hierarchical catalog
path, Inspector contribution, and optional Workspace contribution. The facade
returns that immutable catalog and the Swift Tab popover renders it generically.

`photara.disk.folder` is the second visible bundled definition and first live
asset provider. It is an ordinary no-Workspace node under Input → Filesystem.
Portable authored state contains a stable folder-binding UUID, scan policy, and
last accepted `AssetSet`; macOS security bookmarks and paths are device-only.
The Disk Inspector can grant/rebind, explicitly scan, and explicitly connect to
an available Layout. Scanning fingerprints supported visual files, publishes
stable semantic assets through Core commands, and supplies runtime-resolved
representations to the existing shared proxy service. Tests prove attaching a
folder does not change the graph digest and that save/reopen preserves semantics
when the grant is unavailable. Keep the graph plain and do not begin Stage 4B
lifecycle work, graph polish, advanced docking, Photoshop/UXP, or macros.

The Stage 9 internal architecture checkpoint makes these ownership boundaries
concrete without changing observable behavior. The application host now
assembles an exact-definition runtime registry instead of a built-in fallback
chain. `photara-disk` owns folder enumeration, cheap and verified revisions,
stable provider identities, and reconciliation preparation; the bridge only
publishes the prepared result through revision-checked Core transactions.
`photara-layout` no longer depends on or invokes the proxy service. The
production facade is split across runtime registration, evaluation, and asset
materialization modules, while Swift Gallery state, behavior, and views are
separate product surfaces. The disposable Swift bridge harness was removed;
its historical findings remain documented.

The current performance refinement keeps Disk Finder-like: every file remains
an independent asset and no HDR/SDR pairing is inferred. Disk fingerprinting
and production proxy calls run outside Swift's main actor. Gallery requests a
runtime-only Quick Look thumbnail first; Layout shows the same immediate tiny
path and then upgrades to a shared verified 1K-default F16 HDR-preserving proxy.
Swift uses constrained HDR presentation so highlights remain controlled on HDR
displays and system-mapped on SDR displays. Native thumbnail paths and images
remain non-authoritative. Exact definitions also advertise a neutral default
activation: Disk opens its granted Finder folder and Layout focuses/reveals its
authoring Workspace. Bare `Tab` is captured while Graph is visible and opens
the catalog at the current graph pointer, falling back to center before the
pointer has entered.

The next performance invariant is now encoded directly: publish assets after
cheap discovery, progressively show the best available preview, and verify
bytes in the background. Disk first reconciles stable file identities using a
size/mtime observation fingerprint, then streams SHA-256 verification at
utility priority. Representation descriptors state whether their revision is a
content digest, provider revision, or file observation. Gallery keeps an older
preview visible while requesting progressively better display data and shows
transient loading/updating state. Disposable shared proxy generation may use an
observation-only revision only after the runtime materializer immediately
revalidates the same file evidence. Preserve this provider-neutral
evidence/preview separation for future Lightroom, cloud, and other nodes.

The live `_SUH5024…HDR.TIF` fixture is 6336×9504, 32-bit float, LZW, 761 MiB.
An uncontended Quick Look 384 px request exceeded 60 seconds; the bundled
ImageIO/Core Image helper produced a 384 px F16 HDR preview in 2.91 seconds.
Disk TIFF Gallery requests therefore route immediately to the bounded helper
using revalidated file-observation evidence. Other formats retain the bounded
Quick Look ladder. Whole-byte hashing waits until currently requested previews
finish, and the resulting digest promotes already displayed previews rather
than regenerating them. Re-test `/Users/suhail/Desktop/flattened`; first image
is expected in roughly three seconds on Quasar. Four simultaneous samples then
completed individually in 2.60–2.66 seconds at about 733 MiB peak process
footprint each. The app now chooses four project generation slots on machines
with at least 64 GiB physical memory, two with at least 24 GiB, and one below
that; `photara.proxy-generation-concurrency.v1` may explicitly select 1–4.
Request deduplication still occurs before this limiter. The helper's default
Core Image context can use the GPU, but these giant LZW float TIFFs are
decode/memory dominated, so this primarily improves fill rate.

Gallery constrains every cell from the first square placeholder onward,
preventing late dimensions from drawing across a neighbor. Photo Grid adopts
known proxy/native aspect ratios into justified rows with two-point gutters;
Square Grid stays square and adds a compact filename plus a portable format
pill. Disk discovery
now recognizes common camera RAW extensions plus TIFF, JPEG, PNG, HEIF/HEIC,
AVIF, JPEG XL, PSD/PSB, EXR, WebP, GIF, and BMP. Recognition means the asset is
published; Quick Look or the ImageIO helper must still be able to decode its
particular encoding. Video remains future work. The macOS folder chooser dims
files by design because the grant targets a directory; its prompt now explains
that supported contents will be discovered.

Every Gallery request also enters the project-scoped shared proxy service.
Quick Look can still provide the earliest native pixels, but it cannot leave an
asset permanently outside Photara's cache. Reopening the same project validates
the exact cache-key directory and returns the existing proxy without invoking
the generator; `project_cache_survives_service_reopen_without_regeneration`
proves this boundary. The Gallery's View button and context action present a
full proxy only after it exists. Double-click retains the established external
default-application behavior. The proposed `G` mode-cycling shortcut is deferred
to the post-0.2.0 shortcut pass.

Two Red Meridian files, `DSC05419…HDR.TIF` and `DSC05419…SDR.TIF`, were the only
blank cells in a 24-file folder. Direct inspection showed both are readable
8599×5733, 32-bit float TIFFs but both carry the Photoshop profile `P3D65 PQ
Display Full 12-16-0-1`; adjacent working files carry Display P3 Linear. The
original helper reproduced the failure with the explicit diagnostic that it
could not safely re-emit that profile. Apple generator version 2 recognizes the
P3/PQ source and requests a color-managed extended-linear Display P3 F16 proxy,
rather than relabeling it as ordinary P3. After `/Volumes/whisk` remounted, both
exact sources produced verified 384×256 F16 proxies tagged Display P3 Linear;
the SDR-named source completed in 5.20 seconds on Quasar. Both files carry the
same P3/PQ profile, so Disk continues to treat them as independent assets and
does not infer dynamic range from the filename.

Resize performance no longer depends on repeated FFI descriptor reads and
`NSImage(contentsOfFile:)` calls from every thumbnail body. AppModel retains one
descriptor and decoded image per completed project proxy; ImageIO eagerly
decodes the small proxy off the main actor and Swift layout reads only those
in-memory values. Gallery's thumbnail-size slider remains enabled independently
of asset/Layout selection. The Graph keeps keyboard focus for `Tab` but disables
SwiftUI's canvas-wide focus effect, whose delayed redraw looked like an obsolete
panel border during live resize. Square Grid puts filename and a lightly rounded
format label beneath the image, and failed cells expose the backend message in
their status tooltip.

Local-SSD measurement against the Desktop copies removed NAS latency. The 60.2
MP and 152.7 MP float TIFFs took 2.64 and 6.56 seconds through Photara; Quick
Look full thumbnails took 2.63 and 6.52 seconds, while its low-quality requests
failed after 2.77 and 6.69 seconds. TIFF therefore continues to bypass the
failed low-quality rung. Mapping and eagerly decoding all 40 current project
cache objects took 22.5 ms total with the first image at 5.3 ms. A generator
version change invalidates the old key once; it does not make subsequent cache
hits slower.

Gallery double-click resolves a runtime representation and opens it through
the macOS default application; it does not assign the asset or issue a graph
command. Disk rebind now publishes an explicit empty membership immediately,
then atomically reconciles only that node's prior assets when scanning finishes.
Unrelated imported/provider assets remain in project Asset Context.

The first recognizable-product refinement is implemented in this working
slice. Startup has no implicit project: it presents Create/Open/Recent, with
recent locations in native `UserDefaults`. `open_project_document` validates
portable JSON, imports it into the filesystem store, and rejects divergent
content with an existing project identity; no cloud database was added. The
visible node list is now a minimal spatial Graph. Generic bridge DTOs expose
typed ports, connections, status, versioned brand/workspace metadata, and summary fields so
Project Assets and Layout share one standard Inspector shell. Layout's existing
Stage 9 surface is an optional Workspace activated by double-click, while
Assets/Diagnostics remain project panels. A native Workspace menu restores the
default presentation only. Continue hardening this recognizable shell and real
Layout projects without building final graph editing or docking.

Stage 10's state direction is scope-based, not one database per node. One
Core-owned service will distinguish syncable `user + exact definition`
libraries/preferences, private `project + node instance` operational state,
device-only grants/credentials/paths, and portable Project Document authority.
Nodes receive only their versioned namespace and narrow host capability; they
never receive SQL, connection strings, or another namespace. Saved Layout
presets are the motivating user-scoped example.

## Verification

```console
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo metadata --no-deps --format-version 1
git diff --check
```

Preserve the working tree and do not commit, tag, or push unless the user asks.
