#!/bin/zsh
# Launch the exact existing signed binary. This helper never builds or runs the Graph matrix.
set -euo pipefail
SCRIPT_ROOT="${0:A:h}"
BUILD_ROOT="$SCRIPT_ROOT/.build/verification"
APP_BUNDLE="$BUILD_ROOT/Graph Lab Verification.app"
if [[ $# != 1 || ( "$1" != --request-permissions && "$1" != --permissions-only && "$1" != --probe-permissions ) ]]; then
  print -u2 -- "Usage: $0 --request-permissions | --permissions-only | --probe-permissions"
  exit 2
fi
if [[ ! -d "$APP_BUNDLE" ]]; then
  print -u2 -- "Build the signed verifier first with verify-interactions.sh --build-only."
  exit 1
fi
# Authority is a diagnostic display string and may be unavailable on macOS 27.
# Authenticate the actual leaf certificate and recorded team/requirement instead.
python3 - "$APP_BUNDLE" "$BUILD_ROOT" <<'PY_SIGNATURE'
from pathlib import Path
import os
import re
import subprocess
import sys

app, build = Path(sys.argv[1]), Path(sys.argv[2])
def run(*args):
    result = subprocess.run(args, text=True, capture_output=True)
    if result.returncode:
        sys.exit('Signature validation failed: ' + result.stderr.strip())
    return result.stdout + result.stderr

def field(pattern, text, label):
    match = re.search(pattern, text, re.M)
    if not match:
        sys.exit('Signature validation failed: missing ' + label)
    return match[1]

identities = sorted(set(re.findall(
    r'^\s*\d+\) ([0-9A-Fa-f]{40}) "Apple Development:[^"\n]+"',
    run('/usr/bin/security', 'find-identity', '-v', '-p', 'codesigning'), re.M)))
requested = os.environ.get('PHOTARA_GRAPH_VERIFY_SIGNING_SHA1', '').upper()
if not identities or (requested and requested not in identities):
    sys.exit('A matching valid Apple Development identity is required; check Keychain access and PHOTARA_GRAPH_VERIFY_SIGNING_SHA1.')
identity = requested or identities[0]
try:
    recorded_signature = (build / 'signature.txt').read_text()
    recorded_requirement = (build / 'designated-requirement.txt').read_text()
except OSError:
    sys.exit('Missing signed-build identity records; run verify-interactions.sh --build-only first.')
team = field(r'^TeamIdentifier=([A-Z0-9]{10})$', recorded_signature, 'recorded team')
requirement = field(r'^designated => (.+)$', recorded_requirement, 'recorded designated requirement')
metadata = run('/usr/bin/codesign', '-dv', '--verbose=4', str(app))
if field(r'^TeamIdentifier=([A-Z0-9]{10})$', metadata, 'team') != team:
    sys.exit('Signature validation failed: team changed since build.')
flags = int(field(r'flags=0x([0-9a-fA-F]+)', metadata, 'code-signing flags'), 16)
if flags & 2:
    sys.exit('Signature validation failed: ad-hoc code is not eligible for permission grants.')
current_requirement = field(r'^designated => (.+)$',
    run('/usr/bin/codesign', '-d', '-r-', str(app)), 'designated requirement')
if current_requirement != requirement:
    sys.exit('Signature validation failed: designated requirement changed since build.')
# Exact certificate fingerprint binds the configured identity. The Apple anchor,
# certificate team and bundle identifier must also hold cryptographically.
constraint = (f'identifier "com.photara.graph-lab.verification" and anchor apple generic '
              f'and certificate leaf = H"{identity}" '
              f'and certificate leaf[subject.OU] = "{team}"')
run('/usr/bin/codesign', '--verify', '--strict', '-R', '=' + constraint, str(app))
print(f'SIGNATURE: exact Apple Development certificate {identity}; team {team}; designated requirement unchanged')
PY_SIGNATURE
print -r -- "Exact signed app: $APP_BUNDLE"
exec python3 "$SCRIPT_ROOT/launch-verification.py" --app "$APP_BUNDLE" --run-timeout 120 -- "$1"
