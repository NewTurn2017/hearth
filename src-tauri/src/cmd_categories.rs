// User-editable project categories.
//
// Rename propagates to `projects.category` inside a transaction — we keep the
// column as TEXT without a FK so existing rows survive a category drop-in.
// Delete refuses with a Korean error when any project still references the
// name; the UI shows the live usage count on every row.
//
// The `categories` row is the single source of truth for category color /
// order. The legacy `CATEGORY_COLORS` constant in `types.ts` stays in the
// codebase as the seed list and as a last-resort UI fallback.

use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Clone)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub sort_order: i64,
    /// Derived (SELECT COUNT(*) FROM projects WHERE category = name).
    /// The UI uses this to disable the delete button and to format the
    /// "사용 중 (N개)" error.
    pub usage_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

fn row_to_category(row: &rusqlite::Row) -> rusqlite::Result<Category> {
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
    "SELECT c.id, c.name, c.color, c.sort_order, \
            (SELECT COUNT(*) FROM projects p WHERE p.category = c.name) AS usage_count, \
            c.created_at, c.updated_at \
     FROM categories c";

#[tauri::command]
pub fn get_categories(state: State<'_, AppState>) -> Result<Vec<Category>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let sql = format!("{} ORDER BY c.sort_order ASC, c.id ASC", SELECT_WITH_USAGE);
    let mut stmt = db.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], row_to_category)
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[derive(Debug, Deserialize)]
pub struct CreateCategoryInput {
    pub name: String,
    pub color: Option<String>,
}

#[tauri::command]
pub fn create_category(
    state: State<'_, AppState>,
    input: CreateCategoryInput,
) -> Result<Category, String> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err("카테고리 이름이 비어 있습니다".into());
    }
    let color = input
        .color
        .unwrap_or_else(|| "#6b7280".into())
        .trim()
        .to_string();

    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Reject duplicate name explicitly so the UI can surface the Korean error
    // instead of relying on the raw UNIQUE constraint message.
    let exists: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM categories WHERE name = ?1",
            [&name],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if exists > 0 {
        return Err(format!("이미 존재하는 카테고리 이름입니다: {name}"));
    }

    let next_order: i64 = db
        .query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM categories",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    db.execute(
        "INSERT INTO categories (name, color, sort_order) VALUES (?1, ?2, ?3)",
        rusqlite::params![name, color, next_order],
    )
    .map_err(|e| e.to_string())?;

    let id = db.last_insert_rowid();
    db.query_row(
        &format!("{} WHERE c.id = ?1", SELECT_WITH_USAGE),
        [id],
        row_to_category,
    )
    .map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryInput {
    pub name: Option<String>,
    pub color: Option<String>,
    pub sort_order: Option<i64>,
}

#[tauri::command]
pub fn update_category(
    state: State<'_, AppState>,
    id: i64,
    fields: UpdateCategoryInput,
) -> Result<Category, String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;

    // Resolve the current name first so we can drive the rename cascade from
    // a single place (and reject a duplicate-target-name before touching
    // projects).
    let current_name: String = db
        .query_row(
            "SELECT name FROM categories WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .map_err(|_| format!("카테고리를 찾을 수 없음: id={id}"))?;

    let mut new_name: Option<String> = None;
    if let Some(raw) = fields.name.as_ref() {
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            return Err("카테고리 이름이 비어 있습니다".into());
        }
        if trimmed != current_name {
            let collides: i64 = db
                .query_row(
                    "SELECT COUNT(*) FROM categories WHERE name = ?1 AND id <> ?2",
                    rusqlite::params![trimmed, id],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            if collides > 0 {
                return Err(format!("이미 존재하는 카테고리 이름입니다: {trimmed}"));
            }
            new_name = Some(trimmed);
        }
    }

    let tx = db.transaction().map_err(|e| e.to_string())?;

    if let Some(ref n) = new_name {
        tx.execute(
            "UPDATE categories SET name = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![n, id],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE projects SET category = ?1, updated_at = datetime('now') WHERE category = ?2",
            rusqlite::params![n, current_name],
        )
        .map_err(|e| e.to_string())?;
    }

    if let Some(ref color) = fields.color {
        tx.execute(
            "UPDATE categories SET color = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![color.trim(), id],
        )
        .map_err(|e| e.to_string())?;
    }

    if let Some(ord) = fields.sort_order {
        tx.execute(
            "UPDATE categories SET sort_order = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![ord, id],
        )
        .map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;

    db.query_row(
        &format!("{} WHERE c.id = ?1", SELECT_WITH_USAGE),
        [id],
        row_to_category,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_category(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let (name, usage): (String, i64) = db
        .query_row(
            "SELECT name, (SELECT COUNT(*) FROM projects p WHERE p.category = c.name) \
             FROM categories c WHERE c.id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| format!("카테고리를 찾을 수 없음: id={id}"))?;

    if usage > 0 {
        return Err(format!("카테고리 사용 중 ({usage}개 프로젝트): {name}"));
    }
    db.execute("DELETE FROM categories WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn reorder_categories(
    state: State<'_, AppState>,
    ids: Vec<i64>,
) -> Result<(), String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    for (i, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE categories SET sort_order = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![i as i64, id],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
