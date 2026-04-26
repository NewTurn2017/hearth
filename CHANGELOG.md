# Changelog

All notable changes to Hearth are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.5] - 2026-04-26

### Fixed

- **스킬/CLI DB 변경 현재 탭 즉시 반영**: 앱이 이미 감지하던 SQLite `PRAGMA data_version` 변경 신호를 React 데이터 훅 이벤트로 연결해, 메모보드·캘린더·프로젝트 화면이 탭 이동 없이 자동 갱신되도록 수정했습니다.

## [0.9.4] - 2026-04-26

### Fixed

- **프로젝트 카드 메모 줄바꿈 유지**: 편집 textarea에서 입력한 줄바꿈이 프로젝트 카드 preview에서도 그대로 보이도록 수정했습니다.

## [0.9.3] - 2026-04-26

### Fixed

- **프로젝트 카드 메모 미리보기 2줄 표시 보정**: 프로젝트 보드 카드의 평가/메모 영역을 최대 2줄로 클램프하고, 짧은 메모도 2줄 높이를 확보하도록 조정했습니다.

## [0.9.2] - 2026-04-26

### Fixed

- **설정 열 때 테마가 커스텀 기본값으로 바뀌는 문제 수정**: 설정 모달이 숨겨진 테마 탭까지 mount하면서 커스텀 테마 프리뷰가 자동 실행되던 회귀를 막았습니다. 이제 커스텀 프리뷰는 사용자가 색상/모드를 실제로 변경한 뒤에만 적용됩니다.

## [0.9.1] - 2026-04-26

### Added

- **설정 → 일반 → 수동 업데이트 확인**: 자동 업데이트 주기를 기다리지 않고 사용자가 직접 최신 버전을 확인할 수 있습니다. 최신이면 안내 토스트를 표시하고, 새 버전이 있으면 설정 화면에서 바로 설치 후 재시작할 수 있습니다.

### Changed

- README의 기능 목록, 업데이트 안내, 설정 구조, 테스트 수치를 현재 구현 상태에 맞게 정리했습니다.

## [0.9.0] - 2026-04-26

### Added

- **통합 `hearth` agent skill 단일화**: 프로젝트 등록/수정, 캘린더 등록/수정, 메모 등록/수정, 검색, 오늘 브리핑, 폴더 스캔, 메모 정리를 하나의 `hearth` 스킬이 라우팅합니다. 기존 개별 Hearth 스킬 심링크는 설치/업데이트 시 정리됩니다.
- **메모 매트릭스 2줄 미리보기**: 프로젝트별 메모 타일에서 긴 메모가 한 줄로 잘리지 않고 최대 두 줄까지 표시됩니다.

## [0.8.0] - 2026-04-24

### Added

- **한 줄 설치 (`scripts/install.sh`)**: `curl -sSL .../install.sh | bash` 로 `hearth` CLI + v1 skills 설치. macOS aarch64 + Linux x86_64. 환경변수 `HEARTH_VERSION`/`HEARTH_BIN_DIR`/`HEARTH_SKILLS_DIR` 로 override. `--uninstall` / `--dry-run` / `--version` / `--prefix` / `--skills-dir` 플래그 지원.
- **GitHub Actions release pipeline (`.github/workflows/release-cli.yml`)**: `vX.Y.Z` 태그 push 시 macOS aarch64 + Linux x86_64 바이너리와 version-pinned skills tarball, `SHA256SUMS` 를 자동 빌드해 Release 에 업로드.
- **한글 설치 가이드 (`docs/install-ko.md`)**: 한 줄 설치 · 환경변수 · PATH · Gatekeeper · 업그레이드 · 삭제 · 문제 해결.

### Fixed

- **`scripts/bump-version.sh`**: 워크스페이스 분할 이후 깨진 상태 복구. 이제 `package.json` + `src-tauri/app/tauri.conf.json` + 3개 crate `Cargo.toml` 전부 bump. TDD 회귀 테스트 (`scripts/tests/test_bump_version.sh`) 포함.
- **`scripts/release.sh`**: Actions 가 먼저 Release 를 만든 경우에도 실패하지 않게 `gh release upload --clobber` 로 하드닝.

## [0.7.0] - (unreleased)

### Added

