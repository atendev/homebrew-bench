#!/usr/bin/env bash
# Pulls SRD JSON and TypeScript schemas from 5e-bits/5e-database into data/srd/.
# Usage: scripts/import-srd.sh [commit]   (defaults to the pinned commit below)
set -euo pipefail

REPO_URL="https://github.com/5e-bits/5e-database"
COMMIT="${1:-a6212beb4b278917c2cff41c2b04c4ef4f3e0d6f}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$ROOT/data/srd"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

git -C "$WORK" init -q src
git -C "$WORK/src" remote add origin "$REPO_URL"
git -C "$WORK/src" fetch -q --depth 1 origin "$COMMIT"
git -C "$WORK/src" checkout -q FETCH_HEAD

rm -rf "$DEST/2024"
mkdir -p "$DEST/2024/schemas"
cp "$WORK/src/src/2024/en/"*.json "$DEST/2024/"
cp "$WORK/src/src/2024/schemas/"*.ts "$DEST/2024/schemas/"
cp "$WORK/src/LICENSE.md" "$DEST/LICENSE-5e-database.md"
echo "$COMMIT" > "$DEST/COMMIT"

echo "Imported 5e-database@$COMMIT into $DEST"
