#!/usr/bin/env bash
# Harness for scripts/install.sh. Exits 0 on all-pass.
# Uses file:// URLs against a fixture tree so no network access is needed.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
INSTALL="$REPO_ROOT/scripts/install.sh"
FIXTURES="$SCRIPT_DIR/fixtures"

fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "PASS: $*"; }

[[ -x "$INSTALL" ]] || fail "install.sh missing or not executable"

# 1. Missing-flags help / usage. --help must exit 0 and mention "curl".
out=$("$INSTALL" --help 2>&1)
echo "$out" | grep -q 'curl -sSL' || fail "--help missing the curl one-liner: $out"
pass "--help shows the one-liner"

# 2. Unsupported-platform probe: force FreeBSD via env, expect exit 1.
set +e
out=$(HEARTH_PLATFORM_OVERRIDE="FreeBSD-amd64" "$INSTALL" --dry-run 2>&1)
code=$?
set -e
[[ "$code" != "0" ]] || fail "expected non-zero exit on unsupported platform"
echo "$out" | grep -qi 'unsupported' || fail "unsupported platform did not mention 'unsupported': $out"
pass "unsupported platform aborts"

# 3. Dry-run on macOS/Linux fixture: must print download URLs + targets
#    without touching filesystem.
BIN_DIR="$(mktemp -d -t hearth-install-bin.XXXXXX)"
SKILLS_DIR="$(mktemp -d -t hearth-install-skills.XXXXXX)"
STAGING_DIR="$(mktemp -d -t hearth-install-stage.XXXXXX)"
trap 'rm -rf "$BIN_DIR" "$SKILLS_DIR" "$STAGING_DIR"' EXIT

DRY=$(HEARTH_PLATFORM_OVERRIDE="Darwin-arm64" \
      HEARTH_RELEASES_URL="file://$FIXTURES/release" \
      HEARTH_VERSION="v0.0.0" \
      HEARTH_BIN_DIR="$BIN_DIR" \
      HEARTH_SKILLS_DIR="$SKILLS_DIR" \
      HEARTH_STAGING_DIR="$STAGING_DIR" \
      "$INSTALL" --dry-run 2>&1)
echo "$DRY" | grep -q 'aarch64-apple-darwin' || fail "dry-run missing target triple: $DRY"
echo "$DRY" | grep -q "$BIN_DIR" || fail "dry-run missing bin-dir: $DRY"
echo "$DRY" | grep -q "$SKILLS_DIR" || fail "dry-run missing skills-dir: $DRY"
# No writes.
[[ -z "$(ls "$BIN_DIR")" ]] || fail "dry-run wrote to bin dir"
[[ -z "$(ls "$SKILLS_DIR")" ]] || fail "dry-run wrote to skills dir"
[[ -z "$(ls "$STAGING_DIR")" ]] || fail "dry-run wrote to staging dir"
pass "dry-run prints plan, no writes"

echo
echo "ALL GOOD"
