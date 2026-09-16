#!/usr/bin/env python3
"""BR0 offline source/config guard and optional isolated synthetic app build/furnace.

No runtime descriptor override is added to the product. Synthetic compilation uses
an isolated source snapshot whose checked descriptor has been replaced by a fixture.
"""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import tempfile

import generate_product_configuration as product

ROOT = product.ROOT


def source_guard():
    # Exact compatibility exceptions, not a blanket allowlist for public names.
    exceptions = {
        ("platform/macos/photara-shell/Sources/ApplicationShellPreset.swift", '"PhotaraDeveloperShellPresetV1"'),
    }
    observed = set()
    for surface in ("photara-app", "photara-shell", "photara-ui-foundation"):
        for path in (ROOT / "platform/macos" / surface / "Sources").glob("*.swift"):
            for line in path.read_text().splitlines():
                if line.lstrip().startswith("//"):
                    continue
                for literal in re.findall(r'"[^"\n]*"', line):
                    if "Photara" in literal or literal == '".photara"':
                        key = (str(path.relative_to(ROOT)), literal)
                        assert key in exceptions, f"Unconfigured public name: {key}"
                        observed.add(key)
    assert observed == exceptions, "Review stale compatibility exceptions"
    rust_exceptions = {"photara", ".photara-create-{}-"} # Persisted request/stage read aliases only.
    for name in ("crates/photara-core/src/creation.rs", "crates/photara-store/src/package/creation/filesystem.rs",
                 "crates/photara-library/src/gen2/creation.rs", "crates/photara-bridge/src/creation.rs"):
        source = (ROOT / name).read_text().split("#[cfg(test)]")[0]
        # Core's typed alias implementation follows the original test module.
        if name.endswith("photara-core/src/creation.rs"):
            source += (ROOT / name).read_text().split("/// Validated outer filename policy")[1].split("#[cfg(test)]")[0]
        for literal in re.findall(r'"([^"\n]*)"', source):
            if "Photara" in literal or ".photara" in literal or literal == "photara":
                assert literal in rust_exceptions, f"Hard-coded Rust filename policy: {name}: {literal}"
    descriptor = json.loads((ROOT / "config/product-identity.json").read_text())
    product.configuration(descriptor, "development")
    # Never turn a brand switch into a persisted namespace migration.
    assert descriptor["environments"]["development"]["schemaFamily"] == "photara.service.g2"
    print("BR0 source/config guards: passed")


def synthetic_descriptor():
    d = json.loads((ROOT / "config/product-identity.json").read_text())
    d["identity"].update(
        displayName="Juniper Studio", shortName="Juniper", productName="Juniper Studio",
        executableName="JuniperHost", bundleIdentifier="org.example.juniper",
        projectDocumentUTI="org.example.juniper.project", projectPackageDisplayType="Juniper Project",
        projectPackageExtension="jprtest", legacyProjectPackageExtensions=["photara"],
        callbackScheme="org.example.juniper", keychainService="org.example.juniper.auth",
        apiAudienceNamespace="urn:juniper:api", userAgent="Juniper/0.2.0",
        applicationSupportDirectory="JuniperSupport", cacheDirectory="JuniperCache",
        defaultProjectsDirectory="Juniper", journalDirectory="Recovery", legacyPreferenceSuites=[],
        developmentOperatorDirectory="juniper-operator", developmentOperatorProduct="Juniper Operator",
        websiteURL="https://example.org/", supportURL="https://example.org/support",
        privacyURL="https://example.org/privacy", storeURL="https://example.org/store")
    e = d["environments"]["development"]
    e.update(auth0Issuer="https://identity.example.org/", auth0Audience="urn:juniper:api:test",
             nativeClientId="synthetic-never-enrolled", environmentId="synthetic-brand",
             callbackURL="org.example.juniper://identity.example.org/macos/org.example.juniper/callback",
             logoutURL="org.example.juniper://identity.example.org/macos/org.example.juniper/callback")
    product.configuration(d, "development")
    return d


def run(command, **kwargs):
    subprocess.run(command, check=True, **kwargs)


