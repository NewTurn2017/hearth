//! Projects CRUD. Every mutation tx records an audit_log row.

use crate::audit::{write_audit, Op, Source};
use crate::models::Project;
use rusqlite::{params, Connection};
use serde_json::json;

fn row_to_project(row: &rusqlite::Row) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        priority: row.get(1)?,
        number: row.get(2)?,
        name: row.get(3)?,
        category: row.get(4)?,
        path: row.get(5)?,
        evaluation: row.get(6)?,
        sort_order: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

const SELECT_COLS: &str =
    "id, priority, number, name, category, path, evaluation, sort_order, created_at, updated_at";

pub fn list(conn: &Connection) -> rusqlite::Result<Vec<Project>> {
    let sql = format!(
        "SELECT {SELECT_COLS} FROM projects \
         ORDER BY CASE priority WHEN 'P0' THEN 0 WHEN 'P1' THEN 1 WHEN 'P2' THEN 2 \
         WHEN 'P3' THEN 3 WHEN 'P4' THEN 4 END, sort_order ASC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], row_to_project)?;
    rows.collect()
}

pub fn get(conn: &Connection, id: i64) -> rusqlite::Result<Option<Project>> {
    let sql = format!("SELECT {SELECT_COLS} FROM projects WHERE id=?1");
    let mut stmt = conn.prepare(&sql)?;
    match stmt.query_row([id], row_to_project) {
        Ok(p) => Ok(Some(p)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub struct NewProject<'a> {
    pub name: &'a str,
    pub priority: &'a str,
    pub category: Option<&'a str>,
    pub path: Option<&'a str>,
    pub evaluation: Option<&'a str>,
}

pub fn create(
    conn: &mut Connection,
    source: Source,
    input: &NewProject<'_>,
) -> rusqlite::Result<Project> {
    let tx = conn.transaction()?;
    let max_order: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(sort_order), 0) FROM projects WHERE priority = ?1",
            [input.priority],
            |row| row.get(0),
        )
        .unwrap_or(0);

    tx.execute(
        "INSERT INTO projects (name, priority, category, path, evaluation, sort_order)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            input.name,
            input.priority,
            input.category,
            input.path,
            input.evaluation,
            max_order + 1
        ],
    )?;
    let id = tx.last_insert_rowid();
    let after = json!({
        "name": input.name,
        "priority": input.priority,
        "category": input.category,
        "path": input.path,
        "evaluation": input.evaluation,
    });
    write_audit(&tx, source, Op::Create, "projects", id, None, Some(&after))?;
    tx.commit()?;
    get(conn, id).and_then(|opt| opt.ok_or(rusqlite::Error::QueryReturnedNoRows))
}

pub struct UpdateProject<'a> {
    pub name: Option<&'a str>,
    pub priority: Option<&'a str>,
    pub category: Option<&'a str>,
    pub path: Option<&'a str>,
    pub evaluation: Option<&'a str>,
}

