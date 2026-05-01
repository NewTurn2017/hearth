use crate::bookmark;
use crate::excel_import;
use crate::AppState;
use std::path::Path;
use tauri::State;

/// Sentinel error string the frontend matches on to trigger the
/// just-in-time bookmark prompt for legacy projects (FB-001).
pub const ERR_NEEDS_BOOKMARK: &str = "needs_bookmark";

#[tauri::command]
pub fn open_in_terminal(path: String, project_id: Option<i64>) -> Result<(), String> {
    with_project_access(&path, project_id, |resolved| open_in_terminal_impl(resolved))
}

#[tauri::command]
pub fn open_in_finder(path: String, project_id: Option<i64>) -> Result<(), String> {
    with_project_access(&path, project_id, |resolved| open_in_finder_impl(resolved))
}

/// If a project bookmark exists, resolve it and run `f` against the resolved
/// path with security-scoped access started; otherwise call `f` against the
/// raw path. Returns ERR_NEEDS_BOOKMARK if the path can't be reached and no
/// bookmark is available to grant access.
fn with_project_access<F>(path: &str, project_id: Option<i64>, f: F) -> Result<(), String>
where
    F: FnOnce(&str) -> Result<(), String>,
{
    #[cfg(target_os = "macos")]
    {
        if let Some(id) = project_id {
            if let Some(blob) = bookmark::read_project_bookmark(id) {
                let access = bookmark::start_access(&blob)?;
                let resolved_path = access
                    .url()
                    .path()
                    .ok_or_else(|| "resolved NSURL has no path".to_string())?
                    .to_string();
                let result = f(&resolved_path);
                drop(access);
                return result;
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (project_id, &bookmark::read_project_bookmark);
    }

    // No bookmark available — try the raw path. Under sandbox this works
    // only for paths inside the container; everything else surfaces
    // ERR_NEEDS_BOOKMARK so the frontend can prompt for one.
    let p = Path::new(path);
    if !p.exists() {
        if cfg!(target_os = "macos") && project_id.is_some() {
            return Err(ERR_NEEDS_BOOKMARK.to_string());
        }
        return Err(format!("Path does not exist: {}", path));
    }
    f(path)
}

#[cfg(target_os = "macos")]
fn open_in_finder_impl(path: &str) -> Result<(), String> {
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::{NSString, NSURL};
    {
        let workspace = NSWorkspace::sharedWorkspace();
        let path_ns = NSString::from_str(path);
        let url = NSURL::fileURLWithPath(&path_ns);
        if !workspace.openURL(&url) {
            return Err(format!("NSWorkspace.openURL failed for: {}", path));
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn open_in_terminal_impl(path: &str) -> Result<(), String> {
    // Pin to the system Terminal.app (com.apple.Terminal). Even when the
    // user's default terminal handler is iTerm/Ghostty/etc., this command
    // intentionally surfaces Apple's stock Terminal — matches the prior
    // `open -a Terminal` behavior.
    use objc2_app_kit::{NSWorkspace, NSWorkspaceOpenConfiguration};
    use objc2_foundation::{NSArray, NSString, NSURL};
    {
        let workspace = NSWorkspace::sharedWorkspace();
        let bundle_id = NSString::from_str("com.apple.Terminal");
        let Some(terminal_url) = workspace.URLForApplicationWithBundleIdentifier(&bundle_id)
        else {
            return Err("Terminal.app (com.apple.Terminal) not found".to_string());
        };

        let path_ns = NSString::from_str(path);
        let dir_url = NSURL::fileURLWithPath(&path_ns);
        let urls = NSArray::from_retained_slice(&[dir_url]);
        let config = NSWorkspaceOpenConfiguration::configuration();

        workspace.openURLs_withApplicationAtURL_configuration_completionHandler(
            &urls,
            &terminal_url,
            &config,
            None,
        );
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn open_in_finder_impl(path: &str) -> Result<(), String> {
    use std::process::Command;
    Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to open file manager: {}", e))
}

#[cfg(not(target_os = "macos"))]
fn open_in_terminal_impl(_path: &str) -> Result<(), String> {
    Err("open_in_terminal is only supported on macOS".to_string())
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
