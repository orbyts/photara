#!/usr/bin/env python3
"""Validate an Apple profile and emit exact development entitlements.

The profile and certificate are host inputs, never repository configuration.
Only an exact application identifier and its private Keychain group are emitted.
"""

import argparse
import datetime as dt
import hashlib
import plistlib
import subprocess
from pathlib import Path


def refuse(message: str) -> None:
    raise SystemExit(f"photara-signing:{message}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", required=True, type=Path)
    parser.add_argument("--bundle-id", required=True)
    parser.add_argument("--entitlements", required=True, type=Path)
    args = parser.parse_args()

    if not args.profile.is_file() or not args.bundle_id or any(
        character.isspace() for character in args.bundle_id
    ):
        refuse("invalid-input")
    decoded = subprocess.run(
        ["/usr/bin/security", "cms", "-D", "-i", str(args.profile)],
        check=False,
        capture_output=True,
    )
    if decoded.returncode:
        refuse("profile-decode-failed")
    try:
        profile = plistlib.loads(decoded.stdout)
        teams = profile["TeamIdentifier"]
        allowed = profile["Entitlements"]
        certificates = profile["DeveloperCertificates"]
        expiration = profile["ExpirationDate"]
    except (KeyError, TypeError, ValueError, plistlib.InvalidFileException):
        refuse("profile-shape-invalid")
    if len(teams) != 1 or not isinstance(teams[0], str) or not teams[0]:
        refuse("profile-team-invalid")
    if not isinstance(expiration, dt.datetime):
        refuse("profile-expiration-invalid")
    now = dt.datetime.now(dt.UTC)
    if expiration.tzinfo is None:
        expiration = expiration.replace(tzinfo=dt.UTC)
    if expiration <= now:
        refuse("profile-expired")
    team = teams[0]
    application_identifier = f"{team}.{args.bundle_id}"
    allowed_identifier = allowed.get("com.apple.application-identifier") or allowed.get("application-identifier")
    allowed_groups = allowed.get("keychain-access-groups", [])

    def admitted(value: str, candidate: str) -> bool:
        return value == candidate or (value.endswith(".*") and candidate.startswith(value[:-1]))

    if not isinstance(allowed_identifier, str) or not admitted(allowed_identifier, application_identifier):
        refuse("bundle-not-authorized")
    if not isinstance(allowed_groups, list) or not any(
        isinstance(group, str) and admitted(group, application_identifier)
        for group in allowed_groups
    ):
        refuse("keychain-group-not-authorized")
    if not certificates or not all(isinstance(value, bytes) for value in certificates):
        refuse("profile-certificate-invalid")

    entitlements = {
        "com.apple.application-identifier": application_identifier,
        "com.apple.developer.team-identifier": team,
        "com.apple.security.get-task-allow": True,
        "keychain-access-groups": [application_identifier],
    }
    args.entitlements.parent.mkdir(parents=True, exist_ok=True)
    with args.entitlements.open("wb") as output:
        plistlib.dump(entitlements, output, fmt=plistlib.FMT_XML, sort_keys=True)
    print(hashlib.sha1(certificates[0]).hexdigest().upper())


if __name__ == "__main__":
    main()
