# Hearth Auto-Deploy v1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a `curl -sSL .../install.sh | bash` one-liner that installs the `hearth` CLI binary + 3 v1 skills on macOS aarch64 and Linux x86_64, backed by a GitHub Actions matrix build triggered by `vX.Y.Z` tag pushes.

**Architecture:** Tag-triggered Actions workflow cross-builds the CLI, bundles `skills/` into a tarball, computes `SHA256SUMS`, and uploads all four files to the matching GitHub Release (via `gh release upload --clobber` for idempotency with the existing Tauri `release.sh`). A pure-bash `install.sh` detects platform, fetches the release assets, verifies SHA256, installs the binary to `~/.local/bin/hearth`, stages skills at `~/.local/share/hearth/skills-<version>/`, and symlinks each skill into `~/.claude/skills` and/or `~/.codex/skills` (auto-detected).

**Tech Stack:** GitHub Actions (`actions/checkout`, `dtolnay/rust-toolchain`, `Swatinem/rust-cache`, `actions/upload-artifact`, `actions/download-artifact`), bash 3.2+, `curl`, `shasum` / `sha256sum`, existing `hearth` CLI, existing `scripts/extract-release-notes.sh`. No new Rust code, no SKILL.md changes.

**Spec:** `docs/superpowers/specs/2026-04-23-hearth-auto-deploy-design.md` (commit `1228002`).

---

## Pre-flight audit findings (cross-checked against current repo)

- `scripts/bump-version.sh` is **stale post-workspace-split**. It touches `src-tauri/tauri.conf.json` (no longer exists; now at `src-tauri/app/tauri.conf.json`) and mutates `src-tauri/Cargo.toml` `[package]` block (workspace root now has `[workspace]` / `[workspace.package]` instead). Must be rewritten to touch 5 files: `package.json`, `src-tauri/app/tauri.conf.json`, `src-tauri/app/Cargo.toml`, `src-tauri/core/Cargo.toml`, `src-tauri/cli/Cargo.toml`.
- `scripts/release.sh` (line 274) calls `gh release create "$TAG"` unconditionally. When GitHub Actions creates the release first on tag push, this fails. Switch to create-or-upload pattern with `--clobber`.
- `.github/` directory does not exist yet — the workflow file creates it.
- `scripts/extract-release-notes.sh` exists and works against `## [X.Y.Z]` CHANGELOG sections. The workflow will reuse it.
- Existing release assets (Tauri DMG + updater) must continue to attach cleanly; they live in `scripts/release.sh`'s final step.

---

## File Structure

```
.github/workflows/
└── release-cli.yml                   # NEW — matrix build + bundle + publish
scripts/
├── install.sh                        # NEW — one-liner target
├── tests/
│   └── test_install.sh               # NEW — fixture-based TDD harness
│   └── test_bump_version.sh          # NEW — regression for post-split bump
├── bump-version.sh                   # REWRITTEN — workspace-aware
└── release.sh                        # MODIFIED — gh release upload --clobber
docs/
└── install-ko.md                     # NEW — Korean install guide
README.md                             # MODIFIED — Installation section overhaul
CHANGELOG.md                          # MODIFIED — [0.8.0] entry
```

No Rust code. No SKILL.md. All changes are packaging + CI + docs.

Current branch: `claude/hearth-auto-deploy` (branched off `main` at commit `bce8989`). Working directory: `/Users/genie/dev/tools/hearth`.

---

## Task 1: Fix `scripts/bump-version.sh` (TDD)

`bump-version.sh` is broken post-workspace-split. Fix it and add a regression test so the release pipeline has a reliable version bumper.

**Files:**
- Create: `scripts/tests/test_bump_version.sh`
- Rewrite: `scripts/bump-version.sh`

- [ ] **Step 1: Write the failing test**

Create `scripts/tests/test_bump_version.sh`:

```bash
#!/usr/bin/env bash
# Regression harness for scripts/bump-version.sh.
# Copies the five version manifests to a tmpdir scratch repo, runs
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

# 3. Happy path — isolate by running bump in a temporary worktree copy.
TMP="$(mktemp -d -t hearth-bump-test.XXXXXX)"
trap 'rm -rf "$TMP"' EXIT
cp -R "$REPO_ROOT"/. "$TMP"/
# Drop the .git directory — the test only needs the files, not git state,
# and we don't want to mutate the source repo.
rm -rf "$TMP/.git"

# Record starting versions for comparison.
BEFORE_PKG=$(jq -r .version "$TMP/package.json")
[[ -n "$BEFORE_PKG" && "$BEFORE_PKG" != "null" ]] || fail "package.json missing .version"

# Choose a target version distinct from the current one.
NEW="99.0.0-test"
[[ "$BEFORE_PKG" != "$NEW" ]] || fail "fixture collision: repo already at $NEW"

# Run bump in the copy.
(cd "$TMP" && "$BUMP" "$NEW" >/dev/null) || fail "bump script errored"

# Verify every manifest flipped.
for f in package.json src-tauri/app/tauri.conf.json; do
  got=$(jq -r .version "$TMP/$f")
  [[ "$got" == "$NEW" ]] || fail "$f: expected $NEW, got $got"
done
for f in src-tauri/app/Cargo.toml src-tauri/core/Cargo.toml src-tauri/cli/Cargo.toml; do
  got=$(grep -m1 '^version' "$TMP/$f" | sed -E 's/.*"(.*)".*/\1/')
  [[ "$got" == "$NEW" ]] || fail "$f: expected $NEW, got $got"
done
pass "bump flipped all 5 manifests"

echo
echo "ALL GOOD"
```

Make it executable:

```bash
chmod +x scripts/tests/test_bump_version.sh
```

