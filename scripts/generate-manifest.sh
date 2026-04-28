#!/usr/bin/env bash
# Build dist/release/latest.json for the updater.
# Usage: ./scripts/generate-manifest.sh <version> <signature>
# Writes to dist/release/latest.json.
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "Usage: $0 <version> <signature>" >&2
  exit 64
fi

VER="$1"
SIG="$2"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="$ROOT/dist/release"
mkdir -p "$OUT_DIR"
OUT="$OUT_DIR/latest.json"

NOTES="$("$ROOT/scripts/extract-release-notes.sh" "$VER")"
PUB_DATE="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
URL="https://github.com/withgenie/hearth/releases/latest/download/Hearth.app.tar.gz"

jq -n \
  --arg version "$VER" \
  --arg notes   "$NOTES" \
  --arg pub     "$PUB_DATE" \
  --arg sig     "$SIG" \
  --arg url     "$URL" \
  '{
    version: $version,
    notes:   $notes,
    pub_date: $pub,
    platforms: {
      "darwin-aarch64": { signature: $sig, url: $url }
    }
  }' > "$OUT"

echo "Wrote $OUT"
