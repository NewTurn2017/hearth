# Quick Capture — Design

**버전 타겟:** Hearth 0.5.0
**작성일:** 2026-04-19
**상태:** Draft → User Review

---

## 목적

Hearth가 포커스되어 있지 않아도 **전역 단축키 한 번**으로 한 줄 메모를 뇌에서 앱으로 흘려보낼 수 있게 한다. 뇌 덤프와 앱 전환 사이의 마찰(앱 스위치 → ⌘K → 입력)을 0으로 만드는 게 유일한 목표다.

## 비범위 (Non-Goals)

다음은 이 릴리즈에 넣지 않는다. 별도 spec으로 분리한다 — `docs/superpowers/ideas-backlog.md` 참조.

- 입력 내용으로 메모/일정/프로젝트 자동 분기
- Quick Capture 단계에서 색·프로젝트 지정
- 더블 ⌘ 서먼 · 메뉴바 아이콘
- Today 뷰, 알림 스누즈

## UX 스펙

### 트리거

- 기본 전역 단축키: **`Ctrl+Shift+H`** (Tauri accelerator: `CommandOrControl+Shift+H`, macOS에서도 실제 Ctrl 키 사용).
- 설정에서 리바인드 가능. 설정 키: `shortcut.quick_capture`.
- 창이 열린 상태에서 같은 단축키를 다시 누르면 **토글 닫기**.
- Hearth가 실행 중이 아니어도 단축키가 앱을 백그라운드로 기동시키고 오버레이를 띄운다. (Tauri autostart와 독립적으로 OS가 앱을 깨우는 구조는 없으므로, 이 요건은 **Hearth가 백그라운드에 살아있을 때 한정**으로 실제 보장된다 — Limitation 섹션 참조.)

### 오버레이 창

- 화면 중앙 상단(수평 가운데, 세로 위쪽 1/3 지점)에 작은 창. 폭 560px, 초기 높이 ~80px. 여러 줄(Shift+Enter) 입력 시 최대 200px까지 자동 확장.
- Borderless, 타이틀 없음, decorations off, transparent 배경, always-on-top, skip-taskbar.
- 내부: 단일 `<input>` placeholder `"뇌에 있는 거 한 줄..."`. 포커스 자동 진입.
- 블러(포커스 잃음) → 자동으로 닫힘(저장 없이).
- 닫힐 때 페이드 50ms.

### 입력 동작

| 키 | 동작 |
|---|---|
| `Enter` | 입력값을 메모로 저장. 빈 입력이면 저장하지 않고 닫기만 함 |
| `Shift+Enter` | 줄바꿈 (여러 줄 뇌 덤프 허용) |
| `Esc` | 저장 없이 닫기 |
| 전역 단축키 재입력 | 저장 없이 닫기 (토글) |

### 저장된 메모

- 색: 기본값 `yellow` (노랑)
- 프로젝트: `null` (어디에도 붙지 않음)
- 생성 시각: 현재
- 메모 보드 상단에 쌓임 (기존 정렬이 `created_at DESC`라면 자연히 위로)

### 피드백

- 메인 Hearth 창이 **포커스 혹은 최소한 가시 상태**면 기존 토스트 시스템으로 `"메모 추가됨"` 토스트 노출. 토스트 클릭 시 메모 탭으로 이동 + 해당 메모로 스크롤 + 2초 앰버 펄스(기존 ⌘F highlight pulse 재사용).
- 숨겨져 있으면 무음. 시스템 알림은 쓰지 않는다(피로 누적 방지).

### 설정 UI

**설정 → 일반** 기존 섹션 하단에 "Quick Capture" 블록 추가:

- 현재 단축키 뱃지 표시 (예: `⌃⇧H`)
- "변경" 버튼 → `ShortcutRecorder` 모달 (키 조합 녹화) → 저장 시 `rebind_shortcut` 호출
- 등록 실패 시 인라인 경고: `"단축키를 등록할 수 없습니다. 다른 앱이 사용 중일 수 있어요."`
- "기본값으로 되돌리기" 링크

## 아키텍처

### 새 의존성

- `Cargo.toml`: `tauri-plugin-global-shortcut = "2"`
- `package.json`: `@tauri-apps/plugin-global-shortcut`
- `src-tauri/capabilities/default.json`: `global-shortcut:allow-register`, `unregister`, `is-registered` 권한 추가