- [ ] **Step 2: Run the test to confirm current bump-version.sh fails**

Run:

```bash
./scripts/tests/test_bump_version.sh
```

Expected: FAIL in the "bump flipped all 5 manifests" assertion (current script doesn't touch `src-tauri/app/tauri.conf.json`, `src-tauri/app/Cargo.toml`, `src-tauri/core/Cargo.toml`, `src-tauri/cli/Cargo.toml`). The exact first FAIL line will be one of the five per-file checks.

- [ ] **Step 3: Rewrite the bump script**

Replace the entirety of `scripts/bump-version.sh` with:

```bash
#!/usr/bin/env bash
# Bump the Hearth version in every manifest atomically.
# Handles the post-workspace-split layout: 1 npm manifest + 1 Tauri config
# + 3 Cargo crate manifests.
#
# Usage: ./scripts/bump-version.sh 0.8.0
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <semver>" >&2
  exit 64
fi

NEW="$1"

if ! [[ "$NEW" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.-]+)?$ ]]; then
  echo "Error: '$NEW' is not a plausible semver string." >&2
  exit 65
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# --- JSON manifests ---

for f in package.json src-tauri/app/tauri.conf.json; do
  [[ -f "$f" ]] || { echo "missing: $f" >&2; exit 66; }
  tmp="$(mktemp)"
  jq --arg v "$NEW" '.version = $v' "$f" > "$tmp"
  mv "$tmp" "$f"
done

# --- Cargo manifests (each crate has a [package] block with version) ---

bump_cargo() {
  local path="$1"
  [[ -f "$path" ]] || { echo "missing: $path" >&2; exit 66; }
  python3 - "$path" "$NEW" <<'PY'
import pathlib, re, sys
path = pathlib.Path(sys.argv[1])
new = sys.argv[2]
text = path.read_text()
out, count = re.subn(
    r'(\[package\][\s\S]*?\nversion\s*=\s*")([^"]+)(")',
    lambda m: m.group(1) + new + m.group(3),
    text,
    count=1,
)
if count == 0:
    sys.exit(f"Failed to locate [package] version in {path}")
path.write_text(out)
PY
}

for crate in src-tauri/app/Cargo.toml src-tauri/core/Cargo.toml src-tauri/cli/Cargo.toml; do
  bump_cargo "$crate"
done

echo "Bumped to $NEW in:"
echo "  package.json"
echo "  src-tauri/app/tauri.conf.json"
echo "  src-tauri/app/Cargo.toml"
echo "  src-tauri/core/Cargo.toml"
echo "  src-tauri/cli/Cargo.toml"
```

- [ ] **Step 4: Run the test to verify it passes**

Run:

```bash
./scripts/tests/test_bump_version.sh
```

Expected: 3 `PASS:` lines + `ALL GOOD`, exit 0.

- [ ] **Step 5: Commit**

```bash
git add scripts/bump-version.sh scripts/tests/test_bump_version.sh
git commit -m "fix(scripts): bump-version.sh touches all 5 post-split manifests + TDD harness"
```

---

## Task 2: Harden `scripts/release.sh` against Actions race

Actions will create the GitHub Release on tag push. `release.sh` (running from the developer's Mac) needs to handle the "release already exists" case cleanly.

**Files:**
- Modify: `scripts/release.sh:273-283` (the `gh release create` block)

- [ ] **Step 1: Read the current block**

```bash
sed -n '265,290p' scripts/release.sh
```

Expected: shows the tag-create + `gh release create "$TAG" ... --notes-file ... $DMG $TARBALL $SIG_FILE dist/release/latest.json` block near line 274.

- [ ] **Step 2: Replace the block**

Use Edit with the exact old_string below and the new_string below.

old_string (lines 274-282):

```
  log "gh release create…"
  gh release create "$TAG" \
    --repo "$GH_REPO" \
    --title "Hearth $VERSION" \
    --notes-file dist/release/notes.md \
    "$DMG" \
    "$TARBALL" \
    "$SIG_FILE" \
    "dist/release/latest.json"
```

new_string:

```
  # Actions (release-cli.yml) may have already created the release on tag
  # push. Create-or-upload so both pipelines are idempotent and either can
  # run first. --clobber replaces same-named assets on re-runs.
  if gh release view "$TAG" --repo "$GH_REPO" >/dev/null 2>&1; then
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
```

- [ ] **Step 3: Confirm the diff**

Run:

```bash
git diff scripts/release.sh
```

Expected: only the block above is modified; no other lines touched.

- [ ] **Step 4: Commit**

```bash
git add scripts/release.sh
git commit -m "fix(release): release.sh idempotent against Actions-created release (upload --clobber)"
```

---

## Task 3: GitHub Actions — `release-cli.yml`

Matrix build of the `hearth` CLI on tag push, plus a skills-bundle job, plus a publish job that uploads to the Release.

**Files:**
- Create: `.github/workflows/release-cli.yml`

- [ ] **Step 1: Create the workflow file**

Write this content to `.github/workflows/release-cli.yml`:

```yaml
name: release-cli

on:
  push:
    tags:
      - 'v*'

permissions:
  contents: write

jobs:
  build-cli:
    name: build ${{ matrix.target }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - { os: macos-14,     target: aarch64-apple-darwin,       strip: strip }
          - { os: ubuntu-22.04, target: x86_64-unknown-linux-gnu,   strip: strip }
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri -> src-tauri/target
          key: ${{ matrix.target }}

      - name: Build hearth-cli
        working-directory: src-tauri
        run: cargo build --release --target ${{ matrix.target }} -p hearth-cli

      - name: Strip binary
        run: ${{ matrix.strip }} src-tauri/target/${{ matrix.target }}/release/hearth

      - name: Tar
        run: |
          set -euo pipefail
          VERSION="${GITHUB_REF_NAME}"
          TARBALL="hearth-${VERSION}-${{ matrix.target }}.tar.gz"
          tar czf "$TARBALL" -C src-tauri/target/${{ matrix.target }}/release hearth
          echo "TARBALL=$TARBALL" >> "$GITHUB_ENV"

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: hearth-${{ matrix.target }}
          path: ${{ env.TARBALL }}
          if-no-files-found: error

  bundle-skills:
    name: bundle skills
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v4

      - name: Tar skills
        run: |
          set -euo pipefail
          VERSION="${GITHUB_REF_NAME}"
          TARBALL="hearth-skills-${VERSION}.tar.gz"
          tar czf "$TARBALL" skills/
          echo "TARBALL=$TARBALL" >> "$GITHUB_ENV"

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: skills-bundle
          path: ${{ env.TARBALL }}
          if-no-files-found: error

  publish:
    name: publish release
    needs: [build-cli, bundle-skills]
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v4

      - name: Download artifacts
        uses: actions/download-artifact@v4
        with:
          path: dist

      - name: Flatten + compute SHA256SUMS
        working-directory: dist
        run: |
          set -euo pipefail
          # Artifacts land in subdirs named after the upload; flatten them.
          mv hearth-*/hearth-*.tar.gz .
          mv skills-bundle/hearth-skills-*.tar.gz .
          rmdir hearth-aarch64-apple-darwin hearth-x86_64-unknown-linux-gnu skills-bundle
          ls -la
          sha256sum *.tar.gz > SHA256SUMS
          cat SHA256SUMS

      - name: Extract release notes
        run: |
          set -euo pipefail
          VERSION="${GITHUB_REF_NAME#v}"
          ./scripts/extract-release-notes.sh "$VERSION" > RELEASE_NOTES.md
          echo "---"
          cat RELEASE_NOTES.md

      - name: Create or update release
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          set -euo pipefail
          TAG="${GITHUB_REF_NAME}"
          VERSION="${TAG#v}"
          cd dist
          if gh release view "$TAG" >/dev/null 2>&1; then
            echo "Release $TAG exists; uploading CLI+skills assets with --clobber."
            gh release upload "$TAG" --clobber *.tar.gz SHA256SUMS
            gh release edit "$TAG" --notes-file ../RELEASE_NOTES.md --title "Hearth $VERSION"
          else
            echo "Creating release $TAG."
            gh release create "$TAG" \
              --title "Hearth $VERSION" \
              --notes-file ../RELEASE_NOTES.md \
              *.tar.gz SHA256SUMS
          fi
```

- [ ] **Step 2: Lint locally (optional but recommended)**

If `actionlint` is on PATH:

```bash
actionlint .github/workflows/release-cli.yml
```

Expected: no errors. If `actionlint` is not installed, skip this step — the first real tag push is the real validation.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release-cli.yml
git commit -m "feat(ci): release-cli.yml — tag-triggered CLI matrix build + skills bundle"
```

---

## Task 4: `scripts/install.sh` skeleton + tests + platform detection (TDD)

Set up the test fixture infrastructure and the install-script skeleton. Platform detection, flag parsing, and `--help` land here. Download and install come in Task 5.

**Files:**
- Create: `scripts/tests/test_install.sh`
- Create: `scripts/tests/fixtures/` (fixture release assets used by the harness)
- Create: `scripts/install.sh`

- [ ] **Step 1: Write the failing test harness**

Create `scripts/tests/test_install.sh`:

```bash
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
echo "$out" | grep -q 'curl' || fail "--help missing the curl one-liner: $out"
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
pass "dry-run prints plan, no writes"

echo
echo "ALL GOOD"
```

Make it executable:

```bash
chmod +x scripts/tests/test_install.sh
```

- [ ] **Step 2: Build the fixture tree**

Create `scripts/tests/fixtures/release/` with three tarballs and a `SHA256SUMS`. The fake `hearth` binary is a shell stub that emits `{"ok":true,"data":"/tmp/fake"}` on `db path`:

```bash
mkdir -p scripts/tests/fixtures/release

# Build a fake hearth binary
FAKE_BIN="$(mktemp -d)/hearth"
cat > "$FAKE_BIN" <<'EOF'
#!/usr/bin/env bash
case "$*" in
  "db path") echo '{"ok":true,"data":"/tmp/fake-hearth.db"}' ;;
  *) echo "{\"ok\":true,\"data\":null}" ;;
