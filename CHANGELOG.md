# Changelog

All notable changes to Hearth are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1] - 2026-04-18

### Changed
- **New app icon** — a warm flame over a hearthstone in a deep midnight squircle. The scaffold-default Tauri logo is gone; Hearth now looks like its name.

### Fixed
- `scripts/release.sh` updater-signing regression: the standalone `tauri signer sign` CLI refuses to accept `--private-key-path` when `TAURI_SIGNING_PRIVATE_KEY` is also present in the environment. The release driver now scrubs that env var for just the sign call. This fix is what makes v0.2.1 (and any future release) shippable.

## [0.2.0] - 2026-04-18

First public release.

### Added
- macOS Apple Silicon (aarch64) DMG, signed with Developer ID and notarized by Apple.
- Auto-updater (`tauri-plugin-updater`): checks for updates 30 seconds after launch and every 24 hours. When a newer version is available, a toast offers "지금 재시작" (install now) or "나중에" (skip this version).
- `CHANGELOG.md` + `docs/releasing.md` + `scripts/release.sh` release tooling.
- Content Security Policy restricting network access to `self`, the OpenAI API, and the local MLX endpoint.

### Changed
- Cargo package renamed `tauri-app` → `hearth`; library name `tauri_app_lib` → `hearth_lib`.
- `package.json` / `Cargo.toml` / `tauri.conf.json` populated with authorship, license (MIT), and repository metadata.
- README Installation section now points to the GitHub Releases DMG.

### Known limitations
- Intel Mac (x86_64) builds are not yet distributed — v0.2.0 is Apple Silicon only. MLX backend was Apple Silicon exclusive already.
- Windows and Linux builds are not yet distributed.
- The OpenAI API key is still stored in the local SQLite database in plain text; migration to the macOS Keychain is tracked in a separate spec.

[Unreleased]: https://github.com/NewTurn2017/hearth/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/NewTurn2017/hearth/releases/tag/v0.2.1
[0.2.0]: https://github.com/NewTurn2017/hearth/releases/tag/v0.2.0