### Rust (`src-tauri/src/`)

**새 파일 `cmd_quick_capture.rs`**

```rust
// 공개되는 Tauri commands:
#[tauri::command] async fn get_quick_capture_shortcut() -> Result<String, String>
#[tauri::command] async fn rebind_quick_capture_shortcut(combo: String) -> Result<(), String>
#[tauri::command] async fn show_quick_capture_window(app: AppHandle) -> Result<(), String>
#[tauri::command] async fn hide_quick_capture_window(app: AppHandle) -> Result<(), String>
#[tauri::command] async fn toggle_quick_capture_window(app: AppHandle) -> Result<(), String>
```

- 기동 시 `lib.rs`/`main.rs`의 `setup` 훅에서:
  1. `settings` KV `shortcut.quick_capture` 조회. 없으면 기본값 `"CommandOrControl+Shift+H"` 시드(최초 1회).
  2. 해당 accelerator를 global-shortcut 플러그인에 등록. 핸들러는 `toggle_quick_capture_window`.
  3. 등록 실패 시 KV `shortcut.quick_capture.last_error`에 메시지 저장.
- `rebind_quick_capture_shortcut`:
  1. 기존 단축키 unregister
  2. 새 combo 등록 시도 → 성공 시 KV 갱신 + `last_error` 삭제, 실패 시 옛 combo 재등록 + 에러 반환

**Accelerator 정규화**

프론트에서 들어온 combo 문자열을 Tauri 포맷으로 변환하는 작은 헬퍼(`normalize_accelerator`) 포함. 지원 수정자: `Cmd/Ctrl/Alt/Shift` + 단일 알파벳/숫자/기능키. 잘못된 조합은 `Err("Invalid accelerator: ...")`.

### Tauri 윈도우

런타임 생성 (별도 `tauri.conf.json` 윈도우 항목 불필요):

```rust
WebviewWindowBuilder::new(&app, "quick-capture", WebviewUrl::App("index.html?window=quick-capture".into()))
    .decorations(false)
    .resizable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .transparent(true)
    .inner_size(560.0, 80.0)  // 프론트에서 내용에 따라 resize (최대 200)
    .visible(false)           // show_quick_capture_window 에서 명시적으로 show+focus
    .build()?
```

창은 기동 시 한 번 빌드해 숨겨두고, `show/hide`로 재사용한다(매번 새 창 생성 비용 회피).

중앙 상단 위치 계산: `monitor()` 로 current display 크기 얻어 `set_position` 호출.

### Frontend (`src/`)

**새 라우트 구분**

`main.tsx`에서 `URLSearchParams`의 `window === "quick-capture"`인 경우 `<QuickCapture />`만 렌더(레이아웃 없이). 아니면 기존 `<App />`.

**`src/windows/QuickCapture.tsx`**

```
- useRef<HTMLInputElement> 에 autoFocus
- onKeyDown:
    Enter (no shift) → submit()
    Esc              → invoke("hide_quick_capture_window")
- submit():
    if trimmed empty → invoke("hide_quick_capture_window"); return
    invoke("create_memo", { content, color: "yellow", project_id: null })
    emit("memo:quick-captured", { memo })
    invoke("hide_quick_capture_window")
- window blur 이벤트 → invoke("hide_quick_capture_window")
```

**`src/components/settings/ShortcutRecorder.tsx`**

- 포커스된 영역에서 `keydown` 캡처 → modifier + key 조합을 `Cmd+Shift+H` 형태 표기, Tauri accelerator `CommandOrControl+Shift+H` 형태 반환.
- ESC = 취소, Enter = 확인.
- 수정자만 눌린 상태는 무시.
- 시각적 피드백: 현재까지 눌린 수정자를 실시간 표기.

**설정 통합**

- `SettingsGeneralSection` 에 "Quick Capture" 섹션 추가
- `useQuickCaptureShortcut` 훅: 현재 값 읽고, 리바인드 이벤트로 갱신 (기존 `useAiStatus`/`useCategories` 패턴과 동일한 window CustomEvent — `quick-capture-shortcut:changed`).

**메인 창 토스트**