esac
EOF
chmod +x "$FAKE_BIN"

# Tar the fake binary for each triple
for triple in aarch64-apple-darwin x86_64-unknown-linux-gnu; do
  tar czf "scripts/tests/fixtures/release/hearth-v0.0.0-$triple.tar.gz" \
    -C "$(dirname "$FAKE_BIN")" hearth
done

# Bundle real skills tree for the skills tarball
tar czf scripts/tests/fixtures/release/hearth-skills-v0.0.0.tar.gz skills/

# Compute SHA256SUMS
(cd scripts/tests/fixtures/release && shasum -a 256 *.tar.gz > SHA256SUMS)

ls -la scripts/tests/fixtures/release/
```

Expected: four files in `scripts/tests/fixtures/release/` — 2 `hearth-v0.0.0-*.tar.gz`, 1 `hearth-skills-v0.0.0.tar.gz`, 1 `SHA256SUMS`.

Remove the tmp fake binary:

```bash
rm -rf "$(dirname "$FAKE_BIN")"
```

- [ ] **Step 3: Run the test to verify it fails**

Run:

```bash
./scripts/tests/test_install.sh
```

Expected: FAIL with `install.sh missing or not executable`.

- [ ] **Step 4: Write the install.sh skeleton**

Create `scripts/install.sh`:

```bash
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
      [[ $# -ge 2 ]] || { echo "--version needs a value" >&2; exit 64; }
      ARG_VERSION="$2"; shift 2 ;;
    --prefix)
      [[ $# -ge 2 ]] || { echo "--prefix needs a value" >&2; exit 64; }
      ARG_PREFIX="$2"; shift 2 ;;
    --skills-dir)
      [[ $# -ge 2 ]] || { echo "--skills-dir needs a value" >&2; exit 64; }
      ARG_SKILLS_DIR="$2"; shift 2 ;;
    --uninstall) MODE="uninstall"; shift ;;
    --dry-run)   DRY_RUN=1; shift ;;
    -h|--help)   usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 64 ;;
  esac
done

# ---- logging helpers ----
log()  { printf '\033[1;36m[hearth-install]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[hearth-install]\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31m[hearth-install]\033[0m %s\n' "$*" >&2; exit 1; }

# ---- platform detection ----
detect_platform() {
  local os arch
  if [[ -n "${HEARTH_PLATFORM_OVERRIDE:-}" ]]; then
    IFS='-' read -r os arch <<< "$HEARTH_PLATFORM_OVERRIDE"
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
  log "platform: $TARGET"
  log "version:  ${ARG_VERSION:-${HEARTH_VERSION:-<latest>}}"
  log "binary:   $BIN_DIR/hearth"
  log "staging:  $STAGING_DIR/skills-<version>/"
  for d in "${SKILLS_DIRS[@]}"; do log "skills:   $d"; done
  log "releases: $RELEASES_BASE"
}

if [[ "$DRY_RUN" -eq 1 ]]; then
  log "--- dry-run plan ($MODE) ---"
  print_plan
  log "(dry-run: no writes performed)"
  exit 0
fi

die "install/uninstall path not yet implemented (see Task 5+)"
```

Make it executable:

```bash
chmod +x scripts/install.sh
```

- [ ] **Step 5: Run the test to verify it passes**

Run:

```bash
./scripts/tests/test_install.sh
```

Expected: 3 `PASS:` lines + `ALL GOOD`, exit 0.

- [ ] **Step 6: Commit**

```bash
git add scripts/install.sh scripts/tests/test_install.sh scripts/tests/fixtures/
git commit -m "feat(scripts): install.sh skeleton (platform detect + --help + --dry-run + fixture test harness)"
```

---

## Task 5: install.sh — version resolution + download + SHA256 verify

Replace the `die "install/uninstall path not yet implemented"` stub with the download path: resolve version, fetch tarballs, verify checksums. Stop before extraction (Task 6 handles that).

**Files:**
- Modify: `scripts/install.sh`
- Modify: `scripts/tests/test_install.sh` (add assertion 4: SHA mismatch aborts)

- [ ] **Step 1: Add the SHA-mismatch fixture**

Create a tampered fixture release:

```bash
mkdir -p scripts/tests/fixtures/release-bad
cp scripts/tests/fixtures/release/*.tar.gz scripts/tests/fixtures/release-bad/
# Write a deliberately wrong SHA256SUMS
cat > scripts/tests/fixtures/release-bad/SHA256SUMS <<'EOF'
0000000000000000000000000000000000000000000000000000000000000000  hearth-v0.0.0-aarch64-apple-darwin.tar.gz
0000000000000000000000000000000000000000000000000000000000000000  hearth-v0.0.0-x86_64-unknown-linux-gnu.tar.gz
0000000000000000000000000000000000000000000000000000000000000000  hearth-skills-v0.0.0.tar.gz
EOF
ls scripts/tests/fixtures/release-bad/
```

- [ ] **Step 2: Extend the test harness**

Insert the following block in `scripts/tests/test_install.sh` right before the closing `echo` + `ALL GOOD` lines:

old_string:

```
pass "dry-run prints plan, no writes"

echo
echo "ALL GOOD"
```

new_string:

```
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

echo
echo "ALL GOOD"
```

- [ ] **Step 3: Run the test to verify the new assertion fails**

Run:

```bash
./scripts/tests/test_install.sh
```

Expected: earlier assertions still pass. The new "sha mismatch aborts before install" assertion FAILS because the skeleton's final line is `die "install/uninstall path not yet implemented..."` — which does error, but not specifically with a SHA message.

The FAIL line should be either "sha-mismatch error did not mention sha/checksum" (the die message doesn't contain "sha") or the install-path still being a placeholder. Either way the test run exit is non-zero.

- [ ] **Step 4: Extend install.sh with version resolution + download + verify**

Edit `scripts/install.sh`. Replace the final line:

old_string:

```
die "install/uninstall path not yet implemented (see Task 5+)"
```

new_string:

```
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
  json="$(curl -fsSL "$API_LATEST")" || die "could not query GitHub API for latest release"
  tag="$(echo "$json" | sed -n 's/^  "tag_name": "\(.*\)",*$/\1/p' | head -1)"
  [[ -n "$tag" ]] || die "could not parse tag_name from GitHub API"
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
  curl -fsSL -o "$TMP/$f" "$url" || die "download failed: $url"
done

log "Verifying SHA256 …"
(cd "$TMP" && grep -E "  ($(printf '%s|%s' "$CLI_TARBALL" "$SKILLS_TARBALL"))$" "$SUMS" > SHA256SUMS.filtered)
(cd "$TMP" && $SHA_CHECK SHA256SUMS.filtered >/dev/null) || die "SHA256 checksum verification failed"
log "  OK"

if [[ "$MODE" == "uninstall" ]]; then
  die "uninstall path not yet implemented (see Task 7)"
fi

die "install extraction not yet implemented (see Task 6)"
```

- [ ] **Step 5: Run the test to confirm the SHA assertion passes**

Run:

```bash
./scripts/tests/test_install.sh
```

Expected: 4 `PASS:` lines + `ALL GOOD`, exit 0.

(The good-path install still terminates with "install extraction not yet implemented", but test assertion 4 only triggers on the bad-fixtures path and that now hits the SHA failure correctly.)

- [ ] **Step 6: Commit**

```bash
git add scripts/install.sh scripts/tests/test_install.sh scripts/tests/fixtures/release-bad/
git commit -m "feat(install): version resolution + download + SHA256 verify"
```

---

## Task 6: install.sh — extract + install binary + symlink skills (happy path)

Complete the install mode: extract the binary, create the versioned staging dir, symlink each skill into every target dir.

**Files:**
- Modify: `scripts/install.sh`
- Modify: `scripts/tests/test_install.sh` (add assertion 5: happy-path install lands correctly)

- [ ] **Step 1: Extend the test harness with the happy-path assertion**

In `scripts/tests/test_install.sh`, insert this block right before `echo` + `ALL GOOD`:

old_string:

```
pass "sha mismatch aborts before install"

echo
echo "ALL GOOD"
```

new_string:

```
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

rm -rf "$BIN_DIR3" "$SKILLS_DIR3" "$STAGING_DIR3"

echo
echo "ALL GOOD"
```

- [ ] **Step 2: Run the test to verify the new assertions fail**

Run:

```bash
./scripts/tests/test_install.sh
```

Expected: assertions 1–4 pass; assertion 5 FAILs with "happy-path install errored" (current script still dies on "install extraction not yet implemented").

- [ ] **Step 3: Implement the install path**

Edit `scripts/install.sh`. Replace the final block:

old_string:

```
if [[ "$MODE" == "uninstall" ]]; then
  die "uninstall path not yet implemented (see Task 7)"
fi

die "install extraction not yet implemented (see Task 6)"
```

new_string:

```
# ---- mode dispatch ----
install_mode() {
  # 1. Install binary.
  mkdir -p "$BIN_DIR"
  tar xzf "$TMP/$CLI_TARBALL" -C "$TMP"
  mv "$TMP/hearth" "$BIN_DIR/hearth"
  chmod +x "$BIN_DIR/hearth"
  log "Installed binary: $BIN_DIR/hearth"

  # 2. Extract skills into versioned staging dir.
  local stage="$STAGING_DIR/skills-$VERSION"
  mkdir -p "$stage"
  # The tarball contains skills/<name>/SKILL.md at its root. We want each
  # <name> directly under stage so paths are ~/.local/share/hearth/skills-vX/<name>.
  local tmp_extract="$TMP/skills-extract"
  mkdir -p "$tmp_extract"
  tar xzf "$TMP/$SKILLS_TARBALL" -C "$tmp_extract"
  # tmp_extract now contains skills/. Move each <name> into stage (refresh if exists).
  local entry
  for entry in "$tmp_extract"/skills/*/; do
    [[ -d "$entry" ]] || continue
    local name; name="$(basename "$entry")"
    rm -rf "$stage/$name"
    mv "$entry" "$stage/$name"
  done
  rm -rf "$tmp_extract"
  log "Staged skills: $stage"

  # 3. Symlink each skill into every resolved skills dir.
  local dir name link
  for dir in "${SKILLS_DIRS[@]}"; do
    mkdir -p "$dir"
    for name in $(ls "$stage"); do
      [[ -d "$stage/$name" ]] || continue
      link="$dir/$name"
      if [[ -e "$link" && ! -L "$link" ]]; then
        warn "refusing to overwrite non-symlink: $link"
        continue
      fi
      ln -snf "$stage/$name" "$link"
      log "  linked $link -> $stage/$name"
    done
  done

  # 4. Post-install sanity.
  if "$BIN_DIR/hearth" db path >/dev/null 2>&1; then
    log "✓ hearth $VERSION installed at $BIN_DIR/hearth"
  else
    warn "hearth installed but 'db path' invocation failed — check permissions / PATH"
  fi

  log ""
  log "Next:"
  log "  * Add $BIN_DIR to PATH (echo 'export PATH=\"$BIN_DIR:\$PATH\"' >> ~/.zshrc)"
  log "  * macOS Gatekeeper first-run: xattr -d com.apple.quarantine $BIN_DIR/hearth"
  log "  * Docs: https://github.com/$REPO_SLUG/blob/main/docs/install-ko.md"
}

