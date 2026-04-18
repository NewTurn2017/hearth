# Schedule Notifications + Autostart (v0.3.0) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship v0.3.0 with three user-facing additions and one bug fix — macOS auto-start on login (hidden), native time picker with opt-in notification toggle, 5-min-before and on-time desktop notifications, plus a fix for the Korean IME Enter-swallow bug in the schedule save flow.

**Architecture:** DB is the source of truth for notifications; on boot we cancel and re-schedule all future reminders. `tauri-plugin-autostart` handles Login Items (silent `--hidden` flag), `tauri-plugin-notification` handles scheduled macOS user notifications. Two new bool columns on `schedules` carry reminder preferences. ScheduleModal gains a `notify` toggle that gates the time picker + two reminder checkboxes. A new "일반" settings tab exposes autostart + notification permission.

**Tech Stack:** Tauri 2 (Rust), tauri-plugin-autostart, tauri-plugin-notification, React 19 + TypeScript, rusqlite, chrono, Vitest, `cargo test`.

**Spec:** `docs/superpowers/specs/2026-04-18-schedule-notifications-autostart-design.md`

---

## Phase 0: Pre-flight (verify plugin APIs)

Since plugin API shapes can drift across minor versions, the very first task pins the exact APIs we'll call. All later tasks refer back to the notes captured here.

### Task 1: Verify plugin APIs against docs

**Files:**
- Read-only — no file changes, just documentation capture

- [ ] **Step 1: Resolve plugin doc sources via context7**

Run (the `Skill` equivalent in your environment — any docs tool works):

```
mcp__plugin_context7_context7__resolve-library-id  { libraryName: "tauri-plugin-notification" }
mcp__plugin_context7_context7__resolve-library-id  { libraryName: "tauri-plugin-autostart" }
```

Then:

```
mcp__plugin_context7_context7__query-docs { id: "...", query: "schedule notification at a specific date macOS Rust" }
mcp__plugin_context7_context7__query-docs { id: "...", query: "enable disable isEnabled with args macOS Rust" }
```

- [ ] **Step 2: Capture the exact notification Rust API**

Write a short note inline in `docs/superpowers/plans/2026-04-18-schedule-notifications-autostart.md` at the end of this task — list:

- Plugin crate version (e.g., `2.0.0-rc.*` or `2.x`)
- Builder method for scheduled delivery. Expected shape (Tauri 2 convention):

```rust
use tauri_plugin_notification::{NotificationExt, Schedule};

app.notification()
    .builder()
    .title("…")
    .body("…")
    .id(notification_id)              // i32, stable for cancel()
    .schedule(Schedule::At {
        date: chrono::DateTime<chrono::Utc>,
        allow_while_idle: true,
    })
    .show()?;
```

- Cancel API. Expected: `app.notification().cancel(vec![id1, id2])?;`
- Permission probe: `app.notification().permission_state()` → `PermissionState::{Granted, Denied, Unknown}`.
- Request permission: `app.notification().request_permission()`.

If the real API diverges, update the notes and adjust later tasks accordingly.

- [ ] **Step 3: Capture the exact autostart Rust API**

- Init: `tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--hidden"]))`.
- JS side: `@tauri-apps/plugin-autostart` exports `enable()`, `disable()`, `isEnabled()`. Args passed in `init` are persisted into the generated plist `ProgramArguments`.
- Rust-side enable/disable: via `AutoLaunchManager` extension trait — `app.autolaunch().enable()?;`, `.disable()?`, `.is_enabled()?`.

- [ ] **Step 4: Commit the captured notes**

```bash
git add docs/superpowers/plans/2026-04-18-schedule-notifications-autostart.md
git commit -m "docs(plan): pin tauri plugin API shapes for notifications + autostart"
```

---

## Phase 1: Dependencies + DB migration

### Task 2: Add plugin crates (Rust) and npm packages

**Files:**
- Modify: `src-tauri/Cargo.toml` (dependencies section)
- Modify: `package.json` (dependencies section)

- [ ] **Step 1: Add Rust deps**

Open `src-tauri/Cargo.toml` and add below the existing `tauri-plugin-process = "2"` line:

```toml
tauri-plugin-autostart = "2"
tauri-plugin-notification = "2"
```

- [ ] **Step 2: Add npm deps**

Run:

```bash
cd /Users/genie/dev/tools/hearth
npm install --save @tauri-apps/plugin-autostart@^2 @tauri-apps/plugin-notification@^2
```

- [ ] **Step 3: Sanity-build Rust**

```bash
cd /Users/genie/dev/tools/hearth/src-tauri && cargo build
```

Expected: compiles. Warnings about unused imports for the two new crates are OK — they get used in Task 10.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock package.json package-lock.json
git commit -m "chore(deps): add tauri-plugin-autostart + tauri-plugin-notification"
```

---

### Task 3: DB migration — add reminder columns

**Files:**
- Modify: `src-tauri/src/db.rs`
- Test: `src-tauri/src/db.rs` (add `#[cfg(test)] mod tests`)

- [ ] **Step 1: Write the failing test**

Append to `src-tauri/src/db.rs` (create `#[cfg(test)] mod tests` at end):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn legacy_schema(conn: &Connection) {
        conn.execute_batch(
            "CREATE TABLE schedules (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL,
                time TEXT,
                location TEXT,
                description TEXT,
                notes TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
        .unwrap();
    }

    #[test]
    fn migrates_legacy_schedules_adds_reminder_columns() {
        let conn = Connection::open_in_memory().unwrap();
        legacy_schema(&conn);
        // Seed a legacy row (no reminder cols).
        conn.execute(
            "INSERT INTO schedules (date, time) VALUES ('2026-04-18', '09:00')",
            [],
        )
        .unwrap();

        ensure_schedule_reminder_columns(&conn).unwrap();

        // New columns exist and default to 0.
        let (b5, ba): (i64, i64) = conn
            .query_row(
                "SELECT remind_before_5min, remind_at_start FROM schedules WHERE id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(b5, 0);
        assert_eq!(ba, 0);

        // Running again is a no-op (idempotent).
        ensure_schedule_reminder_columns(&conn).unwrap();
    }
}
```

- [ ] **Step 2: Run the test (expect failure)**

```bash
cd /Users/genie/dev/tools/hearth/src-tauri && cargo test --lib db::tests::migrates_legacy_schedules_adds_reminder_columns -- --nocapture
```

Expected: `error[E0425]: cannot find function 'ensure_schedule_reminder_columns'`.

- [ ] **Step 3: Implement the migration helper**

Add above `fn run_migrations` in `src-tauri/src/db.rs`:

```rust
fn ensure_schedule_reminder_columns(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare("PRAGMA table_info('schedules')")?;
    let cols: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(1))?
        .filter_map(|r| r.ok())
        .collect();
    if !cols.iter().any(|c| c == "remind_before_5min") {
        conn.execute_batch(
            "ALTER TABLE schedules ADD COLUMN remind_before_5min INTEGER NOT NULL DEFAULT 0;",
        )?;
    }
    if !cols.iter().any(|c| c == "remind_at_start") {
        conn.execute_batch(
            "ALTER TABLE schedules ADD COLUMN remind_at_start INTEGER NOT NULL DEFAULT 0;",
        )?;
    }
    Ok(())
}
```

Then call it at the end of `run_migrations`:

```rust
fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        … existing CREATE TABLEs …
        ",
    )?;

    ensure_schedule_reminder_columns(conn)?;   // NEW
    seed_categories_if_empty(conn)?;
    Ok(())
}
```

- [ ] **Step 4: Run the test (expect pass)**

```bash
cd /Users/genie/dev/tools/hearth/src-tauri && cargo test --lib db::tests::migrates_legacy_schedules_adds_reminder_columns -- --nocapture
```

Expected: `test result: ok. 1 passed`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/db.rs
git commit -m "feat(db): idempotent ALTER adds reminder columns to schedules"
```

