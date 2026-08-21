# Node packages

A package has a canonical namespaced ID, independent package version, manifest,
one or more versioned definitions, schemas, typed ports, capabilities,
determinism/effect policy, diagnostics, inspector contribution, state migrations,
and implementation fingerprint.

A definition is code and metadata. A node instance is user-owned graph state
that pins a definition version. Installing a newer package never silently
changes an existing instance.

Built-in and downloadable nodes use the same semantic package contract. A
built-in may be trusted and bundled by the installer, but that does not give it
special graph semantics or direct database access.

The first package is:

```text
package:    photara.layout
definition: photara.layout.compose
input:      photara.asset-set/v1
output:     photara.layout-plan/v1
```

Marketplace loading is deferred until signing, publisher identity, permission
consent, credential isolation, runtime/process isolation, dependency resolution,
revocation, update, rollback, and recovery are designed and tested.
