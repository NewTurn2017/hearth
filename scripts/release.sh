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
  VER_TAURI="$(jq -r .version src-tauri/app/tauri.conf.json)"
  VER_CARGO="$(grep -m1 '^version' src-tauri/app/Cargo.toml | sed -E 's/.*"(.*)".*/\1/')"
  [[ "$VER_PKG" == "$VER_TAURI" && "$VER_PKG" == "$VER_CARGO" ]] \
    || die "version drift: package=$VER_PKG tauri=$VER_TAURI cargo=$VER_CARGO"
  VERSION="$VER_PKG"
  TAG="v$VERSION"
  export VERSION TAG

  # Tag must not exist upstream
  # In the Actions-first flow (release-cli.yml triggers on tag push), the
  # tag already exists by the time release.sh runs. Accept that as the
  # normal case — the publish step below will skip tag creation and upload
  # DMG/updater assets via `gh release upload --clobber`.
  if git ls-remote --tags origin "refs/tags/$TAG" | grep -q .; then
    log "tag $TAG already exists on origin (Actions-first flow)."
    TAG_EXISTS_ON_ORIGIN=1
  else
    TAG_EXISTS_ON_ORIGIN=0
  fi
  export TAG_EXISTS_ON_ORIGIN

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

  # Snapshot the produced bundle into dist/release/snapshot so that any
  # post-build cleanup (XProtect, Gatekeeper quarantine, accidental rebuild)
  # doesn't kill the pipeline. Before we need the originals (staple app,
  # re-tarball, sign), we check if they're still there and restore from the
  # snapshot if not. One-time cost; takes maybe ~150 MB and a few seconds.
  SNAPSHOT_DIR="$ROOT/dist/release/snapshot"
  rm -rf "$SNAPSHOT_DIR"
  mkdir -p "$SNAPSHOT_DIR"
  log "Snapshotting bundle artifacts → $SNAPSHOT_DIR"
  cp -R "$APP" "$SNAPSHOT_DIR/"
  cp    "$TARBALL" "$SNAPSHOT_DIR/"
  cp    "$SIG_FILE" "$SNAPSHOT_DIR/"
  cp    "$DMG" "$SNAPSHOT_DIR/"

  log "Built: $DMG"
}

# Restore an artifact from the snapshot if the current bundle path is gone.
# Tauri's post-build cleanup (or a spurious mac daemon) occasionally wipes
# bundle/macos/ between notarize and staple — we saw this on the first
# 0.3.0 attempt. Calling this before each file read makes staple/sign
# resilient without changing semantics.
restore_if_missing() {
  local src="$1" dst="$2"
  if [[ ! -e "$dst" ]]; then
    [[ -e "$src" ]] || die "snapshot missing: $src (cannot restore $dst)"
    log "Restoring $(basename "$dst") from snapshot"
    mkdir -p "$(dirname "$dst")"
    cp -R "$src" "$dst"
  fi
}

notarize_and_staple() {
  # Belt-and-suspenders: if the snapshot step preserved the DMG/app/tarball
  # and the bundle dir has since been cleared, restore them before notarize.
  restore_if_missing "$SNAPSHOT_DIR/$(basename "$DMG")" "$DMG"
  restore_if_missing "$SNAPSHOT_DIR/Hearth.app" "$APP"
  restore_if_missing "$SNAPSHOT_DIR/$(basename "$TARBALL")" "$TARBALL"
  restore_if_missing "$SNAPSHOT_DIR/$(basename "$SIG_FILE")" "$SIG_FILE"

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
  restore_if_missing "$SNAPSHOT_DIR/$(basename "$DMG")" "$DMG"
  xcrun stapler staple "$DMG"
  xcrun stapler validate "$DMG"

  log "stapler staple .app (updater tarball must carry the staple)…"
  restore_if_missing "$SNAPSHOT_DIR/Hearth.app" "$APP"
  xcrun stapler staple "$APP"
  xcrun stapler validate "$APP"

  log "Repacking Hearth.app.tar.gz with stapled .app…"
  # COPYFILE_DISABLE=1 keeps macOS `tar` from emitting AppleDouble (`._Hearth.app`)
  # metadata files alongside the bundle. Tauri's updater unpacks the archive with
  # a strict loop and aborts on those extra entries ("failed to unpack `._Hearth.app`"),
  # which is how 0.4.1 shipped broken. Keep this flag set here forever.
  (cd "$(dirname "$APP")" && COPYFILE_DISABLE=1 tar --no-xattrs -czf Hearth.app.tar.gz Hearth.app)

  log "Signing updater tarball with Tauri Ed25519 key…"
  # `tauri signer sign` (standalone CLI) treats the env var TAURI_SIGNING_PRIVATE_KEY
  # as *inline* key material and refuses to co-exist with --private-key-path. We set
  # that env in build_and_verify so `tauri build` picks the key up with its transparent
  # path fallback; unset it here so the standalone CLI accepts the explicit flag.
  env -u TAURI_SIGNING_PRIVATE_KEY \
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

  if [[ "${TAG_EXISTS_ON_ORIGIN:-0}" -eq 1 ]]; then
    log "tag $TAG already on origin (set by Actions-first flow); skipping tag create + push."
  else
    # Prefer a GPG-signed tag when the user has signing configured; fall back
    # to an annotated tag otherwise. v0.2.0 was tagged annotated; staying
    # consistent.
    if [[ -n "$(git config --get user.signingkey 2>/dev/null)" ]]; then
      log "Creating GPG-signed git tag $TAG…"
      git tag -s "$TAG" -m "Hearth $VERSION"
    else
      log "Creating annotated git tag $TAG (no GPG signingkey configured)…"
      git tag -a "$TAG" -m "Hearth $VERSION"
    fi
    git push origin "$TAG"
  fi

  # Actions (release-cli.yml) may have already created the release on tag
  # push. Create-or-upload so both pipelines are idempotent and either can
  # run first. --clobber: if release.sh is re-run for the same tag (rare,
  # e.g. a failed first attempt), replace previously-uploaded assets rather
  # than erroring. Note: upload --clobber has a sub-second window between
  # asset delete and re-upload where the updater's latest.json poll could
  # 404. Acceptable for our release cadence.
  local gh_view_exit=0
  gh release view "$TAG" --repo "$GH_REPO" >/dev/null 2>&1 || gh_view_exit=$?
  case "$gh_view_exit" in
    0)  release_exists=1 ;;
    1)  release_exists=0 ;;
    *)  die "gh release view exited $gh_view_exit (auth/network failure?)" ;;
  esac
  if [[ "$release_exists" -eq 1 ]]; then
    log "gh release already exists for $TAG; uploading DMG + updater with --clobber…"
    gh release upload "$TAG" \
      --repo "$GH_REPO" \
      --clobber \
      "$DMG" \
      "$TARBALL" \
      "$SIG_FILE" \
      "dist/release/latest.json"
    log "gh release edit (notes)…"
    gh release edit "$TAG" \
      --repo "$GH_REPO" \
      --title "Hearth $VERSION" \
      --notes-file dist/release/notes.md
  else
    log "gh release create…"
    gh release create "$TAG" \
      --repo "$GH_REPO" \
      --title "Hearth $VERSION" \
      --notes-file dist/release/notes.md \
      "$DMG" \
      "$TARBALL" \
      "$SIG_FILE" \
      "dist/release/latest.json"
  fi

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
