# Changelog

All notable changes to Hearth are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.2] - 2026-04-19

### Fixed
- **DB 손상 크래시 방지 (crash on launch fix)**: `data.db` 파일이 손상된 상태 (`database disk image is malformed`) 로 부팅될 때 앱이 즉시 abort 되던 문제 수정. 이제 손상된 DB 는 `data.db.corrupt-<timestamp>` 로 자동 격리되고, 빈 스키마로 부팅되며, 토스트 알림이 표시되어 사용자가 Settings → 백업 → 복원에서 최근 백업으로 되돌릴 수 있습니다.

### Added
- `db::init_db_with_recovery` — DB 초기화 실패 시 손상 여부 (`DatabaseCorrupt` / `NotADatabase`) 를 감지하고, 손상된 파일 + WAL/SHM 사이드카를 격리 후 빈 DB 로 재시도.
- `db:recovered` Tauri 이벤트 + `useDbRecoveryNotice` 훅 — 자동 복구가 일어났을 때 sticky 토스트로 사용자에게 알림.

## [0.3.1] - 2026-04-18

### Changed
- README: 프로젝트 보드에 "Drag & Drop 재정렬" 기능 명확히 표기.
- README: 일정 알림, 로그인 시 자동실행 기능 설명 추가.
- README: v0.2.0 → 현재 로 버전 표기 개선.

## [0.3.0] - 2026-04-18

### Added
- **로그인 시 자동실행** (설정 → 일반). 백그라운드 숨김 시작, Dock 클릭 시 창 복원.
- **스케줄 알림**: 일정에 "알림 받기" 토글이 생겼고, 켜면 네이티브 시간 피커가 나타납니다. "5분 전" / "정각" 두 가지 오프셋을 독립적으로 선택 가능. 앱이 실행 중일 때 발송됩니다 (tauri-plugin-notification 은 desktop 에서 스케줄링을 지원하지 않아 in-process 타이머로 구현 — 자동실행이 켜져 있으면 로그인 이후 계속 실행되어 알림이 안정적으로 옵니다).
- 캘린더 뷰에서 알림이 켜진 일정 앞에 🔔 아이콘.

> ⚠️ 개발 모드(`npm run tauri dev`) 에서는 macOS 가 알림을 "Terminal" 로 표시합니다 — 플러그인이 is_dev() 에서 고정 ID 를 씁니다. 릴리즈 번들에서는 Hearth 의 공식 bundle id 로 제대로 뜹니다.

### Fixed
- 한글로 입력한 뒤 Enter 로 저장할 때 저장되지 않던 문제 (IME composition이 첫 Enter를 소비).
- 프로젝트 우선순위 재정렬 Drag & Drop: 드롭존 너비를 카드 전체로 확대해 드래그 가능 영역 개선.
- 프로젝트 우선순위 재정렬: pointer-first 이벤트 전략으로 전환해 카드 클릭이 드래그로 오인식되던 충돌 해소.
- 빨간 버튼(창 닫기) 클릭 시 Tauri 윈도우가 파괴되지 않고 숨겨지도록 수정 (재열기 가능하게).

### Removed / Changed
- 로컬 MLX 백엔드 제거. AI 는 OpenAI 전용으로 정리했고, API 키는 **선택 사항**입니다. 키가 없으면 ⌘K AI 명령만 비활성화되고, 프로젝트·메모·캘린더·알림 등 나머지 기능은 그대로 동작합니다.
- 상단바의 AI 상태 표시가 passive 아이콘으로 바뀌었습니다. 클릭하면 설정 → AI 탭이 열립니다.

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
