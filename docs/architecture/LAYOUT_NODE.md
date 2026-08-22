# Built-in Layout node

Layout is the first production node and the first future rich Inspector. It is
an ordinary independently versioned package installed by default, not a Core
node category.

## Semantic boundary

The `photara.layout.compose` definition has one required
`photara.asset-set` input and one `photara.layout-plan` output. The ordered
`AssetSet` is the complete asset input: Layout never reads Gallery selection,
ambient project UI context, storage-provider state, or filesystem paths.

The semantic runtime performs no I/O. It decodes and validates authored state,
validates every placed `AssetId` against the explicit input, resolves exact
geometry, and emits a deterministic plan. Its output is therefore identical
whether proxy storage is warm, empty, unavailable, or corrupt.

```text
authored LayoutState + explicit AssetSet
                  │
                  ▼
       deterministic LayoutPlan

LayoutPlan + project proxy profile
                  │ runtime preview only
                  ▼
       ProjectVisualProxyService
                  │
                  ▼
          ephemeral LayoutProxySet
```

The lower path is deliberately separate. `LayoutProxySet` is not serializable.
Proxy cache keys, descriptors, local paths, leases, and availability are absent
from both `LayoutState` and `LayoutPlan`. A saved project therefore opens and
retains its Layout when every derived proxy file has been deleted.

## Authored state

The version-one hierarchy is:

```text
LayoutState
├── canvas
│   ├── bundled 3:4 or 9:16 profile + profile version + long edge
│   ├── custom positive pixel dimensions
│   └── custom positive aspect + long edge
└── arbitrary positive ordered frames
    ├── decoration: normalized insets, gap, corner radius, background
    ├── arrangement: one, horizontal stack, vertical stack, grid, custom
    └── one or more ordered cells
        ├── optional explicit AssetId (repetition is valid)
        ├── Fit + alignment
        ├── Fill + focal point
        ├── Crop + authored normalized source rectangle
        ├── quarter-turn rotation
        └── custom normalized destination rectangle when applicable
```

Normalized values use unsigned fixed point with one-millionth precision. This
avoids host floating-point differences in saved state, pixel resolution, and
digests. Bundled canvas behavior carries an explicit profile version; later
profile changes require a new version rather than silently changing old
projects.

Frames, arrangements, decorations, and cells are distinct semantic objects.
This keeps later justified, masonry, mosaic, treemap, constraint, or aesthetic
strategies additive behind the same plan instead of coupling state to one UI
template implementation.

## Resolution and editing

Resolution validates unique frame/cell identities, positive frame and cell
counts, normalized bounds, arrangement invariants, and explicit asset
membership. It produces stable ordered frame/cell indices plus normalized and
pixel rectangles. The authored Fit/Fill/Crop and rotation policy is retained in
each resolved cell for downstream render or preview consumers.

Semantic commands cover canvas replacement; frame insert, remove, move, and
replacement; arrangement changes; cell insert, remove, and replacement; and
decoration replacement. Every successful command validates the complete result
and returns an exact inverse. Invalid commands leave their input untouched.
Canonical state and plan digests include every output-affecting semantic field
and exclude proxy/runtime state.

## Project proxy access

Preview code requests proxies through `ProjectVisualProxyService`. Its first
binding combines the project's asset context, representation materializer, and
shared `ProjectProxyService`; Layout receives no provider-specific or backend
object. Representation selection is based on HDR/SDR capabilities required by
the profile, not TIFF or any upstream application identity. The existing paired
local TIFF adapter remains only a development source standing in for future
Photoshop, Lightroom, Lureva, cloud, or other upstream nodes.

One preview request is made per distinct placed asset. Independent Layout
instances requesting the same source fingerprint and exact profile naturally
reuse the same project cache object. Different crops remain Layout semantics
and do not duplicate the source proxy.

## Stage 7 gate evidence

The integration gate persists independent 3:4 and 9:16 Layout nodes that place
the same asset with different crops. Both resolve from explicit `AssetSet`
input, request the same SDR thumbnail through the project interface, and
observe one generation followed by a shared cache hit. It then releases the
runtime leases, clears the complete proxy cache, reopens the project from the
filesystem store, and verifies exact project and authored-state equality.

The native Properties/Inspector arrives later. Docking, rearranging, enlarging,
or detaching that surface must never alter this contract.
