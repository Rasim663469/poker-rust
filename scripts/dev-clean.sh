#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "==> Nettoyage dev pour $(basename "$ROOT_DIR")"
echo
echo "[Avant]"
df -h .
if [ -d target ]; then
  du -sh target || true
else
  echo "target/ n'existe pas encore."
fi

echo
echo "==> cargo clean"
cargo clean

echo
echo "[Apres]"
df -h .
if [ -d target ]; then
  du -sh target || true
else
  echo "target/ supprime (recree automatiquement au prochain build)."
fi

echo
echo "Nettoyage termine."
