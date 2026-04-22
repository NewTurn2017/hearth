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

#[derive(Debug, Clone, serde::Serialize)]
pub struct AuditEntry {
    pub id: i64,
    pub ts: String,
    pub source: String,
    pub op: String,
    pub table_name: String,
    pub row_id: Option<i64>,
    pub before_json: Option<String>,
    pub after_json: Option<String>,
    pub undone: bool,
}

fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<AuditEntry> {
    Ok(AuditEntry {
        id: row.get(0)?,
        ts: row.get(1)?,
        source: row.get(2)?,
        op: row.get(3)?,
        table_name: row.get(4)?,
        row_id: row.get(5)?,
        before_json: row.get(6)?,
        after_json: row.get(7)?,
        undone: row.get::<_, i64>(8)? != 0,
    })
}

pub fn list(
    conn: &Connection,
    limit: i64,
    source_filter: Option<&str>,
    table_filter: Option<&str>,
    include_undone: bool,
) -> rusqlite::Result<Vec<AuditEntry>> {
    let mut sql = String::from(
        "SELECT id,ts,source,op,table_name,row_id,before_json,after_json,undone
         FROM audit_log WHERE 1=1",
    );
    let mut params: Vec<String> = Vec::new();
    if !include_undone {
        sql.push_str(" AND undone = 0");
    }
    if let Some(s) = source_filter {
        sql.push_str(" AND source = ?");
        params.push(s.to_string());
    }
    if let Some(t) = table_filter {
        sql.push_str(" AND table_name = ?");
        params.push(t.to_string());
    }
    sql.push_str(" ORDER BY id DESC LIMIT ?");
    let mut stmt = conn.prepare(&sql)?;
    let mut p_refs: Vec<&dyn rusqlite::types::ToSql> =
        params.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    p_refs.push(&limit);
    let rows = stmt.query_map(p_refs.as_slice(), row_to_entry)?;
    rows.collect()
}

/// Undo the most recent `count` non-undone entries by reversing each one.
pub fn undo(conn: &mut Connection, count: i64) -> rusqlite::Result<Vec<AuditEntry>> {
    let mut done = Vec::new();
    for _ in 0..count {
        let entry = {
            let mut stmt = conn.prepare(
                "SELECT id,ts,source,op,table_name,row_id,before_json,after_json,undone
                 FROM audit_log WHERE undone = 0 ORDER BY id DESC LIMIT 1",
            )?;
            match stmt.query_row([], row_to_entry) {
                Ok(e) => e,
                Err(rusqlite::Error::QueryReturnedNoRows) => break,
                Err(e) => return Err(e),
            }
        };
        apply_reverse(conn, &entry)?;
        conn.execute("UPDATE audit_log SET undone = 1 WHERE id = ?1", [entry.id])?;
        done.push(entry);
    }
    Ok(done)
}

pub fn redo(conn: &mut Connection, count: i64) -> rusqlite::Result<Vec<AuditEntry>> {
    let mut done = Vec::new();
    for _ in 0..count {
        let entry = {
            let mut stmt = conn.prepare(
                "SELECT id,ts,source,op,table_name,row_id,before_json,after_json,undone
                 FROM audit_log WHERE undone = 1 ORDER BY id DESC LIMIT 1",
            )?;
            match stmt.query_row([], row_to_entry) {
                Ok(e) => e,
                Err(rusqlite::Error::QueryReturnedNoRows) => break,
                Err(e) => return Err(e),
            }
        };
        apply_forward(conn, &entry)?;
        conn.execute("UPDATE audit_log SET undone = 0 WHERE id = ?1", [entry.id])?;
        done.push(entry);
    }
    Ok(done)
}

fn apply_reverse(conn: &mut Connection, e: &AuditEntry) -> rusqlite::Result<()> {
    let row_id = e
        .row_id
        .ok_or_else(|| rusqlite::Error::InvalidQuery)?;
    match e.op.as_str() {
        "create" => {
            conn.execute(
                &format!("DELETE FROM {} WHERE id = ?1", e.table_name),
                [row_id],
            )?;
        }
        "delete" => {
            let before = e
                .before_json
                .as_ref()
                .ok_or(rusqlite::Error::InvalidQuery)?;
            let v: serde_json::Value = serde_json::from_str(before).map_err(|_| rusqlite::Error::InvalidQuery)?;
            insert_from_json(conn, &e.table_name, &v, row_id)?;
        }
        "update" => {
            let before = e
                .before_json
                .as_ref()
                .ok_or(rusqlite::Error::InvalidQuery)?;
            let v: serde_json::Value = serde_json::from_str(before).map_err(|_| rusqlite::Error::InvalidQuery)?;
            update_from_json(conn, &e.table_name, &v, row_id)?;
        }
        _ => return Err(rusqlite::Error::InvalidQuery),
    }
    Ok(())
}