def byte_manifest(path):
    return {str(p.relative_to(path)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in sorted(path.rglob("*")) if p.is_file()}


def build_furnace(output):
    output.mkdir(parents=True, exist_ok=False)
    snapshot = output / "source"
    snapshot.mkdir()
    names = subprocess.check_output(["git", "ls-files", "--cached", "--others", "--exclude-standard"], cwd=ROOT, text=True).splitlines()
    for name in names:
        source = ROOT / name
        if source.is_file():
            target = snapshot / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)
    descriptor = synthetic_descriptor()
    (snapshot / "config/product-identity.json").write_text(json.dumps(descriptor, indent=2) + "\n")
    build = output / "build"
    env = dict(os.environ, PHOTARA_APP_BUILD_ROOT=str(build),
               PHOTARA_APP_RUST_TARGET=str(output / "target"), CARGO_NET_OFFLINE="true")
    env.pop("PHOTARA_MACOS_PROVISIONING_PROFILE", None)
    env["PHOTARA_RELEASE_CHANNEL"] = "development"
    run([str(snapshot / "platform/macos/photara-app/build-app.sh")], cwd=snapshot, env=env)
    verify_build(output, descriptor)


def verify_build(output, descriptor):
    snapshot = output / "source"
    build = output / "build"
    env = dict(os.environ)
    app = build / "Juniper Studio.app"
    info = plistlib.loads((app / "Contents/Info.plist").read_bytes())
    i = descriptor["identity"]
    for key, field in [("CFBundleExecutable", "executableName"), ("CFBundleIdentifier", "bundleIdentifier"),
                       ("CFBundleDisplayName", "displayName"), ("CFBundleName", "shortName")]:
        assert info[key] == i[field]
    declaration = info["UTExportedTypeDeclarations"][0]
    assert declaration["UTTypeIdentifier"] == i["projectDocumentUTI"]
    assert declaration["UTTypeTagSpecification"]["public.filename-extension"] == ["jprtest", "photara"]
    assert info["CFBundleURLTypes"][0]["CFBundleURLSchemes"] == [i["callbackScheme"]]
    assert (app / "Contents/MacOS/JuniperHost").exists()
    generated = build / "generated"
    generated_source = (generated / "CheckedReleaseConfiguration.swift").read_text()
    encoded = re.search(r'Data\(base64Encoded: "([A-Za-z0-9+/=]+)"', generated_source).group(1)
    assert json.loads(base64.b64decode(encoded)) == product.configuration(descriptor, "development")
    executable = output / "brand-furnace"
    run(["xcrun", "swiftc", "-swift-version", "6", "-parse-as-library", "-module-cache-path", str(build / "module-cache"),
         str(generated / "PhotaraBridge.swift"), str(generated / "CheckedReleaseConfiguration.swift"),
         str(snapshot / "platform/macos/photara-ui-foundation/Sources/ReleaseConfiguration.swift"),
         str(ROOT / "platform/macos/photara-app/Tests/BrandReadiness/main.swift"),
         "-Xcc", f"-fmodule-map-file={generated}/PhotaraBridgeFFI.modulemap",
         "-L", str(output / "target/debug"), "-lphotara_bridge", "-o", str(executable)])
    env["DYLD_LIBRARY_PATH"] = str(output / "target/debug")
    fixture = Path(tempfile.mkdtemp(prefix="fixture-", dir=output))
    for alias in ("current", "legacy"):
        run([str(executable), "create-" + alias, str(fixture)], env=env)
        receipt = json.loads((fixture / (alias + ".json")).read_text())
        package = Path(receipt["path"])
        before = byte_manifest(package)
        run([str(executable), "reopen-" + alias, str(fixture)], env=env)
        assert byte_manifest(package) == before, "Reopen changed portable bytes"
        (output / (alias + "-manifest.json")).write_text(json.dumps(before, indent=2) + "\n")
    run([str(executable), "create-pending-legacy", str(fixture)], env=env)
    run([str(executable), "reopen-pending-legacy", str(fixture)], env=env)
    print(f"BR0 synthetic build, generated metadata, separate-process creation/reopen and legacy alias: passed; {output}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build-output", type=Path, help="new disposable directory below /private/tmp")
    args = parser.parse_args()
    source_guard()
    if args.build_output:
        path = args.build_output.resolve()
        assert str(path).startswith("/private/tmp/") and not path.exists()
        build_furnace(path)
