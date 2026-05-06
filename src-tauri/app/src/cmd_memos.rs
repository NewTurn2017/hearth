use crate::AppState;
use hearth_core::audit::Source;
use hearth_core::memos::{self, NewMemo, UpdateMemo};
use hearth_core::models::{Memo, MemoTag};
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
    pub font_size: Option<String>,
    pub is_bold: Option<bool>,
    pub focus_x: Option<f64>,
    pub focus_y: Option<f64>,
    pub tag_names: Option<Vec<String>>,
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
            font_size: data.font_size.as_deref(),
            is_bold: data.is_bold,
            focus_x: data.focus_x,
            focus_y: data.focus_y,
            tag_names: data.tag_names.unwrap_or_default(),
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
    pub font_size: Option<String>,
    pub is_bold: Option<bool>,
    pub focus_x: Option<Option<f64>>,
    pub focus_y: Option<Option<f64>>,
    pub tag_names: Option<Vec<String>>,
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
            font_size: fields.font_size.as_deref(),
            is_bold: fields.is_bold,
            focus_x: fields.focus_x,
            focus_y: fields.focus_y,
            tag_names: fields.tag_names,
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
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    let id: i64 = {
        let mut stmt = db
            .prepare("SELECT id FROM memos ORDER BY sort_order ASC LIMIT 1 OFFSET ?1")
            .map_err(|e| e.to_string())?;
        stmt.query_row([(number - 1).max(0)], |r| r.get::<_, i64>(0))
            .map_err(|_| format!("#{} 메모를 찾을 수 없음", number))?
    };
    memos::update(
        &mut db,
        Source::App,
        id,
        &UpdateMemo {
            content: fields.content.as_deref(),
            color: fields.color.as_deref(),
            project_id: fields.project_id,
            font_size: fields.font_size.as_deref(),
            is_bold: fields.is_bold,
            focus_x: fields.focus_x,
            focus_y: fields.focus_y,
            tag_names: fields.tag_names,
        },
    )
    .map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct MemoTagInput {
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemoTagInput {
    pub name: Option<String>,
    pub color: Option<String>,
    pub sort_order: Option<i64>,
}

#[tauri::command]
pub fn get_memo_tags(state: State<'_, AppState>) -> Result<Vec<MemoTag>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    memos::list_memo_tags(&db).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_memo_tag(state: State<'_, AppState>, input: MemoTagInput) -> Result<MemoTag, String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    memos::create_memo_tag(&mut db, Source::App, &input.name, input.color.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_memo_tag(
    state: State<'_, AppState>,
    id: i64,
    fields: UpdateMemoTagInput,
) -> Result<MemoTag, String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    memos::update_memo_tag(
        &mut db,
        Source::App,
        id,
        &memos::UpdateMemoTag {
            name: fields.name.as_deref(),
            color: fields.color.as_deref(),
            sort_order: fields.sort_order,
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_memo_tag(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    memos::delete_memo_tag(&mut db, Source::App, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_memo_tags(state: State<'_, AppState>, ids: Vec<i64>) -> Result<(), String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    memos::reorder_memo_tags(&mut db, &ids).map_err(|e| e.to_string())
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
