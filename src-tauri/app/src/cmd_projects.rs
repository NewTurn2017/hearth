use crate::bookmark::{self, PickFolderResponse};
use crate::AppState;
use hearth_core::audit::Source;
use hearth_core::models::Project;
use hearth_core::projects::{self, NewProject, UpdateProject};
use serde::Deserialize;
use tauri::{AppHandle, State};

#[derive(Debug, Deserialize)]
pub struct ProjectFilter {
    pub priorities: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
}

#[tauri::command]
pub fn get_projects(
    state: State<'_, AppState>,
    filter: Option<ProjectFilter>,
) -> Result<Vec<Project>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let all = projects::list(&db).map_err(|e| e.to_string())?;
    let filtered: Vec<Project> = all
        .into_iter()
        .filter(|p| match filter.as_ref() {
            None => true,
            Some(f) => {
                let pri_ok = match f.priorities.as_ref() {
                    None => true,
                    Some(v) if v.is_empty() => true,
                    Some(v) => v.contains(&p.priority),
                };
                let cat_ok = match f.categories.as_ref() {
                    None => true,
                    Some(v) if v.is_empty() => true,
                    Some(v) => p
                        .category
                        .as_ref()
                        .map_or(false, |c| v.contains(c)),
                };
                pri_ok && cat_ok
            }
        })
        .collect();
    Ok(filtered)
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectInput {
    pub name: Option<String>,
    pub priority: Option<String>,
    pub category: Option<String>,
    pub path: Option<String>,
    pub evaluation: Option<String>,
    /// Security-scoped bookmark for `path`. When present, replaces any
    /// previously-stored bookmark for this project. macOS only; ignored
    /// on other platforms. Sent as a JSON number array (Vec<u8>).
    pub path_bookmark: Option<Vec<u8>>,
}

#[tauri::command]
pub fn update_project(
    state: State<'_, AppState>,
    id: i64,
    fields: UpdateProjectInput,
) -> Result<Project, String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    let updated = projects::update(
        &mut db,
        Source::App,
        id,
        &UpdateProject {
            name: fields.name.as_deref(),
            priority: fields.priority.as_deref(),
            category: fields.category.as_deref(),
            path: fields.path.as_deref(),
            evaluation: fields.evaluation.as_deref(),
        },
    )
    .map_err(|e| e.to_string())?;
    if let Some(blob) = fields.path_bookmark.as_ref() {
        if blob.is_empty() {
            bookmark::clear_project_bookmark(id);
        } else {
            bookmark::write_project_bookmark(id, blob);
        }
    }
    Ok(updated)
}

#[tauri::command]
pub fn create_project(
    state: State<'_, AppState>,
    name: String,
    priority: String,
    category: Option<String>,
    path: Option<String>,
    path_bookmark: Option<Vec<u8>>,
) -> Result<Project, String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    let created = projects::create(
        &mut db,
        Source::App,
        &NewProject {
            name: &name,
            priority: &priority,
            category: category.as_deref(),
            path: path.as_deref(),
            evaluation: None,
        },
    )
    .map_err(|e| e.to_string())?;
    if let Some(blob) = path_bookmark.as_ref() {
        if !blob.is_empty() {
            bookmark::write_project_bookmark(created.id, blob);
        }
    }
    Ok(created)
}

#[tauri::command]
pub fn delete_project(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    projects::delete(&mut db, Source::App, id).map_err(|e| e.to_string())?;
    bookmark::clear_project_bookmark(id);
    Ok(())
}

/// Open NSOpenPanel for a directory and return a freshly-created
/// security-scoped bookmark. The frontend ships the bookmark back via
/// create_project / update_project to persist it.
#[tauri::command]
pub async fn pick_project_folder(
    app: AppHandle,
    suggested: Option<String>,
) -> Result<PickFolderResponse, String> {
    bookmark::pick_directory(app, suggested).await
}

#[tauri::command]
pub fn reorder_projects(state: State<'_, AppState>, ids: Vec<i64>) -> Result<(), String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    projects::reorder(&mut db, &ids).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_projects(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<Project>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    projects::search_like(&db, &query).map_err(|e| e.to_string())
}
