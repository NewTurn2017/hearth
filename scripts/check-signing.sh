#!/usr/bin/env bash
# scripts/check-signing.sh
#
# Pre-flight check for the Mac App Store build & submission pipeline.
# Verifies that every code-signing artifact required by the MAS track is
# present locally before kicking off a build. Run this before
# `scripts/build-mas.sh` (added in a later PR).
#
# Spec: docs/superpowers/specs/2026-04-26-mas-readiness-design.md
#
# Exit codes:
#   0 — all good
#   1 — at least one artifact missing or malformed
#
# Usage:
#   scripts/check-signing.sh

set -euo pipefail

PRIVATE_DIR="${HEARTH_PRIVATE_DIR:-$HOME/dev/private/apple_developer}"
CERTS_DIR="${HEARTH_CERTS_DIR:-$(cd "$(dirname "$0")/.." && pwd)/certs}"
PROFILE_NAME="${HEARTH_MAS_PROFILE:-Hearth_MAS.provisionprofile}"

# Identities we expect in the login keychain. Each entry is
# "policy|substring" — codesigning identities (Apple Distribution) live
# under `-p codesigning`, but installer-signing certs (3rd Party Mac
# Developer Installer, used by productsign for the .pkg) do NOT appear
# under the codesigning policy and must be queried with `-p basic`.
WANT_IDENTITIES=(
  "codesigning|Apple Distribution"
  "basic|3rd Party Mac Developer Installer"
)

fail_count=0

if [[ -t 1 ]]; then
  C_OK='\033[1;32m'; C_ERR='\033[1;31m'; C_DIM='\033[0;90m'; C_RST='\033[0m'
else
  C_OK=''; C_ERR=''; C_DIM=''; C_RST=''
fi
ok()   { printf '%b ✓ %s%b\n' "$C_OK"  "$*" "$C_RST"; }
err()  { printf '%b ✗ %s%b\n' "$C_ERR" "$*" "$C_RST"; fail_count=$((fail_count+1)); }
note() { printf '%b   %s%b\n' "$C_DIM" "$*" "$C_RST"; }

echo "Hearth MAS pre-flight"
echo "---------------------"

# 1. Keychain identities ----------------------------------------------------
identities_codesigning="$(security find-identity -v -p codesigning 2>/dev/null || true)"
identities_basic="$(security find-identity -v -p basic 2>/dev/null || true)"
for entry in "${WANT_IDENTITIES[@]}"; do
  policy="${entry%%|*}"
  want="${entry#*|}"
  case "$policy" in
    codesigning) pool="$identities_codesigning" ;;
    basic)       pool="$identities_basic" ;;
    *)           pool="" ;;
  esac
  if printf '%s\n' "$pool" | grep -Fq "$want"; then
    ok "keychain has identity matching: $want"
  else
    err "keychain MISSING identity: $want"
    note "issue from Apple Developer portal, then double-click .cer to install"
  fi
done

# 2. Provisioning profile ---------------------------------------------------
profile_path="$CERTS_DIR/$PROFILE_NAME"
if [[ -f "$profile_path" ]]; then
  ok "provisioning profile present: $profile_path"
  # Basic sanity: profile is a CMS-signed plist; security cms can decode.
  if security cms -D -i "$profile_path" >/dev/null 2>&1; then
    ok "provisioning profile is a valid CMS envelope"
  else
    err "provisioning profile failed CMS decode (corrupted?)"
  fi
else
  err "provisioning profile MISSING: $profile_path"
  note "set HEARTH_CERTS_DIR / HEARTH_MAS_PROFILE if it lives elsewhere"
fi

# 3. App Store Connect API key ---------------------------------------------
ac_key_id="${AC_API_KEY_ID:-Z2V325X3FY}"
ac_key="$PRIVATE_DIR/AuthKey_${ac_key_id}.p8"
if [[ -f "$ac_key" ]]; then
  ok "App Store Connect API key present: $ac_key"
else
  err "App Store Connect API key MISSING: $ac_key"
fi

# 4. Entitlements files -----------------------------------------------------
ents_mas="src-tauri/app/entitlements.mas.plist"
if [[ -f "$ents_mas" ]]; then
  ok "MAS entitlements file present: $ents_mas"
  if /usr/bin/plutil -lint "$ents_mas" >/dev/null 2>&1; then
    ok "MAS entitlements plist lints clean"
  else
    err "MAS entitlements plist failed plutil -lint"
  fi
else
  err "MAS entitlements file MISSING: $ents_mas"
fi

echo
if [[ "$fail_count" -eq 0 ]]; then
  printf '%bAll MAS signing artifacts ready.%b\n' "$C_OK" "$C_RST"
  exit 0
else
  printf '%b%d issue(s) found — resolve before running scripts/build-mas.sh.%b\n' "$C_ERR" "$fail_count" "$C_RST"
  exit 1
fi
