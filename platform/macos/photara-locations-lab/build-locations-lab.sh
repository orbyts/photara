#!/bin/zsh
set -euo pipefail
SCRIPT_ROOT="${0:A:h}"
exec "$SCRIPT_ROOT/../build-library-lab.sh" locations
