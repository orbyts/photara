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
- built-in Layout node shipped as an ordinary independently namespaced package;
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
└── photara-proxy-imageio/  process-isolated ImageIO/Core Image helper
nodes/
└── photara-layout/     built-in Layout node package
```

The SwiftUI/AppKit application is added after the Rust application facade and
Layout semantics are ready enough to keep Swift focused on presentation and
interaction.

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
