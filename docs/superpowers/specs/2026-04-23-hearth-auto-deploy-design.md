# Hearth Auto-Deploy — Design Spec (v1)

**Date:** 2026-04-23
**Parent goal:** Agent-driven hearth control — CLI (done) → Skills (done) → **Auto-deploy (this)**
**Prereq:** Merged v0.7.0 on `main` (`24cd949`, CLI + Skills v1 + theme).

## 1. Motivation

CLI + Skills landed, but installing them is still a multi-step ritual (build from source, copy binary, run `scripts/install-skills.sh --into ...`). The natural next step is a single `curl -sSL .../install.sh | bash` that gets an agent-facing user fully set up. The target persona: a developer who drives hearth from Claude Code or Codex and may not run the Tauri GUI at all.

## 2. Goals & Non-Goals

### Goals

- One-liner install: `curl -sSL https://raw.githubusercontent.com/NewTurn2017/hearth/main/scripts/install.sh | bash` places `hearth` on PATH and symlinks the 3 v1 skills into the appropriate agent host dir.
- Automated release pipeline: tagging `vX.Y.Z` builds `hearth` for macOS aarch64 + Linux x86_64 and uploads the binaries plus a version-pinned skills tarball to a GitHub Release.
- Korean install guide (`docs/install-ko.md`) covering the one-liner, env vars, PATH setup, Gatekeeper workaround, agent host mapping, upgrade, uninstall, troubleshooting.
- Idempotent re-run → upgrade path. `--uninstall` flag for clean removal.

### Non-Goals (deferred to later sub-projects — must ship eventually)

- Homebrew tap (`brew install newturn2017/tap/hearth`).
- macOS x86_64 and Linux aarch64 target expansion.
- Windows support.
- Custom install domain (`hearth.sh`).
- `hearth --self-update` subcommand.
- `hearth skills list|install|uninstall` CLI subcommand — pairs with the eventual Skills v2 expansion.
- Tauri DMG integration into the one-liner (the GUI app continues to ship via the existing `scripts/release.sh` → notarized DMG flow).

## 3. Core Decisions

| Decision | Value | Why |
|---|---|---|
| Scope | CLI + Skills only (no Tauri app in one-liner) | Agent persona is GUI-free; DMG path already exists and needs notarize which doesn't fit `curl \| bash` |
| Platforms | macOS aarch64 + Linux x86_64 | Covers primary dev machine + Codex/CI/server use case. Full matrix deferred |
| Build automation | GitHub Actions matrix on `v*` tag push | Genuine auto-deploy; no `cross`/Docker on dev machine; easy to extend later |
| Bin location | `${HEARTH_BIN_DIR:-$HOME/.local/bin}/hearth` | User-local, no sudo, standard XDG-ish. Env var for override |
| Skills location | Auto-detect `~/.claude/skills` and/or `~/.codex/skills`; neither → default `~/.claude/skills`; `$HEARTH_SKILLS_DIR` overrides | Non-interactive (curl-pipe-bash friendly); covers the 3 likely setups |
| Binary format | Raw stripped binary inside `.tar.gz` per triple | Small, universal; no notarize needed (CLI has no Apple GUI surface) |
| Skills format | Version-pinned `hearth-skills-<version>.tar.gz` attached to each Release | Install-time reproducibility; a v0.8.0 install gets v0.8.0 skills, not a mid-work `main` |
| Verification | `SHA256SUMS` file uploaded alongside tarballs; install.sh verifies before extract | Protects against partial downloads and tampered mirrors |
| Gatekeeper on macOS | Install script prints `xattr -d com.apple.quarantine` hint on success | Notarize is out of scope for CLI; this is the documented first-run escape hatch |
| Uninstall scope | Remove `hearth` binary + only the symlinks install.sh would have created; preserve staged skill tarball versions | Symmetric with install; no collateral damage |
| Release-note source | Reuse existing `scripts/extract-release-notes.sh` to pull the matching CHANGELOG section for the Release body | Single source of truth |

## 4. GitHub Actions Workflow

**File:** `.github/workflows/release-cli.yml`
**Trigger:** `push` on tags matching `v*`.

### Jobs

**Job `build-cli`** — matrix:
- `{ os: macos-14, target: aarch64-apple-darwin }`
- `{ os: ubuntu-22.04, target: x86_64-unknown-linux-gnu }`

Steps:
1. `actions/checkout@v4`
2. `dtolnay/rust-toolchain@stable` with `targets: ${{ matrix.target }}`
3. `Swatinem/rust-cache@v2` with key including matrix target
4. `cd src-tauri && cargo build --release --target ${{ matrix.target }} -p hearth-cli`
5. `strip` (macOS: `strip`, Linux: `strip` from binutils) the binary
6. `tar czf hearth-${{ github.ref_name }}-${{ matrix.target }}.tar.gz -C src-tauri/target/${{ matrix.target }}/release hearth`
7. Upload as job artifact named `hearth-${{ matrix.target }}`

