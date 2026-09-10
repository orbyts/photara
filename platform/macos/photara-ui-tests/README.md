# Shared UI verification

Run `verify-shared-ui.sh` for isolated Gallery/Inspector fixtures and native HDR
policy checks; run `verify-production-ui.sh` for the real production composition
and adapter/Core integration. The latter builds Photara first. Both accept
`--build-only` to compile before an exclusive native-window test session.

These are test executables, not additional design labs. They write native PNGs
and temporary test projects under `.build`, use isolated UserDefaults suites,
and do not modify user projects. Run them separately from Graph's physical
pointer verification to avoid stealing its input focus.

The source compilation boundaries are also checked by building Gallery Lab and
Inspector Lab independently: neither includes generated bridge code, production
models, Rust libraries, or the other feature's views.
