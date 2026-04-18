use crate::cmd_settings;
use crate::AppState;
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> Result<bool, String> {
    let enabled = app.autolaunch().is_enabled().map_err(|e| e.to_string())?;
    Ok(enabled)
}

#[tauri::command]
pub fn set_autostart(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    if enabled {
        app.autolaunch().enable().map_err(|e| e.to_string())?;
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())?;
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    cmd_settings::write(&db, cmd_settings::K_AUTOSTART, if enabled { "1" } else { "0" })?;
    Ok(())
}
