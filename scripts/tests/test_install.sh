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

# 4. SHA256 mismatch aborts before any install side effect.
BIN_DIR2="$(mktemp -d -t hearth-install-bin2.XXXXXX)"
SKILLS_DIR2="$(mktemp -d -t hearth-install-skills2.XXXXXX)"
STAGING_DIR2="$(mktemp -d -t hearth-install-stage2.XXXXXX)"
set +e
out=$(HEARTH_PLATFORM_OVERRIDE="Darwin-arm64" \
      HEARTH_RELEASES_URL="file://$FIXTURES/release-bad" \
      HEARTH_VERSION="v0.0.0" \
      HEARTH_BIN_DIR="$BIN_DIR2" \
      HEARTH_SKILLS_DIR="$SKILLS_DIR2" \
      HEARTH_STAGING_DIR="$STAGING_DIR2" \
      "$INSTALL" 2>&1)
code=$?
set -e
[[ "$code" != "0" ]] || { echo "$out"; fail "expected non-zero exit on SHA mismatch"; }
echo "$out" | grep -qi 'sha\|checksum' || fail "sha-mismatch error did not mention sha/checksum: $out"
[[ -z "$(ls "$BIN_DIR2")" ]] || fail "sha mismatch wrote to bin dir"
rm -rf "$BIN_DIR2" "$SKILLS_DIR2" "$STAGING_DIR2"
pass "sha mismatch aborts before install"

# 5. Happy-path install: binary lands, each skill symlink resolves into staging.
BIN_DIR3="$(mktemp -d -t hearth-install-bin3.XXXXXX)"
SKILLS_DIR3="$(mktemp -d -t hearth-install-skills3.XXXXXX)"
STAGING_DIR3="$(mktemp -d -t hearth-install-stage3.XXXXXX)"
HEARTH_PLATFORM_OVERRIDE="Darwin-arm64" \
  HEARTH_RELEASES_URL="file://$FIXTURES/release" \
  HEARTH_VERSION="v0.0.0" \
  HEARTH_BIN_DIR="$BIN_DIR3" \
  HEARTH_SKILLS_DIR="$SKILLS_DIR3" \
  HEARTH_STAGING_DIR="$STAGING_DIR3" \
  "$INSTALL" >/dev/null 2>&1 || fail "happy-path install errored"

[[ -x "$BIN_DIR3/hearth" ]] || fail "binary not installed at $BIN_DIR3/hearth"
hearth_out=$("$BIN_DIR3/hearth" db path)
echo "$hearth_out" | grep -q '"ok":true' || fail "installed binary did not run: $hearth_out"
for skill in hearth-today-brief hearth-project-scan hearth-memo-organize; do
  link="$SKILLS_DIR3/$skill"
  [[ -L "$link" ]] || fail "$link not a symlink"
  resolved="$(readlink "$link")"
  [[ "$resolved" == "$STAGING_DIR3/skills-v0.0.0/$skill" ]] \
    || fail "$link -> $resolved (expected staging v0.0.0)"
  [[ -f "$resolved/SKILL.md" ]] || fail "resolved skill dir missing SKILL.md: $resolved"
done
pass "happy-path install landed binary + 3 skill symlinks"

# 6. Re-running install is idempotent.
HEARTH_PLATFORM_OVERRIDE="Darwin-arm64" \
  HEARTH_RELEASES_URL="file://$FIXTURES/release" \
  HEARTH_VERSION="v0.0.0" \
  HEARTH_BIN_DIR="$BIN_DIR3" \
  HEARTH_SKILLS_DIR="$SKILLS_DIR3" \
  HEARTH_STAGING_DIR="$STAGING_DIR3" \
  "$INSTALL" >/dev/null 2>&1 || fail "second install errored"
pass "idempotent re-install"

# 7. --uninstall is surgical: removes binary + our symlinks, preserves others.
# Install into a fresh scratch, add an unrelated symlink, then uninstall.
BIN_DIR4="$(mktemp -d -t hearth-uninstall-bin.XXXXXX)"
SKILLS_DIR4="$(mktemp -d -t hearth-uninstall-skills.XXXXXX)"
STAGING_DIR4="$(mktemp -d -t hearth-uninstall-stage.XXXXXX)"
HEARTH_PLATFORM_OVERRIDE="Darwin-arm64" \
  HEARTH_RELEASES_URL="file://$FIXTURES/release" \
  HEARTH_VERSION="v0.0.0" \
  HEARTH_BIN_DIR="$BIN_DIR4" \
  HEARTH_SKILLS_DIR="$SKILLS_DIR4" \
  HEARTH_STAGING_DIR="$STAGING_DIR4" \
  "$INSTALL" >/dev/null 2>&1 || fail "uninstall-setup install errored"
ln -s /tmp "$SKILLS_DIR4/unrelated-link"
echo "plain file" > "$SKILLS_DIR4/unrelated-file"

HEARTH_PLATFORM_OVERRIDE="Darwin-arm64" \
  HEARTH_RELEASES_URL="file://$FIXTURES/release" \
  HEARTH_VERSION="v0.0.0" \
  HEARTH_BIN_DIR="$BIN_DIR4" \
  HEARTH_SKILLS_DIR="$SKILLS_DIR4" \
  HEARTH_STAGING_DIR="$STAGING_DIR4" \
  "$INSTALL" --uninstall >/dev/null 2>&1 || fail "--uninstall errored"

[[ ! -e "$BIN_DIR4/hearth" ]] || fail "--uninstall did not remove binary"
for skill in hearth-today-brief hearth-project-scan hearth-memo-organize; do
  [[ ! -e "$SKILLS_DIR4/$skill" ]] || fail "--uninstall left skill symlink: $skill"
done
[[ -L "$SKILLS_DIR4/unrelated-link" ]] || fail "--uninstall removed unrelated symlink"
[[ -f "$SKILLS_DIR4/unrelated-file" ]] || fail "--uninstall removed unrelated file"
# Staging dir preserved (rollback-friendly).
[[ -d "$STAGING_DIR4/skills-v0.0.0" ]] || fail "--uninstall wiped staging"
pass "--uninstall surgical"

rm -rf "$BIN_DIR3" "$SKILLS_DIR3" "$STAGING_DIR3" "$BIN_DIR4" "$SKILLS_DIR4" "$STAGING_DIR4"

echo
echo "ALL GOOD"