pub fn update(
    conn: &mut Connection,
    source: Source,
    id: i64,
    patch: &UpdateProject<'_>,
) -> rusqlite::Result<Project> {
    let tx = conn.transaction()?;
    let before = get(&tx, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;

    let mut sets: Vec<&str> = Vec::new();
    let mut vals: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    if let Some(v) = patch.name { sets.push("name = ?"); vals.push(Box::new(v.to_string())); }
    if let Some(v) = patch.priority { sets.push("priority = ?"); vals.push(Box::new(v.to_string())); }
    if let Some(v) = patch.category { sets.push("category = ?"); vals.push(Box::new(v.to_string())); }
    if let Some(v) = patch.path { sets.push("path = ?"); vals.push(Box::new(v.to_string())); }
    if let Some(v) = patch.evaluation { sets.push("evaluation = ?"); vals.push(Box::new(v.to_string())); }

    if sets.is_empty() {
        return Err(rusqlite::Error::ToSqlConversionFailure(
            "no fields to update".into(),
        ));
    }

    sets.push("updated_at = datetime('now')");
    vals.push(Box::new(id));
    let sql = format!("UPDATE projects SET {} WHERE id = ?", sets.join(", "));
    let refs: Vec<&dyn rusqlite::types::ToSql> = vals.iter().map(|p| p.as_ref()).collect();
    tx.execute(&sql, refs.as_slice())?;

    let after = get(&tx, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    let before_json = serde_json::to_value(&before).unwrap();
    let after_json = serde_json::to_value(&after).unwrap();
    write_audit(
        &tx,
        source,
        Op::Update,
        "projects",
        id,
        Some(&before_json),
        Some(&after_json),
    )?;
    tx.commit()?;
    Ok(after)
}

pub fn delete(conn: &mut Connection, source: Source, id: i64) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    let before = get(&tx, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    tx.execute("DELETE FROM projects WHERE id = ?1", [id])?;
    let before_json = serde_json::to_value(&before).unwrap();
    write_audit(
        &tx,
        source,
        Op::Delete,
        "projects",
        id,
        Some(&before_json),
        None,
    )?;
    tx.commit()?;
    Ok(())
}

pub fn search_like(conn: &Connection, query: &str) -> rusqlite::Result<Vec<Project>> {
    let pattern = format!("%{}%", query);
    let sql = format!(
        "SELECT {SELECT_COLS} FROM projects \
         WHERE name LIKE ?1 OR evaluation LIKE ?1 OR category LIKE ?1 \
         ORDER BY sort_order ASC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([&pattern], row_to_project)?;
    rows.collect()
}

pub fn reorder(conn: &mut Connection, ids: &[i64]) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    for (i, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE projects SET sort_order = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![i as i64, id],
        )?;
    }
    tx.commit()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use tempfile::TempDir;

    fn fresh() -> (TempDir, Connection) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("t.db");
        let conn = init_db(&path).unwrap();
        (dir, conn)
    }


#[test]
    fn create_inserts_row_and_audit() {
        let (_d, mut conn) = fresh();
        let p = create(
            &mut conn,
            Source::Cli,
            &NewProject {
                name: "X",
                priority: "P2",
                category: Some("Side"),
                path: None,
                evaluation: None,
            },
        )
        .unwrap();
        assert_eq!(p.name, "X");
        let audit_cnt: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audit_log WHERE table_name='projects' AND op='create' AND row_id=?1",
                [p.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(audit_cnt, 1);
    }

    #[test]
    fn update_patches_only_given_fields_and_records_before_after() {
        let (_d, mut conn) = fresh();
        let p = create(
            &mut conn,
            Source::Cli,
            &NewProject {
                name: "X",
                priority: "P2",
                category: None,
                path: None,
                evaluation: None,
            },
        )
        .unwrap();
        let updated = update(
            &mut conn,
            Source::Cli,
            p.id,
            &UpdateProject {
                name: Some("Y"),
                priority: None,
                category: None,
                path: None,
                evaluation: None,
            },
        )
        .unwrap();
        assert_eq!(updated.name, "Y");
        assert_eq!(updated.priority, "P2");
        let (before_json, after_json): (String, String) = conn
            .query_row(
                "SELECT before_json, after_json FROM audit_log
                 WHERE op='update' AND row_id=?1 ORDER BY id DESC LIMIT 1",
                [p.id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(before_json.contains("\"X\""));
        assert!(after_json.contains("\"Y\""));
    }

    #[test]
    fn delete_removes_row_and_stores_before_json() {
        let (_d, mut conn) = fresh();
        let p = create(
            &mut conn,
            Source::Cli,
            &NewProject {
                name: "D",
                priority: "P3",
                category: None,
                path: None,
                evaluation: None,
            },
        )
        .unwrap();
        delete(&mut conn, Source::Cli, p.id).unwrap();
        assert!(get(&conn, p.id).unwrap().is_none());
        let before_json: String = conn
            .query_row(
                "SELECT before_json FROM audit_log WHERE op='delete' AND row_id=?1",
                [p.id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(before_json.contains("\"D\""));
    }
}
