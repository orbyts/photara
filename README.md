# Photara

Photara is the current code name for a native creative-media workflow
application. It combines a portable Rust Core with independently versioned,
typed node packages and polished platform-native clients.

Photography is the first production use case. The architecture is deliberately
media-general so future video, VFX, ML, host-application, delivery, and other
creative workflows can be composed without adding privileged engine node kinds.

## Current status

The repository is building generation two from a clean foundation toward
`0.2.0`, the first daily-usable application:

- macOS-only product release;
- SwiftUI/AppKit client, with AppKit/Metal where native interaction, graph,
  crop, color, HDR/EDR, or performance needs it;
- portable Rust Core and heavy application logic;
- Houdini-like procedural graph semantics;
- portable human-inspectable project JSON and standalone shareable node-graph
  JSON;
- exact package registration and atomic revision-safe portable project storage;
- project-owned semantic assets with multiple fingerprinted representations,
  including paired HDR/SDR local TIFF development inputs;
- a project-scoped, deduplicated, bounded and disposable proxy service with the
  measured ImageIO/Core Image macOS backend;
- built-in Layout node with deterministic geometry, explicit AssetSet input,
  exact undo, and runtime-only access to shared project proxies;
- a production UniFFI facade verified from Swift 6 on Quasar, plus a deliberately
  small three-region macOS vertical slice with real project lifecycle, import,
  explicit asset binding, typed Layout inspection, and shared proxies;
- future Windows-native client over the same versioned Core facade;
- future node marketplace using the same contracts as built-ins.

Brand name, icons, website, marketplace presentation, and final visual language
remain intentionally undecided while the working application evolves.

## Workspace

```text
crates/
├── photara-bridge/     immutable DTO facade for native clients
├── photara-core/       semantic IDs, graph state, node definitions, diagnostics
├── photara-node-sdk/   package manifest and node registration contracts
├── photara-proxy/      shared derived proxy generation and cache service
└── photara-store/      backend-neutral authoritative persistence boundaries
platform/macos/
├── photara-app/            UniFFI verification and minimal native workspace
└── photara-proxy-imageio/  process-isolated ImageIO/Core Image helper
nodes/
├── photara-asset-set/  explicit project AssetSet source node
├── photara-disk/       authorized-folder AssetSet provider node
└── photara-layout/     built-in Layout node package
```

Stage 8 completed the minimal SwiftUI/AppKit vertical slice over these Rust
contracts. Stage 9 is now building a separate visual Layout authoring surface
over immutable resolved DTOs and shared proxies. Swift owns transient gesture
presentation only; intentional edits and undo/redo remain Core commands.
The native first-look shell launches through Create/Open/Recent and separates a
spatial Graph, generic standard Inspector, optional Layout Workspace, and
project-level Assets without introducing a database dependency.
The immutable node catalog now drives branded hierarchical creation metadata,
and the first live provider is an ordinary Disk node whose macOS folder grant
stays outside the portable project.

## Development

```console
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The workspace version is `0.2.0-alpha.0`. APIs are intentionally internal and
unstable while the first vertical application path is built.

## Documentation

- [Roadmap](ROADMAP.md)
- [Architecture](docs/architecture/README.md)
- [Portable project and node-graph documents](docs/architecture/PROJECT_DOCUMENTS.md)
- [Project assets and representations](docs/architecture/ASSETS.md)
- [Project proxy infrastructure](docs/architecture/PROXIES.md)
- [Codex handoff](docs/CODEX_HANDOFF.md)
- [Changelog](CHANGELOG.md)
