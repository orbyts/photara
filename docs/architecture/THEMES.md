# Native presentation themes

Photara themes are portable presentation documents, not Core or project
semantics. A theme contains the same fixed semantic slots for paired Light and
Dark appearances. The native client's `System` appearance chooses between
those modes; choosing, editing, or reloading a theme cannot dirty a project or
change a graph digest.

The canonical interchange space is standard sRGB using `#RRGGBB` or
`#RRGGBBAA`. This keeps theme files deterministic and straightforward for the
macOS client, a future Windows client, and external authoring tools. Theme
authoring may expose perceptual spaces such as OKLCH, and a future schema may
add an optional Display P3 override with an sRGB fallback, but neither changes
the version-one interchange contract. Application chrome remains SDR; HDR
belongs to visual asset/proxy presentation.

Semantic roles describe use rather than literal product categories. Surfaces,
text hierarchy, borders, focus/selection, representative product surfaces,
status text, and a small set of node color roles are resolved through the same
document. Node catalog taxonomy and node color role are independent: a Disk
definition may appear under `Input / Filesystem` while requesting the same
`node.native` color role as Layout. Exact node definitions advertise a neutral
theme role and retain a literal sRGB accent only as a compatibility fallback.

Theme version one requires paired values for all registered roles and permits
additional paired roles for compatible evolution. Missing required roles,
invalid sRGB values, mismatched Light/Dark keys, and unsupported schema or color
space identifiers fail validation. Contrast checks initially report authoring
warnings rather than silently altering chosen colors.

The macOS `PhotaraTheme.swift` file is the single parser and resolver compiled
into both Photara and the developer Theme Lab. The Lab also compiles the actual
production `WorkspaceView`, Layout Inspector, Graph, Gallery, controls, and
generated bridge facade against an isolated fixture project. It does not keep
hand-authored approximations of those surfaces, so production UI changes appear
in the next Lab build automatically. The Lab imports and exports the portable
JSON. A development override is native client preference state. Photara polls
that explicitly selected file and retains the last valid palette during an
incomplete or invalid authoring write.

The Lab is a bounded developer utility, not another product subsystem. It does
not own a second schema, enter the production facade, create Core commands, or
persist anything in a project.

The Lab may disclose presentation-only authoring help for each stable role:
its intended consumers and whether production coverage is live, partial, or
reserved. This help is not part of the portable JSON schema. A reserved role
allows a future surface or node category to adopt the shared vocabulary without
claiming that changing the slot already affects current UI.
