use rusqlite::{Connection, Result};
use std::path::Path;

pub fn init_db(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;

    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;

    run_migrations(&conn)?;

    Ok(conn)
}

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

fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS projects (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            priority TEXT NOT NULL DEFAULT 'P4',
            number INTEGER,
            name TEXT NOT NULL,
            category TEXT,
            path TEXT,
            evaluation TEXT,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS schedules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,
            time TEXT,
            location TEXT,
            description TEXT,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS memos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL DEFAULT '',
            color TEXT NOT NULL DEFAULT 'yellow',
            project_id INTEGER REFERENCES projects(id) ON DELETE SET NULL,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS clients (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            company_name TEXT,
            ceo TEXT,
            phone TEXT,
            fax TEXT,
            email TEXT,
            offices TEXT,
            project_desc TEXT,
            status TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Key-value store for app-wide preferences (AI provider, model,
        -- OpenAI API key, etc.). Keeping this in the same SQLite DB so backup
        -- / restore flows already cover it.
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS categories (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            name       TEXT    NOT NULL UNIQUE,
            color      TEXT    NOT NULL DEFAULT '#6b7280',
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT    NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT    NOT NULL DEFAULT (datetime('now'))
        );
        ",
    )?;

    ensure_schedule_reminder_columns(conn)?;
    seed_categories_if_empty(conn)?;
    Ok(())
}

fn seed_categories_if_empty(conn: &Connection) -> Result<()> {
    let count: i64 =
        conn.query_row("SELECT COUNT(*) FROM categories", [], |r| r.get(0))?;
    if count > 0 {
        return Ok(());
    }
    let seed: [(&str, &str, i64); 5] = [
        ("Active",  "#22c55e", 0),
        ("Side",    "#f97316", 1),
        ("Lab",     "#a855f7", 2),
        ("Tools",   "#6b7280", 3),
        ("Lecture", "#3b82f6", 4),
    ];
    let tx = conn.unchecked_transaction()?;
    for (name, color, ord) in seed {
        tx.execute(
            "INSERT INTO categories (name, color, sort_order) VALUES (?1, ?2, ?3)",
            rusqlite::params![name, color, ord],
        )?;
    }
    tx.commit()?;
    Ok(())
}

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
