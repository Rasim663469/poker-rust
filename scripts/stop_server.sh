#!/usr/bin/env bash
set -euo pipefail

REMOTE_USER="${REMOTE_USER:-rasim.erben}"
REMOTE_HOST="${REMOTE_HOST:-162.38.111.42}"
REMOTE_APP_DIR="${REMOTE_APP_DIR:-/home/rasim.erben/poker-rust}"

ssh "${REMOTE_USER}@${REMOTE_HOST}" "bash -lc '
set -euo pipefail
pkill -f \"${REMOTE_APP_DIR}/server\" 2>/dev/null || true
echo \"Serveur arrete.\"
'"
