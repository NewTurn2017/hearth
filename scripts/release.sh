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

  # Clean git + on main (dry-run may run from any branch so engineers can
  # verify the pipeline end-to-end before merging the feature branch).
  [[ -z "$(git status --porcelain)" ]] || die "git working tree not clean."
  BRANCH="$(git rev-parse --abbrev-ref HEAD)"
  if [[ "$DRY_RUN" -eq 0 && "$BRANCH" != "main" ]]; then
    die "not on main (current: $BRANCH). Re-run with --dry-run to validate the pipeline from any branch."
  fi

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
           TAURI_SIGNING_PRIVATE_KEY_PATH TAURI_SIGNING_PRIVATE_KEY_PASSWORD GH_REPO; do
    [[ -n "${!v:-}" ]] || die ".env.release missing $v"
  done
  [[ -f "$APPLE_API_KEY_PATH" ]] || die "APPLE_API_KEY_PATH not a file: $APPLE_API_KEY_PATH"
  [[ -f "$TAURI_SIGNING_PRIVATE_KEY_PATH" ]] \
    || die "TAURI_SIGNING_PRIVATE_KEY_PATH not a file: $TAURI_SIGNING_PRIVATE_KEY_PATH"

  # Tooling
  command -v gh >/dev/null         || die "gh CLI not installed."
  gh auth status >/dev/null        || die "gh not authenticated."
  command -v jq >/dev/null         || die "jq not installed."
  xcrun --find notarytool >/dev/null || die "xcrun notarytool missing; install Xcode CLT."
  xcrun --find stapler    >/dev/null || die "xcrun stapler missing; install Xcode CLT."
  security find-identity -v -p codesigning | grep -q "Developer ID Application: jaehyun jang (2UANJX7ATM)" \
    || die "Developer ID signing identity not in keychain."
  # v0.2.0 ships aarch64 (Apple Silicon) only. MLX is Apple Silicon exclusive
  # anyway; Intel support is a follow-up if demand appears.
  command -v rustc >/dev/null || die "rustc not installed."
  HOST_TRIPLE="$(rustc -vV | awk '/^host:/ {print $2}')"
  [[ "$HOST_TRIPLE" == "aarch64-apple-darwin" ]] \
    || die "host rustc triple must be aarch64-apple-darwin (got: $HOST_TRIPLE)"

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
  # `tauri build` reads TAURI_SIGNING_PRIVATE_KEY (with a transparent path
  # fallback — if the value is a valid filesystem path it's treated as one).
  # Our .env.release stores the path under TAURI_SIGNING_PRIVATE_KEY_PATH for
  # consistency with `tauri signer sign --private-key-path`. Bridge the two.
  export TAURI_SIGNING_PRIVATE_KEY="$TAURI_SIGNING_PRIVATE_KEY_PATH"

  log "npm ci…"
  npm ci --silent

  log "cargo fetch…"
  (cd src-tauri && cargo fetch --quiet)

  log "tauri build (aarch64-apple-darwin)…"
  npm run tauri -- build --target aarch64-apple-darwin

  DMG_DIR="src-tauri/target/aarch64-apple-darwin/release/bundle/dmg"
  MACOS_DIR="src-tauri/target/aarch64-apple-darwin/release/bundle/macos"
  DMG="$(ls "$DMG_DIR"/Hearth_*_aarch64.dmg 2>/dev/null | head -1)"
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

