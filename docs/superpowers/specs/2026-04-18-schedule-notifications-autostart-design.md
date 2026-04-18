# Schedule Notifications + Autostart (v0.3.0)

**Status**: Draft — awaiting implementation
**Date**: 2026-04-18
**Target version**: 0.3.0

## Goal

세 가지 기능을 하나의 일관된 릴리즈로 묶어 스케줄 기능을 실사용 가능한 리마인더로 격상한다.

1. **로그인 시 자동 실행** — 백그라운드(창 숨김)로 시작, Dock 아이콘 클릭 시 복원.
2. **일정 시간 입력 UX 개선** — 네이티브 시간 피커 + 저장 버그(한글 IME Enter) 수정.
3. **macOS 데스크탑 알림** — 5분 전 / 정각 선택, 앱 종료 후에도 발송.

## Non-Goals

- Windows / Linux 알림 지원 (v0.2.x 와 동일하게 macOS 전용 빌드).
- 반복 일정 / RRULE 지원.
- 다중 리마인더 오프셋 (15분, 1시간, 1일 전) — 추후 확장.
- 메뉴바 트레이 아이콘.
- 알림에 "스누즈" / "완료" 액션 버튼.

## User Stories

- 사용자가 `새 일정` 에서 **알림 받기** 토글을 켜면 시간 피커가 나타나고, 5분 전 / 정각 체크박스로 언제 알림을 받을지 고를 수 있다.
- 토글을 끄면 시간 없이 "기록용" 일정으로 저장된다 (현재 데이터 모델과 호환).
- 한글로 장소·내용·비고를 입력한 뒤 Enter 를 치면 바로 저장된다. (현재 IME composition 으로 인한 무응답 버그가 사라진다.)
- 사용자가 설정 → 일반 탭에서 **로그인 시 자동 실행** 을 켜면 Mac 재부팅/로그인 때 Hearth 가 조용히 백그라운드에서 시작한다. Dock 아이콘을 클릭하면 창이 뜬다.
- 일정 시각 5분 전, 또는 정각에 macOS 네이티브 알림이 우측 상단에 뜬다. 앱을 종료한 상태여도 알림은 발송된다.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  React (ScheduleModal / SettingsGeneralSection)             │
│    • 알림 토글 → 시간 피커 + 5분전/정각 체크박스             │
│    • IME-safe Enter submit                                  │
│    • 자동실행 토글 + 알림 권한 상태                         │
└──────────────┬──────────────────────────────────────────────┘
               │ Tauri IPC
┌──────────────┴──────────────────────────────────────────────┐
│  Rust core                                                   │
│    cmd_schedules   — CRUD + reminder 컬럼 반영              │
│    cmd_notify      — 스케줄 변경 시 notification 재등록     │
│    cmd_autostart   — autostart 토글 래퍼                   │
│    boot hook       — 시작 시 미래 일정 notification 재등록  │
└──────────────┬──────────────────────────────────────────────┘
               │
   ┌───────────┴──────────┐
   ▼                      ▼
 macOS UNUserNotif.   Login Items (SMAppService)
 (survives quit)      (tauri-plugin-autostart)
```

**Source of truth** — notification 상태는 `schedules` 테이블. 매 부팅마다 DB 를 훑어 현재 등록된 notification 을 cancel 후 미래 일정만 re-schedule. 별도 job 테이블을 두지 않는다.

## Data Model

### 마이그레이션

`db.rs::run_migrations` 에 idempotent 헬퍼 추가:

```rust
fn ensure_schedule_reminder_columns(conn: &Connection) -> Result<()> {
    let cols: Vec<String> = conn
        .prepare("PRAGMA table_info('schedules')")?
        .query_map([], |r| r.get::<_, String>(1))?
        .filter_map(Result::ok)
        .collect();
    if !cols.iter().any(|c| c == "remind_before_5min") {
        conn.execute_batch(
            "ALTER TABLE schedules ADD COLUMN remind_before_5min INTEGER NOT NULL DEFAULT 0;"
        )?;
    }
    if !cols.iter().any(|c| c == "remind_at_start") {
        conn.execute_batch(
            "ALTER TABLE schedules ADD COLUMN remind_at_start INTEGER NOT NULL DEFAULT 0;"
        )?;
    }
    Ok(())
}
```

기존 행은 `0 / 0` = 알림 꺼짐 (현재와 동일한 동작).

### `settings` KV 추가 키

- `autostart.enabled` — `"1" / "0"`. UI 첫 렌더 시 플러그인 상태 조회 레이턴시 없이 즉시 읽기 위한 캐시.
- `notifications.permission` — `"granted" / "denied" / "unknown"`. 권한 UI 상태 표시.

### TypeScript 타입

`src/types.ts`:

```ts
export interface Schedule {
  id: number;
  date: string;
  time: string | null;
  location: string | null;
  description: string | null;
  notes: string | null;
  remind_before_5min: boolean;   // NEW
  remind_at_start: boolean;      // NEW
  created_at: string;
  updated_at: string;
}
```

`ScheduleInput` (API layer) 도 동일 필드 옵셔널 추가. 미지정 시 `false`.

### Notification ID 규칙

```
notification_id = schedule.id * 10 + offset_kind
  offset_kind: 1 = 5분 전, 2 = 정각
