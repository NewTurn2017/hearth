use rusqlite::{params, Connection};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub sort_order: i64,
    pub usage_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

fn row_to(row: &rusqlite::Row) -> rusqlite::Result<Category> {
    Ok(Category {
        id: row.get(0)?,
        name: row.get(1)?,
        color: row.get(2)?,
        sort_order: row.get(3)?,
        usage_count: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

const SELECT_WITH_USAGE: &str =
    "SELECT c.id, c.name, c.color, c.sort_order,
            (SELECT COUNT(*) FROM projects p WHERE p.category = c.name) AS usage_count,
            c.created_at, c.updated_at
     FROM categories c";

pub fn list(conn: &Connection) -> rusqlite::Result<Vec<Category>> {
    let sql = format!("{} ORDER BY c.sort_order ASC, c.id ASC", SELECT_WITH_USAGE);
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], row_to)?;
    rows.collect()
}

pub fn get(conn: &Connection, id: i64) -> rusqlite::Result<Option<Category>> {
    let sql = format!("{} WHERE c.id = ?1", SELECT_WITH_USAGE);
    let mut stmt = conn.prepare(&sql)?;
    match stmt.query_row([id], row_to) {
        Ok(c) => Ok(Some(c)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CategoryError {
    #[error("카테고리 이름이 비어 있습니다")]
    EmptyName,
    #[error("이미 존재하는 카테고리 이름입니다: {0}")]
    Duplicate(String),
    #[error("카테고리를 찾을 수 없음: id={0}")]
    NotFound(i64),
    #[error("카테고리 사용 중 ({count}개 프로젝트): {name}")]
    InUse { name: String, count: i64 },
    #[error("sqlite: {0}")]
    Sql(#[from] rusqlite::Error),
}

pub fn create(conn: &Connection, name: &str, color: Option<&str>) -> Result<Category, CategoryError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CategoryError::EmptyName);
    }
    let color = color.map(|c| c.trim().to_string()).unwrap_or_else(|| "#6b7280".into());
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM categories WHERE name = ?1",
        [name],
        |r| r.get(0),
    )?;
    if exists > 0 {
        return Err(CategoryError::Duplicate(name.to_string()));
    }
    let next_order: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM categories",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    conn.execute(
        "INSERT INTO categories (name, color, sort_order) VALUES (?1, ?2, ?3)",
        params![name, color, next_order],
    )?;
    let id = conn.last_insert_rowid();
    Ok(get(conn, id)?.ok_or(CategoryError::NotFound(id))?)
}

pub struct UpdateCategory<'a> {
    pub name: Option<&'a str>,
    pub color: Option<&'a str>,
    pub sort_order: Option<i64>,
}

pub fn update(conn: &mut Connection, id: i64, patch: &UpdateCategory<'_>) -> Result<Category, CategoryError> {
    let current_name: String = conn
        .query_row("SELECT name FROM categories WHERE id = ?1", [id], |r| r.get(0))
        .map_err(|_| CategoryError::NotFound(id))?;

    let mut new_name: Option<String> = None;
    if let Some(raw) = patch.name {
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            return Err(CategoryError::EmptyName);
        }
        if trimmed != current_name {
            let collides: i64 = conn.query_row(
                "SELECT COUNT(*) FROM categories WHERE name = ?1 AND id <> ?2",
                params![trimmed, id],
                |r| r.get(0),
            )?;
            if collides > 0 {
                return Err(CategoryError::Duplicate(trimmed));
            }
            new_name = Some(trimmed);
        }
    }

    let tx = conn.transaction()?;
    if let Some(n) = new_name.as_deref() {
        tx.execute(
            "UPDATE categories SET name = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![n, id],
        )?;
        tx.execute(
            "UPDATE projects SET category = ?1, updated_at = datetime('now') WHERE category = ?2",
            params![n, current_name],
        )?;
    }
    if let Some(color) = patch.color {
        tx.execute(
            "UPDATE categories SET color = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![color.trim(), id],
        )?;
    }
    if let Some(ord) = patch.sort_order {
        tx.execute(
            "UPDATE categories SET sort_order = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![ord, id],
        )?;
    }
    tx.commit()?;
    Ok(get(conn, id)?.ok_or(CategoryError::NotFound(id))?)
}

pub fn delete(conn: &Connection, id: i64) -> Result<(), CategoryError> {
    let (name, usage): (String, i64) = conn
        .query_row(
            "SELECT name, (SELECT COUNT(*) FROM projects p WHERE p.category = c.name)
             FROM categories c WHERE c.id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| CategoryError::NotFound(id))?;
    if usage > 0 {
        return Err(CategoryError::InUse { name, count: usage });
    }
    conn.execute("DELETE FROM categories WHERE id = ?1", [id])?;
    Ok(())
}

pub fn reorder(conn: &mut Connection, ids: &[i64]) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    for (i, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE categories SET sort_order = ?1, updated_at = datetime('now') WHERE id = ?2",
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

    fn fresh() -> Connection {
        let d = TempDir::new().unwrap();
        let p = d.path().join("t.db");
        std::mem::forget(d);
        init_db(&p).unwrap()
    }

    #[test]
    fn rename_cascades_to_projects() {
        let mut c = fresh();
        // Create a category, then project using it.
        create(&c, "Custom", Some("#00ff00")).unwrap();
        c.execute(
            "INSERT INTO projects (name, priority, category) VALUES ('P1', 'P2', 'Custom')",
            [],
        ).unwrap();
        // Rename.
        let cat = list(&c).unwrap().into_iter().find(|x| x.name == "Custom").unwrap();
        update(&mut c, cat.id, &UpdateCategory {
            name: Some("Renamed"), color: None, sort_order: None,
        }).unwrap();
        // Project should now reference new name.
        let new_cat: String = c.query_row(
            "SELECT category FROM projects WHERE name='P1'", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(new_cat, "Renamed");
    }

    #[test]
    fn delete_refuses_if_in_use() {
        let c = fresh();
        create(&c, "Hot", None).unwrap();
        c.execute(
            "INSERT INTO projects (name, priority, category) VALUES ('X', 'P2', 'Hot')",
            [],
        ).unwrap();
        let cat = list(&c).unwrap().into_iter().find(|x| x.name == "Hot").unwrap();
        let err = delete(&c, cat.id).unwrap_err();
        match err {
            CategoryError::InUse { count, .. } => assert_eq!(count, 1),
            _ => panic!("expected InUse"),
        }
    }

    #[test]
    fn create_rejects_duplicate() {
        let c = fresh();
        create(&c, "Dup", None).unwrap();
        let err = create(&c, "Dup", None).unwrap_err();
        matches!(err, CategoryError::Duplicate(_));
    }
}