notarize_and_staple() {
  log "notarytool submit (may take 2–10 min)…"
  SUBMIT_JSON="$(xcrun notarytool submit "$DMG" \
    --key "$APPLE_API_KEY_PATH" \
    --key-id "$APPLE_API_KEY_ID" \
    --issuer "$APPLE_API_ISSUER" \
    --wait \
    --output-format json)"
  echo "$SUBMIT_JSON" | jq .
  SUBMIT_ID="$(echo "$SUBMIT_JSON" | jq -r .id)"
  STATUS="$(echo "$SUBMIT_JSON" | jq -r .status)"
  if [[ "$STATUS" != "Accepted" ]]; then
    log "notarization failed ($STATUS). Fetching log…"
    xcrun notarytool log "$SUBMIT_ID" \
      --key "$APPLE_API_KEY_PATH" \
      --key-id "$APPLE_API_KEY_ID" \
      --issuer "$APPLE_API_ISSUER" || true
    die "notarization not Accepted."
  fi
  export SUBMIT_ID

  log "stapler staple DMG…"
  xcrun stapler staple "$DMG"
  xcrun stapler validate "$DMG"

  log "stapler staple .app (updater tarball must carry the staple)…"
  xcrun stapler staple "$APP"
  xcrun stapler validate "$APP"

  log "Repacking Hearth.app.tar.gz with stapled .app…"
  (cd "$(dirname "$APP")" && tar -czf Hearth.app.tar.gz Hearth.app)

  log "Signing updater tarball with Tauri Ed25519 key…"
  # `tauri signer sign` (standalone CLI) treats the env var TAURI_SIGNING_PRIVATE_KEY
  # as *inline* key material — only `tauri build` auto-detects paths. Use the explicit
  # --private-key-path flag to avoid silent failures when the env var holds a filesystem
  # path (the common and documented convention in .env.release.example).
  npx tauri signer sign \
    --private-key-path "$TAURI_SIGNING_PRIVATE_KEY_PATH" \
    --password "$TAURI_SIGNING_PRIVATE_KEY_PASSWORD" \
    "$TARBALL" > /dev/null

  # tauri signer writes <file>.sig next to the input; capture the content.
  SIGNATURE="$(cat "$SIG_FILE")"
  [[ -n "$SIGNATURE" ]] || die "updater signature empty."
  export SIGNATURE
  log "Updater signature ready (len=${#SIGNATURE})."
}

write_manifest_and_notes() {
  log "Writing dist/release/latest.json…"
  "$ROOT/scripts/generate-manifest.sh" "$VERSION" "$SIGNATURE"

  log "Extracting release notes…"
  mkdir -p dist/release
  "$ROOT/scripts/extract-release-notes.sh" "$VERSION" > dist/release/notes.md

  # Append user-facing install/update footer.
  cat >> dist/release/notes.md <<'EOF'

---

**설치 (macOS)**

`Hearth_*_aarch64.dmg` 를 받아서 Applications 로 드래그하세요. 첫 실행만 우클릭 → "열기" 로 Gatekeeper 를 1회 통과하면 됩니다 (공증된 빌드라 "알 수 없는 개발자" 경고는 없습니다). v0.2.0 은 Apple Silicon 전용입니다.

**업데이트**

앱이 켜져 있으면 자동으로 새 버전을 확인합니다. 업데이트 토스트에서 "지금 재시작" 을 누르면 2–3초 내에 새 버전으로 교체됩니다.
EOF
}

publish_and_verify() {
  if [[ "$DRY_RUN" -eq 1 ]]; then
    log "dry-run: skipping git tag + gh release create."
    log "artifacts prepared:"
    ls -lh "$DMG" "$TARBALL" "$SIG_FILE" dist/release/latest.json dist/release/notes.md
    return 0
  fi

  log "Creating signed git tag $TAG…"
  git tag -s "$TAG" -m "Hearth $VERSION"
  git push origin "$TAG"

  log "gh release create…"
  gh release create "$TAG" \
    --repo "$GH_REPO" \
    --title "Hearth $VERSION" \
    --notes-file dist/release/notes.md \
    "$DMG" \
    "$TARBALL" \
    "$SIG_FILE" \
    "dist/release/latest.json"

  log "Post-verify: latest.json version round-trip…"
  REMOTE_VER="$(curl -sL "https://github.com/$GH_REPO/releases/latest/download/latest.json" | jq -r .version)"
  [[ "$REMOTE_VER" == "$VERSION" ]] \
    || die "permalink mismatch: expected $VERSION got $REMOTE_VER"

  log "Release published:"
  log "  https://github.com/$GH_REPO/releases/tag/$TAG"
}

main() {
  preflight
  build_and_verify
  notarize_and_staple
  write_manifest_and_notes
  publish_and_verify

  log "Done."
  log "  version:    $VERSION"
  log "  tag:        $TAG"
  log "  dmg:        $DMG"
  log "  notarize:   ${SUBMIT_ID:-(n/a)}"
  if [[ "$DRY_RUN" -eq 0 ]]; then
    log "  release:    https://github.com/$GH_REPO/releases/tag/$TAG"
    log "  manifest:   https://github.com/$GH_REPO/releases/latest/download/latest.json"
  fi
}

main "$@"
