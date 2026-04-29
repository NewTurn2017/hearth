#!/usr/bin/env bash
# Build a signed Mac App Store .pkg for Hearth.
#
# Pipeline:
#   1. check-signing.sh   — fail fast if any cert/profile/API key is missing
#   2. sync-version.js    — package.json → tauri.conf + Cargo manifests
#   3. bump-build-number  — increment CFBundleVersion (build-number.json)
#   4. tauri build        — Tauri's first-pass codesign with Apple Distribution
#   5. patch CFBundleVersion + embed provisioning profile
#   6. re-sign .app with entitlements.mas.plist (provisioning profile baked in)
#   7. productbuild .pkg with 3rd Party Mac Developer Installer
#   8. altool --validate-app — Apple-side metadata sanity check
#
# Spec: docs/superpowers/specs/2026-04-26-mas-readiness-design.md §5
#
# Usage:
#   bash scripts/build-mas.sh
#
# Required env (also enforced by check-signing.sh):
#   APP_STORE_API_KEY_ID
#   APP_STORE_API_ISSUER_ID
#   API_PRIVATE_KEYS_DIR  (defaults to $HOME/.private_keys)

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# 1. Pre-flight ---------------------------------------------------------------
bash scripts/check-signing.sh

# 2. Version sync -------------------------------------------------------------
node scripts/sync-version.js

# 3. Bump CFBundleVersion -----------------------------------------------------
node scripts/bump-build-number.js
BUILD_NUMBER="$(node -p "require('./build-number.json').build")"
VERSION="$(node -p "require('./package.json').version")"
echo "==> Building Hearth ${VERSION} (build ${BUILD_NUMBER})"

# 4. Clean + build ------------------------------------------------------------
rm -rf src-tauri/target/aarch64-apple-darwin/release/bundle/macos
rm -rf dist-mas
mkdir -p dist-mas

# Tauri does the first pass codesign using `signingIdentity` from
# tauri.conf.json (Apple Distribution). We re-sign in step 6 once the
# provisioning profile is embedded.
npx tauri build --bundles app --target aarch64-apple-darwin

APP_PATH="src-tauri/target/aarch64-apple-darwin/release/bundle/macos/Hearth.app"
PKG_PATH="dist-mas/Hearth-${VERSION}-${BUILD_NUMBER}.pkg"
INFO_PLIST="$APP_PATH/Contents/Info.plist"

[[ -d "$APP_PATH" ]] || { echo "build-mas: tauri build did not produce $APP_PATH" >&2; exit 1; }

# 5. Patch CFBundleVersion + embed provisioning profile -----------------------
# Tauri uses .version for both CFBundleShortVersionString and CFBundleVersion;
# App Store needs CFBundleVersion strictly greater on every upload, so we
# stamp it from build-number.json here.
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion ${BUILD_NUMBER}" "$INFO_PLIST"

# Mark export-compliance: HTTPS-only, no custom crypto.
/usr/libexec/PlistBuddy -c "Add :ITSAppUsesNonExemptEncryption bool false" "$INFO_PLIST" 2>/dev/null \
  || /usr/libexec/PlistBuddy -c "Set :ITSAppUsesNonExemptEncryption false" "$INFO_PLIST"

PROFILE_SRC="${HEARTH_CERTS_DIR:-$ROOT/certs}/${HEARTH_MAS_PROFILE:-Hearth_MAS.provisionprofile}"
[[ -f "$PROFILE_SRC" ]] || { echo "build-mas: missing provisioning profile at $PROFILE_SRC" >&2; exit 1; }
cp "$PROFILE_SRC" "$APP_PATH/Contents/embedded.provisionprofile"

# 6. Re-sign with provisioning profile in place -------------------------------
# Apple TN3161 is explicit: do NOT use --deep when signing or verifying. Each
# nested binary is signed on its own; verification only inspects the top-level
# signature anyway.
codesign --force --options runtime \
  --entitlements src-tauri/app/entitlements.mas.plist \
  --sign "Apple Distribution: jaehyun jang (2UANJX7ATM)" \
  "$APP_PATH"

codesign --verify --strict --verbose=2 "$APP_PATH"

# 7. Produce signed installer .pkg --------------------------------------------
productbuild --component "$APP_PATH" /Applications \
  --sign "3rd Party Mac Developer Installer: jaehyun jang (2UANJX7ATM)" \
  "$PKG_PATH"

# 8. Apple-side validation ----------------------------------------------------
# TODO(D14 deadline = 2026-05-10): confirm xcrun altool MAS upload/validate is
#   still supported. altool is being phased out — notarytool covers Direct
#   builds, iTMSTransporter covers App Store/MAS. If altool starts erroring,
#   swap step 8 + scripts/upload-mas.sh to:
#     xcrun iTMSTransporter -m verify -assetFile "$PKG_PATH" \
#       -apiKey "$APP_STORE_API_KEY_ID" -apiIssuer "$APP_STORE_API_ISSUER_ID"
xcrun altool --validate-app -f "$PKG_PATH" -t macos \
  --apiKey "$APP_STORE_API_KEY_ID" \
  --apiIssuer "$APP_STORE_API_ISSUER_ID"

echo
echo "✅ Build OK: $PKG_PATH"
echo "   Version:      $VERSION"
echo "   Build number: $BUILD_NUMBER"
echo
echo "Next: bash scripts/upload-mas.sh"
