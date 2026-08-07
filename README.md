# Photara

Photara is an experimental photography workflow and publishing tool.

The project will provide tooling for managing photographic projects from
selection and editing through derivative generation, media storage, and
publication bookkeeping.

## Status

`v0.0.2` establishes Photara-owned configuration, migrations, and project
initialization on top of Storexa 0.1. Photara owns its schemas, SQL,
repositories, and photography workflow; Storexa owns connection and
transaction plumbing.

See [ROADMAP.md](ROADMAP.md) for the path to the first supported release.

## Configuration

Non-secret configuration lives under `$XDG_CONFIG_HOME/photara` (or the
explicit `PHOTARA_CONFIG_ROOT` override):

```text
photara/
├── config/
│   ├── photara.toml
│   ├── people.yml
│   ├── locations.yml
│   └── scenes.yml
├── cache/
├── schemas/
└── templates/
```

Initialize without overwriting any existing files, then validate after adding
registry entries:

```console
$ photara config init
$ photara config validate
```

## Development database

Photara reads its Neon development connection from
`PHOTARA_DEV_DATABASE_URL`. The variable may be supplied by Apogee, another
secret manager, a shell, or any process supervisor; Photara does not depend on
how it is loaded.

Use the direct Neon URL because Storexa already manages a SQLx connection
pool. Verify the configured database without changing its schema:

```console
$ photara health
```

Apply Photara-owned migrations:

```console
$ photara migrate
```

Initialize a project after its scene, location, and people exist in the
registries:

```console
$ photara project init red-meridian \
    --display-name "Red Meridian" \
    --scene architectural-portrait \
    --location golden-gate-bridge \
    --person valentina-reneff-olson
```

The operation is idempotent. Repeating the same command verifies the existing
database record and `project.json`; supplying conflicting values fails rather
than silently changing project identity.

## Representation ownership

Camera RAW names are immutable. RAWs and XMP sidecars live only in the dated
image archive; working DNGs live in Lightroom Cloud; layered PSBs return beside
their original RAWs; and flattened TIFF masters live in their project folder.
Photara records relationships and never creates permanent convenience copies.

The connection URL must remain outside the repository. Non-secret application
settings will belong under `$XDG_CONFIG_HOME/photara/` as they are introduced.

## License

MIT