---

### Task 4: Extend Schedule model + row parsing

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/cmd_schedules.rs`

- [ ] **Step 1: Extend the struct**

Edit `src-tauri/src/models.rs` Schedule to read:

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Schedule {
    pub id: i64,
    pub date: String,
    pub time: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub remind_before_5min: bool,
    pub remind_at_start: bool,
    pub created_at: String,
    pub updated_at: String,
}
```

- [ ] **Step 2: Update SELECT_COLS and row mapper**

Edit `src-tauri/src/cmd_schedules.rs`:

```rust
const SELECT_COLS: &str =
    "id, date, time, location, description, notes, \
     remind_before_5min, remind_at_start, created_at, updated_at";

fn row_to_schedule(row: &rusqlite::Row) -> rusqlite::Result<Schedule> {
    let b5: i64 = row.get(6)?;
    let ba: i64 = row.get(7)?;
    Ok(Schedule {
        id: row.get(0)?,
        date: row.get(1)?,
        time: row.get(2)?,
        location: row.get(3)?,
        description: row.get(4)?,
        notes: row.get(5)?,
        remind_before_5min: b5 != 0,
        remind_at_start: ba != 0,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}
```

- [ ] **Step 3: Extend ScheduleInput + INSERT/UPDATE**

```rust
#[derive(Debug, Deserialize)]
pub struct ScheduleInput {
    pub date: String,
    pub time: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
    #[serde(default)]
    pub remind_before_5min: bool,
    #[serde(default)]
    pub remind_at_start: bool,
}
```

And in `create_schedule`:

```rust
db.execute(
    "INSERT INTO schedules (date, time, location, description, notes, \
     remind_before_5min, remind_at_start) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    rusqlite::params![
        data.date,
        data.time,
        data.location,
        data.description,
        data.notes,
        data.remind_before_5min as i64,
        data.remind_at_start as i64,
    ],
)
.map_err(|e| e.to_string())?;
```

Mirror change in `update_schedule`:

```rust
db.execute(
    "UPDATE schedules SET date=?1, time=?2, location=?3, description=?4, notes=?5, \
     remind_before_5min=?6, remind_at_start=?7, updated_at=datetime('now') WHERE id=?8",
    rusqlite::params![
        data.date,
        data.time,
        data.location,
        data.description,
        data.notes,
        data.remind_before_5min as i64,
        data.remind_at_start as i64,
        id,
    ],
)
.map_err(|e| e.to_string())?;
```

- [ ] **Step 4: Round-trip integration test**

Append to `src-tauri/src/cmd_schedules.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use tempfile::TempDir;

    fn temp_state() -> (TempDir, rusqlite::Connection) {
        let tmp = TempDir::new().unwrap();
        let conn = init_db(&tmp.path().join("t.db")).unwrap();
        (tmp, conn)
    }

    #[test]
    fn insert_select_roundtrips_reminder_flags() {
        let (_tmp, db) = temp_state();
        db.execute(
            "INSERT INTO schedules (date, time, remind_before_5min, remind_at_start) \
             VALUES ('2026-04-20', '10:00', 1, 0)",
            [],
        )
        .unwrap();

        let sched: Schedule = db
            .query_row(
                &format!("SELECT {} FROM schedules WHERE id=1", SELECT_COLS),
                [],
                row_to_schedule,
            )
            .unwrap();

        assert!(sched.remind_before_5min);
        assert!(!sched.remind_at_start);
    }
}
```

Add `tempfile = "3"` to `[dev-dependencies]` of `src-tauri/Cargo.toml` if not already present (check first; many Tauri skeletons include it).

- [ ] **Step 5: Run all schedule tests**

```bash
cd /Users/genie/dev/tools/hearth/src-tauri && cargo test --lib cmd_schedules
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/cmd_schedules.rs src-tauri/Cargo.toml
git commit -m "feat(schedules): carry reminder flags through Rust model + CRUD"
```

---

## Phase 2: Rust notification engine

### Task 5: cmd_notify — pure helpers (TDD)

**Files:**
- Create: `src-tauri/src/cmd_notify.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod cmd_notify;`)

- [ ] **Step 1: Stub the module**

Create `src-tauri/src/cmd_notify.rs`:

```rust
//! Notification scheduling for calendar reminders.
//!
//! DB is the source of truth for reminder state. On boot, cancel everything
//! and re-schedule just the future reminders; on CRUD, cancel the affected
//! schedule's ids and re-apply. No separate job table — the notification id
//! is derived deterministically from (schedule_id, kind).

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReminderKind {
    Before5Min,
    AtStart,
}

impl ReminderKind {
    pub fn offset_minutes(self) -> i64 {
        match self {
            ReminderKind::Before5Min => 5,
            ReminderKind::AtStart => 0,
        }
    }

    /// Second digit of the derived notification id.
    pub fn code(self) -> i32 {
        match self {
            ReminderKind::Before5Min => 1,
            ReminderKind::AtStart => 2,
        }
    }
}

/// Deterministic notification id from a schedule row id + reminder kind.
pub fn notification_id(schedule_id: i64, kind: ReminderKind) -> i32 {
    // i64 schedule ids above INT_MAX/10 would overflow; in practice the SQLite
    // autoincrement on a personal app stays tiny. Clamp defensively.
    let base = (schedule_id as i64).min(i32::MAX as i64 / 10) as i32;
    base * 10 + kind.code()
}

/// Compute the local trigger time for a reminder. Returns `None` if the
/// date/time strings fail to parse or the resulting instant is ambiguous.
pub fn compute_at(
    date: &str,
    time: &str,
    kind: ReminderKind,
) -> Option<DateTime<Local>> {
    let d = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let t = NaiveTime::parse_from_str(time, "%H:%M").ok()?;
    let naive = NaiveDateTime::new(d, t) - chrono::Duration::minutes(kind.offset_minutes());
    Local.from_local_datetime(&naive).single()
}

/// Skip a reminder whose trigger is already in the past.
pub fn should_skip_past(now: DateTime<Local>, at: DateTime<Local>) -> bool {
    at <= now
}
```