fn apply_forward(conn: &mut Connection, e: &AuditEntry) -> rusqlite::Result<()> {
    let row_id = e
        .row_id
        .ok_or_else(|| rusqlite::Error::InvalidQuery)?;
    match e.op.as_str() {
        "create" => {
            let after = e.after_json.as_ref().ok_or(rusqlite::Error::InvalidQuery)?;
            let v: serde_json::Value = serde_json::from_str(after).map_err(|_| rusqlite::Error::InvalidQuery)?;
            insert_from_json(conn, &e.table_name, &v, row_id)?;
        }
        "delete" => {
            conn.execute(
                &format!("DELETE FROM {} WHERE id = ?1", e.table_name),
                [row_id],
            )?;
        }
        "update" => {
            let after = e.after_json.as_ref().ok_or(rusqlite::Error::InvalidQuery)?;
            let v: serde_json::Value = serde_json::from_str(after).map_err(|_| rusqlite::Error::InvalidQuery)?;
            update_from_json(conn, &e.table_name, &v, row_id)?;
        }
        _ => return Err(rusqlite::Error::InvalidQuery),
    }
    Ok(())
}

fn insert_from_json(
    conn: &mut Connection,
    table: &str,
    v: &serde_json::Value,
    id: i64,
) -> rusqlite::Result<()> {
    let (cols, vals_sql, vals): (Vec<&str>, Vec<String>, Vec<Box<dyn rusqlite::types::ToSql>>) =
        match table {
            "projects" => build_projects_insert(v, id),
            "memos" => build_memos_insert(v, id),
            "schedules" => build_schedules_insert(v, id),
            _ => return Err(rusqlite::Error::InvalidQuery),
        };
    let col_list = cols.join(", ");
    let val_list = vals_sql.join(", ");
    let sql = format!("INSERT OR REPLACE INTO {table} ({col_list}) VALUES ({val_list})");
    let refs: Vec<&dyn rusqlite::types::ToSql> = vals.iter().map(|b| b.as_ref()).collect();
    conn.execute(&sql, refs.as_slice())?;
    Ok(())
}

fn update_from_json(
    conn: &mut Connection,
    table: &str,
    v: &serde_json::Value,
    id: i64,
) -> rusqlite::Result<()> {
    match table {
        "projects" | "memos" | "schedules" => {}
        _ => return Err(rusqlite::Error::InvalidQuery),
    }
    let mut sets: Vec<String> = Vec::new();
    let mut vals: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let obj = v.as_object().ok_or(rusqlite::Error::InvalidQuery)?;
    for (k, val) in obj.iter() {
        if k == "id" { continue; }
        sets.push(format!("{k} = ?"));
        vals.push(json_to_sql(val));
    }
    if sets.is_empty() { return Ok(()); }
    vals.push(Box::new(id));
    let sql = format!(
        "UPDATE {table} SET {} WHERE id = ?",
        sets.join(", ")
    );
    let refs: Vec<&dyn rusqlite::types::ToSql> = vals.iter().map(|b| b.as_ref()).collect();
    conn.execute(&sql, refs.as_slice())?;
    Ok(())
}

fn json_to_sql(v: &serde_json::Value) -> Box<dyn rusqlite::types::ToSql> {
    match v {
        serde_json::Value::Null => Box::new(Option::<String>::None),
        serde_json::Value::Bool(b) => Box::new(if *b { 1i64 } else { 0i64 }),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() { Box::new(i) }
            else if let Some(f) = n.as_f64() { Box::new(f) }
            else { Box::new(n.to_string()) }
        }
        serde_json::Value::String(s) => Box::new(s.clone()),
        _ => Box::new(v.to_string()),
    }
}

