# Photara

Photara is an experimental photography workflow and publishing tool.

The project will provide tooling for managing photographic projects from
selection and editing through derivative generation, media storage, and
publication bookkeeping.

## Status

`v0.0.1` begins application development and consumes Storexa 0.1 for
PostgreSQL persistence infrastructure. Photara owns its schemas, migrations,
SQL, and repositories; Storexa owns connection and transaction plumbing.

```console
$ photara
Hello from Photara.
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

The connection URL must remain outside the repository. Non-secret application
settings will belong under `$XDG_CONFIG_HOME/photara/` as they are introduced.

## License

MIT
