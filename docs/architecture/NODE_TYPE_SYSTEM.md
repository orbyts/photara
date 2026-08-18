# Minimal node value and type system

## Design constraints

The first type system must support Disk → Layout → Photoshop without inventing
types for every future provider. Values should be semantic, versioned, serializable,
content-fingerprintable, and transportable by reference when large. A value is
not necessarily a file and a port is not merely a path.

## Minimal value set

| Value | Purpose | Authoritative? |
| --- | --- | --- |
| `ProjectRef/v1` | Stable project identity and snapshot revision | Reference to authoritative state |
| `AssetSet/v1` | Ordered current assets with representation-set references | Derived query value |
| `LayoutPlan/v1` | Validated editorial layout intent from one Layout instance | Authored semantic output; graph document is authoritative |
| `ResolvedLayoutPlan/v1` | Pixel geometry and execution-ready bindings | Derived/cacheable |
| `ArtifactSet/v1` | Fingerprinted materialized files associated with semantic outputs | Derived but durably tracked |
| `HostStatus/v1` | Observed host/plugin/device capability snapshot | Ephemeral/provider-state input |
| `HostExecutionReceipt/v1` | What host ran, against which request, and what it produced | Durable evidence |
| `VisualProxyRef/v1` | Reference to disposable thumbnail/preview artifact | Derived/cacheable |

`LayoutPlan` may expose its resolved form through one port initially to keep the
graph small, while Core internally distinguishes authored intent from derived
resolution. The distinction must remain explicit in storage and cache keys.

## Asset and representation references

Illustrative serialization:

```json
{
  "type": "photara.asset-set/v1",
  "project_id": "uuid",
  "query": { "kind": "current-flattened-pairs" },
  "assets": [
    {
      "asset_id": "uuid",
      "display_name": "_SUH5118.ARW",
      "renditions": {
        "pair_id": "derived-stable-id",
        "hdr": {
          "file_id": "uuid",
          "role": "hdr",
          "representation": "flattened-tiff",
          "logical_location": "projects:sylvan/masters/flattened/..._HDR.TIF",
          "sha256": "...",
          "byte_size": 123,
          "width": 6000,
          "height": 8000,
          "color_profile": "Display P3 Linear"
        },
        "sdr": {
          "file_id": "uuid",
          "role": "sdr",
          "representation": "flattened-tiff",
          "logical_location": "projects:sylvan/masters/flattened/..._SDR.TIF",
          "sha256": "...",
          "byte_size": 123,
          "width": 6000,
          "height": 8000,
          "color_profile": "Display P3 Linear"
        }
      },
      "proxies": {
        "thumbnail": { "state": "available", "ref": "proxy:sha256:..." },
        "authoring": { "state": "requestable" }
      }
    }
  ]
}
```

The path resolver belongs to the local infrastructure adapter. Remote providers
can emit the same representation semantics with provider locators and their
own proxy capability.

## Layout authored state and output

Illustrative `LayoutPlan/v1`:

```json
{
  "type": "photara.layout-plan/v1",
  "node_instance_id": "uuid-layout-34",
  "profile": {
    "canvas_profile": "photara.canvas.3x4-portrait@1",
    "width": 4500,
    "height": 6000,
    "snapshot_sha256": "..."
  },
  "items": [
    {
      "id": "hero",
      "template": "full-frame@1",
      "template_sha256": "...",
      "placements": [
        {
          "slot": "image",
          "asset_id": "uuid",
          "rendition_pair_id": "derived-stable-id",
          "fit": "crop",
          "focal_point": { "x": 0.5, "y": 0.5 },
          "transform": {
            "crop": { "x": 0.1, "y": 0.0, "width": 0.8, "height": 1.0 },
            "rotation_quarter_turns_cw": 0
          }
        }
      ]
    }
  ],
  "authored_state_sha256": "..."
}
```

The second Layout instance can reference the same asset and pair while owning
a different crop. No transform is stored on `AssetRef`.

## Port rules

- Port types use stable semantic IDs and major versions.
- Connections require exact type compatibility or an explicit registered
  converter; never implicit JSON coercion.
- Input cardinality is `One`, `Optional`, `Many`, or `OrderedMany`.
- Ordering is part of the value digest where editorial order matters.
- A port can declare required capabilities, such as paired HDR/SDR renditions.
- A missing file is not a type mismatch; it is a validation diagnostic on an
  otherwise valid semantic reference.

Example:

```rust
InputPortDefinition {
    id: "assets",
    value_type: "photara.asset-set/v1",
    cardinality: One,
    required_capabilities: ["paired-hdr-sdr", "visual-proxy"],
}
```

## Versioning

Use separate versions for:

1. serialized value schema;
2. node definition behavior;
3. authored-state schema;
4. execution protocol/host plugin;
5. layout template and canvas profile.

A node type upgrade does not silently rewrite an instance. An explicit
migration function returns a new versioned document and migration report.
Unknown fields must not be discarded during a round trip. Published/frozen
values remain readable through old adapters even after new defaults exist.

## Validation phases

1. **Schema** — typed fields and supported versions.
2. **Connection** — port type, cardinality, capability requirements.
3. **Semantic** — unique item IDs, valid template slots, normalized crop,
   compatible dimensions/roles.
4. **Availability** — current file/provider/host can be resolved.
5. **Execution readiness** — all fingerprints current, host protocol supported,
   destination safe.

This preserves the v0.1 distinction between a structurally valid post and a
post ready to render.

## Human-authored state

Human state is not a cache entry. It belongs to the graph document with:

- schema and node definition version;
- optimistic revision or content digest;
- undoable commands/events;
- stable item/placement IDs;
- explicit migration history;
- last successful output digest as evidence, not authority.

The GUI edits authored state through Core commands such as add item, assign
asset, set fit, set normalized transform, and reorder. It must not mutate an
untyped JSON blob directly. CLI and GUI use the same commands.

## Errors and warnings

Suggested diagnostic shape:

```json
{
  "code": "layout.source.changed",
  "severity": "error",
  "message": "The HDR rendition changed after this layout was resolved.",
  "node_instance_id": "uuid",
  "port": "assets",
  "context": { "asset_id": "uuid", "item_id": "hero", "slot": "image" },
  "recovery": { "action": "resolve-again", "safe": true }
}
```

Stable codes matter for UI actions, CLI automation, tests, and future SDK
compatibility. Rust error chains remain internal details.