```

- 결정적 매핑으로 외부 테이블 불필요.
- 삭제 시 `[id*10+1, id*10+2]` 모두 cancel (없어도 no-op).
- 상한: `i32::MAX / 10` ≈ 2억 개 — 개인용 로컬 DB 로 충분.

## UI

### `ScheduleModal` 재설계

```
┌─ 일정 추가 ─────────────────────────┐
│ 날짜           [ 2026-04-18 ▾ ]    │
│                                     │
│ ☐ 알림 받기                         │   ← 토글
│                                     │
│   (토글 ON일 때만 아래 노출)         │
│   시간       [ 15:00 ▾ ]            │   ← <input type="time">
│   ☑ 5분 전   ☐ 정각                 │
│                                     │
│ 장소         [ _____________ ]      │
│ 내용         [ _____________ ]      │
│ 비고         [ _____________ ]      │
│                                     │
│             [취소] [저장]            │
└─────────────────────────────────────┘
```

**상태 머신**:

- `notify === false` → 시간/체크박스 미렌더. 저장 시 `time=null, remind_*=false`.
- `notify === true` 로 전환:
  - `time` 이 비어 있으면 `"09:00"` 기본 주입.
  - `remind_before_5min` 기본 `true`, `remind_at_start` 기본 `false`.
- `notify === true` && `time === ""` → 저장 버튼 `disabled`, hint "시간을 입력해 주세요".
- 편집 모드: `time != null || remind_* === true` 이면 `notify=true` 로 초기화.

### IME-safe Enter Submit

모든 `<Input>` 에 공통 onKeyDown 바인딩:

```tsx
function onEnterSubmit(e: React.KeyboardEvent<HTMLInputElement>) {
  const native = e.nativeEvent as KeyboardEvent & { keyCode?: number };
  if (
    e.key === "Enter" &&
    !native.isComposing &&
    native.keyCode !== 229 &&
    !e.shiftKey
  ) {
    e.preventDefault();
    e.currentTarget.form?.requestSubmit();
  }
}
```

`keyCode === 229` 는 WebKit 에서 IME 진행 중 Enter 의 레거시 신호(이중 안전판). `requestSubmit()` 은 form validation 포함한 정상 submit 경로.

### `SettingsGeneralSection` (신규)

`SettingsDialog` tabs: `[ai, backup, categories]` → `[general, ai, backup, categories]`.

```
┌─ 일반 ──────────────────────────────┐
│  자동 시작                           │
│  ☐ 로그인 시 Hearth 자동 실행         │
│     (백그라운드에서 조용히 시작)       │
│                                      │
│  알림                               │
│  상태: 허용됨 / 차단됨 / 미요청        │
│  [ 권한 요청 ]                       │
└──────────────────────────────────────┘
```

**동작**:

- 자동실행 토글 → `invoke("set_autostart", { enabled })` → Rust 에서 `autolaunch.enable_with_args(["--hidden"])` or `disable()` 호출 + KV 캐시 업데이트.
- 알림 상태: 최초 앱 시작 시 `invoke("notifications_permission")` 로 프로브 → `granted / denied / unknown`. 권한 요청 버튼은 시스템 프롬프트 트리거 (이미 결정된 상태면 "시스템 설정에서 변경" 안내).

### `CalendarView` 미니 개선

이벤트 타이틀에 알림 활성화 표시:

```tsx
title: [
  (s.remind_before_5min || s.remind_at_start) ? "🔔 " : "",
  s.description,
  s.location,
].filter(Boolean).join(" @ ")
```

## Notification Lifecycle

### 부팅 시 재등록

`lib.rs::setup()` 마지막 단계:

```rust
tauri::async_runtime::spawn(async move {
    if let Err(e) = cmd_notify::reschedule_all_future(&app_handle).await {
        eprintln!("notification reschedule failed: {e}");
    }
});
```

로직:

1. DB 에서 `SELECT id FROM schedules WHERE remind_before_5min OR remind_at_start` → 후보 목록.
2. 각 후보의 `[id*10+1, id*10+2]` 두 notification id 를 `cancel()` (idempotent).
3. `SELECT ... WHERE date >= today() AND (remind_before_5min OR remind_at_start)` → 미래 후보.
4. 각 행의 `at` 시각 계산, 과거는 스킵, 그 외 `schedule(id, title, body, at)`.

### 시간 파싱

- `date` = `YYYY-MM-DD`, `time` = `HH:MM`.
- `chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")` + `NaiveTime::parse_from_str(time, "%H:%M")`.
- `Local.from_local_datetime(...).single()` 로 로컬 타임존 적용.
- 5분 전 = `dt - chrono::Duration::minutes(5)`.
- 과거 시각 (`at < now`) 은 silent skip.

