#!/usr/bin/env python3
"""Generate public native constants/plist from one reviewed identity descriptor.

Never accepts runtime credentials, arbitrary descriptor paths or endpoint overrides.
"""
import argparse
import base64
import fcntl
import ipaddress
import json
import os
import re
from pathlib import Path
import plistlib
import tempfile
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[1]
IDENTITY_KEYS = set("legacyPreferenceSuites developmentOperatorDirectory developmentOperatorProduct productName executableName projectDocumentUTI legacyProjectPackageExtensions defaultProjectsDirectory journalDirectory displayName shortName isCodename bundleIdentifier keychainService callbackScheme projectPackageDisplayType projectPackageExtension apiAudienceNamespace serviceHostname userAgent applicationSupportDirectory cacheDirectory defaultLibraryName websiteURL supportURL privacyURL storeURL".split())
ENVIRONMENT_KEYS = set("channel environmentId apiOrigin auth0Issuer auth0Audience nativeClientId callbackURL logoutURL schemaFamily schemaEpoch minimumAPI loggingPolicy telemetryEnabled".split())


def require(condition):
    if not condition:
        raise ValueError("invalid-public-release-configuration")


def public_origin(value):
    url = urlsplit(value)
    require(url.scheme == "https" and url.hostname and "." in url.hostname)
    require(not url.hostname.endswith((".localhost", ".local", ".")))
    require(not url.username and not url.password and url.path == "/")
    require(not url.query and not url.fragment and value == f"https://{url.netloc}/")
    try:
        ipaddress.ip_address(url.hostname)
    except ValueError:
        return
    raise ValueError("remote-literal-address-forbidden")


def configuration(descriptor, channel):
    require(set(descriptor) == {"identity", "environments"})
    require(set(descriptor["identity"]) == IDENTITY_KEYS)
    require(set(descriptor["environments"]) == {"development", "remoteAcceptance", "production"})
    for item in descriptor["environments"].values():
        require(item is None or set(item) == ENVIRONMENT_KEYS)
    identity = descriptor["identity"]
    development = descriptor["environments"]["development"]
    environment = descriptor["environments"].get(channel)
    require(environment is not None and environment["channel"] == channel)
    require(environment["schemaFamily"] == "photara.service.g2")
    require(environment["schemaEpoch"] == 1 and environment["minimumAPI"] == 3)
    require(environment["loggingPolicy"] == "redacted-status-only")
    require(environment["telemetryEnabled"] is False)
    require(0 < len(environment["environmentId"]) <= 128)
    require(environment["nativeClientId"] and environment["auth0Audience"])
    public_origin(environment["auth0Issuer"])
    callback = (f'{identity["callbackScheme"]}://{urlsplit(environment["auth0Issuer"]).hostname}'
                f'/macos/{identity["bundleIdentifier"]}/callback')
    require(environment["callbackURL"] == callback and environment["logoutURL"] == callback)
    if channel == "development":
        url = urlsplit(environment["apiOrigin"])
        require(url.scheme == "http" and url.hostname == "127.0.0.1" and url.port)
        require(environment["apiOrigin"] == f"http://127.0.0.1:{url.port}/")
    else:
        public_origin(environment["apiOrigin"])
        require(environment["environmentId"] != development["environmentId"])
        if channel == "production":
            require(identity["isCodename"] is False)
            require(urlsplit(environment["callbackURL"]).scheme != urlsplit(development["callbackURL"]).scheme)
            for key in ("auth0Issuer", "auth0Audience", "nativeClientId", "callbackURL", "logoutURL"):
                require(environment[key] != development[key])
    for key in ("shortName", "productName", "executableName", "applicationSupportDirectory", "cacheDirectory", "defaultProjectsDirectory", "journalDirectory", "developmentOperatorDirectory", "developmentOperatorProduct"):
        value = identity[key]
        require(isinstance(value, str) and value.strip() == value and value and value not in (".", "..") and not any(ord(c) < 32 or c in "/\\:" for c in value))
    require(isinstance(identity["legacyProjectPackageExtensions"], list))
    require(isinstance(identity["legacyPreferenceSuites"], list) and all(isinstance(x, str) and x and not any(ord(c) < 32 for c in x) for x in identity["legacyPreferenceSuites"]))
    for value in [identity["projectPackageExtension"], *identity["legacyProjectPackageExtensions"]]:
        require(isinstance(value, str) and re.fullmatch(r"[a-z][a-z0-9]{0,31}", value))
    require(isinstance(identity["legacyProjectPackageExtensions"], list))
    require(len(set(identity["legacyProjectPackageExtensions"])) == len(identity["legacyProjectPackageExtensions"]))
    for key in ("bundleIdentifier", "projectDocumentUTI", "callbackScheme", "keychainService"):
        require(isinstance(identity[key], str) and re.fullmatch(r"[A-Za-z][A-Za-z0-9.-]+", identity[key]))
    for key in ("displayName", "userAgent", "projectPackageDisplayType", "defaultLibraryName"):
        require(isinstance(identity[key], str) and identity[key].strip() and not any(ord(c) < 32 for c in identity[key]))
    return {"identity": identity, "environment": environment}


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result)
        result[key] = value
    return result


