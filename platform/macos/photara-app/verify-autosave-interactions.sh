#!/bin/zsh
set -euo pipefail
SCRIPT_ROOT="${0:A:h}"
if [[ $# -gt 1 || ( $# == 1 && "$1" != --full-autosave ) ]]; then
  print -u2 -- 'Usage: verify-autosave-interactions.sh [--full-autosave]'; exit 2
fi
zsh "$SCRIPT_ROOT/verify-controlled-session.sh"
REPOSITORY_ROOT="${SCRIPT_ROOT:h:h:h}"
BUILD_ROOT="${PHOTARA_APP_BUILD_ROOT:-$SCRIPT_ROOT/.build/app}"
RUST_TARGET="${PHOTARA_APP_RUST_TARGET:-${CARGO_TARGET_DIR:-$BUILD_ROOT/rust-target}}"
(
  cd "$REPOSITORY_ROOT"
  CARGO_TARGET_DIR="$RUST_TARGET" \
    CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_DEBUG=2 \
    CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_STRIP=none \
    cargo test -p photara-store --release --lib session_tests -- --nocapture
)
print -- 'GAP: add node is controller-local only; native durable admission is unavailable.'
print -- 'GAP: delete node is not implemented by the shared controller or durable adapter.'
print -- 'GAP: connect has controller/Core support but is refused by the current native durable adapter.'
print -- 'Scope: current regression gate; no claim of full add/delete/connect autosave acceptance.'
if [[ "${1:-}" == --full-autosave ]]; then
  print -u2 -- 'FAIL: full-autosave acceptance requires the three capabilities above and their native persistence evidence.'
  exit 1
fi