// Per-table INSERT column builders. Full column list for INSERT OR REPLACE.
fn build_projects_insert(v: &serde_json::Value, id: i64)
    -> (Vec<&'static str>, Vec<String>, Vec<Box<dyn rusqlite::types::ToSql>>)
{
    let cols = vec!["id","priority","number","name","category","path","evaluation","sort_order","created_at","updated_at"];
    let placeholders: Vec<String> = (1..=cols.len()).map(|i| format!("?{i}")).collect();
    let vals: Vec<Box<dyn rusqlite::types::ToSql>> = vec![
        Box::new(id),
        Box::new(v.get("priority").and_then(|x| x.as_str()).unwrap_or("P4").to_string()),
        Box::new(v.get("number").and_then(|x| x.as_i64())),
        Box::new(v.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string()),
        Box::new(v.get("category").and_then(|x| x.as_str()).map(|s| s.to_string())),
        Box::new(v.get("path").and_then(|x| x.as_str()).map(|s| s.to_string())),
        Box::new(v.get("evaluation").and_then(|x| x.as_str()).map(|s| s.to_string())),
        Box::new(v.get("sort_order").and_then(|x| x.as_i64()).unwrap_or(0)),
        Box::new(v.get("created_at").and_then(|x| x.as_str()).map(|s| s.to_string())),
        Box::new(v.get("updated_at").and_then(|x| x.as_str()).map(|s| s.to_string())),
    ];
    (cols, placeholders, vals)
}

fn build_memos_insert(v: &serde_json::Value, id: i64)
    -> (Vec<&'static str>, Vec<String>, Vec<Box<dyn rusqlite::types::ToSql>>)
{
    let cols = vec!["id","content","color","project_id","sort_order","created_at","updated_at"];
    let placeholders: Vec<String> = (1..=cols.len()).map(|i| format!("?{i}")).collect();
    let vals: Vec<Box<dyn rusqlite::types::ToSql>> = vec![
        Box::new(id),
        Box::new(v.get("content").and_then(|x| x.as_str()).unwrap_or("").to_string()),
        Box::new(v.get("color").and_then(|x| x.as_str()).unwrap_or("yellow").to_string()),
        Box::new(v.get("project_id").and_then(|x| x.as_i64())),
        Box::new(v.get("sort_order").and_then(|x| x.as_i64()).unwrap_or(0)),
        Box::new(v.get("created_at").and_then(|x| x.as_str()).map(|s| s.to_string())),
        Box::new(v.get("updated_at").and_then(|x| x.as_str()).map(|s| s.to_string())),
    ];
    (cols, placeholders, vals)
}

fn build_schedules_insert(v: &serde_json::Value, id: i64)
    -> (Vec<&'static str>, Vec<String>, Vec<Box<dyn rusqlite::types::ToSql>>)
{
    let cols = vec!["id","date","time","location","description","notes","remind_before_5min","remind_at_start","created_at","updated_at"];
    let placeholders: Vec<String> = (1..=cols.len()).map(|i| format!("?{i}")).collect();
    let vals: Vec<Box<dyn rusqlite::types::ToSql>> = vec![
        Box::new(id),
        Box::new(v.get("date").and_then(|x| x.as_str()).unwrap_or("").to_string()),
        Box::new(v.get("time").and_then(|x| x.as_str()).map(|s| s.to_string())),
        Box::new(v.get("location").and_then(|x| x.as_str()).map(|s| s.to_string())),
        Box::new(v.get("description").and_then(|x| x.as_str()).map(|s| s.to_string())),
        Box::new(v.get("notes").and_then(|x| x.as_str()).map(|s| s.to_string())),
        Box::new(v.get("remind_before_5min").and_then(|x| x.as_bool()).unwrap_or(false) as i64),
        Box::new(v.get("remind_at_start").and_then(|x| x.as_bool()).unwrap_or(false) as i64),
        Box::new(v.get("created_at").and_then(|x| x.as_str()).map(|s| s.to_string())),
        Box::new(v.get("updated_at").and_then(|x| x.as_str()).map(|s| s.to_string())),
    ];
    (cols, placeholders, vals)
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

    // Leaks TempDir guard — ok for tests
    fn fresh_conn() -> Connection {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("t.db");
        let conn = init_db(&path).unwrap();
        std::mem::forget(dir);
        conn
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

    #[test]
    fn undo_create_removes_row() {
        use crate::projects;
        let mut c = fresh_conn();
        let p = projects::create(
            &mut c,
            Source::Cli,
            &projects::NewProject {
                name: "x",
                priority: "P2",
                category: None,
                path: None,
                evaluation: None,
            },
        )
        .unwrap();
        undo(&mut c, 1).unwrap();
        assert!(projects::get(&c, p.id).unwrap().is_none());
    }

    #[test]
    fn undo_delete_restores_row() {
        use crate::memos;
        let mut c = fresh_conn();
        let m = memos::create(
            &mut c,
            Source::Cli,
            &memos::NewMemo {
                content: "hi",
                color: "yellow",
                project_id: None,
            },
        )
        .unwrap();
        memos::delete(&mut c, Source::Cli, m.id).unwrap();
        assert!(memos::get(&c, m.id).unwrap().is_none());
        undo(&mut c, 1).unwrap();
        assert_eq!(memos::get(&c, m.id).unwrap().unwrap().content, "hi");
    }

    #[test]
    fn redo_after_undo() {
        use crate::projects;
        let mut c = fresh_conn();
        let p = projects::create(
            &mut c,
            Source::Cli,
            &projects::NewProject {
                name: "x",
                priority: "P2",
                category: None,
                path: None,
                evaluation: None,
            },
        )
        .unwrap();
        undo(&mut c, 1).unwrap();
        redo(&mut c, 1).unwrap();
        assert!(projects::get(&c, p.id).unwrap().is_some());
    }
}
