#!/bin/zsh
# Reassemble all supported component labs and the production app from shared source.
set -euo pipefail
UI_ROOT="${0:A:h}"
"$UI_ROOT/photara-graph-lab/build-graph-lab.sh"
"$UI_ROOT/photara-gallery-lab/build-gallery-lab.sh"
"$UI_ROOT/photara-inspector-lab/build-inspector-lab.sh"
"$UI_ROOT/photara-shell-lab/build-shell-lab.sh"
"$UI_ROOT/photara-app/build-app.sh"
