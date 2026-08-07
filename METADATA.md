# Photara metadata contract

Photara owns photography workflow decisions. The Lightroom Classic plugin is
a thin adapter: it gathers the operator's selections, asks the Rust core for a
reconciliation plan, applies that plan through Lightroom's SDK, and reports the
result. Lua does not duplicate Photara's business rules.

The current JSON contract can be inspected with
`photara metadata plan <project-slug>`. Generating a plan does not modify a
Lightroom catalog.

## Whole-shoot workflow

1. The operator selects the entire shoot in Lightroom Classic.
2. The plugin obtains people, locations, scenes, and projects from Photara's
   machine-readable CLI.
3. The operator chooses the project, participating people, location, and scene.
4. Photara validates those references and returns an inspectable metadata and
   collection reconciliation plan.
5. The plugin applies the plan idempotently and writes Lightroom metadata to
   the RAW files' XMP sidecars.
6. Photara records the outcome through its application-owned repositories.

## Photara-managed metadata

For the selected shoot, Photara may reconcile these IPTC fields:

- Job Identifier: project display name or configured job identifier.
- Scene: configured scene display name.
- Sublocation, City, State/Province, Country, and ISO Country Code: configured
  location fields.
- Creator and Copyright: configured creator identity and copyright value.

People are represented by hierarchical keywords in the form
`people|<role>|<display name>`. Workflow state uses the configured
`workflow|...` hierarchy. Photara adds or removes only values in hierarchies it
owns.

## User-owned metadata

Photara does not overwrite titles, captions, ratings, pick/reject flags, color
labels, unrelated keywords, or user-created collections. Existing metadata is
preserved unless a field is explicitly part of the managed contract above.

## Collection reconciliation

The plugin reconciles four top-level collection trees: People, Locations,
Scenes, and Projects. Project-specific collections and smart collections are
derived from the Rust plan. Repeating the same operation must converge on the
same structure without duplicates and without disturbing unrelated
collections.

## Representation rules

Camera-original filenames remain unchanged while RAWs are managed by the
Lightroom Classic catalog. See `ROADMAP.md` for the single-authoritative-home
rules for RAW/XMP, DNG, PSB, TIFF, delivery renditions, and temporary proofs.
Detailed edit-version lineage is intentionally deferred until after 0.1.0.