**Job `bundle-skills`** — runs-on: `ubuntu-latest`, no matrix:
1. Checkout
2. `tar czf hearth-skills-${{ github.ref_name }}.tar.gz skills/` (the three SKILL.md directories + `skills/README.md`)
3. Upload artifact `skills-bundle`

**Job `publish`** — `needs: [build-cli, bundle-skills]`, runs-on: `ubuntu-latest`:
1. `actions/download-artifact@v4` with `path: dist/` to pull all three artifacts
2. Generate `SHA256SUMS` file over the three tarballs
3. `scripts/extract-release-notes.sh ${{ github.ref_name }}` → `RELEASE_NOTES.md`
4. `gh release create` (or `gh release edit` if the tag already has a release from `scripts/release.sh`) with `--notes-file RELEASE_NOTES.md` and upload all 4 files (`hearth-<ver>-aarch64-apple-darwin.tar.gz`, `hearth-<ver>-x86_64-unknown-linux-gnu.tar.gz`, `hearth-skills-<ver>.tar.gz`, `SHA256SUMS`)

### Idempotency vs. existing `release.sh`

`scripts/release.sh` creates a GitHub Release with the DMG + updater assets. The new workflow runs on the same tag push. Ordering cases:

- Actions runs before `release.sh` finishes: Actions creates the Release. When `release.sh` tries `gh release create`, it fails on "already exists" → needs a tweak to use `gh release upload --clobber` when the release exists.
- `release.sh` runs first locally: Actions detects the existing release and uses `gh release upload --clobber`.

Implementation uses `gh release upload --clobber` unconditionally in both pipelines. An audit of `release.sh` is part of this plan.

### Secrets / auth

`GITHUB_TOKEN` (provided by Actions automatically) has write access to releases in the same repo. No custom secrets needed for CLI build. No codesigning, no notarize.

## 5. `scripts/install.sh`

### One-liner

```bash
curl -sSL https://raw.githubusercontent.com/NewTurn2017/hearth/main/scripts/install.sh | bash
```

### Environment variables

- `HEARTH_VERSION` — pin a specific version tag (default: latest release).
- `HEARTH_BIN_DIR` — install destination for the binary (default: `$HOME/.local/bin`).
- `HEARTH_SKILLS_DIR` — override skills destination (default: auto-detect).
- `HEARTH_TMPDIR` — staging dir (default: `$(mktemp -d)`, deleted on exit).

### Flags (parsed from `bash -s -- <flags>`)

- `--version X.Y.Z` — alias for `HEARTH_VERSION=vX.Y.Z`.
- `--prefix DIR` — alias for `HEARTH_BIN_DIR=DIR`.
- `--skills-dir DIR` — alias for `HEARTH_SKILLS_DIR=DIR`.
- `--uninstall` — remove the binary + only the symlinks this script would create.
- `--dry-run` — print what would happen, no writes.

### Execution order (fixed)

1. `set -euo pipefail`. Detect platform:
   - `uname -s` / `uname -m` → supported: `Darwin`/`arm64` → `aarch64-apple-darwin`; `Linux`/`x86_64` → `x86_64-unknown-linux-gnu`.
   - Unsupported → abort with "unsupported platform <os>/<arch>; see https://github.com/NewTurn2017/hearth/blob/main/docs/install-ko.md".

2. Resolve version:
   - If `$HEARTH_VERSION` set → use it verbatim (e.g. `v0.8.0`).
   - Else → GET `https://api.github.com/repos/NewTurn2017/hearth/releases/latest` → `.tag_name`.
   - Unauthenticated API gives 60 req/hour per IP — plenty for one-shot install.

3. Resolve binary dir: `${HEARTH_BIN_DIR:-$HOME/.local/bin}`. `mkdir -p`.

4. Resolve skills dirs (result is a non-empty list):
   - If `$HEARTH_SKILLS_DIR` set → `[$HEARTH_SKILLS_DIR]`.
   - Else collect: `~/.claude/skills` if `~/.claude` exists, `~/.codex/skills` if `~/.codex` exists.
   - If list still empty → `[$HOME/.claude/skills]` (default).
   - `mkdir -p` each.

5. Download + verify (into `HEARTH_TMPDIR`):
   - Download `SHA256SUMS` from the release.
   - Download `hearth-<version>-<triple>.tar.gz`, `hearth-skills-<version>.tar.gz`.
   - Verify both with `shasum -a 256 -c SHA256SUMS` (or `sha256sum -c` on Linux). Abort on mismatch.

6. Install binary:
   - Extract tarball → `$HEARTH_BIN_DIR/hearth`, `chmod +x`.

