use crate::models::Memo;
use crate::AppState;
use serde::Deserialize;
use tauri::State;

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

#[tauri::command]
pub fn get_memos(state: State<'_, AppState>) -> Result<Vec<Memo>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(&format!(
            "SELECT {} FROM memos ORDER BY sort_order ASC",
            SELECT_COLS
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row_to_memo).map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[derive(Debug, Deserialize)]
pub struct MemoInput {
    pub content: String,
    pub color: Option<String>,
    pub project_id: Option<i64>,
}

#[tauri::command]
pub fn create_memo(state: State<'_, AppState>, data: MemoInput) -> Result<Memo, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let max_order: i64 = db
        .query_row(
            "SELECT COALESCE(MAX(sort_order), 0) FROM memos",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let color = data.color.unwrap_or_else(|| "yellow".into());
    db.execute(
        "INSERT INTO memos (content, color, project_id, sort_order) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![data.content, color, data.project_id, max_order + 1],
    )
    .map_err(|e| e.to_string())?;

    let id = db.last_insert_rowid();
    db.query_row(
        &format!("SELECT {} FROM memos WHERE id = ?1", SELECT_COLS),
        [id],
        row_to_memo,
    )
    .map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemoInput {
    pub content: Option<String>,
    pub color: Option<String>,
    pub project_id: Option<Option<i64>>,
}

#[tauri::command]
pub fn update_memo(
    state: State<'_, AppState>,
    id: i64,
    fields: UpdateMemoInput,
) -> Result<Memo, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut sets: Vec<String> = vec![];
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![];

    if let Some(ref v) = fields.content {
        sets.push("content = ?".into());
        params.push(Box::new(v.clone()));
    }
    if let Some(ref v) = fields.color {
        sets.push("color = ?".into());
        params.push(Box::new(v.clone()));
    }
    if let Some(ref v) = fields.project_id {
        sets.push("project_id = ?".into());
        params.push(Box::new(*v));
    }

    if sets.is_empty() {
        return Err("No fields to update".into());
    }

    sets.push("updated_at = datetime('now')".into());
    params.push(Box::new(id));

    let sql = format!("UPDATE memos SET {} WHERE id = ?", sets.join(", "));
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    db.execute(&sql, param_refs.as_slice()).map_err(|e| e.to_string())?;

    db.query_row(
        &format!("SELECT {} FROM memos WHERE id = ?1", SELECT_COLS),
        [id],
        row_to_memo,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_memo(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute("DELETE FROM memos WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn reorder_memos(state: State<'_, AppState>, ids: Vec<i64>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
    for (i, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE memos SET sort_order = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![i as i64, id],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