Add `mod cmd_notify;` to `src-tauri/src/lib.rs` near the other `mod` lines.

- [ ] **Step 2: Write failing tests**

Append to `src-tauri/src/cmd_notify.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_id_is_deterministic_and_stable() {
        assert_eq!(notification_id(1, ReminderKind::Before5Min), 11);
        assert_eq!(notification_id(1, ReminderKind::AtStart), 12);
        assert_eq!(notification_id(42, ReminderKind::Before5Min), 421);
        assert_eq!(notification_id(42, ReminderKind::AtStart), 422);
    }

    #[test]
    fn compute_at_returns_none_for_garbage() {
        assert!(compute_at("nope", "oops", ReminderKind::AtStart).is_none());
        assert!(compute_at("2026-04-18", "25:99", ReminderKind::AtStart).is_none());
    }

    #[test]
    fn compute_at_subtracts_5_minutes_for_before_kind() {
        let at_start = compute_at("2026-04-18", "10:00", ReminderKind::AtStart).unwrap();
        let before5 = compute_at("2026-04-18", "10:00", ReminderKind::Before5Min).unwrap();
        let diff = at_start.signed_duration_since(before5).num_minutes();
        assert_eq!(diff, 5);
    }

    #[test]
    fn should_skip_past_compares_inclusively() {
        let now = Local
            .from_local_datetime(&NaiveDateTime::parse_from_str(
                "2026-04-18 10:00", "%Y-%m-%d %H:%M").unwrap())
            .single().unwrap();
        let at_past = now - chrono::Duration::minutes(1);
        let at_future = now + chrono::Duration::minutes(1);
        assert!(should_skip_past(now, now));
        assert!(should_skip_past(now, at_past));
        assert!(!should_skip_past(now, at_future));
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cd /Users/genie/dev/tools/hearth/src-tauri && cargo test --lib cmd_notify
```

Expected: 4 passed.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/cmd_notify.rs src-tauri/src/lib.rs
git commit -m "feat(notify): pure helpers for reminder time math + stable ids"
```

---

### Task 6: cmd_notify — schedule/cancel against the plugin

**Files:**
- Modify: `src-tauri/src/cmd_notify.rs`

- [ ] **Step 1: Add plugin-backed apply/cancel**

Append to `src-tauri/src/cmd_notify.rs`:

```rust
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use crate::models::Schedule;

/// Cancel both possible ids for a schedule. Cancelling a non-existent id is
/// a no-op in the plugin.
pub fn cancel_for_id(app: &AppHandle, schedule_id: i64) {
    let ids = vec![
        notification_id(schedule_id, ReminderKind::Before5Min),
        notification_id(schedule_id, ReminderKind::AtStart),
    ];
    if let Err(e) = app.notification().cancel(ids) {
        eprintln!("notification cancel failed for schedule {schedule_id}: {e}");
    }
}

/// Body text shared across both reminder kinds.
fn reminder_body(s: &Schedule) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(d) = s.description.as_deref().filter(|t| !t.is_empty()) {
        parts.push(d.to_string());
    }
    if let Some(l) = s.location.as_deref().filter(|t| !t.is_empty()) {
        parts.push(format!("@ {l}"));
    }
    if parts.is_empty() {
        "일정".to_string()
    } else {
        parts.join(" ")
    }
}

