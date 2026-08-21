# Proxy generation concurrency measurement

This harness measures aggregate resident memory for deliberately distinct
Apple proxy jobs. It complements the service tests: those prove identical
requests deduplicate before scheduling, while this harness measures the cost
when requests genuinely require separate generations.

Build the production helper, retain the Stage 6A TIFF corpus under
`/private/tmp`, then run:

```console
./measure-apple-concurrency.sh HELPER HDR_TIFF OUTPUT_DIR [ITERATIONS]
```

The output CSV is disposable measurement data. The reviewed Quasar medians and
the resulting initial policy are recorded in `docs/architecture/PROXIES.md`.
