# Shared People module

Photara and [the People Lab](../photara-people-lab/README.md) compile these exact
sources. The module receives immutable presentation values and semantic action
callbacks. SQL, file preparation, project commits and account authority live in
the production adapters and Rust facade.

The reusable record browser, thumbnail treatment and editor chrome live in
`photara-library-ui`. People, Locations and Scenes own their editor fields;
Project Info composes those same fields when creating a missing Library record.
Project Info owns project assignments and unique scene occurrences, with stored
name/revision snapshots and project-only date/notes. It never copies a Library
database into a project. Clients are an individual/organization category in People.

Change shared sources here to author People, then rebuild the lab and Photara.
Shell owns the module header/icon, rounded boundary, placement and disclosure;
feature content scrolls independently inside it. See [shared UI](../SHARED_UI.md)
and [Library architecture](../../../docs/LIBRARY_ARCHITECTURE.md).