def write_generated(path, contents):
    # Shared native builds may run concurrently. Do not invalidate swiftc's
    # inputs when the checked coordinates have not changed.
    with path.with_suffix(".lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        if path.exists() and path.read_text() == contents:
            return
        with tempfile.NamedTemporaryFile(mode="w", dir=path.parent, delete=False) as temporary:
            temporary.write(contents)
        os.replace(temporary.name, path)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--channel", choices=("development", "remoteAcceptance", "production"), required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--plist", type=Path)
    args = parser.parse_args()
    selected = configuration(json.loads((ROOT / "config/product-identity.json").read_text(),
                                        object_pairs_hook=unique_object), args.channel)
    encoded = base64.b64encode(json.dumps(selected, separators=(",", ":")).encode()).decode()
    args.output.mkdir(parents=True, exist_ok=True)
    write_generated(args.output / "CheckedReleaseConfiguration.swift",
        '// Generated public coordinates; edit config/product-identity.json.\nimport Foundation\n'
        'extension ReleaseConfiguration {\n'
        '    static let current = try! JSONDecoder().decode(ReleaseConfiguration.self,\n'
        f'        from: Data(base64Encoded: "{encoded}")!)\n'
        '}\n')
    if args.plist:
        with (ROOT / "platform/macos/photara-app/Resources/Info.plist").open("rb") as source:
            info = plistlib.load(source)
        identity = selected["identity"]
        package_type = identity["projectDocumentUTI"]
        info["UTExportedTypeDeclarations"] = [{
            "UTTypeIdentifier": package_type,
            "UTTypeDescription": identity["projectPackageDisplayType"],
            "UTTypeConformsTo": ["com.apple.package"],
            "UTTypeTagSpecification": {"public.filename-extension": list(dict.fromkeys([identity["projectPackageExtension"], *identity["legacyProjectPackageExtensions"]]))},
        }]
        info["CFBundleDocumentTypes"] = [{
            "CFBundleTypeName": identity["projectPackageDisplayType"],
            "CFBundleTypeRole": "Editor", "LSHandlerRank": "Owner",
            "LSTypeIsPackage": True, "LSItemContentTypes": [package_type],
        }]
        info.update(CFBundleDisplayName=identity["displayName"], CFBundleName=identity["shortName"],
                    CFBundleExecutable=identity["executableName"], CFBundleIdentifier=identity["bundleIdentifier"],
                    PhotaraReleaseChannel=args.channel,
                    CFBundleURLTypes=[{"CFBundleURLName": identity["bundleIdentifier"],
                                       "CFBundleURLSchemes": [identity["callbackScheme"]]}])
        with args.plist.open("wb") as target:
            plistlib.dump(info, target)
    print(selected["identity"]["productName"])


if __name__ == "__main__":
    main()
