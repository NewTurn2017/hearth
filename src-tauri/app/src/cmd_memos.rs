use crate::AppState;
use hearth_core::audit::Source;
use hearth_core::memos::{self, NewMemo, UpdateMemo};
use hearth_core::models::Memo;
use serde::Deserialize;
use tauri::State;

#[tauri::command]
pub fn get_memos(state: State<'_, AppState>) -> Result<Vec<Memo>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    memos::list(&db).map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct MemoInput {
    pub content: String,
    pub color: Option<String>,
    pub project_id: Option<i64>,
}

#[tauri::command]
pub fn create_memo(state: State<'_, AppState>, data: MemoInput) -> Result<Memo, String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    let color = data.color.as_deref().unwrap_or("yellow");
    memos::create(
        &mut db,
        Source::App,
        &NewMemo {
            content: &data.content,
            color,
            project_id: data.project_id,
        },
    )
    .map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemoInput {
    pub content: Option<String>,
    pub color: Option<String>,
    /// `null` in JSON → detach (Some(None)), omit or non-null → Some(Some(id)) or None
    pub project_id: Option<Option<i64>>,
}

#[tauri::command]
pub fn update_memo(
    state: State<'_, AppState>,
    id: i64,
    fields: UpdateMemoInput,
) -> Result<Memo, String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    memos::update(
        &mut db,
        Source::App,
        id,
        &UpdateMemo {
            content: fields.content.as_deref(),
            color: fields.color.as_deref(),
            project_id: fields.project_id,
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_memo(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    memos::delete(&mut db, Source::App, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_memos(state: State<'_, AppState>, ids: Vec<i64>) -> Result<(), String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    memos::reorder(&mut db, &ids).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_memo_by_number(
    state: State<'_, AppState>,
    number: i64,
    fields: UpdateMemoInput,
) -> Result<Memo, String> {
    if number < 1 {
        return Err(format!("#{} 메모를 찾을 수 없음", number));
    }
    let new_content = fields.content.ok_or_else(|| "content is required for update_by_number".to_string())?;
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    memos::update_by_number(&mut db, Source::App, number, &new_content)
        .map_err(|_| format!("#{} 메모를 찾을 수 없음", number))
}

#[tauri::command]
pub fn delete_memo_by_number(state: State<'_, AppState>, number: i64) -> Result<(), String> {
    if number < 1 {
        return Err(format!("#{} 메모를 찾을 수 없음", number));
    }
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    memos::delete_by_number(&mut db, Source::App, number)
        .map_err(|_| format!("#{} 메모를 찾을 수 없음", number))
}
