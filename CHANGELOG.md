# Changelog

All notable changes to Hearth are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## 0.3.0 — 2026-04-18

### Added
- **로그인 시 자동실행** (설정 → 일반). 백그라운드 숨김 시작, Dock 클릭 시 창 복원.
- **스케줄 알림**: 일정에 "알림 받기" 토글이 생겼고, 켜면 네이티브 시간 피커가 나타납니다. "5분 전" / "정각" 두 가지 오프셋을 독립적으로 선택 가능. 앱이 실행 중일 때 발송됩니다 (tauri-plugin-notification 은 desktop 에서 스케줄링을 지원하지 않아 in-process 타이머로 구현 — 자동실행이 켜져 있으면 로그인 이후 계속 실행되어 알림이 안정적으로 옵니다).
- 캘린더 뷰에서 알림이 켜진 일정 앞에 🔔 아이콘.

### Fixed
- 한글로 입력한 뒤 Enter 로 저장할 때 저장되지 않던 문제 (IME composition이 첫 Enter를 소비).

### Migration
- `schedules` 테이블에 `remind_before_5min`, `remind_at_start` 컬럼 추가. 기존 행은 모두 `0 / 0` (알림 꺼짐) 상태로 유지.

## [0.2.2] - 2026-04-18

### Added
- **데이터 초기화** (Settings → 백업 → 위험 구역) — one-click wipe for projects · memos · schedules · clients, preserving categories · AI settings · backup path · UI scale. A `pre-reset-<timestamp>.db` snapshot is captured automatically **before** the wipe and kept indefinitely (exempt from rolling retention), so an accidental reset is recoverable from the Restore list.

### Changed
- `list_backups` now returns both rolling `hearth-backup-*.db` entries **and** `pre-reset-*.db` snapshots, so the Restore list shows the pre-reset safety copy.

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

[Unreleased]: https://github.com/NewTurn2017/hearth/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/NewTurn2017/hearth/releases/tag/v0.3.0
[0.2.2]: https://github.com/NewTurn2017/hearth/releases/tag/v0.2.2
[0.2.1]: https://github.com/NewTurn2017/hearth/releases/tag/v0.2.1
[0.2.0]: https://github.com/NewTurn2017/hearth/releases/tag/v0.2.0
