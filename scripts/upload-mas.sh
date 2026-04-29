#!/usr/bin/env bash
# Upload the most recent MAS .pkg to App Store Connect.
#
# Build and upload are split because the build number is consumed on every
# upload (App Store rejects duplicates). Two commands force an explicit
# decision before burning a build.
#
# Spec: docs/superpowers/specs/2026-04-26-mas-readiness-design.md §5
#
# Usage:
#   bash scripts/upload-mas.sh
#
# Required env:
#   APP_STORE_API_KEY_ID
#   APP_STORE_API_ISSUER_ID
#   API_PRIVATE_KEYS_DIR  (defaults to $HOME/.private_keys)

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ -z "${APP_STORE_API_KEY_ID:-}" || -z "${APP_STORE_API_ISSUER_ID:-}" ]]; then
  echo "upload-mas: APP_STORE_API_KEY_ID / APP_STORE_API_ISSUER_ID must be exported" >&2
  exit 1
fi

PKG="$(ls -t dist-mas/*.pkg 2>/dev/null | head -1 || true)"
[[ -n "$PKG" ]] || { echo "upload-mas: no .pkg in dist-mas/ — run scripts/build-mas.sh first" >&2; exit 1; }

echo "==> Uploading $PKG"

# TODO(D14 deadline = 2026-05-10): see build-mas.sh — fall back to
#   `xcrun iTMSTransporter -m upload` if altool MAS upload is removed.
xcrun altool --upload-app -f "$PKG" -t macos \
  --apiKey "$APP_STORE_API_KEY_ID" \
  --apiIssuer "$APP_STORE_API_ISSUER_ID"

echo "✅ Upload accepted by App Store Connect — wait ~10–30 min for processing."
