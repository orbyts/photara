# Proxy backend benchmark

This reproducible Stage 6 harness compares backend implementations against the
exact profiles in `contracts.json`. Generated TIFFs and measured outputs stay
outside the repository by default.

The fixture corpus contains:

- an 8000×5333, 16-bit Display-P3 SDR TIFF with deterministic photographic
  texture;
- its 8000×5333, 32-bit floating-point linear-ACEScg HDR counterpart, including
  negative samples and values above 4.0;
- a 1600×1000, 16-bit Display-P3 TIFF tagged with orientation 6.

On the measured Quasar corpus the compressed sources were 179 MiB SDR, 302 MiB
HDR, and 5.7 MiB orientation. The patterns contain asymmetric color patches for
orientation checks and known wide-gamut samples for comparing ICC conversion.

Run on macOS with Xcode command-line tools, libvips, ImageMagick, and Rust:

```console
sh benchmarks/proxy-backends/generate-fixtures.sh /private/tmp/photara-proxy-benchmark
sh benchmarks/proxy-backends/run-benchmarks.sh \
  /private/tmp/photara-proxy-benchmark \
  /private/tmp/photara-proxy-benchmark/results 3
```

The benchmark executables are intentionally disposable characterization code,
not production proxy generators. `apple_proxy.swift` uses only macOS 26-era
ImageIO/Core Image APIs. `vips_proxy.c` exercises explicit ICC import/export and
float TIFF behavior. The Rust executable demonstrates `image` crate behavior;
it intentionally does not pretend that retaining an ICC blob is a color
transform.

Each run writes raw timing logs, `measurements.csv`, and a computed
`correctness.csv`. Correctness compares wide-gamut samples to the Apple
reference within two 8-bit code values and validates profile, HDR depth/range,
and normalized orientation.

The measured decision and interpretation are in
[`docs/architecture/PROXIES.md`](../../docs/architecture/PROXIES.md).
