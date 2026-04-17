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
        CREATE TABLE memos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL DEFAULT '',
            color TEXT NOT NULL DEFAULT 'yellow',
            project_id INTEGER REFERENCES projects(id) ON DELETE SET NULL,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );
        INSERT INTO memos (content, sort_order) VALUES
            ('first', 0), ('second', 1), ('third', 2);
        "#,
    )
    .unwrap();
    conn
}

#[test]
fn resolve_number_offset_maps_to_correct_memo() {
    let conn = setup_db();
    // number=2 → 2nd memo (sort_order=1) = 'second'
    let (id, content): (i64, String) = conn
        .query_row(
            "SELECT id, content FROM memos ORDER BY sort_order LIMIT 1 OFFSET ?",
            [1_i64],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(content, "second");
    assert_eq!(id, 2);
}

#[test]
fn resolve_number_out_of_range_returns_err() {
    let conn = setup_db();
    let result: rusqlite::Result<(i64, String)> = conn.query_row(
        "SELECT id, content FROM memos ORDER BY sort_order LIMIT 1 OFFSET ?",
        [10_i64],
        |r| Ok((r.get(0)?, r.get(1)?)),
    );
    assert!(matches!(result, Err(rusqlite::Error::QueryReturnedNoRows)));
}
