# Node packages

A package has a canonical namespaced ID, independent package version, manifest,
one or more versioned definitions, schemas, typed ports, capabilities,
determinism/effect policy, diagnostics, inspector contribution, state migrations,
and implementation fingerprint.

Package release version, definition version, value-type version, and persisted
schema/state version are distinct coordinates. A persisted node instance pins
the exact package release, definition identity, and definition version it uses;
updating a registry entry never silently retargets that instance.

Portable projects and standalone graph exports list exact package release
requirements while each node instance retains its exact definition identity and
version. Opening or resaving a document does not require those packages to be
installed; opaque schema-tagged node state remains preserved for later
resolution.

A definition is code and metadata. A node instance is user-owned graph state
that pins a definition version. Installing a newer package never silently
changes an existing instance.

The Stage 4A `NodePackageRegistry` accepts the same validated manifest returned
by a live bundled package or reopened from persistence. It builds exact Core
definition coordinates from the package release plus each definition identity
and version. Resolution never falls forward to another release or definition
version. Manifests require schema version 1, a nonempty display name, at least
one valid definition, unique definition coordinates, and definitions inside the
package namespace. Unknown top-level manifest fields survive JSON round trips.

Built-in and downloadable nodes use the same semantic package contract. A
built-in may be trusted and bundled by the installer, but that does not give it
special graph semantics or direct database access.

```text
Core package/definition registry
├── photara.layout                 bundled by default
├── future first-party packages   bundled or installed later
└── future community/private packages
```

Bundled, official, community, local-development, and private/studio delivery
are distribution policies over the same identities and registry contract, not
different Core node types. The `0.2.0` critical path implements bundled
registration and sensible missing-package behavior, not the remote store or
complete install/update/uninstall lifecycle.

Persisted manifest records describe registrations only. They do not claim that
package code was downloaded, trusted, enabled, migrated, or executable; those
are later lifecycle concerns.

The first package is:

```text
package:    photara.layout
definition: photara.layout.compose
input:      photara.asset-set at value-type version 1
output:     photara.layout-plan at value-type version 1
```

Marketplace loading is deferred until signing, publisher identity, permission
consent, credential isolation, runtime/process isolation, dependency resolution,
revocation, update, rollback, and recovery are designed and tested.
