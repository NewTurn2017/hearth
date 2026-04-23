use crate::AppState;
use hearth_core::categories::{self, Category, CategoryError, UpdateCategory};
use serde::Deserialize;
use tauri::State;

#[tauri::command]
pub fn get_categories(state: State<'_, AppState>) -> Result<Vec<Category>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    categories::list(&db).map_err(|e| e.to_string())
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
    let db = state.db.lock().map_err(|e| e.to_string())?;
    categories::create(&db, &input.name, input.color.as_deref())
        .map_err(|e| format_err(&e))
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
    categories::update(
        &mut db,
        id,
        &UpdateCategory {
            name: fields.name.as_deref(),
            color: fields.color.as_deref(),
            sort_order: fields.sort_order,
        },
    )
    .map_err(|e| format_err(&e))
}

#[tauri::command]
pub fn delete_category(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    categories::delete(&db, id).map_err(|e| format_err(&e))
}

#[tauri::command]
pub fn reorder_categories(state: State<'_, AppState>, ids: Vec<i64>) -> Result<(), String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    categories::reorder(&mut db, &ids).map_err(|e| e.to_string())
}

fn format_err(e: &CategoryError) -> String {
    e.to_string()
}
