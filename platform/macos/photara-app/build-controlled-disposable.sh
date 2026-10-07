#!/bin/zsh
set -euo pipefail
SCRIPT_ROOT="${0:A:h}"
# This enables the reviewed controlled registrar only. No installation/launch.
if [[ -n "${PHOTARA_MACOS_PROVISIONING_PROFILE:-}" || "${PHOTARA_RELEASE_CHANNEL:-development}" != development ]]; then
  print -u2 -- "Controlled disposable build permits development/ad-hoc signing only"; exit 2
fi
export PHOTARA_CONTROLLED_DISPOSABLE_BUILD=1
export PHOTARA_APP_BUILD_ROOT="${PHOTARA_APP_BUILD_ROOT:-$SCRIPT_ROOT/.build/controlled-disposable}"
exec zsh "$SCRIPT_ROOT/build-app.sh"
