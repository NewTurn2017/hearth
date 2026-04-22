//! Audit log write helpers.
//!
//! Every mutation in hearth-core goes through `write_audit` inside the caller's
//! transaction. Phase 3 Task 3.12 adds `list` / `undo` / `redo`.

use rusqlite::{params, Connection};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    App,
    Cli,
    Ai,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::App => "app",
            Source::Cli => "cli",
            Source::Ai => "ai",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Create,
    Update,
    Delete,
}

impl Op {
    pub fn as_str(self) -> &'static str {
        match self {
            Op::Create => "create",
            Op::Update => "update",
            Op::Delete => "delete",
        }
    }
}

/// Write a single audit_log row. Caller is responsible for running this
/// inside a transaction along with the actual data mutation.
pub fn write_audit(
    conn: &Connection,
    source: Source,
    op: Op,
    table: &str,
    row_id: i64,
    before: Option<&Value>,
    after: Option<&Value>,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO audit_log (source, op, table_name, row_id, before_json, after_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            source.as_str(),
            op.as_str(),
            table,
            row_id,
            before.map(|v| v.to_string()),
            after.map(|v| v.to_string()),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use tempfile::TempDir;

    fn tmp() -> (TempDir, Connection) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("t.db");
        let conn = init_db(&path).unwrap();
        (dir, conn)
    }

    #[test]
    fn write_audit_round_trip() {
        let (_dir, conn) = tmp();
        let before = serde_json::json!({ "name": "old" });
        let after = serde_json::json!({ "name": "new" });
        let id = write_audit(
            &conn,
            Source::Cli,
            Op::Update,
            "projects",
            42,
            Some(&before),
            Some(&after),
        )
        .unwrap();
        assert!(id > 0);
        let (src, op, tbl, rid, b, a, undone): (String, String, String, i64, String, String, i64) =
            conn.query_row(
                "SELECT source, op, table_name, row_id, before_json, after_json, undone
                 FROM audit_log WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
            )
            .unwrap();
        assert_eq!(src, "cli");
        assert_eq!(op, "update");
        assert_eq!(tbl, "projects");
        assert_eq!(rid, 42);
        assert!(b.contains("old"));
        assert!(a.contains("new"));
        assert_eq!(undone, 0);
    }
}
