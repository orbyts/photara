# Photara macOS application

This directory is the Stage 8 native-client root. The production-shaped Rust
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

The shell is deliberately small: Assets, Graph, Inspector, and Diagnostics are
stable panel identities mapped into three resizable regions. They can move,
hide, and restore independently of project state. Graph is a selectable node
list. The app performs minimum project lifecycle and paired TIFF import,
populates Gallery from Project Asset Context, binds selection only through an
explicit command, and shows typed Layout inspection plus a shared proxy-backed
preview. Stage 9 owns richer authoring controls.

Generated bindings, Rust targets, module caches, and executable artifacts stay
under `.build`. No Xcode 27 or macOS 27 API is used.
