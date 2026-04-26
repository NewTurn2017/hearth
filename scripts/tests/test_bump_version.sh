#!/usr/bin/env bash
# Regression harness for scripts/bump-version.sh.
# Copies the version manifests to a tmpdir scratch repo, runs
# bump-version.sh against it, asserts every manifest was updated, reverts
# cleanly. Exits 0 on all-pass.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BUMP="$REPO_ROOT/scripts/bump-version.sh"

fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "PASS: $*"; }

[[ -x "$BUMP" ]] || fail "bump-version.sh missing or not executable"

# 1. Usage error.
if "$BUMP" >/dev/null 2>&1; then
  fail "expected non-zero exit when called with no args"
fi
pass "usage error on no args"

# 2. Semver shape check.
if "$BUMP" "not-a-version" >/dev/null 2>&1; then
  fail "expected non-zero exit on bad semver"
fi
pass "rejects bad semver"

# 3. Happy path — isolate by copying only the version manifests into a
# tmpdir scratch. Bulk-copying the whole worktree would drag src-tauri/target
# (multi-GB build artifacts) for no gain.
TMP="$(mktemp -d -t hearth-bump-test.XXXXXX)"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/src-tauri/app" "$TMP/src-tauri/core" "$TMP/src-tauri/cli"
cp "$REPO_ROOT/package.json"                  "$TMP/"
cp "$REPO_ROOT/package-lock.json"             "$TMP/"
cp "$REPO_ROOT/src-tauri/app/tauri.conf.json" "$TMP/src-tauri/app/"
cp "$REPO_ROOT/src-tauri/app/Cargo.toml"      "$TMP/src-tauri/app/"
cp "$REPO_ROOT/src-tauri/core/Cargo.toml"     "$TMP/src-tauri/core/"
cp "$REPO_ROOT/src-tauri/cli/Cargo.toml"      "$TMP/src-tauri/cli/"

# Record starting versions for comparison.
BEFORE_PKG=$(jq -r .version "$TMP/package.json")
[[ -n "$BEFORE_PKG" && "$BEFORE_PKG" != "null" ]] || fail "package.json missing .version"

# Choose a target version distinct from the current one.
NEW="99.0.0-test"
[[ "$BEFORE_PKG" != "$NEW" ]] || fail "fixture collision: repo already at $NEW"

# Run bump in the copy.
(cd "$TMP" && "$BUMP" "$NEW" >/dev/null) || fail "bump script errored"

# Verify every manifest flipped.
for f in package.json package-lock.json src-tauri/app/tauri.conf.json; do
  got=$(jq -r .version "$TMP/$f")
  [[ "$got" == "$NEW" ]] || fail "$f: expected $NEW, got $got"
done
got=$(jq -r '.packages[""].version' "$TMP/package-lock.json")
[[ "$got" == "$NEW" ]] || fail "package-lock root package: expected $NEW, got $got"
for f in src-tauri/app/Cargo.toml src-tauri/core/Cargo.toml src-tauri/cli/Cargo.toml; do
  got=$(grep -m1 '^version' "$TMP/$f" | sed -E 's/.*"(.*)".*/\1/')
  [[ "$got" == "$NEW" ]] || fail "$f: expected $NEW, got $got"
done
pass "bump flipped all manifests"

echo
echo "ALL GOOD"
