#!/bin/zsh
set -euo pipefail
SCRIPT_ROOT="${0:A:h}"
export PHOTARA_LOCAL_LIBRARY_DISPOSABLE_BUILD=1
export PHOTARA_APP_BUILD_ROOT="${PHOTARA_APP_BUILD_ROOT:-$SCRIPT_ROOT/.build/local-libraries}"
exec zsh "$SCRIPT_ROOT/build-controlled-disposable.sh"