uninstall_mode() {
  die "uninstall path not yet implemented (see Task 7)"
}

case "$MODE" in
  install)   install_mode ;;
  uninstall) uninstall_mode ;;
  *) die "unknown mode: $MODE" ;;
esac
```

- [ ] **Step 4: Run the test to verify happy-path + idempotent**

Run:

```bash
./scripts/tests/test_install.sh
```

Expected: 6 `PASS:` lines + `ALL GOOD`, exit 0.

- [ ] **Step 5: Commit**

```bash
git add scripts/install.sh scripts/tests/test_install.sh
git commit -m "feat(install): extract binary + stage skills + symlink into agent host dirs"
```

---

## Task 7: install.sh — `--uninstall` surgical cleanup

Symmetric to install: remove the binary and only the symlinks this script's install mode would create, preserving unrelated files.

**Files:**
- Modify: `scripts/install.sh`
- Modify: `scripts/tests/test_install.sh` (add assertion 7: `--uninstall` is surgical)

- [ ] **Step 1: Extend the test harness**

Insert right before `echo` + `ALL GOOD` in `scripts/tests/test_install.sh`:

old_string:

```
pass "idempotent re-install"

rm -rf "$BIN_DIR3" "$SKILLS_DIR3" "$STAGING_DIR3"

echo
echo "ALL GOOD"
```

new_string:

```
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
```

- [ ] **Step 2: Run the test to verify the new assertion fails**

Run:

```bash
./scripts/tests/test_install.sh
```

Expected: assertions 1–6 pass; `--uninstall surgical` FAILs because the stub still dies on "uninstall path not yet implemented".

- [ ] **Step 3: Implement uninstall_mode**

Edit `scripts/install.sh`. Replace the `uninstall_mode` body:

old_string:

```
uninstall_mode() {
  die "uninstall path not yet implemented (see Task 7)"
}
```

new_string:

```
uninstall_mode() {
  local removed=0 link target dir name stage

  # 1. Remove the binary if it exists.
  if [[ -e "$BIN_DIR/hearth" ]]; then
    rm -f "$BIN_DIR/hearth"
    log "Removed binary: $BIN_DIR/hearth"
    removed=$((removed+1))
  fi

  # 2. Remove only symlinks pointing into our staging tree.
  for dir in "${SKILLS_DIRS[@]}"; do
    [[ -d "$dir" ]] || continue
    for link in "$dir"/*; do
      [[ -L "$link" ]] || continue
      target="$(readlink "$link")"
      # Match links whose target resolves inside STAGING_DIR.
      case "$target" in
        "$STAGING_DIR"/skills-v*/*)
          rm -f "$link"
          log "Removed symlink: $link"
          removed=$((removed+1))
          ;;
      esac
    done
  done

  if [[ "$removed" -eq 0 ]]; then
    log "No hearth artifacts found to remove."
  else
    log "✓ Uninstalled $removed entr$([[ "$removed" -eq 1 ]] && echo y || echo ies)."
    log "  Staging preserved at $STAGING_DIR (rm -rf to fully purge)."
  fi
}
```

- [ ] **Step 4: Run the test to verify --uninstall**

Run:

```bash
./scripts/tests/test_install.sh
```

Expected: 7 `PASS:` lines + `ALL GOOD`, exit 0.

- [ ] **Step 5: Commit**

```bash
git add scripts/install.sh scripts/tests/test_install.sh
git commit -m "feat(install): --uninstall removes binary + our symlinks surgically"
```

---

## Task 8: `docs/install-ko.md` — Korean install guide

**Files:**
- Create: `docs/install-ko.md`

- [ ] **Step 1: Write the Korean guide**

Create `docs/install-ko.md` with this exact content:

````markdown
# Hearth 설치 가이드 (한글)

에이전트 (Claude Code · Codex 등) 에서 `hearth` CLI + v1 스킬을 한 줄로 설치하는 방법.

## 한 줄 설치

```bash
curl -sSL https://raw.githubusercontent.com/NewTurn2017/hearth/main/scripts/install.sh | bash
```

이 명령이 하는 일:
1. 플랫폼 감지 (`Darwin-arm64` → macOS Apple Silicon, `Linux-x86_64` → 리눅스).
2. 최신 GitHub Release 버전을 조회.
3. `hearth` 바이너리를 `~/.local/bin/hearth` 로 설치.
4. 스킬 3종 (`hearth-today-brief`, `hearth-project-scan`, `hearth-memo-organize`) 을 `~/.local/share/hearth/skills-<version>/` 로 압축 해제.
5. 감지된 에이전트 호스트 디렉토리 (`~/.claude/skills`, `~/.codex/skills`) 로 심링크 생성.
6. `hearth db path` 로 동작 확인.

## 환경 변수

| 변수 | 기본값 | 용도 |
|---|---|---|
| `HEARTH_VERSION` | `<latest>` | 특정 태그 고정 (예: `v0.8.0`) |
| `HEARTH_BIN_DIR` | `~/.local/bin` | 바이너리 설치 경로 |
| `HEARTH_SKILLS_DIR` | 자동 감지 | 스킬 심링크 경로. 하나만 지원; 여러 곳 필요하면 스크립트 여러 번 실행 |
| `HEARTH_STAGING_DIR` | `~/.local/share/hearth` | 스킬 버전별 staging 경로 |

예시:

```bash
HEARTH_VERSION=v0.8.0 HEARTH_BIN_DIR=~/bin \
  curl -sSL https://raw.githubusercontent.com/NewTurn2017/hearth/main/scripts/install.sh | bash
