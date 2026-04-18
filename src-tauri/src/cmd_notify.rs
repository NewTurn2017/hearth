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
