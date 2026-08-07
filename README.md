# Photara

Photara is an experimental photography workflow and publishing tool.

The project will provide tooling for managing photographic projects from
selection and editing through derivative generation, media storage, and
publication bookkeeping.

## Status

`v0.0.3` adds application-managed people, location, and scene registries plus
transactional project reconfiguration on top of Storexa 0.1. Photara owns its
schemas, SQL, repositories, and photography workflow; Storexa owns connection
and transaction plumbing.

See [ROADMAP.md](ROADMAP.md) for the path to the first supported release and
[METADATA.md](METADATA.md) for the Lightroom metadata ownership contract.

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

Manage registry entries through Photara so the same application services can
later back the Lightroom plugin or a GUI:

```console
$ photara people add trinity-woodward \
    --display-name "Trinity Woodward" \
    --alias Trin --alias Trinity \
    --role model \
    --social instagram=@theetr1n1ty \
    --social threads=@theetr1n1ty
$ photara people list --json
$ photara people show trinity-woodward
```

Locations and scenes follow the same `add`, `list`, and `show` pattern. Pass
`--replace` to intentionally update an existing registry entry; omission is a
guard against accidental replacement.

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
    --person trinity-woodward
```

The operation is idempotent. Repeating the same command verifies the existing
database record and `project.json`; supplying conflicting values fails rather
than silently changing project identity.

Correct an existing project's associations without changing its durable ID:

```console
$ photara project configure red-meridian \
    --display-name "Red Meridian" \
    --scene architectural-portrait \
    --location golden-gate-bridge \
    --person trinity-woodward
```

## Representation ownership

Camera RAW names are immutable. RAWs and XMP sidecars live only in the dated
image archive; working DNGs live in Lightroom Cloud; layered PSBs return beside
their original RAWs; and flattened TIFF masters live in their project folder.
Photara records relationships and never creates permanent convenience copies.

The connection URL must remain outside the repository. Non-secret application
settings will belong under `$XDG_CONFIG_HOME/photara/` as they are introduced.

## License

MIT
