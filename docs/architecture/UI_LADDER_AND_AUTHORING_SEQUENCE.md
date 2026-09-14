# UI ladder and authoring sequence

Status: proposed execution contract for user review, 2026-09-14.

Photara authors a small adaptive neutral system once and applies it by semantic
ownership. Feature views do not carry independent background palettes. The native
macOS sidebar, titlebar, toolbar, controls, selection geometry, vibrancy and Liquid
Glass remain system-owned so a future SDK rebuild inherits the current macOS design.

## Three-step neutral ladder

The authored ladder has three reusable Light/Dark steps:

1. **Foundation** — application or current-scope content base.
2. **Primary** — a primary Library Browser, Project Graph or peer module surface.
3. **Inset** — a genuine nested canvas, browser, preview well or grouped work area.

The ladder resets at a new independently owned composition root instead of creating
ever more grey values. A Graph and a Node Work Surface are peer composition roots;
each may use Foundation → Primary → Inset internally. Ownership remains explicit in
titles, breadcrumbs, icons and accessibility labels—shade is supportive, never the
only scope signal.

The existing portable roles map as follows until a separately approved schema change:

| Authoring label | Existing token | Use |
| --- | --- | --- |
| Foundation | `surface.canvas` | application and scope content base |
| Primary | `surface.panel` | primary module or owned composition surface |
| Inset | `surface.elevated` | genuine nested content/editor/browser well |
| Native control | `surface.control` | exceptional custom controls only; prefer system controls |
| Image surround | `editor.surround` | fixed color-neutral photograph reference surround |

`graph.background`, `gallery.background`, and similar compatibility roles must resolve
through an explicit mapping to the ladder or a specialized reference role. They are
not separately authored copies of the same background. Node-category color remains
limited to icons, small port/status affordances and focus feedback. Photographs never
sit on a colored surface.

The Opening primary action is an ordinary native action on Foundation. It must not
paint a blue or tinted content area. System accent remains appropriate for native
selection, focus and small functional affordances.

## Lab responsibilities

- Preserve **Graph Lab** as the accepted owner of Graph canvas, nodes, ports,
  connections and floating Graph controls. It consumes mapped shared tokens but is
  not redesigned by this work.
- Rebuild **Theme Lab** as the only editor for paired Light/Dark ladder values,
  reference surround, text, borders, selection and semantic status colors. Show the
  complete three-step ladder together and validate neutrality and contrast.
- Simplify **Shell Lab** into an integrated scenario browser. It selects Opening,
  Create Project, Project Browser, Project/Graph, Inspector and Node Work Surface
  states and previews the exact production views. It authors shared geometry and
  behavior only; it does not duplicate colors or feature internals.
- Feature labs inherit the ladder read-only and expose only their typed content,
  interactions and meaningful states. Completed-context controls never leak into the
  currently selected scenario.
- Lab overrides remain developer presentation state. They never enter Core, a
  Library, Project package, Graph digest or cloud synchronization.

## Approval-sized implementation slices

### UI0 — ladder and lab foundation

Freeze the token mapping, remove chromatic casts from the default neutral surfaces,
rebuild Theme/Shell Lab responsibilities, and make Opening use Foundation without a
custom blue content tint. Render Light/Dark plus narrow/standard screenshots from the
real production views. Obtain approval, assemble into Photara and run visual,
accessibility, contrast and regression checks. Do not change Graph content.

### UI1 — Create Project

Freeze the typed create contract, then mock and approve the native UI before wiring.
The first draft selects Library, project name and package destination; shows the
default `~/Pictures/Photara/Projects` and a user-selected root such as Whisk; validates
name/path/collision/permission state; and explains that the package stores authored
state and manifests rather than the external archive.

The production gate is one staged operation that creates the Project identity,
initial Graph, Project Catalog record and `.photara` package (a folder on platforms
without package presentation), then either commits all visible coordinates or reports
a recoverable incomplete operation. Verify SQLite, Neon projection, on-disk package,
restart, cancellation, duplicate click, unavailable volume and rollback behavior.

### UI2 — Project Browser and package recovery

Mock, approve and implement typed grid/list/card browsing, search/sort, recent state,
Open Package, missing/moved package, Locate, move-through-Photara and duplicate identity
handling. Reuse browser presentation primitives without introducing an untyped Core
Gallery item. Library database import/export remains a later bounded backup slice;
opening or locating a Project package is not database import.

### UI3 — Project window and Graph integration

Open the created Project into the accepted Graph canvas. Add multiple saved Graph
selection, empty/first-node/running/success/failure states, project status and durable
save/reopen. Preserve Graph Lab behavior and apply only the shared outer hierarchy.

### UI4 — node authoring

Approve and implement the searchable categorized node catalog, ordinary built-in
Layout and source nodes, typed port connection, Inspector selection and run diagnostics.
Node category color appears only in permitted small affordances.

### UI5 — Inspector and Node Work Surfaces

Approve the Graph-owned dockable/floating Inspector and a Layout-owned Work Surface.
The Layout Work Surface composes its own Asset Browser/Gallery as a split child using
the same local ladder mapping and the fixed photograph reference surround. Test resize,
dock/float, multiple Layout nodes, selection, authoring, save/reopen and accessibility.

### UI6 — Library browsers and contextual pickers

Complete People, Location Kinds, Locations and Project Info as application-owned typed
Library browsers. Nodes reuse host-owned typed pickers inline without turning those
records into nodes or an untyped Gallery domain.

Each UI slice follows: contract → real Light/Dark raster states → user approval → same
production source in owning lab → Photara assembly → interaction/persistence/
accessibility/regression proof → focused commit. Heavy cross-boundary work uses Astra;
bounded view authoring and ordinary verification may use the default model.
