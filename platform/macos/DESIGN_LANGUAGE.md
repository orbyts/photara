# Photara native design language

Photara follows the current Apple platform hierarchy while keeping feature modules
independently authorable. The goal is a small semantic contract that naturally adopts
future system appearances when rebuilt with a newer SDK.

The canonical product vocabulary and ownership come from
[D19](../../docs/architecture/LIBRARY_AND_NODE_WORK_SURFACES.md). Library is the
durable scope; Project belongs to one Library; Graph owns Nodes, its Canvas and
the Inspector surface; every Node supplies a required Inspector contract and may
supply an optional Work Surface. Current pre-D19 UI/source labels remain migration
facts only. Future UI copy uses Library, Project, Graph, Inspector, Work Surface,
Canvas, Panel, Browser and Window Layout consistently.

## The hierarchy

1. **Application foundation** (`surface.canvas`) is the adaptive neutral window
   background. It is standard content, never Liquid Glass.
2. **Scope chrome** identifies the current Library and, when open, Project. It
   remains visually quiet but unambiguous in every window or detached surface.
3. **Primary surface** (`surface.panel`) hosts a Library Browser or the Project's
   Graph. The Graph surface owns its Canvas, Inspector Panel and active optional
   Node Work Surface; Gallery is not an ambient Project surface.
4. **Panel / Work Surface level** uses a shared neutral surface role for Graph-owned
   Inspector and Node-provided authoring composition. Nested components do not add
   arbitrary frames merely to signal ownership.
5. **Content level** (`surface.elevated`) is optional. Use it only for a genuine
   inset canvas, asset Browser, preview, editor well or grouped control region—not
   as a default wrapper around every section.
6. **Functional control layer** is the only Liquid Glass layer. It floats above
   content and contains important navigation or controls.

The hierarchy is communicated through a small shared ladder of adaptive neutral
surface roles, spacing, corner radius and elevation in both Light and Dark—not by
feature-local colors or permanent separator lines. Scope text/identity remains the
accessible source of truth; shade alone never conveys Library, Project or selection.
Exact new token names and contrast values are frozen in the Theme/Shell contract
before visual implementation.

Color changes are authored once in Theme Lab. Shell Lab owns shared geometry: outer
inset, gutter, module content inset, module corner radius, header metrics, pane sizes
and status geometry. A feature lab must not redefine these values.

## Liquid Glass rule

Ask one question: **Is this an important navigation/control surface floating above
content?** If yes, use a native system component or the shared glass convention. If
it is content, a content container, a section, a selection fill or decoration, do
not use Liquid Glass.

Use Liquid Glass for the native window toolbar, Graph's floating tool rail and zoom
control, transient media controls, and a compact module-action group only when it
actually overlays scrolling content. Let macOS select blur, shadow, adaptivity,
contrast, grouping and accessibility fallback. Do not expose product blur-radius,
glass-shadow or toolbar-pill controls.

Do not use Liquid Glass for module backgrounds, the Graph canvas, Inspector sections,
Gallery cells, asset thumbnails, nodes, status bars, empty states or selection.
Custom glass controls must respect Reduce Transparency and must share one container
when adjacent. Graph's existing accepted tool rail and zoom treatment already obey
this rule and remain Graph-owned.

References:

- [Apple HIG: Materials](https://developer.apple.com/design/human-interface-guidelines/materials)
- [Apple HIG: Toolbars](https://developer.apple.com/design/human-interface-guidelines/toolbars)
- [Adopting Liquid Glass](https://developer.apple.com/documentation/TechnologyOverviews/adopting-liquid-glass)
- [Build an AppKit app with the new design](https://developer.apple.com/videos/play/wwdc2025/310/)

## Window toolbar

Photara supplies semantic toolbar items and macOS renders their Liquid Glass:

- Leading: current Library identity and, in a Project window, project thumbnail/name
  as a clear `Library / Project` scope.
- Center: application identity.
- Trailing group 1: authorized Library modules (People, Organizations, Locations,
  Location Kinds and Projects) plus Account access.
- Trailing group 2: current Graph and optional Node Work Surface actions.
- Trailing group 3: project actions (Add Node, Run/Cancel, Save, Window Layout).

Unavailable capabilities do not leave dead controls. Use system symbols, native help,
menu-bar command equivalents and system overflow. Do not add custom toolbar material,
blur, capsule, border or shadow. Settings remains a standard application command.

## Module chrome

The Shell is the only owner of module framing. Every hosted feature receives:

- one `surface.panel` background;
- one shared corner radius and inset system;
- a flat title area integrated into the module base;
- a subtle `selection.background` title tint when focused;
- no resting perimeter stroke or header separator;
- no permanent split line; the resize affordance appears only during dragging;
- an optional restrained elevation authored once by Shell Lab.

Feature modules supply content and semantic actions only. They must not add another
outer background, border, title bar, close button, placement button or module shadow.
Feature-internal sections use spacing and headings first. An inset surface is reserved
for a distinct work area, not ordinary metadata such as Inspector definition fields.

## Scrolling and controls

Scrollable content extends through its available content region. When fixed custom
controls overlap scrolling content, use the platform's automatic or soft scroll-edge
effect to preserve legibility. Scrolling alone does not justify glass. Ordinary fields,
pickers, sliders and buttons remain native controls and inherit the SDK's current
interaction appearance.

Module-specific commands belong near their module. Graph may own Add Node, Center,
Zoom, Validate or Export commands. Project Save remains in the application toolbar
while the graph is part of the project document; use “Save Graph” only if graphs later
become independently persisted artifacts.

## Lab ownership

- **Theme Lab:** the shared adaptive palette and contrast.
- **Shell Lab:** launcher, application/module hierarchy, shared geometry, toolbar
  identity and integrated module scenarios.
- **Graph Lab:** Graph content, nodes, connections, canvas and its accepted floating
  controls. It does not author the surrounding module.
- **Gallery Lab:** the shared asset Browser/Gallery component used by Layout,
  Gallery and metadata Work Surfaces: cards, HDR presentation, filtering, selection
  and empty states. It does not imply a project-wide Gallery or author module chrome.
- **Inspector Lab:** the Graph-owned Inspector renderer, empty/selection states,
  common hierarchy and Node-supplied typed sections. It does not author Graph chrome.
- **People, Locations, Location Kinds and Project Info Labs:** their browsers, editors,
  records and empty/loading/error states. They do not author module chrome or glass.
- **Photara:** production adapters, persistence and assembly; no copied lab views.

Standalone feature labs may use a plain neutral host around their content for testing.
Only Shell Lab and Photara show the authoritative integrated frame.

## Review checklist

- Is there exactly one module frame?
- Is every color a semantic Theme role rather than a feature-local shell color?
- Does glass contain only important floating controls or navigation?
- Does the system own toolbar grouping and optical behavior?
- Are selection and resize feedback subtle and transient?
- Does Light/Dark, Reduce Transparency, Increase Contrast and narrow-window behavior
  remain legible?
- Did the change stay inside the owning lab/module boundary?
