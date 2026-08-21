# Generation-two architecture

## Reading order

1. [Core](CORE.md)
2. [Node packages](NODE_PACKAGES.md)
3. [Persistence](PERSISTENCE.md)
4. [Layout node](LAYOUT_NODE.md)
5. [Native clients](NATIVE_CLIENTS.md)

`ROADMAP.md` is authoritative for implementation order and release gates.

## System shape

```mermaid
flowchart TB
    Mac["macOS client\nSwiftUI · AppKit · Metal"]
    Win["future Windows-native client"]
    Test["Rust CLI/test harness"]
    Facade["versioned application facade\nDTOs · commands · progress · cancellation"]
    Core["portable Rust Core\ngraph · values · evaluation · evidence"]
    Runtime["node package host\ncapabilities · state · execution"]
    Layout["photara.layout\ninstalled by default"]
    Other["future independently installed nodes"]
    Store["Core state service"]
    Mac --> Facade
    Win --> Facade
    Test --> Facade
    Facade --> Core
    Core --> Runtime
    Runtime --> Layout
    Runtime --> Other
    Core --> Store
```

Platform clients own native presentation. Core owns semantics. Nodes own
declared behavior and namespaced state. No layer reaches around these contracts.
