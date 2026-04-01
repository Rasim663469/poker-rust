#!/usr/bin/env bash
set -euo pipefail

REMOTE_USER="${REMOTE_USER:-rasim.erben}"
REMOTE_HOST="${REMOTE_HOST:-162.38.111.42}"
REMOTE_APP_DIR="${REMOTE_APP_DIR:-/home/rasim.erben/poker-rust}"
REMOTE_ENV_URL="${REMOTE_ENV_URL:-postgres://poker:poker@127.0.0.1:5433/poker}"

echo "==> Build local du serveur"
cargo build --release --bin server

echo "==> Copie du binaire sur ${REMOTE_USER}@${REMOTE_HOST}"
scp target/release/server "${REMOTE_USER}@${REMOTE_HOST}:${REMOTE_APP_DIR}/server"

echo "==> Redemarrage du serveur distant"
ssh "${REMOTE_USER}@${REMOTE_HOST}" "bash -lc '
set -euo pipefail
cd \"${REMOTE_APP_DIR}\"
cat > .env <<EOF
DATABASE_URL=${REMOTE_ENV_URL}
EOF
chmod +x server
pkill -f \"${REMOTE_APP_DIR}/server\" 2>/dev/null || true
nohup \"${REMOTE_APP_DIR}/server\" > \"${REMOTE_APP_DIR}/server.log\" 2>&1 &
sleep 2
if ss -ltn | grep -q ':9090 '; then
  echo \"Serveur relance sur 9090. Dernieres lignes de log:\"
  tail -n 20 \"${REMOTE_APP_DIR}/server.log\" || true
else
  echo \"Echec du redemarrage du serveur. Dernieres lignes de log:\" >&2
  tail -n 50 \"${REMOTE_APP_DIR}/server.log\" >&2 || true
  exit 1
fi
'"

echo "==> Deploiement termine"
