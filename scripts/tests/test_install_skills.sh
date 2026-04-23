#!/usr/bin/env bash
# Harness for scripts/install-skills.sh. Exits 0 on all-pass.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
INSTALL="$REPO_ROOT/scripts/install-skills.sh"

fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "PASS: $*"; }

[[ -x "$INSTALL" ]] || fail "install-skills.sh missing or not executable"

TMP="$(mktemp -d -t hearth-install-test.XXXXXX)"
trap 'rm -rf "$TMP"' EXIT

# 1. Missing --into must error with exit 64.
if "$INSTALL" >/dev/null 2>&1; then
  fail "expected non-zero exit when --into omitted"
fi
code=$("$INSTALL" >/dev/null 2>&1; echo $?)
[[ "$code" == "64" ]] || fail "expected exit 64 when --into omitted, got $code"
pass "missing --into exits 64"

# 2. Happy path: install links each skills/* as symlink into target.
TARGET="$TMP/target"
"$INSTALL" --into "$TARGET" >"$TMP/install.log" 2>&1 \
  || { cat "$TMP/install.log" >&2; fail "install failed"; }

for skill in "$REPO_ROOT/skills"/*/; do
  name="$(basename "$skill")"
  link="$TARGET/$name"
  [[ -L "$link" ]] || fail "$link is not a symlink"
  resolved="$(readlink "$link")"
  expected="$(cd "$skill" && pwd)"
  [[ "$resolved" == "$expected" ]] || fail "$link -> $resolved (expected $expected)"
done
pass "symlinks created for all skills"

# 3. Re-running install is idempotent (replaces symlinks, no error).
"$INSTALL" --into "$TARGET" >/dev/null 2>&1 || fail "second install errored"
pass "idempotent re-install"

# 4. Refuses to overwrite a non-symlink at the target.
# Remove one symlink and put a real file there.
first_skill="$(find "$TARGET" -mindepth 1 -maxdepth 1 -type l | head -n1)"
rm "$first_skill"
echo "real file" > "$first_skill"
if "$INSTALL" --into "$TARGET" >/dev/null 2>&1; then
  fail "expected failure when non-symlink exists at target"
fi
pass "refuses to overwrite non-symlink"
# Cleanup for next test
rm "$first_skill"

# 5. --remove only removes symlinks this script would create.
# Add an unrelated symlink inside target; verify it survives --remove.
ln -s /tmp "$TARGET/unrelated-link"
"$INSTALL" --into "$TARGET" >/dev/null 2>&1 || fail "reinstall before remove"
"$INSTALL" --into "$TARGET" --remove >/dev/null 2>&1 || fail "--remove errored"
for skill in "$REPO_ROOT/skills"/*/; do
  name="$(basename "$skill")"
  [[ ! -e "$TARGET/$name" ]] || fail "$TARGET/$name still present after --remove"
done
[[ -L "$TARGET/unrelated-link" ]] || fail "--remove deleted an unrelated symlink"
pass "--remove surgical"

echo
echo "ALL GOOD"
