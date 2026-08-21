# Photara ImageIO/Core Image proxy helper

This short-lived macOS executable is the first production generator behind the
backend-neutral `photara-proxy` service. It implements the two exact Stage 6A
paths: a Display P3/sRGB SDR thumbnail to sRGB U8 PNG, and an embedded-color HDR
authoring preview to F16 TIFF. Unsupported source/profile combinations fail
explicitly.

Build with the stable toolchain available on Quasar:

```console
swift build -c release -Xswiftc -warnings-as-errors
```

The application runtime supplies the resulting executable path through
`ImageIoGeneratorConfig`. The helper uses Foundation, ImageIO, Core Graphics,
and Core Image APIs available before macOS 27. Its process boundary releases the
large decoder working set after each bounded generation; it is not a UI target
or the Swift application facade.