- **Cargo workspace split**: `hearth-core` (pure logic) / `hearth-app` (Tauri) / `hearth-cli` (binary). All three share a single `Cargo.lock` and unified versioning.
- **New `hearth` CLI binary** — full CRUD for projects / memos / schedules / categories / clients, plus composite views (`today`, `overdue`, `stats`), audit log (`log show` / `undo` / `redo`), and import/export (JSON).
- **App live-refresh** — detects external DB writes via `PRAGMA data_version` polling (500ms) and emits window custom events (`projects:changed`, `memos:changed`, etc.) so the UI reflects CLI/background changes without restart.
- **DB schema additions**:
  - `audit_log` table: tracks all mutations (op, table, row_id, before/after JSON, source, timestamp, undone flag)
  - FTS5 virtual tables: `projects_fts`, `memos_fts`, `schedules_fts` with sync triggers (standalone mode for CLI searches)
- **Search powered by FTS5**: `hearth search` uses full-text search and returns ranked results with snippet highlighting.
- **Agent Skills (v1)**: 3 skills callable from Claude Code, Codex, or any host that loads standard `SKILL.md`:
  - `hearth-today-brief` — read-only 한국어 브리핑 (오늘 일정 + P0 + 최근 메모 + 연체).
  - `hearth-project-scan` — 디렉토리 → hearth 프로젝트 등록. 사용자 승인 후에만 적용.
  - `hearth-memo-organize` — 메모 → 프로젝트 보수적 재연결. 승인 후에만 적용.
- **`scripts/install-skills.sh`**: manual install path — symlinks `skills/*` into a user-specified dir. Requires explicit `--into`; supports `--remove`.
- **`scripts/smoke-skills.sh`**: seeds a throwaway DB and exercises every CLI recipe each v1 skill depends on.

### Notes

- App and CLI mutations share one `audit_log`. Changes made in the app appear in `hearth log show` and vice versa.
- Wire the React Undo toast to `hearth_core::audit::undo` so app + CLI share one undo stack (optional, pending follow-up spec).

## [0.6.0] - 2026-04-20

### Added

- **메모보드 매트릭스 뷰**: 헤더에 리스트 ↔ 매트릭스 토글 추가. 매트릭스는 프로젝트별 타일을 화면 폭에 맞춰 2/3/4열 그리드로 배치하고, 각 타일 안에서는 메모를 한 줄씩 컴팩트 리스트로 보여줘 한눈에 모든 프로젝트를 조감할 수 있습니다. 타일 행 클릭 → 인라인 편집, 우클릭 → 색상 변경 · 프로젝트 이동 · 삭제. 마지막 선택한 뷰는 `localStorage` 에 저장됩니다.

## [0.5.0] - 2026-04-20

### Added

- **Quick Capture**: 전역 단축키(기본 `⌃⇧H`, 설정 → 일반에서 리바인드 가능)로 Hearth가 포커스되지 않은 상태에서도 작은 오버레이 창이 떠서 한 줄 메모를 바로 저장합니다. `Enter` 저장 · `Shift+Enter` 줄바꿈 · `Esc` 취소 · 다시 단축키로 토글 닫기 · 다른 앱 클릭 시 자동 닫힘. 저장되면 메인 창에 "메모 추가됨" 토스트 + 해당 메모 카드로 스크롤 + 앰버 펄스.
- **설정 → 일반 → Quick Capture 섹션**: 현재 단축키 뱃지 + 변경 버튼(키 조합 녹화 UI) + 등록 실패 시 인라인 경고.

### Notes

- Hearth가 완전히 종료된 상태에서는 단축키가 작동하지 않습니다. 설정 → 일반 → "로그인 시 자동 실행" 을 켜두는 걸 추천합니다.

## [0.4.3] - 2026-04-19

### Added

- **`⌘F` 전체 검색 팔레트**: 프로젝트 · 메모 · 일정을 한 번에 퍼지 검색 (Fuse.js, 제목 가중치 ↑). 결과 클릭 시 해당 탭으로 전환 + 아이템으로 자동 스크롤 + 2초 동안 앰버 글로우 펄스로 위치 하이라이트. 일정은 해당 월로 이동하면서 이벤트 핑크가 노랗게 번쩍입니다. AI 모드(`⌘K`) 와 별개의 로컬 전용 검색이라 키 없이도 동작합니다.

## [0.4.2] - 2026-04-19

