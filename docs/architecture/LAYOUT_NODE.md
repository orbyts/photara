# Built-in Layout node

Layout is the first production node and the first rich inspector. It is an
ordinary package installed by default, not a Core node category.

Its authored hierarchy is:

```text
Layout instance
└── output canvas profile or custom aspect/dimensions
    └── arbitrary positive ordered frames
        └── per-frame cells, arrangement, and decoration
            └── explicit asset placement
                ├── Fit
                ├── Fill
                └── Crop with authored transform
```

Frames and cells are separate. A frame may contain one cell, a stack, a uniform
grid, or custom normalized cells. Destination limits do not constrain the
Layout node's frame count.

Fit contains the source automatically. Fill covers the cell automatically using
alignment/focal policy. Crop requires an authored normalized transform when
unresolved. Quarter-turn rotation is applied before crop resolution. Repeated
assets and independent transforms across Layout instances are valid.

Every strategy emits the same versioned semantic Layout plan. Later uniform,
justified, masonry, packed/mosaic, treemap, constraint, and aesthetic algorithms
are additive definition versions or strategies whose parameters and tie-breaking
participate in fingerprints.

The Properties/Inspector is the first major native UI investment. It edits this
state exclusively through Core commands and may be docked, rearranged, enlarged
into a focused authoring surface, or eventually detached without changing the
Layout contract. Its production slice covers frame ordering, canvas and
template choice, proxy assignment, Fit/Fill/Crop, rotation, visual crop
authoring, validation, resolved preview, undo, multiple instances, and
save/reopen before graph rendering or docking receives comparable polish.