### 알림 내용

- **5분 전**: title `"일정 5분 전"`, body = `description` 또는 `"일정"` fallback.
- **정각**: title `"일정 시작"`, body 동일.
- 모두 macOS 기본 sound, badge 없음.

### CRUD 훅

- `create_schedule` / `update_schedule` → 정상 리턴 후 `notify::apply_for_id(id)` 호출.
- `delete_schedule` → `notify::cancel_for_id(id)`.
- `apply_for_id(id)`: cancel → DB row 조회 → 플래그·시각 기반 0/1/2개 재등록. 멱등.

### 권한 처리

최초 스케줄 등록 시도 전:

- `NotificationExt::permission_state()` 가 `Unknown` 이면 `request_permission()` 호출.
- `Denied` 면 `Err("notifications-denied")` 반환 → UI 에서 토스트 + 설정 → 일반 탭 안내.

## Autostart

### 의존성

`src-tauri/Cargo.toml`:

```toml
tauri-plugin-autostart = "2"
tauri-plugin-notification = "2"
```

### Capabilities

`src-tauri/capabilities/default.json` permissions 배열에 추가:

```json
"autostart:default",
"notification:default"
```

### 플러그인 초기화

`lib.rs`:

```rust
use tauri_plugin_autostart::MacosLauncher;

.plugin(tauri_plugin_autostart::init(
    MacosLauncher::LaunchAgent,
    Some(vec!["--hidden"]),
))
.plugin(tauri_plugin_notification::init())
```

### Hidden-on-boot 로직

`tauri.conf.json` main window: `"visible": false` (항상 숨김 시작).

`lib.rs::setup()`:

```rust
let launched_hidden = std::env::args().any(|a| a == "--hidden");
if let Some(window) = app.get_webview_window("main") {
    if !launched_hidden {
        let _ = window.show();
        let _ = window.set_focus();
    }
    // else: leave hidden; Dock 클릭 Reopen 으로 복원
}
```

### Dock 아이콘 Reopen 핸들러

`.run` 의 `RunEvent` 콜백:

```rust
app.run(|app_handle, event| {
    if let tauri::RunEvent::Reopen { has_visible_windows, .. } = event {
        if !has_visible_windows {
            if let Some(win) = app_handle.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }
    }
});
```

기존 `.run(tauri::generate_context!())` 한 줄을 위 분기로 확장. 기존 `on_window_event::Destroyed` 훅은 그대로 유지.

### 백엔드 API

`cmd_autostart.rs` (신규):

```rust
#[tauri::command]
pub async fn get_autostart(app: AppHandle) -> Result<bool, String> { ... }

#[tauri::command]
pub async fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> { ... }
```

`set_autostart` 는 플러그인 호출 + `settings` KV 캐시 동기화.

## Error Handling

| 시나리오 | 동작 |
|---|---|
| 알림 권한 거부 | 토스트 "알림 권한이 차단되어 있습니다. 설정 → 일반 탭에서 재요청하세요." |
| 자동실행 등록 실패 | 토스트 + 토글 롤백, 에러 메시지 표시 |
| 과거 시각 reschedule 시도 | silent skip (stderr 로그만) |
| DB 마이그레이션 실패 | 기존 동작 유지 — panic + "백업 복원 권장" README 링크 |
| 플러그인 미지원 플랫폼 | 현재 v0.3.0 macOS 전용 빌드 — non-macOS 코드 경로는 `#[cfg(target_os = "macos")]` 가드 |

## Testing

### Rust 유닛 테스트

