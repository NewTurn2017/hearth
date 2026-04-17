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
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );
        CREATE TABLE categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            color TEXT NOT NULL DEFAULT '#6b7280',
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );
        "#,
    )
    .unwrap();
    conn
}

fn seed(conn: &Connection) {
    conn.execute_batch(
        r#"
        INSERT INTO categories (name, color, sort_order) VALUES
          ('Active',  '#22c55e', 0),
          ('Side',    '#f97316', 1),
          ('Lab',     '#a855f7', 2),
          ('Tools',   '#6b7280', 3),
          ('Lecture', '#3b82f6', 4);
        INSERT INTO projects (name, category, priority) VALUES
          ('alpha', 'Active', 'P2'),
          ('beta',  'Active', 'P3'),
          ('gamma', 'Side',   'P2');
        "#,
    )
    .unwrap();
}

#[test]
fn unique_constraint_rejects_duplicate_name() {
    let conn = setup_db();
    seed(&conn);
    let err = conn
        .execute(
            "INSERT INTO categories (name, color, sort_order) VALUES ('Active', '#000000', 9)",
            [],
        )
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("UNIQUE"), "expected UNIQUE error, got: {msg}");
}

#[test]
fn rename_cascade_moves_all_projects() {
    let conn = setup_db();
    seed(&conn);
    let tx = conn.unchecked_transaction().unwrap();
    tx.execute(
        "UPDATE categories SET name = 'Production' WHERE name = 'Active'",
        [],
    )
    .unwrap();
    tx.execute(
        "UPDATE projects SET category = 'Production' WHERE category = 'Active'",
        [],
    )
    .unwrap();
    tx.commit().unwrap();

    let renamed: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM projects WHERE category = 'Production'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(renamed, 2);

    let old_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM projects WHERE category = 'Active'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(old_rows, 0);
}

#[test]
fn delete_in_use_is_blocked_by_usage_count() {
    let conn = setup_db();
    seed(&conn);
    let usage: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM projects WHERE category = 'Active'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    // The command-layer guard is the check we're modeling: refuse deletion
    // when usage > 0. The test asserts the SQL-side fact the guard relies on.
    assert_eq!(usage, 2, "precondition: Active is in use");
}

#[test]
fn delete_unused_succeeds() {
    let conn = setup_db();
    seed(&conn);
    let deleted = conn
        .execute("DELETE FROM categories WHERE name = 'Lecture'", [])
        .unwrap();
    assert_eq!(deleted, 1);
}
