#!/usr/bin/env bash
# Bump the Hearth version in every manifest atomically.
# Handles the post-workspace-split layout: 1 npm manifest + 1 Tauri config
# + 3 Cargo crate manifests.
#
# Usage: ./scripts/bump-version.sh 0.8.0
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <semver>" >&2
  exit 64
fi

NEW="$1"

if ! [[ "$NEW" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.-]+)?$ ]]; then
  echo "Error: '$NEW' is not a plausible semver string." >&2
  exit 65
fi

# Resolve repo root: prefer the current working directory if it looks like
# a Hearth checkout (so tests and ad-hoc invocations can target a copy),
# otherwise fall back to the script's own parent directory.
if [[ -f "package.json" && -f "src-tauri/app/tauri.conf.json" ]]; then
  ROOT="$(pwd)"
else
  ROOT="$(cd "$(dirname "$0")/.." && pwd)"
fi
cd "$ROOT"

# --- JSON manifests ---

for f in package.json src-tauri/app/tauri.conf.json; do
  [[ -f "$f" ]] || { echo "missing: $f" >&2; exit 66; }
  tmp="$(mktemp)"
  jq --arg v "$NEW" '.version = $v' "$f" > "$tmp"
  mv "$tmp" "$f"
done

# --- Cargo manifests (each crate has a [package] block with version) ---

bump_cargo() {
  local path="$1"
  [[ -f "$path" ]] || { echo "missing: $path" >&2; exit 66; }
  python3 - "$path" "$NEW" <<'PY'
import pathlib, re, sys
path = pathlib.Path(sys.argv[1])
new = sys.argv[2]
text = path.read_text()
out, count = re.subn(
    r'(\[package\][\s\S]*?\nversion\s*=\s*")([^"]+)(")',
    lambda m: m.group(1) + new + m.group(3),
    text,
    count=1,
)
if count == 0:
    sys.exit(f"Failed to locate [package] version in {path}")
path.write_text(out)
PY
}

for crate in src-tauri/app/Cargo.toml src-tauri/core/Cargo.toml src-tauri/cli/Cargo.toml; do
  bump_cargo "$crate"
done

echo "Bumped to $NEW in:"
echo "  package.json"
echo "  src-tauri/app/tauri.conf.json"
echo "  src-tauri/app/Cargo.toml"
echo "  src-tauri/core/Cargo.toml"
echo "  src-tauri/cli/Cargo.toml"
