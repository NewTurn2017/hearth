#!/usr/bin/env bash
# Hearth one-line installer.
#
# Usage:
#   curl -sSL https://raw.githubusercontent.com/NewTurn2017/hearth/main/scripts/install.sh | bash
#   curl -sSL ... | bash -s -- --version v0.8.0
#   curl -sSL ... | bash -s -- --uninstall
#
# Env overrides:
#   HEARTH_VERSION     — pin a tag (default: latest release)
#   HEARTH_BIN_DIR     — binary install dir (default: $HOME/.local/bin)
#   HEARTH_SKILLS_DIR  — skills link dir (default: auto-detect ~/.claude + ~/.codex)
#   HEARTH_STAGING_DIR — versioned skill staging dir (default: $HOME/.local/share/hearth)
#   HEARTH_RELEASES_URL — release asset base URL (default: GitHub download URL)
#   HEARTH_PLATFORM_OVERRIDE — test-only: force "OS-ARCH" string (e.g. "Darwin-arm64")

set -euo pipefail

# ---- constants ----
REPO_SLUG="NewTurn2017/hearth"
RELEASES_BASE="${HEARTH_RELEASES_URL:-https://github.com/$REPO_SLUG/releases/download}"
API_LATEST="https://api.github.com/repos/$REPO_SLUG/releases/latest"

DEFAULT_BIN_DIR="$HOME/.local/bin"
DEFAULT_STAGING_DIR="$HOME/.local/share/hearth"

# ---- parsed args ----
MODE="install"     # install | uninstall
DRY_RUN=0
ARG_VERSION=""
ARG_PREFIX=""
ARG_SKILLS_DIR=""

usage() {
  cat <<EOF
hearth installer

One-liner:
  curl -sSL https://raw.githubusercontent.com/$REPO_SLUG/main/scripts/install.sh | bash

Flags (pass via: curl ... | bash -s -- <flags>):
  --version X.Y.Z         Pin specific version tag (default: latest release)
  --prefix DIR            Binary install dir (default: \$HOME/.local/bin)
  --skills-dir DIR        Skills link dir (default: auto-detect ~/.claude, ~/.codex)
  --uninstall             Remove binary + symlinks this script would create
  --dry-run               Print planned actions without writing
  -h, --help              Show this help

Env overrides: HEARTH_VERSION, HEARTH_BIN_DIR, HEARTH_SKILLS_DIR, HEARTH_STAGING_DIR
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      [[ $# -ge 2 && "$2" != --* ]] || { echo "--version needs a non-flag value" >&2; exit 64; }
      ARG_VERSION="$2"; shift 2 ;;
    --prefix)
      [[ $# -ge 2 && "$2" != --* ]] || { echo "--prefix needs a non-flag value" >&2; exit 64; }
      ARG_PREFIX="$2"; shift 2 ;;
    --skills-dir)
      [[ $# -ge 2 && "$2" != --* ]] || { echo "--skills-dir needs a non-flag value" >&2; exit 64; }
      ARG_SKILLS_DIR="$2"; shift 2 ;;
    --uninstall) MODE="uninstall"; shift ;;
    --dry-run)   DRY_RUN=1; shift ;;
    -h|--help)   usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 64 ;;
  esac
done

# ---- logging helpers ----
# Color only when stdout is a TTY. curl|bash pipe is NOT a TTY so users
# should see clean ASCII instead of raw \033 escape sequences.
if [[ -t 1 ]]; then
  _C_CYAN='\033[1;36m'; _C_YELLOW='\033[1;33m'; _C_RED='\033[1;31m'; _C_RESET='\033[0m'
else
  _C_CYAN=''; _C_YELLOW=''; _C_RED=''; _C_RESET=''
fi
log()  { printf '%b[hearth-install]%b %s\n' "$_C_CYAN"   "$_C_RESET" "$*"; }
warn() { printf '%b[hearth-install]%b %s\n' "$_C_YELLOW" "$_C_RESET" "$*" >&2; }
die()  { printf '%b[hearth-install]%b %s\n' "$_C_RED"    "$_C_RESET" "$*" >&2; exit 1; }

# ---- platform detection ----
detect_platform() {
  local os arch
  if [[ -n "${HEARTH_PLATFORM_OVERRIDE:-}" ]]; then
    local rest
    IFS='-' read -r os arch rest <<< "$HEARTH_PLATFORM_OVERRIDE"
    [[ -z "$rest" ]] || die "HEARTH_PLATFORM_OVERRIDE must be 'OS-ARCH' (got '$HEARTH_PLATFORM_OVERRIDE')"
  else
    os="$(uname -s)"
    arch="$(uname -m)"
  fi

  case "$os-$arch" in
    Darwin-arm64|Darwin-aarch64) echo "aarch64-apple-darwin" ;;
    Linux-x86_64)                echo "x86_64-unknown-linux-gnu" ;;
    *) die "unsupported platform $os/$arch; see https://github.com/$REPO_SLUG/blob/main/docs/install-ko.md" ;;
  esac
}