7. Install skills:
   - Extract `hearth-skills-<version>.tar.gz` into `$HOME/.local/share/hearth/skills-<version>/` (versioned staging; multiple versions coexist).
   - For each resolved skills dir and each `skills-<version>/<name>/`, create/refresh an absolute-path symlink `<dir>/<name>` → `$HOME/.local/share/hearth/skills-<version>/<name>`. Refuse to overwrite a non-symlink (same contract as `install-skills.sh`).

8. Verify: run `$HEARTH_BIN_DIR/hearth db path`. Non-zero → print error + hint.

9. Print summary:
   ```
   ✓ hearth <version> installed at <bindir>/hearth
   ✓ Skills v<version> linked into: <list>

   Next steps:
     - Ensure <bindir> is on your PATH:
         echo 'export PATH="<bindir>:$PATH"' >> ~/.zshrc
     - macOS first-run (if you see "cannot be opened"):
         xattr -d com.apple.quarantine <bindir>/hearth

   Docs:
     - Korean: https://github.com/NewTurn2017/hearth/blob/main/docs/install-ko.md
     - Skills: https://github.com/NewTurn2017/hearth/blob/main/skills/README.md
   ```

### Uninstall path

`--uninstall`:
1. Resolve the same `HEARTH_BIN_DIR` and skills dirs.
2. Remove `$HEARTH_BIN_DIR/hearth` if it exists.
3. For each skills dir, remove symlinks whose name matches a current/prior-version skill and whose target is inside `$HOME/.local/share/hearth/`. Preserve unrelated entries and direct (non-symlinked) skill dirs.
4. Do NOT remove `$HOME/.local/share/hearth/skills-<version>/` automatically (keep for rollback); print hint: `rm -rf ~/.local/share/hearth/` to fully purge.

### Dry-run

`--dry-run` prints every action (URL downloads, chmod, symlink create, etc.) without side effects. Validates platform detection and SHA fetch succeed.

## 6. Tests

### `scripts/tests/test_install.sh`

Harness (TDD): written before `install.sh`, fails until `install.sh` is executable and behaves.