```

## 플래그

`bash -s -- <flag>` 형식으로 전달:

- `--version X.Y.Z` — 특정 버전 설치
- `--prefix DIR` — 바이너리 설치 경로
- `--skills-dir DIR` — 스킬 심링크 경로
- `--uninstall` — 제거
- `--dry-run` — 실제 쓰기 없이 계획만 출력

예시:

```bash
curl -sSL ... | bash -s -- --version v0.8.0 --dry-run
```

## PATH 추가

`~/.local/bin` 이 `$PATH` 에 없으면 설치는 됐지만 `hearth` 명령이 안 먹음:

```bash
# zsh
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc && source ~/.zshrc

# bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc && source ~/.bashrc
```

확인:

```bash
which hearth
hearth db path
```

## macOS Gatekeeper (첫 실행)

CLI 는 notarize 되지 않습니다 (의도된 선택 — GUI 가 아니어서 Gatekeeper 가 curl|bash 경로에 안 맞음). 첫 실행 시 "cannot be opened" 에러가 나오면:

```bash
xattr -d com.apple.quarantine ~/.local/bin/hearth
```

다시 실행:

```bash
hearth db path
```

한 번만 하면 됩니다.

## 에이전트 호스트별 동작

설치 스크립트는 다음을 자동 감지합니다:

- `~/.claude` 존재 → `~/.claude/skills/` 로 심링크
- `~/.codex` 존재 → `~/.codex/skills/` 로 심링크
- 둘 다 있으면 둘 다 심링크
- 둘 다 없으면 `~/.claude/skills/` 로 기본

수동 지정:

```bash
HEARTH_SKILLS_DIR=~/.codex/skills \
  curl -sSL https://raw.githubusercontent.com/NewTurn2017/hearth/main/scripts/install.sh | bash
