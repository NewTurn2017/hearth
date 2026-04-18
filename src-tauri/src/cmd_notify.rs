//! Notification scheduling for calendar reminders.
//!
//! DB is the source of truth for reminder state. On boot, cancel everything
//! and re-schedule just the future reminders; on CRUD, cancel the affected
//! schedule's ids and re-apply. No separate job table — the notification id
//! is derived deterministically from (schedule_id, kind).

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;
use tauri::async_runtime::JoinHandle;

use crate::models::Schedule;
use crate::AppState;

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
    let base = schedule_id.min(i32::MAX as i64 / 10) as i32;
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

#[derive(Default)]
pub struct Scheduler {
    handles: Mutex<HashMap<i32, JoinHandle<()>>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    fn abort(&self, id: i32) {
        if let Ok(mut map) = self.handles.lock() {
            if let Some(h) = map.remove(&id) {
                h.abort();
            }
        }
    }

    fn insert(&self, id: i32, handle: JoinHandle<()>) {
        if let Ok(mut map) = self.handles.lock() {
            if let Some(prev) = map.insert(id, handle) {
                prev.abort();
            }
        }
    }
}

/// Cancel both possible ids for a schedule. Calling with a schedule id whose
/// handles aren't present is a no-op.
pub fn cancel_for_id(app: &AppHandle, schedule_id: i64) {
    let Some(sched) = app.try_state::<Scheduler>() else {
        return;
    };
    let ids = [
        notification_id(schedule_id, ReminderKind::Before5Min),
        notification_id(schedule_id, ReminderKind::AtStart),
    ];
    for id in ids {
        sched.abort(id);
    }
}

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

fn title_for(kind: ReminderKind) -> &'static str {
    match kind {
        ReminderKind::Before5Min => "일정 5분 전",
        ReminderKind::AtStart => "일정 시작",
    }
}

/// Spawn a tokio task that sleeps until `at_local`, then calls the plugin's
/// `.show()` on the main thread via the app handle. On cancel the task is
/// aborted.
fn spawn_fire(
    app: &AppHandle,
    id: i32,
    kind: ReminderKind,
    at_local: DateTime<Local>,
    body: String,
) {
    let handle = app.clone();
    let task = tauri::async_runtime::spawn(async move {
        let now = Local::now();
        let delta = at_local.signed_duration_since(now);
        // signed_duration_since can be negative if the system clock shifted —
        // treat negative as "fire immediately" for safety.
        let secs = delta.num_seconds().max(0) as u64;
        tokio::time::sleep(tokio::time::Duration::from_secs(secs)).await;
        if let Err(e) = handle
            .notification()
            .builder()
            .title(title_for(kind))
            .body(body)
            .show()
        {
            eprintln!("notification show failed for id {id}: {e}");
        }
    });

    if let Some(sched) = app.try_state::<Scheduler>() {
        sched.insert(id, task);
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
    let body = reminder_body(s);
    let mut reqs: Vec<ReminderKind> = Vec::new();
    if s.remind_before_5min {
        reqs.push(ReminderKind::Before5Min);
    }
    if s.remind_at_start {
        reqs.push(ReminderKind::AtStart);
    }

    for kind in reqs {
        let Some(at_local) = compute_at(&s.date, time, kind) else {
            continue;
        };
        if should_skip_past(now, at_local) {
            continue;
        }
        let id = notification_id(s.id, kind);
        spawn_fire(app, id, kind, at_local, body.clone());
    }
    Ok(())
}

/// Walk the DB, abort any stale scheduler tasks, then re-apply just the
/// future reminders. Runs once at boot; no-op if the scheduler state isn't
/// yet registered (shouldn't happen — setup() manages it before spawning).
pub fn reschedule_all_future(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let rows: Vec<Schedule> = {
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
        rows
    }; // stmt dropped here

    drop(db);

    for s in rows {
        if let Err(e) = apply_for(app, &s) {
            eprintln!("reschedule failed for {}: {}", s.id, e);
        }
    }
    Ok(())
}

// ── Permission commands ────────────────────────────────────────────────────

use tauri_plugin_notification::PermissionState;

/// Return the current notification permission state as a normalized string.
/// Desktop (macOS/Windows/Linux) always returns `"granted"` from the plugin;
/// the command still normalises so the JS side has a stable contract.
#[tauri::command]
pub async fn notifications_permission(app: AppHandle) -> Result<String, String> {
    let state = app
        .notification()
        .permission_state()
        .map_err(|e| e.to_string())?;
    Ok(match state {
        PermissionState::Granted => "granted".into(),
        PermissionState::Denied => "denied".into(),
        _ => "unknown".into(),
    })
}

/// Request notification permission and return the result as a normalized string.
#[tauri::command]
pub async fn notifications_request(app: AppHandle) -> Result<String, String> {
    let state = app
        .notification()
        .request_permission()
        .map_err(|e| e.to_string())?;
    Ok(match state {
        PermissionState::Granted => "granted".into(),
        PermissionState::Denied => "denied".into(),
        _ => "unknown".into(),
    })
}

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
