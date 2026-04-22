use crate::excel_import;
use crate::AppState;
use std::path::Path;
use std::process::Command;
use tauri::State;

#[tauri::command]
pub fn open_in_terminal(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    // macOS 기본 터미널 앱(Terminal.app)에서 해당 경로를 연다.
    // 사용자가 기본 터미널을 다른 앱(iTerm 등)으로 바꿨더라도 `open -a Terminal`은
    // 시스템 Terminal.app을 호출하므로, 추후 기본값 감지가 필요하면 여기서 분기한다.
    Command::new("open")
        .args(["-a", "Terminal", &path])
        .spawn()
        .map_err(|e| format!("Failed to open Terminal: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn open_in_finder(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    Command::new("open")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("Failed to open Finder: {}", e))?;
    Ok(())
}

#[derive(serde::Serialize)]
pub struct ImportResult {
    pub projects_imported: usize,
}

#[tauri::command]
pub fn import_excel(
    state: State<'_, AppState>,
    file_path: String,
    clear_existing: bool,
) -> Result<ImportResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    if clear_existing {
        db.execute_batch(
            "DELETE FROM memos; DELETE FROM schedules; DELETE FROM clients; DELETE FROM projects;",
        )
        .map_err(|e| e.to_string())?;
    }

    let count = excel_import::import_projects_from_xlsx(&db, &file_path)?;

    Ok(ImportResult {
        projects_imported: count,
    })
}
