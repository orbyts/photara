# Photara macOS application

This directory is the native-client root. The production-shaped Rust
facade is generated into Swift with workspace-pinned UniFFI and verified before
the SwiftUI/AppKit workspace grows around it.

On Quasar:

```console
platform/macos/photara-app/verify-bridge.sh
```

The verification compiles with the active Xcode/Swift toolchain and exercises
portable project create/save/reopen, paired TIFF import, explicit `AssetSet`
and Layout binding, typed Layout inspection without JSON decoding, shared
SDR/HDR proxy references, revision-checked semantic commands, structured
diagnostics, progress callbacks, and Swift-triggered cancellation. It also
proves that Gallery selection/filter/visibility and Inspector placement cannot
change the graph digest. Generated bindings and binaries stay under `.build`.

The bridge imports no SwiftUI or AppKit API and has no macOS 27 dependency.
The workspace UI consumes the same facade; it does not create a parallel
semantic state path.

Compile the first native shell with:

```console
platform/macos/photara-app/build-app.sh
```

The build produces a self-contained, ad-hoc-signed application bundle at
`platform/macos/photara-app/.build/app/Photara.app`. Launch that bundle through
Finder or with:

```console
open platform/macos/photara-app/.build/app/Photara.app
```

Launching the bundle, rather than its inner Mach-O executable, gives Photara
its own macOS application identity, activation behavior, and menu bar.

The shell stays deliberately small but now has recognizable product structure.
With no active project it presents Create/Open/Recent; recent entries remain
native client state and portable documents open through the filesystem facade,
without a database. Assets, Graph, Layout Workspace, Inspector, and Diagnostics
are stable pane identities mapped into three resizable regions. They can move,
hide, and restore independently of project state.

Graph is a primitive spatial canvas with node cards, typed ports, connections,
selection, pan, and zoom. Selection drives one generic Inspector shell from
typed bridge summaries. `Tab` or the graph add control opens the node menu;
Layout is its sole current entry, without making the graph shell Layout-only.
Double-click activates Layout's optional Workspace.
The standard Inspector remains available for every node definition. Versioned
definitions may later augment it with custom semantic controls independently of
whether they advertise a full visual Workspace; Layout is simply the first node
that needs both.
The app still populates Assets from Project Asset Context, binds only through
explicit commands, and composes typed resolved Layout cells from shared proxy
references. Inspector controls issue semantic Core commands; crop dragging is
transient in Swift and commits once at gesture completion.

Generated bindings, Rust targets, module caches, and executable artifacts stay
under `.build`. No Xcode 27 or macOS 27 API is used.
