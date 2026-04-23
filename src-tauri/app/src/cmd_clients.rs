use crate::AppState;
use hearth_core::clients;
use hearth_core::models::Client;
use tauri::State;

#[tauri::command]
pub fn get_clients(state: State<'_, AppState>) -> Result<Vec<Client>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    clients::list(&db).map_err(|e| e.to_string())
}
