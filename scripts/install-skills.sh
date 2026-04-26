#!/usr/bin/env bash
# Install or remove symlinks for hearth agent skills into a host's skills dir.
# Usage: install-skills.sh --into <path> [--remove]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SKILLS_DIR="$REPO_ROOT/skills"
LEGACY_SKILLS=(
  hearth-today-brief
  hearth-project-scan
  hearth-memo-organize
)

INTO=""
REMOVE=0

usage() {
  cat <<EOF
Usage: $0 --into <path> [--remove]

  --into <path>    Target skills dir. No default — must be explicit.
  --remove         Remove only the symlinks this script would create.
  -h, --help       Show this help.

Hints (these are NOT defaults — always pass --into explicitly):
  Claude Code:  ~/.claude/skills   (\$CLAUDE_SKILLS_DIR if set)
  Codex:        ~/.codex/skills    (\$CODEX_SKILLS_DIR if set)
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --into)
      [[ $# -ge 2 ]] || { echo "--into needs a value" >&2; usage >&2; exit 64; }
      case "$2" in
        --*) echo "--into value cannot start with '--' (got: $2)" >&2; usage >&2; exit 64 ;;
      esac
      INTO="$2"; shift 2 ;;
    --remove) REMOVE=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 64 ;;
  esac
done

if [[ -z "$INTO" ]]; then
  usage >&2
  exit 64
fi

if [[ -e "$INTO" && ! -d "$INTO" ]]; then
  echo "error: --into path exists and is not a directory: $INTO" >&2
  exit 64
fi

mkdir -p "$INTO"

count=0
legacy_removed=0
cleanup_legacy_links() {
  local name target
  for name in "${LEGACY_SKILLS[@]}"; do
    target="$INTO/$name"
    if [[ -L "$target" ]]; then
      rm "$target"
      echo "removed legacy: $target"
      if [[ $REMOVE -eq 1 ]]; then
        count=$((count+1))
      else
        legacy_removed=$((legacy_removed+1))
      fi
    fi
  done
}

cleanup_legacy_links

for skill_path in "$SKILLS_DIR"/*/; do
  [[ -d "$skill_path" ]] || continue
  [[ -f "$skill_path/SKILL.md" ]] || continue
  name="$(basename "$skill_path")"
  target="$INTO/$name"

  if [[ $REMOVE -eq 1 ]]; then
    if [[ -L "$target" ]]; then
      rm "$target"
      echo "removed: $target"
      count=$((count+1))
    else
      echo "skip (not a symlink we created): $target"
    fi
  else
    if [[ -e "$target" && ! -L "$target" ]]; then
      echo "refusing to overwrite non-symlink: $target" >&2
      exit 1
    fi
    abs_skill="$(cd "$skill_path" && pwd)"
    ln -snf "$abs_skill" "$target"
    echo "linked: $target -> $abs_skill"
    count=$((count+1))
  fi
done

echo
if [[ $REMOVE -eq 1 ]]; then
  echo "Done. Removed $count symlink(s) from $INTO"
else
  echo "Done. Installed $count skill(s) into $INTO"
  if [[ "$legacy_removed" -gt 0 ]]; then
    echo "Also removed $legacy_removed legacy skill symlink(s)."
  fi
  echo "Verify: ls -l \"$INTO\" | grep hearth"
fi
