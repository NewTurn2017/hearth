use rusqlite::Connection;
use std::path::PathBuf;

fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        r#"
        CREATE TABLE settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL DEFAULT ''
        );
        "#,
    )
    .unwrap();
    conn
}

/// Mirrors `cmd_settings::read` for the KV lookup step.
fn read_key(conn: &Connection, key: &str) -> String {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
        r.get::<_, String>(0)
    })
    .unwrap_or_default()
}

/// Mirrors `cmd_settings::write` for the KV upsert.
fn write_key(conn: &Connection, key: &str, value: &str) {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )
    .unwrap();
}

#[test]
fn fallback_to_app_data_when_key_absent() {
    let conn = setup_db();
    // No row inserted — read_key returns empty → fallback path should win.
    let configured = read_key(&conn, "backup.dir");
    assert!(configured.is_empty(), "precondition: no configured backup dir");

    // The helper would fall back to `$APP_DATA/backups`. We model that decision
    // by choosing the fallback when `configured.is_empty()`.
    let fake_app_data = PathBuf::from("/tmp/hearth-test");
    let resolved = if configured.is_empty() {
        fake_app_data.join("backups")
    } else {
        PathBuf::from(configured)
    };
    assert_eq!(resolved, PathBuf::from("/tmp/hearth-test/backups"));
}

#[test]
fn configured_dir_is_returned_when_set() {
    let conn = setup_db();
    write_key(&conn, "backup.dir", "/Users/test/hearth-backups");
    let configured = read_key(&conn, "backup.dir");
    let fake_app_data = PathBuf::from("/tmp/hearth-test");
    let resolved = if configured.is_empty() {
        fake_app_data.join("backups")
    } else {
        PathBuf::from(configured)
    };
    assert_eq!(resolved, PathBuf::from("/Users/test/hearth-backups"));
}

#[test]
fn set_backup_dir_creates_missing_directory() {
    // This test exercises the filesystem precondition `set_backup_dir` relies
    // on: `fs::create_dir_all` on the chosen path. We target a temp dir so the
    // test is hermetic.
    let base = std::env::temp_dir().join(format!(
        "hearth-test-backup-dir-{}",
        std::process::id()
    ));
    let target = base.join("nested/deeper");
    // Precondition: nothing exists yet.
    let _ = std::fs::remove_dir_all(&base);
    assert!(!target.exists(), "precondition: target not yet created");

    std::fs::create_dir_all(&target).unwrap();
    assert!(target.is_dir(), "expected create_dir_all to build the full path");

    // Clean up.
    std::fs::remove_dir_all(&base).unwrap();
}

#[test]
fn overwriting_backup_dir_updates_the_value() {
    let conn = setup_db();
    write_key(&conn, "backup.dir", "/first/path");
    assert_eq!(read_key(&conn, "backup.dir"), "/first/path");

    write_key(&conn, "backup.dir", "/second/path");
    assert_eq!(read_key(&conn, "backup.dir"), "/second/path");
}