Assertions (minimum set):
1. Missing/unsupported platform stub exits non-zero with the platform-unsupported message.
2. `--dry-run` prints expected URL + extraction plan, no filesystem writes.
3. SHA mismatch against a tampered `SHA256SUMS` fixture aborts before any extraction.
4. Happy path with fixture tarballs (served from a local `python -m http.server` or file:// URL via an env-overridable base URL `HEARTH_RELEASES_URL`): binary lands at `$HEARTH_BIN_DIR/hearth`, each skill is a symlink, `hearth db path` succeeds (using a dummy binary script fixture).
5. Re-run is idempotent: symlinks refreshed, no error on existing binary.
6. `--uninstall` removes binary + only this-script's symlinks; preserves unrelated symlinks and direct files.

Fixture strategy: the test creates a tmp release dir with:
- A fake `hearth-v0.0.0-<triple>.tar.gz` whose contents is a shell stub emitting `{"ok":true,"data":"/tmp/fake"}` on `db path`.
- A fake `hearth-skills-v0.0.0.tar.gz` containing `skills/hearth-today-brief/SKILL.md` etc.
- A matching `SHA256SUMS`.

The test sets `HEARTH_RELEASES_URL=file:///.../fixture-release` and `HEARTH_VERSION=v0.0.0` so install.sh downloads from the local path.

Required install.sh hook: treat the release URL as an env var (default production URL).

### CI dry-run

Before the first real tag, push a test tag `v0.0.0-rc1` → watch the workflow → delete the rc release + tag. No merge needed in main for this.

### Manual smoke on clean environments

Before calling v1 done:
1. Fresh macOS user: run the one-liner, verify `hearth today` from a new shell.
2. Ubuntu 22.04 container: `docker run -it ubuntu:22.04 bash -c 'apt update && apt install -y curl ca-certificates && <one-liner>'`; verify `hearth today`.

## 7. `docs/install-ko.md` outline

Sections:
1. **한 줄 설치** — the curl command + 1-sentence explanation of what lands where.
2. **환경 변수** — `HEARTH_VERSION`, `HEARTH_BIN_DIR`, `HEARTH_SKILLS_DIR` with examples.
3. **플래그** — `--version`, `--prefix`, `--skills-dir`, `--uninstall`, `--dry-run`.
4. **PATH 추가** — zsh/bash snippets for `~/.local/bin`.
5. **macOS Gatekeeper** — `xattr -d com.apple.quarantine` walkthrough with rationale (CLI notarize 안 함, 이는 의도된 선택).
6. **에이전트 호스트별 동작** — `~/.claude/skills` vs `~/.codex/skills` auto-detect; 둘 다 있는 경우, 수동 override 예시.
7. **업그레이드** — 같은 one-liner 재실행.
8. **삭제** — `curl ... | bash -s -- --uninstall` + 수동 정리.
9. **문제 해결** — command-not-found, cannot-be-opened, SHA mismatch, 샌드박스/proxy 환경에서의 대안 (tarball 수동 다운로드 후 `install.sh <local-tarball>` v2 고려사항으로 언급).

## 8. `scripts/bump-version.sh` audit

Current script (from before the workspace split) likely bumps `src-tauri/Cargo.toml` only. After the split, three crate manifests need bumping:

- `src-tauri/app/Cargo.toml`
- `src-tauri/core/Cargo.toml`
- `src-tauri/cli/Cargo.toml`

Plus the workspace root `src-tauri/Cargo.toml` `[package]` version (still used by the Tauri app crate for backwards-compat metadata).

Plan will audit and, if needed, update `bump-version.sh` to touch all four. `package.json` and `src-tauri/app/tauri.conf.json` versions must stay in sync — the existing preflight in `release.sh` already asserts this.

## 9. Repo layout (added / modified)

```
.github/workflows/
└── release-cli.yml                   # NEW — tag-triggered build matrix + publish job
scripts/
├── install.sh                        # NEW — the one-liner target
├── tests/
│   └── test_install.sh               # NEW — fixture-based TDD harness
├── bump-version.sh                   # MODIFIED if audit finds workspace-crate gap
└── release.sh                        # MODIFIED — `gh release upload --clobber` to be safe against Actions race
docs/
└── install-ko.md                     # NEW
README.md                             # MODIFIED — Installation section (one-liner on top, DMG second, build-from-source last)
CHANGELOG.md                          # MODIFIED — [0.8.0] entry listing auto-deploy + one-liner
```

No Rust code changes. No SKILL.md changes.

## 10. Release process (user-facing)

1. `./scripts/bump-version.sh 0.8.0` — bumps all version manifests (post-audit).
2. Edit `CHANGELOG.md`: rename `[Unreleased]` → `[0.8.0] - YYYY-MM-DD`.
3. `git commit -am "chore: 0.8.0"` and `git push`.
4. `git tag v0.8.0 && git push --tags`.
5. GitHub Actions runs `release-cli.yml` → Release now has CLI tarballs + skills tarball + SHA256SUMS.
6. (Optional, macOS-only) `./scripts/release.sh` — attaches the notarized DMG + updater assets to the same release.

## 11. v1 Deliverables Checklist

- [ ] `.github/workflows/release-cli.yml`
- [ ] `scripts/install.sh`
- [ ] `scripts/tests/test_install.sh` (TDD harness)
- [ ] `scripts/bump-version.sh` — audited, bumps all 4 workspace manifests (if needed)
- [ ] `scripts/release.sh` — `gh release upload --clobber` hardening
- [ ] `docs/install-ko.md`
- [ ] `README.md` — Installation section overhauled
- [ ] `CHANGELOG.md` — `[0.8.0]` entry
- [ ] This spec committed
- [ ] Implementation plan (next step, writing-plans skill)
- [ ] v0.0.0-rc CI dry-run executed successfully
- [ ] Live smoke on fresh macOS shell + Ubuntu container

## 12. Out-of-Scope Note (explicit)

These items **are needed eventually** and are committed to follow-up sub-projects:

1. **Homebrew tap** — `brew install newturn2017/tap/hearth`. Requires separate `homebrew-hearth` repo + Formula.rb auto-generation on release.
2. **Platform matrix expansion** — macOS x86_64, Linux aarch64, Windows. Trivial Actions matrix additions once demand appears.
3. **Custom domain** — `hearth.sh/install` short URL. Pure DNS + static redirect config.
4. **`hearth --self-update`** — reach back to the Releases API and upgrade in-place. Pairs well with a future skills registry.
5. **`hearth skills list|install|uninstall`** — the long-pending CLI subcommand. Ships with Skills v2.
6. **One-liner that includes Tauri DMG** — requires server-side unpack of the notarized DMG, or shipping a separate signed installer pkg. Out of reach without distributing notarization secrets.

## 13. Open Questions (none blocking v1)

- Skills tarball layout: should it flatten to `skills/<name>/SKILL.md` when extracted, or nest under `hearth-skills-<version>/skills/<name>/SKILL.md`? Plan will pin this before writing `install.sh`. Current preference: the inner nesting (`skills-<version>/<name>/`) so multiple versions coexist at `~/.local/share/hearth/`.
- `release.sh` currently runs `gh release create` unconditionally. Minimal fix: check if the release exists, and if yes use `gh release upload --clobber` instead. The audit step will formalize this.
- Whether to ship `SHA256SUMS.sig` (GPG-signed checksums) is v2 polish. v1 ships plain SHA256SUMS; the `curl`-pipe-bash model already trusts GitHub's TLS for integrity.