### Fixed

- **자동 업데이트 실패 수정 (`failed to unpack ._Hearth.app`)**: 릴리즈 스크립트가 macOS `tar` 기본 동작으로 AppleDouble 메타데이터(`._Hearth.app`)를 tarball 에 포함시켜 Tauri updater 가 압축 해제에 실패하던 문제. `COPYFILE_DISABLE=1` + `--no-xattrs` 로 메타데이터 제외. 0.4.1 업데이트가 실패했다면 0.4.2 로 바로 올라옵니다.

### Notes

- 0.4.1 의 기능 변경(기본 터미널, Finder 피커)은 그대로 포함됩니다.

## [0.4.1] - 2026-04-19

### Changed

- **프로젝트 경로 열기: Ghostty → macOS 기본 터미널 (Terminal.app)**: `open -a Ghostty` 하드코딩을 제거하고 시스템 기본 Terminal.app 을 호출합니다. 컨텍스트 메뉴/호버 버튼 라벨도 "터미널에서 열기" 로 변경.

### Added

- **프로젝트 경로 Finder 피커**: 프로젝트 생성/편집 폼의 경로 입력 옆에 폴더 선택 버튼이 생겼습니다. 직접 입력 대신 Finder 다이얼로그로 폴더를 고를 수 있고, 기존 경로가 있으면 그 위치부터 열립니다.

## [0.4.0] - 2026-04-19

### Added

- **프로젝트 카드 우클릭 → "프로젝트 메모 추가"**: 메뉴 최상단에서 해당 프로젝트 스코프로 NewMemo 다이얼로그가 바로 열립니다.
- **TopBar 버전 + 업데이트 버튼**: 앱 로고 옆에 현재 버전 (`v0.4.0`) 을 항상 표시하고, 새 버전이 감지되면 우측에 primary 색상 "업데이트 vX" 버튼이 나타납니다. 클릭 시 download + relaunch. 기존 sticky 토스트는 유지.
- **설정 → 백업/가져오기** (탭 이름 `백업` → `백업/가져오기`):
  - 백업 경로 박스를 클릭하거나 `열기` 버튼을 누르면 Finder 에서 현재 백업 폴더가 열립니다.
  - `변경…` picker 가 현재 백업 위치에서 시작되도록 기본 경로 설정.
  - `Excel 가져오기` 섹션이 설정 안으로 들어왔고, 파일 picker 도 백업 위치에서 열립니다.
- **GFS 스타일 백업 보관 정책**: 단순 롤링 5개 → 최근 7일 (일별 1개) + 4주 (주별 1개) + 3개월 (월별 1개). 하루에 앱을 여러 번 닫아도 이전 복원점이 보존됩니다. `pre-reset-*.db` 스냅샷은 영향받지 않음.

### Changed

- **프로젝트 보드 반응형 그리드**: `grid-cols-1 md:grid-cols-2` → `repeat(auto-fill, minmax(320px, 1fr))`. 화면 넓이에 따라 1→2→3→4열 자동 확장 (MemoBoard 와 동일한 전략).
- **TopBar 외부 `가져오기` 버튼 제거**: `설정 → 백업/가져오기` 탭으로 이동해 백업/가져오기 워크플로우가 한 곳에서 완결되도록 재편.

### Migration

- 백업 파일 (`hearth-backup-*.db`) 의 보관 정책이 바뀌어, 처음 종료/수동 백업이 실행될 때 기존 파일들이 새 버킷 규칙 (일/주/월) 으로 재분류됩니다. `pre-reset-*.db` 는 그대로 유지.

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

[Unreleased]: https://github.com/NewTurn2017/hearth/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/NewTurn2017/hearth/releases/tag/v0.4.0
[0.3.2]: https://github.com/NewTurn2017/hearth/releases/tag/v0.3.2
[0.3.1]: https://github.com/NewTurn2017/hearth/releases/tag/v0.3.1
[0.3.0]: https://github.com/NewTurn2017/hearth/releases/tag/v0.3.0
[0.2.2]: https://github.com/NewTurn2017/hearth/releases/tag/v0.2.2
[0.2.1]: https://github.com/NewTurn2017/hearth/releases/tag/v0.2.1
[0.2.0]: https://github.com/NewTurn2017/hearth/releases/tag/v0.2.0
