use crate::audit::{write_audit, Op, Source};
use crate::models::Schedule;
use rusqlite::{params, Connection};

fn row_to(row: &rusqlite::Row) -> rusqlite::Result<Schedule> {
    Ok(Schedule {
        id: row.get(0)?,
        date: row.get(1)?,
        time: row.get(2)?,
        location: row.get(3)?,
        description: row.get(4)?,
        notes: row.get(5)?,
        remind_before_5min: row.get::<_, i64>(6)? != 0,
        remind_at_start: row.get::<_, i64>(7)? != 0,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

const COLS: &str = "id, date, time, location, description, notes, remind_before_5min, remind_at_start, created_at, updated_at";

pub fn list(conn: &Connection, month: Option<&str>) -> rusqlite::Result<Vec<Schedule>> {
    let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match month {
        Some(m) => (
            format!(
                "SELECT {COLS} FROM schedules WHERE substr(date,1,7) = ?1 ORDER BY date, COALESCE(time,'')"
            ),
            vec![Box::new(m.to_string())],
        ),
        None => (
            format!("SELECT {COLS} FROM schedules ORDER BY date, COALESCE(time,'')"),
            vec![],
        ),
    };
    let mut stmt = conn.prepare(&sql)?;
    let refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
    let rows = stmt.query_map(refs.as_slice(), row_to)?;
    rows.collect()
}

pub fn list_range(conn: &Connection, from: &str, to: &str) -> rusqlite::Result<Vec<Schedule>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM schedules WHERE date >= ?1 AND date <= ?2 ORDER BY date, COALESCE(time,'')"
    ))?;
    let rows = stmt.query_map([from, to], row_to)?;
    rows.collect()
}

pub fn get(conn: &Connection, id: i64) -> rusqlite::Result<Option<Schedule>> {
    let mut stmt = conn.prepare(&format!("SELECT {COLS} FROM schedules WHERE id=?1"))?;
    match stmt.query_row([id], row_to) {
        Ok(s) => Ok(Some(s)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub struct NewSchedule<'a> {
    pub date: &'a str,
    pub time: Option<&'a str>,
    pub location: Option<&'a str>,
    pub description: Option<&'a str>,
    pub notes: Option<&'a str>,
    pub remind_before_5min: bool,
    pub remind_at_start: bool,
}

pub fn create(conn: &mut Connection, source: Source, input: &NewSchedule<'_>) -> rusqlite::Result<Schedule> {
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO schedules (date, time, location, description, notes, remind_before_5min, remind_at_start)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            input.date,
            input.time,
            input.location,
            input.description,
            input.notes,
            input.remind_before_5min as i64,
            input.remind_at_start as i64,
        ],
    )?;
    let id = tx.last_insert_rowid();
    let after = serde_json::json!({
        "date": input.date,
        "time": input.time,
        "location": input.location,
        "description": input.description,
        "notes": input.notes,
        "remind_before_5min": input.remind_before_5min,
        "remind_at_start": input.remind_at_start,
    });
    write_audit(&tx, source, Op::Create, "schedules", id, None, Some(&after))?;
    tx.commit()?;
    get(conn, id).and_then(|o| o.ok_or(rusqlite::Error::QueryReturnedNoRows))
}

pub struct UpdateSchedule<'a> {
    pub date: Option<&'a str>,
    pub time: Option<&'a str>,
    pub location: Option<&'a str>,
    pub description: Option<&'a str>,
    pub notes: Option<&'a str>,
    pub remind_before_5min: Option<bool>,
    pub remind_at_start: Option<bool>,
}

pub fn update(conn: &mut Connection, source: Source, id: i64, patch: &UpdateSchedule<'_>) -> rusqlite::Result<Schedule> {
    let tx = conn.transaction()?;
    let before = get(&tx, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    let mut sets: Vec<&str> = Vec::new();
    let mut vals: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    if let Some(v) = patch.date { sets.push("date = ?"); vals.push(Box::new(v.to_string())); }
    if let Some(v) = patch.time { sets.push("time = ?"); vals.push(Box::new(v.to_string())); }
    if let Some(v) = patch.location { sets.push("location = ?"); vals.push(Box::new(v.to_string())); }
    if let Some(v) = patch.description { sets.push("description = ?"); vals.push(Box::new(v.to_string())); }
    if let Some(v) = patch.notes { sets.push("notes = ?"); vals.push(Box::new(v.to_string())); }
    if let Some(v) = patch.remind_before_5min { sets.push("remind_before_5min = ?"); vals.push(Box::new(v as i64)); }
    if let Some(v) = patch.remind_at_start { sets.push("remind_at_start = ?"); vals.push(Box::new(v as i64)); }
    if sets.is_empty() { return Err(rusqlite::Error::ToSqlConversionFailure("no fields".into())); }
    sets.push("updated_at = datetime('now')");
    vals.push(Box::new(id));
    let sql = format!("UPDATE schedules SET {} WHERE id = ?", sets.join(", "));
    let refs: Vec<&dyn rusqlite::types::ToSql> = vals.iter().map(|b| b.as_ref()).collect();
    tx.execute(&sql, refs.as_slice())?;
    let after = get(&tx, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    let bj = serde_json::to_value(&before).unwrap();
    let aj = serde_json::to_value(&after).unwrap();
    write_audit(&tx, source, Op::Update, "schedules", id, Some(&bj), Some(&aj))?;
    tx.commit()?;
    Ok(after)
}

pub fn delete(conn: &mut Connection, source: Source, id: i64) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    let before = get(&tx, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    tx.execute("DELETE FROM schedules WHERE id=?1", [id])?;
    let bj = serde_json::to_value(&before).unwrap();
    write_audit(&tx, source, Op::Delete, "schedules", id, Some(&bj), None)?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use tempfile::TempDir;

    fn fresh() -> Connection {
        let d = TempDir::new().unwrap();
        let p = d.path().join("t.db");
        std::mem::forget(d);
        init_db(&p).unwrap()
    }

    #[test]
    fn create_and_list() {
        let mut c = fresh();
        create(
            &mut c, Source::Cli,
            &NewSchedule {
                date: "2026-05-01", time: Some("09:00"),
                location: None, description: Some("dentist"), notes: None,
                remind_before_5min: true, remind_at_start: false,
            },
        ).unwrap();
        let all = list(&c, None).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].date, "2026-05-01");
    }

    #[test]
    fn list_filtered_by_month() {
        let mut c = fresh();
        for date in ["2026-05-01", "2026-05-15", "2026-06-02"] {
            create(
                &mut c, Source::Cli,
                &NewSchedule {
                    date, time: None, location: None, description: None, notes: None,
                    remind_before_5min: false, remind_at_start: false,
                },
            ).unwrap();
        }
        let may = list(&c, Some("2026-05")).unwrap();
        assert_eq!(may.len(), 2);
    }

    #[test]
    fn update_changes_reminder_flags() {
        let mut c = fresh();
        let s = create(
            &mut c, Source::Cli,
            &NewSchedule {
                date: "2026-05-01", time: None, location: None,
                description: None, notes: None,
                remind_before_5min: false, remind_at_start: false,
            },
        ).unwrap();
        let updated = update(
            &mut c, Source::Cli, s.id,
            &UpdateSchedule {
                date: None, time: None, location: None, description: None, notes: None,
                remind_before_5min: Some(true), remind_at_start: Some(true),
            },
        ).unwrap();
        assert!(updated.remind_before_5min);
        assert!(updated.remind_at_start);
    }
}
