use crate::audit::{write_audit, Op, Source};
use crate::models::{Memo, MemoFontSize, MemoTag};
use rusqlite::{params, Connection};
use std::collections::HashMap;

fn row_to_memo(row: &rusqlite::Row) -> rusqlite::Result<Memo> {
    Ok(Memo {
        id: row.get(0)?,
        content: row.get(1)?,
        color: row.get(2)?,
        project_id: row.get(3)?,
        sort_order: row.get(4)?,
        font_size: MemoFontSize::parse(row.get::<_, String>(5)?.as_str())?,
        is_bold: row.get::<_, i64>(6)? != 0,
        focus_x: row.get(7)?,
        focus_y: row.get(8)?,
        tags: Vec::new(),
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn row_to_tag(row: &rusqlite::Row) -> rusqlite::Result<MemoTag> {
    Ok(MemoTag {
        id: row.get(0)?,
        name: row.get(1)?,
        color: row.get(2)?,
        sort_order: row.get(3)?,
        usage_count: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

const SELECT_COLS: &str =
    "id, content, color, project_id, sort_order, font_size, is_bold, focus_x, focus_y, created_at, updated_at";

fn tags_for_memo(conn: &Connection, memo_id: i64) -> rusqlite::Result<Vec<MemoTag>> {
    let mut stmt = conn.prepare(
        "SELECT mt.id, mt.name, mt.color, mt.sort_order,
                (SELECT COUNT(*) FROM memo_tag_links all_links WHERE all_links.tag_id = mt.id) AS usage_count,
                mt.created_at, mt.updated_at
         FROM memo_tags mt
         JOIN memo_tag_links mtl ON mtl.tag_id = mt.id
         WHERE mtl.memo_id = ?1
         ORDER BY mt.sort_order ASC, mt.name ASC",
    )?;
    let rows = stmt.query_map([memo_id], row_to_tag)?;
    rows.collect()
}

fn tags_for_memos(
    conn: &Connection,
    memo_ids: &[i64],
) -> rusqlite::Result<HashMap<i64, Vec<MemoTag>>> {
    let mut tags_by_memo: HashMap<i64, Vec<MemoTag>> = HashMap::new();
    if memo_ids.is_empty() {
        return Ok(tags_by_memo);
    }
    let placeholders = std::iter::repeat("?")
        .take(memo_ids.len())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT mtl.memo_id, mt.id, mt.name, mt.color, mt.sort_order,
                (SELECT COUNT(*) FROM memo_tag_links all_links WHERE all_links.tag_id = mt.id) AS usage_count,
                mt.created_at, mt.updated_at
         FROM memo_tags mt
         JOIN memo_tag_links mtl ON mtl.tag_id = mt.id
         WHERE mtl.memo_id IN ({placeholders})
         ORDER BY mtl.memo_id ASC, mt.sort_order ASC, mt.name ASC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(memo_ids.iter()), |row| {
        Ok((
            row.get::<_, i64>(0)?,
            MemoTag {
                id: row.get(1)?,
                name: row.get(2)?,
                color: row.get(3)?,
                sort_order: row.get(4)?,
                usage_count: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            },
        ))
    })?;
    for row in rows {
        let (memo_id, tag) = row?;
        tags_by_memo.entry(memo_id).or_default().push(tag);
    }
    Ok(tags_by_memo)
}

fn attach_tags(conn: &Connection, mut memo: Memo) -> rusqlite::Result<Memo> {
    memo.tags = tags_for_memo(conn, memo.id)?;
    Ok(memo)
}

fn get_memo_tag(conn: &Connection, id: i64) -> rusqlite::Result<Option<MemoTag>> {
    let mut stmt = conn.prepare(
        "SELECT mt.id, mt.name, mt.color, mt.sort_order,
                (SELECT COUNT(*) FROM memo_tag_links all_links WHERE all_links.tag_id = mt.id) AS usage_count,
                mt.created_at, mt.updated_at
         FROM memo_tags mt
         WHERE mt.id = ?1",
    )?;
    match stmt.query_row([id], row_to_tag) {
        Ok(tag) => Ok(Some(tag)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub struct UpdateMemoTag<'a> {
    pub name: Option<&'a str>,
    pub color: Option<&'a str>,
    pub sort_order: Option<i64>,
}

pub fn list_memo_tags(conn: &Connection) -> rusqlite::Result<Vec<MemoTag>> {
    let mut stmt = conn.prepare(
        "SELECT mt.id, mt.name, mt.color, mt.sort_order,
                (SELECT COUNT(*) FROM memo_tag_links all_links WHERE all_links.tag_id = mt.id) AS usage_count,
                mt.created_at, mt.updated_at
         FROM memo_tags mt
         ORDER BY mt.sort_order ASC, mt.name ASC",
    )?;
    let rows = stmt.query_map([], row_to_tag)?;
    rows.collect()
}

pub fn create_memo_tag(
    conn: &mut Connection,
    source: Source,
    name: &str,
    color: Option<&str>,
) -> rusqlite::Result<MemoTag> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(rusqlite::Error::InvalidParameterName(
            "memo tag name cannot be empty".to_string(),
        ));
    }
    let tx = conn.transaction()?;
    let next_order: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM memo_tags",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    tx.execute(
        "INSERT INTO memo_tags (name, color, sort_order) VALUES (?1, ?2, ?3)",
        params![trimmed, color.unwrap_or("#6b7280"), next_order],
    )?;
    let id = tx.last_insert_rowid();
    let after = get_memo_tag(&tx, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    let aj = serde_json::to_value(&after).unwrap();
    write_audit(&tx, source, Op::Create, "memo_tags", id, None, Some(&aj))?;
    tx.commit()?;
    Ok(after)
}

pub fn update_memo_tag(
    conn: &mut Connection,
    source: Source,
    id: i64,
    patch: &UpdateMemoTag<'_>,
) -> rusqlite::Result<MemoTag> {
    let tx = conn.transaction()?;
    let before = get_memo_tag(&tx, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    let mut sets: Vec<&str> = Vec::new();
    let mut vals: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    if let Some(name) = patch.name {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(rusqlite::Error::InvalidParameterName(
                "memo tag name cannot be empty".to_string(),
            ));
        }
        sets.push("name = ?");
        vals.push(Box::new(trimmed.to_string()));
    }
    if let Some(color) = patch.color {
        sets.push("color = ?");
        vals.push(Box::new(color.to_string()));
    }
    if let Some(sort_order) = patch.sort_order {
        sets.push("sort_order = ?");
        vals.push(Box::new(sort_order));
    }
    if sets.is_empty() {
        return Err(rusqlite::Error::ToSqlConversionFailure("no fields".into()));
    }
    sets.push("updated_at = datetime('now')");
    vals.push(Box::new(id));
    let sql = format!("UPDATE memo_tags SET {} WHERE id = ?", sets.join(", "));
    let refs: Vec<&dyn rusqlite::types::ToSql> = vals.iter().map(|p| p.as_ref()).collect();
    tx.execute(&sql, refs.as_slice())?;
    let after = get_memo_tag(&tx, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    let bj = serde_json::to_value(&before).unwrap();
    let aj = serde_json::to_value(&after).unwrap();
    write_audit(
        &tx,
        source,
        Op::Update,
        "memo_tags",
        id,
        Some(&bj),
        Some(&aj),
    )?;
    tx.commit()?;
    Ok(after)
}

pub fn delete_memo_tag(conn: &mut Connection, source: Source, id: i64) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    let before = get_memo_tag(&tx, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    let linked_memo_ids = {
        let mut stmt = tx
            .prepare("SELECT memo_id FROM memo_tag_links WHERE tag_id = ?1 ORDER BY memo_id ASC")?;
        let rows = stmt.query_map([id], |row| row.get::<_, i64>(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    tx.execute("DELETE FROM memo_tags WHERE id = ?1", [id])?;
    let mut bj = serde_json::to_value(&before).unwrap();
    if let Some(obj) = bj.as_object_mut() {
        obj.insert(
            "linked_memo_ids".to_string(),
            serde_json::json!(linked_memo_ids),
        );
    }
    write_audit(&tx, source, Op::Delete, "memo_tags", id, Some(&bj), None)?;
    tx.commit()?;
    Ok(())
}

pub fn reorder_memo_tags(conn: &mut Connection, ids: &[i64]) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    for (i, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE memo_tags SET sort_order = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![i as i64, id],
        )?;
    }
    tx.commit()
}

pub fn list(conn: &Connection) -> rusqlite::Result<Vec<Memo>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLS} FROM memos ORDER BY sort_order ASC"
    ))?;
    let rows = stmt.query_map([], row_to_memo)?;
    let mut memos = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    let memo_ids = memos.iter().map(|m| m.id).collect::<Vec<_>>();
    let mut tags_by_memo = tags_for_memos(conn, &memo_ids)?;
    for memo in &mut memos {
        memo.tags = tags_by_memo.remove(&memo.id).unwrap_or_default();
    }
    Ok(memos)
}

pub fn get(conn: &Connection, id: i64) -> rusqlite::Result<Option<Memo>> {
    let mut stmt = conn.prepare(&format!("SELECT {SELECT_COLS} FROM memos WHERE id=?1"))?;
    match stmt.query_row([id], row_to_memo) {
        Ok(m) => Ok(Some(attach_tags(conn, m)?)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub struct NewMemo<'a> {
    pub content: &'a str,
    pub color: &'a str,
    pub project_id: Option<i64>,
    pub font_size: Option<&'a str>,
    pub is_bold: Option<bool>,
    pub focus_x: Option<f64>,
    pub focus_y: Option<f64>,
    pub tag_names: Vec<String>,
}

pub fn create(
    conn: &mut Connection,
    source: Source,
    input: &NewMemo<'_>,
) -> rusqlite::Result<Memo> {
    let tx = conn.transaction()?;
    let font_size = match input.font_size {
        Some(v) => MemoFontSize::parse(v)?,
        None => MemoFontSize::default(),
    };
    let is_bold = input.is_bold.unwrap_or(false);
    let focus_x = input.focus_x.map(clamp_focus_coordinate);
    let focus_y = input.focus_y.map(clamp_focus_coordinate);
    let max_order: i64 = tx
        .query_row("SELECT COALESCE(MAX(sort_order), 0) FROM memos", [], |r| {
            r.get(0)
        })
        .unwrap_or(0);
    tx.execute(
        "INSERT INTO memos (content, color, project_id, sort_order, font_size, is_bold, focus_x, focus_y)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            input.content,
            input.color,
            input.project_id,
            max_order + 1,
            font_size.as_str(),
            is_bold as i64,
            focus_x,
            focus_y
        ],
    )?;
    let id = tx.last_insert_rowid();
    replace_tags(&tx, id, &input.tag_names)?;
    let after = get(&tx, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    let aj = serde_json::to_value(&after).unwrap();
    write_audit(&tx, source, Op::Create, "memos", id, None, Some(&aj))?;
    tx.commit()?;
    Ok(after)
}

pub struct UpdateMemo<'a> {
    pub content: Option<&'a str>,
    pub color: Option<&'a str>,
    /// `Some(Some(id))` → attach, `Some(None)` → detach, `None` → no change.
    pub project_id: Option<Option<i64>>,
    pub font_size: Option<&'a str>,
    pub is_bold: Option<bool>,
    pub focus_x: Option<Option<f64>>,
    pub focus_y: Option<Option<f64>>,
    pub tag_names: Option<Vec<String>>,
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
    if let Some(v) = patch.font_size {
        let font_size = MemoFontSize::parse(v)?;
        sets.push("font_size = ?");
        vals.push(Box::new(font_size.as_str().to_string()));
    }
    if let Some(v) = patch.is_bold {
        sets.push("is_bold = ?");
        vals.push(Box::new(v as i64));
    }
    if let Some(v) = patch.focus_x {
        sets.push("focus_x = ?");
        vals.push(Box::new(v.map(clamp_focus_coordinate)));
    }
    if let Some(v) = patch.focus_y {
        sets.push("focus_y = ?");
        vals.push(Box::new(v.map(clamp_focus_coordinate)));
    }
    if sets.is_empty() && patch.tag_names.is_none() {
        return Err(rusqlite::Error::ToSqlConversionFailure("no fields".into()));
    }
    if !sets.is_empty() {
        sets.push("updated_at = datetime('now')");
        vals.push(Box::new(id));
        let sql = format!("UPDATE memos SET {} WHERE id = ?", sets.join(", "));
        let refs: Vec<&dyn rusqlite::types::ToSql> = vals.iter().map(|p| p.as_ref()).collect();
        tx.execute(&sql, refs.as_slice())?;
    }
    if let Some(tag_names) = &patch.tag_names {
        replace_tags(&tx, id, tag_names)?;
        tx.execute(
            "UPDATE memos SET updated_at = datetime('now') WHERE id = ?1",
            [id],
        )?;
    }
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
        let mut stmt =
            conn.prepare("SELECT id FROM memos ORDER BY sort_order ASC LIMIT 1 OFFSET ?1")?;
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
            font_size: None,
            is_bold: None,
            focus_x: None,
            focus_y: None,
            tag_names: None,
        },
    )
}

fn clamp_focus_coordinate(v: f64) -> f64 {
    v.clamp(0.0, 1.0)
}

fn normalized_tag_names(tag_names: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for raw in tag_names {
        let name = raw.trim();
        if name.is_empty() || out.iter().any(|existing| existing == name) {
            continue;
        }
        out.push(name.to_string());
    }
    out
}

pub(crate) fn tag_id_for_name(conn: &Connection, name: &str) -> rusqlite::Result<i64> {
    match conn.query_row("SELECT id FROM memo_tags WHERE name = ?1", [name], |r| {
        r.get(0)
    }) {
        Ok(id) => Ok(id),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            let next_order: i64 = conn
                .query_row(
                    "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM memo_tags",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            conn.execute(
                "INSERT INTO memo_tags (name, sort_order) VALUES (?1, ?2)",
                params![name, next_order],
            )?;
            Ok(conn.last_insert_rowid())
        }
        Err(e) => Err(e),
    }
}

pub(crate) fn replace_tags(
    conn: &Connection,
    memo_id: i64,
    tag_names: &[String],
) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM memo_tag_links WHERE memo_id = ?1", [memo_id])?;
    for name in normalized_tag_names(tag_names) {
        let tag_id = tag_id_for_name(conn, &name)?;
        conn.execute(
            "INSERT OR IGNORE INTO memo_tag_links (memo_id, tag_id) VALUES (?1, ?2)",
            params![memo_id, tag_id],
        )?;
    }
    Ok(())
}

pub fn delete_by_number(
    conn: &mut Connection,
    source: Source,
    number: i64,
) -> rusqlite::Result<()> {
    let id: i64 = {
        let mut stmt =
            conn.prepare("SELECT id FROM memos ORDER BY sort_order ASC LIMIT 1 OFFSET ?1")?;
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
    fn memo_tag_crud_and_reorder() {
        let mut c = fresh();
        let created = create_memo_tag(&mut c, Source::Cli, "긴급", Some("#f97316")).unwrap();
        assert_eq!(created.name, "긴급");
        assert_eq!(created.color, "#f97316");

        let renamed = update_memo_tag(
            &mut c,
            Source::Cli,
            created.id,
            &UpdateMemoTag {
                name: Some("긴급검토"),
                color: Some("#fb7185"),
                sort_order: Some(9),
            },
        )
        .unwrap();
        assert_eq!(renamed.name, "긴급검토");
        assert_eq!(renamed.sort_order, 9);

        reorder_memo_tags(&mut c, &[renamed.id]).unwrap();
        let listed = list_memo_tags(&c).unwrap();
        assert_eq!(listed[0].id, renamed.id);

        delete_memo_tag(&mut c, Source::Cli, renamed.id).unwrap();
        assert!(list_memo_tags(&c)
            .unwrap()
            .iter()
            .all(|t| t.id != renamed.id));
    }

    #[test]
    fn delete_memo_tag_cascades_links_but_leaves_memo() {
        let mut c = fresh();
        let memo = create(
            &mut c,
            Source::Cli,
            &NewMemo {
                content: "keep memo",
                color: "yellow",
                project_id: None,
                font_size: None,
                is_bold: None,
                focus_x: None,
                focus_y: None,
                tag_names: vec!["임시".to_string()],
            },
        )
        .unwrap();
        let tag_id = memo.tags[0].id;

        delete_memo_tag(&mut c, Source::Cli, tag_id).unwrap();

        let memo_after = get(&c, memo.id).unwrap().unwrap();
        assert_eq!(memo_after.content, "keep memo");
        assert!(memo_after.tags.is_empty());
    }

    #[test]
    fn create_defaults_style_position_and_tags() {
        let mut c = fresh();
        let created = create(
            &mut c,
            Source::Cli,
            &NewMemo {
                content: "plain",
                color: "yellow",
                project_id: None,
                font_size: None,
                is_bold: None,
                focus_x: None,
                focus_y: None,
                tag_names: vec![],
            },
        )
        .unwrap();

        assert_eq!(created.font_size, crate::models::MemoFontSize::Normal);
        assert!(!created.is_bold);
        assert_eq!(created.focus_x, None);
        assert_eq!(created.focus_y, None);
        assert!(created.tags.is_empty());
    }

    #[test]
    fn update_rejects_invalid_font_size() {
        let mut c = fresh();
        let m = create(
            &mut c,
            Source::Cli,
            &NewMemo {
                content: "x",
                color: "yellow",
                project_id: None,
                font_size: None,
                is_bold: None,
                focus_x: None,
                focus_y: None,
                tag_names: vec![],
            },
        )
        .unwrap();

        let err = update(
            &mut c,
            Source::Cli,
            m.id,
            &UpdateMemo {
                content: None,
                color: None,
                project_id: None,
                font_size: Some("huge"),
                is_bold: None,
                focus_x: None,
                focus_y: None,
                tag_names: None,
            },
        )
        .unwrap_err();

        assert!(err.to_string().contains("font_size"));
    }

    #[test]
    fn update_clamps_focus_coordinates_and_replaces_tags() {
        let mut c = fresh();
        let m = create(
            &mut c,
            Source::Cli,
            &NewMemo {
                content: "tagged",
                color: "blue",
                project_id: None,
                font_size: Some("large"),
                is_bold: Some(true),
                focus_x: Some(1.5),
                focus_y: Some(-0.25),
                tag_names: vec!["검토".to_string(), "중요".to_string()],
            },
        )
        .unwrap();

        assert_eq!(m.font_size, crate::models::MemoFontSize::Large);
        assert!(m.is_bold);
        assert_eq!(m.focus_x, Some(1.0));
        assert_eq!(m.focus_y, Some(0.0));
        assert_eq!(
            m.tags.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
            vec!["중요", "검토"]
        );

        let updated = update(
            &mut c,
            Source::Cli,
            m.id,
            &UpdateMemo {
                content: None,
                color: None,
                project_id: None,
                font_size: Some("small"),
                is_bold: Some(false),
                focus_x: Some(Some(0.42)),
                focus_y: Some(Some(0.18)),
                tag_names: Some(vec!["대기".to_string()]),
            },
        )
        .unwrap();

        assert_eq!(updated.font_size, crate::models::MemoFontSize::Small);
        assert!(!updated.is_bold);
        assert_eq!(updated.focus_x, Some(0.42));
        assert_eq!(updated.focus_y, Some(0.18));
        assert_eq!(
            updated
                .tags
                .iter()
                .map(|t| t.name.as_str())
                .collect::<Vec<_>>(),
            vec!["대기"]
        );
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
                font_size: None,
                is_bold: None,
                focus_x: None,
                focus_y: None,
                tag_names: vec![],
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
                font_size: None,
                is_bold: None,
                focus_x: None,
                focus_y: None,
                tag_names: vec![],
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
                font_size: None,
                is_bold: None,
                focus_x: None,
                focus_y: None,
                tag_names: None,
            },
        )
        .unwrap();
        assert_eq!(updated.project_id, None);
    }

    #[test]
    fn update_by_number_changes_content() {
        let mut c = fresh();
        create(
            &mut c,
            Source::Cli,
            &NewMemo {
                content: "first",
                color: "yellow",
                project_id: None,
                font_size: None,
                is_bold: None,
                focus_x: None,
                focus_y: None,
                tag_names: vec![],
            },
        )
        .unwrap();
        create(
            &mut c,
            Source::Cli,
            &NewMemo {
                content: "second",
                color: "yellow",
                project_id: None,
                font_size: None,
                is_bold: None,
                focus_x: None,
                focus_y: None,
                tag_names: vec![],
            },
        )
        .unwrap();
        let updated = update_by_number(&mut c, Source::Cli, 1, "updated first").unwrap();
        assert_eq!(updated.content, "updated first");
    }

    #[test]
    fn delete_by_number_removes_correct_memo() {
        let mut c = fresh();
        create(
            &mut c,
            Source::Cli,
            &NewMemo {
                content: "a",
                color: "yellow",
                project_id: None,
                font_size: None,
                is_bold: None,
                focus_x: None,
                focus_y: None,
                tag_names: vec![],
            },
        )
        .unwrap();
        create(
            &mut c,
            Source::Cli,
            &NewMemo {
                content: "b",
                color: "yellow",
                project_id: None,
                font_size: None,
                is_bold: None,
                focus_x: None,
                focus_y: None,
                tag_names: vec![],
            },
        )
        .unwrap();
        delete_by_number(&mut c, Source::Cli, 1).unwrap();
        let remaining = list(&c).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].content, "b");
    }
}
