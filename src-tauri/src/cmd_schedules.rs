use crate::models::Schedule;
use crate::AppState;
use serde::Deserialize;
use tauri::State;

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

const SELECT_COLS: &str =
    "id, date, time, location, description, notes, \
     remind_before_5min, remind_at_start, created_at, updated_at";

#[tauri::command]
pub fn get_schedules(
    state: State<'_, AppState>,
    month: Option<String>,
) -> Result<Vec<Schedule>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let (sql, params): (String, Vec<String>) = match month {
        Some(m) => (
            format!(
                "SELECT {} FROM schedules WHERE date LIKE ?1 ORDER BY date, time",
                SELECT_COLS
            ),
            vec![format!("{}%", m)],
        ),
        None => (
            format!("SELECT {} FROM schedules ORDER BY date, time", SELECT_COLS),
            vec![],
        ),
    };

    let mut stmt = db.prepare(&sql).map_err(|e| e.to_string())?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        params.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    let rows = stmt
        .query_map(param_refs.as_slice(), row_to_schedule)
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
pub fn create_schedule(
    state: State<'_, AppState>,
    data: ScheduleInput,
) -> Result<Schedule, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
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

    let id = db.last_insert_rowid();
    db.query_row(
        &format!("SELECT {} FROM schedules WHERE id = ?1", SELECT_COLS),
        [id],
        row_to_schedule,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_schedule(
    state: State<'_, AppState>,
    id: i64,
    data: ScheduleInput,
) -> Result<Schedule, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
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

    db.query_row(
        &format!("SELECT {} FROM schedules WHERE id = ?1", SELECT_COLS),
        [id],
        row_to_schedule,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_schedule(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute("DELETE FROM schedules WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

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