/// Re-apply notifications for a single schedule row. Idempotent: cancels any
/// prior entries first.
pub fn apply_for(app: &AppHandle, s: &Schedule) -> Result<(), String> {
    cancel_for_id(app, s.id);

    let Some(time) = s.time.as_deref().filter(|t| !t.is_empty()) else {
        return Ok(()); // no time → nothing to schedule
    };

    let now = Local::now();
    let mut reqs: Vec<ReminderKind> = Vec::new();
    if s.remind_before_5min { reqs.push(ReminderKind::Before5Min); }
    if s.remind_at_start   { reqs.push(ReminderKind::AtStart); }

    for kind in reqs {
        let Some(at_local) = compute_at(&s.date, time, kind) else { continue };
        if should_skip_past(now, at_local) { continue; }
        let at_utc = at_local.with_timezone(&chrono::Utc);
        let id = notification_id(s.id, kind);
        let title = match kind {
            ReminderKind::Before5Min => "일정 5분 전",
            ReminderKind::AtStart    => "일정 시작",
        };
        app.notification()
            .builder()
            .id(id)
            .title(title)
            .body(reminder_body(s))
            .schedule(tauri_plugin_notification::Schedule::At {
                date: at_utc,
                allow_while_idle: true,
            })
            .show()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

**Note:** If `Step 1 of Task 1` revealed a different API shape (e.g., no `.id()` or different Schedule enum), update the two calls above to match. The body and logic stay identical.

- [ ] **Step 2: Build to catch API mismatches**

```bash
cd /Users/genie/dev/tools/hearth/src-tauri && cargo build --lib
```

Fix until it compiles.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/cmd_notify.rs
git commit -m "feat(notify): apply + cancel reminders via plugin"
```

---

### Task 7: cmd_notify — boot reschedule

**Files:**
- Modify: `src-tauri/src/cmd_notify.rs`

- [ ] **Step 1: Add reschedule_all_future**

Append to `src-tauri/src/cmd_notify.rs`:

```rust
use tauri::Manager;
use crate::AppState;

/// Walk the DB, cancel every schedule with any reminder flag set, then
/// re-apply just the future ones. Runs once at boot.
pub fn reschedule_all_future(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Pull everything with any reminder flag, past or future — so we can
    // cancel stale future entries from a prior boot before re-applying.
    let mut stmt = db
        .prepare(
            "SELECT id, date, time, location, description, notes, \
             remind_before_5min, remind_at_start, created_at, updated_at \
             FROM schedules \
             WHERE remind_before_5min = 1 OR remind_at_start = 1",
        )
        .map_err(|e| e.to_string())?;

    let rows: Vec<Schedule> = stmt
        .query_map([], |row| {
            let b5: i64 = row.get(6)?;
            let ba: i64 = row.get(7)?;
            Ok(Schedule {
                id: row.get(0)?,
                date: row.get(1)?,
                time: row.get(2)?,
                location: row.get(3)?,
                description: row.get(4)?,
                notes: row.get(5)?,
                remind_before_5min: b5 != 0,
                remind_at_start: ba != 0,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    drop(db);

    for s in rows {
        if let Err(e) = apply_for(app, &s) {
            eprintln!("reschedule failed for {}: {}", s.id, e);
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/cmd_notify.rs
git commit -m "feat(notify): reschedule_all_future walks DB to restore reminders"
```

---

### Task 8: Wire CRUD hooks + permission commands

**Files:**
- Modify: `src-tauri/src/cmd_schedules.rs`
- Modify: `src-tauri/src/cmd_notify.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Call apply/cancel from CRUD handlers**

Edit `cmd_schedules.rs`. Change signatures to take `AppHandle`:

```rust
use tauri::{AppHandle, State};

#[tauri::command]
pub fn create_schedule(
    app: AppHandle,
    state: State<'_, AppState>,
    data: ScheduleInput,
) -> Result<Schedule, String> {
    // … existing INSERT + last_insert_rowid + query_row …
    let sched = /* the query_row result */;
    crate::cmd_notify::apply_for(&app, &sched).ok();
    Ok(sched)
}

#[tauri::command]
pub fn update_schedule(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    data: ScheduleInput,
) -> Result<Schedule, String> {
    // … existing UPDATE + query_row …
    let sched = /* the query_row result */;
    crate::cmd_notify::apply_for(&app, &sched).ok();
    Ok(sched)
}

#[tauri::command]
pub fn delete_schedule(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute("DELETE FROM schedules WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    drop(db);
    crate::cmd_notify::cancel_for_id(&app, id);
    Ok(())
}
```

- [ ] **Step 2: Add permission-state + request commands to cmd_notify**

Append to `src-tauri/src/cmd_notify.rs`:

```rust
#[tauri::command]
pub async fn notifications_permission(app: AppHandle) -> Result<String, String> {
    match app.notification().permission_state() {
        Ok(tauri_plugin_notification::PermissionState::Granted) => Ok("granted".into()),
        Ok(tauri_plugin_notification::PermissionState::Denied)  => Ok("denied".into()),
        Ok(_) => Ok("unknown".into()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn notifications_request(app: AppHandle) -> Result<String, String> {
    match app.notification().request_permission() {
        Ok(tauri_plugin_notification::PermissionState::Granted) => Ok("granted".into()),
        Ok(tauri_plugin_notification::PermissionState::Denied)  => Ok("denied".into()),
        Ok(_) => Ok("unknown".into()),
        Err(e) => Err(e.to_string()),
    }
}
```

Adjust if the plugin exposes a different return type (see Task 1 notes).

- [ ] **Step 3: Register commands**

In `src-tauri/src/lib.rs` `invoke_handler!` macro, add:

```rust
cmd_notify::notifications_permission,
cmd_notify::notifications_request,
```

- [ ] **Step 4: Build**

```bash
cd /Users/genie/dev/tools/hearth/src-tauri && cargo build --lib
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/cmd_schedules.rs src-tauri/src/cmd_notify.rs src-tauri/src/lib.rs
git commit -m "feat(notify): fire apply/cancel from schedule CRUD + permission cmds"
```

---

## Phase 3: Autostart

### Task 9: cmd_autostart module

**Files:**
- Create: `src-tauri/src/cmd_autostart.rs`
- Modify: `src-tauri/src/cmd_settings.rs` (add `K_AUTOSTART` constant)
- Modify: `src-tauri/src/lib.rs` (`mod cmd_autostart;`)

- [ ] **Step 1: Add the K_AUTOSTART key**

Edit `src-tauri/src/cmd_settings.rs`, inside the constants block near the top:

```rust
pub(crate) const K_AUTOSTART: &str = "autostart.enabled";
pub(crate) const K_NOTIF_PERMISSION: &str = "notifications.permission";
```

- [ ] **Step 2: Create the autostart commands**

Create `src-tauri/src/cmd_autostart.rs`:

```rust
use crate::cmd_settings;
use crate::AppState;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt;

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> Result<bool, String> {
    let enabled = app
        .autolaunch()
        .is_enabled()
        .map_err(|e| e.to_string())?;
    Ok(enabled)
}

#[tauri::command]
pub fn set_autostart(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    if enabled {
        app.autolaunch().enable().map_err(|e| e.to_string())?;
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())?;
    }
    // Mirror into the KV cache so UI first-paint can show the state without
    // round-tripping through the plugin.
    let db = state.db.lock().map_err(|e| e.to_string())?;
    cmd_settings::write(&db, cmd_settings::K_AUTOSTART, if enabled { "1" } else { "0" })?;
    Ok(())
}
```

Make `cmd_settings::write` and `cmd_settings::K_AUTOSTART` / `K_NOTIF_PERMISSION` accessible — they already live under `pub(crate)`, so they work as-is.

- [ ] **Step 3: Register the module + commands**

`src-tauri/src/lib.rs`:

```rust
mod cmd_autostart;
```

And in `invoke_handler!`:

```rust
cmd_autostart::get_autostart,
cmd_autostart::set_autostart,
```

- [ ] **Step 4: Build**

```bash
cd /Users/genie/dev/tools/hearth/src-tauri && cargo build --lib
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/cmd_autostart.rs src-tauri/src/cmd_settings.rs src-tauri/src/lib.rs
git commit -m "feat(autostart): get/set commands + settings KV cache"
```

---

### Task 10: lib.rs — plugins, hidden-launch, Reopen handler

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/tauri.conf.json` (window `visible: false`)
- Modify: `src-tauri/capabilities/default.json` (+ 2 perms)

- [ ] **Step 1: Flip main window to start hidden**

Edit `src-tauri/tauri.conf.json`:

```json
"windows": [
  {
    "title": "Hearth",
    "width": 1200,
    "height": 800,
    "minWidth": 900,
    "minHeight": 600,
    "visible": false
  }
]
```

- [ ] **Step 2: Grant the new capabilities**

Edit `src-tauri/capabilities/default.json` permissions array to include:

```json
"autostart:default",
"notification:default"
```

Final shape:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "opener:default",
    "dialog:default",
    "updater:default",
    "process:allow-restart",
    "autostart:default",
    "notification:default"
  ]
}
```

- [ ] **Step 3: Wire plugins in lib.rs**

Inside `pub fn run()`, add to the builder chain (after the existing `.plugin(tauri_plugin_process::init())` line):

```rust
.plugin(tauri_plugin_autostart::init(
    tauri_plugin_autostart::MacosLauncher::LaunchAgent,
    Some(vec!["--hidden".to_string()]),
))
.plugin(tauri_plugin_notification::init())
```

- [ ] **Step 4: Handle hidden launch in setup()**

Extend the `.setup(|app| { ... })` block. After `app.manage(AppState { ... })` and before `Ok(())`:

```rust
let launched_hidden = std::env::args().any(|a| a == "--hidden");
if !launched_hidden {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

// Kick the boot reschedule on a background task so DB/permission work
// doesn't block window paint.
let app_handle = app.handle().clone();
tauri::async_runtime::spawn(async move {
    if let Err(e) = crate::cmd_notify::reschedule_all_future(&app_handle) {
        eprintln!("notification boot reschedule failed: {e}");
    }
});
```

- [ ] **Step 5: Handle Dock Reopen**

Replace the final line:

```rust
.run(tauri::generate_context!())
.expect("error while running tauri application");
```

with:

```rust
.build(tauri::generate_context!())
.expect("error while building tauri application")
.run(|app_handle, event| {
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

- [ ] **Step 6: Build + basic smoke**

```bash
cd /Users/genie/dev/tools/hearth && npm run tauri dev
```

Expected: Hearth opens normally, no regressions. Close the window with the red button (or `Cmd+W`), then re-open via Dock — window should come back.

To test hidden boot:

```bash
cd /Users/genie/dev/tools/hearth/src-tauri/target/debug
./hearth --hidden &
```

Window stays hidden. `Dock` click reveals it.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/tauri.conf.json src-tauri/capabilities/default.json
git commit -m "feat(app): init autostart+notification plugins, hidden launch, Reopen"
```

---

## Phase 4: Frontend types + API

### Task 11: TS types + api helpers

**Files:**
- Modify: `src/types.ts`
- Modify: `src/api.ts`

- [ ] **Step 1: Extend Schedule type**

Edit `src/types.ts`:

```ts
export interface Schedule {
  id: number;
  date: string;
  time: string | null;
  location: string | null;
  description: string | null;
  notes: string | null;
  remind_before_5min: boolean;
  remind_at_start: boolean;
  created_at: string;
  updated_at: string;
}
```

- [ ] **Step 2: Widen api.ts create/update**

Edit `src/api.ts`:

```ts
type ScheduleInput = {
  date: string;
  time?: string;
  location?: string;
  description?: string;
  notes?: string;
  remind_before_5min?: boolean;
  remind_at_start?: boolean;
};

export const createSchedule = (data: ScheduleInput) =>
  invoke<Schedule>("create_schedule", { data });

export const updateSchedule = (id: number, data: ScheduleInput) =>
  invoke<Schedule>("update_schedule", { id, data });
```

(Replace the inline type-literal signatures with the named `ScheduleInput`.)

- [ ] **Step 3: Add new endpoints**

Append to `src/api.ts`:

```ts
// Autostart (tauri-plugin-autostart toggle, persisted via Login Items).
export const getAutostart = () => invoke<boolean>("get_autostart");
export const setAutostart = (enabled: boolean) =>
  invoke<void>("set_autostart", { enabled });

// Notification permission probe + request.
export type NotificationPermission = "granted" | "denied" | "unknown";
export const notificationsPermission = () =>
  invoke<NotificationPermission>("notifications_permission");
export const notificationsRequest = () =>
  invoke<NotificationPermission>("notifications_request");
```

- [ ] **Step 4: Type-check**

```bash
cd /Users/genie/dev/tools/hearth && npm run build
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/types.ts src/api.ts
git commit -m "feat(api): TS types+invoke helpers for reminders + autostart"
```

---

## Phase 5: Schedule UI

### Task 12: ScheduleModal — notify toggle + time picker (TDD)

**Files:**
- Modify: `src/components/ScheduleModal.tsx`
- Create: `src/components/__tests__/ScheduleModal.test.tsx`

- [ ] **Step 1: Write failing tests**

Create `src/components/__tests__/ScheduleModal.test.tsx`:

```tsx
import "@testing-library/jest-dom";
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ScheduleModal } from "../ScheduleModal";

describe("ScheduleModal notify toggle", () => {
  it("hides the time picker when notify is off", () => {
    render(
      <ScheduleModal onSave={vi.fn()} onClose={vi.fn()} initialDate="2026-04-20" />
    );
    expect(screen.queryByLabelText("시간")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("5분 전")).not.toBeInTheDocument();
  });

  it("reveals time picker + checkboxes when notify is turned on", () => {
    render(
      <ScheduleModal onSave={vi.fn()} onClose={vi.fn()} initialDate="2026-04-20" />
    );
    fireEvent.click(screen.getByLabelText("알림 받기"));
    expect(screen.getByLabelText("시간")).toBeInTheDocument();
    expect(screen.getByLabelText("5분 전")).toBeChecked();
    expect(screen.getByLabelText("정각")).not.toBeChecked();
  });

  it("emits notify fields on save when toggle is on", () => {
    const onSave = vi.fn();
    render(
      <ScheduleModal onSave={onSave} onClose={vi.fn()} initialDate="2026-04-20" />
    );
    fireEvent.click(screen.getByLabelText("알림 받기"));
    fireEvent.click(screen.getByRole("button", { name: "저장" }));
    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({
        date: "2026-04-20",
        time: "09:00",
        remind_before_5min: true,
        remind_at_start: false,
      })
    );
  });

  it("omits time + flags when toggle stays off", () => {
    const onSave = vi.fn();
    render(
      <ScheduleModal onSave={onSave} onClose={vi.fn()} initialDate="2026-04-20" />
    );
    fireEvent.click(screen.getByRole("button", { name: "저장" }));
    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({
        date: "2026-04-20",
        time: undefined,
        remind_before_5min: false,
        remind_at_start: false,
      })
    );
  });

  it("hydrates notify=true when editing a schedule with time", () => {
    render(
      <ScheduleModal
        onSave={vi.fn()}
        onClose={vi.fn()}
        schedule={{
          id: 1,
          date: "2026-04-20",
          time: "10:00",
          location: null,
          description: null,
          notes: null,
          remind_before_5min: true,
          remind_at_start: false,
          created_at: "",
          updated_at: "",
        }}
      />
    );
    expect(screen.getByLabelText("알림 받기")).toBeChecked();
    expect(screen.getByLabelText("시간")).toHaveValue("10:00");
    expect(screen.getByLabelText("5분 전")).toBeChecked();
  });
});
```

- [ ] **Step 2: Run tests — expect failures**

```bash
cd /Users/genie/dev/tools/hearth && npm test -- --run src/components/__tests__/ScheduleModal.test.tsx
```

Expected: all 5 fail.

- [ ] **Step 3: Rewrite ScheduleModal**

Replace `src/components/ScheduleModal.tsx` entirely with:

```tsx
import { useState } from "react";
import { Trash2 } from "lucide-react";
import type { Schedule } from "../types";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";

type SaveData = {
  date: string;
  time?: string;
  location?: string;
  description?: string;
  notes?: string;
  remind_before_5min?: boolean;
  remind_at_start?: boolean;
};

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

export function ScheduleModal({
  schedule,
  initialDate,
  onSave,
  onDelete,
  onClose,
}: {
  schedule?: Schedule;
  initialDate?: string;
  onSave: (data: SaveData) => void;
  onDelete?: () => void;
  onClose: () => void;
}) {
  const initialNotify =
    !!schedule && (
      !!schedule.time ||
      schedule.remind_before_5min ||
      schedule.remind_at_start
    );

  const [date, setDate] = useState(schedule?.date ?? initialDate ?? "");
  const [notify, setNotify] = useState(initialNotify);
  const [time, setTime] = useState(
    schedule?.time ?? (initialNotify ? "09:00" : "")
  );
  const [remindBefore5, setRemindBefore5] = useState(
    schedule?.remind_before_5min ?? true
  );
  const [remindAtStart, setRemindAtStart] = useState(
    schedule?.remind_at_start ?? false
  );
  const [location, setLocation] = useState(schedule?.location ?? "");
  const [description, setDescription] = useState(schedule?.description ?? "");
  const [notes, setNotes] = useState(schedule?.notes ?? "");

  const isEdit = !!schedule;
  const timeMissing = notify && !time;

  function toggleNotify() {
    const next = !notify;
    setNotify(next);
    if (next && !time) setTime("09:00");
    if (next && !remindBefore5 && !remindAtStart) setRemindBefore5(true);
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!date || timeMissing) return;
    onSave({
      date,
      time: notify ? time : undefined,
      location: location || undefined,
      description: description || undefined,
      notes: notes || undefined,
      remind_before_5min: notify ? remindBefore5 : false,
      remind_at_start: notify ? remindAtStart : false,
    });
  }

  return (
    <Dialog open onClose={onClose} labelledBy="schedule-title">
      <form onSubmit={handleSubmit}>
        <h2
          id="schedule-title"
          className="text-heading text-[var(--color-text-hi)] mb-4"
        >
          일정 {isEdit ? "수정" : "추가"}
        </h2>

        <div className="flex flex-col gap-3">
          <div>
            <label className="text-[11px] text-[var(--color-text-muted)] mb-1 block">
              날짜
            </label>
            <Input
              type="date"
              value={date}
              onChange={(e) => setDate(e.target.value)}
              onKeyDown={onEnterSubmit}
              required
            />
          </div>

          <label className="flex items-center gap-2 text-[13px] select-none">
            <input
              type="checkbox"
              checked={notify}
              onChange={toggleNotify}
              aria-label="알림 받기"
            />
            <span>알림 받기</span>
          </label>

          {notify && (
            <>
              <div>
                <label
                  htmlFor="schedule-time"
                  className="text-[11px] text-[var(--color-text-muted)] mb-1 block"
                >
                  시간
                </label>
                <Input
                  id="schedule-time"
                  type="time"
                  value={time}
                  onChange={(e) => setTime(e.target.value)}
                  onKeyDown={onEnterSubmit}
                  aria-label="시간"
                />
              </div>
              <div className="flex gap-4 text-[13px]">
                <label className="flex items-center gap-1.5 select-none">
                  <input
                    type="checkbox"
                    checked={remindBefore5}
                    onChange={(e) => setRemindBefore5(e.target.checked)}
                    aria-label="5분 전"
                  />
                  <span>5분 전</span>
                </label>
                <label className="flex items-center gap-1.5 select-none">
                  <input
                    type="checkbox"
                    checked={remindAtStart}
                    onChange={(e) => setRemindAtStart(e.target.checked)}
                    aria-label="정각"
                  />
                  <span>정각</span>
                </label>
              </div>
            </>
          )}

          <div>
            <label className="text-[11px] text-[var(--color-text-muted)] mb-1 block">
              장소
            </label>
            <Input
              type="text"
              value={location}
              onChange={(e) => setLocation(e.target.value)}
              onKeyDown={onEnterSubmit}
            />
          </div>
          <div>
            <label className="text-[11px] text-[var(--color-text-muted)] mb-1 block">
              내용
            </label>
            <Input
              type="text"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              onKeyDown={onEnterSubmit}
            />
          </div>
          <div>
            <label className="text-[11px] text-[var(--color-text-muted)] mb-1 block">
              비고
            </label>
            <Input
              type="text"
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              onKeyDown={onEnterSubmit}
            />
          </div>

          {timeMissing && (
            <div className="text-[11px] text-[var(--color-danger)]">
              시간을 입력해 주세요.
            </div>
          )}
        </div>

        <div className="flex justify-between mt-5">
          <div>
            {isEdit && onDelete && (
              <Button
                type="button"
                variant="danger"
                size="sm"
                leftIcon={Trash2}
                onClick={onDelete}
              >
                삭제
              </Button>
            )}
          </div>
          <div className="flex gap-2">
            <Button type="button" variant="secondary" onClick={onClose}>
              취소
            </Button>
            <Button type="submit" variant="primary" disabled={timeMissing}>
              저장
            </Button>
          </div>
        </div>
      </form>
    </Dialog>
  );
}
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cd /Users/genie/dev/tools/hearth && npm test -- --run src/components/__tests__/ScheduleModal.test.tsx
```

Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add src/components/ScheduleModal.tsx src/components/__tests__/ScheduleModal.test.tsx
git commit -m "feat(schedule-modal): notify toggle + native time picker + reminder boxes"
```

---

### Task 13: IME-safe Enter submit — explicit test

**Files:**
- Modify: `src/components/__tests__/ScheduleModal.test.tsx`

- [ ] **Step 1: Add IME tests**

Append to the `describe("ScheduleModal notify toggle", ...)` file, below the existing cases, add a second describe:

```tsx
describe("ScheduleModal IME-safe Enter", () => {
  it("submits on Enter when composition is not active", () => {
    const onSave = vi.fn();
    render(<ScheduleModal onSave={onSave} onClose={vi.fn()} initialDate="2026-04-20" />);
    const location = screen.getByLabelText("장소");
    // fireEvent.keyDown exposes the synthetic KeyboardEvent; jsdom default
    // isComposing=false / keyCode=13 for Enter.
    fireEvent.keyDown(location, { key: "Enter", code: "Enter" });
    expect(onSave).toHaveBeenCalled();
  });

  it("does NOT submit on Enter during IME composition", () => {
    const onSave = vi.fn();
    render(<ScheduleModal onSave={onSave} onClose={vi.fn()} initialDate="2026-04-20" />);
    const location = screen.getByLabelText("장소");
    fireEvent.keyDown(location, {
      key: "Enter",
      code: "Enter",
      keyCode: 229,   // WebKit legacy marker for IME-in-progress
      isComposing: true,
    });
    expect(onSave).not.toHaveBeenCalled();
  });
});
```

You'll need to expose `aria-label="장소"` etc. on each Input in ScheduleModal — update the JSX:

```tsx
<Input … aria-label="장소" />   // in 장소 field
<Input … aria-label="내용" />   // in 내용
<Input … aria-label="비고" />   // in 비고
```

- [ ] **Step 2: Run tests**

```bash
cd /Users/genie/dev/tools/hearth && npm test -- --run src/components/__tests__/ScheduleModal.test.tsx
```

Expected: all passing (including the 2 new IME cases).

- [ ] **Step 3: Commit**

```bash
git add src/components/ScheduleModal.tsx src/components/__tests__/ScheduleModal.test.tsx
git commit -m "test(schedule-modal): IME-safe Enter submit coverage"
```

---

### Task 14: CalendarView — bell prefix on notified events

**Files:**
- Modify: `src/components/CalendarView.tsx`

- [ ] **Step 1: Update title mapping**

In `CalendarView.tsx`, locate the `events: CalendarEvent[] = useMemo(...)` block and replace the `title` line:

```ts
const hasReminder = s.remind_before_5min || s.remind_at_start;
return {
  id: s.id,
  title:
    (hasReminder ? "🔔 " : "") +
    ([s.description, s.location].filter(Boolean).join(" @ ") || "일정"),
  start,
  end,
  resource: s,
};
```

- [ ] **Step 2: Smoke-build**

```bash
cd /Users/genie/dev/tools/hearth && npm run build
```

Expected: build passes.

- [ ] **Step 3: Commit**

```bash
git add src/components/CalendarView.tsx
git commit -m "feat(calendar): 🔔 prefix marks events with reminders"
```

---

## Phase 6: Settings UI

### Task 15: SettingsGeneralSection (autostart + notification status)

**Files:**
- Create: `src/components/SettingsGeneralSection.tsx`
- Create: `src/components/__tests__/SettingsGeneralSection.test.tsx`

- [ ] **Step 1: Failing test**

Create `src/components/__tests__/SettingsGeneralSection.test.tsx`:

```tsx
import "@testing-library/jest-dom";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { SettingsGeneralSection } from "../SettingsGeneralSection";

vi.mock("../../api", () => ({
  getAutostart: vi.fn().mockResolvedValue(false),
  setAutostart: vi.fn().mockResolvedValue(undefined),
  notificationsPermission: vi.fn().mockResolvedValue("unknown"),
  notificationsRequest: vi.fn().mockResolvedValue("granted"),
}));

import * as api from "../../api";

describe("SettingsGeneralSection", () => {
  beforeEach(() => { vi.clearAllMocks(); });

  it("reads initial autostart state and toggles via set_autostart", async () => {
    render(<SettingsGeneralSection active />);
    const toggle = await screen.findByLabelText("로그인 시 Hearth 자동 실행");
    expect(toggle).not.toBeChecked();
    fireEvent.click(toggle);
    await waitFor(() =>
      expect(api.setAutostart).toHaveBeenCalledWith(true)
    );
  });

  it("fires notifications_request when the permission button is clicked", async () => {
    render(<SettingsGeneralSection active />);
    const btn = await screen.findByRole("button", { name: "권한 요청" });
    fireEvent.click(btn);
    await waitFor(() =>
      expect(api.notificationsRequest).toHaveBeenCalled()
    );
  });
});
```

- [ ] **Step 2: Run failing**

```bash
npm test -- --run src/components/__tests__/SettingsGeneralSection.test.tsx
```

Expected: file cannot be imported.

- [ ] **Step 3: Implement the component**

Create `src/components/SettingsGeneralSection.tsx`:

```tsx
import { useEffect, useState } from "react";
import { Button } from "../ui/Button";
import * as api from "../api";
import type { NotificationPermission } from "../api";

export function SettingsGeneralSection({ active }: { active: boolean }) {
  const [autostart, setAutostartState] = useState<boolean>(false);
  const [perm, setPerm] = useState<NotificationPermission>("unknown");
  const [busy, setBusy] = useState(false);

  async function refresh() {
    try {
      const [a, p] = await Promise.all([
        api.getAutostart(),
        api.notificationsPermission(),
      ]);
      setAutostartState(a);
      setPerm(p);
    } catch (e) {
      console.error("general settings load failed:", e);
    }
  }

  useEffect(() => {
    if (active) refresh();
  }, [active]);

  async function toggleAutostart(next: boolean) {
    setBusy(true);
    try {
      await api.setAutostart(next);
      setAutostartState(next);
    } catch (e) {
      console.error("set_autostart failed:", e);
    } finally {
      setBusy(false);
    }
  }

  async function requestPerm() {
    setBusy(true);
    try {
      const p = await api.notificationsRequest();
      setPerm(p);
    } catch (e) {
      console.error("notifications_request failed:", e);
    } finally {
      setBusy(false);
    }
  }

  const permLabel = {
    granted: "허용됨",
    denied: "차단됨",
    unknown: "미요청",
  }[perm];

  return (
    <div className="flex flex-col gap-6">
      <section>
        <h3 className="text-[13px] text-[var(--color-text-hi)] mb-2">자동 시작</h3>
        <label className="flex items-center gap-2 text-[13px]">
          <input
            type="checkbox"
            checked={autostart}
            disabled={busy}
            onChange={(e) => toggleAutostart(e.target.checked)}
            aria-label="로그인 시 Hearth 자동 실행"
          />
          <span>로그인 시 Hearth 자동 실행 (백그라운드에서 조용히 시작)</span>
        </label>
      </section>

      <section>
        <h3 className="text-[13px] text-[var(--color-text-hi)] mb-2">알림</h3>
        <div className="flex items-center gap-3 text-[13px]">
          <span>상태: {permLabel}</span>
          {perm !== "granted" && (
            <Button
              size="sm"
              variant="secondary"
              onClick={requestPerm}
              disabled={busy}
            >
              권한 요청
            </Button>
          )}
        </div>
        {perm === "denied" && (
          <p className="text-[11px] text-[var(--color-text-muted)] mt-2">
            macOS 시스템 설정 → 알림 → Hearth 에서 허용으로 변경해 주세요.
          </p>
        )}
      </section>
    </div>
  );
}
```

- [ ] **Step 4: Run tests — expect pass**

```bash
npm test -- --run src/components/__tests__/SettingsGeneralSection.test.tsx
```

Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add src/components/SettingsGeneralSection.tsx src/components/__tests__/SettingsGeneralSection.test.tsx
git commit -m "feat(settings): general tab with autostart toggle + notification perm"
```

---

### Task 16: Wire the new tab into SettingsDialog

**Files:**
- Modify: `src/components/SettingsDialog.tsx`

- [ ] **Step 1: Add 'general' to the tabs**

Edit `src/components/SettingsDialog.tsx`:

```tsx
import { SettingsGeneralSection } from "./SettingsGeneralSection";

type TabKey = "general" | "ai" | "backup" | "categories";

const TABS: { key: TabKey; label: string }[] = [
  { key: "general", label: "일반" },
  { key: "ai", label: "AI" },
  { key: "backup", label: "백업" },
  { key: "categories", label: "카테고리" },
];

export function SettingsDialog({
  open,
  onClose,
  initialTab = "general",
}: {
  open: boolean;
  onClose: () => void;
  initialTab?: TabKey;
}) {
  const [tab, setTab] = useState<TabKey>(initialTab);

  return (
    <Dialog …>
      {/* existing h2 + tablist … */}

      <div className={tab === "general" ? "" : "hidden"}>
        <SettingsGeneralSection active={tab === "general"} />
      </div>
      <div className={tab === "ai" ? "" : "hidden"}>
        <SettingsAiSection active={tab === "ai"} />
      </div>
      <div className={tab === "backup" ? "" : "hidden"}>
        <SettingsBackupSection active={tab === "backup"} />
      </div>
      <div className={tab === "categories" ? "" : "hidden"}>
        <SettingsCategoriesSection />
      </div>

      {/* close button … */}
    </Dialog>
  );
}
```

- [ ] **Step 2: Smoke build + full test run**

```bash
cd /Users/genie/dev/tools/hearth && npm run build && npm test
```

Expected: build passes, all vitest suites green.

- [ ] **Step 3: Commit**

```bash
git add src/components/SettingsDialog.tsx
git commit -m "feat(settings): expose new general tab as default landing"
```

---

## Phase 7: Version bump + manual verification

### Task 17: Bump to 0.3.0 and update CHANGELOG

**Files:**
- Modify: `package.json`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Atomic version bump**

Run the existing release helper (per repo convention):

```bash
cd /Users/genie/dev/tools/hearth
# Inspect scripts/release.sh first — if it has a "bump-only" mode, use that.
# Otherwise, do it manually:
```

Manual bump:

- `package.json`: `"version": "0.3.0"`
- `src-tauri/Cargo.toml`: `version = "0.3.0"`
- `src-tauri/tauri.conf.json`: `"version": "0.3.0"`

Then:

```bash
cd /Users/genie/dev/tools/hearth/src-tauri && cargo build
```

(Refreshes Cargo.lock.)

- [ ] **Step 2: Append CHANGELOG entry**

Add under a new `## 0.3.0 — 2026-04-18` heading in `CHANGELOG.md`:

```markdown
## 0.3.0 — 2026-04-18

### Added
- **로그인 시 자동실행** (설정 → 일반). 백그라운드 숨김 시작, Dock 클릭 시 창 복원.
- **스케줄 알림**: 일정에 "알림 받기" 토글이 생겼고, 켜면 네이티브 시간 피커가 나타납니다. "5분 전" / "정각" 두 가지 오프셋을 독립적으로 선택 가능. 앱을 종료해도 알림은 macOS 시스템이 발송합니다.
- 캘린더 뷰에서 알림이 켜진 일정 앞에 🔔 아이콘.

### Fixed
- 한글로 입력한 뒤 Enter 로 저장할 때 저장되지 않던 문제 (IME composition이 첫 Enter를 소비).

### Migration
- `schedules` 테이블에 `remind_before_5min`, `remind_at_start` 컬럼 추가. 기존 행은 모두 `0 / 0` (알림 꺼짐) 상태로 유지.
```

- [ ] **Step 3: Commit**

```bash
git add package.json package-lock.json src-tauri/Cargo.toml src-tauri/Cargo.lock \
        src-tauri/tauri.conf.json CHANGELOG.md
git commit -m "chore: bump to 0.3.0 (autostart + notifications + IME fix)"
```

---

### Task 18: Manual verification

**Files:** None — operational only.

- [ ] **Step 1: Full dev-mode run**

```bash
cd /Users/genie/dev/tools/hearth && npm run tauri dev
```

- [ ] **Step 2: Schedule CRUD smoke**

1. Open 캘린더 tab, click any empty day.
2. Type 일정 name, toggle 알림 받기, check default time picker shows `09:00`.
3. Change time to a moment ~90 seconds from now.
4. Keep 5분 전 off, turn 정각 on.
5. Hit Save. Confirm event row appears in calendar with 🔔.
6. Wait for the native notification. It should fire in the top-right corner.

- [ ] **Step 3: IME bug regression**

1. Open 일정 edit dialog on an existing row.
2. In 내용 (description) field, switch to 한글 IME, type "회의" and press Enter.
3. First Enter commits Korean composition; second Enter should save and close the dialog.
   - **Expected change**: no longer need a *third* press — second Enter saves. (Prior bug: save never fires.)

- [ ] **Step 4: Autostart + hidden boot**

1. Settings → 일반 → toggle "로그인 시 Hearth 자동 실행".
2. Verify `~/Library/LaunchAgents/com.newturn2017.hearth.plist` exists:
   ```bash
   ls -la ~/Library/LaunchAgents | grep hearth
   ```
3. Log out / log back in (or run `launchctl kickstart -k gui/$(id -u)/com.newturn2017.hearth`).
4. Verify Hearth is running but window is hidden:
   ```bash
   pgrep -af hearth
   osascript -e 'tell application "System Events" to get visible of process "Hearth"'
   ```
   Expected: running, `visible: false`.
5. Click Hearth's Dock icon → window should appear.

- [ ] **Step 5: Notification permission edge case**

1. Deny permission via macOS System Settings → Notifications → Hearth.
2. Reopen Hearth, create a scheduled reminder for ~60s out.
3. Verify: a toast surfaces the "notifications-denied" error (or the relevant UI hint). The Rust side logs a warning.
4. Go to Settings → 일반 → 권한 요청 — the button no-ops since system setting takes precedence; confirm the "시스템 설정에서 허용" helper text is shown.

- [ ] **Step 6: Build a release bundle**

```bash
cd /Users/genie/dev/tools/hearth && npm run tauri build
```

Expected: `src-tauri/target/release/bundle/macos/Hearth.app` + dmg produced.

- [ ] **Step 7: Final commit (none required unless fixups)**

If any issues surfaced during Steps 2–6, fix and commit per feature, then rerun this task. Otherwise no-op.

---

## Self-Review Notes (author-only, do not strip)

**Spec coverage check:**

| Spec section | Task(s) covering it |
|---|---|
| Data Model — migration | 3 |
| Data Model — TS types | 11 |
| Notification ID rule | 5 |
| ScheduleModal redesign | 12 |
| IME-safe Enter submit | 13, plus covered in 12 handler |
| SettingsGeneralSection | 15 |
| SettingsDialog tab wiring | 16 |
| CalendarView 🔔 | 14 |
| Notification lifecycle (boot + CRUD) | 7, 8 |
| Time parsing | 5 |
| Notification content | 6 |
| Permission | 8, 15 |
| Autostart deps + capabilities + init | 2, 10 |
| Hidden-on-boot + Reopen | 10 |
| Cargo.toml / capabilities / tauri.conf.json | 2, 10 |
| Rollout / version bump / CHANGELOG | 17 |
| Manual verification | 18 |

All spec checklist items map to at least one task. No orphans.

**Placeholder scan:** no `TBD`, no "handle edge cases" without code, no "similar to Task N". Every code block is self-contained.

**Type consistency:**

- Rust: `ReminderKind::Before5Min` and `ReminderKind::AtStart` used identically in Tasks 5, 6, 7.
- `notification_id(i64, ReminderKind) -> i32` same signature across Tasks 5/6.
- `apply_for(&AppHandle, &Schedule) -> Result<(), String>` consistent in 6/7/8.
- TS: `Schedule` extended once in Task 11 with `remind_before_5min: boolean` / `remind_at_start: boolean`; ScheduleModal (Task 12) reads/writes exactly those keys.
- `NotificationPermission = "granted" | "denied" | "unknown"` defined in Task 11, used in Task 15.