```

## 업그레이드

같은 한 줄 설치 명령을 다시 실행하세요. 바이너리와 심링크가 최신으로 교체됩니다. 이전 버전의 스킬은 `~/.local/share/hearth/skills-<old>/` 에 그대로 남아있습니다 (롤백용).

## 삭제

```bash
curl -sSL https://raw.githubusercontent.com/NewTurn2017/hearth/main/scripts/install.sh | bash -s -- --uninstall
```

스크립트가 한 일만 되돌립니다 — 바이너리와 우리가 만든 심링크만 제거. staging 디렉토리는 보존됩니다. 완전 삭제:

```bash
rm -rf ~/.local/share/hearth
```

## 문제 해결

### `hearth: command not found`

PATH 에 `~/.local/bin` 이 없습니다. 위 "PATH 추가" 섹션 참고.

### `cannot be opened because the developer cannot be verified`

macOS Gatekeeper. 위 "macOS Gatekeeper" 섹션 참고.

### `SHA256 checksum verification failed`

다운로드가 중간에 끊겼거나 CDN 캐시 문제. 재실행해 보세요. 계속되면 issue 로 제보.

### `unsupported platform`

현재 macOS aarch64 (Apple Silicon) 와 Linux x86_64 만 지원합니다. 다른 플랫폼은 소스에서 빌드:

```bash
git clone https://github.com/NewTurn2017/hearth.git
cd hearth/src-tauri && cargo build --release -p hearth-cli
```

그리고 `target/release/hearth` 를 PATH 에 둡니다.

### `jq: command not found` 등 도구 부재

`curl` 과 `tar`, `shasum` (또는 `sha256sum`) 만 있으면 됩니다. 리눅스 최소 환경:

```bash
apt-get update && apt-get install -y curl tar ca-certificates
```

## 관련 문서

- CLI 사용법: [`docs/hearth-cli-ko.md`](./hearth-cli-ko.md)
- 스킬 개요: [`skills/README.md`](../skills/README.md)
- 디자인 스펙: [`docs/superpowers/specs/2026-04-23-hearth-auto-deploy-design.md`](./superpowers/specs/2026-04-23-hearth-auto-deploy-design.md)
````

- [ ] **Step 2: Verify the file**

Run:

```bash
wc -l docs/install-ko.md
head -5 docs/install-ko.md
```

Expected: non-zero line count (~130 lines), first heading `# Hearth 설치 가이드 (한글)`.