TARGET="$(detect_platform)"

# ---- dir resolution ----
resolve_bin_dir() {
  if [[ -n "$ARG_PREFIX" ]]; then echo "$ARG_PREFIX"; return; fi
  echo "${HEARTH_BIN_DIR:-$DEFAULT_BIN_DIR}"
}

resolve_skills_dirs() {
  if [[ -n "$ARG_SKILLS_DIR" ]]; then echo "$ARG_SKILLS_DIR"; return; fi
  if [[ -n "${HEARTH_SKILLS_DIR:-}" ]]; then echo "$HEARTH_SKILLS_DIR"; return; fi
  local out=()
  [[ -d "$HOME/.claude" ]] && out+=("$HOME/.claude/skills")
  [[ -d "$HOME/.codex" ]]  && out+=("$HOME/.codex/skills")
  if [[ ${#out[@]} -eq 0 ]]; then out+=("$HOME/.claude/skills"); fi
  printf '%s\n' "${out[@]}"
}

STAGING_DIR="${HEARTH_STAGING_DIR:-$DEFAULT_STAGING_DIR}"
BIN_DIR="$(resolve_bin_dir)"
# resolve_skills_dirs prints one per line; read into array.
SKILLS_DIRS=()
while IFS= read -r line; do SKILLS_DIRS+=("$line"); done < <(resolve_skills_dirs)

# ---- dry-run plan printer ----
print_plan() {
  local _v="${ARG_VERSION:-${HEARTH_VERSION:-<latest>}}"
  log "platform: $TARGET"
  log "version:  $_v"
  log "binary:   $BIN_DIR/hearth"
  log "staging:  $STAGING_DIR/skills-$_v/"
  for d in "${SKILLS_DIRS[@]}"; do log "skills:   $d"; done
  log "releases: $RELEASES_BASE"
}

if [[ "$DRY_RUN" -eq 1 ]]; then
  log "--- dry-run plan ($MODE) ---"
  print_plan
  log "(dry-run: no writes performed)"
  exit 0
fi

# ---- required tools ----
for tool in curl tar; do
  command -v "$tool" >/dev/null 2>&1 || die "required tool missing: $tool"
done

# shasum vs sha256sum portability
if command -v shasum >/dev/null 2>&1; then
  SHA_CHECK="shasum -a 256 -c"
else
  command -v sha256sum >/dev/null 2>&1 || die "neither shasum nor sha256sum is installed"
  SHA_CHECK="sha256sum -c"
fi

# ---- version resolution ----
resolve_version() {
  if [[ -n "$ARG_VERSION" ]]; then echo "$ARG_VERSION"; return; fi
  if [[ -n "${HEARTH_VERSION:-}" ]]; then echo "$HEARTH_VERSION"; return; fi
  # Query GitHub API for latest release (unauthenticated = 60 req/hr/IP).
  local json tag
  json="$(curl -fsSL --connect-timeout 15 --max-time 30 "$API_LATEST")" \
    || die "could not query GitHub API for latest release"
  # Whitespace-agnostic tag_name parse (tolerates pretty + compact JSON).
  tag="$(echo "$json" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)"
  [[ -n "$tag" ]] || die "could not parse tag_name from GitHub API response (first 120 chars: ${json:0:120})"
  echo "$tag"
}

VERSION="$(resolve_version)"
[[ "$VERSION" == v* ]] || VERSION="v$VERSION"   # tolerate both "0.8.0" and "v0.8.0"

# ---- download + verify ----
TMP="$(mktemp -d -t hearth-install.XXXXXX)"
trap 'rm -rf "$TMP"' EXIT

CLI_TARBALL="hearth-${VERSION}-${TARGET}.tar.gz"
SKILLS_TARBALL="hearth-skills-${VERSION}.tar.gz"
SUMS="SHA256SUMS"

log "Downloading $VERSION assets for $TARGET …"
for f in "$CLI_TARBALL" "$SKILLS_TARBALL" "$SUMS"; do
  url="$RELEASES_BASE/$VERSION/$f"
  log "  $url"
  curl -fsSL --connect-timeout 15 --max-time 120 -o "$TMP/$f" "$url" \
    || die "download failed: $url"
done

log "Verifying SHA256 …"
(cd "$TMP" && grep -E "  ($(printf '%s|%s' "$CLI_TARBALL" "$SKILLS_TARBALL"))$" "$SUMS" > SHA256SUMS.filtered) \
  || die "SHA256SUMS does not contain entries for $CLI_TARBALL or $SKILLS_TARBALL"
(cd "$TMP" && $SHA_CHECK SHA256SUMS.filtered >/dev/null) || die "SHA256 checksum verification failed"
log "  OK"

if [[ "$MODE" == "uninstall" ]]; then
  die "uninstall path not yet implemented (see Task 7)"
fi

die "install extraction not yet implemented (see Task 6)"
