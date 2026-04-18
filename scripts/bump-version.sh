#!/usr/bin/env bash
# Bump the Hearth version in all three manifests atomically.
# Usage: ./scripts/bump-version.sh 0.3.0
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <semver>" >&2
  exit 64
fi

NEW="$1"

# Basic semver shape check (X.Y.Z with optional -prerelease)
if ! [[ "$NEW" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.-]+)?$ ]]; then
  echo "Error: '$NEW' is not a plausible semver string." >&2
  exit 65
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# package.json
tmp="$(mktemp)"
jq --arg v "$NEW" '.version = $v' package.json > "$tmp"
mv "$tmp" package.json

# src-tauri/tauri.conf.json
tmp="$(mktemp)"
jq --arg v "$NEW" '.version = $v' src-tauri/tauri.conf.json > "$tmp"
mv "$tmp" src-tauri/tauri.conf.json

# src-tauri/Cargo.toml — bespoke edit of the first `version = "…"` after [package]
python3 - "$NEW" <<'PY'
import pathlib, re, sys
new = sys.argv[1]
p = pathlib.Path("src-tauri/Cargo.toml")
text = p.read_text()
out, count = re.subn(
    r'(\[package\][\s\S]*?\nversion\s*=\s*")([^"]+)(")',
    lambda m: m.group(1) + new + m.group(3),
    text,
    count=1,
)
if count == 0:
    sys.exit("Failed to locate version in Cargo.toml [package] block")
p.write_text(out)
PY

echo "Bumped to $NEW in package.json, tauri.conf.json, Cargo.toml"