- [ ] **Step 3: Commit**

```bash
git add docs/install-ko.md
git commit -m "docs: Korean install guide (install-ko.md) for the one-liner flow"
```

---

## Task 9: README.md — Installation section overhaul

Rewrite the top-level README so the one-liner is the primary install path, with DMG and build-from-source as alternatives.

**Files:**
- Modify: `README.md` (Installation section)

- [ ] **Step 1: Inspect the current Installation section**

Run:

```bash
rg -n '^## Installation|^### macOS \(공식 릴리즈\)|^### Windows / Linux|^### 업데이트|^### 시스템 요구사항|^## Usage' README.md
```

Expected: output shows the Installation section runs from `## Installation` (around line 46 on `main`) to just before `## Usage` (around line 74). Use the bounding line numbers from this output to guide the edit.

- [ ] **Step 2: Replace the Installation section**

Use Edit with:

old_string (exact current block — confirm via the above `rg` + `sed -n 'START,ENDp' README.md` before editing):

```
## Installation

### macOS (공식 릴리즈)
```

new_string:

```
## Installation

### 한 줄 설치 (CLI + Skills, macOS aarch64 · Linux x86_64)

```bash
curl -sSL https://raw.githubusercontent.com/NewTurn2017/hearth/main/scripts/install.sh | bash
```

`hearth` 바이너리가 `~/.local/bin/hearth` 에, 스킬 3종이 감지된 에이전트 호스트 디렉토리 (`~/.claude/skills`, `~/.codex/skills`) 에 심링크됩니다. 자세한 내용·환경변수·삭제 방법은 [`docs/install-ko.md`](docs/install-ko.md) 참고.

### macOS (공식 릴리즈)
```

**Note:** this edit only prepends a new `### 한 줄 설치` subsection at the top of the Installation section. It does NOT remove any existing content (macOS 공식 릴리즈 / Windows · Linux / 업데이트 / 시스템 요구사항 stay intact).

- [ ] **Step 3: Verify**

Run:

```bash
rg -n '^### 한 줄 설치' README.md
rg -n '^### macOS \(공식 릴리즈\)' README.md
rg -n '^## Usage' README.md
```

Expected: one match each; the `### 한 줄 설치` line comes before `### macOS (공식 릴리즈)` which comes before `## Usage`.

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs(readme): add 한 줄 설치 subsection (CLI + Skills one-liner)"
```

---

## Task 10: CHANGELOG.md — `[0.8.0]` entry

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Inspect the current CHANGELOG head**

Run:

```bash
head -30 CHANGELOG.md
```

Expected: current top section is `## [Unreleased]` followed by `## [0.7.0]` with CLI + Skills + theme work.

- [ ] **Step 2: Insert a new `[0.8.0]` section**

Use Edit with:

