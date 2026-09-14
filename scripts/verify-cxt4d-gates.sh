#!/bin/zsh
# No UI build is produced until native, real-process and installed-service gates pass.
set -euo pipefail
SCRIPT_ROOT="${0:A:h}"
REPOSITORY_ROOT="${SCRIPT_ROOT:h}"
cd "$REPOSITORY_ROOT"
: "${PHOTARA_MACOS_PROVISIONING_PROFILE:?Set the desktop profile}"
: "${PHOTARA_OPERATOR_PROVISIONING_PROFILE:?Set the operator profile}"
export PHOTARA_APP_RUST_TARGET="${PHOTARA_APP_RUST_TARGET:-/private/tmp/photara-cxt4d-furnace-rust}"
export PHOTARA_BRIDGE_RUST_TARGET="$PHOTARA_APP_RUST_TARGET"
EVIDENCE="$REPOSITORY_ROOT/platform/macos/photara-app/.build/cxt4d-gates"
mkdir -p "$EVIDENCE"
python3 - "$EVIDENCE/source-before.json" <<'PY'
import hashlib,json,pathlib,sys
root=pathlib.Path.cwd()
# A failed new run must never leave an older passing marker looking current.
pathlib.Path(sys.argv[1]).with_name('passed.json').unlink(missing_ok=True)
paths=list(root.glob('platform/macos/photara-ui-foundation/Sources/*.swift'))
paths+=list(root.glob('platform/macos/photara-app/Sources/*.swift'))
paths+=list(root.glob('platform/macos/photara-shell/Sources/*.swift'))
paths+=list(root.glob('scripts/*service*.swift'))+[root/'config/product-identity.json']
paths+=list(root.glob('crates/photara-service/src/**/*.rs'))+list(root.glob('crates/photara-library/src/gen2/onboarding.rs'))+list(root.glob('crates/photara-bridge/src/onboarding.rs'))
paths+=list(root.glob('platform/macos/photara-app/Tests/**/*.swift'))+list(root.glob('scripts/*furnace*'))+[root/'scripts/verify_service_postgres.py']
paths+=list(root.glob('crates/photara-library/migrations/generation_two/*.sql'))+list(root.glob('crates/photara-library/src/gen2/*.rs'))+list(root.glob('platform/macos/photara-ui-foundation/Tests/*.swift'))
paths+=list(root.glob('platform/macos/photara-ui-tests/*.swift'))+[root/'scripts/verify-cxt4d-gates.sh']
paths=sorted(set(paths))
pathlib.Path(sys.argv[1]).write_text(json.dumps({str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(paths)},indent=2)+'\n')
PY
zsh scripts/test-native-authentication.sh
zsh scripts/test-native-onboarding.sh
python3 scripts/verify_service_postgres.py
zsh scripts/test-service-process-furnace.sh
zsh scripts/test-installed-service.sh
zsh scripts/test-native-onboarding-keychain.sh
CARGO_TARGET_DIR="$PHOTARA_APP_RUST_TARGET" cargo test -p photara-library -p photara-bridge -p photara-service
platform/macos/photara-app/verify-bridge.sh
python3 scripts/test_product_configuration.py
python3 scripts/verify_generation_two_schema.py
python3 scripts/verify_generation_two_naming.py
git diff --check
zsh platform/macos/photara-ui-tests/verify-shared-ui.sh
zsh platform/macos/photara-ui-tests/verify-production-ui.sh
codesign --verify --strict --verbose=2 platform/macos/photara-app/.build/app/Photara.app
python3 - "$EVIDENCE" <<'PY'
import hashlib,json,pathlib,sys
root=pathlib.Path.cwd(); evidence=pathlib.Path(sys.argv[1]); before=json.loads((evidence/'source-before.json').read_text())
assert all(hashlib.sha256((root/p).read_bytes()).hexdigest()==digest for p,digest in before.items()), 'Source changed during gate execution'
app=root/'platform/macos/photara-app/.build/app/Photara.app'
report={'status':'passed','executable_sha256':hashlib.sha256((app/'Contents/MacOS/Photara').read_bytes()).hexdigest(),'app':str(app),'source_sha256':before,'live_google_acceptance':False,'neon_mutations':False}
(evidence/'passed.json').write_text(json.dumps(report,indent=2)+'\n')
print('CXT4d gates passed; signed build and exact source manifest:',evidence/'passed.json')
PY
