use crate::audit::{write_audit, Op, Source};
use crate::models::Memo;
use rusqlite::{params, Connection};

fn row_to_memo(row: &rusqlite::Row) -> rusqlite::Result<Memo> {
    Ok(Memo {
        id: row.get(0)?,
        content: row.get(1)?,
        color: row.get(2)?,
        project_id: row.get(3)?,
        sort_order: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

const SELECT_COLS: &str = "id, content, color, project_id, sort_order, created_at, updated_at";

pub fn list(conn: &Connection) -> rusqlite::Result<Vec<Memo>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLS} FROM memos ORDER BY sort_order ASC"
    ))?;
    let rows = stmt.query_map([], row_to_memo)?;
    rows.collect()
}

pub fn get(conn: &Connection, id: i64) -> rusqlite::Result<Option<Memo>> {
    let mut stmt = conn.prepare(&format!("SELECT {SELECT_COLS} FROM memos WHERE id=?1"))?;
    match stmt.query_row([id], row_to_memo) {
        Ok(m) => Ok(Some(m)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub struct NewMemo<'a> {
    pub content: &'a str,
    pub color: &'a str,
    pub project_id: Option<i64>,
}

pub fn create(conn: &mut Connection, source: Source, input: &NewMemo<'_>) -> rusqlite::Result<Memo> {
    let tx = conn.transaction()?;
    let max_order: i64 = tx
        .query_row("SELECT COALESCE(MAX(sort_order), 0) FROM memos", [], |r| {
            r.get(0)
        })
        .unwrap_or(0);
    tx.execute(
        "INSERT INTO memos (content, color, project_id, sort_order)
         VALUES (?1, ?2, ?3, ?4)",
        params![input.content, input.color, input.project_id, max_order + 1],
    )?;
    let id = tx.last_insert_rowid();
    let after = serde_json::json!({
        "content": input.content, "color": input.color, "project_id": input.project_id,
    });
    write_audit(&tx, source, Op::Create, "memos", id, None, Some(&after))?;
    tx.commit()?;
    get(conn, id).and_then(|opt| opt.ok_or(rusqlite::Error::QueryReturnedNoRows))
}

pub struct UpdateMemo<'a> {
    pub content: Option<&'a str>,
    pub color: Option<&'a str>,
    /// `Some(Some(id))` → attach, `Some(None)` → detach, `None` → no change.
    pub project_id: Option<Option<i64>>,
}

pub fn update(
    conn: &mut Connection,
    source: Source,
    id: i64,
    patch: &UpdateMemo<'_>,
) -> rusqlite::Result<Memo> {
    let tx = conn.transaction()?;
    let before = get(&tx, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    let mut sets: Vec<&str> = Vec::new();
    let mut vals: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    if let Some(v) = patch.content {
        sets.push("content = ?");
        vals.push(Box::new(v.to_string()));
    }
    if let Some(v) = patch.color {
        sets.push("color = ?");
        vals.push(Box::new(v.to_string()));
    }
    if let Some(pid) = patch.project_id {
        sets.push("project_id = ?");
        vals.push(Box::new(pid));
    }
    if sets.is_empty() {
        return Err(rusqlite::Error::ToSqlConversionFailure("no fields".into()));
    }
    sets.push("updated_at = datetime('now')");
    vals.push(Box::new(id));
    let sql = format!("UPDATE memos SET {} WHERE id = ?", sets.join(", "));
    let refs: Vec<&dyn rusqlite::types::ToSql> = vals.iter().map(|p| p.as_ref()).collect();
    tx.execute(&sql, refs.as_slice())?;
    let after = get(&tx, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    let bj = serde_json::to_value(&before).unwrap();
    let aj = serde_json::to_value(&after).unwrap();
    write_audit(&tx, source, Op::Update, "memos", id, Some(&bj), Some(&aj))?;
    tx.commit()?;
    Ok(after)
}

pub fn delete(conn: &mut Connection, source: Source, id: i64) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    let before = get(&tx, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    tx.execute("DELETE FROM memos WHERE id=?1", [id])?;
    let bj = serde_json::to_value(&before).unwrap();
    write_audit(&tx, source, Op::Delete, "memos", id, Some(&bj), None)?;
    tx.commit()?;
    Ok(())
}

pub fn reorder(conn: &mut Connection, ids: &[i64]) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    for (i, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE memos SET sort_order=?1, updated_at=datetime('now') WHERE id=?2",
            params![i as i64, id],
        )?;
    }
    tx.commit()
}

pub fn update_by_number(
    conn: &mut Connection,
    source: Source,
    number: i64,
    new_content: &str,
) -> rusqlite::Result<Memo> {
    let id: i64 = {
        let mut stmt = conn.prepare(
            "SELECT id FROM memos ORDER BY sort_order ASC LIMIT 1 OFFSET ?1",
        )?;
        stmt.query_row([(number - 1).max(0)], |r| r.get::<_, i64>(0))?
    };
    update(
        conn,
        source,
        id,
        &UpdateMemo {
            content: Some(new_content),
            color: None,
            project_id: None,
        },
    )
}

pub fn delete_by_number(conn: &mut Connection, source: Source, number: i64) -> rusqlite::Result<()> {
    let id: i64 = {
        let mut stmt = conn.prepare(
            "SELECT id FROM memos ORDER BY sort_order ASC LIMIT 1 OFFSET ?1",
        )?;
        stmt.query_row([(number - 1).max(0)], |r| r.get::<_, i64>(0))?
    };
    delete(conn, source, id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use tempfile::TempDir;

    fn fresh() -> Connection {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("t.db");
        std::mem::forget(dir);
        init_db(&path).unwrap()
    }

    #[test]
    fn create_and_list() {
        let mut c = fresh();
        create(
            &mut c,
            Source::Cli,
            &NewMemo {
                content: "hi",
                color: "yellow",
                project_id: None,
            },
        )
        .unwrap();
        assert_eq!(list(&c).unwrap().len(), 1);
    }

    #[test]
    fn update_detach_project() {
        let mut c = fresh();
        // Insert a real project so FK is satisfied, then attach memo, then detach.
        c.execute(
            "INSERT INTO projects (name, priority, sort_order) VALUES ('P', 'P2', 1)",
            [],
        )
        .unwrap();
        let project_id: i64 = c.last_insert_rowid();
        let m = create(
            &mut c,
            Source::Cli,
            &NewMemo {
                content: "a",
                color: "pink",
                project_id: Some(project_id),
            },
        )
        .unwrap();
        let updated = update(
            &mut c,
            Source::Cli,
            m.id,
            &UpdateMemo {
                content: None,
                color: None,
                project_id: Some(None),
            },
        )
        .unwrap();
        assert_eq!(updated.project_id, None);
    }

    #[test]
    fn update_by_number_changes_content() {
        let mut c = fresh();
        create(&mut c, Source::Cli, &NewMemo { content: "first", color: "yellow", project_id: None }).unwrap();
        create(&mut c, Source::Cli, &NewMemo { content: "second", color: "yellow", project_id: None }).unwrap();
        let updated = update_by_number(&mut c, Source::Cli, 1, "updated first").unwrap();
        assert_eq!(updated.content, "updated first");
    }

    #[test]
    fn delete_by_number_removes_correct_memo() {
        let mut c = fresh();
        create(&mut c, Source::Cli, &NewMemo { content: "a", color: "yellow", project_id: None }).unwrap();
        create(&mut c, Source::Cli, &NewMemo { content: "b", color: "yellow", project_id: None }).unwrap();
        delete_by_number(&mut c, Source::Cli, 1).unwrap();
        let remaining = list(&c).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].content, "b");
    }
}