old_string:

```
## [Unreleased]

## [0.7.0]
```

new_string:

```
## [Unreleased]

## [0.8.0] - (unreleased)

### Added
- **한 줄 설치 (`scripts/install.sh`)**: `curl -sSL .../install.sh | bash` 로 `hearth` CLI + v1 skills 설치. macOS aarch64 + Linux x86_64. 환경변수 `HEARTH_VERSION`/`HEARTH_BIN_DIR`/`HEARTH_SKILLS_DIR` 로 override. `--uninstall` / `--dry-run` / `--version` / `--prefix` / `--skills-dir` 플래그 지원.
- **GitHub Actions release pipeline (`.github/workflows/release-cli.yml`)**: `vX.Y.Z` 태그 push 시 macOS aarch64 + Linux x86_64 바이너리와 version-pinned skills tarball, `SHA256SUMS` 를 자동 빌드해 Release 에 업로드.
- **한글 설치 가이드 (`docs/install-ko.md`)**: 한 줄 설치 · 환경변수 · PATH · Gatekeeper · 업그레이드 · 삭제 · 문제 해결.

### Fixed
- **`scripts/bump-version.sh`**: 워크스페이스 분할 이후 깨진 상태 복구. 이제 `package.json` + `src-tauri/app/tauri.conf.json` + 3개 crate `Cargo.toml` 전부 bump. TDD 회귀 테스트 (`scripts/tests/test_bump_version.sh`) 포함.
- **`scripts/release.sh`**: Actions 가 먼저 Release 를 만든 경우에도 실패하지 않게 `gh release upload --clobber` 로 하드닝.

## [0.7.0]
```

- [ ] **Step 3: Verify**

Run:

```bash
rg -n '^## \[0\.8\.0\]|^## \[0\.7\.0\]|^## \[Unreleased\]' CHANGELOG.md | head -5
```

Expected: `Unreleased` then `0.8.0` then `0.7.0` in that order.

- [ ] **Step 4: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs(changelog): [0.8.0] — one-liner installer + release-cli Actions"
```

---

## Task 11: Final E2E verification (no new commits)

**Files:** none (read-only verification).

- [ ] **Step 1: Run both test harnesses**

```bash
./scripts/tests/test_bump_version.sh
./scripts/tests/test_install.sh
```

Expected: both print `ALL GOOD`, exit 0.

- [ ] **Step 2: Smoke the release.sh diff**

```bash
grep -A 2 'gh release view' scripts/release.sh | head -10
```

Expected: shows the new `gh release view "$TAG"` conditional block from Task 2.

- [ ] **Step 3: Validate the workflow YAML**

If `actionlint` is installed:

```bash
actionlint .github/workflows/release-cli.yml
```

Expected: no errors. If not installed:

```bash
python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/release-cli.yml')); print('yaml ok')"
```

Expected: `yaml ok`.

- [ ] **Step 4: Inspect the branch**

```bash
git log --oneline main..HEAD
git status
```

Expected: ~10 commits on `claude/hearth-auto-deploy` ahead of `main`, working tree clean (or only pre-existing untracked build artifacts under `src-tauri/app/gen/` / `src-tauri/target/` — gitignored).

- [ ] **Step 5: Manual CI dry-run (after PR review, before calling v1 done)**

This step is **outside the plan's task scope** but is the last required gate before marking the spec's deliverables checklist complete. The developer:

1. Merges this branch to `main` (PR flow).
2. Pushes a throwaway pre-release tag: `git tag v0.0.0-rc.1 && git push --tags`.
3. Watches [the Actions run](https://github.com/NewTurn2017/hearth/actions) → verifies the release body + assets + SHA256SUMS.
4. Deletes the dry-run release + tag:
   ```bash
   gh release delete v0.0.0-rc.1 --yes
   git push --delete origin v0.0.0-rc.1
   git tag -d v0.0.0-rc.1
   ```
5. Smoke-installs from the real release on a fresh macOS shell and an Ubuntu 22.04 container:
   ```bash
   # macOS
   HEARTH_VERSION=v0.0.0-rc.1 curl -sSL .../install.sh | bash
   # Ubuntu
   docker run --rm -it ubuntu:22.04 bash -c \
     'apt-get update && apt-get install -y curl tar ca-certificates && \
      curl -sSL .../install.sh | bash && hearth today'
   ```

---

## Appendix: what this plan does NOT do

Per spec §12, these are deferred sub-projects:

1. Homebrew tap (`brew install newturn2017/tap/hearth`) + Formula.rb auto-generation.
2. macOS x86_64 + Linux aarch64 + Windows matrix expansion.
3. Custom `hearth.sh/install` short URL.
4. `hearth --self-update` CLI subcommand.
5. `hearth skills list|install|uninstall` CLI subcommand (ships with Skills v2).
6. Bundling the Tauri DMG into the one-liner.
7. GPG-signed `SHA256SUMS.sig`.

Do not attempt these in this plan. If a task as-written requires any of them, stop and escalate.

---

## Appendix: quick reference

**Branch:** `claude/hearth-auto-deploy` (off `main` @ `bce8989`).

**Key verified paths (post-workspace-split):**
- Workspace root Cargo: `src-tauri/Cargo.toml` has `[workspace]` + `[workspace.package]` (no top-level version).
- Crate versions live in `src-tauri/{app,core,cli}/Cargo.toml`.
- Tauri config: `src-tauri/app/tauri.conf.json`.

**Exit code conventions in new scripts:**
- 0 success; 1 runtime failure; 64 usage error; 65 invalid arg; 66 missing file.

**No Rust code changes. No SKILL.md changes.** If a task asks you to edit Rust source or a skill file, stop — something has gone off-plan.
