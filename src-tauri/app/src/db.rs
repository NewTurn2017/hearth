use rusqlite::{Connection, Result};
use std::path::{Path, PathBuf};

pub fn init_db(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;

    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;

    run_migrations(&conn)?;

    Ok(conn)
}

/// Outcome of `init_db_with_recovery`.
pub enum DbInitOutcome {
    Ok(Connection),
    /// DB was corrupt; the old file was renamed to `quarantined_to` and a
    /// fresh empty DB is returned. The caller should surface this so the user
    /// can restore from a backup (Settings → 백업).
    Recovered {
        conn: Connection,
        quarantined_to: PathBuf,
    },
}

fn is_corruption(err: &rusqlite::Error) -> bool {
    matches!(
        err,
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: rusqlite::ErrorCode::DatabaseCorrupt
                    | rusqlite::ErrorCode::NotADatabase,
                ..
            },
            _,
        )
    )
}

fn timestamp_tag() -> String {
    chrono::Local::now().format("%Y%m%d-%H%M%S").to_string()
}

/// Open the DB; if the file is malformed, rename the corrupt files aside and
/// boot from an empty schema instead of panicking. The caller receives the
/// path we quarantined to so it can notify the user.
pub fn init_db_with_recovery(db_path: &Path) -> Result<DbInitOutcome> {
    match init_db(db_path) {
        Ok(conn) => Ok(DbInitOutcome::Ok(conn)),
        Err(e) if is_corruption(&e) => {
            let ts = timestamp_tag();
            let file_name = db_path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "data.db".to_string());
            let quarantined_to =
                db_path.with_file_name(format!("{file_name}.corrupt-{ts}"));
            // Move the main file. If rename fails (e.g. the DB pre-open created
            // a 0-byte file that isn't the source of the corruption), try a
            // copy-then-delete as a fallback so we always clear the path.
            let _ = std::fs::rename(db_path, &quarantined_to)
                .or_else(|_| std::fs::copy(db_path, &quarantined_to).map(|_| ()))
                .and_then(|_| {
                    if db_path.exists() {
                        std::fs::remove_file(db_path)
                    } else {
                        Ok(())
                    }
                });
            // WAL / SHM sidecars inherit the corruption — quarantine them too.
            for suffix in ["-wal", "-shm"] {
                let sidecar = db_path.with_file_name(format!("{file_name}{suffix}"));
                if sidecar.exists() {
                    let dst = db_path
                        .with_file_name(format!("{file_name}{suffix}.corrupt-{ts}"));
                    let _ = std::fs::rename(&sidecar, &dst);
                }
            }
            let conn = init_db(db_path)?;
            Ok(DbInitOutcome::Recovered {
                conn,
                quarantined_to,
            })
        }
        Err(e) => Err(e),
    }
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

    #[test]
    fn init_with_recovery_returns_ok_for_healthy_db() {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("data.db");
        // First init — fresh file.
        match init_db_with_recovery(&db_path).unwrap() {
            DbInitOutcome::Ok(_) => {}
            _ => panic!("fresh DB should return Ok"),
        }
        // Second init — existing healthy file.
        match init_db_with_recovery(&db_path).unwrap() {
            DbInitOutcome::Ok(_) => {}
            _ => panic!("existing healthy DB should return Ok"),
        }
    }

    #[test]
    fn init_with_recovery_quarantines_malformed_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("data.db");
        // Write non-SQLite bytes — Connection::open itself succeeds but the
        // first PRAGMA / migration query fails with NotADatabase.
        std::fs::write(&db_path, b"not a sqlite file, definitely corrupt").unwrap();

        let outcome = init_db_with_recovery(&db_path).unwrap();
        let quarantined = match outcome {
            DbInitOutcome::Recovered { quarantined_to, .. } => quarantined_to,
            _ => panic!("expected Recovered outcome for corrupt DB"),
        };
        assert!(
            quarantined.exists(),
            "quarantined file should exist at {:?}",
            quarantined
        );
        assert!(
            db_path.exists(),
            "fresh DB should be created at original path"
        );
        // And the fresh DB should have our schema.
        let conn = Connection::open(&db_path).unwrap();
        conn.query_row("SELECT COUNT(*) FROM projects", [], |r| r.get::<_, i64>(0))
            .unwrap();
    }
}
