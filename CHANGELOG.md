# Changelog

All notable changes to Hearth are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