- `App`(또는 `Layout`) 레벨에서 `memo:quick-captured` 윈도우 이벤트 구독
- 기존 토스트 시스템으로 `"메모 추가됨 — 클릭해서 보기"` 노출
- 클릭 시 메모 탭으로 전환 + 해당 `memo.id`를 ⌘F highlight pulse 대상으로 전달 (기존 로직 재사용)
- `memos:changed` 도 함께 디스패치해 `useMemos` 갱신

### 데이터 흐름 (정상 경로)

```
[다른 앱 포커스] → 사용자 ⌃⇧H
       ↓
Rust global-shortcut 핸들러
       ↓
toggle_quick_capture_window (숨김 상태라면 show + position + focus)
       ↓
사용자 입력 → Enter
       ↓
invoke("create_memo", ...) → rusqlite INSERT
       ↓
emit "memo:quick-captured" (윈도우 이벤트) + hide_quick_capture_window
       ↓
메인 창 Layout이 리스너로 토스트 표시 + memos:changed 전파
```

## 에러 & 경계

| 상황 | 동작 |
|---|---|
| 단축키 등록 실패(OS 점유) | KV에 `last_error` 저장. 메인 창 첫 렌더 시 1회 토스트 + 설정 섹션에 인라인 경고 |
| 리바인드 실패 | 새 combo 거부, 옛 combo 유지, 사용자에게 에러 메시지 |
| 빈 입력 + Enter | 저장 없이 닫기 (no-op) |
| DB INSERT 실패 | 오버레이 닫지 않고 인라인 에러 1줄 (`"저장 실패 — 다시 시도"`). 사용자가 텍스트 잃지 않게 |
| 오버레이 열린 상태에서 앱 종료 | 창은 메인 윈도우 라이프사이클에 붙어 자동 정리 |
| 멀티 모니터 | `primary_monitor()` 기준. 후속 개선 여지 있음 |

## 테스트

### Rust 단위/통합 (로컬, API 콜 없음)

- `cmd_quick_capture::normalize_accelerator` — 합법 조합, 수정자 없는 키, 잘못된 문자열 케이스
- `settings` KV 왕복 — 기본값 시드, rebind 후 재조회
- `cmd_memos::create_memo` 는 기존 테스트 커버 → 신규 테스트 불필요

### Frontend Vitest

- `QuickCapture` — Enter 제출이 `create_memo` 호출, Esc 닫기, 빈 입력 no-op, Shift+Enter 는 줄바꿈만
- `ShortcutRecorder` — modifier+key 조합 캡처, 수정자 전용 입력 무시, Esc 취소

### 수동 QA 체크리스트

1. 다른 앱에서 `⌃⇧H` → 오버레이 → "test 메모" → Enter → 메모 탭에 추가됨 확인 + 토스트 클릭으로 이동
2. 메인 창 숨긴 상태에서 단축키 사용 → 토스트 없음 / 메모는 저장됨 확인
3. 설정에서 `⌃⇧J`로 리바인드 → 옛 조합(`⌃⇧H`) 무효, 새 조합 작동
4. 이미 OS가 점유한 조합(예: `⌘Space`)으로 리바인드 시도 → 인라인 에러
5. 오버레이 열린 상태에서 다른 앱 클릭 → 자동 닫힘
6. `Shift+Enter` 로 2줄 입력 → 저장 시 줄바꿈 유지 확인

## Limitations (명시적)

- **Hearth가 완전히 종료된 상태에서는 단축키가 동작하지 않는다.** Tauri 앱은 OS 레벨 데몬이 아니라 프로세스다. "로그인 시 자동실행" 을 켜두는 게 실질적인 해결책이며, 이 점을 설정 섹션 도움말에 한 줄로 명시한다.
- 멀티 모니터에서 "포커스된 모니터"에 띄우는 건 1차 범위에서 제외. 항상 primary.

## 릴리즈 체크

- CHANGELOG `[0.5.0]` 엔트리: "Quick Capture — 전역 단축키로 한 줄 메모"
- README "Features" 리스트에 한 줄 추가
- 설정 → 일반에 "Quick Capture 단축키" 섹션 스크린샷(선택)

## 예상 규모

- Rust 신규: ~200 LoC + 테스트 ~80 LoC
- Frontend 신규: `QuickCapture` ~80 LoC, `ShortcutRecorder` ~120 LoC, 설정 통합 ~60 LoC + 테스트 ~100 LoC
- 1~2일치 구현.
