# Releasing Hearth

End-to-end checklist for cutting a new macOS release. Runs fully from a developer Mac with the credentials stored in `/Users/genie/dev/private/apple_developer/`. See [the release design spec](superpowers/specs/2026-04-18-tauri-release-design.md) for the rationale behind each step.

## Prerequisites (once)

- `Developer ID Application: jaehyun jang (2UANJX7ATM)` in keychain
- `.env.release` filled from `.env.release.example`
- `/Users/genie/dev/private/apple_developer/hearth_updater.key` (+ passphrase)
- `rustc -vV` reports host triple `aarch64-apple-darwin` (v0.2.0 ships Apple Silicon only — Homebrew rust or rustup both work)
- `gh auth status` OK
- `xcrun --find notarytool && xcrun --find stapler` both resolve

## Bump version

```bash
./scripts/bump-version.sh 0.3.0   # pick the next semver
```

This updates `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.

## Write the changelog entry

Add a new `## [0.3.0] - YYYY-MM-DD` section to `CHANGELOG.md` with the user-visible changes in this release (`Added` / `Changed` / `Fixed` / `Removed`). Commit.

## Dry-run the release

```bash
./scripts/release.sh --dry-run
```

This runs steps 1–9 (build, codesign verify, notarize, staple, sign updater tarball, build `latest.json`, extract release notes) but stops before tagging and publishing. Review `dist/release/latest.json` and `dist/release/notes.md` for sanity.

## Tier 3: Updater round-trip dry-run (first release only)

Skip this section for releases **after** v0.2.0 — it only exists to prove the updater works before any user installs v0.2.0. For v0.3.0+, rely on the fact that real v0.2.0 → v0.3.0 auto-update verifies the path implicitly.

For the **first** release (v0.2.0):

1. Check out a throwaway branch. Temporarily set `version = "0.1.9"` in all three manifests and `pubkey` unchanged.
2. `npm run tauri build -- --target aarch64-apple-darwin` — you get a local `fake-0.1.9.app`.
3. In a macOS guest/test user account, copy the fake app into `~/Applications` and launch.
4. While that fake app is running, upload the real v0.2.0 build to a **private** test GitHub Release (draft, or a separate test repo). Point the fake build's endpoint at that test `latest.json`.
5. Wait ~30 s after app launch. The "새 버전 0.2.0 준비됨" toast should appear.
6. Click **지금 재시작**. Within 2–3 seconds the app relaunches; window title remains "Hearth" and the DB (projects/memos/etc.) persists.
7. Delete the test release/repo; discard the throwaway branch; confirm the real `tauri.conf.json` endpoint is back to production.

If any step fails, fix in `scripts/release.sh` or `useAppUpdater.ts` before cutting v0.2.0.

## Real release

```bash
./scripts/release.sh
```

Expected elapsed time: 10–20 minutes, dominated by notarization (Apple typically responds in 2–5 min but the SLA is up to 1 hour). The script streams progress and aborts on any non-`Accepted` status.

On success: a new `vX.Y.Z` tag is pushed, a GitHub Release is published with 4 assets, and the script prints the Release URL.

## Tier 2: Post-release smoke

Perform once per release in a macOS guest user account (to reproduce first-run Gatekeeper):

- [ ] Download DMG from Releases → Applications
- [ ] Right-click → Open (1 Gatekeeper prompt max)
- [ ] Main window renders (sidebar + tabs)
- [ ] `⌘K` palette opens
- [ ] Create a project; quit; reopen; project persists
- [ ] Settings → AI → OpenAI key → `프로젝트 목록 보여줘` returns
- [ ] Settings → AI → provider `local` + MLX running → chat responds
- [ ] Settings → Backup → 지금 백업 → file created in configured dir
- [ ] `~/Library/Application Support/com.newturn2017.hearth/data.db` exists

Any failure → yank (`gh release delete vX.Y.Z && git push --delete origin vX.Y.Z`), fix, roll forward with vX.Y.(Z+1). Do **not** edit an existing release in place.

## Rollback

Prefer a roll-forward (vX.Y.(Z+1)). Only yank for security incidents; the automatic update channel will pull the new release to all existing users within ~24 hours of their next launch.