`cmd_notify` 에 pure-function 테스트:

- `compute_at(date: &str, time: &str, offset_min: i64) -> Result<DateTime<Local>, _>` — 정상 케이스, 24시 경계, 월말 경계, 파싱 실패.
- `notification_id(schedule_id: i64, kind: ReminderKind) -> i32` — 결정적 매핑.
- `should_skip_past(now, at)` — 분기.

`cmd_schedules` 통합 테스트:

- ALTER TABLE 마이그레이션 후 기존 스키마 행이 정상 읽히는지.
- 새 필드 저장/조회 round-trip.

### 프론트엔드 Vitest

`ScheduleModal.test.tsx`:

- 초기 상태: 알림 토글 OFF → 시간 필드 미렌더.
- 토글 ON → 시간 필드 & 체크박스 렌더, 기본값 `time="09:00", remind_before_5min=true`.
- 편집 모드: 기존 스케줄의 time 있는 것 → notify=true.
- Enter 키 + isComposing=true → preventDefault 안 됨 (IME 커밋 보존).
- Enter 키 + isComposing=false → `requestSubmit` 호출.

`SettingsGeneralSection.test.tsx`:

- autostart 토글 → `set_autostart(true)` 호출.
- 권한 상태 `denied` 렌더링 → "시스템 설정" 안내 텍스트.

### 수동 검증 체크리스트 (PR 설명용)

- [ ] 5분 전 알림 발송 — 6분 뒤 일정 등록해 6분 대기.
- [ ] 정각 알림 발송 — 1분 뒤 일정 등록해 1분 대기.
- [ ] 앱 종료 후 알림 발송 — `Cmd+Q` 종료 상태에서 알림 시각까지 대기.
- [ ] 로그인 자동 실행 후 창 숨겨짐 → Dock 클릭 → 창 표시.
- [ ] 한글 입력 + Enter → 저장 성공 (기존 버그 회귀 확인).
- [ ] 기존 v0.2.2 DB 로 시작 → 마이그레이션 성공, 스케줄 전부 로드.
- [ ] 알림 권한 최초 요청 시 macOS 프롬프트 출현.
- [ ] 권한 거부 상태에서 알림 토글 켜면 토스트 안내.

## Migration / Rollout

- **DB**: `PRAGMA table_info` 기반 idempotent ALTER. 기존 사용자 영향 없음.
- **버전**: `0.2.2 → 0.3.0` (minor bump — 새 기능 3개 + 스키마 확장).
- **백업**: `auto_backup_on_close` 가 이미 매 종료마다 수행. 릴리즈 노트에 "0.2.x 에서 0.3.0 업그레이드 시 설정 → 백업 → 지금 백업 권장" 추가.
- **권한 재요청**: 기존 사용자는 알림 권한이 없는 상태로 업그레이드. 최초 알림-활성 일정 저장 시 프롬프트.

## Files Touched

| File | Change |
|---|---|
| `src-tauri/Cargo.toml` | +2 deps (autostart, notification) |
| `src-tauri/capabilities/default.json` | +2 permissions |
| `src-tauri/tauri.conf.json` | window `visible: false` |
| `src-tauri/src/lib.rs` | plugins init + hidden-launch + Reopen + boot reschedule |
| `src-tauri/src/db.rs` | ALTER TABLE 마이그레이션 |
| `src-tauri/src/models.rs` | `Schedule` 구조체 확장 |
| `src-tauri/src/cmd_schedules.rs` | Input/Row 매핑 + notify hook |
| `src-tauri/src/cmd_notify.rs` | **신규** — 스케줄링 + 부팅 리스토어 |
| `src-tauri/src/cmd_autostart.rs` | **신규** — 토글 래퍼 |
| `src-tauri/src/cmd_settings.rs` | `notifications.permission` 헬퍼 |
| `src/types.ts` | `Schedule` + `ScheduleInput` 확장 |
| `src/api.ts` | notify/autostart invoke 헬퍼 |
| `src/components/ScheduleModal.tsx` | 토글 + 타임 피커 + 체크박스 + Enter 핸들러 |
| `src/components/SettingsDialog.tsx` | "general" 탭 추가 |
| `src/components/SettingsGeneralSection.tsx` | **신규** |
| `src/components/CalendarView.tsx` | 🔔 prefix in title |
| `src/hooks/useAutostart.ts` | **신규** (선택) |

## Open Questions

없음 — Q1 (알림 대상 범위 = 토글 기반 옵트인), Q2 (자동실행 시 백그라운드) 모두 확정.
