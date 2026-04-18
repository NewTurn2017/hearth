//! Integration test for `cmd_backup::reset_data`'s wipe transaction.
//!
//! We mirror the SQL the command runs against an in-memory DB seeded with the
//! real schema (matches `db::init_db`). The filesystem snapshot step is tested
//! separately via the command; this test focuses on "what gets deleted and
//! what survives," which is the part with sharp edges.

use rusqlite::Connection;

fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        r#"
        CREATE TABLE projects (
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
        CREATE TABLE memos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL DEFAULT '',
            color TEXT NOT NULL DEFAULT 'yellow',
            project_id INTEGER REFERENCES projects(id) ON DELETE SET NULL,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE schedules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,
            time TEXT,
            location TEXT,
            description TEXT,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE clients (
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
        CREATE TABLE categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            color TEXT NOT NULL DEFAULT '#6b7280',
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL DEFAULT ''
        );
        "#,
    )
    .unwrap();
    conn
}

fn seed(conn: &Connection) {
    for i in 0..3 {
        conn.execute(
            "INSERT INTO projects (name, priority) VALUES (?1, 'P0')",
            [format!("proj-{}", i)],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO memos (content) VALUES (?1)",
            [format!("memo-{}", i)],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO schedules (date, description) VALUES (?1, ?2)",
            [format!("2026-04-{:02}", i + 1), format!("sch-{}", i)],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO clients (company_name) VALUES (?1)",
            [format!("client-{}", i)],
        )
        .unwrap();
    }
    conn.execute(
        "INSERT INTO categories (name, color) VALUES ('Active', '#F59E0B')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('ai.provider', 'openai')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('ui.scale', '1')",
        [],
    )
    .unwrap();
}

/// Mirrors the wipe transaction inside `cmd_backup::reset_data`.
fn wipe(conn: &mut Connection) {
    let tx = conn.transaction().unwrap();
    for table in ["memos", "schedules", "projects", "clients"] {
        tx.execute(&format!("DELETE FROM {}", table), []).unwrap();
    }
    tx.execute(
        "DELETE FROM sqlite_sequence WHERE name IN (?, ?, ?, ?)",
        ["memos", "schedules", "projects", "clients"],
    )
    .unwrap();
    tx.commit().unwrap();
}

fn count(conn: &Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |r| r.get(0))
        .unwrap()
}

#[test]
fn wipe_clears_user_content() {
    let mut conn = setup_db();
    seed(&conn);

    assert_eq!(count(&conn, "projects"), 3);
    assert_eq!(count(&conn, "memos"), 3);
    assert_eq!(count(&conn, "schedules"), 3);
    assert_eq!(count(&conn, "clients"), 3);

    wipe(&mut conn);

    assert_eq!(count(&conn, "projects"), 0);
    assert_eq!(count(&conn, "memos"), 0);
    assert_eq!(count(&conn, "schedules"), 0);
    assert_eq!(count(&conn, "clients"), 0);
}

#[test]
fn wipe_preserves_categories_and_settings() {
    let mut conn = setup_db();
    seed(&conn);

    wipe(&mut conn);

    // Categories survive — the user may have customized them.
    assert_eq!(count(&conn, "categories"), 1);
    let name: String = conn
        .query_row("SELECT name FROM categories LIMIT 1", [], |r| r.get(0))
        .unwrap();
    assert_eq!(name, "Active");

    // Settings survive — AI creds, backup dir, UI scale are not user content.
    assert_eq!(count(&conn, "settings"), 2);
    let provider: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'ai.provider'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(provider, "openai");
}

#[test]
fn wipe_resets_autoincrement_sequence() {
    let mut conn = setup_db();
    seed(&conn);
    wipe(&mut conn);

    // After the wipe, the next project row should start at id=1 again.
    conn.execute(
        "INSERT INTO projects (name) VALUES ('first-after-reset')",
        [],
    )
    .unwrap();
    let new_id: i64 = conn
        .query_row("SELECT id FROM projects", [], |r| r.get(0))
        .unwrap();
    assert_eq!(new_id, 1);
}

#[test]
fn wipe_is_idempotent_on_empty_db() {
    let mut conn = setup_db();
    // No seeding — tables are already empty.
    wipe(&mut conn);
    wipe(&mut conn); // second call must not blow up
    assert_eq!(count(&conn, "projects"), 0);
}
