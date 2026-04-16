use crate::AppState;
use chrono::Local;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};

fn backup_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("backups");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn db_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("data.db"))
}

#[tauri::command]
pub fn backup_db(
    state: State<'_, AppState>,
    app: AppHandle,
    dest_path: Option<String>,
) -> Result<String, String> {
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|e| e.to_string())?;
    }

    let source = db_path(&app)?;
    let dest = match dest_path {
        Some(p) => PathBuf::from(p),
        None => {
            let dir = backup_dir(&app)?;
            let timestamp = Local::now().format("%Y-%m-%d-%H%M%S");
            dir.join(format!("project-genie-backup-{}.db", timestamp))
        }
    };

    fs::copy(&source, &dest).map_err(|e| format!("Backup failed: {}", e))?;

    let dir = backup_dir(&app)?;
    let mut backups: Vec<_> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("project-genie-backup-")
        })
        .collect();
    backups.sort_by_key(|e| std::cmp::Reverse(e.file_name()));
    for old in backups.into_iter().skip(5) {
        fs::remove_file(old.path()).ok();
    }

    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub fn restore_db(app: AppHandle, src_path: String) -> Result<(), String> {
    let source = PathBuf::from(&src_path);
    if !source.exists() {
        return Err("Backup file not found".into());
    }
    let dest = db_path(&app)?;
    fs::copy(&source, &dest).map_err(|e| format!("Restore failed: {}", e))?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct BackupInfo {
    pub path: String,
    pub filename: String,
    pub size_bytes: u64,
    pub created: String,
}

#[tauri::command]
pub fn list_backups(app: AppHandle) -> Result<Vec<BackupInfo>, String> {
    let dir = backup_dir(&app)?;
    let mut backups: Vec<BackupInfo> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("project-genie-backup-")
        })
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            let modified = meta
                .modified()
                .ok()
                .map(|t| {
                    let dt: chrono::DateTime<Local> = t.into();
                    dt.format("%Y-%m-%d %H:%M:%S").to_string()
                })
                .unwrap_or_default();
            Some(BackupInfo {
                path: e.path().to_string_lossy().to_string(),
                filename: e.file_name().to_string_lossy().to_string(),
                size_bytes: meta.len(),
                created: modified,
            })
        })
        .collect();
    backups.sort_by(|a, b| b.filename.cmp(&a.filename));
    Ok(backups)
}

pub fn auto_backup_on_close(app: &AppHandle) {
    let state: State<'_, AppState> = app.state();
    if let Ok(db) = state.db.lock() {
        db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").ok();
    }
    let source = app
        .path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("data.db"));
    let dest_dir = app
        .path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("backups"));

    if let (Some(src), Some(dir)) = (source, dest_dir) {
        fs::create_dir_all(&dir).ok();
        let timestamp = Local::now().format("%Y-%m-%d-%H%M%S");
        let dest = dir.join(format!("project-genie-backup-{}.db", timestamp));
        fs::copy(&src, &dest).ok();

        if let Ok(entries) = fs::read_dir(&dir) {
            let mut backups: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .starts_with("project-genie-backup-")
                })
                .collect();
            backups.sort_by_key(|e| std::cmp::Reverse(e.file_name()));
            for old in backups.into_iter().skip(5) {
                fs::remove_file(old.path()).ok();
            }
        }
    }
}
