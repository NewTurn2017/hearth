#!/usr/bin/env bash
# Extract the body (without heading) of a CHANGELOG section.
# Usage: ./scripts/extract-release-notes.sh 0.2.0
# Prints to stdout. Exit non-zero if no such section exists.
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <version>" >&2
  exit 64
fi

VER="$1"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CHANGELOG="$ROOT/CHANGELOG.md"

if [[ ! -f "$CHANGELOG" ]]; then
  echo "CHANGELOG.md not found at $CHANGELOG" >&2
  exit 66
fi

# Emit lines between "## [VER]" and the next "## [" heading (exclusive).
awk -v ver="$VER" '
  BEGIN { found = 0; in_section = 0 }
  /^## \[/ {
    if (in_section) { exit }
    if ($0 ~ "^## \\[" ver "\\]") { found = 1; in_section = 1; next }
  }
  in_section { print }
  END { if (!found) exit 1 }
' "$CHANGELOG" | sed -e '/./,$!d' | awk 'NR>0 { buf = buf $0 "\n" } END { printf "%s", buf }'
