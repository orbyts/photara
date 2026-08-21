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
```

The workspace is `0.2.0-alpha.0`. The initial scaffold contains canonical IDs,
node/port definitions, graph documents/revisions, diagnostics, an optimistic
in-memory repository, a versioned client DTO, and a Layout package registered
through the ordinary node SDK.

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

Follow Roadmap section 1:

1. review the initial ID syntax and decide canonical version/type identity;
2. add connection and value registry contracts;
3. define canonical serialization and digests;
4. add tests with Layout plus at least one materially different synthetic node;
5. keep the public surface internal and unstable.

After Roadmap section 2 supplies one real command and evaluation path, perform
the Roadmap section 3 Swift bridge spike. It must cover one immutable DTO, one
structured error, progress, and cancellation. It is deliberately a disposable
interoperability harness, not a production UI foundation.

Do not start the production Swift app, database provider, legacy importer, or
provider nodes before these contracts pass their generality gate.

## Verification

```console
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo metadata --no-deps --format-version 1
git diff --check
```

Preserve the working tree and do not commit, tag, or push unless the user asks.
