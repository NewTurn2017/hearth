use crate::cmd_settings::{self, K_BACKUP_DIR};
use crate::AppState;
use chrono::Local;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};

/// Resolve the directory that holds rolling backups. Reads `backup.dir` from
/// the settings KV; falls back to `$APP_DATA/backups` when unset so first-run
/// behavior matches the pre-setting world.
fn backup_dir(app: &AppHandle, state: &State<'_, AppState>) -> Result<PathBuf, String> {
    let configured = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        cmd_settings::read(&db, K_BACKUP_DIR)?
    };
    let dir = if configured.is_empty() {
        app.path()
            .app_data_dir()
            .map_err(|e| e.to_string())?
            .join("backups")
    } else {
        PathBuf::from(configured)
    };
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
pub fn get_backup_dir(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    Ok(backup_dir(&app, &state)?.to_string_lossy().to_string())
}

#[tauri::command]
pub fn set_backup_dir(
    state: State<'_, AppState>,
    path: String,
) -> Result<String, String> {
    let canonical = PathBuf::from(path.trim());
    if canonical.as_os_str().is_empty() {
        return Err("백업 위치가 비어 있습니다".into());
    }
    fs::create_dir_all(&canonical)
        .map_err(|e| format!("백업 폴더를 만들 수 없습니다: {e}"))?;
    let stored = canonical.to_string_lossy().to_string();
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        cmd_settings::write(&db, K_BACKUP_DIR, &stored)?;
    }
    Ok(stored)
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
    let dir = backup_dir(&app, &state)?;
    let dest = match dest_path {
        Some(p) => PathBuf::from(p),
        None => {
            let timestamp = Local::now().format("%Y-%m-%d-%H%M%S");
            dir.join(format!("hearth-backup-{}.db", timestamp))
        }
    };

    fs::copy(&source, &dest).map_err(|e| format!("Backup failed: {}", e))?;

    let mut backups: Vec<_> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("hearth-backup-")
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
pub fn list_backups(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<Vec<BackupInfo>, String> {
    let dir = backup_dir(&app, &state)?;
    let mut backups: Vec<BackupInfo> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name();
            let n = name.to_string_lossy();
            // `hearth-backup-*.db` → regular rolling backups
            // `pre-reset-*.db`     → snapshots captured right before a reset
            n.starts_with("hearth-backup-") || n.starts_with("pre-reset-")
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

/// Wipe user content (projects, memos, schedules, clients) in a single
/// transaction, after taking a `pre-reset-<ts>.db` snapshot in the backup
/// directory. The snapshot uses its own filename prefix so the normal
/// rolling-retention logic (which only touches `hearth-backup-*.db`) leaves
/// it alone — the user should always be able to get back to the pre-reset
/// state via the Restore list in the 백업 tab.
///
/// Categories, settings (AI creds, backup dir, UI scale), and the
/// `sqlite_sequence` table are **preserved** — the intent is "empty my
/// workspace" not "factory reset."
#[tauri::command]
pub fn reset_data(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    // 1) Flush WAL so the snapshot carries the latest committed state.
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|e| e.to_string())?;
    }

    // 2) Copy the current DB to <backup_dir>/pre-reset-<ts>.db.
    let source = db_path(&app)?;
    let dir = backup_dir(&app, &state)?;
    let timestamp = Local::now().format("%Y-%m-%d-%H%M%S");
    let snapshot = dir.join(format!("pre-reset-{}.db", timestamp));
    fs::copy(&source, &snapshot)
        .map_err(|e| format!("pre-reset snapshot failed: {}", e))?;

    // 3) Wipe user content in a single transaction. `sqlite_sequence` is
    // reset too so the next created row starts at id=1 again — otherwise
    // AUTOINCREMENT would keep climbing past the deleted max.
    {
        let mut db = state.db.lock().map_err(|e| e.to_string())?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        for table in ["memos", "schedules", "projects", "clients"] {
            tx.execute(&format!("DELETE FROM {}", table), [])
                .map_err(|e| e.to_string())?;
        }
        tx.execute(
            "DELETE FROM sqlite_sequence WHERE name IN (?, ?, ?, ?)",
            ["memos", "schedules", "projects", "clients"],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
    }

    Ok(snapshot.to_string_lossy().to_string())
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
    let dest_dir = backup_dir(app, &state).ok();

    if let (Some(src), Some(dir)) = (source, dest_dir) {
        let timestamp = Local::now().format("%Y-%m-%d-%H%M%S");
        let dest = dir.join(format!("hearth-backup-{}.db", timestamp));
        fs::copy(&src, &dest).ok();

        if let Ok(entries) = fs::read_dir(&dir) {
            let mut backups: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .starts_with("hearth-backup-")
                })
                .collect();
            backups.sort_by_key(|e| std::cmp::Reverse(e.file_name()));
            for old in backups.into_iter().skip(5) {
                fs::remove_file(old.path()).ok();
            }
        }
    }
}
