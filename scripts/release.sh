#!/usr/bin/env bash
# End-to-end macOS release driver for Hearth.
# See docs/superpowers/specs/2026-04-18-tauri-release-design.md for design.
#
# Usage:
#   ./scripts/release.sh               # full release
#   ./scripts/release.sh --dry-run     # stop before tag push + gh release create
#   ./scripts/release.sh --skip-tests  # skip preflight test runs
#   ./scripts/release.sh --verbose     # set -x
#
# Environment comes from .env.release.
set -euo pipefail

DRY_RUN=0
SKIP_TESTS=0
for arg in "$@"; do
  case "$arg" in
    --dry-run)     DRY_RUN=1 ;;
    --skip-tests)  SKIP_TESTS=1 ;;
    --verbose)     set -x ;;
    *) echo "Unknown flag: $arg" >&2; exit 64 ;;
  esac
done

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ ! -f .env.release ]]; then
  echo "Missing .env.release (copy .env.release.example and fill in)." >&2
  exit 66
fi
# shellcheck disable=SC1091
set -a; . ./.env.release; set +a

log() { printf '\033[1;36m[release]\033[0m %s\n' "$*"; }
die() { printf '\033[1;31m[release]\033[0m %s\n' "$*" >&2; exit 1; }

preflight() {
  log "Preflight…"

  # Clean git + on main
  [[ -z "$(git status --porcelain)" ]] || die "git working tree not clean."
  BRANCH="$(git rev-parse --abbrev-ref HEAD)"
  [[ "$BRANCH" == "main" ]] || die "not on main (current: $BRANCH)."

  # Version sync across 3 manifests
  VER_PKG="$(jq -r .version package.json)"
  VER_TAURI="$(jq -r .version src-tauri/tauri.conf.json)"
  VER_CARGO="$(grep -m1 '^version' src-tauri/Cargo.toml | sed -E 's/.*"(.*)".*/\1/')"
  [[ "$VER_PKG" == "$VER_TAURI" && "$VER_PKG" == "$VER_CARGO" ]] \
    || die "version drift: package=$VER_PKG tauri=$VER_TAURI cargo=$VER_CARGO"
  VERSION="$VER_PKG"
  TAG="v$VERSION"
  export VERSION TAG

  # Tag must not exist upstream
  if git ls-remote --tags origin "refs/tags/$TAG" | grep -q .; then
    die "tag $TAG already exists on origin."
  fi

  # CHANGELOG section exists
  grep -q "^## \[$VERSION\]" CHANGELOG.md \
    || die "CHANGELOG.md has no '## [$VERSION]' section."

  # .env.release fields
  for v in APPLE_API_KEY_PATH APPLE_API_KEY_ID APPLE_API_ISSUER \
           TAURI_SIGNING_PRIVATE_KEY TAURI_SIGNING_PRIVATE_KEY_PASSWORD GH_REPO; do
    [[ -n "${!v:-}" ]] || die ".env.release missing $v"
  done
  [[ -f "$APPLE_API_KEY_PATH" ]] || die "APPLE_API_KEY_PATH not a file: $APPLE_API_KEY_PATH"
  [[ -f "$TAURI_SIGNING_PRIVATE_KEY" ]] \
    || die "TAURI_SIGNING_PRIVATE_KEY not a file: $TAURI_SIGNING_PRIVATE_KEY"

  # Tooling
  command -v gh >/dev/null         || die "gh CLI not installed."
  gh auth status >/dev/null        || die "gh not authenticated."
  command -v jq >/dev/null         || die "jq not installed."
  xcrun --find notarytool >/dev/null || die "xcrun notarytool missing; install Xcode CLT."
  xcrun --find stapler    >/dev/null || die "xcrun stapler missing; install Xcode CLT."
  security find-identity -v -p codesigning | grep -q "Developer ID Application: jaehyun jang (2UANJX7ATM)" \
    || die "Developer ID signing identity not in keychain."
  rustup target list --installed | grep -q aarch64-apple-darwin \
    || die "rustup target aarch64-apple-darwin not installed."
  rustup target list --installed | grep -q x86_64-apple-darwin \
    || die "rustup target x86_64-apple-darwin not installed."

  # Tests
  if [[ "$SKIP_TESTS" -eq 0 ]]; then
    log "cargo test…"
    (cd src-tauri && cargo test --quiet)
    log "npm test…"
    npm test --silent
  else
    log "skipping tests (--skip-tests)"
  fi

  log "Preflight OK — version=$VERSION tag=$TAG"
}

build_and_verify() {
  log "npm ci…"
  npm ci --silent

  log "cargo fetch…"
  (cd src-tauri && cargo fetch --quiet)

  log "tauri build (universal-apple-darwin)…"
  npm run tauri -- build --target universal-apple-darwin

  DMG_DIR="src-tauri/target/universal-apple-darwin/release/bundle/dmg"
  MACOS_DIR="src-tauri/target/universal-apple-darwin/release/bundle/macos"
  DMG="$(ls "$DMG_DIR"/Hearth_*_universal.dmg 2>/dev/null | head -1)"
  APP="$MACOS_DIR/Hearth.app"
  TARBALL="$MACOS_DIR/Hearth.app.tar.gz"
  SIG_FILE="$MACOS_DIR/Hearth.app.tar.gz.sig"
  [[ -f "$DMG" ]]     || die "DMG not produced: $DMG_DIR"
  [[ -d "$APP" ]]     || die "Hearth.app not produced: $APP"
  [[ -f "$TARBALL" ]] || die "updater tarball not produced: $TARBALL"
  [[ -f "$SIG_FILE" ]] || die "updater signature not produced: $SIG_FILE"
  export DMG APP TARBALL SIG_FILE

  log "codesign --verify…"
  codesign --verify --deep --strict --verbose=2 "$APP" \
    || die "codesign verify failed on $APP"

  log "Built: $DMG"
}

main() {
  preflight
  build_and_verify
  log "(notarize/staple/publish stages added in subsequent tasks)"
  if [[ "$DRY_RUN" -eq 1 ]]; then
    log "dry-run: stopping before notarize."
  fi
}

main "$@"
